# BCS Constant-Time Implementation — Design Document

**Version:** 0.2.0-ct  
**Target:** `bcs-core-rust` v0.2.x with `--features ct`  
**Status:** Specification (implementation in progress)  
**Last updated:** 2026-05-17

---

## 1. Why Constant-Time?

The current reference implementation uses `num_bigint::BigUint`, whose
arithmetic operations branch and loop on the *value* of their operands.
Concretely, scalar multiplication `k · G` exhibits the following leaks:

1. **Bit-length leak.** `BigUint::bits()` reveals the most-significant set
   bit of the secret scalar `k`.
2. **Double-and-add leak.** A standard `while bit_index < n` loop performs
   *additions only when the bit is 1*. The total runtime is therefore a
   direct function of the Hamming weight of `k`.
3. **Branching reductions.** `BigUint::div_rem` and modular inversion via
   the extended Euclidean algorithm contain data-dependent branches in
   the `num-bigint` crate.

A network attacker who can observe even a few microseconds of timing
variance over `10^4`–`10^6` ECDH handshakes can therefore reconstruct
significant portions of the secret. This is the single largest gap
between *"research reference"* and *"production crypto"*.

The constant-time (CT) implementation closes this gap.

---

## 2. Field `F_p_521`

### 2.1 Limb layout

The field prime is

```text
p_521 = 0xC9E2 9EAA 2DA9 BA21 ... E0BA 9F39 ...   (521 bits)
```

We represent an element `a ∈ F_p` as

```rust
pub struct Fp521 {
    pub(crate) limbs: [u64; 9],   // little-endian
}
```

giving us **576 bits of storage** with a 55-bit head-room (`576 − 521 = 55`).
This head-room is generous enough to defer reduction for up to 32 lazy
additions before any carry could escape the top limb — far more than any
of our addition chains require.

### 2.2 Montgomery representation

To multiply two field elements in constant time without a division step
we work in **Montgomery form**:

```text
R   = 2^576 mod p
R^2 = (2^576)^2 mod p          (pre-computed constant)
ã   = a · R mod p              (Montgomery form of a)
```

Multiplication in Montgomery form is

```text
mont_mul(ã, b̃)  =  ã · b̃ · R^{-1} mod p
```

implemented by the standard CIOS (Coarsely Integrated Operand Scanning)
algorithm with a fixed loop count and no data-dependent branches.
Final subtraction of `p` is performed unconditionally with a
constant-time conditional select.

### 2.3 API surface (final-form)

```rust
impl Fp521 {
    pub const ZERO: Self;
    pub const ONE:  Self;          // in Montgomery form: R mod p
    pub const P:    Self;          // for serialization checks

    pub fn from_bytes_be(b: &[u8; 66]) -> CtOption<Self>;
    pub fn to_bytes_be(&self) -> [u8; 66];

    pub fn add(&self, rhs: &Self) -> Self;       // CT
    pub fn sub(&self, rhs: &Self) -> Self;       // CT
    pub fn neg(&self) -> Self;                   // CT
    pub fn mul(&self, rhs: &Self) -> Self;       // CT, Montgomery
    pub fn square(&self) -> Self;                // CT, Montgomery
    pub fn invert(&self) -> CtOption<Self>;      // CT, Fermat via addition chain
    pub fn sqrt(&self) -> CtOption<Self>;        // CT, Tonelli-Shanks (p ≡ 3 mod 4? check)

    pub fn conditional_swap(a: &mut Self, b: &mut Self, choice: Choice);
}
```

All methods are `#[inline]` and contain *no* `if` on secret data and
*no* loop whose bound depends on secret data.

### 2.4 Inversion

`p_521 − 2` has Hamming weight ≈ 260. A naive square-and-multiply costs
≈ 780 field multiplications. We instead use an **addition chain** of
length ≈ 540 generated offline (the same technique used by
`curve25519-dalek` and `p256`).

The chain is encoded as a `static` array of opcodes (`Square` /
`Multiply(i)`) and executed in a fixed-length loop.

---

## 3. Scalar `Z/nZ` where `n = n_521`

```rust
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Scalar {
    pub(crate) limbs: [u64; 9],
}
```

Operations required:

- `Scalar::from_bytes_be_reduced(b: &[u8; 66]) -> Self`  — wide reduce.
- `Scalar::to_bytes_be(&self) -> [u8; 66]`.
- `Scalar::bits_msb_first(&self) -> impl Iterator<Item = u8>`  — exactly
  521 bits, regardless of leading zeros.
- `add`, `sub`, `mul` mod n (CT, Barrett reduction with a precomputed
  constant `μ = ⌊2^1042 / n⌋`).
- Automatic `Zeroize` on drop.

The Montgomery ladder only needs `bits_msb_first` and conditional
swapping of points; it does not need scalar multiplication mod n.

---

## 4. Elliptic-curve point

### 4.1 Coordinate system

We use **Jacobian projective** coordinates:

```text
(X : Y : Z)   represents the affine point  (X / Z^2,  Y / Z^3)
Z = 0          represents the point at infinity O
```

```rust
pub struct ProjPoint {
    pub(crate) x: Fp521,
    pub(crate) y: Fp521,
    pub(crate) z: Fp521,
}
```

Doubling and addition formulas: the **complete addition formulas**
of *Renes–Costello–Batina, EUROCRYPT 2016* for short Weierstrass
curves `y² = x³ + ax + b` with `a ≠ 0` (our curve has
`a = 5, b = 4`, plus the `−2x²` term which we absorb by a change of
variable `x ← x − 2/3` to put the curve in short Weierstrass form
internally — only inside CT code).

These formulas are **strongly unified**: the same code handles
`P + P`, `P + Q`, `P + O`, and `P + (−P)` without branches, which is
exactly what we need for a side-channel-clean implementation.

### 4.2 Conditional swap

```rust
fn conditional_swap(a: &mut ProjPoint, b: &mut ProjPoint, c: Choice) {
    Fp521::conditional_swap(&mut a.x, &mut b.x, c);
    Fp521::conditional_swap(&mut a.y, &mut b.y, c);
    Fp521::conditional_swap(&mut a.z, &mut b.z, c);
}
```

---

## 5. Scalar multiplication — Montgomery ladder

```rust
pub fn scalar_mul_ct(k: &Scalar, p: &ProjPoint) -> ProjPoint {
    let mut r0 = ProjPoint::identity();
    let mut r1 = p.clone();

    // Always 521 iterations, regardless of k.
    for bit in k.bits_msb_first() {
        let b = Choice::from(bit);
        ProjPoint::conditional_swap(&mut r0, &mut r1, b);
        r1 = r0.add(&r1);          // complete formula, no branches
        r0 = r0.double();          // complete formula, no branches
        ProjPoint::conditional_swap(&mut r0, &mut r1, b);
    }
    r0
}
```

**Invariants** preserved each iteration:

```text
r1 − r0 = P    (always)
r0      = ⌊k_high⌋ · P
r1      = (⌊k_high⌋ + 1) · P
```

where `k_high` is the prefix of `k` consumed so far.

---

## 6. Test plan

### 6.1 Parity tests (`tests/test_ct_parity.rs`)

For each of the 10 frozen test vectors in
`tests/test_vectors_521.rs`, run **both** the BigUint reference impl
and the CT impl and assert that the resulting:

- public-key bytes,
- shared-secret bytes,
- HKDF output bytes,
- AES-GCM ciphertext bytes,

are **byte-equal**. If even one byte differs, the CT impl is wrong and
the test fails.

### 6.2 Side-channel test (`dudect/leak_test.rs`)

We use the `dudect` methodology (Reparaz–Balasch–Verbauwhede, 2017):

1. Generate `N = 10^6` random scalars.
2. Split into class 0 (random) and class 1 (a fixed reference scalar).
3. Measure `scalar_mul_ct` runtime with `rdtsc` for each.
4. Compute Welch's t-statistic.
5. **Pass criterion: |t| < 4.5** at any percentile cropping level.

A failure means there is a measurable timing leak; we then iterate.

### 6.3 Benchmarks (`benches/ct_bench.rs`)

We benchmark, with `criterion`:

- `Fp521::mul` (target: < 2 µs on x86-64 with AVX2).
- `Fp521::invert` (target: < 1 ms).
- `scalar_mul_ct` (target: < 5 ms; OpenSSL P-521 reference ≈ 1 ms).

A 5× slowdown vs OpenSSL is acceptable for the *first* CT release. The
plan for v0.3.0 is to integrate fiat-crypto-generated field code, at
which point we should be within 2× of OpenSSL.

---

## 7. Acceptance criteria — `v0.2.0-ct` ships when

- [ ] `cargo build --features ct` succeeds with `#![deny(warnings)]`.
- [ ] `cargo test --features ct` passes, including the 10 parity vectors.
- [ ] `cargo bench --features ct` produces a numeric report.
- [ ] `dudect/leak_test` reports `|t| < 4.5` for `10^6` samples.
- [ ] No `unsafe` in `src/ct/`.
- [ ] `#[deny(unsafe_code)]` on `mod ct`.

---

## 8. References

1. Renes, Costello, Batina. *Complete addition formulas for prime order
   elliptic curves.* EUROCRYPT 2016.
2. Bos, Halderman, Heninger, Moore, Naehrig, Wustrow. *Elliptic Curve
   Cryptography in Practice.* Financial Crypto 2014.
3. Reparaz, Balasch, Verbauwhede. *Dude, is my code constant time?*
   DATE 2017 — `dudect`.
4. Erbsen, Philipoom, Gross, Sloan, Chlipala. *Simple High-Level Code
   for Cryptographic Arithmetic.* IEEE S&P 2019 — fiat-crypto.
5. RFC 8032 (ed25519/ed448), RFC 7748 (X25519/X448) — encoding
   conventions copied where applicable.

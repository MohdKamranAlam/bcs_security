# bcs-core-rust

Research reference implementation of the Bismillah Cryptosystem (BCS)
elliptic curve family.

## Curves supported

| Curve | `p` bits | ECDLP security | KDF |
|-------|----------|----------------|------|
| **BCS-256** | 256 | ≈ 2¹²⁸ | HKDF-SHA-256 |
| **BCS-521** | 521 | ≈ 2²⁶⁰ | HKDF-SHA-512 |

Both curves share the same Weierstrass form derived from the Bismillah
master equation `T_A = 17·B² + 5·B + 4 = 6236`:

```text
E: y² = x³ − 2x² + 5x + 4   over   F_p
G  = (0, 2)
cofactor h = 1   (n is prime on both curves)
```

## Important status

This crate is a correctness-oriented Rust reference, **not yet a
constant-time production library**. The twist order on both curves is
composite, so the API enforces **strict public-key validation** before
every ECDH operation. Off-curve / infinity / out-of-range coordinates
are rejected.

Production hardening still requires:

- fixed-width field representation instead of `BigUint`
- constant-time scalar multiplication
- no secret-dependent branches or memory accesses
- audited RNG and key handling
- fuzzing and external cryptographic review

## API at a glance

```rust
use bcs_core_rust::{bcs256, bcs521, hkdf_sha512_64};

// Pick a curve
let curve = bcs521();   // or bcs256()

// Key generation
let alice_sk = curve.generate_private_key();
let alice_pk = curve.public_key(&alice_sk)?;

// Receive Bob's public key over the wire, validate strictly
curve.validate_public_key(&bob_pk)?;

// ECDH
let shared_x = curve.ecdh(&alice_sk, &bob_pk)?;  // 66 bytes on BCS-521

// Derive a 64-byte symmetric key
let key = hkdf_sha512_64(&shared_x, b"BCS-521 salt", b"BCS-521 ECDH v1");
```

## Implemented now

- Generic `Curve` struct holding `p`, `n`, `field_bytes`, generator `G`
- BCS-256 and BCS-521 factories: `bcs256()`, `bcs521()`
- Field arithmetic modulo `p`
- Affine point addition / doubling for curves with `a₂ = -2`
- Double-and-add scalar multiplication
- Strict `validate_public_key` per BCS Twist Validation Policy
- ECDH returning the shared `x` as fixed-length big-endian bytes
- HKDF-SHA-256 (32 bytes) and HKDF-SHA-512 (64 bytes or arbitrary length)
- Unit tests for both curves: generator, invalid-point rejection,
  doubling-vs-addition, `n·G = O`, ECDH agreement, out-of-range rejection
- Backwards-compatible v0.1 top-level functions for BCS-256

## Run

```bash
# all tests (BCS-256 fast, BCS-521 has a few 1–3s 521-bit scalar muls)
cargo test --release

# end-to-end demos
cargo run --release --example bcs256_ecdh_demo
cargo run --release --example bcs521_ecdh_demo
```

## Parameters (frozen)

### BCS-256

```text
p = 75403776646910504885013085564245979049841362888363155420739536990720881516533
n = 75403776646910504885013085564245979049566799732270309665248923838363814402301
G = (0, 2)
```

### BCS-521 (frozen 2026-05-16)

```text
p = 6684878480953803875615041384236581248565144626959181331935475284049641722241859916862100650133594325184551397342003884026080714561754188260987347401802009363
n = 6684878480953803875615041384236581248565144626959181331935475284049641722241859866323302474084432391273141914599968033004535887077643952596722791822942474231
G = (0, 2)
```

Found via parallel Rust + PARI/GP SEA search (166 attempts, ~64 min on
4 cores). See `../bcs-spec/bcs-521.md` for the full audit checklist.

## Roadmap

- Sage independent cardinality proof for BCS-521
- ML-KEM-1024 post-quantum hybrid wire protocol
- Constant-time hardening (when moving toward production)
- External cryptographer review

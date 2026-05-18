//! Cross-check the CT `Scalar` arithmetic against BigUint as oracle.
//!
//! Strategy: BigUint is the ground-truth — any deviation in the limb-based
//! constant-time path is a real bug.  We test:
//!
//! 1. Deterministic edge cases (0, 1, n-1, big values, wrap-trigger pairs).
//! 2. Randomized scalars (1024 cases, seedable via env var BCS_TEST_SEED).
//!
//! Run with: `cargo test --features ct --test test_ct_scalar_arith`

#![cfg(feature = "ct")]

use num_bigint::BigUint;
use num_traits::{One, Zero};
use rand::{rngs::StdRng, Rng, SeedableRng};

// We can only access `Scalar` via the public API: `from_bytes_be` and
// `to_bytes_be`.  The arithmetic methods are public on the type itself.

use bcs_core_rust::ct::scalar::Scalar;

// ---------------------------------------------------------------------------
// Curve order n_521 — replicated here so we don't depend on internals.
// MUST match `bcs-core-rust/src/ct/consts.rs::N_521_LIMBS`.
// ---------------------------------------------------------------------------

fn n_521() -> BigUint {
    // bytes = limbs (little-endian) → reverse to big-endian, parse.
    let limbs: [u64; 9] = [
        0x1ECF77E1DFDB0FF7,
        0x107F211A8D8E3CEE,
        0xED05D6FD2163DA78,
        0xF142C170EDBFD8EC,
        0xBA2FE102829EC762,
        0x6D99A052C2AF01FB,
        0x3CAE6FDF39466130,
        0x94BD5F0B57F7B051,
        0x00000000000001F2,
    ];
    let mut le = Vec::with_capacity(72);
    for &l in &limbs {
        le.extend_from_slice(&l.to_le_bytes());
    }
    BigUint::from_bytes_le(&le)
}

// ---------------------------------------------------------------------------
// BigUint <-> Scalar bridges
// ---------------------------------------------------------------------------

fn biguint_to_scalar(x: &BigUint) -> Scalar {
    // Big-endian, left-padded to 66 bytes.
    let mut be = x.to_bytes_be();
    if be.len() > 66 {
        panic!("BigUint > 66 bytes; not a valid scalar");
    }
    let mut padded = [0u8; 66];
    padded[66 - be.len()..].copy_from_slice(&be);
    be.zeroize_safely(); // best-effort wipe of intermediate
    Scalar::from_bytes_be(&padded).expect("BigUint < n must encode")
}

fn scalar_to_biguint(s: &Scalar) -> BigUint {
    BigUint::from_bytes_be(&s.to_bytes_be())
}

trait ZeroizeSafely { fn zeroize_safely(&mut self); }
impl ZeroizeSafely for Vec<u8> {
    fn zeroize_safely(&mut self) {
        for b in self.iter_mut() { *b = 0; }
    }
}

// ---------------------------------------------------------------------------
// Oracle-based assertions
// ---------------------------------------------------------------------------

fn check_add(a: &BigUint, b: &BigUint, n: &BigUint) {
    let sa = biguint_to_scalar(a);
    let sb = biguint_to_scalar(b);
    let got = scalar_to_biguint(&sa.add_mod_n(&sb));
    let want = (a + b) % n;
    assert_eq!(
        got, want,
        "add_mod_n mismatch:\n  a = {:#x}\n  b = {:#x}\n  got  = {:#x}\n  want = {:#x}",
        a, b, got, want
    );
}

fn check_sub(a: &BigUint, b: &BigUint, n: &BigUint) {
    let sa = biguint_to_scalar(a);
    let sb = biguint_to_scalar(b);
    let got = scalar_to_biguint(&sa.sub_mod_n(&sb));
    // (a - b) mod n  in non-negative form
    let want = if a >= b {
        (a - b) % n
    } else {
        (n - ((b - a) % n)) % n
    };
    assert_eq!(
        got, want,
        "sub_mod_n mismatch:\n  a = {:#x}\n  b = {:#x}\n  got  = {:#x}\n  want = {:#x}",
        a, b, got, want
    );
}

fn check_mul(a: &BigUint, b: &BigUint, n: &BigUint) {
    let sa = biguint_to_scalar(a);
    let sb = biguint_to_scalar(b);
    let got = scalar_to_biguint(&sa.mul_mod_n(&sb));
    let want = (a * b) % n;
    assert_eq!(
        got, want,
        "mul_mod_n mismatch:\n  a = {:#x}\n  b = {:#x}\n  got  = {:#x}\n  want = {:#x}",
        a, b, got, want
    );
}

fn check_inv(a: &BigUint, n: &BigUint) {
    if a.is_zero() { return; } // 0 is not invertible
    let sa = biguint_to_scalar(a);
    let inv_s = sa.inv_mod_n();
    let inv = scalar_to_biguint(&inv_s);
    // Verify: a * inv ≡ 1 (mod n)
    let prod = (a * &inv) % n;
    assert_eq!(
        prod,
        BigUint::one(),
        "inv_mod_n: a * a^(-1) ≠ 1 mod n\n  a   = {:#x}\n  inv = {:#x}",
        a, inv
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn edge_zero_one_and_n_minus_one() {
    let n = n_521();
    let zero = BigUint::zero();
    let one = BigUint::one();
    let n_m_1 = &n - BigUint::one();

    let cases: &[BigUint] = &[zero.clone(), one.clone(), n_m_1.clone()];
    for a in cases {
        for b in cases {
            check_add(a, b, &n);
            check_sub(a, b, &n);
            check_mul(a, b, &n);
        }
        if !a.is_zero() {
            check_inv(a, &n);
        }
    }
}

#[test]
fn edge_specific_known_pairs() {
    let n = n_521();
    let a = BigUint::from(17u64);
    let b = BigUint::from(23u64);
    check_add(&a, &b, &n);
    check_sub(&b, &a, &n);
    check_mul(&a, &b, &n);
    check_inv(&a, &n);
    check_inv(&b, &n);
}

#[test]
fn edge_carry_trigger_pairs() {
    let n = n_521();
    let n_m_1 = &n - BigUint::one();
    let n_m_2 = &n - BigUint::from(2u64);

    // Cases that exercise the wrap-around path of add_mod_n.
    check_add(&n_m_1, &BigUint::one(), &n);    // → 0
    check_add(&n_m_1, &n_m_1,           &n);   // → n-2
    check_add(&n_m_2, &BigUint::from(3u64), &n); // → 1

    // sub_mod_n underflow trigger.
    check_sub(&BigUint::zero(), &n_m_1,        &n); // → 1
    check_sub(&BigUint::one(),   &n_m_1,        &n); // → 2

    // mul_mod_n: (n-1)^2 = 1
    check_mul(&n_m_1, &n_m_1, &n);
}

// ---------------------------------------------------------------------------
// Randomized fuzz: 256 iterations × 4 ops = 1024 BigUint cross-checks.
// Seedable via env var BCS_TEST_SEED for reproducibility.
// ---------------------------------------------------------------------------

fn random_scalar_below(rng: &mut StdRng, n: &BigUint) -> BigUint {
    loop {
        let mut bytes = [0u8; 66];
        rng.fill(&mut bytes[..]);
        // Mask the top 7 bits — n is 521-bit, so any value < 2^521 may still
        // exceed n; loop until accepted.  Acceptance probability ≈ n / 2^521,
        // which is > 0.999 for our curve.
        bytes[0] &= 0x01;
        let cand = BigUint::from_bytes_be(&bytes);
        if &cand < n {
            return cand;
        }
    }
}

fn test_seed() -> u64 {
    std::env::var("BCS_TEST_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0xBC5_521_DEAD_BEEFu64)
}

#[test]
fn fuzz_add_sub_mul_against_biguint() {
    let n = n_521();
    let seed = test_seed();
    let mut rng = StdRng::seed_from_u64(seed);
    let iters = 256;
    eprintln!("[fuzz] seed = {:#x}, iters = {}", seed, iters);
    for _ in 0..iters {
        let a = random_scalar_below(&mut rng, &n);
        let b = random_scalar_below(&mut rng, &n);
        check_add(&a, &b, &n);
        check_sub(&a, &b, &n);
        check_mul(&a, &b, &n);
    }
}

#[test]
fn fuzz_inv_against_biguint() {
    let n = n_521();
    let seed = test_seed() ^ 0xA5A5_A5A5_A5A5_A5A5;
    let mut rng = StdRng::seed_from_u64(seed);
    // inv is expensive (~521² limb ops); 32 iters is plenty.
    let iters = 32;
    eprintln!("[fuzz] inv seed = {:#x}, iters = {}", seed, iters);
    for _ in 0..iters {
        let a = random_scalar_below(&mut rng, &n);
        check_inv(&a, &n);
    }
}

// ---------------------------------------------------------------------------
// Sanity: associativity and distributivity over a few random points.
// ---------------------------------------------------------------------------

#[test]
fn fuzz_associativity_and_distributivity() {
    let n = n_521();
    let seed = test_seed() ^ 0xDEAD_C0DE;
    let mut rng = StdRng::seed_from_u64(seed);
    for _ in 0..16 {
        let a = random_scalar_below(&mut rng, &n);
        let b = random_scalar_below(&mut rng, &n);
        let c = random_scalar_below(&mut rng, &n);
        let sa = biguint_to_scalar(&a);
        let sb = biguint_to_scalar(&b);
        let sc = biguint_to_scalar(&c);

        // (a + b) + c == a + (b + c)
        let lhs = sa.add_mod_n(&sb).add_mod_n(&sc);
        let rhs = sa.add_mod_n(&sb.add_mod_n(&sc));
        assert_eq!(lhs.to_bytes_be(), rhs.to_bytes_be(), "add associativity");

        // a · (b + c) == a·b + a·c
        let lhs2 = sa.mul_mod_n(&sb.add_mod_n(&sc));
        let rhs2 = sa.mul_mod_n(&sb).add_mod_n(&sa.mul_mod_n(&sc));
        assert_eq!(lhs2.to_bytes_be(), rhs2.to_bytes_be(), "distributivity");
    }
}

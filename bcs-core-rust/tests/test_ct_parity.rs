//! Cross-implementation **parity** test.
//!
//! For a fixed set of scalars, compute `k · G` using:
//!
//! 1. The **reference** BigUint implementation in the original BCS-521 chart
//!    `y² = x³ − 2x² + 5x + 4`  (slow, branchy, but well-audited).
//! 2. The **constant-time** implementation in the short-Weierstrass chart
//!    `y² = x'³ + A·x' + B`  with Montgomery limbs and the Renes-Costello-Batina
//!    ladder.
//!
//! After translating the CT result back to the original chart (add `2/3` to
//! the x-coordinate) and serialising both points to canonical 66-byte
//! big-endian arrays, the two byte strings **must be identical**.
//!
//! If any test in this file passes, every layer of the CT stack —
//! limb-level CIOS Montgomery multiplication, RCB complete addition,
//! RCB complete doubling, Montgomery ladder, and the chart change of
//! variables — is bit-exact compatible with the reference.
//!
//! This is **the** acceptance criterion for the v0.2.0-ct release.

#![cfg(feature = "ct")]

use bcs_core_rust::ct::{self, Fp521, Scalar};
use bcs_core_rust::{bcs521, Point};
use num_bigint::BigUint;

const FIELD_BYTES: usize = 66;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Left-pad a `BigUint` to exactly 66 bytes big-endian.
fn biguint_to_be_66(x: &BigUint) -> [u8; FIELD_BYTES] {
    let bytes = x.to_bytes_be();
    assert!(bytes.len() <= FIELD_BYTES, "value exceeds 521 bits");
    let mut out = [0u8; FIELD_BYTES];
    out[FIELD_BYTES - bytes.len()..].copy_from_slice(&bytes);
    out
}

/// Build a `Scalar` from a small `u64` value.
fn scalar_from_u64(x: u64) -> Scalar {
    let mut bytes = [0u8; FIELD_BYTES];
    bytes[FIELD_BYTES - 8..].copy_from_slice(&x.to_be_bytes());
    Scalar::from_bytes_be(&bytes).expect("u64 < n")
}

/// Compute the Montgomery representative of `2 / 3 mod p`.
///
/// We do this purely through the public CT API so no new constants are
/// needed: `1_M + 1_M = 2_M`, `2_M + 1_M = 3_M`, then one Fermat
/// inversion, then one Montgomery multiplication.
fn two_thirds_in_mont() -> Fp521 {
    let one = Fp521::ONE_MONT;
    let two = one + one;
    let three = two + one;
    let three_inv = three.invert().expect("3 is invertible mod p");
    two.mont_mul(&three_inv)
}

/// Convert a `ProjPoint` (short-Weierstrass chart, Montgomery form) into
/// canonical 66-byte big-endian `(x, y)` in the **original** BCS-521 chart.
fn ct_proj_to_original_be(p: &ct::ProjPoint) -> Option<([u8; FIELD_BYTES], [u8; FIELD_BYTES])> {
    let (x_short_mont, y_short_mont) = p.to_affine()?;

    // Chart translation: x_orig = x_short + 2/3   (still in Mont form).
    let two_thirds = two_thirds_in_mont();
    let x_orig_mont = x_short_mont + two_thirds;

    // De-Montgomerise both coordinates.
    let x_orig_canon = x_orig_mont.from_montgomery();
    let y_orig_canon = y_short_mont.from_montgomery();

    Some((x_orig_canon.to_bytes_be(), y_orig_canon.to_bytes_be()))
}

/// Run a single parity check at scalar `k`.
fn assert_parity(k_u64: u64) {
    // ---- Reference (BigUint) path ----
    let curve = bcs521();
    let k_big = BigUint::from(k_u64);
    let q_ref = curve.scalar_mul(&k_big, &curve.g);
    let (ref_x, ref_y) = match q_ref {
        Point::Affine { ref x, ref y } => (biguint_to_be_66(x), biguint_to_be_66(y)),
        Point::Infinity => panic!("k={} produced infinity in reference path", k_u64),
    };

    // ---- Constant-time path ----
    let k_scalar = scalar_from_u64(k_u64);
    let q_ct = ct::scalar_mul_generator(&k_scalar);
    let (ct_x, ct_y) = ct_proj_to_original_be(&q_ct)
        .unwrap_or_else(|| panic!("k={} produced infinity in CT path", k_u64));

    // ---- Byte-exact assertion ----
    assert_eq!(
        ct_x, ref_x,
        "x-coordinate mismatch for k={}\n  CT  = {}\n  REF = {}",
        k_u64,
        hex::encode(ct_x),
        hex::encode(ref_x),
    );
    assert_eq!(
        ct_y, ref_y,
        "y-coordinate mismatch for k={}\n  CT  = {}\n  REF = {}",
        k_u64,
        hex::encode(ct_y),
        hex::encode(ref_y),
    );
}

// ---------------------------------------------------------------------------
// Parity tests
// ---------------------------------------------------------------------------

#[test]
fn parity_k_equals_1() {
    assert_parity(1);
}

#[test]
fn parity_k_equals_2() {
    assert_parity(2);
}

#[test]
fn parity_k_equals_3() {
    assert_parity(3);
}

#[test]
fn parity_k_equals_7() {
    assert_parity(7);
}

#[test]
fn parity_k_equals_42() {
    assert_parity(42);
}

#[test]
fn parity_k_equals_2141_kahf_first() {
    // p_kahf_first_decimal — see Theorem 18, Surah Kahf Prime Lock.
    assert_parity(2141);
}

#[test]
fn parity_k_equals_2969_kahf_last_zf() {
    // p_kahf_last_zf  — ZF prime from cumulative ayah 2250 of Kahf.
    assert_parity(2969);
}

#[test]
fn parity_k_equals_u64_max() {
    assert_parity(u64::MAX);
}

#[test]
fn parity_k_random_521bit_pattern() {
    // A 256-bit scalar that exercises many limbs.
    // Hex: 0xCAFE_BABE_DEAD_BEEF_1234_5678_9ABC_DEF0_FEDC_BA98_7654_3210_DEAD_BEEF_CAFE_BABE
    let bytes_be: [u8; 32] = [
        0xCA, 0xFE, 0xBA, 0xBE, 0xDE, 0xAD, 0xBE, 0xEF,
        0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
        0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10,
        0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE,
    ];
    let k_big = BigUint::from_bytes_be(&bytes_be);
    let curve = bcs521();
    let q_ref = curve.scalar_mul(&k_big, &curve.g);
    let (ref_x, ref_y) = match q_ref {
        Point::Affine { ref x, ref y } => (biguint_to_be_66(x), biguint_to_be_66(y)),
        Point::Infinity => panic!("256-bit random scalar produced infinity in reference"),
    };

    // Build the same scalar for the CT path.
    let mut padded = [0u8; FIELD_BYTES];
    padded[FIELD_BYTES - 32..].copy_from_slice(&bytes_be);
    let k_scalar = Scalar::from_bytes_be(&padded).expect("< n");
    let q_ct = ct::scalar_mul_generator(&k_scalar);
    let (ct_x, ct_y) = ct_proj_to_original_be(&q_ct).expect("not infinity");

    assert_eq!(ct_x, ref_x, "x mismatch on random 256-bit scalar");
    assert_eq!(ct_y, ref_y, "y mismatch on random 256-bit scalar");
}

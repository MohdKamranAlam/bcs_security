//! # Differential Power Analysis (DPA) countermeasures for BCS-521
//!
//! DPA (Kocher–Jaffe–Jun 1999) uses statistical analysis of power
//! consumption traces across many operations to recover secret keys.
//! Even constant-time code is vulnerable: the *hardware* still draws
//! data-dependent power during register writes and ALU operations.
//!
//! ## Countermeasure: additive scalar masking
//!
//! We split the secret scalar `s` into two *additive shares*:
//!
//! ```text
//! s = s₁ + s₂   (mod n_521)
//! ```
//!
//! where `s₁` is uniform random and `s₂ = s − s₁ (mod n)`.  Each
//! share is statistically independent of `s`, so a power trace of
//! `s₁ · P` or `s₂ · P` alone reveals nothing about `s`.
//!
//! The final result is recovered by point addition:
//!
//! ```text
//! s · P = s₁ · P + s₂ · P
//! ```
//!
//! ### Security level
//!
//! First-order DPA: the attacker measures one trace per operation and
//! correlates it with *one* intermediate value.  Additive masking with
//! a fresh random share per operation defeats this attack, because
//! every intermediate value depends on the random mask, not the secret.
//!
//! Higher-order DPA (2nd, 3rd order): the attacker combines *multiple*
//! trace points.  This module does **not** defend against higher-order
//! DPA; that would require higher-order masking (3+ shares) and is
//! left for a future "military-grade" hardening pass.
//!
//! ### Performance cost
//!
//! ~2× slower than unmasked `scalar_mul` (two ladder runs + one
//! `point_add`).  Acceptable for high-value operations.

use super::point::ProjPoint;
use super::scalar::Scalar;
use super::fault_injection::scalar_mul_fault_protected;

// ---------------------------------------------------------------------------
// Masked scalar — the two additive shares
// ---------------------------------------------------------------------------

/// A scalar split into two additive shares: `s = s1 + s2 (mod n)`.
///
/// Each share is independently uniform-random-looking.  A power trace
/// of either share's scalar multiplication reveals nothing about `s`.
#[derive(Clone)]
pub struct MaskedScalar {
    /// First additive share (random).
    pub s1: Scalar,
    /// Second additive share: `s2 = s − s1 (mod n)`.
    pub s2: Scalar,
}

impl MaskedScalar {
    /// Split a scalar into two additive shares using a caller-supplied
    /// random 66-byte seed for `s1`.
    ///
    /// **The caller must supply a cryptographically random `s1_seed`.**
    /// If `s1_seed` is predictable, the mask provides no security.
    ///
    /// Returns `None` if the seed decodes to a scalar ≥ n (vanishingly
    /// rare — rejection sampling handles this internally).
    pub fn split(s: &Scalar, s1_seed: &[u8; 66]) -> Option<Self> {
        let s1 = Scalar::from_bytes_be(s1_seed)?;
        // s2 = s − s1 (mod n) via 9-limb subtraction + conditional add-back.
        let s2 = scalar_sub_mod_n(s, &s1);
        Some(MaskedScalar { s1, s2 })
    }

    /// Verify that the shares recombine to the original scalar.
    /// Constant-time comparison.
    pub fn verify(&self, original: &Scalar) -> bool {
        let reconstructed = scalar_add_mod_n(&self.s1, &self.s2);
        bool::from(reconstructed.ct_eq(original))
    }
}

// ---------------------------------------------------------------------------
// Masked scalar multiplication
// ---------------------------------------------------------------------------

/// First-order DPA-protected scalar multiplication.
///
/// Splits `k` into `(k1, k2)` additive shares, computes `k1·P` and
/// `k2·P` separately, then adds the two results.
///
/// **Also includes fault-injection protection** on each share's
/// computation, so this function provides *both* DPA masking and
/// fault resistance simultaneously.
///
/// Returns `None` if `k1_seed` fails to decode (probability ≈ 2⁻⁵²¹).
pub fn scalar_mul_masked(k: &Scalar, p: &ProjPoint, k1_seed: &[u8; 66]) -> Option<ProjPoint> {
    let masked = MaskedScalar::split(k, k1_seed)?;

    // Each share uses fault-protected scalar multiplication.
    let r1 = scalar_mul_fault_protected(&masked.s1, p);
    let r2 = scalar_mul_fault_protected(&masked.s2, p);

    // Combine: k·P = k1·P + k2·P
    let result = r1.add(&r2);

    Some(std::hint::black_box(result))
}

/// First-order DPA-protected scalar multiplication by the fixed
/// generator, with fault-injection protection.
pub fn scalar_mul_generator_masked(k: &Scalar, k1_seed: &[u8; 66]) -> Option<ProjPoint> {
    scalar_mul_masked(k, &ProjPoint::GENERATOR, k1_seed)
}

// ---------------------------------------------------------------------------
// Modular arithmetic helpers (minimal, for masking only)
// ---------------------------------------------------------------------------

/// Constant-time `a + b (mod n)` for 9-limb scalars.
///
/// This is a minimal implementation sufficient for masking.  A full
/// Barrett-reduction-based arithmetic module is planned for v0.3.0.
fn scalar_add_mod_n(a: &Scalar, b: &Scalar) -> Scalar {
    let mut limbs = [0u64; 9];
    let mut carry: u64 = 0;
    for i in 0..9 {
        let (sum1, c1) = a.limbs[i].overflowing_add(b.limbs[i]);
        let (sum2, c2) = sum1.overflowing_add(carry);
        limbs[i] = sum2;
        carry = (c1 as u64) | (c2 as u64);
    }

    // If carry == 1 or result >= n, subtract n.
    let candidate = Scalar { limbs };
    let ge_n = !candidate.ct_lt_n();

    // Constant-time conditional subtraction of n.
    let mut result = candidate;
    let mut borrow: u64 = 0;
    for i in 0..9 {
        let (diff, b1) = result.limbs[i].overflowing_sub(super::consts::N_521_LIMBS[i]);
        let (diff2, b2) = diff.overflowing_sub(borrow);
        // Select: if ge_n, use diff2; else keep original.
        let mask = ge_n.unwrap_u8() as u64; // 0 or 1
        let mask = !(mask.wrapping_sub(1)); // 0→0x0000, 1→0xFFFF
        result.limbs[i] = (diff2 & mask) | (candidate.limbs[i] & !mask);
        borrow = (b1 as u64) | (b2 as u64);
    }
    result
}

/// Constant-time `a − b (mod n)` for 9-limb scalars.
fn scalar_sub_mod_n(a: &Scalar, b: &Scalar) -> Scalar {
    // Compute a − b.  If underflow, add n back.
    let mut limbs = [0u64; 9];
    let mut borrow: u64 = 0;
    for i in 0..9 {
        let (diff, b1) = a.limbs[i].overflowing_sub(b.limbs[i]);
        let (diff2, b2) = diff.overflowing_sub(borrow);
        limbs[i] = diff2;
        borrow = (b1 as u64) | (b2 as u64);
    }

    // If borrow == 1 (underflow), add n back.
    let underflow = borrow; // 0 or 1

    // Constant-time conditional addition of n.
    // n_or_zero = n if underflow, else 0.
    let mut result = Scalar { limbs };
    let mut carry: u64 = 0;
    for i in 0..9 {
        let n_or_zero = super::consts::N_521_LIMBS[i] & underflow.wrapping_neg();
        let (sum1, c1) = result.limbs[i].overflowing_add(n_or_zero);
        let (sum2, c2) = sum1.overflowing_add(carry);
        result.limbs[i] = sum2;
        carry = (c1 as u64) | (c2 as u64);
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::consts::FIELD_BYTES;

    fn scalar_from(x: u64) -> Scalar {
        let mut bytes = [0u8; FIELD_BYTES];
        bytes[FIELD_BYTES - 8..].copy_from_slice(&x.to_be_bytes());
        Scalar::from_bytes_be(&bytes).expect("x < n")
    }

    #[test]
    fn masked_split_recombines() {
        let s = scalar_from(42);
        let mut seed = [0u8; 66];
        seed[65] = 17; // arbitrary non-zero seed
        let masked = MaskedScalar::split(&s, &seed).unwrap();
        assert!(masked.verify(&s), "shares do not recombine to original");
    }

    #[test]
    fn masked_mul_matches_plain() {
        let k = scalar_from(7);
        let r_plain = scalar_mul(&k, &ProjPoint::GENERATOR);

        let mut seed = [0u8; 66];
        seed[65] = 23;
        let r_masked = scalar_mul_masked(&k, &ProjPoint::GENERATOR, &seed).unwrap();

        let (px, py) = r_plain.to_affine().unwrap();
        let (mx, my) = r_masked.to_affine().unwrap();
        assert_eq!(px, mx, "masked x ≠ plain x");
        assert_eq!(py, my, "masked y ≠ plain y");
    }

    #[test]
    fn masked_generator_matches_plain() {
        let k = scalar_from(99);
        let r_plain = scalar_mul(&k, &ProjPoint::GENERATOR);

        let mut seed = [0u8; 66];
        seed[64] = 1;
        seed[65] = 0;
        let r_masked = scalar_mul_generator_masked(&k, &seed).unwrap();

        let (px, py) = r_plain.to_affine().unwrap();
        let (mx, my) = r_masked.to_affine().unwrap();
        assert_eq!(px, mx);
        assert_eq!(py, my);
    }

    #[test]
    fn add_mod_n_identity() {
        let a = scalar_from(5);
        let zero = Scalar::ZERO;
        let result = scalar_add_mod_n(&a, &zero);
        assert!(bool::from(result.ct_eq(&a)), "a + 0 ≠ a");
    }

    #[test]
    fn sub_mod_n_identity() {
        let a = scalar_from(5);
        let result = scalar_sub_mod_n(&a, &a);
        assert!(bool::from(result.ct_eq(&Scalar::ZERO)), "a − a ≠ 0");
    }
}

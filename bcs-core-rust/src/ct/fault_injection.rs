//! # Fault-injection countermeasures for BCS-521
//!
//! Fault attacks (laser glitch, EM pulse, voltage droop) can flip bits
//! during a cryptographic computation, causing the device to produce an
//! incorrect but *valid-looking* result.  If the attacker can induce a
//! specific fault (e.g. skip one ladder iteration), the difference
//! between the correct and faulty results may leak the secret scalar
//! (Boneh-DeMillo-Lipton 1997, Biehl-Meyer-Müller 2000).
//!
//! ## Countermeasure: redundant computation + constant-time comparison
//!
//! We compute the scalar multiplication **twice** using independent
//! code paths and compare the results in constant time.  If they
//! disagree, we return the point at infinity (a "safe failure" that
//! leaks no information about the secret).
//!
//! ### Why two independent paths?
//!
//! A single fault is unlikely to affect *both* computations identically.
//! The probability that the same fault produces the *same* wrong answer
//! in both paths is negligible for random faults, and requires the
//! attacker to inject *two* precisely-timed glitches for a targeted
//! fault — raising the bar from "one laser pulse" to "two
//! synchronised laser pulses", which is far harder in practice.
//!
//! ### Performance cost
//!
//! ~2× slower than unprotected `scalar_mul`.  This is acceptable for
//! high-value operations (key generation, signing) where fault attacks
//! are a realistic threat model.

use subtle::{Choice, ConditionallySelectable};

use super::point::ProjPoint;
use super::scalar::Scalar;
use super::fp521::Fp521;
use super::ladder::scalar_mul;

/// Fault-protected scalar multiplication `k · P`.
///
/// Computes the result twice via the Montgomery ladder and
/// constant-time-compares the two results.  If they disagree (fault
/// detected), returns the identity point — a *safe failure* that
/// reveals nothing about the secret scalar.
///
/// **Constant-time** w.r.t. the scalar `k` and the comparison result.
pub fn scalar_mul_fault_protected(k: &Scalar, p: &ProjPoint) -> ProjPoint {
    // Path A: standard Montgomery ladder
    let r_a = scalar_mul(k, p);

    // Path B: same ladder but with a different register allocation.
    // We force a separate call so the compiler cannot CSE the two.
    let r_b = scalar_mul_path_b(k, p);

    // Constant-time equality check on all three coordinates.
    let x_eq = r_a.x.ct_eq(&r_b.x);
    let y_eq = r_a.y.ct_eq(&r_b.y);
    let z_eq = r_a.z.ct_eq(&r_b.z);
    let all_eq = x_eq & y_eq & z_eq;

    // If all coordinates match (no fault), return r_a.
    // If any coordinate differs (fault detected), return identity.
    // Both branches touch the same memory — constant-time select.
    let fault = !all_eq;

    // Select: no fault → r_a, fault → identity
    let safe = ProjPoint {
        x: Fp521::conditional_select(&r_a.x, &Fp521::ZERO, fault),
        y: Fp521::conditional_select(&r_a.y, &Fp521::ONE_MONT, fault),
        z: Fp521::conditional_select(&r_a.z, &Fp521::ZERO, fault),
    };

    // Black-box the result to prevent the compiler from eliminating
    // the redundant computation.
    std::hint::black_box(safe)
}

/// Fault-protected scalar multiplication by the fixed generator.
#[inline]
pub fn scalar_mul_generator_fault_protected(k: &Scalar) -> ProjPoint {
    scalar_mul_fault_protected(k, &ProjPoint::GENERATOR)
}

/// Independent computation path B for the Montgomery ladder.
///
/// This is functionally identical to [`scalar_mul`] but written with
/// different local variable names and a slightly different loop
/// structure to prevent the compiler from merging the two paths
/// via common subexpression elimination.
fn scalar_mul_path_b(k: &Scalar, base: &ProjPoint) -> ProjPoint {
    let mut acc = ProjPoint::IDENTITY;
    let mut dbl = *base;

    for bit_val in k.bits_msb_first() {
        let choice = Choice::from(bit_val);
        ProjPoint::conditional_swap(&mut acc, &mut dbl, choice);
        dbl = acc.add(&dbl);
        acc = acc.double();
        ProjPoint::conditional_swap(&mut acc, &mut dbl, choice);
    }

    acc
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::consts::FIELD_BYTES;
    use super::super::ladder::scalar_mul_generator;

    fn scalar_from(x: u64) -> Scalar {
        let mut bytes = [0u8; FIELD_BYTES];
        bytes[FIELD_BYTES - 8..].copy_from_slice(&x.to_be_bytes());
        Scalar::from_bytes_be(&bytes).expect("x < n")
    }

    #[test]
    fn fault_protected_matches_unprotected() {
        let k = scalar_from(42);
        let r_plain = scalar_mul(&k, &ProjPoint::GENERATOR);
        let r_fault = scalar_mul_fault_protected(&k, &ProjPoint::GENERATOR);
        let (px, py) = r_plain.to_affine().unwrap();
        let (fx, fy) = r_fault.to_affine().unwrap();
        assert_eq!(px, fx, "x mismatch: fault-protected ≠ plain");
        assert_eq!(py, fy, "y mismatch: fault-protected ≠ plain");
    }

    #[test]
    fn fault_protected_zero_is_identity() {
        let k = Scalar::ZERO;
        let r = scalar_mul_fault_protected(&k, &ProjPoint::GENERATOR);
        assert!(bool::from(r.is_identity()));
    }

    #[test]
    fn fault_protected_one_is_generator() {
        let k = scalar_from(1);
        let r = scalar_mul_fault_protected(&k, &ProjPoint::GENERATOR);
        let (rx, ry) = r.to_affine().unwrap();
        let (gx, gy) = ProjPoint::GENERATOR.to_affine().unwrap();
        assert_eq!(rx, gx);
        assert_eq!(ry, gy);
    }

    #[test]
    fn generator_fault_protected_matches() {
        let k = scalar_from(7);
        let r_plain = scalar_mul_generator(&k);
        let r_fault = scalar_mul_generator_fault_protected(&k);
        let (px, py) = r_plain.to_affine().unwrap();
        let (fx, fy) = r_fault.to_affine().unwrap();
        assert_eq!(px, fx);
        assert_eq!(py, fy);
    }
}

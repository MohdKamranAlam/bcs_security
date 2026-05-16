//! Constant-time scalar multiplication via the Montgomery ladder.
//!
//! The Montgomery ladder is the canonical algorithm for
//! side-channel-clean elliptic-curve scalar multiplication.  It runs
//! in **exactly `FIELD_BITS = 521` iterations** regardless of the
//! scalar value, and every iteration performs the same two point
//! operations — one addition and one doubling — interleaved with two
//! constant-time conditional swaps.
//!
//! ## Invariant
//!
//! Throughout the loop:  `r1 − r0 = P`.  After consuming `i` bits of
//! the scalar `k` (MSB-first), `r0 = ⌊k_i⌋ · P` and
//! `r1 = (⌊k_i⌋ + 1) · P`, where `k_i` is the prefix of length `i`.
//!
//! At the end of all 521 iterations, `r0 = k · P`.
//!
//! ## Why this is constant-time
//!
//! 1. The loop count is a public constant (521).
//! 2. The two `conditional_swap` calls always touch every limb of
//!    both points.
//! 3. The `add` and `double` use Renes-Costello-Batina complete
//!    formulas — no branches, no special cases.
//! 4. There is no `if` on any secret bit; the only place a secret bit
//!    is consulted is inside `Fp521::conditional_swap`, which is itself
//!    a bit-blend.

use subtle::Choice;

use super::point::ProjPoint;
use super::scalar::Scalar;

/// Constant-time scalar multiplication `k · P` via the Montgomery ladder.
///
/// **Always** runs in 521 iterations.  No secret-dependent branches.
pub fn scalar_mul(k: &Scalar, p: &ProjPoint) -> ProjPoint {
    let mut r0 = ProjPoint::IDENTITY;
    let mut r1 = *p;

    for bit in k.bits_msb_first() {
        let b = Choice::from(bit);
        ProjPoint::conditional_swap(&mut r0, &mut r1, b);
        r1 = r0.add(&r1);
        r0 = r0.double();
        ProjPoint::conditional_swap(&mut r0, &mut r1, b);
    }
    r0
}

/// Constant-time scalar multiplication by the fixed generator `G'`
/// (short-Weierstrass chart).
///
/// Today this is a thin wrapper around [`scalar_mul`].  In a future
/// release we will replace it with a windowed comb that pre-computes
/// `2^w · G'` for small `w`, gaining a 4-8× speed-up for fixed-base
/// operations (key-pair generation, signing).
#[inline]
pub fn scalar_mul_generator(k: &Scalar) -> ProjPoint {
    scalar_mul(k, &ProjPoint::GENERATOR)
}

// ---------------------------------------------------------------
// Tests
// ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::consts::FIELD_BYTES;

    /// Build a scalar holding the small integer `x`.
    fn scalar_from(x: u64) -> Scalar {
        let mut bytes = [0u8; FIELD_BYTES];
        bytes[FIELD_BYTES - 8..].copy_from_slice(&x.to_be_bytes());
        Scalar::from_bytes_be(&bytes).expect("x < n")
    }

    #[test]
    fn ladder_zero_times_g_is_identity() {
        let k = Scalar::ZERO;
        let r = scalar_mul(&k, &ProjPoint::GENERATOR);
        assert!(bool::from(r.is_identity()));
    }

    #[test]
    fn ladder_one_times_g_is_g() {
        let k = scalar_from(1);
        let r = scalar_mul(&k, &ProjPoint::GENERATOR);
        let (rx, ry) = r.to_affine().expect("1·G not at infinity");
        let (gx, gy) = ProjPoint::GENERATOR.to_affine().unwrap();
        assert_eq!(rx, gx);
        assert_eq!(ry, gy);
    }

    #[test]
    fn ladder_two_times_g_matches_double() {
        let k    = scalar_from(2);
        let two_g = scalar_mul(&k, &ProjPoint::GENERATOR);
        let dbl  = ProjPoint::GENERATOR.double();
        let (lx, ly) = two_g.to_affine().expect("2·G not at infinity");
        let (rx, ry) = dbl.to_affine().expect("double(G) not at infinity");
        assert_eq!(lx, rx);
        assert_eq!(ly, ry);
    }

    #[test]
    fn ladder_three_times_g_matches_add_chain() {
        let k    = scalar_from(3);
        let three_g = scalar_mul(&k, &ProjPoint::GENERATOR);
        // 3G = 2G + G
        let chain = ProjPoint::GENERATOR.double().add(&ProjPoint::GENERATOR);
        let (lx, ly) = three_g.to_affine().expect("3·G not at infinity");
        let (rx, ry) = chain.to_affine().expect("2G+G not at infinity");
        assert_eq!(lx, rx);
        assert_eq!(ly, ry);
    }

    #[test]
    fn ladder_seven_times_g_matches_add_chain() {
        let k   = scalar_from(7);
        let l7g = scalar_mul(&k, &ProjPoint::GENERATOR);
        // 7G = 4G + 2G + G  =  double(double(G)) + double(G) + G
        let g  = ProjPoint::GENERATOR;
        let g2 = g.double();
        let g4 = g2.double();
        let r  = g4.add(&g2).add(&g);
        let (lx, ly) = l7g.to_affine().expect("7·G not at infinity");
        let (rx, ry) = r.to_affine().expect("chain not at infinity");
        assert_eq!(lx, rx);
        assert_eq!(ly, ry);
    }
}

//! Constant-time elliptic-curve point on the BCS-521 curve
//! (short-Weierstrass chart  `y² = x'³ + A·x' + B`).
//!
//! We use **homogeneous projective coordinates** `(X : Y : Z)`, where
//! the affine point is `(X / Z,  Y / Z)` and the point at infinity is
//! `(0 : 1 : 0)`.
//!
//! Addition and doubling use the **complete formulas** of
//! Renes–Costello–Batina, EUROCRYPT 2016 (Algorithms 1 and 3).  They
//! are *strongly unified*: the same code handles `P + Q`, `P + P`,
//! `P + O`, `O + Q`, and `P + (-P)` without any data-dependent
//! branches.
//!
//! ## Status
//!
//! - [x] type, identity, generator, conditional_swap
//! - [x] complete projective `add`  (RCB 2016 Algorithm 1)
//! - [x] complete projective `double` (RCB 2016 Algorithm 3)
//! - [x] `to_affine` via Montgomery inversion of `Z`
//! - [x] `is_identity` constant-time
//! - [x] Montgomery ladder lives in `ladder.rs`
//!
//! ## Coordinate-chart reminder
//!
//! The Rust API exposes points in the **original** BCS chart
//! `y² = x³ − 2x² + 5x + 4`.  Internally we operate in the
//! short-Weierstrass chart obtained by substituting `x = x' + 2/3`.
//! Conversion between the two is purely additive on the x-coordinate
//! and so leaks no secret information; it is performed by the
//! free functions `to_short` / `from_short`.

use subtle::Choice;

use super::consts::{
    SHORT_A_MONT_LIMBS, SHORT_B3_MONT_LIMBS, SHORT_GX_MONT_LIMBS, SHORT_GY_MONT_LIMBS,
};
use super::fp521::Fp521;

/// A point on the BCS-521 curve in homogeneous projective coordinates
/// `(X : Y : Z)` over `F_p`, with every coordinate already in
/// Montgomery form.
#[derive(Clone, Copy, Debug)]
pub struct ProjPoint {
    pub(crate) x: Fp521,
    pub(crate) y: Fp521,
    pub(crate) z: Fp521,
}

impl ProjPoint {
    // ---------------------------------------------------------------
    // Constants
    // ---------------------------------------------------------------

    /// The point at infinity  `O = (0 : 1 : 0)`.
    pub const IDENTITY: Self = Self {
        x: Fp521::ZERO,
        y: Fp521::ONE_MONT,
        z: Fp521::ZERO,
    };

    /// The generator `G' = (−2/3 : 2 : 1)` of the short-Weierstrass chart.
    ///
    /// Note: this is *not* the same x-coordinate as the user-facing
    /// generator `G = (0, 2)`.  Internal CT code uses `G'`; the public
    /// API will re-translate to `G` on (de)serialization.
    pub const GENERATOR: Self = Self {
        x: Fp521 { limbs: SHORT_GX_MONT_LIMBS },
        y: Fp521 { limbs: SHORT_GY_MONT_LIMBS },
        z: Fp521::ONE_MONT,
    };

    // ---------------------------------------------------------------
    // Predicates
    // ---------------------------------------------------------------

    /// Constant-time test: is this the point at infinity?
    /// In homogeneous projective coordinates: `O ⇔ Z = 0`.
    #[inline]
    pub fn is_identity(&self) -> Choice {
        self.z.ct_eq(&Fp521::ZERO)
    }

    /// Swap two projective points if `c == 1`, else leave them.
    /// Touches every limb of every coordinate; constant-time.
    #[inline]
    pub fn conditional_swap(a: &mut Self, b: &mut Self, c: Choice) {
        Fp521::conditional_swap(&mut a.x, &mut b.x, c);
        Fp521::conditional_swap(&mut a.y, &mut b.y, c);
        Fp521::conditional_swap(&mut a.z, &mut b.z, c);
    }

    // ===============================================================
    // Renes-Costello-Batina, EUROCRYPT 2016, Algorithm 1
    //                            complete addition for any a, b
    // ===============================================================
    //
    // Cost: 12 M + 2 amul + 1 b3mul + 23 A.  In our setting `a` and
    // `b3` are full 9-limb constants, so amul = b3mul = mont_mul.
    // Total: 15 Mont-muls per add.
    //
    // The formula is fully unified — no branches, no special cases.

    /// Complete point addition `self + rhs`.
    #[allow(clippy::many_single_char_names)]
    pub fn add(&self, rhs: &Self) -> Self {
        let a  = Fp521 { limbs: SHORT_A_MONT_LIMBS };
        let b3 = Fp521 { limbs: SHORT_B3_MONT_LIMBS };

        let (x1, y1, z1) = (self.x, self.y, self.z);
        let (x2, y2, z2) = (rhs.x,  rhs.y,  rhs.z);

        // Lines numbered as in RCB 2016 Algorithm 1.
        let t0 = x1.mont_mul(&x2);                // 1.
        let t1 = y1.mont_mul(&y2);                // 2.
        let t2 = z1.mont_mul(&z2);                // 3.
        let t3 = (x1 + y1).mont_mul(&(x2 + y2));  // 4-6.
        let t3 = t3 - (t0 + t1);                  // 7-8.

        let t4 = (x1 + z1).mont_mul(&(x2 + z2));  // 9-11.
        let t4 = t4 - (t0 + t2);                  // 12-13.

        let t5 = (y1 + z1).mont_mul(&(y2 + z2));  // 14-16.
        let x3_tmp = t1 + t2;                     // 17.
        let t5 = t5 - x3_tmp;                     // 18.

        let z3 = a.mont_mul(&t4);                 // 19.
        let x3 = b3.mont_mul(&t2);                // 20.
        let z3 = x3 + z3;                         // 21.
        let x3 = t1 - z3;                         // 22.
        let z3 = t1 + z3;                         // 23.
        let y3 = x3.mont_mul(&z3);                // 24.

        let t1_new = (t0 + t0) + t0;              // 25-26.
        let t2_new = a.mont_mul(&t2);             // 27.
        let t4_new = b3.mont_mul(&t4);            // 28.
        let t1_new = t1_new + t2_new;             // 29.
        let t2_new2 = t0 - t2_new;                // 30.
        let t2_new2 = a.mont_mul(&t2_new2);       // 31.
        let t4_new = t4_new + t2_new2;            // 32.

        let t0_tmp = t1_new.mont_mul(&t4_new);    // 33.
        let y3 = y3 + t0_tmp;                     // 34.

        let t0_tmp2 = t5.mont_mul(&t4_new);       // 35.
        let x3 = t3.mont_mul(&x3);                // 36.
        let x3 = x3 - t0_tmp2;                    // 37.

        let t0_tmp3 = t3.mont_mul(&t1_new);       // 38.
        let z3 = t5.mont_mul(&z3);                // 39.
        let z3 = z3 + t0_tmp3;                    // 40.

        Self { x: x3, y: y3, z: z3 }
    }

    // ===============================================================
    // Renes-Costello-Batina, EUROCRYPT 2016, Algorithm 3
    //                              complete doubling for any a, b
    // ===============================================================
    //
    // Cost: 8 M + 3 S + 3 amul + 2 b3mul + 19 A
    //     ≈ 13 M + 3 S + 19 A   in our setting.

    /// Complete point doubling `2 · self`.
    pub fn double(&self) -> Self {
        let a  = Fp521 { limbs: SHORT_A_MONT_LIMBS };
        let b3 = Fp521 { limbs: SHORT_B3_MONT_LIMBS };

        let (x, y, z) = (self.x, self.y, self.z);

        let t0 = x.square();                       // 1.  X²
        let t1 = y.square();                       // 2.  Y²
        let t2 = z.square();                       // 3.  Z²
        let t3 = x.mont_mul(&y);                   // 4.
        let t3 = t3 + t3;                          // 5.

        let z3 = x.mont_mul(&z);                   // 6.
        let z3 = z3 + z3;                          // 7.

        let x3 = a.mont_mul(&z3);                  // 8.
        let y3 = b3.mont_mul(&t2);                 // 9.
        let y3 = x3 + y3;                          // 10.
        let x3 = t1 - y3;                          // 11.
        let y3 = t1 + y3;                          // 12.
        let y3 = x3.mont_mul(&y3);                 // 13.
        let x3 = t3.mont_mul(&x3);                 // 14.

        let z3 = b3.mont_mul(&z3);                 // 15.
        let t2 = a.mont_mul(&t2);                  // 16.
        let t3 = t0 - t2;                          // 17.
        let t3 = a.mont_mul(&t3);                  // 18.
        let t3 = t3 + z3;                          // 19.

        let z3 = t0 + t0;                          // 20.
        let t0 = z3 + t0;                          // 21.
        let t0 = t0 + t2;                          // 22.
        let t0 = t0.mont_mul(&t3);                 // 23.
        let y3 = y3 + t0;                          // 24.

        let t2 = y.mont_mul(&z);                   // 25.
        let t2 = t2 + t2;                          // 26.
        let t0 = t2.mont_mul(&t3);                 // 27.
        let x3 = x3 - t0;                          // 28.
        let z3 = t2.mont_mul(&t1);                 // 29.
        let z3 = z3 + z3;                          // 30.
        let z3 = z3 + z3;                          // 31.

        Self { x: x3, y: y3, z: z3 }
    }

    // ===============================================================
    // Affine conversion
    // ===============================================================

    /// Return the affine `(x, y)` of `self` (in the short-Weierstrass
    /// chart, in Montgomery form), or `None` if `self` is the point
    /// at infinity.
    ///
    /// Uses one Montgomery inversion of `Z`.
    pub fn to_affine(&self) -> Option<(Fp521, Fp521)> {
        if bool::from(self.is_identity()) {
            return None;
        }
        let z_inv = self.z.invert()?;
        let x_aff = self.x.mont_mul(&z_inv);
        let y_aff = self.y.mont_mul(&z_inv);
        Some((x_aff, y_aff))
    }
}

// ---------------------------------------------------------------
// Tests
// ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_at_infinity() {
        assert!(bool::from(ProjPoint::IDENTITY.is_identity()));
    }

    #[test]
    fn generator_is_not_identity() {
        assert!(!bool::from(ProjPoint::GENERATOR.is_identity()));
    }

    #[test]
    fn double_identity_is_identity() {
        let two_o = ProjPoint::IDENTITY.double();
        assert!(bool::from(two_o.is_identity()));
    }

    #[test]
    fn add_identity_left_is_rhs() {
        let g = ProjPoint::GENERATOR;
        let sum = ProjPoint::IDENTITY.add(&g);
        // Projectively (X3 : Y3 : Z3) should be equivalent to G.
        // Verify via affine.
        let (gx, gy) = g.to_affine().unwrap();
        let (sx, sy) = sum.to_affine().expect("not at infinity");
        assert_eq!(sx, gx);
        assert_eq!(sy, gy);
    }

    #[test]
    fn add_identity_right_is_lhs() {
        let g = ProjPoint::GENERATOR;
        let sum = g.add(&ProjPoint::IDENTITY);
        let (gx, gy) = g.to_affine().unwrap();
        let (sx, sy) = sum.to_affine().expect("not at infinity");
        assert_eq!(sx, gx);
        assert_eq!(sy, gy);
    }

    #[test]
    fn double_g_equals_add_g_g() {
        let g = ProjPoint::GENERATOR;
        let dbl = g.double();
        let add = g.add(&g);
        let (dx, dy) = dbl.to_affine().expect("2G not at infinity");
        let (ax, ay) = add.to_affine().expect("G+G not at infinity");
        assert_eq!(ax, dx, "x-coords disagree between double() and add(g,g)");
        assert_eq!(ay, dy, "y-coords disagree between double() and add(g,g)");
    }

    #[test]
    fn cond_swap_choice_zero_noop() {
        let mut a = ProjPoint::IDENTITY;
        let mut b = ProjPoint::GENERATOR;
        ProjPoint::conditional_swap(&mut a, &mut b, Choice::from(0));
        assert!(bool::from(a.is_identity()));
        assert!(!bool::from(b.is_identity()));
    }

    #[test]
    fn cond_swap_choice_one_swaps() {
        let mut a = ProjPoint::IDENTITY;
        let mut b = ProjPoint::GENERATOR;
        ProjPoint::conditional_swap(&mut a, &mut b, Choice::from(1));
        assert!(!bool::from(a.is_identity()));
        assert!(bool::from(b.is_identity()));
    }
}

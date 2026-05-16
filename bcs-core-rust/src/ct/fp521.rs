//! 9-limb constant-time field element for `F_{p_521}`.
//!
//! See `BCS_CT_DESIGN.md` § 2 for the algorithmic specification.
//!
//! ## Status
//!
//! Step 1 of the CT roll-out:
//!   - [x] type, byte (de)serialization
//!   - [x] constant-time `conditional_swap`
//!   - [x] constant-time `add`, `sub`, `neg`  (canonical form, mod p)
//!   - [ ] Montgomery `mul`, `square`        ← **next milestone**
//!   - [ ] Fermat inversion via addition chain
//!
//! All operations currently exposed here operate on **canonical** form
//! (i.e. `0 ≤ a < p`).  Once Montgomery multiplication lands, the
//! type-state distinction between `Fp521Canonical` and
//! `Fp521Montgomery` will become a compile-time guarantee.  For now we
//! deliberately keep a single concrete type with documented
//! pre-conditions to keep the diff small and reviewable.

use core::ops::{Add, Neg, Sub};
use subtle::{Choice, ConditionallySelectable, ConstantTimeEq};

use super::consts::{FIELD_BYTES, P_521_LIMBS};

/// Field element `a ∈ F_p` in 9-limb little-endian form.
#[derive(Clone, Copy, Debug)]
pub struct Fp521 {
    pub(crate) limbs: [u64; 9],
}

impl Fp521 {
    /// The zero element of the field.
    pub const ZERO: Self = Self { limbs: [0; 9] };

    /// The multiplicative identity in *canonical* form.
    pub const ONE_CANONICAL: Self = Self {
        limbs: [1, 0, 0, 0, 0, 0, 0, 0, 0],
    };

    /// The prime `p_521` itself, useful for canonical-form checks.
    pub const P: Self = Self { limbs: P_521_LIMBS };

    // ---------------------------------------------------------------
    // Serialization
    // ---------------------------------------------------------------

    /// Decode 66 big-endian bytes into a field element.
    ///
    /// Returns `None` (as a `subtle::CtOption` in the final API; for
    /// now `Option<Self>`) if the encoded value is `≥ p_521`.
    ///
    /// **Constant-time** with respect to the value of `bytes`.
    pub fn from_bytes_be(bytes: &[u8; FIELD_BYTES]) -> Option<Self> {
        // Pack bytes into 9 little-endian u64 limbs.
        let mut limbs = [0u64; 9];
        // bytes[0]   = most-significant byte; we want it in the top
        // bits of `limbs[8]`.  Limb 8 holds at most 9 bits (since the
        // prime is 521 bits = 64*8 + 9), so we treat the leading byte
        // specially.
        //
        // For simplicity we first reverse to little-endian, then chunk
        // into u64s.  No data-dependent branches.
        let mut le = [0u8; 72]; // 9 * 8 = 72; pad with zeros
        for i in 0..FIELD_BYTES {
            le[i] = bytes[FIELD_BYTES - 1 - i];
        }
        for i in 0..9 {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&le[i * 8..(i + 1) * 8]);
            limbs[i] = u64::from_le_bytes(buf);
        }
        let candidate = Self { limbs };
        // Range-check `candidate < P` in constant time.
        if candidate.ct_lt_p().unwrap_u8() == 1 {
            Some(candidate)
        } else {
            None
        }
    }

    /// Encode the field element as 66 big-endian bytes.
    ///
    /// **Constant-time** with respect to the value.
    pub fn to_bytes_be(&self) -> [u8; FIELD_BYTES] {
        let mut le = [0u8; 72];
        for i in 0..9 {
            le[i * 8..(i + 1) * 8].copy_from_slice(&self.limbs[i].to_le_bytes());
        }
        let mut be = [0u8; FIELD_BYTES];
        for i in 0..FIELD_BYTES {
            be[i] = le[FIELD_BYTES - 1 - i];
        }
        be
    }

    // ---------------------------------------------------------------
    // Constant-time helpers
    // ---------------------------------------------------------------

    /// Swap `a` and `b` if `choice == 1`, else leave them.
    /// Always touches every limb of both operands.
    #[inline]
    pub fn conditional_swap(a: &mut Self, b: &mut Self, choice: Choice) {
        for i in 0..9 {
            u64::conditional_swap(&mut a.limbs[i], &mut b.limbs[i], choice);
        }
    }

    /// Constant-time comparison.  Returns 1 iff `self < P_521`.
    #[inline]
    fn ct_lt_p(&self) -> Choice {
        // Subtract-with-borrow `self - P`; if the final borrow is 1,
        // then `self < P`.
        let mut borrow: u64 = 0;
        for i in 0..9 {
            let (diff, b1) = self.limbs[i].overflowing_sub(P_521_LIMBS[i]);
            let (_, b2) = diff.overflowing_sub(borrow);
            borrow = (b1 as u64) | (b2 as u64);
        }
        Choice::from(borrow as u8)
    }

    // ---------------------------------------------------------------
    // Field arithmetic — canonical form, constant-time
    // ---------------------------------------------------------------

    /// Modular addition: `(self + rhs) mod p`.
    ///
    /// Algorithm: full addition (with carry into a 10th word), then
    /// conditional subtraction of `p`.  Both branches are always
    /// executed; the result is selected with `Choice`.
    #[inline]
    pub fn add_mod_p(&self, rhs: &Self) -> Self {
        // 1. raw add with carry
        let mut sum = [0u64; 10];
        let mut carry: u64 = 0;
        for i in 0..9 {
            let (s1, c1) = self.limbs[i].overflowing_add(rhs.limbs[i]);
            let (s2, c2) = s1.overflowing_add(carry);
            sum[i] = s2;
            carry = (c1 as u64) + (c2 as u64);
        }
        sum[9] = carry;

        // 2. tentative `sum - p`
        let mut diff = [0u64; 10];
        let mut borrow: u64 = 0;
        for i in 0..9 {
            let (d1, b1) = sum[i].overflowing_sub(P_521_LIMBS[i]);
            let (d2, b2) = d1.overflowing_sub(borrow);
            diff[i] = d2;
            borrow = (b1 as u64) + (b2 as u64);
        }
        let (d9, b9) = sum[9].overflowing_sub(borrow);
        diff[9] = d9;
        let final_borrow = b9 as u8;

        // 3. select: if final_borrow == 0 then sum-p else sum.
        //    final_borrow==0  ⇒  sum ≥ p  ⇒  take diff
        //    final_borrow==1  ⇒  sum <  p ⇒  take sum
        let take_sum = Choice::from(final_borrow);
        let mut out = [0u64; 9];
        for i in 0..9 {
            out[i] = u64::conditional_select(&diff[i], &sum[i], take_sum);
        }
        Self { limbs: out }
    }

    /// Modular subtraction: `(self - rhs) mod p`.
    #[inline]
    pub fn sub_mod_p(&self, rhs: &Self) -> Self {
        // 1. raw sub with borrow
        let mut diff = [0u64; 9];
        let mut borrow: u64 = 0;
        for i in 0..9 {
            let (d1, b1) = self.limbs[i].overflowing_sub(rhs.limbs[i]);
            let (d2, b2) = d1.overflowing_sub(borrow);
            diff[i] = d2;
            borrow = (b1 as u64) + (b2 as u64);
        }
        let final_borrow = borrow as u8;

        // 2. conditional add of p (always compute, select with Choice)
        let mut fixed = [0u64; 9];
        let mut carry: u64 = 0;
        for i in 0..9 {
            let (s1, c1) = diff[i].overflowing_add(P_521_LIMBS[i]);
            let (s2, c2) = s1.overflowing_add(carry);
            fixed[i] = s2;
            carry = (c1 as u64) + (c2 as u64);
        }
        let take_fixed = Choice::from(final_borrow);
        let mut out = [0u64; 9];
        for i in 0..9 {
            out[i] = u64::conditional_select(&diff[i], &fixed[i], take_fixed);
        }
        Self { limbs: out }
    }

    /// Modular negation: `(-self) mod p`.
    #[inline]
    pub fn neg_mod_p(&self) -> Self {
        Self::ZERO.sub_mod_p(self)
    }

    /// Constant-time equality test.
    #[inline]
    pub fn ct_eq(&self, rhs: &Self) -> Choice {
        let mut acc: u64 = 0;
        for i in 0..9 {
            acc |= self.limbs[i] ^ rhs.limbs[i];
        }
        // acc == 0  ⇔  equal
        // Map acc to a Choice via the subtle helper.
        u64::ct_eq(&acc, &0)
    }
}

// ---------------------------------------------------------------
// Trait impls
// ---------------------------------------------------------------

impl Add for Fp521 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self { self.add_mod_p(&rhs) }
}

impl Sub for Fp521 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self { self.sub_mod_p(&rhs) }
}

impl Neg for Fp521 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self { self.neg_mod_p() }
}

impl PartialEq for Fp521 {
    fn eq(&self, rhs: &Self) -> bool { bool::from(self.ct_eq(rhs)) }
}
impl Eq for Fp521 {}

#[cfg(feature = "zeroize")]
impl zeroize::Zeroize for Fp521 {
    fn zeroize(&mut self) { self.limbs.zeroize(); }
}

// ---------------------------------------------------------------
// Unit tests for the canonical-form path
// ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_zero() {
        let bytes = [0u8; FIELD_BYTES];
        let a = Fp521::from_bytes_be(&bytes).expect("zero must decode");
        assert_eq!(a.to_bytes_be(), bytes);
    }

    #[test]
    fn roundtrip_one() {
        let mut bytes = [0u8; FIELD_BYTES];
        bytes[FIELD_BYTES - 1] = 1;
        let a = Fp521::from_bytes_be(&bytes).expect("one must decode");
        assert_eq!(a.to_bytes_be(), bytes);
    }

    #[test]
    fn add_zero_is_identity() {
        let mut bytes = [0u8; FIELD_BYTES];
        bytes[FIELD_BYTES - 1] = 7;
        let a = Fp521::from_bytes_be(&bytes).unwrap();
        let z = Fp521::ZERO;
        assert_eq!((a + z).to_bytes_be(), bytes);
    }

    #[test]
    fn sub_self_is_zero() {
        let mut bytes = [0u8; FIELD_BYTES];
        bytes[FIELD_BYTES - 1] = 42;
        let a = Fp521::from_bytes_be(&bytes).unwrap();
        let diff = a - a;
        assert_eq!(diff.to_bytes_be(), [0u8; FIELD_BYTES]);
    }

    #[test]
    fn neg_of_neg_is_identity() {
        let mut bytes = [0u8; FIELD_BYTES];
        bytes[FIELD_BYTES - 1] = 9;
        let a = Fp521::from_bytes_be(&bytes).unwrap();
        let a2 = -(-a);
        assert_eq!(a2.to_bytes_be(), bytes);
    }

    #[test]
    fn p_is_out_of_range() {
        // `p` itself is not a valid canonical element.
        let p_bytes = Fp521::P.to_bytes_be();
        assert!(Fp521::from_bytes_be(&p_bytes).is_none());
    }

    #[test]
    fn cond_swap_choice_zero_noop() {
        let mut a = Fp521::ZERO;
        let mut b = Fp521::ONE_CANONICAL;
        Fp521::conditional_swap(&mut a, &mut b, Choice::from(0));
        assert_eq!(a, Fp521::ZERO);
        assert_eq!(b, Fp521::ONE_CANONICAL);
    }

    #[test]
    fn cond_swap_choice_one_swaps() {
        let mut a = Fp521::ZERO;
        let mut b = Fp521::ONE_CANONICAL;
        Fp521::conditional_swap(&mut a, &mut b, Choice::from(1));
        assert_eq!(a, Fp521::ONE_CANONICAL);
        assert_eq!(b, Fp521::ZERO);
    }
}

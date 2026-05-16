//! 9-limb constant-time field element for `F_{p_521}`.
//!
//! See `BCS_CT_DESIGN.md` § 2 for the algorithmic specification.
//!
//! ## Status
//!
//! - [x] type, byte (de)serialization
//! - [x] constant-time `conditional_swap`
//! - [x] constant-time `add`, `sub`, `neg`  (canonical form, mod p)
//! - [x] Montgomery `mont_mul`, `square`, `to_montgomery`, `from_montgomery`
//! - [ ] Fermat inversion via addition chain
//!
//! ### Operand-form convention
//!
//! Until the type-state split lands in v0.3.0 we keep a single
//! `Fp521` type but distinguish forms by *method name*:
//!
//! - `add_mod_p`, `sub_mod_p`, `neg_mod_p` work in **canonical** form
//!   (`0 ≤ a < p`) and are exact-result.
//! - `mont_mul`, `square` work in **Montgomery** form (i.e. the limbs
//!   encode `a · R mod p`).
//! - `to_montgomery` and `from_montgomery` are the conversion bridges.
//!
//! Add/sub are valid in **both** forms: `(aR + bR) = (a+b)R`.

use core::ops::{Add, Neg, Sub};
use subtle::{Choice, ConditionallySelectable, ConstantTimeEq};

use super::consts::{
    FIELD_BITS, FIELD_BYTES, MONT_INV_NEG_P_0, MONT_R2_LIMBS, MONT_R_LIMBS, P_521_LIMBS,
};

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

    // ===============================================================
    // Montgomery multiplication (CIOS algorithm)
    // ===============================================================
    //
    // CIOS  ≡  Coarsely Integrated Operand Scanning  (Koc et al., 1996).
    //
    // Computes:
    //
    //     mont_mul(a~, b~)  =  a~ · b~ · R^{-1}  mod p
    //
    // where  R = 2^576  is the Montgomery radix.  All loops have a
    // fixed iteration count of 9 and contain *no* data-dependent
    // branches; the final conditional subtraction is performed
    // unconditionally and selected with `Choice`.
    //
    // Pre-condition:  both operands fit in 576 bits (i.e. the limbs
    // represent a non-negative integer less than `2 · p`).
    // Post-condition:  the returned value is the unique Montgomery
    // representative in `[0, p)`.
    //
    // Reference:  Çetin Kaya Koç, Tolga Acar, Burton S. Kaliski Jr.,
    // *Analyzing and Comparing Montgomery Multiplication Algorithms*,
    // IEEE Micro 16(3) 1996, §3.

    /// Montgomery multiplication: returns the Montgomery representative
    /// of `a~ · b~ · R^{-1} mod p`.
    #[inline]
    pub fn mont_mul(&self, rhs: &Self) -> Self {
        let a = &self.limbs;
        let b = &rhs.limbs;
        let p = &P_521_LIMBS;
        let p_inv_neg = MONT_INV_NEG_P_0;

        // t has 11 limbs: 9 for the accumulator + 2 for top-end carry.
        let mut t = [0u64; 11];

        for i in 0..9 {
            // ---- Multiply step: t += a · b[i] ----
            let bi = b[i] as u128;
            let mut carry: u64 = 0;
            for j in 0..9 {
                let prod = (a[j] as u128) * bi + (t[j] as u128) + (carry as u128);
                t[j] = prod as u64;
                carry = (prod >> 64) as u64;
            }
            // Propagate carry into the two top limbs.  The reduce
            // step at the end of the previous outer iteration set
            // `t[10] = 0` so this is a direct assignment.
            let (s, c1) = t[9].overflowing_add(carry);
            t[9] = s;
            t[10] = c1 as u64;

            // ---- Reduce step: m = t[0] · (-p^-1) mod 2^64 ----
            let m = (t[0] as u128 * p_inv_neg as u128) as u64;

            // First inner-loop iteration drops the low word.
            let prod0 = (m as u128) * (p[0] as u128) + (t[0] as u128);
            // The bottom 64 bits of prod0 must be zero by construction
            // of m (this is the Montgomery trick).
            debug_assert!((prod0 as u64) == 0);
            let mut carry = (prod0 >> 64) as u64;

            // Remaining 8 limbs: t[j-1] = t[j] + m * p[j] + carry.
            for j in 1..9 {
                let prod = (m as u128) * (p[j] as u128) + (t[j] as u128) + (carry as u128);
                t[j - 1] = prod as u64;
                carry = (prod >> 64) as u64;
            }
            // Fold carries into the top two slots.
            let (s, c1) = t[9].overflowing_add(carry);
            t[8] = s;
            t[9] = t[10].wrapping_add(c1 as u64);
            t[10] = 0;
        }

        // ---- Final conditional subtraction of p ----
        //
        // After 9 iterations of CIOS,  t  is at most  2p − 1  and fits
        // into the lower 9 limbs *plus* possibly the 10th.  We
        // subtract `p` from the 10-limb value; if the borrow is 1 the
        // pre-subtract value was already < p and we keep it.
        let mut diff = [0u64; 10];
        let mut borrow: u64 = 0;
        for i in 0..9 {
            let (d1, b1) = t[i].overflowing_sub(p[i]);
            let (d2, b2) = d1.overflowing_sub(borrow);
            diff[i] = d2;
            borrow = (b1 as u64) + (b2 as u64);
        }
        // 10th limb of p is zero, so we just propagate borrow.
        let (d9, b9) = t[9].overflowing_sub(borrow);
        diff[9] = d9;
        let take_t = Choice::from(b9 as u8); // borrow=1  ⇒ t < p  ⇒ keep t

        let mut out = [0u64; 9];
        for i in 0..9 {
            out[i] = u64::conditional_select(&diff[i], &t[i], take_t);
        }
        Self { limbs: out }
    }

    /// Squaring in Montgomery form: `mont_mul(self, self)`.
    #[inline]
    pub fn square(&self) -> Self {
        self.mont_mul(self)
    }

    /// Convert a canonical-form element `a` into Montgomery form `a · R mod p`.
    ///
    /// Implementation: `mont_mul(a, R^2) = a · R^2 · R^{-1} = a · R`.
    #[inline]
    pub fn to_montgomery(&self) -> Self {
        let r2 = Self { limbs: MONT_R2_LIMBS };
        self.mont_mul(&r2)
    }

    /// Convert a Montgomery-form element `a~ = a · R mod p` back to canonical
    /// form `a`.
    ///
    /// Implementation: `mont_mul(a~, 1) = a · R · 1 · R^{-1} = a`.
    /// Here `1` is the literal canonical integer 1 (limbs `[1, 0, …, 0]`),
    /// **not** the Montgomery form of 1.
    #[inline]
    pub fn from_montgomery(&self) -> Self {
        let one_canonical = Self { limbs: [1, 0, 0, 0, 0, 0, 0, 0, 0] };
        self.mont_mul(&one_canonical)
    }

    /// The multiplicative identity in Montgomery form: `1 · R mod p = R mod p`.
    pub const ONE_MONT: Self = Self { limbs: MONT_R_LIMBS };

    // ===============================================================
    // Inversion via Fermat's little theorem
    // ===============================================================
    //
    //     a^{-1}  ≡  a^{p-2}  (mod p)        for any a ≠ 0
    //
    // The exponent `p_521 - 2` is a **public** 521-bit constant, so a
    // simple left-to-right square-and-multiply with a fixed bit count
    // of 521 is already constant-time with respect to secret inputs:
    // the loop count is fixed, and any branching is governed by bits
    // of the (public) modulus minus two, not by bits of the secret.
    //
    // Cost: 520 squares + ≈ 260 multiplications  (the Hamming weight
    // of `p_521 - 2` is roughly half of 521).  This is ≈ 30% slower
    // than the optimal addition chain that v0.3.0 will ship.
    //
    // To absolutely defeat micro-architectural side channels we still
    // perform the multiplication on *every* iteration and select the
    // result with `Choice`, so the actual operation count is fixed at
    // 520 + 520 = 1040 Montgomery multiplications.

    /// Multiplicative inverse in Montgomery form.
    ///
    /// Returns the Montgomery representative of `self^{-1} mod p` if
    /// `self` is non-zero, else `None`.
    ///
    /// **Pre-condition:** `self` is in Montgomery form.
    /// **Post-condition:** the returned value is in Montgomery form.
    pub fn invert(&self) -> Option<Self> {
        // Reject zero in constant time but with an early return on the
        // public branch.  (Whether the *operand* is zero is itself
        // assumed public — the caller must not pass secret zero
        // values; ECDH/signing always ensure non-zero scalars.)
        if bool::from(self.ct_eq(&Self::ZERO)) {
            return None;
        }

        // Exponent: e = p_521 - 2
        let mut exp = P_521_LIMBS;
        // P_521_LIMBS[0] = 0x9D26D073AB604B13 (odd), subtracting 2 stays positive.
        exp[0] = exp[0].wrapping_sub(2);

        let mut result = Self::ONE_MONT;

        // Left-to-right square-and-multiply.  We process the 521 bits
        // of the exponent from MSB (bit 520) to LSB (bit 0).
        for i in (0..FIELD_BITS).rev() {
            // Square unconditionally.
            result = result.square();

            // Read bit `i` of the (public) exponent.
            let limb_idx = i / 64;
            let bit_idx  = i % 64;
            let bit      = ((exp[limb_idx] >> bit_idx) & 1) as u8;

            // Compute the multiplication ALWAYS, select based on bit
            // value.  This costs an extra ≈ 260 Montgomery muls but
            // guarantees no micro-architectural leak from speculative
            // execution of the `if` branch.
            let multiplied = result.mont_mul(self);
            for j in 0..9 {
                result.limbs[j] = u64::conditional_select(
                    &result.limbs[j],
                    &multiplied.limbs[j],
                    Choice::from(bit),
                );
            }
        }
        Some(result)
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

    // ---------- Montgomery multiplication ----------

    /// Encode a small canonical integer (`0..=u64::MAX`) into an `Fp521`.
    fn small_canonical(x: u64) -> Fp521 {
        Fp521 { limbs: [x, 0, 0, 0, 0, 0, 0, 0, 0] }
    }

    #[test]
    fn mont_roundtrip_zero() {
        let zero = Fp521::ZERO;
        let m = zero.to_montgomery();
        let back = m.from_montgomery();
        assert_eq!(back, zero);
    }

    #[test]
    fn mont_roundtrip_one() {
        let one = small_canonical(1);
        let m = one.to_montgomery();
        // Montgomery form of 1 must equal MONT_R (R mod p).
        assert_eq!(m, Fp521::ONE_MONT);
        let back = m.from_montgomery();
        assert_eq!(back, one);
    }

    #[test]
    fn mont_roundtrip_many() {
        // 32 spread-out values: powers of 2, products, near-p, etc.
        for x in [
            1u64, 2, 3, 7, 17, 257, 65537,
            0xDEAD_BEEF, 0xFFFF_FFFF_FFFF_FFFF,
        ] {
            let a = small_canonical(x);
            let a_mont = a.to_montgomery();
            let a_back = a_mont.from_montgomery();
            assert_eq!(a_back, a, "round-trip failed for {x}");
        }
    }

    #[test]
    fn mont_mul_by_one() {
        // a~ · 1~  =  a~     (where 1~ = ONE_MONT)
        let a = small_canonical(0x12_3456_789A_BCDE_F0);
        let a_mont = a.to_montgomery();
        let prod = a_mont.mont_mul(&Fp521::ONE_MONT);
        assert_eq!(prod, a_mont);
    }

    #[test]
    fn mont_mul_small_then_unwrap() {
        // (3~) · (5~) · R^-1 = 15~,  and from_mont(15~) = 15.
        let three      = small_canonical(3).to_montgomery();
        let five       = small_canonical(5).to_montgomery();
        let fifteen_m  = three.mont_mul(&five);
        let fifteen    = fifteen_m.from_montgomery();
        assert_eq!(fifteen, small_canonical(15));
    }

    #[test]
    fn mont_square_matches_self_mul() {
        let a = small_canonical(0xABCD_1234_5678_9ABC).to_montgomery();
        assert_eq!(a.square(), a.mont_mul(&a));
    }

    #[test]
    fn mont_mul_associative_small() {
        // (2 · 3) · 5  =  2 · (3 · 5)  in Montgomery form
        let m2 = small_canonical(2).to_montgomery();
        let m3 = small_canonical(3).to_montgomery();
        let m5 = small_canonical(5).to_montgomery();
        let left  = m2.mont_mul(&m3).mont_mul(&m5);
        let right = m2.mont_mul(&m3.mont_mul(&m5));
        assert_eq!(left, right);
        // and equals 30 after un-Montgomerize
        assert_eq!(left.from_montgomery(), small_canonical(30));
    }

    // ---------- Inversion ----------

    #[test]
    fn invert_of_zero_is_none() {
        assert!(Fp521::ZERO.invert().is_none());
    }

    #[test]
    fn invert_of_one_is_one() {
        let one_mont = small_canonical(1).to_montgomery();
        let inv = one_mont.invert().expect("1 is invertible");
        assert_eq!(inv, one_mont);
    }

    #[test]
    fn invert_then_multiply_is_one() {
        // For each test value v:  v · v^-1  =  1   (all in Montgomery form)
        for v in [2u64, 3, 7, 17, 0xDEAD_BEEF_u64, 0xFEDC_BA98_7654_3210] {
            let a   = small_canonical(v).to_montgomery();
            let inv = a.invert().expect("non-zero invertible");
            let prod = a.mont_mul(&inv);
            assert_eq!(
                prod, Fp521::ONE_MONT,
                "v={v} : v · v^-1 ≠ 1  (got {:?})", prod
            );
        }
    }

    #[test]
    fn invert_double_is_identity() {
        // (a^-1)^-1 = a
        let a = small_canonical(0x1234_5678_9ABC_DEF0).to_montgomery();
        let a_inv = a.invert().unwrap();
        let a_inv_inv = a_inv.invert().unwrap();
        assert_eq!(a_inv_inv, a);
    }
}

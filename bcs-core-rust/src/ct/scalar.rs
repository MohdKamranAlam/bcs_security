//! Scalar arithmetic modulo `n_521`.
//!
//! See `BCS_CT_DESIGN.md` § 3.
//!
//! ## Status
//!
//! - [x] type, byte (de)serialization
//! - [x] MSB-first bit iterator (constant 521 iterations)
//! - [x] `Zeroize` / `ZeroizeOnDrop`
//! - [x] `add_mod_n`, `sub_mod_n` (canonical form, mod n)
//! - [x] Montgomery multiplication mod n (`mont_mul_n`)
//! - [x] `to_montgomery_n`, `from_montgomery_n`
//! - [x] `mul_mod_n` — canonical `a·b mod n`
//! - [x] `inv_mod_n` — Fermat inversion `a^(n-2) mod n` (constant-time)
//!
//! These operations are required by ECDSA sign:
//!
//! ```text
//! s = k⁻¹ · (z + r·d)  mod n
//!     ^^^^   ^^^^^^^^
//!     inv_mod_n  mul_mod_n
//! ```

// bool-as-u64 (e.g. `b1 as u64`) is the intentional CT-safe idiom throughout
// this module.  Clippy's `cast_lossless` suggestion (`u64::from(b)`) is
// equivalent but suppressed here to keep the CT code readable.
#![allow(clippy::cast_lossless)]

use subtle::{Choice, ConstantTimeEq};
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::consts::{
    FIELD_BITS, FIELD_BYTES, MONT_INV_NEG_N_0, N_521_LIMBS, N_521_MINUS_2_LIMBS, N_MONT_R2_LIMBS,
    N_MONT_R_LIMBS,
};
use super::aggressive_zeroize::AggressiveZeroize;

/// A scalar in `Z / n_521 Z`, stored as 9 little-endian `u64` limbs.
///
/// `Drop` zeroizes the limbs.  This struct is **secret** by default
/// and must never be `Debug`-printed in production.
///
/// `Scalar` is intentionally **not** `Copy`: every duplication of a
/// secret should be a deliberate `.clone()` so it is easy to audit
/// where secrets exist in memory.  `Clone` is provided for the cases
/// where duplication is genuinely necessary (e.g. in tests).
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Scalar {
    pub(crate) limbs: [u64; 9],
}

impl core::fmt::Debug for Scalar {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Scalar(***)")
    }
}

impl AggressiveZeroize for Scalar {
    fn aggressive_zeroize(&mut self) {
        super::aggressive_zeroize::aggressive_clear_u64(&mut self.limbs);
    }
}

impl Scalar {
    /// The zero scalar.
    pub const ZERO: Self = Self { limbs: [0; 9] };

    /// `n_521` itself, used for range checks.
    pub const N: Self = Self { limbs: N_521_LIMBS };

    /// The multiplicative identity in Montgomery form mod n: `1 · R mod n`.
    pub const ONE_MONT_N: Self = Self { limbs: N_MONT_R_LIMBS };

    // ---------------------------------------------------------------
    // Serialization
    // ---------------------------------------------------------------

    /// Decode 66 big-endian bytes into a scalar.
    ///
    /// Returns `None` if the encoded value is `≥ n_521`.
    /// **Constant-time** w.r.t. the value of `bytes`.
    pub fn from_bytes_be(bytes: &[u8; FIELD_BYTES]) -> Option<Self> {
        let mut le = [0u8; 72];
        for i in 0..FIELD_BYTES {
            le[i] = bytes[FIELD_BYTES - 1 - i];
        }
        let mut limbs = [0u64; 9];
        for i in 0..9 {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&le[i * 8..(i + 1) * 8]);
            limbs[i] = u64::from_le_bytes(buf);
        }
        let candidate = Self { limbs };
        if candidate.ct_lt_n().unwrap_u8() == 1 {
            Some(candidate)
        } else {
            None
        }
    }

    /// Encode as 66 big-endian bytes.  **Constant-time.**
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

    /// Constant-time `self < n_521`?
    ///
    /// The explicit `for i in 0..9` form is intentional — see the
    /// header comment on `Fp521::ct_lt_p` for the constant-time
    /// rationale that applies to every 9-limb indexed loop in this
    /// module.
    #[inline]
    #[allow(clippy::needless_range_loop)]
    pub(crate) fn ct_lt_n(&self) -> Choice {
        let mut borrow: u64 = 0;
        for i in 0..9 {
            let (diff, b1) = self.limbs[i].overflowing_sub(N_521_LIMBS[i]);
            let (_, b2) = diff.overflowing_sub(borrow);
            borrow = (b1 as u64) | (b2 as u64);
        }
        Choice::from(borrow as u8)
    }

    /// Constant-time equality.
    #[inline]
    pub fn ct_eq(&self, rhs: &Self) -> Choice {
        let mut acc: u64 = 0;
        for i in 0..9 {
            acc |= self.limbs[i] ^ rhs.limbs[i];
        }
        u64::ct_eq(&acc, &0)
    }

    // ---------------------------------------------------------------
    // Bit iteration for the Montgomery ladder
    // ---------------------------------------------------------------

    /// Return bit `i` (0 = LSB) as a `u8 ∈ {0,1}`.  **Constant-time.**
    #[inline]
    pub fn bit(&self, i: usize) -> u8 {
        debug_assert!(i < FIELD_BITS);
        let limb = self.limbs[i / 64];
        ((limb >> (i % 64)) & 1) as u8
    }

    /// Iterate bits from MSB (index 520) down to LSB (index 0).
    /// Yields exactly `FIELD_BITS = 521` items regardless of value.
    pub fn bits_msb_first(&self) -> impl Iterator<Item = u8> + '_ {
        (0..FIELD_BITS).rev().map(move |i| self.bit(i))
    }

    // ===============================================================
    // Addition / subtraction mod n  (canonical form)
    // ===============================================================

    /// `self + rhs mod n`.  Both operands must be in canonical form
    /// (`0 ≤ x < n`).  **Constant-time.**
    #[inline]
    #[allow(clippy::needless_range_loop)]
    pub fn add_mod_n(&self, rhs: &Self) -> Self {
        // Add with carry propagation across 9 limbs.
        let mut limbs = [0u64; 9];
        let mut carry: u64 = 0;
        for i in 0..9 {
            let (s1, c1) = self.limbs[i].overflowing_add(rhs.limbs[i]);
            let (s2, c2) = s1.overflowing_add(carry);
            limbs[i] = s2;
            carry = (c1 as u64) + (c2 as u64);
        }

        // `sum` is at most `2n − 2 + carry_bit`, i.e. at most `2n − 1`.
        // Subtract n and select based on whether the subtraction underflowed.
        let mut diff = [0u64; 9];
        let mut borrow: u64 = 0;
        for i in 0..9 {
            let (d1, b1) = limbs[i].overflowing_sub(N_521_LIMBS[i]);
            let (d2, b2) = d1.overflowing_sub(borrow);
            diff[i] = d2;
            borrow = (b1 as u64) | (b2 as u64);
        }

        // If borrow=1, the sum was < n, so keep the original sum.
        let keep_sum = Choice::from(borrow as u8);
        let mut out = [0u64; 9];
        for i in 0..9 {
            out[i] = u64::conditional_select(&diff[i], &limbs[i], keep_sum);
        }
        Self { limbs: out }
    }

    /// `self − rhs mod n`.  Both operands must be in canonical form.
    /// **Constant-time.**
    #[inline]
    #[allow(clippy::needless_range_loop)]
    pub fn sub_mod_n(&self, rhs: &Self) -> Self {
        let mut diff = [0u64; 9];
        let mut borrow: u64 = 0;
        for i in 0..9 {
            let (d1, b1) = self.limbs[i].overflowing_sub(rhs.limbs[i]);
            let (d2, b2) = d1.overflowing_sub(borrow);
            diff[i] = d2;
            borrow = (b1 as u64) | (b2 as u64);
        }

        // If borrow=1, self < rhs, so we need to add n back.
        let mut sum = [0u64; 9];
        let mut carry: u64 = 0;
        for i in 0..9 {
            let (s1, c1) = diff[i].overflowing_add(N_521_LIMBS[i]);
            let (s2, c2) = s1.overflowing_add(carry);
            sum[i] = s2;
            carry = (c1 as u64) + (c2 as u64);
        }

        let add_n = Choice::from(borrow as u8);
        let mut out = [0u64; 9];
        for i in 0..9 {
            out[i] = u64::conditional_select(&diff[i], &sum[i], add_n);
        }
        Self { limbs: out }
    }

    // ===============================================================
    // Montgomery multiplication mod n
    // ===============================================================
    //
    // Identical algorithm to Fp521::mont_mul but with modulus n_521
    // and the corresponding Montgomery constants.

    /// Montgomery multiplication mod n: returns the Montgomery
    /// representative of `ã · b̃ · R^{-1} mod n`.
    ///
    /// **Constant-time.**  Both inputs must be in Montgomery form.
    #[inline]
    #[allow(clippy::needless_range_loop)]
    pub fn mont_mul_n(&self, rhs: &Self) -> Self {
        let a = &self.limbs;
        let b = &rhs.limbs;
        let n = &N_521_LIMBS;
        let n_inv_neg = MONT_INV_NEG_N_0;

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
            let (s, c1) = t[9].overflowing_add(carry);
            t[9] = s;
            t[10] = c1 as u64;

            // ---- Reduce step: m = t[0] · (-n^-1) mod 2^64 ----
            let m = (t[0] as u128 * n_inv_neg as u128) as u64;

            let prod0 = (m as u128) * (n[0] as u128) + (t[0] as u128);
            debug_assert!((prod0 as u64) == 0);
            let mut carry = (prod0 >> 64) as u64;

            for j in 1..9 {
                let prod = (m as u128) * (n[j] as u128) + (t[j] as u128) + (carry as u128);
                t[j - 1] = prod as u64;
                carry = (prod >> 64) as u64;
            }
            let (s, c1) = t[9].overflowing_add(carry);
            t[8] = s;
            t[9] = t[10].wrapping_add(c1 as u64);
            t[10] = 0;
        }

        // ---- Final conditional subtraction of n ----
        let mut diff = [0u64; 10];
        let mut borrow: u64 = 0;
        for i in 0..9 {
            let (d1, b1) = t[i].overflowing_sub(n[i]);
            let (d2, b2) = d1.overflowing_sub(borrow);
            diff[i] = d2;
            borrow = (b1 as u64) + (b2 as u64);
        }
        let (d9, b9) = t[9].overflowing_sub(borrow);
        diff[9] = d9;
        let take_t = Choice::from(b9 as u8);

        let mut out = [0u64; 9];
        for i in 0..9 {
            out[i] = u64::conditional_select(&diff[i], &t[i], take_t);
        }
        Self { limbs: out }
    }

    /// Squaring in Montgomery form mod n: `mont_mul_n(self, self)`.
    #[inline]
    pub fn square_n(&self) -> Self {
        self.mont_mul_n(self)
    }

    /// Convert a canonical-form scalar `a` into Montgomery form mod n:
    /// `a · R mod n`.
    #[inline]
    pub fn to_montgomery_n(&self) -> Self {
        let r2 = Self { limbs: N_MONT_R2_LIMBS };
        self.mont_mul_n(&r2)
    }

    /// Convert a Montgomery-form scalar `ã = a · R mod n` back to
    /// canonical form `a`.
    #[inline]
    pub fn from_montgomery_n(&self) -> Self {
        let one_canonical = Self { limbs: [1, 0, 0, 0, 0, 0, 0, 0, 0] };
        self.mont_mul_n(&one_canonical)
    }

    // ===============================================================
    // Public API: mul_mod_n, inv_mod_n
    // ===============================================================

    /// Canonical multiplication: `self · rhs mod n`.
    ///
    /// Both inputs are in canonical form; the result is in canonical
    /// form.  **Constant-time.**
    pub fn mul_mod_n(&self, rhs: &Self) -> Self {
        let a_mont = self.to_montgomery_n();
        let b_mont = rhs.to_montgomery_n();
        let prod_mont = a_mont.mont_mul_n(&b_mont);
        prod_mont.from_montgomery_n()
    }

    /// Multiplicative inverse mod n: `self^{-1} mod n`.
    ///
    /// Uses Fermat's little theorem: `a^{n-2} mod n`.
    /// Returns `None` if `self` is zero.
    ///
    /// **Constant-time** (the exponent `n-2` is public; we perform a
    /// multiplication on every iteration and select with `Choice`).
    pub fn inv_mod_n(&self) -> Option<Self> {
        // Reject zero.
        if bool::from(self.ct_eq(&Self::ZERO)) {
            return None;
        }

        // Exponent: e = n_521 - 2  (pre-computed constant).
        let exp = N_521_MINUS_2_LIMBS;

        let mut result = Self::ONE_MONT_N;
        let a_mont = self.to_montgomery_n();

        // Left-to-right square-and-multiply over 521 bits.
        for i in (0..FIELD_BITS).rev() {
            result = result.square_n();

            let limb_idx = i / 64;
            let bit_idx = i % 64;
            let bit = ((exp[limb_idx] >> bit_idx) & 1) as u8;

            let multiplied = result.mont_mul_n(&a_mont);
            for j in 0..9 {
                result.limbs[j] = u64::conditional_select(
                    &result.limbs[j],
                    &multiplied.limbs[j],
                    Choice::from(bit),
                );
            }
        }

        // Convert back from Montgomery form.
        Some(result.from_montgomery_n())
    }
}

// ---------------------------------------------------------------
// Trait impls
// ---------------------------------------------------------------

impl PartialEq for Scalar {
    fn eq(&self, rhs: &Self) -> bool {
        bool::from(self.ct_eq(rhs))
    }
}
impl Eq for Scalar {}

// ---------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_one() {
        let mut bytes = [0u8; FIELD_BYTES];
        bytes[FIELD_BYTES - 1] = 1;
        let s = Scalar::from_bytes_be(&bytes).unwrap();
        assert_eq!(s.to_bytes_be(), bytes);
    }

    #[test]
    fn bit_count_is_constant() {
        let s = Scalar::ZERO;
        assert_eq!(s.bits_msb_first().count(), FIELD_BITS);
    }

    #[test]
    fn bit_zero_lsb_one() {
        let mut bytes = [0u8; FIELD_BYTES];
        bytes[FIELD_BYTES - 1] = 1;
        let s = Scalar::from_bytes_be(&bytes).unwrap();
        assert_eq!(s.bit(0), 1);
        assert_eq!(s.bit(1), 0);
    }

    #[test]
    fn n_is_out_of_range() {
        let n_bytes = Scalar::N.to_bytes_be();
        assert!(Scalar::from_bytes_be(&n_bytes).is_none());
    }

    #[test]
    fn debug_does_not_leak() {
        let mut bytes = [0u8; FIELD_BYTES];
        bytes[0] = 0xFF;
        // 0xFF...FF would be > n; use a small value
        let mut small = [0u8; FIELD_BYTES];
        small[FIELD_BYTES - 1] = 7;
        let s = Scalar::from_bytes_be(&small).unwrap();
        let printed = format!("{:?}", s);
        assert_eq!(printed, "Scalar(***)");
        let _ = bytes; // silence unused
    }

    // -----------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------

    fn small_scalar(v: u64) -> Scalar {
        let mut b = [0u8; FIELD_BYTES];
        b[FIELD_BYTES - 8..].copy_from_slice(&v.to_be_bytes());
        Scalar::from_bytes_be(&b).expect("small scalar in range")
    }

    /// Build the scalar `(n - 1)` directly from `N_521_LIMBS`.
    fn n_minus_one() -> Scalar {
        let mut limbs = N_521_LIMBS;
        let (v, b) = limbs[0].overflowing_sub(1);
        limbs[0] = v;
        debug_assert!(!b, "n_521 is odd, n - 1 cannot underflow limb 0");
        Scalar { limbs }
    }

    /// Build a scalar one (i.e. limbs = [1, 0, ...]).
    fn one() -> Scalar {
        let mut s = Scalar::ZERO;
        s.limbs[0] = 1;
        s
    }

    /// Reject any scalar that is not in [0, n).
    fn assert_in_range(s: &Scalar) {
        assert_eq!(
            s.ct_lt_n().unwrap_u8(), 1,
            "scalar must be in [0, n_521) but was not: limbs = {:?}",
            s.limbs
        );
    }

    // -----------------------------------------------------------------
    // add_mod_n
    // -----------------------------------------------------------------

    #[test]
    fn add_mod_n_small() {
        let c = small_scalar(3).add_mod_n(&small_scalar(7));
        assert_eq!(c, small_scalar(10));
    }

    #[test]
    fn add_mod_n_zero_identity() {
        let a = small_scalar(0xdead_beef);
        assert_eq!(a.add_mod_n(&Scalar::ZERO), a);
        assert_eq!(Scalar::ZERO.add_mod_n(&a), a);
    }

    #[test]
    fn add_mod_n_wraps_n_minus_one_plus_one() {
        // (n-1) + 1 = n ≡ 0 (mod n)
        let c = n_minus_one().add_mod_n(&one());
        assert_eq!(c, Scalar::ZERO);
        assert_in_range(&c);
    }

    #[test]
    fn add_mod_n_wraps_double_n_minus_one() {
        // (n-1) + (n-1) = 2n - 2 ≡ n - 2 (mod n)
        let c = n_minus_one().add_mod_n(&n_minus_one());
        // Expected = n - 2
        let mut expected = N_521_LIMBS;
        let (v, _) = expected[0].overflowing_sub(2);
        expected[0] = v;
        let expected = Scalar { limbs: expected };
        assert_eq!(c, expected);
        assert_in_range(&c);
    }

    #[test]
    fn add_mod_n_commutative() {
        let a = small_scalar(0x1234_5678_9abc_def0);
        let b = small_scalar(0x0fed_cba9_8765_4321);
        assert_eq!(a.add_mod_n(&b), b.add_mod_n(&a));
    }

    // -----------------------------------------------------------------
    // sub_mod_n
    // -----------------------------------------------------------------

    #[test]
    fn sub_mod_n_no_wrap() {
        let c = small_scalar(10).sub_mod_n(&small_scalar(3));
        assert_eq!(c, small_scalar(7));
    }

    #[test]
    fn sub_mod_n_wraps() {
        // 2 - 3 ≡ n - 1 (mod n)
        let c = small_scalar(2).sub_mod_n(&small_scalar(3));
        assert_eq!(c, n_minus_one());
    }

    #[test]
    fn sub_mod_n_self_is_zero() {
        let a = small_scalar(0xc0ffee);
        let c = a.sub_mod_n(&a);
        assert_eq!(c, Scalar::ZERO);
    }

    #[test]
    fn add_sub_inverse() {
        let a = small_scalar(12345);
        let b = small_scalar(67890);
        let back = a.add_mod_n(&b).sub_mod_n(&b);
        assert_eq!(a, back);
    }

    #[test]
    fn add_sub_inverse_with_wrap() {
        let a = n_minus_one();
        let b = small_scalar(0x9999);
        let back = a.add_mod_n(&b).sub_mod_n(&b);
        assert_eq!(a, back);
    }

    // -----------------------------------------------------------------
    // Montgomery multiplication mod n
    // -----------------------------------------------------------------

    #[test]
    fn mont_mul_n_by_one() {
        let a = small_scalar(0x1234_5678_9abc_def0);
        let a_mont = a.to_montgomery_n();
        let one_mont = Scalar::ONE_MONT_N;
        let prod = a_mont.mont_mul_n(&one_mont);
        let result = prod.from_montgomery_n();
        assert_eq!(result, a);
    }

    #[test]
    fn mul_mod_n_small() {
        let c = small_scalar(6).mul_mod_n(&small_scalar(7));
        assert_eq!(c, small_scalar(42));
    }

    #[test]
    fn mul_mod_n_by_zero() {
        assert_eq!(small_scalar(999).mul_mod_n(&Scalar::ZERO), Scalar::ZERO);
        assert_eq!(Scalar::ZERO.mul_mod_n(&small_scalar(999)), Scalar::ZERO);
    }

    #[test]
    fn mul_mod_n_by_one() {
        let a = small_scalar(0xabcd_ef01_2345_6789);
        assert_eq!(a.mul_mod_n(&one()), a);
        assert_eq!(one().mul_mod_n(&a), a);
    }

    #[test]
    fn mul_mod_n_commutative() {
        let a = small_scalar(0x1111_2222_3333_4444);
        let b = small_scalar(0x5555_6666_7777_8888);
        assert_eq!(a.mul_mod_n(&b), b.mul_mod_n(&a));
    }

    #[test]
    fn mul_mod_n_n_minus_one_squared() {
        // (n-1)^2 ≡ 1 (mod n)   because (n-1) ≡ -1
        let c = n_minus_one().mul_mod_n(&n_minus_one());
        assert_eq!(c, one());
    }

    #[test]
    fn mul_mod_n_associative() {
        let m2 = small_scalar(2);
        let m3 = small_scalar(3);
        let m5 = small_scalar(5);
        let left = m2.mul_mod_n(&m3).mul_mod_n(&m5);
        let right = m2.mul_mod_n(&m3.mul_mod_n(&m5));
        assert_eq!(left, right);
    }

    // -----------------------------------------------------------------
    // inv_mod_n  (Fermat via Montgomery)
    // -----------------------------------------------------------------

    #[test]
    fn inv_of_zero_is_none() {
        assert!(Scalar::ZERO.inv_mod_n().is_none());
    }

    #[test]
    fn inv_mod_n_of_one_is_one() {
        let inv = one().inv_mod_n().unwrap();
        assert_eq!(inv, one());
    }

    #[test]
    fn inv_mod_n_of_n_minus_one_is_n_minus_one() {
        let inv = n_minus_one().inv_mod_n().unwrap();
        assert_eq!(inv, n_minus_one());
    }

    #[test]
    fn inv_then_mul_is_one() {
        for v in [2u64, 3, 7, 17, 0xDEAD_BEEF, 0xFEDC_BA98_7654_3210] {
            let a = small_scalar(v);
            let a_inv = a.inv_mod_n().unwrap();
            let prod = a.mul_mod_n(&a_inv);
            assert_eq!(prod, one(), "a · a^(-1) ≠ 1 for v = {v:#x}");
        }
    }

    #[test]
    fn inv_double_is_identity() {
        let a = small_scalar(0x1234_5678_9abc_def0);
        let a_inv = a.inv_mod_n().unwrap();
        let a_inv_inv = a_inv.inv_mod_n().unwrap();
        assert_eq!(a_inv_inv, a);
    }

    // -----------------------------------------------------------------
    // ct_lt_n correctness
    // -----------------------------------------------------------------

    #[test]
    fn ct_lt_n_boundary() {
        assert_eq!(Scalar::ZERO.ct_lt_n().unwrap_u8(), 1, "0 < n");
        assert_eq!(one().ct_lt_n().unwrap_u8(), 1, "1 < n");
        assert_eq!(n_minus_one().ct_lt_n().unwrap_u8(), 1, "n-1 < n");
        let n_scalar = Scalar { limbs: N_521_LIMBS };
        assert_eq!(n_scalar.ct_lt_n().unwrap_u8(), 0, "n ≮ n");
    }

    // -----------------------------------------------------------------
    // Cross-check with BigUint reference
    // -----------------------------------------------------------------

    #[test]
    fn inv_matches_biguint_reference() {
        use num_bigint::BigUint;
        use num_traits::{One, Zero};

        let n = BigUint::parse_bytes(
            b"6684878480953803875615041384236581248565144626959181331935475284049641722241859866323302474084432391273141914599968033004535887077643952596722791822942474231",
            10,
        ).unwrap();

        for v in [2u64, 7, 19, 0xDEAD_BEEF, 0x1234_5678_9ABC_DEF0] {
            let a = small_scalar(v);
            let ct_inv = a.inv_mod_n().unwrap();
            let ct_inv_bytes = ct_inv.to_bytes_be();

            let a_big = BigUint::from(v);
            let n_minus_2 = &n - BigUint::from(2u64);
            let ref_inv = a_big.modpow(&n_minus_2, &n);

            let ref_bytes = {
                let b = ref_inv.to_bytes_be();
                let mut padded = [0u8; FIELD_BYTES];
                let offset = FIELD_BYTES - b.len();
                padded[offset..].copy_from_slice(&b);
                padded
            };

            assert_eq!(ct_inv_bytes, ref_bytes, "inv_mod_n mismatch for v={v:#x}");
        }
    }

    #[test]
    fn mul_matches_biguint_reference() {
        use num_bigint::BigUint;

        let n = BigUint::parse_bytes(
            b"6684878480953803875615041384236581248565144626959181331935475284049641722241859866323302474084432391273141914599968033004535887077643952596722791822942474231",
            10,
        ).unwrap();

        let a = small_scalar(0xDEAD_BEEF);
        let b = small_scalar(0xCAFE_BABE);
        let ct_prod = a.mul_mod_n(&b);
        let ct_bytes = ct_prod.to_bytes_be();

        let a_big = BigUint::from(0xDEAD_BEEFu64);
        let b_big = BigUint::from(0xCAFE_BABEu64);
        let ref_prod = (&a_big * &b_big) % &n;

        let ref_bytes = {
            let rb = ref_prod.to_bytes_be();
            let mut padded = [0u8; FIELD_BYTES];
            let offset = FIELD_BYTES - rb.len();
            padded[offset..].copy_from_slice(&rb);
            padded
        };

        assert_eq!(ct_bytes, ref_bytes);
    }
}

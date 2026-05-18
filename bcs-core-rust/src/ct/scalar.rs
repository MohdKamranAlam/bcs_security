//! Scalar arithmetic modulo `n_521`.
//!
//! See `BCS_CT_DESIGN.md` § 3.
//!
//! ## Status
//!
//! - [x] type, byte (de)serialization
//! - [x] MSB-first bit iterator (constant 521 iterations)
//! - [x] `Zeroize` / `ZeroizeOnDrop`
//! - [ ] `add`, `sub`, `mul` mod n via Barrett reduction (v0.3.0, for EC-DSA-style sign)
//!
//! For the Montgomery ladder we only need bit-iteration, so this
//! minimal API is sufficient for `v0.2.0-ct` to start producing
//! ciphertexts.  Full arithmetic mod `n` arrives with the EC-DSA-style
//! sign path in `v0.3.0`.

use subtle::{Choice, ConstantTimeEq};
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::consts::{FIELD_BITS, FIELD_BYTES, N_521_LIMBS};
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

    // ---------------------------------------------------------------
    // Scalar arithmetic mod n_521  (CT — no secret-dependent branches)
    // ---------------------------------------------------------------

    /// Constant-time `(self + rhs) mod n_521`.
    ///
    /// Both inputs must be in `[0, n)`.  The output is in `[0, n)`.
    ///
    /// Algorithm: compute the 9-limb sum (may overflow into bit 521), then
    /// unconditionally subtract `n` using a bitmask derived from the overflow
    /// flag and from `ct_lt_n`.  No branches on secret data.
    pub fn add_mod_n(&self, rhs: &Self) -> Self {
        // --- Step 1: full-width addition (carry into bit 576) ---
        let mut sum = [0u64; 9];
        let mut carry: u64 = 0;
        for i in 0..9 {
            let (s1, c1) = self.limbs[i].overflowing_add(rhs.limbs[i]);
            let (s2, c2) = s1.overflowing_add(carry);
            sum[i] = s2;
            carry = (c1 as u64) | (c2 as u64);
        }

        // --- Step 2: decide whether to subtract n (CT) ---
        // `carry == 1` → sum >= 2^576 > n → must subtract.
        // `carry == 0` → sum might still be >= n; use `ct_lt_n`.
        //
        // BUG-FIX (v0.3.0): the previous version used `(!lt.unwrap_u8()) as u64`
        // to invert the `< n` flag, but `!` on a `u8` flips ALL 8 bits, so
        // `!1u8 = 254` and `!0u8 = 255` — neither is a valid {0,1} flag.
        // Use `^ 1u64` (or `1 - x`) to flip a {0,1} value.
        let sum_scalar = Self { limbs: sum };
        let lt = sum_scalar.ct_lt_n().unwrap_u8() as u64; // 1 iff sum < n, else 0
        let needs_sub: u64 = carry | (lt ^ 1u64); // 1 iff sum >= n

        // --- Step 3: unconditional conditional-subtract via bitmask ---
        // mask = 0xFFFF...FFFF if needs_sub, 0 if not
        let mask = needs_sub.wrapping_neg();
        let mut result = [0u64; 9];
        let mut borrow: u64 = 0;
        for i in 0..9 {
            let sub = N_521_LIMBS[i] & mask;
            let (d1, b1) = sum[i].overflowing_sub(sub);
            let (d2, b2) = d1.overflowing_sub(borrow);
            result[i] = d2;
            borrow = (b1 as u64) | (b2 as u64);
        }
        // borrow == 0 is guaranteed: sum - n < n when needs_sub was correct.
        Self { limbs: result }
    }

    /// Constant-time `(self - rhs) mod n_521`.
    ///
    /// Both inputs must be in `[0, n)`.  The output is in `[0, n)`.
    pub fn sub_mod_n(&self, rhs: &Self) -> Self {
        // Subtract with borrow.
        let mut diff = [0u64; 9];
        let mut borrow: u64 = 0;
        for i in 0..9 {
            let (d1, b1) = self.limbs[i].overflowing_sub(rhs.limbs[i]);
            let (d2, b2) = d1.overflowing_sub(borrow);
            diff[i] = d2;
            borrow = (b1 as u64) | (b2 as u64);
        }
        // If borrow == 1 the subtraction underflowed → add n back (CT).
        let mask = borrow.wrapping_neg(); // 0xFF..FF iff borrow
        let mut result = [0u64; 9];
        let mut carry: u64 = 0;
        for i in 0..9 {
            let add = N_521_LIMBS[i] & mask;
            let (s1, c1) = diff[i].overflowing_add(add);
            let (s2, c2) = s1.overflowing_add(carry);
            result[i] = s2;
            carry = (c1 as u64) | (c2 as u64);
        }
        Self { limbs: result }
    }

    /// Constant-time `(self * rhs) mod n_521`.
    ///
    /// Uses a double-and-add method over the bits of `rhs` with CT selection,
    /// so the loop body is independent of any secret bit.
    ///
    /// Complexity: 521 iterations × 2 `add_mod_n` calls.
    ///
    /// **v0.3.1 roadmap**: replace with Barrett-reduced 18-limb schoolbook
    /// multiplication for a ~100× throughput improvement.
    pub fn mul_mod_n(&self, rhs: &Self) -> Self {
        let mut result = Self::ZERO;
        for bit in rhs.bits_msb_first() {
            // Double
            result = result.add_mod_n(&result.clone());
            // CT-select: if bit==1, compute result+self; else result+0.
            // We compute both and pick with a bitmask.
            let added = result.add_mod_n(self);
            let mask = (bit as u64).wrapping_neg(); // 0xFF..FF iff bit==1
            let mut selected = [0u64; 9];
            for i in 0..9 {
                selected[i] = (added.limbs[i] & mask) | (result.limbs[i] & !mask);
            }
            result = Self { limbs: selected };
        }
        result
    }

    /// Constant-time Fermat inversion: `self^(n-2) mod n`.
    ///
    /// Valid because `n_521` is prime (Fermat's little theorem).
    /// Uses square-and-multiply via `mul_mod_n` — same bit loop, CT selection.
    ///
    /// Returns `ZERO` if `self == ZERO` (caller must validate inputs).
    pub fn inv_mod_n(&self) -> Self {
        // Exponent = n - 2.  We represent it by decrementing the limb copy of N.
        let mut exp_limbs = N_521_LIMBS;
        // Subtract 2: borrow chain through limbs.
        let (e0, b) = exp_limbs[0].overflowing_sub(2);
        exp_limbs[0] = e0;
        let mut carry_b = b as u64;
        for i in 1..9 {
            let (ei, bi) = exp_limbs[i].overflowing_sub(carry_b);
            exp_limbs[i] = ei;
            carry_b = bi as u64;
        }
        let exp = Self { limbs: exp_limbs };

        // Square-and-multiply.
        let mut result = {
            // 1 in scalar form: limbs all zero except limbs[0] = 1.
            let mut one = Self::ZERO;
            one.limbs[0] = 1;
            one
        };
        for bit in exp.bits_msb_first() {
            // Square
            result = result.mul_mod_n(&result.clone());
            // CT multiply by self if bit==1
            let multiplied = result.mul_mod_n(self);
            let mask = (bit as u64).wrapping_neg();
            let mut selected = [0u64; 9];
            for i in 0..9 {
                selected[i] = (multiplied.limbs[i] & mask) | (result.limbs[i] & !mask);
            }
            result = Self { limbs: selected };
        }
        result
    }
}

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
        assert_eq!(c.to_bytes_be(), small_scalar(10).to_bytes_be());
    }

    #[test]
    fn add_mod_n_zero_identity() {
        let a = small_scalar(0xdead_beef);
        let c = a.add_mod_n(&Scalar::ZERO);
        assert_eq!(c.to_bytes_be(), a.to_bytes_be());
        let c2 = Scalar::ZERO.add_mod_n(&a);
        assert_eq!(c2.to_bytes_be(), a.to_bytes_be());
    }

    #[test]
    fn add_mod_n_wraps_n_minus_one_plus_one() {
        // (n-1) + 1 = n ≡ 0 (mod n)
        let c = n_minus_one().add_mod_n(&one());
        assert_eq!(c.to_bytes_be(), Scalar::ZERO.to_bytes_be());
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
        assert_eq!(c.to_bytes_be(), expected.to_bytes_be());
        assert_in_range(&c);
    }

    #[test]
    fn add_mod_n_commutative() {
        let a = small_scalar(0x1234_5678_9abc_def0);
        let b = small_scalar(0x0fed_cba9_8765_4321);
        assert_eq!(
            a.add_mod_n(&b).to_bytes_be(),
            b.add_mod_n(&a).to_bytes_be()
        );
    }

    // -----------------------------------------------------------------
    // sub_mod_n
    // -----------------------------------------------------------------

    #[test]
    fn sub_mod_n_no_wrap() {
        let c = small_scalar(10).sub_mod_n(&small_scalar(3));
        assert_eq!(c.to_bytes_be(), small_scalar(7).to_bytes_be());
    }

    #[test]
    fn sub_mod_n_wraps() {
        // 2 - 3 ≡ n - 1 (mod n)
        let c = small_scalar(2).sub_mod_n(&small_scalar(3));
        assert_eq!(c.to_bytes_be(), n_minus_one().to_bytes_be());
    }

    #[test]
    fn sub_mod_n_self_is_zero() {
        let a = small_scalar(0xc0ffee);
        let c = a.sub_mod_n(&a);
        assert_eq!(c.to_bytes_be(), Scalar::ZERO.to_bytes_be());
    }

    #[test]
    fn add_sub_inverse() {
        let a = small_scalar(12345);
        let b = small_scalar(67890);
        let back = a.add_mod_n(&b).sub_mod_n(&b);
        assert_eq!(a.to_bytes_be(), back.to_bytes_be());
    }

    #[test]
    fn add_sub_inverse_with_wrap() {
        // (n-1) + b - b = n-1 even though intermediate wrapped.
        let a = n_minus_one();
        let b = small_scalar(0x9999);
        let back = a.add_mod_n(&b).sub_mod_n(&b);
        assert_eq!(a.to_bytes_be(), back.to_bytes_be());
    }

    // -----------------------------------------------------------------
    // mul_mod_n
    // -----------------------------------------------------------------

    #[test]
    fn mul_mod_n_small() {
        let c = small_scalar(6).mul_mod_n(&small_scalar(7));
        assert_eq!(c.to_bytes_be(), small_scalar(42).to_bytes_be());
    }

    #[test]
    fn mul_mod_n_by_zero() {
        let c = small_scalar(999).mul_mod_n(&Scalar::ZERO);
        assert_eq!(c.to_bytes_be(), Scalar::ZERO.to_bytes_be());
        let c2 = Scalar::ZERO.mul_mod_n(&small_scalar(999));
        assert_eq!(c2.to_bytes_be(), Scalar::ZERO.to_bytes_be());
    }

    #[test]
    fn mul_mod_n_by_one() {
        let a = small_scalar(0xabcd_ef01_2345_6789);
        let c = a.mul_mod_n(&one());
        assert_eq!(c.to_bytes_be(), a.to_bytes_be());
        let c2 = one().mul_mod_n(&a);
        assert_eq!(c2.to_bytes_be(), a.to_bytes_be());
    }

    #[test]
    fn mul_mod_n_commutative() {
        let a = small_scalar(0x1111_2222_3333_4444);
        let b = small_scalar(0x5555_6666_7777_8888);
        assert_eq!(
            a.mul_mod_n(&b).to_bytes_be(),
            b.mul_mod_n(&a).to_bytes_be()
        );
    }

    #[test]
    fn mul_mod_n_n_minus_one_squared() {
        // (n-1)^2 ≡ 1 (mod n)   because (n-1) ≡ -1
        let c = n_minus_one().mul_mod_n(&n_minus_one());
        assert_eq!(c.to_bytes_be(), one().to_bytes_be());
    }

    // -----------------------------------------------------------------
    // inv_mod_n  (Fermat)
    // -----------------------------------------------------------------

    #[test]
    fn inv_mod_n_roundtrip_small() {
        let a = small_scalar(17);
        let inv = a.inv_mod_n();
        let one_back = a.mul_mod_n(&inv);
        assert_eq!(one_back.to_bytes_be(), one().to_bytes_be());
    }

    #[test]
    fn inv_mod_n_of_one_is_one() {
        let inv = one().inv_mod_n();
        assert_eq!(inv.to_bytes_be(), one().to_bytes_be());
    }

    #[test]
    fn inv_mod_n_of_n_minus_one_is_n_minus_one() {
        // (n-1)·(n-1) ≡ 1 ⇒ (n-1)^(-1) = n-1
        let inv = n_minus_one().inv_mod_n();
        assert_eq!(inv.to_bytes_be(), n_minus_one().to_bytes_be());
    }

    #[test]
    fn inv_mod_n_random_values_roundtrip() {
        // Use a handful of fixed but non-trivial values.
        for v in [3u64, 5, 65537, 0xdead_beefu64, 0x1234_5678_9abc_def0u64] {
            let a = small_scalar(v);
            let inv = a.inv_mod_n();
            let back = a.mul_mod_n(&inv);
            assert_eq!(
                back.to_bytes_be(), one().to_bytes_be(),
                "a · a^(-1) ≠ 1 for v = {:#x}", v
            );
        }
    }

    // -----------------------------------------------------------------
    // ct_lt_n correctness
    // -----------------------------------------------------------------

    #[test]
    fn ct_lt_n_boundary() {
        assert_eq!(Scalar::ZERO.ct_lt_n().unwrap_u8(), 1, "0 < n");
        assert_eq!(one().ct_lt_n().unwrap_u8(), 1, "1 < n");
        assert_eq!(n_minus_one().ct_lt_n().unwrap_u8(), 1, "n-1 < n");
        // Constructing exactly n violates the invariant; check the raw scalar.
        let n_scalar = Scalar { limbs: N_521_LIMBS };
        assert_eq!(n_scalar.ct_lt_n().unwrap_u8(), 0, "n ≮ n");
    }
}

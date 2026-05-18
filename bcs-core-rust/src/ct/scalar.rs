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
    fn ct_lt_n(&self) -> Choice {
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
}

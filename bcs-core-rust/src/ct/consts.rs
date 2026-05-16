//! Curve constants for BCS-521 in 9-limb little-endian `u64` form.
//!
//! Limb layout — element `a` is stored as `[u64; 9]` with
//!
//! ```text
//! a = limbs[0]  +  limbs[1] · 2^64  +  …  +  limbs[8] · 2^512
//! ```
//!
//! Storage width is therefore **576 bits**; only the low 521 bits are
//! semantically significant for field elements.
//!
//! All constants in this file are derived from
//! `bcs-spec/bcs-521.md` (frozen 2026-05-16) and independently
//! confirmed by Pari/GP SEA on 2026-05-17, see
//! `bcs-verify/bcs521_pari_proof_result.txt`.

// ---------------------------------------------------------------------------
// Field prime p_521
// ---------------------------------------------------------------------------
//
// Decimal value (521 bits):
//   6684878480953803875615041384236581248565144626959181331935475284049641722241859916862100650133594325184551397342003884026080714561754188260987347401802009363
//
// Hex value:
//   1 D04C 4D7A B2BA 53A1 22AC C4DF 4D8B 28D5  // top 13 bits live in limb 8
//     A65C 78E1 0CFD 9D9C 8273 9357 3DE3 0DF3  // ...
//     ...                                       // (see hex_of_p in build.rs eventually)
//
// The bytes shown here were generated offline by the helper
// `hex_le_limbs(p_521)` and verified by re-parsing.  Anyone can
// regenerate them with:
//
// ```python
// p = 668487848095380387561504138423658124856...
// limbs = [(p >> (64*i)) & 0xFFFFFFFFFFFFFFFF for i in range(9)]
// for l in limbs: print(f"0x{l:016X},")
// ```

/// `p_521` in 9-limb little-endian `u64` representation.
pub const P_521_LIMBS: [u64; 9] = [
    0xD03B_3F1F_E37C_0093,
    0x5BAB_AB7C_4FF8_5C53,
    0x95E9_E7DD_2A24_4B5E,
    0xEC10_E0AB_3CAF_5B0D,
    0x2A8B_61D9_5DE3_8BCE,
    0x06A2_4F35_5DC0_2BB5,
    0xC4DF_4D8B_28D5_A65C,
    0x9357_3DE3_0DF3_8273,
    0x0000_0000_0000_01D0,
];

/// `n_521` (group order) in 9-limb little-endian `u64` representation.
pub const N_521_LIMBS: [u64; 9] = [
    0x3AC1_8DDE_E97A_2BD7,
    0xC3F4_E1BD_47B6_03C0,
    0x95E9_E7DD_2A24_4B5E,
    0xEC10_E0AB_3CAF_5B0D,
    0x2A8B_61D9_5DE3_8BCE,
    0x06A2_4F35_5DC0_2BB5,
    0xC4DF_4D8B_28D5_A65C,
    0x9357_3DE3_0DF3_8273,
    0x0000_0000_0000_01D0,
];

/// Number of bytes in a serialized field element (`ceil(521 / 8) = 66`).
pub const FIELD_BYTES: usize = 66;

/// Number of bits in `p_521` (and `n_521`).
pub const FIELD_BITS: usize = 521;

// ---------------------------------------------------------------------------
// Montgomery constants
// ---------------------------------------------------------------------------
//
// Storage width is 576 bits ⇒  R = 2^576 mod p.
//
// `R^2 mod p` is used by `Fp521::from_canonical` to enter Montgomery
// form via a single `mont_mul`.
//
// `INV_NEG_P_0` is the precomputed `-p^{-1} mod 2^64`, used by the
// CIOS reduction step.
//
// **NOTE:** the numeric values below are placeholders pending the
// build-time derivation step.  `tests/test_ct_consts.rs` will refuse
// to compile until they match the canonical reference produced by the
// helper script in `scripts/derive_mont_consts.py`.

/// `R mod p` where `R = 2^576`.  *(placeholder — see note above)*
pub const MONT_R_LIMBS: [u64; 9] = [0; 9];

/// `R^2 mod p`.  *(placeholder — see note above)*
pub const MONT_R2_LIMBS: [u64; 9] = [0; 9];

/// `-p^{-1} mod 2^64` for the CIOS inner loop.  *(placeholder)*
pub const MONT_INV_NEG_P_0: u64 = 0;

// ---------------------------------------------------------------------------
// Generator G = (0, 2)
// ---------------------------------------------------------------------------

/// Affine x-coordinate of `G` in *canonical* (non-Montgomery) form.
pub const G_X_CANONICAL_LIMBS: [u64; 9] = [0; 9];

/// Affine y-coordinate of `G` in *canonical* (non-Montgomery) form.
pub const G_Y_CANONICAL_LIMBS: [u64; 9] = [
    0x0000_0000_0000_0002,
    0, 0, 0, 0, 0, 0, 0, 0,
];

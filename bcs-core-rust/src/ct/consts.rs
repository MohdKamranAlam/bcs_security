//! Curve and Montgomery constants for BCS-521 in 9-limb little-endian
//! `u64` form.
//!
//! Limb layout — element `a` is stored as `[u64; 9]` with
//!
//! ```text
//! a = limbs[0]  +  limbs[1] · 2^64  +  …  +  limbs[8] · 2^512
//! ```
//!
//! Storage width is **576 bits**; only the low 521 bits are
//! semantically significant for field elements.
//!
//! ## Provenance
//!
//! Every constant below is the output of
//! `scripts/derive_mont_consts.py`, which takes only the canonical
//! decimal values of `p_521`, `n_521`, `G_X`, `G_Y` from
//! `bcs-spec/bcs-521.md` (frozen 2026-05-16, independently confirmed
//! by Pari/GP SEA on 2026-05-17 — see
//! `bcs-verify/bcs521_pari_proof_result.txt`).
//!
//! The integrity check `CT_CONSTS_SHA256` at the bottom of this file
//! lets `cargo test --features ct ct_consts_sha` verify that no
//! transcription error crept in.
//!
//! **DO NOT EDIT THE CONSTANTS BY HAND** — re-run the Python script
//! and paste its output.

/// Number of bytes in a serialized field element (`ceil(521 / 8) = 66`).
pub const FIELD_BYTES: usize = 66;

/// Number of bits in `p_521` (and `n_521`).
pub const FIELD_BITS: usize = 521;

// =========================================================================
// BEGIN AUTO  (output of scripts/derive_mont_consts.py @ 2026-05-17)
// =========================================================================

/// `p_521` (the field prime).
pub const P_521_LIMBS: [u64; 9] = [
    0x9D26D073AB604B13,
    0x0384CB1B9E1D1D8E,
    0x1F4DE8E1F388BE38,
    0x60FEB3A35BB4BAA4,
    0xBA2FE102829EC763,
    0x6D99A052C2AF01FB,
    0x3CAE6FDF39466130,
    0x94BD5F0B57F7B051,
    0x00000000000001F2,
];

/// `n_521` (the group order).
pub const N_521_LIMBS: [u64; 9] = [
    0x1ECF77E1DFDB0FF7,
    0x107F211A8D8E3CEE,
    0xED05D6FD2163DA78,
    0xF142C170EDBFD8EC,
    0xBA2FE102829EC762,
    0x6D99A052C2AF01FB,
    0x3CAE6FDF39466130,
    0x94BD5F0B57F7B051,
    0x00000000000001F2,
];

/// `R mod p` where `R = 2^576`.
pub const MONT_R_LIMBS: [u64; 9] = [
    0x0411D91D52AB6D97,
    0xAE081FA7C26EAD95,
    0x48A2AFB2AF2ACFE8,
    0xFFAA64649A21F9A3,
    0x67AFDDA99DF94573,
    0x0D20C0C014B5C5CD,
    0xD8FE5D9E429FE477,
    0x318A89CDE0EF2B72,
    0x0000000000000018,
];

/// `R^2 mod p` where `R = 2^576`.
pub const MONT_R2_LIMBS: [u64; 9] = [
    0x61AE5ABE8EE12A31,
    0xA254D4EA99EFDF53,
    0xD83F57C2E9A4DFD0,
    0x8A63FFC8880943FE,
    0xBB9F2FF2E99A5410,
    0x2E1E2D840392666C,
    0x5BE838A7E458E1B7,
    0x990AFF8C18172B24,
    0x00000000000000AA,
];

/// `-p^{-1} mod 2^64` for the CIOS reduction inner loop.
pub const MONT_INV_NEG_P_0: u64 = 0xAFF9CD6A47B2C8E5;

/// Affine x-coordinate of `G` in *canonical* (non-Montgomery) form.
pub const G_X_CANONICAL_LIMBS: [u64; 9] = [0; 9];

/// Affine y-coordinate of `G` in *canonical* (non-Montgomery) form.
pub const G_Y_CANONICAL_LIMBS: [u64; 9] = [
    0x0000000000000002, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// Affine x-coordinate of `G` in *Montgomery* form (`G_X · R mod p`).
pub const G_X_MONT_LIMBS: [u64; 9] = [0; 9];

/// Affine y-coordinate of `G` in *Montgomery* form (`G_Y · R mod p`).
pub const G_Y_MONT_LIMBS: [u64; 9] = [
    0x0823B23AA556DB2E,
    0x5C103F4F84DD5B2A,
    0x91455F655E559FD1,
    0xFF54C8C93443F346,
    0xCF5FBB533BF28AE7,
    0x1A418180296B8B9A,
    0xB1FCBB3C853FC8EE,
    0x6315139BC1DE56E5,
    0x0000000000000030,
];

// ---- Short Weierstrass form  y² = x'³ + A·x' + B  ----
// (substitution  x = x' + 2/3  applied to  y² = x³ − 2x² + 5x + 4)
//   A     = 11/3   mod p
//   B     = 182/27 mod p
//   G'_x  = −2/3   mod p
//   G'_y  =  2     (unchanged)
// The change of variables is verified offline: G'_y² ≡ G'_x³ + A·G'_x + B (mod p).

/// `A` in Montgomery form (short-Weierstrass coefficient).
pub const SHORT_A_MONT_LIMBS: [u64; 9] = [
    0x0EEC1C162F1F3C7F,
    0xD3731EBC7395D1CD,
    0xB4FF2EE48247A4FF,
    0x541B701B8A7C9356,
    0x7C2F82189891FEA9,
    0x3022C2C04BEFD546,
    0x1BA4AC999EF4F05F,
    0xB5A6A3F2E36CF4A5,
    0x0000000000000058,
];

/// `B` in Montgomery form (short-Weierstrass coefficient).
pub const SHORT_B_MONT_LIMBS: [u64; 9] = [
    0xDE09FCFF2D7DB95F,
    0x2DFABA449F2CA567,
    0xB1AE72B3EDEF5A0D,
    0x7CE46ED31A9309D8,
    0x156F0E35CFA8C14B,
    0x6094BA7FB00D198D,
    0xCAECC245CAB58CB7,
    0x46A29AA871D7A204,
    0x0000000000000149,
];

/// `3 · B mod p` in Montgomery form.  Used by the Renes-Costello-Batina
/// complete addition / doubling formulas (Algorithms 1 & 3 of
/// EUROCRYPT 2016 §3).
pub const SHORT_B3_MONT_LIMBS: [u64; 9] = [
    0xFCF72689DD18E10A,
    0x866B63B23F68D2A8,
    0xF5BD6F39D6454FEF,
    0x15AE98D5F40462E5,
    0x861D499EEC5B7C7F,
    0xB4248F2C4D784AAB,
    0x2417D6F226DA44F5,
    0x3F2A70EDFD8F35BD,
    0x00000000000001E9,
];

/// Affine x-coordinate of `G'` in Montgomery form (short-Weierstrass chart).
pub const SHORT_GX_MONT_LIMBS: [u64; 9] = [
    0x9A703FB574435759,
    0x3A2A0B5671D35480,
    0x4436C9157EC188F2,
    0x0BE270B59F9E1437,
    0x750FF7E6C3F89916,
    0x64D91FD2B4E07E1D,
    0xAC04DC2062311E36,
    0x73B6588217583E04,
    0x00000000000001E2,
];

/// Affine y-coordinate of `G'` in Montgomery form (short-Weierstrass chart).
pub const SHORT_GY_MONT_LIMBS: [u64; 9] = [
    0x0823B23AA556DB2E,
    0x5C103F4F84DD5B2A,
    0x91455F655E559FD1,
    0xFF54C8C93443F346,
    0xCF5FBB533BF28AE7,
    0x1A418180296B8B9A,
    0xB1FCBB3C853FC8EE,
    0x6315139BC1DE56E5,
    0x0000000000000030,
];

/// SHA-256 of the canonical Rust source emitted by
/// `scripts/derive_mont_consts.py` covering all of the constants
/// above.  Used by `tests/test_ct_consts.rs` to detect manual
/// transcription errors.
pub const CT_CONSTS_SHA256: &str =
    "77eef76902afaa70f5a53d7cc0896134cb31012e9376fc042401ed9e83c302fd";

// =========================================================================
// END AUTO
// =========================================================================

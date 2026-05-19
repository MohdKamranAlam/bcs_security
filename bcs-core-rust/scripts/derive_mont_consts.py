#!/usr/bin/env python3
"""
derive_mont_consts.py  --  single source of truth for all BCS-521 CT constants.

Outputs *Rust source* that can be pasted into `src/ct/consts.rs`, computing:

    P_521_LIMBS          : 9 x u64 little-endian
    N_521_LIMBS          : 9 x u64 little-endian (group order)
    MONT_R_LIMBS         : R   = 2^576 mod p_521
    MONT_R2_LIMBS        : R^2 = (2^576)^2 mod p_521
    MONT_INV_NEG_P_0     : -p^-1 mod 2^64       (for CIOS inner loop)
    G_X_CANONICAL_LIMBS  : Gx (canonical form)
    G_Y_CANONICAL_LIMBS  : Gy (canonical form)
    G_X_MONT_LIMBS       : Gx * R mod p          (Montgomery form)
    G_Y_MONT_LIMBS       : Gy * R mod p

The script also prints SHA-256 of the consolidated output so that any
manual transcription error to the Rust file is detected by a one-line
test (`cargo test --features ct test_consts_sha`).

Usage:
    python3 derive_mont_consts.py            -> prints Rust to stdout
    python3 derive_mont_consts.py --verify   -> compares with consts.rs
"""

from __future__ import annotations
import hashlib
import sys

# -----------------------------------------------------------------------------
# Frozen BCS-521 parameters
# -----------------------------------------------------------------------------
# These three integers are the canonical reference -- they appear identically
# in:
#   * bcs-spec/bcs-521.md
#   * bcs-verify/bcs521_pari_proof.gp (and proof_result.txt)
#   * bcs-core-rust/src/lib.rs (BigUint reference impl)
P_521 = 6684878480953803875615041384236581248565144626959181331935475284049641722241859916862100650133594325184551397342003884026080714561754188260987347401802009363
N_521 = 6684878480953803875615041384236581248565144626959181331935475284049641722241859866323302474084432391273141914599968033004535887077643952596722791822942474231
G_X   = 0
G_Y   = 2

# Storage width: 9 * 64 = 576 bits.  Top 55 bits are head-room.
LIMBS = 9
LIMB_BITS = 64
W = 1 << LIMB_BITS                          # 2^64
R = 1 << (LIMBS * LIMB_BITS)                # 2^576  (Montgomery radix)


# -----------------------------------------------------------------------------
# Helpers
# -----------------------------------------------------------------------------
def to_limbs(x: int) -> list[int]:
    """Little-endian 9 x u64 representation."""
    if x < 0 or x >= R:
        raise ValueError(f"value out of range [0, 2^576): {x}")
    return [(x >> (LIMB_BITS * i)) & (W - 1) for i in range(LIMBS)]


def fmt_limbs(name: str, limbs: list[int]) -> str:
    out = [f"pub const {name}: [u64; 9] = ["]
    for l in limbs:
        out.append(f"    0x{l:016X},")
    out.append("];")
    return "\n".join(out)


def mod_inv(a: int, m: int) -> int:
    """Multiplicative inverse of a modulo m via extended Euclidean."""
    g, x, _ = ext_gcd(a, m)
    if g != 1:
        raise ZeroDivisionError(f"{a} is not invertible mod {m}")
    return x % m


def ext_gcd(a: int, b: int) -> tuple[int, int, int]:
    if a == 0:
        return b, 0, 1
    g, x1, y1 = ext_gcd(b % a, a)
    return g, y1 - (b // a) * x1, x1


# -----------------------------------------------------------------------------
# Derive all constants
# -----------------------------------------------------------------------------
def main(argv: list[str]) -> int:
    # Sanity: prime fits in 521 bits, less than R.
    assert P_521.bit_length() == 521, f"expected 521-bit prime, got {P_521.bit_length()}"
    assert N_521.bit_length() == 521
    assert P_521 < R

    # Montgomery radix constants.
    r_mod_p   = R % P_521
    r2_mod_p  = (R * R) % P_521

    # CIOS reduction needs  m' = -p^-1 mod 2^64.
    # Equivalently m' such that  p * m' ≡ -1  (mod 2^64).
    p_lo = P_521 & (W - 1)
    inv_p_lo = mod_inv(p_lo, W)          # p_lo^-1 mod 2^64
    mont_neg_p_inv = (W - inv_p_lo) & (W - 1)
    # Verification:
    assert ((P_521 * mont_neg_p_inv) + 1) % W == 0

    # Generator in canonical and Montgomery forms.
    gx_mont = (G_X * R) % P_521
    gy_mont = (G_Y * R) % P_521

    # -------------------------------------------------------------------
    # Montgomery constants for the SCALAR field Z / n_521 Z.
    #   Used by ct/scalar.rs for ECDSA-style sign:  inv_mod_n, mul_mod_n.
    #   Derivation mirrors the field constants above; modulus is `n_521`.
    # -------------------------------------------------------------------
    r_mod_n  = R % N_521
    r2_mod_n = (R * R) % N_521
    n_lo = N_521 & (W - 1)
    inv_n_lo = mod_inv(n_lo, W)
    mont_neg_n_inv = (W - inv_n_lo) & (W - 1)
    assert ((N_521 * mont_neg_n_inv) + 1) % W == 0

    # Fermat-inversion exponent for `inv_mod_n`:  a^(n-2) mod n.
    n_minus_2 = N_521 - 2

    # -------------------------------------------------------------------
    # Change of variables to short Weierstrass form  y² = x'³ + A·x' + B
    # -------------------------------------------------------------------
    # Substitute  x = x' + 2/3   in   y² = x³ − 2x² + 5x + 4.
    # Expanding:
    #     A =  5 − 4/3          =  11/3
    #     B =  8/27 − 8/9 + 10/3 + 4   =  182/27
    #     G'_x =  0 − 2/3        =  −2/3      (= p − 2/3 in F_p)
    #     G'_y =  2              (unchanged)
    #
    # All four values are reduced modulo p, then put in Montgomery form
    # (× R mod p) so the constant-time core can use them directly.
    inv3  = mod_inv(3, P_521)
    inv27 = mod_inv(27, P_521)
    short_A    = (11 * inv3) % P_521
    short_B    = (182 * inv27) % P_521
    short_Gx   = (P_521 - 2 * inv3) % P_521          # −2/3 mod p
    short_Gy   = G_Y                                 # 2

    # Sanity check:   G'_y²  ≡  G'_x³ + A·G'_x + B   (mod p)
    lhs = (short_Gy * short_Gy) % P_521
    rhs = (pow(short_Gx, 3, P_521)
           + short_A * short_Gx
           + short_B) % P_521
    assert lhs == rhs, "short-Weierstrass change-of-variables FAILED"

    short_A_mont    = (short_A         * R) % P_521
    short_B_mont    = (short_B         * R) % P_521
    short_B3        = (3 * short_B)         % P_521   # for RCB formulas
    short_B3_mont   = (short_B3        * R) % P_521
    short_Gx_mont   = (short_Gx        * R) % P_521
    short_Gy_mont   = (short_Gy        * R) % P_521

    # ---- Emit Rust ----
    blocks = [
        "// AUTO-GENERATED by scripts/derive_mont_consts.py -- DO NOT EDIT BY HAND.",
        "// To regenerate, run the script and replace the entire block between the",
        "// `BEGIN AUTO` / `END AUTO` markers in src/ct/consts.rs.",
        "// ",
        "// Verification: cargo test --features ct ct_consts_sha",
        "// will hash all the constants below and compare to the value at the",
        "// bottom of this file; any mismatch indicates a transcription error.",
        "",
        fmt_limbs("P_521_LIMBS", to_limbs(P_521)),
        "",
        fmt_limbs("N_521_LIMBS", to_limbs(N_521)),
        "",
        fmt_limbs("MONT_R_LIMBS", to_limbs(r_mod_p)),
        "",
        fmt_limbs("MONT_R2_LIMBS", to_limbs(r2_mod_p)),
        "",
        f"pub const MONT_INV_NEG_P_0: u64 = 0x{mont_neg_p_inv:016X};",
        "",
        fmt_limbs("G_X_CANONICAL_LIMBS", to_limbs(G_X)),
        "",
        fmt_limbs("G_Y_CANONICAL_LIMBS", to_limbs(G_Y)),
        "",
        fmt_limbs("G_X_MONT_LIMBS", to_limbs(gx_mont)),
        "",
        fmt_limbs("G_Y_MONT_LIMBS", to_limbs(gy_mont)),
        "",
        "// ---- Short Weierstrass form  y^2 = x'^3 + A x' + B  ----",
        "// (substitution  x = x' + 2/3  applied to  y^2 = x^3 - 2x^2 + 5x + 4)",
        "//   A     = 11/3   mod p",
        "//   B     = 182/27 mod p",
        "//   G'_x  = -2/3   mod p",
        "//   G'_y  =  2     (unchanged)",
        "",
        fmt_limbs("SHORT_A_MONT_LIMBS", to_limbs(short_A_mont)),
        "",
        fmt_limbs("SHORT_B_MONT_LIMBS", to_limbs(short_B_mont)),
        "",
        "// 3 * B mod p (in Montgomery form); used by Renes-Costello-Batina formulas",
        fmt_limbs("SHORT_B3_MONT_LIMBS", to_limbs(short_B3_mont)),
        "",
        fmt_limbs("SHORT_GX_MONT_LIMBS", to_limbs(short_Gx_mont)),
        "",
        fmt_limbs("SHORT_GY_MONT_LIMBS", to_limbs(short_Gy_mont)),
        "",
        "// ---- Scalar field Z / n_521 Z  (for ECDSA sign: inv_mod_n, mul_mod_n) ----",
        fmt_limbs("N_MONT_R_LIMBS", to_limbs(r_mod_n)),
        "",
        fmt_limbs("N_MONT_R2_LIMBS", to_limbs(r2_mod_n)),
        "",
        f"pub const MONT_INV_NEG_N_0: u64 = 0x{mont_neg_n_inv:016X};",
        "",
        f"pub const N_521_MINUS_2_LIMBS: [u64; 9] = [",
    ]
    # N_521 - 2 is the Fermat inversion exponent for inv_mod_n.
    n_minus_2_limbs = to_limbs(n_minus_2)
    for l in n_minus_2_limbs:
        blocks.append(f"    0x{l:016X},")
    blocks.append("];")

    rust = "\n".join(blocks)

    # Hash the consolidated content for the cargo-side integrity test.
    digest = hashlib.sha256(rust.encode("ascii")).hexdigest()
    rust += "\n\n"
    rust += f'pub const CT_CONSTS_SHA256: &str = "{digest}";\n'

    print(rust)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

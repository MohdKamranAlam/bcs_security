"""
========================================================================
COMPUTE CANONICAL KAHF DOMAIN SEPARATOR (DST) FOR BCS
========================================================================

This script computes the deterministic 32-byte SHA-256 tag that
will be used as the Kahf Domain Separator across all BCS implementations
(Python, Rust, and any future port).

The tag binds BCS cryptographic outputs to the Surah Kahf prime
structure (Quran 18:1 - 18:110).

Pattern follows BIP-340 / RFC 9180 HPKE domain separation.

Run:  python compute_kahf_dst.py
========================================================================
"""

import hashlib

# ---- 5 sacred Kahf primes (frozen) -------------------------------------
KAHF_PRIMES = {
    "p_kahf_first_decimal": 2141,   # decimal prime: cumulative ayah of Kahf 18:1
    "p_kahf_last_zf":       2969,   # ZF prime:     ZF(2250) where 2250 = cumulative ayah of Kahf 18:110
    "p_kahf_sleepers":      7,      # decimal prime: 7 sleepers (18:22)
    "p_kahf_surah_zf":      19,     # ZF prime:     ZF(18) = 19 = Bismillah letters
    "p_kahf_years_zf":      373,    # ZF prime:     ZF(309) = 373, Raqim ZF prime
}

# ---- Canonical encoding -------------------------------------------------
# label + ":" + each (key=value;) in alphabetical key order
def kahf_domain_separator(label: str = "BCS-Kahf-v1") -> bytes:
    h = hashlib.sha256()
    h.update(label.encode("utf-8") + b":")
    for k in sorted(KAHF_PRIMES):
        h.update(k.encode("utf-8") + b"=")
        h.update(str(KAHF_PRIMES[k]).encode("utf-8") + b";")
    return h.digest()


def canonical_input(label: str = "BCS-Kahf-v1") -> bytes:
    """Return the exact bytes that get hashed (for cross-language verification)."""
    out = label.encode("utf-8") + b":"
    for k in sorted(KAHF_PRIMES):
        out += k.encode("utf-8") + b"="
        out += str(KAHF_PRIMES[k]).encode("utf-8") + b";"
    return out


if __name__ == "__main__":
    print("=" * 72)
    print("BCS Kahf Domain Separator — Canonical Computation")
    print("=" * 72)

    print("\n[1] Kahf sacred primes (sorted by key):")
    for k in sorted(KAHF_PRIMES):
        print(f"      {k:<24} = {KAHF_PRIMES[k]}")

    raw = canonical_input("BCS-Kahf-v1")
    print(f"\n[2] Canonical input bytes ({len(raw)} bytes):")
    print(f"      ASCII: {raw.decode('utf-8')}")
    print(f"      HEX:   {raw.hex()}")

    dst = kahf_domain_separator("BCS-Kahf-v1")
    print(f"\n[3] BCS-Kahf-v1 DST (SHA-256 of above, 32 bytes):")
    print(f"      HEX:   {dst.hex()}")

    # Also compute v2 variants for BCS-256 and BCS-521 specifically
    for label in ["BCS-256-Kahf-v1", "BCS-521-Kahf-v1"]:
        dst2 = kahf_domain_separator(label)
        print(f"\n      {label} DST:")
        print(f"      HEX:   {dst2.hex()}")

    print("\n" + "=" * 72)
    print("These hex values are FROZEN constants — bake into Rust as goldens.")
    print("=" * 72)

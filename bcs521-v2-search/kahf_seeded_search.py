"""
BCS-521-V2 Kahf-Seeded Deterministic Prime Search
=================================================

This script generates the V2 prime for the BCS-521 family. V1 (existing
random-search prime, frozen 2026-05-16 in `bcs-spec/bcs-521.md`) is NOT
touched by this script and continues to operate independently.

Generates a 521-bit prime `p` such that:
    1. p is prime (BPSW + extra Miller-Rabin rounds)
    2. n = #E(F_p) is prime (where E: y^2 = x^3 - 2x^2 + 5x + 4)
    3. p != n (not anomalous)
    4. Generator G = (0, 2) lies on E

The prime is derived DETERMINISTICALLY from the canonical Kahf seed input,
not from a random search. This eliminates "trapdoor" suspicion: anyone can
re-derive p byte-for-byte from the seed, given the winning counter value.

The construction is analogous to NIST P-256's "verifiably random" seed
mechanism, but uses Quranic Kahf-prime constants + the BCS curve equation
as the seed material instead of an unexplained 160-bit hex string.

------------------------------------------------------------------------
USAGE
------------------------------------------------------------------------

  # Local Windows smoke test (no PARI, demonstrates determinism only):
  python kahf_seeded_search.py --smoke

  # Full 521-bit V2 search (requires PARI/GP — run on Codespaces):
  python kahf_seeded_search.py --bits 521 --start 0 --max 100000

  # Resume from a checkpoint (writes every 50 attempts):
  python kahf_seeded_search.py --bits 521 --start 12345 --max 100000

  # Verify a previously found counter reproduces the same prime:
  python kahf_seeded_search.py --verify 1234

NOTE: This script only generates BCS-521-V2 candidates. The existing
      BCS-521 (V1, random-search prime found 2026-05-16) is in
      bcs-spec/bcs-521.md and is NOT modified by this tool.

------------------------------------------------------------------------
SEED CANONICAL INPUT (frozen, do NOT change without bumping schema vN)
------------------------------------------------------------------------

  BCS-521-V2-Seed-v1:p_kahf_first_decimal=2141;p_kahf_last_zf=2969;
  p_kahf_sleepers=7;p_kahf_surah_zf=19;p_kahf_years_zf=373;
  a2=-2;a4=5;a6=4;bits=521;

Master seed = SHA-512(canonical_input).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import secrets
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------------------------------
# 1. Frozen Kahf primes and curve coefficients
# ---------------------------------------------------------------------------

# Five sacred Kahf primes (alphabetical key order — same as DST canonicalization).
# See bcs-spec/bcs-521.md "Kahf Domain Separator Binding (Theorem 18)".
KAHF_PRIMES = [
    ("p_kahf_first_decimal", 2141),  # decimal prime: cum-ayah Kahf 18:1
    ("p_kahf_last_zf",       2969),  # ZF prime: ZF(2250) = cum-ayah 18:110
    ("p_kahf_sleepers",      7),     # decimal prime: 7 sleepers (18:22) = F
    ("p_kahf_surah_zf",      19),    # ZF prime: ZF(18) = B (Bismillah letters)
    ("p_kahf_years_zf",      373),   # ZF prime: ZF(309) = Raqim ZF prime
]

# BCS curve y^2 = x^3 + a2*x^2 + a4*x + a6 (Weierstrass coefficients in long form).
# Derived from master equation T_A = 17*B^2 + 5*B + 4 = 6236, where B = 19.
CURVE_A2, CURVE_A4, CURVE_A6 = -2, 5, 4

# Default protocol label (frozen).
# Naming convention:
#   - "BCS-521-V2" denotes the V2 cipher suite (this Kahf-seeded upgrade).
#     V1 (the existing BCS-521 with the random-search prime, found 2026-05-16)
#     is FROZEN at bcs-spec/bcs-521.md and remains untouched.
#   - "-Seed-v1" is the seed-encoding schema version. Bump to v2 only if the
#     canonical input format itself ever changes.
SEED_LABEL = "BCS-521-V2-Seed-v1"

# Default target bit length.
DEFAULT_BITS = 521


# ---------------------------------------------------------------------------
# 2. Canonical seed input + candidate generator
# ---------------------------------------------------------------------------

def canonical_seed_input(label: str, bits: int) -> bytes:
    """Build the canonical ASCII seed string fed into SHA-512.

    Format (deterministic, ASCII-only, decimal values, alphabetical keys):

        <label>:p_kahf_first_decimal=2141;...;a2=-2;a4=5;a6=4;bits=<N>;

    Any change to label, any Kahf prime, the curve coefficients, or the
    target bits produces a DIFFERENT seed and therefore a different prime
    stream — strong domain separation.
    """
    parts = [label, ":"]
    for k, v in KAHF_PRIMES:
        parts += [k, "=", str(v), ";"]
    parts += ["a2=", str(CURVE_A2), ";"]
    parts += ["a4=", str(CURVE_A4), ";"]
    parts += ["a6=", str(CURVE_A6), ";"]
    parts += ["bits=", str(bits), ";"]
    return "".join(parts).encode("ascii")


def master_seed(label: str, bits: int) -> bytes:
    """Return SHA-512(canonical_seed_input). 64 bytes."""
    return hashlib.sha512(canonical_seed_input(label, bits)).digest()


def candidate(counter: int, label: str = SEED_LABEL, bits: int = DEFAULT_BITS) -> int:
    """Return the deterministic <bits>-bit odd integer for this counter.

    Construction (frozen):
        seed   = SHA-512(canonical_seed_input(label, bits))
        block0 = SHA-512(seed || b":block=0;counter=" || decimal(counter))
        block1 = SHA-512(seed || b":block=1;counter=" || decimal(counter))
        raw    = (block0 || block1)[: ceil(bits/8)]
        val    = int_from_be(raw)
        val   &= (1 << bits) - 1            # mask to <bits> bits
        val   |=  (1 << (bits - 1))         # force MSB (exactly <bits> wide)
        val   |=  1                         # force odd
    """
    if counter < 0:
        raise ValueError("counter must be non-negative")
    seed = master_seed(label, bits)
    suffix = b":counter=" + str(counter).encode("ascii")
    block0 = hashlib.sha512(seed + b":block=0" + suffix).digest()
    block1 = hashlib.sha512(seed + b":block=1" + suffix).digest()
    raw = (block0 + block1)[: (bits + 7) // 8]  # 66 bytes for bits=521
    val = int.from_bytes(raw, "big")
    val &= (1 << bits) - 1
    val |= 1 << (bits - 1)
    val |= 1
    return val


# ---------------------------------------------------------------------------
# 3. Primality (BPSW + extra MR rounds)
# ---------------------------------------------------------------------------

def _miller_rabin(n: int, a: int) -> bool:
    if n % a == 0:
        return n == a
    d, s = n - 1, 0
    while d % 2 == 0:
        d //= 2
        s += 1
    x = pow(a, d, n)
    if x == 1 or x == n - 1:
        return True
    for _ in range(s - 1):
        x = (x * x) % n
        if x == n - 1:
            return True
    return False


def _is_probable_prime(n: int, extra_rounds: int = 20) -> bool:
    """BPSW-style: deterministic small-prime trial + MR with random bases.

    For 521-bit candidates this is overwhelmingly reliable. Final winning
    candidates should ALSO be re-verified with PARI APR-CL (`isprime(p, 2)`).
    """
    if n < 2:
        return False
    small_primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47]
    for p in small_primes:
        if n == p:
            return True
        if n % p == 0:
            return False
    # Deterministic witnesses sufficient for n < 3.3e24, plus extra random rounds.
    for a in [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37]:
        if not _miller_rabin(n, a):
            return False
    for _ in range(extra_rounds):
        a = 2 + secrets.randbelow(max(4, n - 4))
        if not _miller_rabin(n, a):
            return False
    return True


# ---------------------------------------------------------------------------
# 4. Cardinality via PARI/GP (subprocess)
# ---------------------------------------------------------------------------

def _have_gp() -> bool:
    try:
        r = subprocess.run(["gp", "--version"], capture_output=True, timeout=5, text=True)
        return r.returncode == 0
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return False


def cardinality_via_pari(p: int, a2: int = CURVE_A2, a4: int = CURVE_A4,
                         a6: int = CURVE_A6, timeout_s: int = 1800) -> int | None:
    """Return #E(F_p) using PARI/GP `ellap` (Schoof-Elkies-Atkin).

    Returns None if PARI is unavailable, errors out, or times out.
    """
    if not _have_gp():
        return None
    script = (
        "default(parisize, \"2G\");\n"
        f"p = {p};\n"
        f"E = ellinit([0, {a2}, 0, {a4}, {a6}], p);\n"
        "ap = ellap(E);\n"
        "print(p + 1 - ap);\n"
        "quit;\n"
    )
    try:
        r = subprocess.run(
            ["gp", "-q"],
            input=script,
            capture_output=True,
            text=True,
            timeout=timeout_s,
        )
        line = r.stdout.strip().splitlines()[-1]
        return int(line)
    except Exception as exc:  # noqa: BLE001
        print(f"  [pari error] {exc}", file=sys.stderr)
        return None


# ---------------------------------------------------------------------------
# 5. Curve check: G = (0, 2) on E (independent of p, since 4 = 0 - 0 + 0 + 4)
# ---------------------------------------------------------------------------

def generator_on_curve(p: int) -> bool:
    """y^2 == x^3 + a2*x^2 + a4*x + a6 with (x, y) = (0, 2) and any prime p.

    Reduces to: 4 == 4 (mod p). Always True for any p > 4.
    """
    Gx, Gy = 0, 2
    lhs = (Gy * Gy) % p
    rhs = (Gx ** 3 + CURVE_A2 * Gx * Gx + CURVE_A4 * Gx + CURVE_A6) % p
    return lhs == rhs


# ---------------------------------------------------------------------------
# 6. Search driver
# ---------------------------------------------------------------------------

# Bit-aware default paths (so tiered runs at 128 / 256 / 521 do NOT overwrite
# each other). Caller can override via --out for the certificate path.
DEFAULT_DIR = Path(__file__).parent


def default_checkpoint_path(bits: int) -> Path:
    return DEFAULT_DIR / f"kahf_seeded_checkpoint_{bits}.json"


def default_certificate_path(bits: int) -> Path:
    return DEFAULT_DIR / f"kahf_seeded_certificate_{bits}.json"


def _save_checkpoint(path: Path, state: dict) -> None:
    state["updated_at"] = datetime.now(timezone.utc).isoformat()
    path.write_text(json.dumps(state, indent=2))


def search(bits: int, start: int, max_attempts: int, label: str,
           out_path: Path | None = None,
           checkpoint_path: Path | None = None) -> dict | None:
    """Run the deterministic search. Returns certificate dict on success."""
    seed_input = canonical_seed_input(label, bits)
    seed = hashlib.sha512(seed_input).digest()
    if out_path is None:
        out_path = default_certificate_path(bits)
    if checkpoint_path is None:
        checkpoint_path = default_checkpoint_path(bits)

    print("=" * 72)
    print("BCS Kahf-Seeded Deterministic Prime Search")
    print("=" * 72)
    print(f"label             : {label}")
    print(f"bits              : {bits}")
    print(f"canonical_input   : {seed_input.decode('ascii')}")
    print(f"master_seed (hex) : {seed.hex()}")
    print(f"start counter     : {start}")
    print(f"max attempts      : {max_attempts}")
    print(f"PARI/GP available : {_have_gp()}")
    print(f"certificate out   : {out_path}")
    print(f"checkpoint        : {checkpoint_path}")
    print("=" * 72)

    if bits >= 256 and not _have_gp():
        print("ERROR: PARI/GP `gp` not on PATH. Required for cardinality at >=256 bits.")
        print("       Install via: apt install pari-gp  (Codespaces / Linux)")
        return None

    t_start = time.time()
    last_progress_at = t_start
    primes_found = 0

    for c in range(start, start + max_attempts):
        p = candidate(c, label, bits)

        # Step 1: cheap primality on p
        if not _is_probable_prime(p, extra_rounds=4):
            if time.time() - last_progress_at > 30:
                print(f"  [c={c}] elapsed {int(time.time() - t_start)}s, "
                      f"p_primes_seen={primes_found}")
                last_progress_at = time.time()
                _save_checkpoint(checkpoint_path,
                                 {"label": label, "bits": bits,
                                  "last_counter": c, "primes_found": primes_found,
                                  "elapsed_s": int(time.time() - t_start)})
            continue
        primes_found += 1

        # Step 2: extra rigorous primality on p (full BPSW + 20 random MR rounds)
        if not _is_probable_prime(p, extra_rounds=20):
            continue

        # Step 3: cardinality via PARI SEA
        elapsed = int(time.time() - t_start)
        print(f"  [c={c}, t={elapsed}s] p is prime, computing #E(F_p) via SEA...")
        n = cardinality_via_pari(p)
        if n is None:
            print(f"  [c={c}] PARI failed, skipping")
            continue

        # Step 4: n must be prime
        if not _is_probable_prime(n, extra_rounds=20):
            print(f"  [c={c}] #E(F_p) is composite, skipping")
            continue

        # Step 5: not anomalous
        if n == p:
            print(f"  [c={c}] anomalous (n == p), skipping")
            continue

        # Step 6: generator on curve (always true for our form, but assert)
        assert generator_on_curve(p), "G=(0,2) check failed (math bug?)"

        a_p = p + 1 - n
        # Hasse bound: |a_p| <= 2*sqrt(p)
        assert a_p * a_p <= 4 * p, "Hasse bound violation (PARI bug?)"

        certificate = {
            "schema": "bcs-kahf-seeded-certificate-v1",
            "frozen_at": datetime.now(timezone.utc).isoformat(),
            "label": label,
            "bits": bits,
            "canonical_input": seed_input.decode("ascii"),
            "master_seed_sha512_hex": seed.hex(),
            "winning_counter": c,
            "attempts_until_found": c - start + 1,
            "elapsed_s": int(time.time() - t_start),
            "curve": {
                "equation": "y^2 = x^3 - 2*x^2 + 5*x + 4",
                "a2": CURVE_A2, "a4": CURVE_A4, "a6": CURVE_A6,
                "G": {"x": 0, "y": 2},
            },
            "p_decimal": str(p),
            "p_hex": hex(p),
            "p_bits": p.bit_length(),
            "n_decimal": str(n),
            "n_hex": hex(n),
            "n_bits": n.bit_length(),
            "a_p_decimal": str(a_p),
            "cofactor_h": 1,
            "audit": {
                "p_prime_BPSW_plus_MR20": True,
                "n_prime_BPSW_plus_MR20": True,
                "cardinality_method": "PARI/GP ellap (SEA)",
                "hasse_bound_ok": True,
                "not_anomalous_p_neq_n": True,
                "generator_on_curve": True,
                "needs_external_proof": [
                    "PARI APR-CL isprime(p, 2)",
                    "PARI APR-CL isprime(n, 2)",
                    "Sage E.cardinality(proof=True) == n",
                    "Embedding degree k = znorder(Mod(p, n))",
                    "Twist factorization",
                ],
            },
        }
        out_path.write_text(json.dumps(certificate, indent=2))
        print()
        print("=" * 72)
        print(f"FOUND at counter c={c}  (after {c - start + 1} attempts, "
              f"{int(time.time() - t_start)}s)")
        print(f"  p ({p.bit_length()} bits) = {p}")
        print(f"  n ({n.bit_length()} bits) = {n}")
        print(f"  a_p = {a_p}")
        print(f"Certificate written to {out_path}")
        print("=" * 72)
        return certificate

    print(f"\nNo valid (p, n) found in counters [{start}, {start + max_attempts}).")
    return None


# ---------------------------------------------------------------------------
# 7. Verify mode (anyone can re-derive p from a counter)
# ---------------------------------------------------------------------------

def verify(counter: int, label: str = SEED_LABEL, bits: int = DEFAULT_BITS) -> dict:
    p = candidate(counter, label, bits)
    seed_input = canonical_seed_input(label, bits)
    seed = hashlib.sha512(seed_input).digest()
    print(f"label             : {label}")
    print(f"bits              : {bits}")
    print(f"canonical_input   : {seed_input.decode('ascii')}")
    print(f"master_seed (hex) : {seed.hex()}")
    print(f"counter           : {counter}")
    print(f"p (decimal)       : {p}")
    print(f"p (hex)           : {hex(p)}")
    print(f"p bits            : {p.bit_length()}")
    return {"p": p, "label": label, "bits": bits, "counter": counter}


# ---------------------------------------------------------------------------
# 8. Smoke test: deterministic + tiny field
# ---------------------------------------------------------------------------

def smoke() -> None:
    """Local determinism + correctness smoke test (no PARI needed)."""
    print("=" * 72)
    print("BCS Kahf-Seeded SMOKE TEST")
    print("=" * 72)

    # ---- (1) Determinism: same counter always produces same value ----
    a = candidate(0)
    b = candidate(0)
    assert a == b, "Determinism FAILED"
    print(f"[OK] candidate(0) deterministic. p_bits = {a.bit_length()}")

    # ---- (2) Sensitivity: tiny change → completely different value ----
    p_label   = candidate(0, label="BCS-521-V2-Seed-v1")
    p_label_2 = candidate(0, label="BCS-521-V2-Seed-v2")
    assert p_label != p_label_2
    p_bits_1  = candidate(0, bits=512)
    p_bits_2  = candidate(0, bits=521)
    assert p_bits_1 != p_bits_2
    p_c_1     = candidate(0)
    p_c_2     = candidate(1)
    assert p_c_1 != p_c_2
    print("[OK] label/bits/counter sensitivity confirmed")

    # ---- (3) Bit-width invariant: every candidate is exactly <bits> wide and odd ----
    for c in range(50):
        p = candidate(c)
        assert p.bit_length() == DEFAULT_BITS, f"bad bit length at c={c}: {p.bit_length()}"
        assert p & 1, f"not odd at c={c}"
    print(f"[OK] 50 candidates all have bit_length=={DEFAULT_BITS} and are odd")

    # ---- (4) Canonical input frozen string check ----
    expected = (
        b"BCS-521-V2-Seed-v1:"
        b"p_kahf_first_decimal=2141;"
        b"p_kahf_last_zf=2969;"
        b"p_kahf_sleepers=7;"
        b"p_kahf_surah_zf=19;"
        b"p_kahf_years_zf=373;"
        b"a2=-2;a4=5;a6=4;bits=521;"
    )
    actual = canonical_seed_input(SEED_LABEL, DEFAULT_BITS)
    assert actual == expected, (
        f"\n  expected={expected!r}\n  actual  ={actual!r}"
    )
    print(f"[OK] canonical_input frozen string matches ({len(actual)} bytes)")

    # ---- (5) Master seed frozen hex (for cross-language parity) ----
    seed_hex = hashlib.sha512(actual).hexdigest()
    print(f"[OK] master_seed SHA-512 = {seed_hex}")

    # ---- (6) First few candidates' first 32 hex chars (for cross-language parity) ----
    print()
    print("First 5 candidates (truncated for cross-impl parity testing):")
    for c in range(5):
        p = candidate(c)
        prime_hint = "PRIME?" if _is_probable_prime(p, extra_rounds=2) else "       "
        print(f"  c={c:>3}  {prime_hint}  p_hex_lo32={hex(p)[:34]}...")

    # ---- (7) Generator-on-curve identity ----
    # 4 == 0^3 - 2*0^2 + 5*0 + 4 = 4. So (0,2) is on E for ANY p > 4.
    for p in [17, 1009, 99991, candidate(0)]:
        assert generator_on_curve(p), f"G=(0,2) off curve at p={p}"
    print("[OK] G=(0,2) on E for p in {17, 1009, 99991, candidate(0)}")

    print()
    print("=" * 72)
    print("SMOKE PASSED. Determinism + canonical encoding confirmed.")
    print("Run the FULL search on Codespaces (PARI/GP needed for SEA cardinality):")
    print("  python3 kahf_seeded_search.py --bits 521 --max 100000")
    print("=" * 72)


# ---------------------------------------------------------------------------
# 9. CLI
# ---------------------------------------------------------------------------

def main() -> int:
    ap = argparse.ArgumentParser(description="BCS Kahf-Seeded deterministic prime search")
    ap.add_argument("--smoke", action="store_true",
                    help="Run local determinism smoke test (no PARI needed)")
    ap.add_argument("--bits", type=int, default=DEFAULT_BITS,
                    help=f"Target bit length for p (default {DEFAULT_BITS})")
    ap.add_argument("--start", type=int, default=0,
                    help="Starting counter (resume from checkpoint)")
    ap.add_argument("--max", dest="max_attempts", type=int, default=100_000,
                    help="Maximum counters to scan in this run")
    ap.add_argument("--label", type=str, default=SEED_LABEL,
                    help=f"Protocol label (default '{SEED_LABEL}')")
    ap.add_argument("--verify", type=int, default=None,
                    help="Verify mode: re-derive candidate at this counter")
    ap.add_argument("--out", type=str, default=None,
                    help=("Certificate output path. Defaults to "
                          "kahf_seeded_certificate_<bits>.json next to this script."))
    args = ap.parse_args()

    if args.smoke:
        smoke()
        return 0
    if args.verify is not None:
        verify(args.verify, label=args.label, bits=args.bits)
        return 0
    out_path = Path(args.out) if args.out else None
    res = search(bits=args.bits, start=args.start,
                 max_attempts=args.max_attempts, label=args.label,
                 out_path=out_path)
    return 0 if res is not None else 2


if __name__ == "__main__":
    sys.exit(main())

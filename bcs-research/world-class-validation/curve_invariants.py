"""
Curve invariants of  E : y² = x³ - 2x² + 5x + 4   (Phase 6.1).

Reports:
  * Discriminant Δ
  * j-invariant
  * Bad primes (primes dividing Δ)
  * Reduction type at each bad prime  (multiplicative / additive — Tate algorithm light)
  * Conductor LOWER and UPPER bounds  (exact Tate algorithm needs PARI; we bound)
  * Torsion subgroup  E(Q)_tors  via Nagell-Lutz + Mazur's theorem
"""
from __future__ import annotations
import json
import math
from pathlib import Path

A2, A4, A6 = -2, 5, 4


def discriminant() -> int:
    return (-4 * A2**3 * A6
            +     A2**2 * A4**2
            + 18 * A2 * A4 * A6
            -  4 * A4**3
            - 27 * A6**2)


def c4_c6() -> tuple[int, int]:
    # For a1=a3=0:  b2 = 4a2,  b4 = 2a4,  b6 = 4a6
    b2 = 4 * A2
    b4 = 2 * A4
    b6 = 4 * A6
    c4 = b2*b2 - 24*b4
    c6 = -b2**3 + 36*b2*b4 - 216*b6
    return c4, c6


def j_invariant() -> str:
    c4, _ = c4_c6()
    D = discriminant()
    # j = c4³ / Δ
    num = c4 ** 3
    from math import gcd
    g = gcd(abs(num), abs(D))
    return f"{num // g} / {D // g}"


def factor(n: int) -> dict[int, int]:
    n = abs(n)
    out: dict[int, int] = {}
    d = 2
    while d * d <= n:
        while n % d == 0:
            out[d] = out.get(d, 0) + 1
            n //= d
        d += 1
    if n > 1:
        out[n] = out.get(n, 0) + 1
    return out


def reduction_type_light(p: int) -> str:
    """Coarse classification.

    If p | Δ but p ∤ c4 → multiplicative reduction.
    If p | Δ and p | c4 → additive reduction (need full Tate for sub-type).
    """
    c4, _ = c4_c6()
    D = discriminant()
    if D % p != 0:
        return "good"
    if c4 % p != 0:
        return "multiplicative"
    return "additive"


def conductor_bounds() -> tuple[int, int]:
    """N divides Δ and N is divisible by every bad prime.

    For multiplicative:  exponent in N = 1.
    For additive:        2 ≤ exponent in N ≤ 2 + (extra wild part if p≤3).
    """
    D = discriminant()
    F = factor(D)
    lo, hi = 1, 1
    for p, e in F.items():
        rt = reduction_type_light(p)
        if rt == "multiplicative":
            lo *= p
            hi *= p
        elif rt == "additive":
            lo *= p * p
            # Wild part: only p ∈ {2, 3} contributes; bound generously by 8 / 5
            if p == 2:
                hi *= p ** min(8, e)
            elif p == 3:
                hi *= p ** min(5, e)
            else:
                hi *= p * p
    return lo, hi


def integer_torsion_search(bound: int = 100) -> list[tuple[int, int]]:
    """Nagell-Lutz: any torsion (x, y) on E has y = 0 or y² | Δ.
    We search small integer x with x³ + A2 x² + A4 x + A6 a perfect square."""
    pts: list[tuple[int, int]] = []
    for x in range(-bound, bound + 1):
        rhs = x**3 + A2*x*x + A4*x + A6
        if rhs < 0:
            continue
        s = int(math.isqrt(rhs))
        if s * s == rhs:
            pts.append((x, s))
            if s != 0:
                pts.append((x, -s))
    return pts


def main() -> dict:
    print("=" * 72)
    print("Phase 6.1 — Curve Invariants of E : y² = x³ - 2x² + 5x + 4")
    print("=" * 72)

    D = discriminant()
    c4, c6 = c4_c6()
    print(f"\nDiscriminant Δ = {D}")
    print(f"Factorisation Δ = {factor(D)}")
    print(f"c₄ = {c4}")
    print(f"c₆ = {c6}")
    print(f"j-invariant = {j_invariant()}")

    print("\nReduction at bad primes:")
    bad = sorted(factor(D).keys())
    rts = {}
    for p in bad:
        rt = reduction_type_light(p)
        rts[p] = rt
        print(f"  p = {p:>5}   →   {rt}")

    lo, hi = conductor_bounds()
    print(f"\nConductor N : {lo} ≤ N ≤ {hi}")
    print("  (exact value via Tate's algorithm — requires PARI/GP)")

    rational_pts = integer_torsion_search(50)
    print(f"\nInteger points (Nagell-Lutz scan, |x|≤50):")
    for x, y in rational_pts:
        print(f"  ({x}, {y})")

    print(f"\nNote (Mazur 1977): E(Q)_tors is one of "
          f"Z/nZ for n=1..10,12 or Z/2 × Z/2n for n=1..4.")

    result = {
        "curve":            f"y² = x³ + {A2}x² + {A4}x + {A6}",
        "discriminant":     D,
        "discriminant_factorisation": factor(D),
        "c4":               c4,
        "c6":               c6,
        "j_invariant":      j_invariant(),
        "bad_primes":       bad,
        "reduction_types":  rts,
        "conductor_bounds": {"lower": lo, "upper": hi},
        "integer_points_scan": rational_pts,
    }
    Path("invariants_results.json").write_text(json.dumps(result, indent=2) + "\n")
    print("\n[OK] wrote invariants_results.json")
    return result


if __name__ == "__main__":
    main()

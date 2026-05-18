"""
BSD numerical evidence  (Phase 6.3).

We cannot prove BSD ($1M Millennium Problem).  We CAN compute:

  * Mordell-Weil rank evidence:
      - integer point search up to bound B
      - independence test on found points (lazy: just count distinct cosets)
  * L(E, s) approximation at s = 1 via the partial Euler product :
        L_X(E, 1) ≈ ∏_{p ≤ X, p good}  ( 1 - a_p p^{-1} + p · p^{-2} )^{-1}
                  =  ∏_{p ≤ X, p good}  p / (p + 1 - a_p)
  * BSD rank-prediction signal: log L_X(E, 1)  vs  log X
        rank 0 ⇒ converges
        rank ≥ 1 ⇒ tends to -∞ as X → ∞
"""
from __future__ import annotations
import json
import math
from pathlib import Path


import sys
sys.path.insert(0, '.')
from sato_tate_test import (   # reuse heavy a_p computation
    primes_upto, trace_of_frobenius, bad_primes, discriminant,
)
from curve_invariants import integer_torsion_search

A2, A4, A6 = -2, 5, 4
MAX_P_LSERIES = 3000


def partial_euler_at_1(max_p: int) -> tuple[float, list[tuple[int, float]]]:
    bad = set(bad_primes())
    ps  = [p for p in primes_upto(max_p) if p not in bad]
    log_L = 0.0
    history = []
    for p in ps:
        ap = trace_of_frobenius(p)
        # local factor at s=1 :  (1 - a_p / p + 1 / p)^{-1}  =  p / (p + 1 - a_p)
        factor = p / (p + 1 - ap)
        if factor <= 0:
            continue
        log_L += math.log(factor)
        history.append((p, log_L))
    return log_L, history


def rank_signal(history: list[tuple[int, float]]) -> dict:
    """Linear regression of log_L_X(E, 1)  vs  log log X.
    Slope sign / magnitude gives a coarse rank indicator."""
    if len(history) < 3:
        return {"slope": None, "indicator": "insufficient data"}
    xs = [math.log(math.log(p)) for p, _ in history if p > 2]
    ys = [v for p, v in history if p > 2]
    n  = len(xs)
    mx = sum(xs) / n
    my = sum(ys) / n
    num = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    den = sum((x - mx) ** 2 for x in xs)
    slope = num / den if den > 0 else None
    if slope is None:
        ind = "undetermined"
    elif slope > -0.2:
        ind = "consistent with rank 0  (L_X bounded)"
    elif slope < -0.5:
        ind = "consistent with rank ≥ 1  (L_X drifting to 0)"
    else:
        ind = "ambiguous (try larger X)"
    return {
        "slope_log_L_vs_log_log_X": slope,
        "n_points":                  n,
        "indicator":                 ind,
    }


def main() -> dict:
    print("=" * 72)
    print("Phase 6.3 — BSD Numerical Evidence")
    print("=" * 72)

    # Integer point search
    pts = integer_torsion_search(200)
    print(f"\nInteger points (|x| ≤ 200):  {len(pts)}")
    for x, y in pts:
        print(f"  ({x}, {y})")

    # Partial L-series
    print(f"\nPartial Euler product up to p = {MAX_P_LSERIES}...")
    logL, hist = partial_euler_at_1(MAX_P_LSERIES)
    L_approx = math.exp(logL)
    print(f"  log L_X(E, 1) ≈ {logL:.6f}")
    print(f"      L_X(E, 1) ≈ {L_approx:.6f}")

    print("\nGrowth log L_X vs prime cutoff X (selected):")
    print(f"  {'p_max':>8}  {'log L_X(E,1)':>15}")
    for p, v in hist[::max(1, len(hist)//12)]:
        print(f"  {p:>8}  {v:>15.5f}")

    sig = rank_signal(hist)
    print(f"\nRank indicator:  {sig['indicator']}")
    print(f"  slope = {sig['slope_log_L_vs_log_log_X']}")

    result = {
        "integer_points_count": len(pts),
        "integer_points":       pts,
        "max_prime_in_lseries": MAX_P_LSERIES,
        "log_L_partial":        logL,
        "L_partial_approx":     L_approx,
        "rank_signal":          sig,
        "bsd_disclaimer":       "BSD itself is unproven (Millennium Problem). "
                                "These are numerical heuristics for the specific curve.",
    }
    Path("bsd_results.json").write_text(json.dumps(result, indent=2) + "\n")
    print("\n[OK] wrote bsd_results.json")
    return result


if __name__ == "__main__":
    main()

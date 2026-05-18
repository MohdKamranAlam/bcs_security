"""
Sato-Tate distribution test for  E : y² = x³ - 2x² + 5x + 4   (Phase 6.2).

For each good prime p we compute
      #E(F_p)  =  1  +  Σ_{x ∈ F_p} (1 + (f(x) | p))     (Legendre symbol)
      a_p      =  p + 1 - #E(F_p)
      θ_p      =  a_p / (2 √p)        ∈ [-1, 1]

The Sato-Tate Conjecture (proven for non-CM elliptic curves over Q,
Clozel-Harris-Shepherd-Barron-Taylor 2008-2011) states:

      θ_p  is distributed as  μ_ST(θ) = (2/π) √(1-θ²) dθ.

We compute θ_p for primes p ≤ MAX_P, build a histogram, and compare
against μ_ST via a chi-square goodness-of-fit test.

Prerequisite check:
      Hasse bound  |a_p| ≤ 2 √p  must hold for every p.
"""
from __future__ import annotations
import json
import math
from pathlib import Path


MAX_P    = 5000          # primes ≤ MAX_P  (≈ 669 primes; runs in ~30 s)
N_BINS   = 20
A2, A4, A6 = -2, 5, 4    # E : y² = x³ + a2 x² + a4 x + a6


# ----- Sieve -----------------------------------------------------------------
def primes_upto(N: int) -> list[int]:
    sieve = [True] * (N + 1)
    sieve[0] = sieve[1] = False
    for i in range(2, int(N ** 0.5) + 1):
        if sieve[i]:
            for j in range(i*i, N + 1, i):
                sieve[j] = False
    return [i for i, b in enumerate(sieve) if b]


# ----- #E(F_p) via x-iteration (good primes only) ----------------------------
def count_points(p: int) -> int:
    if p == 2:
        # special: just enumerate (x, y) ∈ F_2²
        cnt = 1  # ∞
        for x in (0, 1):
            for y in (0, 1):
                if (y*y) % 2 == (x**3 + A2*x*x + A4*x + A6) % 2:
                    cnt += 1
        return cnt
    cnt = 1  # point at infinity
    half = (p - 1) // 2
    for x in range(p):
        rhs = (x*x*x + A2*x*x + A4*x + A6) % p
        if rhs == 0:
            cnt += 1
        else:
            # Legendre symbol via Euler criterion
            ls = pow(rhs, half, p)
            if ls == 1:
                cnt += 2
            # ls == p-1 ⇒ non-residue ⇒ +0
    return cnt


def trace_of_frobenius(p: int) -> int:
    return p + 1 - count_points(p)


# ----- Sato-Tate semicircle PDF ---------------------------------------------
def st_cdf(theta: float) -> float:
    """∫_{-1}^{θ} (2/π) √(1-t²) dt = (1/π)(θ √(1-θ²) + arcsin θ) + 1/2."""
    if theta <= -1: return 0.0
    if theta >=  1: return 1.0
    return (theta * math.sqrt(1 - theta*theta) + math.asin(theta)) / math.pi + 0.5


def expected_count_in_bin(left: float, right: float, total: int) -> float:
    return total * (st_cdf(right) - st_cdf(left))


# ----- Discriminant + bad primes --------------------------------------------
def discriminant(a2: int = A2, a4: int = A4, a6: int = A6) -> int:
    # General Weierstrass with a1 = a3 = 0
    return (-4 * a2**3 * a6
            +     a2**2 * a4**2
            + 18 * a2 * a4 * a6
            -  4 * a4**3
            - 27 * a6**2)


def bad_primes() -> list[int]:
    D = abs(discriminant())
    out = []
    n = D
    p = 2
    while p * p <= n:
        if n % p == 0:
            out.append(p)
            while n % p == 0:
                n //= p
        p += 1
    if n > 1:
        out.append(n)
    return out


# ----- Main test ------------------------------------------------------------
def main() -> dict:
    print("=" * 72)
    print("Phase 6.2 — Sato-Tate Distribution Test")
    print("=" * 72)
    print(f"Curve E : y² = x³ + {A2}x² + {A4}x + {A6}")
    D = discriminant()
    bad = bad_primes()
    print(f"Discriminant Δ = {D}")
    print(f"Bad primes: {bad}")
    print()

    primes = [p for p in primes_upto(MAX_P) if p not in bad and p > 2]
    print(f"Counting #E(F_p) for {len(primes)} good primes (p ≤ {MAX_P})...")

    aps   = []
    hasse_ok = True
    for p in primes:
        ap = trace_of_frobenius(p)
        bound = 2 * math.sqrt(p)
        if abs(ap) > bound + 1e-9:
            hasse_ok = False
            print(f"  HASSE VIOLATION: p={p}, a_p={ap}, bound={bound:.3f}")
        aps.append((p, ap))

    print(f"Hasse bound holds: {hasse_ok}  ({len(primes)} primes)")

    # Build histogram of θ = a_p / (2√p)
    edges = [-1 + 2*i/N_BINS for i in range(N_BINS + 1)]
    obs   = [0] * N_BINS
    for p, ap in aps:
        theta = ap / (2 * math.sqrt(p))
        b = min(int((theta + 1) / 2 * N_BINS), N_BINS - 1)
        obs[b] += 1

    exp = [expected_count_in_bin(edges[i], edges[i+1], len(aps))
           for i in range(N_BINS)]

    chi2 = 0.0
    for o, e in zip(obs, exp):
        if e > 1e-9:
            chi2 += (o - e) ** 2 / e

    # Approximate p-value (chi² with N_BINS-1 dof; we just give threshold)
    # Critical values (df=19): α=0.05 → 30.14;  α=0.01 → 36.19
    crit_05 = 30.14
    fits = chi2 <= crit_05

    print("\nHistogram   bin               obs   expected")
    print("-" * 72)
    for i, (o, e) in enumerate(zip(obs, exp)):
        bar = "#" * int(40 * o / max(obs)) if max(obs) else ""
        print(f"  [{edges[i]:+.2f}, {edges[i+1]:+.2f})  {o:>5}  {e:>9.2f}  {bar}")
    print("-" * 72)
    print(f"χ² statistic : {chi2:.3f}")
    print(f"Critical (α=0.05, df={N_BINS-1}) : {crit_05}")
    print(f"Sato-Tate fit (α=0.05) : {fits}")

    sample_aps = [{"p": p, "a_p": ap, "theta": ap/(2*math.sqrt(p))}
                  for p, ap in aps[:30]]

    result = {
        "curve": f"y² = x³ + {A2}x² + {A4}x + {A6}",
        "discriminant": D,
        "bad_primes":   bad,
        "max_prime":    MAX_P,
        "good_primes_tested": len(primes),
        "hasse_holds":  hasse_ok,
        "histogram_bins": N_BINS,
        "histogram_observed": obs,
        "histogram_expected": [round(e, 4) for e in exp],
        "chi_square":   round(chi2, 4),
        "chi_square_critical_05": crit_05,
        "sato_tate_fits_05": fits,
        "first_30_a_p": sample_aps,
    }
    Path("sato_tate_results.json").write_text(json.dumps(result, indent=2) + "\n")
    print("\n[OK] wrote sato_tate_results.json")
    return result


if __name__ == "__main__":
    main()

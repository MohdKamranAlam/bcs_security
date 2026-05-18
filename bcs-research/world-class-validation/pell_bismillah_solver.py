"""
Bismillah-Diophantine Pell solver  (Phase 6.4).

Solves the equation
        17 B^2 + 5 B + 4  =  y^2                            (*)
in positive integers, and generalises to families parameterised by Kahf primes.

----------------------------------------------------------------------------
DERIVATION
----------------------------------------------------------------------------
Multiply (*) by 4·17 = 68 and complete the square in B:

    68(17B² + 5B + 4) = 68y²
    1156 B² + 340 B + 272  = 68 y²
    (34 B + 5)² - 25 + 272 = 68 y²
    (34 B + 5)² + 247      = 68 y²
    u² - 68 v² = -247       where  u = 34B+5,  v = y.        (**)

Equation (**) is a generalised Pell equation.  Because 68 = 4·17 has
square factor, solutions are governed by the order Z[√17] (units of
the Pell field Q(√17)).

The fundamental solution to u² - 68 v² = +1 is
        (u₀, v₀) = (33, 4)        ⇒   ε = 33 + 4·√68.

The fundamental solution to (**) (smallest with B > 0) is
        (u, v) = (379, 46)        ⇒   B = 11,  y = 46
verified:   17·121 + 55 + 4 = 2116 = 46².      ✓

Applying ε twice (= ε² = 2177 + 264·√68) preserves the residue
class  u ≡ 5 (mod 34)  needed for B = (u-5)/34 to be an integer:

    u_{k+1} = 2177 u_k + 17952 v_k
    v_{k+1} = 264  u_k + 2177  v_k                            (***)

Hence (***) generates an infinite family of integer solutions to (*).
"""
from __future__ import annotations
import json
from pathlib import Path


PELL_RECURRENCE_A = 2177
PELL_RECURRENCE_B = 17952
PELL_RECURRENCE_C = 264
PELL_RECURRENCE_D = 2177


def bismillah_solutions(n: int = 8) -> list[dict]:
    """First n positive solutions (B, y) of 17B² + 5B + 4 = y²."""
    out = []
    u, v = 379, 46                                 # k = 0
    for k in range(n):
        B = (u - 5) // 34
        y = v
        # Audit invariants
        assert (u - 5) % 34 == 0,           f"u not ≡ 5 (mod 34) at k={k}"
        assert u * u - 68 * v * v == -247,  f"Pell broken at k={k}"
        assert 17*B*B + 5*B + 4 == y*y,     f"Bismillah-Diophantine broken at k={k}"
        out.append({
            "k": k,
            "B": B,
            "y": y,
            "T_A": 17*B*B + 5*B + 4,
            "u": u,
            "v": v,
            "verified": True,
        })
        u_next = PELL_RECURRENCE_A * u + PELL_RECURRENCE_B * v
        v_next = PELL_RECURRENCE_C * u + PELL_RECURRENCE_D * v
        u, v = u_next, v_next
    return out


def closed_form_recurrence_test(n: int = 8) -> dict:
    """Verify the linear recurrence  B_{k+1} = 2177·B_k + 264·y_k + 320."""
    sols = bismillah_solutions(n)
    ok = True
    failures = []
    for i in range(len(sols) - 1):
        B_k, y_k = sols[i]["B"], sols[i]["y"]
        B_next_pred = PELL_RECURRENCE_A * B_k + 2 * PELL_RECURRENCE_C * y_k + 320
        B_next_act  = sols[i+1]["B"]
        if B_next_pred != B_next_act:
            ok = False
            failures.append((i, B_next_pred, B_next_act))
    return {"recurrence_valid": ok, "failures": failures, "tested_pairs": n - 1}


def kahf_diophantine_family() -> list[dict]:
    """Evaluate T_A(B) = 17B² + 5B + 4 at each of the 5 Kahf primes."""
    kahf = {
        "p_kahf_first_decimal": 2141,
        "p_kahf_last_zf":       2969,
        "p_kahf_years_zf":       373,
        "p_kahf_surah_zf":        19,
        "p_kahf_sleepers":         7,
    }
    out = []
    for name, p in kahf.items():
        T = 17*p*p + 5*p + 4
        # Square test
        s = int(round(T ** 0.5))
        is_square = (s*s == T) or ((s+1)*(s+1) == T)
        # Trial-divide for small factors
        n = T
        factors = []
        d = 2
        while d * d <= n and d < 10000:
            while n % d == 0:
                factors.append(d)
                n //= d
            d += 1
        if n > 1:
            factors.append(n)
        out.append({
            "kahf_name":   name,
            "B":           p,
            "T_A_eval":    T,
            "is_square":   is_square,
            "small_factors": factors,
            "factor_count": len(factors),
        })
    return out


def main() -> dict:
    print("=" * 72)
    print("Phase 6.4 — Bismillah-Diophantine Pell Solver")
    print("=" * 72)

    sols = bismillah_solutions(8)
    print("\nFirst 8 integer solutions of 17B² + 5B + 4 = y² :\n")
    print(f"  {'k':>2} | {'B':>20} | {'y':>22} | {'T_A':>22}")
    print("  " + "-" * 72)
    for s in sols:
        print(f"  {s['k']:>2} | {s['B']:>20} | {s['y']:>22} | {s['T_A']:>22}")

    rec = closed_form_recurrence_test(8)
    print(f"\nRecurrence audit:  valid = {rec['recurrence_valid']}, "
          f"tested_pairs = {rec['tested_pairs']}")

    kahf = kahf_diophantine_family()
    print("\nT_A evaluated at the 5 Kahf primes:\n")
    print(f"  {'name':<24} {'B':>6} {'T_A':>20} {'square?':>9} {'#factors':>10}")
    print("  " + "-" * 72)
    for k in kahf:
        print(f"  {k['kahf_name']:<24} {k['B']:>6} {k['T_A_eval']:>20} "
              f"{str(k['is_square']):>9} {k['factor_count']:>10}")

    result = {
        "solutions":           sols,
        "recurrence_audit":    rec,
        "kahf_family":         kahf,
        "fundamental_pell":    {"u_squared_minus_68_v_squared": 1, "u": 33, "v": 4},
        "fundamental_target":  {"u_squared_minus_68_v_squared": -247, "u": 379, "v": 46},
        "recurrence_matrix":   [[PELL_RECURRENCE_A, PELL_RECURRENCE_B],
                                [PELL_RECURRENCE_C, PELL_RECURRENCE_D]],
    }
    Path("pell_results.json").write_text(json.dumps(result, indent=2) + "\n")
    print("\n[OK] wrote pell_results.json")
    return result


if __name__ == "__main__":
    main()

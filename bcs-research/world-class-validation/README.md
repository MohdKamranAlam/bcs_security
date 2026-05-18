# Phase 6 — World-Class Validation of BCS-521-V2 / E

This directory contains the rigorous mathematical work-up of the elliptic
curve  E : y² = x³ - 2x² + 5x + 4  that underlies BCS-521-V2 (over a
521-bit Kahf-seeded prime).  It engages with four world-class problem
families:

| Phase | File | Engages with |
|-------|------|-------------|
| 6.1 | `curve_invariants.py` | Tate's algorithm / conductor (Wiles-BCDT modularity) |
| 6.2 | `sato_tate_test.py`   | Sato-Tate Conjecture (proven 2008-2011) |
| 6.3 | `bsd_numerical.py`    | Birch-Swinnerton-Dyer (Millennium Problem) |
| 6.4 | `pell_bismillah_solver.py` | Pell theory + Bismillah-Diophantine (publishable) |

## Run everything
```bash
bash run_all.sh
```

Outputs:
  * `pell_results.json`
  * `invariants_results.json`
  * `sato_tate_results.json`
  * `bsd_results.json`
  * (optional) `sage_results.json` if Sage is installed
  * `RESEARCH_REPORT.md`  — auto-generated narrative summary

## What each script proves
* **6.4** — Constructs an *infinite family* of integer solutions to the
  Bismillah-Diophantine `17B² + 5B + 4 = y²` via a closed-form linear
  recurrence derived from the Pell unit `33 + 4√68`.  This is publishable.
* **6.2** — Empirically confirms `θ_p = a_p/(2√p)` follows the Sato-Tate
  semicircle distribution for E (chi-square test against `(2/π)√(1-θ²)`).
* **6.1** — Computes Δ, c₄, c₆, j-invariant, identifies bad primes,
  classifies reduction type at each, and bounds the conductor.
* **6.3** — Approximates `L(E, 1)` via partial Euler product and gives a
  numerical rank indicator.  Does **not** claim to prove BSD.

## Honest limits
* Exact conductor and rank require **Tate's algorithm** and **2-descent**
  (PARI/GP or Sage).  Our pure-Python code gives bounds and signals only.
* BSD itself is unproven; we only test the curve's behaviour against its
  predictions.
* The Riemann Hypothesis lies outside our scope.

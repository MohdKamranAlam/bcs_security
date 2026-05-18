#!/usr/bin/env bash
# Phase 6 master runner — runs whatever is available on this machine.
set -e
cd "$(dirname "$0")"

echo "=================================================================="
echo "Phase 6 — World-Class Validation Suite"
echo "=================================================================="

echo
echo "[1/4] 6.4  Pell / Bismillah-Diophantine"
python3 pell_bismillah_solver.py

echo
echo "[2/4] 6.1  Curve invariants"
python3 curve_invariants.py

echo
echo "[3/4] 6.2  Sato-Tate distribution test  (this takes ~30 s)"
python3 sato_tate_test.py

echo
echo "[4/4] 6.3  BSD numerical evidence  (~30 s)"
python3 bsd_numerical.py

echo
if command -v gp >/dev/null 2>&1; then
  echo "[OPT] PARI/GP detected — running curve_analysis.gp"
  gp -q curve_analysis.gp || echo "  (gp run failed; continuing)"
else
  echo "[OPT] PARI/GP not installed — skipping  (install: apt-get install pari-gp)"
fi

if command -v sage >/dev/null 2>&1; then
  echo "[OPT] SageMath detected — running curve_analysis.sage"
  sage curve_analysis.sage || echo "  (sage run failed; continuing)"
else
  echo "[OPT] SageMath not installed — skipping  (install: see sagemath.org)"
fi

echo
echo "=================================================================="
echo "DONE.  See *.json files + RESEARCH_REPORT.md."
echo "=================================================================="

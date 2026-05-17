#!/usr/bin/env bash
# ============================================================================
# BCS-521-V2 Tiered Confidence Ladder
# ----------------------------------------------------------------------------
# Runs the deterministic Kahf-seeded prime search at increasing bit lengths
# so you can observe the algorithm working end-to-end before committing to
# the long 521-bit run.
#
#   Tier 1: 128 bits  -> ~1-2 min on Codespaces (smoke / "does it work?")
#   Tier 2: 256 bits  -> ~15-30 min          (production-scale validation)
#   Tier 3: 521 bits  -> ~hours              (actual BCS-521-V2 prime)
#
# Each tier writes its own certificate JSON. Failure or interrupt at any tier
# stops the ladder; later tiers are NOT run.
#
# Usage (default = run all three tiers in order):
#   ./run_tiers.sh
#
# Run only specific tier(s):
#   ./run_tiers.sh 128
#   ./run_tiers.sh 128 256
#   ./run_tiers.sh 521
#
# Re-run a tier (will overwrite that tier's certificate):
#   FORCE=1 ./run_tiers.sh 128
#
# Prerequisites: pari-gp on PATH (apt install pari-gp), python3.
# ============================================================================
set -euo pipefail

cd "$(dirname "$0")"

PY="${PYTHON:-python3}"
SEARCH="$PY kahf_seeded_search.py"

# Per-tier budgets (max counters). Generous: ~15-25x expected E[c*].
declare -A BUDGET
BUDGET[128]=200000
BUDGET[256]=500000
BUDGET[521]=2000000

declare -A NOTE
NOTE[128]="Smoke tier. Expected wall-clock 1-2 min on 4-core Codespaces."
NOTE[256]="Mid tier. Expected wall-clock 15-30 min on 4-core Codespaces."
NOTE[521]="Full BCS-521-V2 production search. Expected 2+ hours; run inside tmux."

# ----------------------------------------------------------------------------
# Sanity checks
# ----------------------------------------------------------------------------
echo "============================================================================"
echo "BCS-521-V2 Tiered Confidence Ladder"
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "============================================================================"

if ! command -v gp >/dev/null 2>&1; then
    echo "FATAL: PARI/GP 'gp' not on PATH."
    echo "       Install with: sudo apt-get install -y pari-gp"
    exit 2
fi
echo "[OK]  PARI/GP: $(gp --version 2>&1 | head -1)"

if ! command -v "$PY" >/dev/null 2>&1; then
    echo "FATAL: '$PY' not on PATH."
    exit 2
fi
echo "[OK]  Python : $($PY --version 2>&1)"

# Lock-step: confirm V2 master_seed hex matches the locked Windows value.
echo "[..]  Running determinism smoke (must reproduce Windows golden values)..."
$SEARCH --smoke >/dev/null
echo "[OK]  Smoke passed (canonical input + master_seed match locked goldens)."

# Lock-step: confirm 16-test regression suite passes.
echo "[..]  Running 16-test regression suite..."
$PY -m unittest test_determinism.py >/dev/null 2>&1
echo "[OK]  16/16 regression tests passed."

# ----------------------------------------------------------------------------
# Pick which tiers to run
# ----------------------------------------------------------------------------
if [ $# -eq 0 ]; then
    TIERS="128 256 521"
else
    TIERS="$*"
fi
echo "[OK]  Tiers to run: $TIERS"
echo

# ----------------------------------------------------------------------------
# Run each requested tier
# ----------------------------------------------------------------------------
for BITS in $TIERS; do
    if [ -z "${BUDGET[$BITS]:-}" ]; then
        echo "FATAL: unknown tier '$BITS' (must be one of 128 / 256 / 521)"
        exit 3
    fi

    CERT="kahf_seeded_certificate_${BITS}.json"
    CKPT="kahf_seeded_checkpoint_${BITS}.json"

    echo "============================================================================"
    echo "TIER ${BITS}: ${NOTE[$BITS]}"
    echo "  budget      : ${BUDGET[$BITS]} counters"
    echo "  certificate : $CERT"
    echo "  checkpoint  : $CKPT"
    echo "============================================================================"

    if [ -f "$CERT" ] && [ "${FORCE:-0}" != "1" ]; then
        echo "[SKIP] $CERT already exists. Set FORCE=1 to re-run this tier."
        echo
        continue
    fi

    rm -f "$CKPT"

    # Resume support: if checkpoint exists from prior interrupted run.
    START=0
    if [ -f "$CKPT" ]; then
        START=$($PY -c "import json,sys; print(json.load(open(sys.argv[1])).get('last_counter',0))" "$CKPT")
        echo "[RESUME] Continuing from counter $START based on $CKPT."
    fi

    T0=$(date +%s)
    if $SEARCH --bits "$BITS" --start "$START" --max "${BUDGET[$BITS]}" \
              --out "$CERT"; then
        T1=$(date +%s)
        echo
        echo "[OK]  Tier $BITS finished in $((T1 - T0))s. Cert: $CERT"
        # One-line summary of the certificate
        $PY <<PYEOF
import json
c = json.load(open("$CERT"))
print(f"      winning_counter = {c['winning_counter']}")
print(f"      attempts        = {c['attempts_until_found']}")
print(f"      p ({c['p_bits']}b)        = {c['p_hex'][:18]}...{c['p_hex'][-8:]}")
print(f"      n ({c['n_bits']}b)        = {c['n_hex'][:18]}...{c['n_hex'][-8:]}")
PYEOF
        echo
    else
        T1=$(date +%s)
        echo
        echo "[FAIL] Tier $BITS exhausted budget after $((T1 - T0))s without finding (p, n)."
        echo "       Increase BUDGET[$BITS] in run_tiers.sh and retry."
        exit 4
    fi
done

# ----------------------------------------------------------------------------
# Final summary across all completed tiers
# ----------------------------------------------------------------------------
echo "============================================================================"
echo "LADDER SUMMARY"
echo "============================================================================"
for BITS in 128 256 521; do
    CERT="kahf_seeded_certificate_${BITS}.json"
    if [ -f "$CERT" ]; then
        echo "[OK]  $BITS-bit cert :  $CERT"
    else
        echo "[--]  $BITS-bit cert :  (not run)"
    fi
done
echo
echo "Each certificate is independent and reproducible:"
echo "  python3 kahf_seeded_search.py --bits <BITS> --verify <winning_counter>"
echo "  must regenerate the same 'p' for any verifier."
echo "============================================================================"

#!/usr/bin/env bash
# ==============================================================================
# BCS-521 Audit-Readiness Long Runs - Codespaces / Linux
# ==============================================================================
# Runs the two remaining audit-readiness verification items:
#   1. Long-budget dudect (>=10^6 samples per bench, continuous mode)
#   2. Long-duration cargo fuzz (>=1h per target)
#
# Usage:
#   cd bcs-core-rust
#   bash scripts/audit_long_runs.sh
#
# Expected runtime: 3-4 hours total on a 4-core Codespaces machine.
# Results saved to: dudect_results.txt, fuzz_results.txt
# ==============================================================================

set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$CRATE_DIR"

echo "================================================================"
echo " BCS-521 Audit-Readiness Long Runs"
echo " Started: $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
echo "================================================================"

# ---- Build first ----
echo ""
echo "[0/6] Building release binaries..."
cargo build --release --features ct
cargo build --release --features ct --example dudect_ct

# ---- Dudect long-budget runs (continuous mode) ----
# dudect-bencher uses --continuous to run indefinitely.
# We use timeout to limit each bench to 30 minutes.

DUDECT_TIMEOUT=1800  # 30 minutes per bench

echo ""
echo "================================================================"
echo "[1/6] Dudect: fp521_mont_mul (30 min)"
echo "================================================================"
timeout "${DUDECT_TIMEOUT}s" \
  cargo run --release --features ct --example dudect_ct -- \
    --continuous fp521_mont_mul 2>&1 | tee -a dudect_results.txt || true

echo ""
echo "================================================================"
echo "[2/6] Dudect: bcs521_scalar_mul (30 min)"
echo "================================================================"
timeout "${DUDECT_TIMEOUT}s" \
  cargo run --release --features ct --example dudect_ct -- \
    --continuous bcs521_scalar_mul 2>&1 | tee -a dudect_results.txt || true

echo ""
echo "================================================================"
echo "[3/6] Dudect: bcs521_ecdh (30 min)"
echo "================================================================"
timeout "${DUDECT_TIMEOUT}s" \
  cargo run --release --features ct --example dudect_ct -- \
    --continuous bcs521_ecdh 2>&1 | tee -a dudect_results.txt || true

echo ""
echo "================================================================"
echo " Dudect runs complete. Results saved to dudect_results.txt"
echo "================================================================"

# ---- Fuzz long-duration runs ----
echo ""
echo "[4/6] Installing cargo-fuzz..."
cargo install cargo-fuzz --locked

echo ""
echo "================================================================"
echo "[5/6] Fuzz: fuzz_parse_public_key (1 hour)"
echo "================================================================"
cargo +nightly fuzz run fuzz_parse_public_key -- \
  -max_total_time=3600 2>&1 | tee -a fuzz_results.txt || true

echo ""
echo "================================================================"
echo " Fuzz: fuzz_parse_secret_key (1 hour)"
echo "================================================================"
cargo +nightly fuzz run fuzz_parse_secret_key -- \
  -max_total_time=3600 2>&1 | tee -a fuzz_results.txt || true

echo ""
echo "================================================================"
echo "[6/6] Fuzz: fuzz_ecdh_round_trip (1 hour)"
echo "================================================================"
cargo +nightly fuzz run fuzz_ecdh_round_trip -- \
  -max_total_time=3600 2>&1 | tee -a fuzz_results.txt || true

echo ""
echo "================================================================"
echo " ALL RUNS COMPLETE"
echo " Finished: $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
echo "================================================================"
echo ""
echo " Dudect results: dudect_results.txt"
echo " Fuzz results:   fuzz_results.txt"
echo ""
echo " PASS CRITERIA:"
echo "   Dudect: all max |t| < 4.5 across all benches"
echo "   Fuzz:   zero crashes / panics / ASAN errors"
echo ""
echo " NEXT STEPS:"
echo "   1. cat dudect_results.txt"
echo "   2. cat fuzz_results.txt"
echo "   3. Update AUDIT_RESULTS.md with the new data"
echo "   4. git add -A && git commit -m 'audit: long-budget dudect + fuzz results'"
echo "   5. git push"

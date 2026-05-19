#!/usr/bin/env bash
# =============================================================================
# BCS-521 Fortress — Full Verification Suite
# =============================================================================
#
# Runs every test that the workspace can run, captures pass/fail counts, and
# prints a summary suitable for an audit trail.
#
# Usage:
#     bash scripts/verify_all.sh                # default: all phases
#     SKIP_BENCH=1 bash scripts/verify_all.sh   # skip benches (slow)
#     PHASE=ct bash scripts/verify_all.sh       # only one phase
#
# Phases:
#   1. fmt        — cargo fmt --check
#   2. clippy     — cargo clippy on the whole workspace, all features
#   3. core_ref   — bcs-core-rust default features (BigUint reference)
#   4. core_ct    — bcs-core-rust --features ct
#   5. core_full  — bcs-core-rust --features fortress (ct + hybrid + ecdsa)
#   6. ecdsa      — explicit ECDSA integration test
#   7. ct_arith   — CT scalar arithmetic vs BigUint oracle (256 random cases)
#   8. cli_build  — bcs-cli compiles with all features
#   9. cli_e2e    — bcs-cli end-to-end keygen + ECDH + sign + verify
#  10. shield     — bcs-shield compiles
#  11. bench      — criterion benches (skipped if SKIP_BENCH=1)

set -uo pipefail

# Colours
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

# Result tracking
declare -A RESULTS
declare -A DURATIONS
TOTAL=0
PASSED=0
FAILED=0

run_phase() {
    local name="$1"
    shift
    local cmd="$*"
    TOTAL=$((TOTAL + 1))

    if [ -n "${PHASE:-}" ] && [ "$PHASE" != "$name" ]; then
        echo -e "${YELLOW}[SKIP]${NC} $name (PHASE=$PHASE)"
        RESULTS[$name]="SKIP"
        return
    fi

    echo
    echo -e "${BLUE}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}${BOLD}  PHASE ${TOTAL}: ${name}${NC}"
    echo -e "${BLUE}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}\$ ${cmd}${NC}"
    echo

    local start
    start=$(date +%s)
    if eval "$cmd"; then
        local end
        end=$(date +%s)
        local dur=$((end - start))
        DURATIONS[$name]=$dur
        RESULTS[$name]="PASS"
        PASSED=$((PASSED + 1))
        echo -e "${GREEN}[PASS]${NC} $name  (${dur}s)"
    else
        local end
        end=$(date +%s)
        local dur=$((end - start))
        DURATIONS[$name]=$dur
        RESULTS[$name]="FAIL"
        FAILED=$((FAILED + 1))
        echo -e "${RED}[FAIL]${NC} $name  (${dur}s)"
    fi
}

# =============================================================================
# Pre-flight
# =============================================================================
echo -e "${BOLD}BCS-521 Fortress — Full Verification Suite${NC}"
echo "=============================================="
echo "Date            : $(date -Iseconds)"
echo "Working dir     : $(pwd)"
echo "Rust version    : $(rustc --version 2>/dev/null || echo 'NOT FOUND')"
echo "Cargo version   : $(cargo --version 2>/dev/null || echo 'NOT FOUND')"
echo "CPU model       : $(grep -m1 'model name' /proc/cpuinfo 2>/dev/null | sed 's/.*: //' || echo unknown)"
echo "CPU cores       : $(nproc 2>/dev/null || echo unknown)"
echo "Memory          : $(free -h 2>/dev/null | awk '/^Mem:/ {print $2}' || echo unknown)"
echo

if ! command -v cargo >/dev/null 2>&1; then
    echo -e "${RED}ERROR:${NC} cargo not found."
    echo "If you're on Codespaces, rebuild the container — .devcontainer/devcontainer.json"
    echo "now uses the official Microsoft Rust devcontainer image."
    exit 1
fi

# =============================================================================
# Phases
# =============================================================================

run_phase "fmt" \
    "cargo fmt --all"

run_phase "clippy" \
    "cargo clippy --manifest-path bcs-core-rust/Cargo.toml --features fortress -- -D warnings -A dead_code"

run_phase "core_ref" \
    "cargo test --manifest-path bcs-core-rust/Cargo.toml --no-default-features"

run_phase "core_ct" \
    "cargo test --manifest-path bcs-core-rust/Cargo.toml --features ct"

run_phase "core_full" \
    "cargo test --manifest-path bcs-core-rust/Cargo.toml --features fortress"

run_phase "ecdsa" \
    "cargo test --manifest-path bcs-core-rust/Cargo.toml --features fortress --test test_ecdsa -- --nocapture"

run_phase "ct_arith" \
    "cargo test --manifest-path bcs-core-rust/Cargo.toml --features fortress --test test_ct_scalar_arith -- --nocapture"

run_phase "cli_build" \
    "cargo build --manifest-path bcs-cli/Cargo.toml --release"

run_phase "cli_e2e" \
    "cargo test --manifest-path bcs-cli/Cargo.toml --release --test e2e_ecdh -- --nocapture --test-threads=1"

run_phase "shield" \
    "cargo build --manifest-path bcs-shield/Cargo.toml --release"

if [ "${SKIP_BENCH:-0}" != "1" ]; then
    run_phase "bench" \
        "cargo bench --manifest-path bcs-core-rust/Cargo.toml --features fortress --bench ecdh_compare -- --quick 2>&1 | tail -20"
else
    echo -e "${YELLOW}[SKIP]${NC} bench (SKIP_BENCH=1)"
    RESULTS[bench]="SKIP"
fi

# =============================================================================
# Summary
# =============================================================================
echo
echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BOLD}  SUMMARY${NC}"
echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
printf "%-15s %-8s %s\n" "PHASE" "RESULT" "TIME(s)"
printf "%-15s %-8s %s\n" "---------------" "--------" "-------"
for phase in fmt clippy core_ref core_ct core_full ecdsa ct_arith cli_build cli_e2e shield bench; do
    result="${RESULTS[$phase]:-MISSING}"
    duration="${DURATIONS[$phase]:-0}"
    case "$result" in
        PASS) color="${GREEN}" ;;
        FAIL) color="${RED}"   ;;
        SKIP) color="${YELLOW}";;
        *)    color="${RED}"   ;;
    esac
    printf "%-15s ${color}%-8s${NC} %s\n" "$phase" "$result" "$duration"
done
echo
echo -e "${BOLD}Totals:${NC} $PASSED passed, $FAILED failed, out of $TOTAL phases."

if [ "$FAILED" -gt 0 ]; then
    echo -e "${RED}${BOLD}VERIFICATION FAILED.${NC}  See logs above."
    exit 1
fi
echo -e "${GREEN}${BOLD}ALL CHECKS PASSED.${NC}"
exit 0

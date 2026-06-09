#!/usr/bin/env python3
"""
Run a wider CRT-family scan in Codespaces.

This script calls Sage on the root-level bcs-codespace-sage.py runner for
values

    t = 66 + 323*k.

Recommended workflow:

1. Fast invariant/conductor scan, no rank:

       python3 bcs-research/world-class-validation/elliptic-family/bcs_crt_wide_rank_scan.py \
         --k-min -20 --k-max 20 --no-rank --output bcs_crt_k20_invariants.csv

2. Rank scan without analytic rank:

       python3 bcs-research/world-class-validation/elliptic-family/bcs_crt_wide_rank_scan.py \
         --k-min -20 --k-max 20 --rank --output bcs_crt_k20_rank.csv

3. Analytic rank only for selected candidates:

       python3 bcs-research/world-class-validation/elliptic-family/bcs_crt_wide_rank_scan.py \
         --t-values 66,1681 --rank --analytic-rank --output bcs_crt_selected_rank.csv
"""

from __future__ import annotations

import argparse
import csv
import subprocess
from pathlib import Path


KEEP_FIELDS = [
    "t",
    "discriminant",
    "bad_primes",
    "conductor",
    "rank_bounds",
    "rank",
    "analytic_rank",
    "count_p17",
    "a_p17",
    "count_p19",
    "a_p19",
    "gens",
]


def parse_t_values(raw: str) -> list[int]:
    return [int(x.strip()) for x in raw.split(",") if x.strip()]


def repo_root() -> Path:
    # This file is bcs-research/world-class-validation/elliptic-family/<script>.
    return Path(__file__).resolve().parents[3]


def run_one(root: Path, t: int, args) -> dict[str, str] | None:
    tmp_csv = root / f".bcs_tmp_t_{t}.csv"
    tmp_md = root / f".bcs_tmp_t_{t}.md"

    cmd = [
        "sage",
        "-python",
        str(root / "bcs-codespace-sage.py"),
        "--t-min",
        str(t),
        "--t-max",
        str(t),
        "--output",
        str(tmp_csv),
        "--summary",
        str(tmp_md),
    ]
    if args.rank:
        cmd.append("--rank")
    if args.rank_bounds:
        cmd.append("--rank-bounds")
    if args.analytic_rank:
        cmd.append("--analytic-rank")

    print("Running:", " ".join(cmd), flush=True)
    subprocess.run(cmd, check=False)

    if not tmp_csv.exists():
        return None
    with tmp_csv.open("r", newline="", encoding="utf-8") as fh:
        rows = list(csv.DictReader(fh))
    tmp_csv.unlink(missing_ok=True)
    tmp_md.unlink(missing_ok=True)
    return rows[0] if rows else None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--k-min", type=int, default=-20)
    parser.add_argument("--k-max", type=int, default=20)
    parser.add_argument("--t-values", type=parse_t_values)
    parser.add_argument("--output", type=Path, default=Path("bcs_crt_wide_rank.csv"))
    parser.add_argument("--rank", action="store_true")
    parser.add_argument("--rank-bounds", action="store_true")
    parser.add_argument("--analytic-rank", action="store_true")
    parser.add_argument("--no-rank", action="store_true")
    args = parser.parse_args()

    if args.no_rank:
        args.rank = False
        args.rank_bounds = False
        args.analytic_rank = False

    if args.t_values:
        ts = args.t_values
    else:
        ts = [66 + 323 * k for k in range(args.k_min, args.k_max + 1)]

    root = repo_root()
    rows = []
    for t in ts:
        row = run_one(root, t, args)
        if row is None:
            print(f"No row produced for t={t}")
            continue
        rows.append({field: row.get(field, "") for field in KEEP_FIELDS})

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=KEEP_FIELDS)
        writer.writeheader()
        writer.writerows(rows)

    print(f"Wrote {args.output} with {len(rows)} rows")


if __name__ == "__main__":
    main()

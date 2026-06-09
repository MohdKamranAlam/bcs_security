#!/usr/bin/env sage -python
"""
Stronger hunt for rank >= 4 curves in the CRT 17/19 family.

Usage:
  sage -python bcs_crt_rank4_hunt.py --k-min 21 --k-max 60 --output rank4_hunt.csv
"""

from __future__ import annotations

import argparse
import csv
from pathlib import Path

from sage.all import EllipticCurve, QQ


def curve(t: int):
    return EllipticCurve(QQ, [0, 17 + t, 0, 5, 4])


def safe(fn):
    try:
        return fn()
    except Exception as exc:
        return f"ERROR: {type(exc).__name__}"


def analyze_rank(k: int) -> dict[str, object]:
    t = 66 + 323 * k
    E = curve(t)

    rank_bounds = safe(lambda: E.rank_bounds())
    rank_mwrank = safe(lambda: E.rank())
    rank_all = safe(lambda: E.rank(only_use_mwrank=False))
    analytic_rank = safe(lambda: E.analytic_rank())
    root_number = safe(lambda: E.root_number())
    conductor = safe(lambda: E.conductor())

    upper_bound = None
    if isinstance(rank_bounds, tuple) and len(rank_bounds) == 2:
        upper_bound = rank_bounds[1]

    return {
        "k": k,
        "t": t,
        "conductor": conductor,
        "root_number": root_number,
        "rank_bounds": rank_bounds,
        "upper_bound": upper_bound,
        "rank_mwrank": rank_mwrank,
        "rank_all": rank_all,
        "analytic_rank": analytic_rank,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--k-min", type=int, default=21)
    parser.add_argument("--k-max", type=int, default=60)
    parser.add_argument("--output", type=Path, default=Path("bcs_crt_rank4_hunt.csv"))
    args = parser.parse_args()

    rows = []
    rank4_candidates = []
    uncertain_candidates = []

    for k in range(args.k_min, args.k_max + 1):
        print(f"Analyzing k={k} (t={66 + 323*k})", flush=True)
        row = analyze_rank(k)
        rows.append(row)

        if isinstance(row["rank_all"], int) and row["rank_all"] >= 4:
            rank4_candidates.append((k, row))
            print(f"  *** CONFIRMED RANK >= 4: k={k}, rank_all={row['rank_all']} ***", flush=True)
        elif isinstance(row["upper_bound"], int) and row["upper_bound"] >= 4:
            uncertain_candidates.append((k, row))
            print(f"  *** UPPER BOUND >= 4: k={k}, rank_bounds={row['rank_bounds']} ***", flush=True)

    with args.output.open("w", newline="", encoding="utf-8") as fh:
        fieldnames = [
            "k",
            "t",
            "conductor",
            "root_number",
            "rank_bounds",
            "upper_bound",
            "rank_mwrank",
            "rank_all",
            "analytic_rank",
        ]
        writer = csv.DictWriter(fh, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    print(f"\nWrote {args.output}")
    print(f"Confirmed rank >= 4 candidates: {len(rank4_candidates)}")
    print(f"Upper-bound >= 4 uncertain candidates: {len(uncertain_candidates)}")

    if rank4_candidates:
        print("\n=== CONFIRMED RANK >= 4 RESULTS ===")
        for k, row in rank4_candidates:
            print(f"k={k:3d}, t={row['t']:5d}, rank_all={row['rank_all']}, root_number={row['root_number']}, conductor={row['conductor']}")

    if uncertain_candidates:
        print("\n=== UPPER-BOUND >= 4 RESULTS ===")
        for k, row in uncertain_candidates:
            print(f"k={k:3d}, t={row['t']:5d}, rank_bounds={row['rank_bounds']}, rank_mwrank={row['rank_mwrank']}, rank_all={row['rank_all']}")


if __name__ == "__main__":
    main()

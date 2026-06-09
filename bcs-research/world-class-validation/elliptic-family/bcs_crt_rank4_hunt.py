#!/usr/bin/env sage -python
"""
Hunt for rank >= 4 curves in the CRT 17/19 family.

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
    rank = safe(lambda: E.rank())
    analytic_rank = safe(lambda: E.analytic_rank())
    root_number = safe(lambda: E.root_number())
    conductor = safe(lambda: E.conductor())
    
    return {
        "k": k,
        "t": t,
        "conductor": conductor,
        "root_number": root_number,
        "rank_bounds": rank_bounds,
        "rank": rank,
        "analytic_rank": analytic_rank,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--k-min", type=int, default=21)
    parser.add_argument("--k-max", type=int, default=60)
    parser.add_argument("--output", type=Path, default=Path("bcs_crt_rank4_hunt.csv"))
    args = parser.parse_args()

    rows = []
    rank4_found = []
    
    for k in range(args.k_min, args.k_max + 1):
        print(f"Analyzing k={k} (t={66 + 323*k})", flush=True)
        row = analyze_rank(k)
        rows.append(row)
        
        rank = row["rank"]
        if isinstance(rank, int) and rank >= 4:
            rank4_found.append(row)
            print(f"  *** RANK >= 4 FOUND: k={k}, rank={rank} ***", flush=True)
    
    # Write CSV
    with args.output.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=["k", "t", "conductor", "root_number", "rank_bounds", "rank", "analytic_rank"])
        writer.writeheader()
        writer.writerows(rows)
    
    print(f"\nWrote {args.output}")
    print(f"Rank >= 4 candidates found: {len(rank4_found)}")
    
    if rank4_found:
        print("\n=== RANK >= 4 RESULTS ===")
        for row in rank4_found:
            print(f"k={row['k']:3d}, t={row['t']:5d}, rank={row['rank']}, root_number={row['root_number']}, conductor={row['conductor']}")


if __name__ == "__main__":
    main()

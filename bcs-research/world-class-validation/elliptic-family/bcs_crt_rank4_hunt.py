#!/usr/bin/env sage -python
"""
Stronger hunt for rank >= 4 curves in the CRT 17/19 family.

Usage:
  sage -python bcs_crt_rank4_hunt.py --k-min 21 --k-max 60 --output rank4_hunt.csv
"""

from __future__ import annotations

import argparse
import csv
import time
from pathlib import Path

from sage.all import EllipticCurve, QQ


known_bad = {
    66,
    3942,
    235210,
    1456150,
    652849,
    405754,
    28408885,
}


def curve(t: int):
    return EllipticCurve(QQ, [0, 17 + t, 0, 5, 4])


def safe(fn):
    try:
        return fn()
    except Exception as exc:
        return f"ERROR: {type(exc).__name__}"


def analyze_rank(k: int, mode: str) -> dict[str, object]:
    t = 66 + 323 * k
    E = curve(t)

    row = {
        "k": k,
        "t": t,
        "conductor": safe(lambda: E.conductor()),
        "root_number": safe(lambda: E.root_number()),
        "rank_bounds": safe(lambda: E.rank_bounds()),
        "upper_bound": None,
        "rank_mwrank": "",
        "rank_all": "",
        "analytic_rank": "",
    }

    if isinstance(row["rank_bounds"], tuple) and len(row["rank_bounds"]) == 2:
        row["upper_bound"] = row["rank_bounds"][1]

    if mode in {"deep", "candidate"}:
        row["rank_mwrank"] = safe(lambda: E.rank())
        row["rank_all"] = safe(lambda: E.rank(only_use_mwrank=False))
        row["analytic_rank"] = safe(lambda: E.analytic_rank())

    return row


def deep_analyze_candidates(rows: list[dict[str, object]]) -> list[dict[str, object]]:
    for row in rows:
        if isinstance(row["upper_bound"], int) and row["upper_bound"] >= 4:
            k = row["k"]
            t = row["t"]
            print(f"Deep analyzing candidate k={k} (t={t})", flush=True)
            E = curve(t)
            row["rank_mwrank"] = safe(lambda: E.rank())
            row["rank_all"] = safe(lambda: E.rank(only_use_mwrank=False))
            row["analytic_rank"] = safe(lambda: E.analytic_rank())
    return rows


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["fast", "deep", "candidate"], default="fast")
    parser.add_argument("--k-min", type=int, default=21)
    parser.add_argument("--k-max", type=int, default=60)
    parser.add_argument("--output", type=Path, default=Path("bcs_crt_rank4_hunt.csv"))
    args = parser.parse_args()

    rows = []
    start_total = time.perf_counter()

    for k in range(args.k_min, args.k_max + 1):
        t = 66 + 323 * k
        start = time.perf_counter()
        if t in known_bad:
            print(f"Skipping known bad t={t} (k={k})", flush=True)
            row = {
                "k": k,
                "t": t,
                "conductor": "SKIPPED",
                "root_number": "SKIPPED",
                "rank_bounds": "SKIPPED",
                "upper_bound": "SKIPPED",
                "rank_mwrank": "SKIPPED",
                "rank_all": "SKIPPED",
                "analytic_rank": "SKIPPED",
            }
        else:
            print(f"Analyzing k={k} (t={t}) mode={args.mode}", flush=True)
            row = analyze_rank(k, args.mode)
        rows.append(row)
        elapsed = time.perf_counter() - start
        print(f"  done in {elapsed:.1f}s", flush=True)

    if args.mode == "candidate":
        rows = deep_analyze_candidates(rows)

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

    total_elapsed = time.perf_counter() - start_total
    print(f"\nWrote {args.output}")
    print(f"Total time: {total_elapsed:.1f}s")

    confirmed = [r for r in rows if isinstance(r["rank_all"], int) and r["rank_all"] >= 4]
    upper = [r for r in rows if isinstance(r["upper_bound"], int) and r["upper_bound"] >= 4]

    print(f"Confirmed rank >= 4 candidates: {len(confirmed)}")
    print(f"Upper-bound >= 4 candidates: {len(upper)}")

    if confirmed:
        print("\n=== CONFIRMED RANK >= 4 RESULTS ===")
        for row in confirmed:
            print(f"k={row['k']:3d}, t={row['t']:5d}, rank_all={row['rank_all']}, root_number={row['root_number']}, conductor={row['conductor']}")

    if upper:
        print("\n=== UPPER-BOUND >= 4 RESULTS ===")
        for row in upper:
            print(f"k={row['k']:3d}, t={row['t']:5d}, rank_bounds={row['rank_bounds']}, rank_mwrank={row['rank_mwrank']}, rank_all={row['rank_all']}")


if __name__ == "__main__":
    main()

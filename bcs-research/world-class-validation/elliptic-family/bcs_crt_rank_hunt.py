#!/usr/bin/env python3
"""
Rank hunt for the BCS CRT class t = 66 + 323*k.

This script is intentionally operational: it runs the existing Sage scanner
one t-value at a time and writes an aggregate CSV plus a markdown summary.

Use in Codespaces from the repo root:

    python3 bcs-research/world-class-validation/elliptic-family/bcs_crt_rank_hunt.py \
      --k-min -20 --k-max 20 \
      --output bcs_crt_rank_hunt_k20.csv \
      --summary bcs_crt_rank_hunt_k20.md

By default this computes algebraic rank but not analytic rank. Add
`--analytic-rank` only for a small candidate set.
"""

from __future__ import annotations

import argparse
import csv
import subprocess
import sys
from pathlib import Path


FIELDS = [
    "k",
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


def repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def as_int(value: object):
    try:
        return int(str(value))
    except Exception:
        return None


def run_sage_for_t(root: Path, k: int, t: int, analytic_rank: bool) -> dict[str, str]:
    tmp_csv = root / f".rank_hunt_{t}.csv"
    tmp_md = root / f".rank_hunt_{t}.md"

    cmd = [
        "sage",
        "-python",
        str(root / "bcs-codespace-sage.py"),
        "--t-min",
        str(t),
        "--t-max",
        str(t),
        "--rank",
        "--output",
        str(tmp_csv),
        "--summary",
        str(tmp_md),
    ]
    if analytic_rank:
        cmd.append("--analytic-rank")

    print(f"[k={k:>4}, t={t:>8}] running Sage", flush=True)
    completed = subprocess.run(cmd, check=False)

    if completed.returncode != 0:
        return {"k": str(k), "t": str(t), "rank": f"SAGE_EXIT_{completed.returncode}"}

    if not tmp_csv.exists():
        return {"k": str(k), "t": str(t), "rank": "NO_CSV"}

    with tmp_csv.open("r", newline="", encoding="utf-8") as fh:
        rows = list(csv.DictReader(fh))

    tmp_csv.unlink(missing_ok=True)
    tmp_md.unlink(missing_ok=True)

    if not rows:
        return {"k": str(k), "t": str(t), "rank": "EMPTY_CSV"}

    src = rows[0]
    out = {field: src.get(field, "") for field in FIELDS}
    out["k"] = str(k)
    out["t"] = str(t)
    return out


def write_csv(path: Path, rows: list[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(rows)


def write_summary(path: Path, rows: list[dict[str, str]]) -> None:
    certified = [r for r in rows if as_int(r.get("rank")) is not None]
    uncertain = [r for r in rows if as_int(r.get("rank")) is None]
    rank_ge_4 = [r for r in certified if (as_int(r.get("rank")) or 0) >= 4]
    rank_ge_3 = [r for r in certified if (as_int(r.get("rank")) or 0) >= 3]

    lines = [
        "# BCS CRT Rank Hunt Summary",
        "",
        f"- Rows: `{len(rows)}`",
        f"- Certified rank rows: `{len(certified)}`",
        f"- Uncertain/error rows: `{len(uncertain)}`",
        f"- Rank >= 4: `{len(rank_ge_4)}`",
        f"- Rank >= 3: `{len(rank_ge_3)}`",
        "",
        "## Rank >= 4 Candidates",
        "",
    ]
    if rank_ge_4:
        for r in rank_ge_4:
            lines.append(
                f"- k={r.get('k')}, t={r.get('t')}, rank={r.get('rank')}, "
                f"conductor={r.get('conductor')}"
            )
    else:
        lines.append("- none")

    lines.extend(["", "## Rank >= 3 Certified Rows", ""])
    if rank_ge_3:
        for r in rank_ge_3:
            lines.append(
                f"- k={r.get('k')}, t={r.get('t')}, rank={r.get('rank')}, "
                f"analytic_rank={r.get('analytic_rank')}, conductor={r.get('conductor')}"
            )
    else:
        lines.append("- none")

    if uncertain:
        lines.extend(["", "## Uncertain/Error Rows", ""])
        for r in uncertain[:50]:
            lines.append(f"- k={r.get('k')}, t={r.get('t')}, rank={r.get('rank')}")
        if len(uncertain) > 50:
            lines.append(f"- ... {len(uncertain) - 50} more")

    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--k-min", type=int, default=-20)
    parser.add_argument("--k-max", type=int, default=20)
    parser.add_argument("--output", type=Path, default=Path("bcs_crt_rank_hunt.csv"))
    parser.add_argument("--summary", type=Path, default=Path("bcs_crt_rank_hunt.md"))
    parser.add_argument("--analytic-rank", action="store_true")
    parser.add_argument(
        "--resume",
        action="store_true",
        help="Skip rows already present in the output CSV.",
    )
    args = parser.parse_args()

    if args.k_min > args.k_max:
        raise SystemExit("--k-min must be <= --k-max")

    root = repo_root()
    existing: dict[int, dict[str, str]] = {}
    if args.resume and args.output.exists():
        with args.output.open("r", newline="", encoding="utf-8") as fh:
            for row in csv.DictReader(fh):
                try:
                    existing[int(row["k"])] = row
                except Exception:
                    pass

    rows: list[dict[str, str]] = []
    for k in range(args.k_min, args.k_max + 1):
        t = 66 + 323 * k
        if k in existing:
            print(f"[k={k:>4}, t={t:>8}] resume: using existing row", flush=True)
            rows.append(existing[k])
        else:
            rows.append(run_sage_for_t(root, k, t, args.analytic_rank))
        write_csv(args.output, rows)
        write_summary(args.summary, rows)

    print(f"Wrote {args.output}")
    print(f"Wrote {args.summary}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("Interrupted", file=sys.stderr)
        raise


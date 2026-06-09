#!/usr/bin/env sage -python
"""
BCS Codespace Sage runner

Family:
    E_t: y^2 = x^3 + (17+t)x^2 + 5x + 4

Use:
    sage -python bcs_codespace_sage.py --t-min -20 --t-max 20 --rank --analytic-rank

Outputs:
    bcs_family_sage.csv
    bcs_family_summary.md
"""

from __future__ import annotations

import argparse
import csv
from collections import Counter
from pathlib import Path

from sage.all import EllipticCurve, GF, QQ, factor, prime_divisors


DEFAULT_PRIMES = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37]


def parse_primes(raw: str) -> list[int]:
    return [int(x.strip()) for x in raw.split(",") if x.strip()]


def curve(t: int):
    return EllipticCurve(QQ, [0, 17 + t, 0, 5, 4])


def safe(label: str, fn):
    try:
        return fn()
    except Exception as exc:
        return f"{label}_ERROR: {type(exc).__name__}: {exc}"


def point_count_original_model(t: int, p: int) -> int:
    """Count E_t(F_p) directly on the original model plus point at infinity."""
    F = GF(p)
    a2 = F(17 + t)
    total = 1
    for x in F:
        rhs = x**3 + a2 * x**2 + F(5) * x + F(4)
        for y in F:
            if y**2 == rhs:
                total += 1
    return int(total)


def scan(args) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for t in range(args.t_min, args.t_max + 1):
        E = curve(t)
        disc = E.discriminant()
        smooth = disc != 0
        row: dict[str, object] = {
            "t": t,
            "smooth": bool(smooth),
            "ainvs": list(E.ainvs()),
            "discriminant": disc,
            "discriminant_factorization": factor(disc),
            "j": E.j_invariant() if smooth else "singular",
            "bad_primes": ";".join(str(p) for p in prime_divisors(disc)) if smooth else "",
        }

        if smooth:
            Emin = safe("minimal_model", lambda: E.global_minimal_model())
            row["minimal_ainvs"] = (
                list(Emin.ainvs()) if hasattr(Emin, "ainvs") else str(Emin)
            )
            conductor = safe("conductor", lambda: E.conductor())
            row["conductor"] = conductor
            row["conductor_factorization"] = (
                factor(conductor) if not isinstance(conductor, str) else ""
            )
            row["torsion_order"] = safe("torsion", lambda: E.torsion_order())
            row["torsion_structure"] = safe(
                "torsion_structure", lambda: E.torsion_subgroup().invariants()
            )
            row["rank_bounds"] = (
                safe("rank_bounds", lambda: E.rank_bounds())
                if args.rank_bounds or args.rank
                else ""
            )
            row["rank"] = safe("rank", lambda: E.rank()) if args.rank else ""
            row["analytic_rank"] = (
                safe("analytic_rank", lambda: E.analytic_rank())
                if args.analytic_rank
                else ""
            )
            row["gens"] = safe("gens", lambda: E.gens()) if args.rank else ""
        else:
            row.update(
                {
                    "minimal_ainvs": "",
                    "conductor": "",
                    "conductor_factorization": "",
                    "torsion_order": "",
                    "torsion_structure": "",
                    "rank_bounds": "",
                    "rank": "",
                    "analytic_rank": "",
                    "gens": "",
                }
            )

        for p in args.primes:
            n = point_count_original_model(t, p)
            row[f"count_p{p}"] = n
            row[f"a_p{p}"] = p + 1 - n
            row[f"good_p{p}"] = "yes" if smooth and disc % p != 0 else "no"
        rows.append(row)
    return rows


def write_csv(rows: list[dict[str, object]], output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    keys: list[str] = []
    for row in rows:
        for key in row:
            if key not in keys:
                keys.append(key)
    with output.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=keys)
        writer.writeheader()
        writer.writerows(rows)


def as_int(value):
    try:
        return int(value)
    except Exception:
        return None


def first_t(rows: list[dict[str, object]], limit: int = 20) -> str:
    vals = [str(row["t"]) for row in rows[:limit]]
    if not vals:
        return "none"
    suffix = "" if len(rows) <= limit else f" ... (+{len(rows) - limit} more)"
    return ", ".join(vals) + suffix


def write_summary(rows: list[dict[str, object]], output: Path) -> None:
    buckets = [
        ("count_p17 = 19", [r for r in rows if r.get("count_p17") == 19]),
        ("count_p19 = 17", [r for r in rows if r.get("count_p19") == 17]),
        ("a_p17 = -1", [r for r in rows if r.get("a_p17") == -1]),
        ("a_p19 = 3", [r for r in rows if r.get("a_p19") == 3]),
        (
            "bad primes include 17",
            [r for r in rows if "17" in str(r.get("bad_primes", "")).split(";")],
        ),
        (
            "bad primes include 19",
            [r for r in rows if "19" in str(r.get("bad_primes", "")).split(";")],
        ),
        (
            "bad primes include 1471",
            [r for r in rows if "1471" in str(r.get("bad_primes", "")).split(";")],
        ),
        ("rank = 1", [r for r in rows if as_int(r.get("rank")) == 1]),
        ("rank >= 2", [r for r in rows if (as_int(r.get("rank")) or -1) >= 2]),
    ]

    conductor_counter = Counter(
        str(r.get("conductor")) for r in rows if str(r.get("conductor", "")).isdigit()
    )

    lines = [
        "# BCS Security Family Scan Summary",
        "",
        f"- Rows: `{len(rows)}`",
        f"- t-range: `{rows[0]['t']}` to `{rows[-1]['t']}`" if rows else "- t-range: none",
        "",
        "## Important Buckets",
        "",
    ]
    for name, bucket in buckets:
        lines.append(f"- **{name}**: `{len(bucket)}` row(s); t = {first_t(bucket)}")

    if conductor_counter:
        lines.extend(["", "## Most Repeated Conductors", ""])
        for conductor, count in conductor_counter.most_common(20):
            lines.append(f"- `{conductor}`: `{count}` time(s)")

    output.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--t-min", type=int, default=-20)
    parser.add_argument("--t-max", type=int, default=20)
    parser.add_argument("--primes", type=parse_primes, default=DEFAULT_PRIMES)
    parser.add_argument("--output", type=Path, default=Path("bcs_family_sage.csv"))
    parser.add_argument("--summary", type=Path, default=Path("bcs_family_summary.md"))
    parser.add_argument("--rank", action="store_true")
    parser.add_argument(
        "--rank-bounds",
        action="store_true",
        help="Compute rank bounds; can invoke mwrank/descent and be slow.",
    )
    parser.add_argument("--analytic-rank", action="store_true")
    args = parser.parse_args()

    if args.t_min > args.t_max:
        raise SystemExit("--t-min must be <= --t-max")

    rows = scan(args)
    write_csv(rows, args.output)
    write_summary(rows, args.summary)

    print(f"Wrote CSV: {args.output}")
    print(f"Wrote summary: {args.summary}")


if __name__ == "__main__":
    main()

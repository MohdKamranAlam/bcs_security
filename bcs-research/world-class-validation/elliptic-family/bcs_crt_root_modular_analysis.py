#!/usr/bin/env sage -python
"""
Advanced Sage analysis for the BCS CRT 17/19 family.

This script adds the next layer beyond rank:

* global root number (rank parity signal),
* Frobenius traces a_17 and a_19,
* local reduction data at bad primes,
* rank / analytic rank for selected cases.

Use from repo root:

    sage -python bcs-research/world-class-validation/elliptic-family/bcs_crt_root_modular_analysis.py \
      --input bcs_crt_rank3_analytic.csv \
      --output bcs_crt_root_modular_rank3.csv \
      --report BCS_CRT_ROOT_MODULAR_ANALYSIS.md

Or analyze the full k-window:

    sage -python bcs-research/world-class-validation/elliptic-family/bcs_crt_root_modular_analysis.py \
      --k-min -20 --k-max 20 \
      --output bcs_crt_root_modular_k20.csv \
      --report BCS_CRT_ROOT_MODULAR_K20.md
"""

from __future__ import annotations

import argparse
import csv
from pathlib import Path

from sage.all import EllipticCurve, QQ, factor


FIELDS = [
    "k",
    "t",
    "ainvs",
    "minimal_ainvs",
    "discriminant",
    "conductor",
    "conductor_factorization",
    "root_number",
    "rank_bounds",
    "rank",
    "analytic_rank",
    "torsion_order",
    "ap17",
    "ap19",
    "count_p17",
    "count_p19",
    "bad_primes",
    "local_data",
    "gens",
]


def curve(t: int):
    return EllipticCurve(QQ, [0, 17 + t, 0, 5, 4])


def safe(fn):
    try:
        return fn()
    except Exception as exc:
        return f"ERROR: {type(exc).__name__}: {exc}"


def parse_t_values(raw: str) -> list[int]:
    return [int(x.strip()) for x in raw.split(",") if x.strip()]


def read_t_values(path: Path) -> list[int]:
    with path.open("r", newline="", encoding="utf-8") as fh:
        return [int(row["t"]) for row in csv.DictReader(fh)]


def local_data_summary(E) -> str:
    parts = []
    for p in E.bad_primes():
        ld = E.local_data(p)
        parts.append(
            f"p={p}:kodaira={ld.kodaira_symbol()},"
            f"cond_exp={ld.conductor_valuation()},"
            f"disc_exp={ld.discriminant_valuation()},"
            f"tamagawa={ld.tamagawa_number()}"
        )
    return "; ".join(parts)


def analyze_t(t: int) -> dict[str, object]:
    E = curve(t)
    Emin = safe(lambda: E.global_minimal_model())
    conductor = safe(lambda: E.conductor())
    rank_bounds = safe(lambda: E.rank_bounds())
    rank = safe(lambda: E.rank())
    analytic_rank = safe(lambda: E.analytic_rank())
    gens = safe(lambda: E.gens())
    ap17 = safe(lambda: E.ap(17))
    ap19 = safe(lambda: E.ap(19))
    root_number = safe(lambda: E.root_number())
    torsion_order = safe(lambda: E.torsion_order())
    local_data = safe(lambda: local_data_summary(E))

    return {
        "k": "" if (t - 66) % 323 else (t - 66) // 323,
        "t": t,
        "ainvs": list(E.ainvs()),
        "minimal_ainvs": list(Emin.ainvs()) if hasattr(Emin, "ainvs") else Emin,
        "discriminant": E.discriminant(),
        "conductor": conductor,
        "conductor_factorization": factor(conductor) if not isinstance(conductor, str) else "",
        "root_number": root_number,
        "rank_bounds": rank_bounds,
        "rank": rank,
        "analytic_rank": analytic_rank,
        "torsion_order": torsion_order,
        "ap17": ap17,
        "ap19": ap19,
        "count_p17": 18 - ap17 if isinstance(ap17, int) else "",
        "count_p19": 20 - ap19 if isinstance(ap19, int) else "",
        "bad_primes": ";".join(str(p) for p in E.bad_primes()),
        "local_data": local_data,
        "gens": gens,
    }


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(rows)


def write_report(path: Path, rows: list[dict[str, object]]) -> None:
    root_minus = [r for r in rows if str(r["root_number"]) == "-1"]
    root_plus = [r for r in rows if str(r["root_number"]) == "1"]
    rank3 = [r for r in rows if str(r["rank"]) == "3"]
    mismatch = [
        r for r in rows
        if str(r["rank"]).lstrip("-").isdigit()
        and str(r["root_number"]) in {"-1", "1"}
        and ((int(r["rank"]) % 2 == 1) != (str(r["root_number"]) == "-1"))
    ]

    lines = [
        "# BCS CRT Root Number and Modular Local Data",
        "",
        f"- Rows: `{len(rows)}`",
        f"- root number -1: `{len(root_minus)}`",
        f"- root number +1: `{len(root_plus)}`",
        f"- certified rank 3 rows: `{len(rank3)}`",
        f"- parity mismatches among certified rank rows: `{len(mismatch)}`",
        "",
        "## Rank-3 Rows",
        "",
        "| k | t | root number | rank | analytic rank | conductor | a17 | a19 |",
        "|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for r in rank3:
        lines.append(
            f"| {r['k']} | {r['t']} | {r['root_number']} | {r['rank']} | "
            f"{r['analytic_rank']} | {r['conductor']} | {r['ap17']} | {r['ap19']} |"
        )

    lines.extend(["", "## Interpretation", ""])
    lines.append(
        "The values `a17=-1` and `a19=3` are the Frobenius traces behind "
        "the point-count exchange `#F17=19`, `#F19=17`."
    )
    lines.append(
        "The root number gives the parity signal expected from the parity "
        "conjecture/BSD: root number `-1` corresponds to odd analytic rank."
    )
    if mismatch:
        lines.extend(["", "## Parity Mismatches", ""])
        for r in mismatch:
            lines.append(f"- t={r['t']}: root={r['root_number']}, rank={r['rank']}")

    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, help="CSV with a t column.")
    parser.add_argument("--t-values", type=parse_t_values)
    parser.add_argument("--k-min", type=int)
    parser.add_argument("--k-max", type=int)
    parser.add_argument("--output", type=Path, default=Path("bcs_crt_root_modular.csv"))
    parser.add_argument("--report", type=Path, default=Path("BCS_CRT_ROOT_MODULAR_ANALYSIS.md"))
    args = parser.parse_args()

    if args.input:
        ts = read_t_values(args.input)
    elif args.t_values:
        ts = args.t_values
    elif args.k_min is not None and args.k_max is not None:
        ts = [66 + 323 * k for k in range(args.k_min, args.k_max + 1)]
    else:
        ts = [66, 1681, 2973, 3942, 6526]

    rows = []
    for t in ts:
        print(f"Analyzing t={t}", flush=True)
        rows.append(analyze_t(t))

    write_csv(args.output, rows)
    write_report(args.report, rows)
    print(f"Wrote {args.output}")
    print(f"Wrote {args.report}")


if __name__ == "__main__":
    main()


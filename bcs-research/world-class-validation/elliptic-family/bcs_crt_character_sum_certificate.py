#!/usr/bin/env python3
"""
Character-sum certificate for the BCS CRT 17/19 exchange.

For

    E_t: y^2 = x^3 + (17+t)x^2 + 5x + 4

and an odd prime p of good reduction, the point count is

    #E_t(F_p) = p + 1 + sum_x chi(f_t(x)),

where chi is the quadratic character of F_p and

    f_t(x) = x^3 + (17+t)x^2 + 5x + 4.

This script gives a reproducible certificate for the two exact local facts:

    t = -2 mod 17  =>  #E_t(F_17) = 19
    t =  9 mod 19  =>  #E_t(F_19) = 17

No Sage dependency.
"""

from __future__ import annotations

import argparse
from pathlib import Path


def chi(a: int, p: int) -> int:
    a %= p
    if a == 0:
        return 0
    return 1 if pow(a, (p - 1) // 2, p) == 1 else -1


def f_value(t: int, x: int, p: int) -> int:
    return (x**3 + (17 + t) * x * x + 5 * x + 4) % p


def character_sum(t: int, p: int) -> tuple[int, list[tuple[int, int, int]]]:
    rows = []
    total = 0
    for x in range(p):
        fx = f_value(t, x, p)
        cx = chi(fx, p)
        total += cx
        rows.append((x, fx, cx))
    return total, rows


def point_count(t: int, p: int) -> int:
    s, _ = character_sum(t, p)
    return p + 1 + s


def residue_hits(p: int, target_count: int) -> list[int]:
    return [r for r in range(p) if point_count(r, p) == target_count]


def render_case(t: int, p: int, label: str) -> list[str]:
    s, rows = character_sum(t, p)
    n = p + 1 + s
    lines = [
        f"## {label}",
        "",
        f"- prime p: `{p}`",
        f"- residue t mod p: `{t % p}`",
        f"- character sum S_p(t): `{s}`",
        f"- point count p + 1 + S_p(t): `{p} + 1 + ({s}) = {n}`",
        "",
        "| x | f_t(x) mod p | chi(f_t(x)) |",
        "|---:|---:|---:|",
    ]
    lines.extend(f"| {x} | {fx} | {cx} |" for x, fx, cx in rows)
    lines.append("")
    return lines


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path("bcs_crt_character_sum_certificate.md"))
    args = parser.parse_args()

    hits17 = residue_hits(17, 19)
    hits19 = residue_hits(19, 17)

    lines = [
        "# BCS CRT 17/19 Character-Sum Certificate",
        "",
        "Family:",
        "",
        "```text",
        "E_t: y^2 = x^3 + (17+t)x^2 + 5x + 4",
        "```",
        "",
        "For odd prime `p`, the finite-field point count is",
        "",
        "```text",
        "#E_t(F_p) = p + 1 + sum_x chi(x^3 + (17+t)x^2 + 5x + 4)",
        "```",
        "",
        "where `chi` is the quadratic character modulo `p`.",
        "",
        "## Residue Scan",
        "",
        f"- residues mod 17 with `#E_t(F_17)=19`: `{hits17}`",
        f"- residues mod 19 with `#E_t(F_19)=17`: `{hits19}`",
        "",
        "Thus the relevant congruences are:",
        "",
        "```text",
        "t = 15 = -2 mod 17",
        "t = 9 mod 19",
        "```",
        "",
        "By CRT this is equivalent to:",
        "",
        "```text",
        "t = 66 mod 323",
        "```",
        "",
    ]
    lines.extend(render_case(15, 17, "Certificate for p=17"))
    lines.extend(render_case(9, 19, "Certificate for p=19"))

    args.output.write_text("\n".join(lines), encoding="utf-8")
    print(f"Wrote {args.output}")


if __name__ == "__main__":
    main()


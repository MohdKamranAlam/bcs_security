#!/usr/bin/env python3
"""
Pure-Python quick scan for Codespaces when Sage is not installed.

Family:
    E_t: y^2 = x^3 + (17+t)x^2 + 5x + 4

Use:
    python3 bcs_family_quick.py --t-min -500 --t-max 500
"""

from __future__ import annotations

import argparse
import csv
from fractions import Fraction
from pathlib import Path


DEFAULT_PRIMES = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37]


def parse_primes(raw: str) -> list[int]:
    return [int(x.strip()) for x in raw.split(",") if x.strip()]


def c4(t: int) -> int:
    return 16 * (t * t + 34 * t + 274)


def c6(t: int) -> int:
    return -32 * (2 * t**3 + 102 * t * t + 1689 * t + 9169)


def discriminant(t: int) -> int:
    return -16 * (16 * t**3 + 791 * t * t + 12662 * t + 66195)


def factorint(n: int) -> dict[int, int]:
    n = abs(n)
    out: dict[int, int] = {}
    p = 2
    while p * p <= n:
        while n % p == 0:
            out[p] = out.get(p, 0) + 1
            n //= p
        p += 1 if p == 2 else 2
    if n > 1:
        out[n] = out.get(n, 0) + 1
    return out


def fmt_factor(n: int) -> str:
    sign = "-" if n < 0 else ""
    factors = factorint(n)
    if not factors:
        return str(n)
    return sign + "*".join(
        str(p) if e == 1 else f"{p}^{e}" for p, e in sorted(factors.items())
    )


def j_invariant(t: int) -> Fraction:
    return Fraction(c4(t) ** 3, discriminant(t))


def count_points(t: int, p: int) -> int:
    a2 = (17 + t) % p
    total = 1
    for x in range(p):
        rhs = (x**3 + a2 * x * x + 5 * x + 4) % p
        total += sum(1 for y in range(p) if y * y % p == rhs)
    return total


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--t-min", type=int, default=-500)
    parser.add_argument("--t-max", type=int, default=500)
    parser.add_argument("--primes", type=parse_primes, default=DEFAULT_PRIMES)
    parser.add_argument("--output", type=Path, default=Path("bcs_family_quick.csv"))
    args = parser.parse_args()

    fields = [
        "t",
        "discriminant",
        "discriminant_factorization",
        "j",
        "bad_primes",
        "c4",
        "c6",
    ]
    for p in args.primes:
        fields += [f"count_p{p}", f"a_p{p}", f"good_p{p}"]

    with args.output.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields)
        writer.writeheader()
        for t in range(args.t_min, args.t_max + 1):
            disc = discriminant(t)
            j = j_invariant(t)
            row = {
                "t": t,
                "discriminant": disc,
                "discriminant_factorization": fmt_factor(disc),
                "j": f"{j.numerator}/{j.denominator}",
                "bad_primes": ";".join(str(p) for p in sorted(factorint(disc))),
                "c4": c4(t),
                "c6": c6(t),
            }
            for p in args.primes:
                n = count_points(t, p)
                row[f"count_p{p}"] = n
                row[f"a_p{p}"] = p + 1 - n
                row[f"good_p{p}"] = "yes" if disc % p != 0 else "no"
            writer.writerow(row)

    print(f"Wrote CSV: {args.output}")


if __name__ == "__main__":
    main()

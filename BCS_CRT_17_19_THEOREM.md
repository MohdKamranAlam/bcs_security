# BCS CRT 17/19 Theorem and Rank Evidence

## Family

We study the elliptic-curve family

\[
E_t: y^2 = x^3 + (17+t)x^2 + 5x + 4.
\]

## Theorem: Local 17/19 Exchange

For every integer \(t\) satisfying

\[
t \equiv 66 \pmod{323},
\]

we have

\[
\#E_t(\mathbb{F}_{17}) = 19
\]

and

\[
\#E_t(\mathbb{F}_{19}) = 17.
\]

## Proof

Over \(\mathbb{F}_p\), the point count depends only on \(t \bmod p\).

A direct residue computation gives:

\[
t \equiv 15 \pmod{17}
\]

equivalently

\[
t \equiv -2 \pmod{17}
\]

implies

\[
\#E_t(\mathbb{F}_{17}) = 19.
\]

Also,

\[
t \equiv 9 \pmod{19}
\]

implies

\[
\#E_t(\mathbb{F}_{19}) = 17.
\]

Solving

\[
t \equiv -2 \pmod{17},\qquad t \equiv 9 \pmod{19}
\]

by the Chinese Remainder Theorem gives

\[
t \equiv 66 \pmod{323}.
\]

Therefore every integer \(t \equiv 66 \pmod{323}\) satisfies the simultaneous local point-count exchange.

## Sage Rank Evidence

For tested values \(t=66+323k\), Sage gives:

| t | rank | analytic rank | conductor |
|---:|---:|---:|---:|
| -1226 | 1 | 1 | 226486904936 |
| -903 | 1 | 1 | 2786852076 |
| -580 | 2 | 2 | 45807637840 |
| -257 | 1 | 1 | 3560586688 |
| 66 | 3 | 3 | 71579352 |
| 389 | 2 | 2 | 533254264 |
| 712 | 2 | 2 | 98962884656 |
| 1035 | 1 | 1 | 99199980480 |
| 1358 | 2 | 2 | 332367922456 |
| 1681 | 3 | 3 | 78258195824 |

The strongest certified cases are:

\[
\operatorname{rank} E_{66}(\mathbb{Q}) = 3
\]

and

\[
\operatorname{rank} E_{1681}(\mathbb{Q}) = 3.
\]

## Caveat

The local 17/19 point-count exchange is exact.

The positive-rank pattern is computational evidence from tested values, not yet a theorem for all \(t \equiv 66 \pmod{323}\).

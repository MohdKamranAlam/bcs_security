# BCS CRT 17/19 Rank Finding

Family:

\[
E_t: y^2=x^3+(17+t)x^2+5x+4.
\]

## Local 17/19 Point-Count Exchange

The CRT class

\[
t \equiv 66 \pmod{323}
\]

satisfies the local finite-field exchange

\[
\#E_t(\mathbb{F}_{17})=19,\quad \#E_t(\mathbb{F}_{19})=17.
\]

This follows from the congruence conditions:

\[
t\equiv -2 \pmod{17},\quad t\equiv 9 \pmod{19}.
\]

## Sage Rank Evidence

Tested values \(t=66+323k\) show repeated positive Mordell-Weil rank.

| t | rank bounds | rank | analytic rank | point counts |
|---:|---:|---:|---:|---|
| -1549 | (2,4) | uncertified | 2 | #F17=19, #F19=17 |
| -1226 | (1,1) | 1 | 1 | #F17=19, #F19=17 |
| -903 | (1,1) | 1 | 1 | #F17=19, #F19=17 |
| -580 | (2,2) | 2 | 2 | #F17=19, #F19=17 |
| -257 | (1,1) | 1 | 1 | #F17=19, #F19=17 |
| 66 | (3,3) | 3 | 3 | #F17=19, #F19=17 |
| 389 | (2,2) | 2 | 2 | #F17=19, #F19=17 |
| 712 | (2,2) | 2 | 2 | #F17=19, #F19=17 |
| 1035 | (1,1) | 1 | 1 | #F17=19, #F19=17 |
| 1358 | (2,2) | 2 | 2 | #F17=19, #F19=17 |
| 1681 | (3,3) | 3 | 3 | #F17=19, #F19=17 |

## Main Finding

In the BCS family, the CRT class \(t\equiv66\pmod{323}\) gives a clean 17/19 finite-field exchange and repeatedly produces positive-rank curves. The strongest certified examples are:

\[
\operatorname{rank} E_{66}(\mathbb{Q}) = 3
\]

and

\[
\operatorname{rank} E_{1681}(\mathbb{Q}) = 3.
\]

## Caveat

The point-count exchange is exact and congruence-based. The rank pattern is computational Sage evidence from tested cases, not yet a theorem for all \(t\equiv66\pmod{323}\).

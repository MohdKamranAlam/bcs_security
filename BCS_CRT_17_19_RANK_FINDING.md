# BCS CRT 17/19 Rank Finding

Family:

\[
E_t: y^2=x^3+(17+t)x^2+5x+4.
\]

The CRT class

\[
t \equiv 66 \pmod{323}
\]

satisfies the local finite-field exchange

\[
\#E_t(\mathbb{F}_{17})=19,\quad \#E_t(\mathbb{F}_{19})=17.
\]

Computational Sage evidence for tested values shows repeated positive Mordell-Weil rank, including exact rank-3 examples at \(t=66\) and \(t=1681\).

See `bcs_crt_rank_table.csv` for the full table.

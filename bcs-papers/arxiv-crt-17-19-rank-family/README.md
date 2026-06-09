# Paper Draft: CRT 17/19 Rank Family

Working title:

```text
A CRT-Controlled 17/19 Point-Count Exchange in a Rank-Positive Elliptic Curve Family
```

This draft is a computational number-theory note attached to the BCS research
program. It is not a cryptographic security proof for BCS-521.

## Core Claims

- Exact theorem:
  `t = 66 mod 323 => #E_t(F_17)=19 and #E_t(F_19)=17`.
- Generic structure:
  the section `P=(0,2)` is non-torsion, so `rank E(Q(t)) >= 1`.
- Certified examples:
  `rank E_66(Q)=3` and `rank E_1681(Q)=3`.

## Build

```bash
pdflatex paper.tex
```

## Data Sources

- `../../BCS_CRT_17_19_THEOREM.md`
- `../../BCS_GENERIC_RANK_PROOF.md`
- `../../BCS_RANK3_MINI_REPORT.md`
- `../../bcs_crt_rank_table.csv`
- `../../bcs_crt_selected_rank.csv`


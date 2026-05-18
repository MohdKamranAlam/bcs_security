# Paper: Pell + LMFDB 1424.d1

## Title
*The Diophantine Equation `17B² + 5B + 4 = y²` and the Elliptic Curve LMFDB 1424.d1*

## Build
```bash
make           # produces paper.pdf
make clean
make distclean # also removes paper.pdf
```

Requires `pdflatex` and `bibtex`. The paper uses only standard packages
(`amsmath`, `amssymb`, `amsthm`, `geometry`, `hyperref`, `booktabs`,
`microtype`, `cite`, `mathtools`).

The Cyrillic Sha character `\Sha` requires the `cyrillic` package or
`unicode-math` if compiled with `xelatex`/`lualatex`. With pure
`pdflatex` the macro is defined to fall back gracefully.

## Submission to arXiv

1. Verify the paper compiles cleanly: `make clean && make`.
2. Check page count is 6–10 pages (currently first draft).
3. Run a spell-check pass on `paper.tex`.
4. Fill in author, affiliation, and email (currently placeholder).
5. Decide on arXiv subject classification:
   - **Primary:** `math.NT` (Number Theory)
   - **Secondary:** `math.AG` (Algebraic Geometry) — optional
6. Optionally cross-list to `cs.CR` (Cryptography) only if a follow-up
   IACR-paper-style cryptographic discussion is added.
7. Bundle for arXiv upload:
   ```
   tar czvf pell-1424d1.tar.gz paper.tex references.bib
   ```
8. Upload at https://arxiv.org/submit and follow the prompts.
9. After approval (usually 1 business day), the arXiv ID will be of
   the form `arXiv:26XX.NNNNN [math.NT]`.

## Optional journal targets (after arXiv)

- **Integers** (open access, free, fast turnaround)
- **Journal of Integer Sequences** (open access)
- **Moscow Journal of Combinatorics and Number Theory**
- **Acta Arithmetica** (more traditional)
- **Research in Number Theory** (Springer, open access)

## Key empirical inputs

All numerical values in the paper come from:
- LMFDB curve page: <https://www.lmfdb.org/EllipticCurve/Q/1424/d/1>
- Local artifact: `bcs-research/world-class-validation/lmfdb_curve_match.json`
- Local artifact: `bcs-research/world-class-validation/curve_invariants.json`
- Local artifact: `bcs-research/world-class-validation/sato_tate_results.json`

## Outstanding TODOs before submission

- [ ] Author name + affiliation + email
- [ ] Run a final $\chi^2$ Sato–Tate computation with binned counts
      filled into Table~\ref{tab:satotate} (currently shown as
      schematic).
- [ ] Optional: produce a small generator-search table to support the
      "open finite-search problem" in Section~7.
- [ ] Spell-check, grammar pass.
- [ ] Decide whether to list co-authors.

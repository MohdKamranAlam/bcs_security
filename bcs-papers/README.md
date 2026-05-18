# BCS — Phase 7 Academic Papers

This directory hosts two academic papers produced from the BCS / Phase 6
world-class validation work.

```
bcs-papers/
├── arxiv-pell-1424d1/         Number-theory short note (target: arXiv math.NT)
│   ├── paper.tex
│   ├── references.bib
│   ├── README.md
│   └── Makefile
└── iacr-eprint-bcs521-v2/      Cryptography paper (target: IACR ePrint)
    ├── paper.tex
    ├── references.bib
    ├── README.md
    └── Makefile
```

## Paper 1 — arxiv-pell-1424d1

**Title:** *The Diophantine Equation 17B² + 5B + 4 = y² and the Elliptic Curve LMFDB 1424.d1*

**Audience:** number theorists, computational arithmetic-geometry community.

**Length:** 6–10 pages.

**Submission:** arXiv `math.NT` (no peer review required), then optionally to
*Integers* / *Moscow Journal of Combinatorics and Number Theory* / *Acta Arithmetica* /
*Journal of Integer Sequences*.

**Status:** first complete draft.

## Paper 2 — iacr-eprint-bcs521-v2

**Title:** *BCS-521-V2: A Verifiably-Random 521-bit Elliptic Curve with Audit-Grade Reproducibility and a Hybrid Post-Quantum KEM*

**Audience:** cryptographers, IETF/CFRG, NIST PQC reviewers.

**Length:** 15–20 pages.

**Submission:** IACR Cryptology ePrint Archive (`eprint.iacr.org`).
Optional follow-up: Real World Crypto, IEEE S\&P, Selected Areas in Cryptography (SAC).

**Status:** first complete draft.

## Build instructions

Each paper has a `Makefile`:

```bash
cd arxiv-pell-1424d1
make            # produces paper.pdf
make clean
```

Requires `pdflatex`, `bibtex`, and standard LaTeX classes. Both papers use
plain `article` class (or `iacrtrans` for paper 2 if `iacrtrans.cls` is
available).

## Submission checklists

See each paper's local `README.md` for venue-specific submission steps.

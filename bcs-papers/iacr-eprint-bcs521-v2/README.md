# Paper: BCS-521-V2 (IACR ePrint)

## Title
*BCS-521-V2: A Verifiably-Random 521-bit Elliptic Curve with Audit-Grade
Reproducibility and a Hybrid Post-Quantum KEM*

## Build
```bash
make            # produces paper.pdf
make clean
make distclean  # also removes paper.pdf
```

Requires `pdflatex` and `bibtex`. Uses only standard packages
(`amsmath`, `amssymb`, `geometry`, `hyperref`, `booktabs`, `microtype`,
`cite`, `enumitem`, `listings`, `xcolor`).

If you have `iacrtrans.cls` (IACR's official class file) available,
you can switch to it by replacing the `\documentclass` line with
`\documentclass{iacrtrans}` and adjusting front matter accordingly.

## Submission to IACR Cryptology ePrint Archive

1. Build cleanly: `make distclean && make`.
2. Open <https://eprint.iacr.org/submit/> and choose
   *Submit a new paper*.
3. Fill in the form:
   - **Title:** copy from `paper.tex`
   - **Authors:** list with affiliation and email
   - **Abstract:** copy verbatim from `paper.tex`
   - **Keywords:** "elliptic curve cryptography; verifiably random
     curves; hybrid post-quantum KEM; ML-KEM; LMFDB; audit-grade"
4. Upload the `.tex` source bundle:
   ```bash
   tar czvf bcs521-v2-eprint.tar.gz paper.tex references.bib
   ```
5. ePrint will compile and produce a draft PDF; verify it looks correct,
   then publish.
6. Citation form: `https://eprint.iacr.org/2026/NNNN`.

## Optional follow-up venues

After ePrint posting:
- **CFRG / IETF Hybrid KEM I-D** — short technical note formatted as
  an Internet-Draft, citing this ePrint paper.
- **Selected Areas in Cryptography (SAC)** — full conference submission
  if reviewers consider the contribution sufficient.
- **Real World Crypto (RWC)** — short talk, primarily for deployment
  audience.
- **IEEE S&P / USENIX Security** — only if the paper grows to include
  a substantial new technical idea (e.g. formal verification of the
  constant-time path, or a novel hybrid composition).

## Key empirical inputs (cite from repo)

- Master seed and prime — `bcs521-v2-search/kahf_seeded_search.py`.
- Winning counter $c^* = 28738$ — `MEMORY[694b98cc]`.
- Curve audit JSON — `bcs_security_audit_*.json`.
- KAT JSON — `test_vectors_bcs256.json`,
  `bcs521_v2_kats.json` (output of `generate_bcs521_v2_kats.py`).
- LMFDB pedigree — companion paper, this directory's sibling
  `../arxiv-pell-1424d1/`.

## Outstanding TODOs before submission

- [ ] Author name + affiliation + email
- [ ] Replace the "..." in the master-seed SHA-512 hex
      (Appendix A) with the full 128-character hex value.
- [ ] Add a clean ASCII art / TikZ figure illustrating the
      Kahf-seeded prime-search loop (currently text-only).
- [ ] Cross-check the BSD numerical agreement in
      Section~\ref{sec:pedigree} against the latest LMFDB snapshot.
- [ ] If submitting as a NIST PQC contribution: add a section
      describing exact NIST-API compliance for the hybrid KEM
      (encaps/decaps signatures, byte serialisation).
- [ ] Decide whether `\Sha` (Cyrillic Sha for Tate--Shafarevich) is
      acceptable with default `cyrillic` package, or whether to
      switch to `\mathrm{Ш}` via `\usepackage[T2A]{fontenc}`.

## Reference implementations cited

- Python reference: `bcs521-v2-search/kahf_seeded_search.py`
- Rust audit-faithful port: `bcs-core-rust/src/kahf_seeded.rs`
- Hybrid KEM: `bcs-core-rust/src/hybrid.rs`
- Constant-time path: `bcs-core-rust/src/ct/` (feature-flagged)

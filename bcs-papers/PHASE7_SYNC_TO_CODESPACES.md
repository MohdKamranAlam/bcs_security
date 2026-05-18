# Phase 7 — Sync `bcs-papers/` to Codespaces (Step-by-step)

## What you have locally (Windows, `d:\project\interview_prepration\bcs-papers\`)

```
bcs-papers/
├── README.md
├── PHASE7_SYNC_TO_CODESPACES.md            ← (this file)
├── arxiv-pell-1424d1/
│   ├── paper.tex                           ← Pell + 1424.d1, full LaTeX draft
│   ├── references.bib
│   ├── Makefile
│   └── README.md
└── iacr-eprint-bcs521-v2/
    ├── paper.tex                           ← BCS-521-V2, full LaTeX draft
    ├── references.bib
    ├── Makefile
    └── README.md
```

Both papers are **first complete drafts** ready for build & light editing
(author/affiliation, schematic table fill-ins, spell-check).

---

## Goal — push these to GitHub via Codespaces

We want the following on the master branch of
<https://github.com/MohdKamranAlam/bcs_security>:

```
bcs-papers/                 ← whole directory committed
```

There are two equally valid sync strategies. Pick **A** if you prefer
working from Codespaces, **B** if you prefer pushing from your local
Windows machine.

---

## Strategy A — Recreate files in Codespaces, then push

Pros: no need to clone the repo locally; works from any browser.
Cons: lots of copy-paste.

### A.1 Open Codespaces terminal

```bash
cd /workspaces/bcs_security
mkdir -p bcs-papers/arxiv-pell-1424d1
mkdir -p bcs-papers/iacr-eprint-bcs521-v2
```

### A.2 Recreate each file

For each of the 10 files listed above, in VS Code web (your Codespace):

1. Right-click on the appropriate folder in the Explorer tree.
2. *New File* — enter the file name exactly (`paper.tex`, etc.).
3. Open the corresponding local file from
   `d:\project\interview_prepration\bcs-papers\<...>` in any text editor.
4. Select all (Ctrl-A) → copy (Ctrl-C) → paste (Ctrl-V) into VS Code.
5. Save (Ctrl-S).

There are exactly **10 files** to recreate. Each is independent.

### A.3 Build sanity-check (optional, in Codespaces)

```bash
sudo apt-get install -y texlive-latex-recommended \
                        texlive-latex-extra \
                        texlive-fonts-recommended \
                        texlive-bibtex-extra biber

cd bcs-papers/arxiv-pell-1424d1
make            # should produce paper.pdf with ~8 pages

cd ../iacr-eprint-bcs521-v2
make            # should produce paper.pdf with ~15 pages
```

If you don't have TeX in Codespaces, skip the build — it's a quality
check, not required for committing.

### A.4 Commit and push

```bash
cd /workspaces/bcs_security

git add bcs-papers/
git status                  # verify all 10 files staged
git commit -m "Phase 7: complete first drafts of both papers (Pell + BCS-521-V2)"

TOKEN=$(grep -v '^#' bcs-core-rust/.env | grep -oP '(?<==).*' | head -1)
git push "https://MohdKamranAlam:${TOKEN}@github.com/MohdKamranAlam/bcs_security.git" master
```

---

## Strategy B — Push directly from Windows

Pros: no copy-paste; uses what you already have.
Cons: need git installed on Windows and the GitHub token saved locally.

### B.1 If repository is not yet cloned locally

```powershell
cd d:\project
git clone https://MohdKamranAlam:<TOKEN>@github.com/MohdKamranAlam/bcs_security.git
```

This creates `d:\project\bcs_security\`.

### B.2 Copy `bcs-papers/` into the cloned repo

```powershell
robocopy `
    d:\project\interview_prepration\bcs-papers `
    d:\project\bcs_security\bcs-papers `
    /MIR
```

### B.3 Commit and push

```powershell
cd d:\project\bcs_security
git add bcs-papers
git commit -m "Phase 7: complete first drafts of both papers (Pell + BCS-521-V2)"
git push origin master
```

---

## After the push lands on GitHub

The Codespace will see the new files automatically (no manual pull
needed if the Codespace was created after the push; otherwise
`git pull` in the Codespace terminal).

You can then mark Phase 7 as **first-draft complete** in any project
tracker.

---

## Editing checklist before public submission

For each paper:

- [ ] Replace placeholder author / affiliation / email in `paper.tex`.
- [ ] Run a spell-check and grammar pass on `paper.tex`.
- [ ] Fill in any `\ldots` / schematic content (e.g. Sato–Tate table
      bins in the Pell paper; full master-seed hex in the BCS paper).
- [ ] Run `make` and inspect `paper.pdf` for layout issues.
- [ ] For the BCS-521-V2 paper, double-check Appendix A against the
      frozen values in `bcs-core-rust/src/kahf_seeded.rs`.

When all boxes are ticked, the papers are submission-ready
(arXiv `math.NT` for Pell, IACR ePrint for BCS-521-V2).

# BCS CRT 17/19 Research Plan

This plan upgrades the current finding from an internal repo note into a
reproducible computational number-theory note.

## Current Defensible Result

Family:

```text
E_t: y^2 = x^3 + (17+t)x^2 + 5x + 4.
```

Exact theorem:

```text
t = 66 mod 323  =>  #E_t(F_17) = 19 and #E_t(F_19) = 17.
```

Structural result:

```text
P=(0,2) is a non-torsion section, so rank E(Q(t)) >= 1.
```

Certified rank-3 examples:

```text
rank E_66(Q)   = 3, analytic rank = 3.
rank E_1681(Q) = 3, analytic rank = 3.
```

## What Makes It Interesting

The finding combines three usually separate layers:

1. A CRT-controlled local finite-field condition.
2. A global rational section on an elliptic surface.
3. Certified high-rank specializations inside the same CRT class.

This does not directly prove cryptographic security. It is mathematical
pedigree and computational arithmetic geometry attached to the BCS project.

## Publication Ladder

### Level 1: Reproducible Note

Goal: arXiv-style computational note.

Requirements:

- Exact CRT proof.
- Table of tested `t = 66 + 323k`.
- Sage scripts and CSV outputs.
- Rank-3 mini-report.
- Honest caveats.

Status: mostly complete.

### Level 2: Rank-Hunt Upgrade

Goal: find rank 4 or rank 5 in the same CRT class.

Why it matters:

- Rank 3 is interesting.
- Rank 4/5 in the same structured CRT class would make the result more
  notable computationally.

Command:

```bash
python3 bcs-research/world-class-validation/elliptic-family/bcs_crt_rank_hunt.py \
  --k-min -50 \
  --k-max 50 \
  --output bcs_crt_rank_hunt_k50.csv \
  --summary bcs_crt_rank_hunt_k50.md
```

### Level 3: Modular/Galois Explanation

Goal: explain whether the 17/19 local exchange has a modular-form or Galois
representation interpretation beyond the direct CRT proof.

Tasks:

- Identify LMFDB/newform data for `E_66` and `E_1681` if available.
- Record Frobenius traces:

```text
a_17 = -1
a_19 = 3
```

- Compare with Fourier coefficients of the attached modular forms.
- Check whether rank-3 examples have unusual root number/sign patterns.

### Level 4: Generic Rank Theorem

Current result:

```text
rank E(Q(t)) >= 1.
```

Harder target:

```text
rank E(Q(t)) exactly 1, or find/prove additional independent sections.
```

If additional independent sections exist over `Q(t)`, then the rank behavior
in the CRT class would have a deeper explanation.

## Honest Non-Claims

- We do not yet prove that all `t = 66 mod 323` have positive rank.
- We do not yet prove that this class has density of rank >= 2 or rank >= 3.
- We do not use the small-field 17/19 theorem as a cryptographic security
  proof for BCS-521.
- We do not claim BSD is proven; equal algebraic/analytic ranks are
  Sage-computed evidence for the tested curves.


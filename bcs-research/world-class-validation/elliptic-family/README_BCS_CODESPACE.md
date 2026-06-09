# BCS Security Codespace Run

From repo root:

```bash
cd bcs-research/world-class-validation/elliptic-family
```

## 1. Quick Scan Without Sage

Use this when Codespace does not have Sage installed:

```bash
python3 bcs_family_quick.py \
  --t-min -500 \
  --t-max 500 \
  --primes 2,3,5,7,11,13,17,19,23,29,31,37 \
  --output bcs_family_quick.csv
```

This gives:

- discriminant
- j-invariant
- bad primes from discriminant
- point counts over selected finite fields
- Frobenius traces `a_p`

## 2. Full Sage Scan

Install Sage if needed:

```bash
sudo apt-get update
sudo apt-get install -y sagemath
```

Run exact scan:

```bash
sage -python bcs_codespace_sage.py \
  --t-min -20 \
  --t-max 20 \
  --rank \
  --analytic-rank \
  --output bcs_family_sage.csv \
  --summary bcs_family_summary.md
```

For bigger ranges, first skip rank:

```bash
sage -python bcs_codespace_sage.py \
  --t-min -200 \
  --t-max 200 \
  --output bcs_family_sage_big.csv \
  --summary bcs_family_sage_big_summary.md
```

Do not use `--rank`, `--rank-bounds`, or `--analytic-rank` on large ranges
until the quick scan has identified a small candidate list. These options can
invoke descent/mwrank and may run for a long time.

## CRT Class Wide Scan

For the special class

```text
t = 66 + 323*k
```

run a wider invariant scan first:

```bash
cd /workspaces/bcs_security

python3 bcs-research/world-class-validation/elliptic-family/bcs_crt_wide_rank_scan.py \
  --k-min -20 \
  --k-max 20 \
  --no-rank \
  --output bcs_crt_k20_invariants.csv
```

Then run rank without analytic rank:

```bash
python3 bcs-research/world-class-validation/elliptic-family/bcs_crt_wide_rank_scan.py \
  --k-min -20 \
  --k-max 20 \
  --rank \
  --output bcs_crt_k20_rank.csv
```

Finally run analytic rank only for selected strong candidates:

```bash
python3 bcs-research/world-class-validation/elliptic-family/bcs_crt_wide_rank_scan.py \
  --t-values 66,1681 \
  --rank \
  --analytic-rank \
  --output bcs_crt_selected_rank.csv
```

## Advanced Layer: Character Sums, Root Numbers, Frobenius Data

Generate a pure-Python character-sum certificate for the exact CRT theorem:

```bash
python3 bcs-research/world-class-validation/elliptic-family/bcs_crt_character_sum_certificate.py \
  --output BCS_CRT_CHARACTER_SUM_CERTIFICATE.md
```

Analyze the certified rank-3 cases with Sage:

```bash
sage -python bcs-research/world-class-validation/elliptic-family/bcs_crt_root_modular_analysis.py \
  --input bcs_crt_rank3_analytic.csv \
  --output bcs_crt_root_modular_rank3.csv \
  --report BCS_CRT_ROOT_MODULAR_RANK3.md
```

Analyze the full `k=-20..20` CRT window:

```bash
sage -python bcs-research/world-class-validation/elliptic-family/bcs_crt_root_modular_analysis.py \
  --k-min -20 \
  --k-max 20 \
  --output bcs_crt_root_modular_k20.csv \
  --report BCS_CRT_ROOT_MODULAR_K20.md
```

## Baseline Expected for t = 0

For

```text
E_0: y^2 = x^3 + 17x^2 + 5x + 4
```

expected values:

- discriminant: `-1059120`
- j-invariant: `-5266130944/66195`
- bad primes: `2,3,5,1471`
- conductor: `353040`
- rank: `1`
- analytic rank: `1`
- generator: `(0, 2)` in original coordinates
- `#E_0(F_17) = 16`
- `#E_0(F_19) = 15`

## Root-Level Shortcut

The repository root also contains `bcs-codespace-sage.py`, so this works from
the repo root:

```bash
sage -python bcs-codespace-sage.py \
  --t-min -20 \
  --t-max 20 \
  --rank \
  --analytic-rank \
  --output bcs_family_sage.csv \
  --summary bcs_family_summary.md
```

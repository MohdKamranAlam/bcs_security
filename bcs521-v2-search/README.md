# BCS-521-V2 Deterministic Prime Search

This directory contains the implementation for **BCS-521-V2** — the V2
cipher suite of the BCS-521 family. V2 derives its 521-bit prime
deterministically from a SHA-512 of the canonical Kahf seed input,
giving a NIST P-256-style "verifiably random" origin.

> **V1 (existing BCS-521) is FROZEN and NOT touched by this directory.**
> See `bcs-spec/bcs-521.md` for V1.
> See `bcs-spec/bcs-521-v2.md` for the V2 (this) draft spec.

---

## What this is

Instead of a randomly searched 521-bit prime (V1's approach), V2 builds `p` by:

1. Building a frozen **canonical seed input** that combines:
   - protocol label `BCS-521-V2-Seed-v1`
   - the 5 sacred Kahf primes (alphabetical order)
   - the Bismillah curve coefficients `a2 = -2, a4 = 5, a6 = 4`
   - the target bit length `bits = 521`
2. `master_seed = SHA-512(canonical_input)` (64 bytes, frozen hex below)
3. Generating candidates `candidate(c) = mask521( SHA-512( seed ‖ ":block=0;counter=c" ) ‖ SHA-512( seed ‖ ":block=1;counter=c" ) )`
4. Iterating `c = 0, 1, 2, ...` and accepting the first `p` that satisfies
   - `p` prime (BPSW + 20 MR rounds)
   - `n = #E(F_p)` prime (PARI SEA, then BPSW + MR)
   - `n ≠ p` (not anomalous)

Anyone can re-derive the winning V2 prime from the seed text alone,
given the published winning counter. **No trapdoor possible.**

V1's prime stays exactly as it was, in `bcs-spec/bcs-521.md`. V1 and V2
are independent suites that share only the curve equation and generator.

---

## Frozen V2 canonical values (locked by tests)

```text
canonical_input  (147 bytes ASCII):
  BCS-521-V2-Seed-v1:p_kahf_first_decimal=2141;p_kahf_last_zf=2969;p_kahf_sleepers=7;p_kahf_surah_zf=19;p_kahf_years_zf=373;a2=-2;a4=5;a6=4;bits=521;

master_seed  (SHA-512, 64 bytes hex):
  a7e2095812a53b18111510409951b3472dcdbfdc49a08600dd83f3b644a8ebed
  dcd856198544a56d905272203057ee7b6c1a55b080fd8d51a9144b739ed95cbd

candidate(0).hex()  starts with  0x1b8ec6cb7c8819a2a74bb8f092f4ef96
candidate(1).hex()  starts with  0x1a2365833e84694635fc5975a8893150
candidate(2).hex()  starts with  0x192a5f1f9af87e2256108555f0b34ce5
```

Any future Rust port must reproduce the master seed and candidate
prefixes byte-for-byte.

---

## Quick start

### Local (Windows / Linux / macOS) — determinism smoke test, no PARI

```bash
python kahf_seeded_search.py --smoke
```

Validates the SHA-512 derivation, frozen canonical input, bit-width
invariants, and curve-identity check. Should finish in under 1 second.

### Run the regression suite (13 tests)

```bash
python -m unittest test_determinism.py -v
```

### Verify a specific counter reproduces a specific candidate

```bash
python kahf_seeded_search.py --verify 1234
```

Prints `p = candidate(1234)` and the seed parameters used. Useful for
cross-language parity (Rust ↔ Python should agree byte-for-byte).

### Full V2 521-bit search — Codespaces (PARI/GP required)

`gp` (PARI) is required for `ellap` (SEA cardinality at 521 bits). Use a
GitHub Codespace or any Linux machine with PARI installed.

```bash
# 1. Install PARI/GP if not already present
sudo apt-get update && sudo apt-get install -y pari-gp

# 2. Verify PARI is on PATH
gp --version | head -1   # expect: GP/PARI CALCULATOR Version 2.13.x or later

# 3. Run search inside tmux (will take hours)
tmux new -s bcs521-v2
python3 kahf_seeded_search.py --bits 521 --start 0 --max 1000000 \
    2>&1 | tee bcs521_v2_run.log
# Ctrl-B then D to detach.  tmux attach -t bcs521-v2 to reattach.

# 4. When found, the file kahf_seeded_certificate.json is written.
cat kahf_seeded_certificate.json
```

---

## Expected runtime

521-bit prime density ≈ 1 / 361, and `n` prime density (heuristic) ≈ 1 / 361,
so on average ≈ **130 000 counters** must be scanned before a winning pair
appears. SEA cardinality dominates wall-clock time:

| Hardware | Expected wall clock |
|---|---|
| 1 core (laptop)            | ~8 hours   |
| 4-core Codespaces          | ~2 hours   |
| 32-core GCP n2-standard-32 | ~15 minutes |

These are E[`c*`] estimates; actual single runs can vary 2–3× either way.

---

## Resume after preemption

The script writes `kahf_seeded_checkpoint.json` every ~30 seconds with the
last counter scanned. To resume:

```bash
LAST=$(jq -r .last_counter kahf_seeded_checkpoint.json)
python3 kahf_seeded_search.py --bits 521 --start $LAST --max 1000000
```

---

## Output files

| File | Purpose |
|---|---|
| `kahf_seeded_checkpoint.json`  | progress checkpoint (resume support)        |
| `kahf_seeded_certificate.json` | final certificate when `(p, n)` found       |
| `bcs521_v2_run.log`            | tee'd stdout if you used the `tee` command |

The certificate is the artefact you commit to the repo and reference from
the frozen V2 spec `bcs-spec/bcs-521-v2.md`.

---

## V1 vs V2 (at a glance)

| Aspect | V1 (`BCS-521`)             | V2 (`BCS-521-V2`)                    |
|---|---|---|
| Spec file       | `bcs-spec/bcs-521.md`      | `bcs-spec/bcs-521-v2.md`             |
| Prime origin    | Random parallel search      | Deterministic SHA-512(Kahf seed)     |
| Status today    | Frozen, audited             | Draft, search pending                |
| Mutual impact   | None — V1 is untouched      | None — V2 is fully independent       |
| Curve equation  | `y² = x³ − 2x² + 5x + 4`    | `y² = x³ − 2x² + 5x + 4` *(same)*    |
| Generator       | `(0, 2)`                    | `(0, 2)` *(same)*                    |

A deployment may select either suite at protocol negotiation time. Both
will share the same `bcs-core-rust` API surface (in a future Rust port);
they differ only in the underlying `(p, n)` pair.

---

## Why V2 is a stronger origin story than V1

NIST P-256, the most widely deployed elliptic curve, was generated using
an unexplained 160-bit hex seed `c49d3608 86e70493 6a6678 e1139d26
b7819f7e 90`. Critics have asked for years where that seed came from.
NIST has never given a satisfying answer.

By contrast, **every byte** of the BCS-521-V2 master seed traces back to:

- The Quranic master equation `T_A = 17·B² + 5·B + 4 = 6236`
- The 5 sacred Kahf primes verified in `quran_math.py::verify_kahf_prime_lock`
- The frozen alphabetical/decimal/ASCII canonicalization

Anyone arguing "trapdoor in the prime" must explain how a 521-bit
attacker-chosen prime could be smuggled through a SHA-512 hash of 147
bytes whose every character has Quranic provenance — which is
cryptographically infeasible.

This is the **strongest possible "nothing-up-my-sleeve" story** for a
fresh elliptic curve. V1 keeps its (already strong) audit; V2 adds this
extra layer for deployments that want the verifiable origin.

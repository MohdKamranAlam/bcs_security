# BCS-521-V2 Deterministic Prime Search — Tiered Confidence Ladder

This directory contains the implementation for **BCS-521-V2** — the V2 cipher
suite of the BCS-521 family. V2 derives its prime deterministically from a
SHA-512 of the canonical Kahf seed input, giving a NIST P-256-style
"verifiably random" origin.

> **V1 (existing BCS-521) is FROZEN and NEVER touched by this directory.**
> See `bcs-spec/bcs-521.md` for V1.
> See `bcs-spec/bcs-521-v2.md` for the V2 (this) draft spec.

---

## 1. Recommended workflow — the 3-tier ladder

Don't jump straight to 521 bits. Run a confidence ladder so you can SEE the
algorithm successfully find a `(p, n)` pair before committing to the long run:

| Tier | Bits | Expected wall clock (4-core Codespaces) | Purpose |
|---|---|---|---|
| 1 | 128 | **~1-2 minutes** | "Does the algorithm work end-to-end?" smoke proof |
| 2 | 256 | ~15-30 minutes | "Does it scale to production-grade SEA?" mid validation |
| 3 | 521 | ~hours | The actual BCS-521-V2 frozen prime |

Each tier writes its own certificate JSON (`kahf_seeded_certificate_<bits>.json`)
so they never overwrite each other.

### One-shot runner

```bash
# On Codespaces, after `git pull` and `apt install pari-gp`:
cd bcs521-v2-search
chmod +x run_tiers.sh
./run_tiers.sh                 # runs all three: 128 -> 256 -> 521
```

### Single tier only

```bash
./run_tiers.sh 128             # tier 1 only (proof of life)
./run_tiers.sh 128 256         # tiers 1 + 2 (skip the long one for now)
./run_tiers.sh 521             # tier 3 only (after you trust 1 and 2)
```

### Re-run a tier

```bash
FORCE=1 ./run_tiers.sh 128     # ignores existing cert, regenerates it
```

The runner first executes the smoke test + 16-test regression suite to
confirm the algorithm matches the locked Windows golden values, *then*
runs each requested tier in order, stopping on first failure.

---

## 2. What this is

Instead of a randomly searched 521-bit prime (V1's approach), V2 builds `p` by:

1. Building a frozen **canonical seed input** that combines:
   - protocol label `BCS-521-V2-Seed-v1`
   - the 5 sacred Kahf primes (alphabetical order)
   - the Bismillah curve coefficients `a2 = -2, a4 = 5, a6 = 4`
   - the target bit length `bits = N`
2. `master_seed = SHA-512(canonical_input)` (64 bytes; deterministic)
3. Generating candidates via SHA-512 with two block indices and a counter
4. Iterating `c = 0, 1, 2, ...` and accepting the first `p` that satisfies
   - `p` prime (BPSW + 20 MR rounds)
   - `n = #E(F_p)` prime (PARI SEA, then BPSW + MR)
   - `n ≠ p` (not anomalous)

Anyone holding the seed text can re-derive every candidate byte-for-byte
given just the published winning counter. **No trapdoor possible.**

---

## 3. Frozen V2 canonical values (locked by tests)

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

The above are for **bits = 521**. Each tier (128, 256, …) computes its own
master_seed because `bits=N` is part of the canonical input.

---

## 4. Manual (per-tier) commands — if you don't want the runner

```bash
cd bcs521-v2-search

# Tier 1 — 128-bit, fast smoke
python3 kahf_seeded_search.py --bits 128 --start 0 --max 200000 \
    --out kahf_seeded_certificate_128.json

# Tier 2 — 256-bit, mid run
python3 kahf_seeded_search.py --bits 256 --start 0 --max 500000 \
    --out kahf_seeded_certificate_256.json

# Tier 3 — full 521-bit (use tmux because of long wall clock)
tmux new -s bcs521-v2
python3 kahf_seeded_search.py --bits 521 --start 0 --max 2000000 \
    --out kahf_seeded_certificate_521.json 2>&1 | tee bcs521_v2_run_521.log
# Detach: Ctrl-B then D    Reattach: tmux attach -t bcs521-v2
```

---

## 5. Local sanity (no PARI required)

```bash
python kahf_seeded_search.py --smoke           # locks canonical encoding
python -m unittest test_determinism.py -v      # 16 unit tests
python kahf_seeded_search.py --verify 1234     # re-derive candidate(1234)
```

These run on plain Windows / macOS / Linux with no external dependencies.
They prove **determinism** but not **prime-finding** (which needs PARI).

---

## 6. Output files

| File | Purpose |
|---|---|
| `kahf_seeded_certificate_128.json` | Tier 1 (128-bit) winning prime + counter |
| `kahf_seeded_certificate_256.json` | Tier 2 (256-bit) winning prime + counter |
| `kahf_seeded_certificate_521.json` | Tier 3 (521-bit) winning prime + counter |
| `kahf_seeded_checkpoint_<bits>.json` | per-tier resume state (auto-written every ~30s) |
| `bcs521_v2_run_<bits>.log` | tee'd stdout if you used the manual commands |

Each certificate is **independent**. The 521-bit certificate is what gets
referenced from the frozen V2 spec `bcs-spec/bcs-521-v2.md`. The 128/256
certificates are confidence artefacts (you may keep or discard them).

---

## 7. Resume after preemption

The script writes `kahf_seeded_checkpoint_<bits>.json` every ~30 seconds with
the last counter scanned for that tier. To resume tier 521:

```bash
LAST=$(jq -r .last_counter kahf_seeded_checkpoint_521.json)
python3 kahf_seeded_search.py --bits 521 --start $LAST --max 2000000 \
    --out kahf_seeded_certificate_521.json
```

(Or just re-run `./run_tiers.sh 521` — the runner will detect the checkpoint.)

---

## 8. V1 vs V2 (at a glance)

| Aspect | V1 (`BCS-521`) | V2 (`BCS-521-V2`) |
|---|---|---|
| Spec file       | `bcs-spec/bcs-521.md`     | `bcs-spec/bcs-521-v2.md`             |
| Prime origin    | Random parallel search     | Deterministic SHA-512(Kahf seed)     |
| Status today    | Frozen, audited            | Draft, search pending                |
| Mutual impact   | None — V1 is untouched     | None — V2 is fully independent       |
| Curve equation  | `y² = x³ − 2x² + 5x + 4`   | `y² = x³ − 2x² + 5x + 4` *(same)*    |
| Generator       | `(0, 2)`                   | `(0, 2)` *(same)*                    |

A deployment may select either suite at protocol negotiation time. They
will share the same `bcs-core-rust` API surface (in a future Rust port);
they differ only in the underlying `(p, n)` pair.

---

## 9. Why V2's origin story is stronger than V1's

NIST P-256, the most widely deployed elliptic curve, was generated using an
unexplained 160-bit hex seed `c49d3608 86e70493 6a6678 e1139d26 b7819f7e 90`.
Critics have asked for years where that seed came from. NIST has never given
a satisfying answer.

By contrast, **every byte** of the BCS-521-V2 master seed traces back to:

- The Quranic master equation `T_A = 17·B² + 5·B + 4 = 6236`
- The 5 sacred Kahf primes verified in `quran_math.py::verify_kahf_prime_lock`
- The frozen alphabetical/decimal/ASCII canonicalization

Anyone arguing "trapdoor in the prime" must explain how a 521-bit
attacker-chosen prime could be smuggled through a SHA-512 hash of 147 bytes
whose every character has Quranic provenance — which is cryptographically
infeasible.

This is the **strongest possible "nothing-up-my-sleeve" story** for a fresh
elliptic curve. V1 keeps its (already strong) audit; V2 adds this verifiable
origin layer for deployments that demand it.

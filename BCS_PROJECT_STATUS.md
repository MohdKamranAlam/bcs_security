# Bismillah Cryptosystem (BCS) — Master Project Status

**Last updated:** 2026-05-17  
**Status:** Phase 3 complete + **independent cardinality proof complete**. **10/10 frozen test vectors PASS on Codespaces (Rust ↔ Python byte-equal, Kahf-bound included)** AND **Pari/GP SEA `VERDICT: PASS` on BCS-521 cardinality** (see `bcs-verify/bcs521_pari_proof_result.txt`). Next: constant-time Rust v0.2.0. See `BCS_WORLD_CLASS_ROADMAP.md` for full path to world-class.

---

## 1. Mission

Build the world's first **Qur'an-transparent, auditable, post-quantum-hybrid** elliptic curve cryptosystem, in which every constant is mathematically derived from Qur'anic numbers via the Bismillah master equation:

```text
T_A = 17·B² + 5·B + 4 = 6236
where  B = 19  (letters of Bismillah)
       T_A = 6236  (total ayahs of the Qur'an)

Curve:  E:  y² = x³ − 2x² + 5x + 4
Generator:  G = (0, 2)
Bismillah identity:  #E(F_17) = 19   ← Surah Kahf prime-lock
```

---

## 2. Curves — Current State

| Curve | Status | bits | ECDLP security | KDF | Storage |
|-------|--------|------|----------------|------|---------|
| **BCS-128** | toy, verified | 128 | ~2⁶⁴ | n/a | smoke-test only |
| **BCS-256** | CONDITIONAL PASS ✅ | 256 | ≈ 2¹²⁸ | HKDF-SHA-256 | spec frozen, Rust impl done, Colab audit done |
| **BCS-521** | **STRONG CLASSICAL ECC PASS** ✅ | 521 | ≈ 2²⁶⁰ | HKDF-SHA-512 | spec frozen, Rust 10/10 pass, independent Python verified, twist composite (safe with point validation) |

### BCS-521 parameters (frozen)

```text
p = 6684878480953803875615041384236581248565144626959181331935475284049641722241859916862100650133594325184551397342003884026080714561754188260987347401802009363
n = 6684878480953803875615041384236581248565144626959181331935475284049641722241859866323302474084432391273141914599968033004535887077643952596722791822942474231
a_p = 595387981786604061933914900482742035851821544827484116235664264555578850535133
G = (0, 2)
cofactor h = 1
```

Found via parallel **Rust + PARI/GP SEA** search on GitHub Codespaces (4-core), 166 attempts, **64 minutes**.

---

## 3. What's Done

### Phase 1 — Math foundation
- [x] Bismillah master equation derived
- [x] Curve `y² = x³ − 2x² + 5x + 4` chosen and validated on small fields
- [x] `#E(F_17) = 19` identity proven
- [x] Surah Kahf (18) prime-lock with neighbours 17 & 19 documented

### Phase 2 — BCS-256
- [x] 256-bit secure prime found
- [x] BPSW primality of `p` and `n`
- [x] Hasse bound, not-anomalous, not-supersingular checks
- [x] ECDLP ≈ 2¹²⁸
- [x] MOV embedding degree lower-bound `k ≥ 2³⁰`
- [x] Spec frozen: `bcs-spec/bcs-256.md`
- [x] Honest audit script: `BCS_HONEST_VERIFY_COLAB.py`
- [x] Rust reference impl with strict point validation
- [x] Sage proof script: `bcs-verify/bcs256_sage_proof.sage`
- [x] Twist policy: `bcs-verify/twist_validation_policy.md`

### Phase 3 — BCS-521 (NEW, completed today)
- [x] Parallel Rust+PARI orchestrator: `bcs521-search/`
- [x] Smoke test 128-bit: pass in 0.5 s
- [x] **521-bit prime found** via 4-core Codespaces (166 attempts, 64 min)
- [x] Audit: `p` prime, `n` prime, Hasse, not-anomalous, ECDLP ≈ 2²⁶⁰
- [x] Twist factorization: small factors `{43, 47, 58651, 106856077, …}` — composite, point validation enforced
- [x] Generator `G = (0, 2)` confirmed on curve
- [x] **Spec frozen:** `bcs-spec/bcs-521.md`
- [x] **Rust core dual-curve:** `bcs-core-rust/src/lib.rs` v0.2.0
- [x] **10/10 Rust tests pass** (5 BCS-256, 5 BCS-521)
- [x] **n·G = O** and **(n+1)·G = G** verified via Rust
- [x] **Twist factorization:** `43 × 47 × 50551 × R` where R = 495-bit composite (safe with point validation, like Curve25519)
- [x] **Verdict upgraded: STRONG CLASSICAL ECC PASS** (per independent reviewer)
- [x] **End-to-end ECDH demo works:** `cargo run --example bcs521_ecdh_demo`
- [x] Master state JSON backup: `bcs-spec/BCS_MASTER_STATE.json`

---

## 4. Repositories / Folders

```text
d:\project\interview_prepration\
├── bcs-spec\
│   ├── bcs-256.md                    BCS-256 spec
│   ├── bcs-521.md                    BCS-521 spec  (frozen 2026-05-16)
│   ├── BCS_MASTER_STATE.json         Authoritative parameter backup
│   ├── known-limitations.md
│   └── test-vectors-bcs-256.json     Draft
│
├── bcs-verify\
│   ├── bcs256_sage_proof.sage        Sage independent proof
│   ├── bcs521_sage_proof.sage        Sage independent proof  (NEW)
│   └── twist_validation_policy.md
│
├── bcs-core-rust\                    Rust reference implementation
│   ├── Cargo.toml                    v0.2.0, dual-curve
│   ├── src\lib.rs                    Generic Curve struct, both curves
│   ├── examples\bcs521_ecdh_demo.rs  Full ECDH flow demo
│   ├── examples\bcs256_ecdh_demo.rs
│   └── README.md
│
├── bcs521-search\                    Rust + PARI/GP parallel finder
│   ├── Cargo.toml
│   ├── src\main.rs
│   ├── README.md
│   └── RUN_ON_GCP.md
│
├── bcs-demo\halal-certificate\       Halal certificate use-case
│   ├── README.md
│   └── certificate_example.json
│
├── BCS_HONEST_VERIFY_COLAB.py        Colab audit BCS-256
├── BCS_521_VERIFY_COLAB.py           Colab audit BCS-521
├── BCS_521_SAGE_COLAB.py             Colab Sage runner BCS-521
├── BCS_521_TEST_VECTORS_COLAB.py     Test vector generator
├── BCS_COLAB_NOTEBOOK.py             Original prime search notebook
├── BCS_KAHF_256_COLAB.py             Kahf prime-lock audit
├── BCS_SECURITY_AUDIT_COLAB.py
├── quran_math.py                     Core Qur'anic-math library
├── QURAN_MATHEMATICAL_FINDINGS.md
└── BCS_PROJECT_STATUS.md             ← THIS FILE
```

---

## 5. Rust Implementation Summary

### `bcs-core-rust v0.2.0` API

```rust
use bcs_core_rust::{bcs256, bcs521, hkdf_sha512_64};

let curve = bcs521();                                     // or bcs256()
let alice_sk = curve.generate_private_key();
let alice_pk = curve.public_key(&alice_sk)?;
curve.validate_public_key(&bob_pk)?;                      // STRICT
let shared_x = curve.ecdh(&alice_sk, &bob_pk)?;           // 66 bytes
let key      = hkdf_sha512_64(&shared_x, salt, info);     // 64 bytes
```

**Tests (10/10 PASS):**
- `bcs256_generator_valid`
- `bcs256_invalid_rejected`
- `bcs256_n_times_g_infinity`
- `bcs256_ecdh_agreement`
- `bcs521_generator_valid`
- `bcs521_bits` (asserts p, n are 521-bit)
- `bcs521_doubling` (2G via doubling == 2G via add)
- `bcs521_invalid_rejected`
- `bcs521_n_times_g_infinity`
- `bcs521_ecdh_agreement`

**Demo verified output:**
```text
=== BCS-521 ECDH Demo ===
p bits = 521, n bits = 521
[OK] G=(0,2) on curve
[OK] Both public keys validated
[OK] ECDH shared secret matches (66 bytes)
shared_x  = 003b16cc6add3eebd0...
HKDF key  = bc0d3950456a7d014c7f...
[OK] Off-curve key rejected
=== BCS-521 ECDH demo complete ===
```

---

## 6. Open Items / Roadmap

### Phase 4 — Independent verification (NEXT — PRIORITY ORDER)

**1. Sage independent proof first (most important)**
- [ ] Run `BCS_521_SAGE_COLAB.py` in Colab with SageMath kernel (~30 min)
- [ ] Goal: `p.is_prime(proof=True)`, `n.is_prime(proof=True)`, `E.cardinality(proof=True) == n`
- [ ] Embedding degree exact computation
- [ ] After pass: upgrade status to **VERIFIED HIGH-SECURITY CLASSICAL ECC**

**2. Freeze test vectors (after Sage pass)**
- [ ] `bcs521_test_vectors.json` via `BCS_521_TEST_VECTORS_COLAB.py`
- [ ] Include: private key, public key, shared secret, HKDF output, AES-GCM ciphertext + tag
- [ ] Rust `bcs-core-rust` tests to use these frozen vectors exactly

**3. Rust regression against test vectors**
- [ ] Update `bcs521_ecdh_agreement` test to use frozen vector from Sage-proven run

### Phase 5 — Post-Quantum Hybrid
- [ ] Wire protocol design: BCS-521 ECDH + ML-KEM-1024
- [ ] Final-key derivation:
  ```text
  k = HKDF-SHA-512(BCS_ecdh ‖ MLKEM_ss, salt="BCS-521-PQ-v1", info=ctx)
  ```
- [ ] Rust integration with `pqcrypto-mlkem` crate
- [ ] Test vectors for hybrid mode

### Phase 6 — Production hardening
- [ ] Fixed-width field representation (replace BigUint)
- [ ] Constant-time scalar multiplication (Montgomery ladder)
- [ ] No secret-dependent branches/memory accesses
- [ ] Audited RNG and key handling
- [ ] Fuzzing harness (cargo-fuzz)

### Phase 7 — Real-world demo
- [ ] Halal certificate signing demo end-to-end
- [ ] QR-code payload spec & verifier
- [ ] Web demo / mobile app PoC

### Phase 8 — External review
- [ ] Submit to ≥ 2 independent cryptographers
- [ ] White paper draft
- [ ] Public release with full transparency

---

## 7. Known Limitations (honest)

| Limitation | Status |
|---|---|
| Not constant-time | Acknowledged — research reference only |
| Twist order composite | Mitigated by mandatory point validation |
| No ECPP/Pocklington certificate | Pending Sage |
| No external cryptographer audit | Pending Phase 8 |
| Shor's algorithm breaks ECC | Mitigated only by PQ hybrid (Phase 5) |

Full details: `bcs-spec/known-limitations.md`

---

## 8. Reproduction Quick-start

```bash
# 1) Find a fresh 521-bit secure prime (4-core, ~1 hr)
cd bcs521-search
cargo build --release
./target/release/bcs521-search --bits 521 --workers 0 --time-budget-sec 86400

# 2) Run the Rust core tests
cd ../bcs-core-rust
cargo test --release
cargo run --release --example bcs521_ecdh_demo

# 3) Honest audit in Colab (Python only)
#    -> BCS_521_VERIFY_COLAB.py

# 4) Independent Sage proof in Colab (SageMath kernel)
#    -> BCS_521_SAGE_COLAB.py

# 5) Generate frozen test vectors
#    -> BCS_521_TEST_VECTORS_COLAB.py
```

---

## 9. Niyat (Intention)

> لَيْسَ لِلْإِنسَانِ إِلَّا مَا سَعَىٰ — *An-Najm 53:39*

The goal is **honest, rigorous mathematics** — not marketing.  
Every parameter is auditable. Every check is named.  
"Bismillah" is in the name only after external review confirms the math.

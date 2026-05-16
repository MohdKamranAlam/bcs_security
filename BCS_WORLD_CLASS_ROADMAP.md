# BCS World-Class Roadmap

**Status as of 2026-05-17 02:39 IST**

This document is the **single source of truth** for taking the BCS cryptosystem from
"verified research prototype" to **world-class, production-grade, standardized**
elliptic-curve infrastructure.

---

## 0. Current Status — Honest Scorecard

| # | Component | Status | Evidence | Grade |
|---|---|---|---|---|
| 1 | BCS-256 curve `y²=x³−2x²+5x+4 mod p_256` | ✅ Frozen, audited | `bcs-spec/bcs-256.md`, Pari/GP verified | **A** |
| 2 | BCS-521 curve same form, p_521 found | ✅ Frozen, audited | `bcs-spec/bcs-521.md`, Codespaces 2026-05-16 | **A** |
| 3 | Rust reference impl (BigUint affine) | ✅ Working | `bcs-core-rust/src/lib.rs`, all unit tests pass | **B+** |
| 4 | Frozen test vectors (10 cases) | ✅ Verified on Codespaces | `tests/test_vectors_521.rs`, 10/10 PASS 2026-05-17 | **A** |
| 5 | Kahf binding (5 sacred primes) | ✅ Cross-lang parity | Rust+Python byte-equal DST + HKDF + AES-GCM | **A+** |
| 6 | Sage `proof=True` cardinality | ⏳ Pending | `bcs-verify/bcs521_sage_proof.sage` ready, needs Colab run | **—** |
| 7 | Full twist factorization | ⚠️ Partial | Small factors 43·47·50551 found; 495-bit composite remains | **C** |
| 8 | Constant-time implementation | ❌ Missing | BigUint is variable-time → timing-attack vulnerable | **F** |
| 9 | Post-quantum hybrid | ❌ Missing | No ML-KEM integration yet | **F** |
| 10 | Formal verification (fiat-crypto) | ❌ Missing | No machine-checked proofs of field ops | **F** |
| 11 | External cryptographer audit | ❌ Missing | No third-party review | **F** |
| 12 | Halal certificate demo (E2E) | ⏳ Designed | `bcs-demo/halal-certificate/README.md` exists, no app | **C** |
| 13 | IETF/CFRG draft | ❌ Missing | Not submitted | **F** |
| 14 | Multi-language SDKs (C, Py, JS, Go) | ❌ Missing | Only Rust + Python reference | **F** |
| 15 | Bug bounty program | ❌ Missing | No public listing | **F** |
| 16 | Academic paper | ❌ Missing | Not written | **F** |

**Honest overall grade today: B−** (strong math, weak production-engineering).
**Target grade: A+** (mathematical proof + constant-time + audited + standardized + adopted).

---

## 1. Completion Plan — Each F → A

For every ❌/⚠️ item: **What · Why · How · Time · Cost · Owner**.

---

### Item 6 — Sage `proof=True` Cardinality Certificate
- **What:** Independent, formal proof that `#E(F_p_521) = n` using Schoof-Elkies-Atkin (SEA).
- **Why:** Without this, mathematicians treat the count as "probabilistic" (Miller-Rabin ≠ proof). With SEA proof=True, `n` is mathematically certified prime and Hasse-bound-satisfying.
- **How:**
  1. Open Google Colab → install Sage:
     ```bash
     !apt -qq install sagemath-jupyter 2>&1 | tail -3
     ```
  2. Run the existing script `bcs-verify/bcs521_sage_proof.sage` (already authored).
  3. Save signed certificate JSON to `bcs-verify/bcs521_cardinality_certificate.json`.
- **Expected runtime:** 30–90 minutes on Colab Pro (4-core).
- **Cost:** $0 (Colab free) or $10/month (Colab Pro for faster).
- **Owner:** Mohd Kamran (run the notebook).
- **Acceptance criteria:** `E.cardinality(algorithm='pari', proof=True) == n` returns True; certificate file committed.

---

### Item 7 — Full Twist Factorization
- **What:** Factorize the remaining **495-bit composite** of the twist order using Pollard rho + ECM up to digit-limit 70.
- **Why:** Even though point validation makes twist attacks impossible at protocol level, fully knowing the twist factorization eliminates "what if there's a small factor we missed" worries from auditors.
- **How:**
  1. Use `cypari2` (Pari/GP Python binding) OR Sage:
     ```python
     from sympy import factorint
     # Or in Sage:
     # n_twist_remaining.factor(limit=10**70)
     ```
  2. Run ECM B1=10^7 (1 hour), then B1=10^8 (8 hours) if needed.
  3. If still composite after 1 day → declare "≥495-bit factor present" as final answer.
- **Cost:** $0 (free compute time on Colab/Codespaces).
- **Acceptance criteria:** Updated twist table in `bcs-521.md` with all known factors and bit-length of the unfactored portion.

---

### Item 8 — Constant-Time Rust v0.2.0 (BIGGEST GAP)
- **What:** Replace `BigUint`-based arithmetic with constant-time field operations and constant-time scalar multiplication.
- **Why:** **THIS IS THE #1 PRODUCTION BLOCKER.** Variable-time code leaks secret key bits via timing channels. No bank/government will use BCS without this.
- **How:**
  1. **9-limb 521-bit field** (`u64` × 9 = 576 bits, 521 used):
     ```rust
     pub struct Fp521([u64; 9]);
     ```
  2. **Montgomery form** for multiplication:
     - Pre-compute `R = 2^576 mod p`, `R² mod p`
     - `mont_mul(a, b)`: schoolbook + reduce, constant-time
  3. **Inversion via Fermat:** `a^(p-2) mod p` using addition-chain (no branches).
  4. **Constant-time conditional swap:**
     ```rust
     use subtle::{Choice, ConditionallySelectable};
     ```
  5. **Montgomery ladder** for scalar mul (replaces double-and-add):
     ```rust
     fn scalar_mul_ct(k: &Scalar, p: &Point) -> Point {
         let (mut r0, mut r1) = (Point::identity(), p.clone());
         for bit in k.bits_msb_first() {
             let b = Choice::from(bit as u8);
             Point::conditional_swap(&mut r0, &mut r1, b);
             r1 = r0.add(&r1);
             r0 = r0.double();
             Point::conditional_swap(&mut r0, &mut r1, b);
         }
         r0
     }
     ```
  6. **Side-channel hygiene:** `zeroize::Zeroize` on Drop for `Scalar` and shared secrets.
  7. **Test:** Run **dudect** (timing test) — assert no significant timing difference for 10⁶ random inputs.
- **Time:** 1 week solo (or 3 days with focus).
- **Cost:** $0.
- **Crates:** `subtle`, `zeroize`, `crypto-bigint` (optional helper).
- **Acceptance criteria:**
  - `#![forbid(unsafe_code)]` at crate root.
  - All existing 10 test vectors still pass.
  - `cargo bench` numbers within 5× of OpenSSL P-521.
  - dudect test passes.

---

### Item 9 — Post-Quantum Hybrid Protocol (BCS-521 + ML-KEM-1024, Kahf-bound)
- **What:** Wire protocol that combines classical ECDH (BCS-521) and PQ KEM (ML-KEM-1024, NIST FIPS-203 winner) so security holds if either survives quantum attacks.
- **Why:** NIST migration deadline 2030. Pure ECC dies when Shor's algorithm runs on a fault-tolerant quantum computer (estimated 2030-2040). Hybrid = belt + suspenders.
- **How:**
  1. Wire format:
     ```text
     Initiator → Responder:  ec_pub_521 || mlkem_pub_1024
     Responder → Initiator:  ec_pub_521 || mlkem_ciphertext

     shared = HKDF-SHA-512(
         salt   = nil,
         ikm    = ECDH(BCS-521) || ML-KEM.decap,
         info   = KahfDST("BCS-521+MLKEM1024-Hybrid-v1") || transcript_hash,
         length = 64
     )
     ```
  2. Rust impl using `ml-kem` crate (RustCrypto org, FIPS-203 ref impl).
  3. Spec: `bcs-spec/bcs-521-pq-hybrid.md`.
  4. Test vectors with frozen MLKEM seed.
- **Time:** 3 days.
- **Cost:** $0.
- **Acceptance criteria:** Round-trip works, transcript binding tested, Kahf DST appears in info.

---

### Item 10 — Formal Verification (fiat-crypto)
- **What:** Use the fiat-crypto tool (MIT/Google, used by BoringSSL/Firefox/Tor) to **machine-prove** that the field arithmetic is correct.
- **Why:** Manual implementations have bugs (e.g., CVE-2014-0224, CVE-2020-0601). Formal verification eliminates this class of vulnerability.
- **How:**
  1. Add `Fp521` spec to fiat-crypto's prime list:
     ```
     ./src/Specific/solinas64/Fp521.v   // prime = our p_521
     ```
  2. Run `make` → outputs verified Rust/C code for add/mul/sub/square.
  3. Replace hand-written Montgomery ops with fiat-crypto output.
- **Time:** 1 week (steep learning curve for Coq).
- **Cost:** $0.
- **Owner:** Need Coq/Rust expert (or self-learn over 2 weeks).
- **Acceptance criteria:** All field ops sourced from fiat-crypto generation; spec committed in `bcs-formal/`.

---

### Item 11 — External Cryptographer Audit
- **What:** Pay a reputable security firm to audit BCS spec + Rust impl + Kahf binding.
- **Why:** "I tested it" ≠ "Trail of Bits says it's safe." Banks/governments require third-party audit.
- **How / Options:**

  | Firm | Type | Cost | Time | Reputation |
  |---|---|---|---|---|
  | Trail of Bits | Comprehensive | $40k–$80k | 4–6 weeks | Top-tier US |
  | NCC Group | Comprehensive | $50k–$100k | 4–8 weeks | Top-tier UK |
  | Cure53 | Web + crypto | $20k–$50k | 3–5 weeks | Top-tier DE |
  | Quarkslab | Crypto focus | $30k–$70k | 4–6 weeks | Top-tier FR |
  | Solo (Filippo Valsorda) | Crypto only | $10k–$20k | 2–3 weeks | Highly respected |
- **Recommendation:** Start with **Filippo Valsorda solo review** ($10k) to find obvious issues; then full Trail of Bits ($40k+) before any major customer.
- **Time:** 4–8 weeks lead time + audit.
- **Cost:** $10k minimum, $50k recommended.
- **Acceptance criteria:** Public audit report PDF, all High/Medium findings remediated.

---

### Item 12 — Halal Certificate End-to-End Demo
- **What:** Working web app + mobile SDK that issues and verifies Kahf-bound digital certificates for Islamic finance contracts, halal-supply-chain proofs, Quran translation signatures.
- **Why:** Concrete product for client pitch. Demonstrates real-world utility of Kahf binding (only BCS does this; ed25519/P-521 don't).
- **How:**
  1. **Backend (Rust + Axum):**
     - `POST /issue` — sign payload with BCS-521 + Kahf DST.
     - `POST /verify` — verify signature, return signer DN + timestamp.
  2. **Frontend (React + TailwindCSS + shadcn/ui):**
     - Certificate viewer (PDF-like).
     - QR code → verify URL.
     - Beautiful Islamic-themed UI.
  3. **Mobile SDK (Kotlin + Swift wrappers via UniFFI):** Sign/verify locally.
  4. **Use cases shipped:**
     - Halal-certified product (e.g., Tayyabaat seal).
     - Nikah-nama (marriage contract) digital signature.
     - Quran translation provenance.
- **Time:** 1 week MVP, 3 weeks polished.
- **Cost:** $50/yr (domain + hosting).
- **Acceptance criteria:** Live demo on `bcs.tayyabaat.com` (or chosen domain).

---

### Item 13 — IETF/CFRG Draft
- **What:** Submit `draft-bcs-521-curve-00` to the Crypto Forum Research Group at IETF.
- **Why:** Path to becoming a recognized RFC. Required for adoption by browsers, TLS libraries, governments.
- **How:**
  1. Use ed25519 (RFC 8032) and X25519 (RFC 7748) as templates.
  2. Include: curve params, point encoding, validation rules, test vectors, security considerations, IANA registration.
  3. Mention Kahf binding as **optional domain separator** (CFRG-friendly framing, no religious advocacy in normative text).
  4. Submit via Datatracker, attend CFRG meetings (virtual, free).
- **Time:** 2 weeks initial draft + 6–18 months iteration to RFC.
- **Cost:** $0 (free, but time-intensive).
- **Acceptance criteria:** Draft published on datatracker.ietf.org with `-00` suffix.

---

### Item 14 — Multi-Language Reference Libraries
- **What:** Port BCS to C, Python, JavaScript/WASM, Go.
- **Why:** Adoption ≠ Rust-only. Banks use Java/C++, web devs use JS, scientists use Python.
- **How:**

  | Language | Approach | Time |
  |---|---|---|
  | **Python** | `pyo3` wrapper around `bcs-core-rust` | 1 week |
  | **JS/WASM** | `wasm-pack` from `bcs-core-rust` | 1 week |
  | **C** | Hand-port or `cbindgen` from Rust | 2 weeks |
  | **Go** | `cgo` over C, or pure Go | 2 weeks |
  | **Java/Kotlin** | JNI over C, or UniFFI | 2 weeks |
- **Time:** 6 weeks total (2 langs in parallel).
- **Cost:** $0.
- **Acceptance criteria:** Each lang has the same 10 test vectors passing.

---

### Item 15 — Bug Bounty Program
- **What:** Public listing on HackerOne or Immunefi with payouts for crypto vulnerabilities.
- **Why:** Crowdsourced security review. White-hat researchers find what audit firms miss.
- **How:**
  1. Set up HackerOne page (`hackerone.com/bcs-crypto`).
  2. Scope: BCS-521 reference impl, Kahf binding, halal demo.
  3. Payout tiers: Critical $25k, High $10k, Medium $2k, Low $200.
- **Time:** 1 week setup.
- **Cost:** $10k–$50k pool to start; pay-per-bug after.
- **Acceptance criteria:** Live bounty page with ≥3 valid submissions in first 6 months.

---

### Item 16 — Academic Paper
- **What:** Peer-reviewed paper on arXiv (preprint) then submit to **Crypto / Eurocrypt / Asiacrypt** or **IACR ePrint Archive**.
- **Why:** Academic legitimacy. Required citation for IETF draft. Mathematicians take papers seriously, not GitHub READMEs.
- **Title (proposed):** *"BCS-521: A Novel 521-bit Prime Curve with Liturgical Domain Separation"*
- **Sections:**
  1. Introduction (Bismillah-Kahf mathematical structure).
  2. Curve construction `y² = x³ − 2x² + 5x + 4`.
  3. Prime search algorithm + Hasse bound proof.
  4. Security analysis (ECDLP, MOV, anomalous, twist).
  5. Kahf domain separator: motivation, encoding, security claims.
  6. Performance benchmarks.
  7. Comparison with P-521, ed448, secp521r1.
  8. Open problems.
- **Time:** 6 weeks writing + 2 months review.
- **Cost:** $0 (arXiv free; conference fees $500–$1500 if accepted).
- **Acceptance criteria:** Paper on arXiv, ePrint number assigned.

---

## 2. Recommended Execution Order

```
WEEK 0 (NOW — done today!): Codespaces 10/10 tests ✅
WEEK 1: Sage cardinality proof + full twist factor                  [Math credibility]
WEEK 2-3: Constant-time Rust v0.2.0                                 [Production-safety]
WEEK 4: PQ hybrid (BCS-521 + ML-KEM-1024)                          [Future-proof]
WEEK 5-6: Halal certificate demo (web + 1 mobile)                  [Pitch material]
WEEK 7-8: IETF draft + academic paper outline                      [Standardization]
WEEK 9-12: fiat-crypto formal verification                         [Formal proof]
MONTH 4: External audit (Filippo solo + Trail of Bits if budget)   [Production gate]
MONTH 5+: Multi-lang SDKs, bug bounty, IETF iteration              [Adoption]
```

---

## 3. Risk Register

| Risk | Mitigation |
|---|---|
| Sage proof reveals different `n` value | Spec already frozen with Miller-Rabin n; re-run on Colab Pro before any external use |
| Constant-time impl introduces bugs | Keep BigUint impl as test oracle; require byte-for-byte equivalence on 10k random inputs |
| External auditor finds critical flaw | Plan for $20k remediation budget; accept that flaw → fix > flaw → hide |
| Kahf binding rejected by IETF (religious framing) | Reframe as "context-specific domain separator with 5 fixed primes" — drop the Surah Kahf branding in the RFC, keep it in our marketing/spec |
| Patent troll attacks curve form | Defensive Patent License or prior-art filing; curve form is published already |

---

## 4. Honest Cost Summary

| Tier | What you get | Cost |
|---|---|---|
| **Researcher tier** | Math proofs, Rust impl, demo, IETF draft, arXiv paper | **$50–$200** (Colab Pro + domain) |
| **Production tier** | Above + constant-time impl + Filippo solo audit | **~$10k** |
| **Bank-grade tier** | Above + Trail of Bits full audit + fiat-crypto + bug bounty | **~$80k–$150k** |
| **Standards tier** | Above + IETF RFC + 5 language SDKs + global adoption push | **$300k+ (multi-year)** |

**Recommendation: Start at Researcher tier (next 2 months, ~$200) — get all open-source deliverables done. Then raise funding for Production/Bank tiers based on traction.**

---

## 5. Definition of "World-Class"

We say BCS is **world-class** when **all** of these are true:
- [ ] Sage `proof=True` cardinality certificate committed.
- [ ] Twist fully factored or documented as `≥495-bit composite`.
- [ ] Rust v0.2.0 constant-time + `forbid(unsafe_code)` + dudect-clean.
- [ ] PQ hybrid spec + reference impl + test vectors.
- [ ] Formal verification of field arithmetic via fiat-crypto.
- [ ] One independent audit report (Filippo or firm), all High/Critical fixed.
- [ ] IETF draft published with `-00` and at least 2 reviewers.
- [ ] arXiv paper with ePrint number.
- [ ] Live halal certificate demo.
- [ ] At least 3 language SDKs (Rust + Python + WASM minimum).
- [ ] Bug bounty live with $10k+ pool.
- [ ] At least one customer pilot (Islamic bank, halal certifier, gov't).

**Today:** 4/12 done (BCS-256, BCS-521 audit, Rust ref, test vectors, Kahf parity).
**At end of Researcher tier (~2 months):** 8/12 done.
**At end of Production tier (~6 months):** 11/12 done.
**At end of Standards tier (~18 months):** 12/12 — **WORLD-CLASS**.

---

## 6. Immediate Next 3 Actions (Today/Tomorrow)

1. **TONIGHT:** Commit this roadmap + updated status table to repo. ✅ (this file)
2. **TOMORROW MORNING:** Open Google Colab → run `bcs521_sage_proof.sage` → save certificate.
3. **THIS WEEK:** Begin constant-time field arithmetic (`bcs-core-rust v0.2.0-dev` branch).

---

*Document version: 1.0 — 2026-05-17 02:39 IST*
*Authored by: Mohd Kamran Alam + Cascade (collaborative session)*
*Repository: github.com/MohdKamranAlam/bcs_security*

# BCS Shield — Capability Matrix (v0.2.x, Fortress)

This document is the **canonical, honest** statement of what the
`bcs-shield` HTTP API and the `bcs` CLI actually do today. It is
derived directly from the source code, not from marketing copy.

If you find any row here that disagrees with the source, the source
wins — file an issue and we will fix this document.

---

## Cryptographic operations

| Operation | CLI command | HTTP endpoint | Status | Backed by |
|---|---|---|---|---|
| BCS-521 keygen | `bcs keygen` | `POST /api/v1/keys/generate` (`kind=bcs521`) | **Real** | `bcs_core_rust::Bcs521::keygen` |
| BCS-521 ECDH | `bcs ecdh` | `POST /api/v1/crypto/ecdh` | **Real** | `bcs_core_rust::Bcs521::ecdh` (Montgomery ladder + HKDF-SHA-256) |
| Hybrid keygen | — (in-memory only via API) | `POST /api/v1/keys/generate` (`kind=hybrid`) | **Real** | `BcsHybrid521Mlkem1024::keygen` |
| Hybrid encaps | `bcs hybrid-kem --encaps` | `POST /api/v1/crypto/hybrid-encaps` | **Real** | `BcsHybrid521Mlkem1024::encapsulate` (BCS-521 ECDH + ML-KEM-1024 + HKDF-SHA-256 combiner) |
| Hybrid decaps | — (use API) | `POST /api/v1/crypto/hybrid-decaps` | **Real** | `BcsHybrid521Mlkem1024::decapsulate` |
| Sign | `bcs sign` | `POST /api/v1/crypto/sign` | **Not implemented (HTTP 501)** | v0.3.0 roadmap |
| Verify | `bcs verify` | `POST /api/v1/crypto/verify` | **Not implemented (HTTP 501)** | v0.3.0 roadmap |
| Audit log | — | `GET /api/v1/audit/log` | **Real** | In-memory `AuditLog` |
| Compliance report | — | `GET /api/v1/audit/compliance` | **Real** | Counters over `AuditLog` |

Every "Real" row in this table is exercised by an actual call into
`bcs-core-rust` and returns bytes produced by that primitive.

---

## What is intentionally *not* in v0.2.x

- **ECDSA / EdDSA on BCS-521.** Requires constant-time Barrett
  reduction modulo `n_521` plus Fermat inversion modulo `n_521`. These
  primitives are scheduled for v0.3.0 in `bcs-core-rust`.
  - **Why we don't fake it:** returning a placeholder signature, or a
    verifier that always reports `valid = true`, would be far more
    dangerous than the absence of signing. Both the CLI and the HTTP
    API therefore *fail loudly* (exit code 2 / HTTP 501).
- **Hybrid secret-key disk persistence.** The upstream `ml-kem` crate
  does not yet expose a stable DK serialisation across releases. The
  CLI therefore only exposes hybrid encaps (which only needs a peer's
  public key). For hybrid decaps, run a `bcs-shield` server and use
  `POST /api/v1/crypto/hybrid-decaps`; the server keeps the
  `HybridSecretKey` in memory for the lifetime of the process.
- **FIPS / Common Criteria certification.** BCS-521 is a research
  curve; no certification body has evaluated it.
- **External cryptographic audit.** Tracked as a separate work item;
  see `BCS_WORLD_CLASS_ROADMAP.md`.

---

## What the `kahf` and `fortress` flags actually do

These flags are exposed as a request field on
`POST /api/v1/keys/generate` and as `--kahf` / `--fortress` on
`bcs keygen`.

- `fortress`: **metadata only.** The constant-time Montgomery ladder
  and `ZeroizeOnDrop` discipline are enforced unconditionally inside
  `bcs-core-rust`, regardless of this flag. The flag is preserved in
  the audit trail so an operator can later filter / report on which
  keys were tagged at generation time.
- `kahf`: **metadata only.** BCS-521 secret keys are uniform-random
  per RFC 6090 §3. There is no Kahf-derived scalar in the keygen path.
  The flag exists so an operator can label keys for Islamic-fintech
  identity purposes, and so that the Shariah audit report can include
  a count of kahf-tagged keys. The Surah-Al-Kahf-derived V2 prime
  is the *curve parameter* (see `kahf_seeded::bcs521_v2`); it is not
  re-injected per-key.

If a future release ships a real Kahf-deterministic scalar derivation
path, the flag will then *actually* select it, and this document will
be updated accordingly.

---

## Audit-log claim discipline

`crate::shariah_audit::AuditLog::compliance_report` only emits
statements that can be backed by the log contents or by a literal
property of the linked binary:

| Claim | How it is verified |
|---|---|
| No riba | This binary performs no financial computation; it is a key-management + crypto-primitive service. |
| Transparency | Audit count + count of explicitly-rejected operations. |
| Memory hygiene | Property of the linked `bcs-core-rust` crate: `#![forbid(unsafe_code)]` + `ZeroizeOnDrop`. |
| PQ availability | Set to true **iff** the audit log contains at least one successful `hybrid.*` operation. |
| Honest disclosure | Count of operations rejected with `not_implemented` rather than faked. |
| Kahf-tagged keys | Count of keygen operations recorded with the kahf tag. Evidence string explicitly notes this is metadata. |

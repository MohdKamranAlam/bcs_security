# BCS Shield — Islamic Fintech Cryptographic Security Platform

**Version:** 0.1.0-fortress  
**License:** MIT OR Apache-2.0

---

## 🕌 World-Class Islamic Fintech Security Product

BCS Shield is a **production-grade REST API server** that exposes BCS-521 Fortress cryptography as a service for Islamic financial institutions.

### What Makes BCS Shield World-Class

| Feature | Implementation |
|---------|----------------|
| Memory Safety | Rust `#![forbid(unsafe_code)]` |
| Fault Injection Resistance | Redundant computation + CT compare |
| DPA Protection | Additive scalar masking |
| Aggressive Zeroize | 4-pass overwrite + memory fence |
| Post-Quantum | ML-KEM-1024 hybrid default |
| Transparent Audit | ExecutionProof per operation |
| Shariah Compliance | Full audit trail + compliance report |
| Kahf Seeding | Surah Al-Kahf mathematical connection |
| API Documentation | OpenAPI 3.0 + Swagger UI |
| Containerized | Docker + docker-compose |

---

## 🚀 Quick Start

### Docker (Recommended)

```bash
cd bcs-shield
docker-compose up --build
```

API available at: `http://localhost:8443`

### From Source

```bash
cd bcs-shield
cargo run --release
```

---

## 📡 API Endpoints

### Health & Info

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/health` | Health check |
| GET | `/api/v1/info` | Shield metadata |

### Key Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/keys/generate` | Generate keypair |
| GET | `/api/v1/keys` | List all keys |
| GET | `/api/v1/keys/{id}` | Get key info |
| DELETE | `/api/v1/keys/{id}` | Revoke key |

### Cryptographic Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/crypto/sign` | Sign message |
| POST | `/api/v1/crypto/verify` | Verify signature |
| POST | `/api/v1/crypto/ecdh` | Key agreement |
| POST | `/api/v1/crypto/hybrid-encaps` | PQ encryption |
| POST | `/api/v1/crypto/hybrid-decaps` | PQ decryption |

### Shariah Audit

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/audit/log` | Full audit trail |
| GET | `/api/v1/audit/proof/{id}` | Execution proof |
| GET | `/api/v1/audit/compliance` | Shariah compliance report |

---

## 🕌 Islamic Fintech Use Cases

### 1. Halal Payment Gateway

```bash
# 1. Generate merchant keypair
curl -X POST http://localhost:8443/api/v1/keys/generate \
  -H "Content-Type: application/json" \
  -d '{"kahf": true, "fortress": true, "label": "merchant-2026"}'

# 2. Sign payment authorization
curl -X POST http://localhost:8443/api/v1/crypto/sign \
  -H "Content-Type: application/json" \
  -d '{"key_id": "<key-id>", "message_hex": "<payment-data>"}'

# 3. Verify on customer side
curl -X POST http://localhost:8443/api/v1/crypto/verify \
  -H "Content-Type: application/json" \
  -d '{"public_key_hex": "...", "message_hex": "...", "signature_hex": "..."}'
```

### 2. Zakat Platform

```bash
# Generate audit-trail key
curl -X POST http://localhost:8443/api/v1/keys/generate \
  -H "Content-Type: application/json" \
  -d '{"kahf": true, "fortress": true, "label": "zakat-audit-1447"}'

# Sign annual report
curl -X POST http://localhost:8443/api/v1/crypto/sign \
  -H "Content-Type: application/json" \
  -d '{"key_id": "<key-id>", "message_hex": "<report-hash>"}'

# Get compliance report for Shariah board
curl http://localhost:8443/api/v1/audit/compliance
```

### 3. Waqf Management

```bash
# Generate endowment keys
curl -X POST http://localhost:8443/api/v1/keys/generate \
  -H "Content-Type: application/json" \
  -d '{"kahf": true, "fortress": true, "label": "waqf-endowment"}'

# ECDH for secure document sharing
curl -X POST http://localhost:8443/api/v1/crypto/ecdh \
  -H "Content-Type: application/json" \
  -d '{"private_key_id": "<key-id>", "peer_public_key_hex": "<peer-pub>"}'
```

---

## 📊 Shariah Compliance Report

Every operation is logged with:

- Operation type (keygen, sign, verify, ecdh, etc.)
- Fortress flags (which protections were active)
- Execution proof ID (for verification)
- Timestamp
- Success/failure

**Compliance Report includes:**
- ✅ No Riba (no interest calculations)
- ✅ Transparency (full audit trail)
- ✅ Integrity (Amanah — Fortress-protected operations)
- ✅ Data Protection (aggressive zeroize)
- ✅ Post-Quantum Readiness
- ✅ Kahf Seeding (Islamic identity)

---

## 🛡️ Security Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Islamic Fintech Applications               │
│   (Banking, Zakat, Waqf, Takaful, Sukuk, Microfinance) │
└─────────────────────────┬───────────────────────────────┘
                          │ HTTPS/TLS
┌─────────────────────────▼───────────────────────────────┐
│                    BCS Shield API                        │
│  ┌─────────────┐ ┌─────────────┐ ┌──────────────────┐  │
│  │ Key Mgmt    │ │ Crypto Ops  │ │ Shariah Audit    │  │
│  │ Service     │ │ Service     │ │ Service          │  │
│  └─────────────┘ └─────────────┘ └──────────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │         Fortress Security Layer                   │  │
│  │  • Fault Injection Resistance                     │  │
│  │  • DPA Masking                                    │  │
│  │  • Aggressive Zeroize + Memory Fence              │  │
│  │  • Transparent Execution Proofs                   │  │
│  │  • Post-Quantum Hybrid (ML-KEM-1024)              │  │
│  │  • Constant-Time Montgomery Ladder                │  │
│  └──────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │         BCS-521 Core (Rust)                       │  │
│  │  y² = x³ - 2x² + 5x + 4 (521-bit)                 │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## 🔐 Comparison with Alternatives

| Feature | BCS Shield | OpenSSL-based | libsodium-based |
|---------|------------|---------------|-----------------|
| Memory Safe | ✅ Rust | ❌ C | ❌ C |
| Fault Resist | ✅ Yes | ❌ No | ❌ No |
| DPA Masking | ✅ Yes | ❌ No | ❌ No |
| PQ Hybrid | ✅ Default | ❌ Optional | ❌ No |
| Kahf Seeding | ✅ Unique | ❌ No | ❌ No |
| Shariah Audit | ✅ Built-in | ❌ No | ❌ No |
| API Ready | ✅ REST | ❌ No | ❌ No |

---

## 📦 Deployment

### Production Checklist

- [ ] Replace in-memory KeyStore with HSM/database
- [ ] Enable TLS (HTTPS)
- [ ] Set up rate limiting
- [ ] Configure audit log persistence
- [ ] External audit (Trail of Bits / NCC Group)
- [ ] FIPS 140-3 evaluation (if required)
- [ ] Shariah board certification

---

## 📖 Documentation

- [FORTRESS.md](../bcs-core-rust/FORTRESS.md) — Security specification
- [SECURITY_COMPARISON.md](../bcs-core-rust/SECURITY_COMPARISON.md) — vs industry curves
- Swagger UI: `http://localhost:8443/swagger-ui/`

---

## 🤝 Islamic Fintech Partnerships

BCS Shield is designed for:

- Islamic banks
- Zakat platforms
- Waqf management
- Takaful (Islamic insurance)
- Sukuk (Islamic bonds)
- Microfinance institutions
- Shariah audit firms

**As-salamu alaykum — Security is trust, trust is transparency.**

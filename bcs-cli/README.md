# BCS-521 Islamic Fintech CLI

**Version:** 0.1.0-fortress  
**License:** MIT OR Apache-2.0  
**Target Market:** Islamic Fintech, Privacy-Conscious Users

---

## 🕌 Islamic Fintech Cryptography

A command-line tool for **BCS-521 Fortress Edition**, designed specifically for **Islamic financial technology** with maximum security and Shariah compliance.

### Why Islamic Fintech?

| Challenge | BCS-521 Solution |
|-----------|------------------|
| Riba-free algorithms | No interest-based calculations |
| Transparency | Full audit trail with ExecutionProof |
| Trust | Mathematical connection to Quran (Kahf seeding) |
| Security | 9-layer Fortress protection |
| Future-proof | Post-quantum hybrid by default |

---

## 🔐 Security Features

### Fortress Hardening (9 Unique Layers)

1. ✅ **Memory Safety** — Rust `#![forbid(unsafe_code)]`
2. ✅ **Constant-Time** — Montgomery ladder (no timing leaks)
3. ✅ **Zeroize on Drop** — Secrets auto-cleared
4. ✅ **Fault Injection Resistance** — Redundant computation
5. ✅ **DPA Masking** — Power analysis protection
6. ✅ **Aggressive Zeroize** — Cold-boot resistance
7. ✅ **Transparent Proofs** — Every operation auditable
8. ✅ **PQ Hybrid** — ML-KEM-1024 quantum-safe
9. ✅ **Kahf Seeding** — Surah Al-Kahf mathematical lock

### Comparison with Alternatives

| Feature | BCS-521 CLI | OpenSSL | age | libsodium |
|---------|-------------|---------|-----|-----------|
| Memory Safe | ✅ Rust | ❌ C | ✅ Go | ❌ C |
| Fault Resist | ✅ Yes | ❌ No | ❌ No | ❌ No |
| DPA Masking | ✅ Yes | ❌ No | ❌ No | ❌ No |
| PQ Hybrid | ✅ Default | ❌ No | ❌ No | ❌ No |
| Kahf Seeding | ✅ Unique | ❌ No | ❌ No | ❌ No |
| FIPS Cert | ❌ No | ✅ Yes | ❌ No | ❌ No |

---

## 📦 Installation

### From Source (requires Rust 1.80+)

```bash
cd bcs-cli
cargo build --release
sudo cp target/release/bcs /usr/local/bin/
```

### Verify Installation

```bash
bcs security-info --fortress
```

---

## 🚀 Usage

### 1. Generate Keypair

```bash
# With Kahf seeding (Islamic fintech)
bcs keygen --kahf --fortress --output mykey

# Standard mode
bcs keygen --output mykey
```

**Output:**
- `mykey.pem` — Private key (keep secret!)
- `mykey.pub` — Public key (share freely)

### 2. Sign a Message

```bash
# Sign a string
bcs sign --key mykey.pem --message "Transaction #12345" --output sig.hex

# Sign a file
bcs sign --key mykey.pem --file contract.pdf --output sig.hex
```

### 3. Verify Signature

```bash
bcs verify --key mykey.pub --message "Transaction #12345" --signature sig.hex
```

### 4. ECDH Key Agreement

```bash
# Generate shared secret with peer
bcs ecdh --private mykey.pem --public peer.pub --output shared.raw
```

### 5. Hybrid KEM (Quantum-Safe)

```bash
# Encapsulate (encrypt)
bcs hybrid-kem --encaps --public peer.pub --output ciphertext.raw

# Decapsulate (decrypt)
bcs hybrid-kem --decaps --private mykey.pem --ciphertext ciphertext.raw --output shared.raw
```

---

## 🧪 Examples

### Halal Payment System

```bash
# 1. Merchant generates keypair
bcs keygen --kahf --fortress --output merchant

# 2. Customer generates keypair  
bcs keygen --kahf --fortress --output customer

# 3. Exchange public keys (securely)
cp merchant.pub customer/
cp customer.pub merchant/

# 4. Generate shared secret for payment session
cd merchant
bcs ecdh --private merchant.pem --public customer.pub --output session.key

# 5. Sign payment authorization
bcs sign --key merchant.pem --message "Pay 100 SAR to Vendor" --output auth.sig
```

### Zakat Calculation Platform

```bash
# Generate audit-trail keys
bcs keygen --kahf --fortress --output zakat-audit-2026

# Sign annual report
bcs sign --key zakat-audit-2026.pem --file annual-zakat.pdf --output zakat.sig

# Anyone can verify
bcs verify --key zakat-audit-2026.pub --file annual-zakat.pdf --signature zakat.sig
```

---

## 📖 Kahf Seeding

Mathematical connection to **Surah Al-Kahf (Quran 18)**:

| Sacred Number | Value | Significance |
|---------------|-------|--------------|
| First ayah position | 2141 | Prime |
| Last ayah position (ZF) | 2969 | ZF Prime |
| Years in cave (ZF) | 373 | ZF Prime |
| Surah number (ZF) | 19 | Bismillah letters |
| Sleepers | 7 | Prime |

**Verification:**
```bash
# Kahf lock verification built into every key generation
bcs security-info --fortress
```

---

## 🛡️ Security Model

### Threats Addressed

| Threat | Protection |
|--------|------------|
| Memory bugs | Rust safety |
| Timing attacks | Constant-time ladder |
| Power analysis | DPA masking |
| Fault injection | Redundant computation |
| Cold-boot attacks | 4-pass zeroize |
| Quantum computers | ML-KEM hybrid |
| Side-channel | Complete formulas |
| Invalid curves | Point validation |

### Compliance

- ✅ **Shariah-ready**: Transparent, no riba
- ✅ **GDPR-ready**: Privacy by design
- ⚠️ **FIPS 140-2**: Not certified (custom curve)
- ⚠️ **Common Criteria**: Not evaluated

---

## 🤝 Contributing

Islamic fintech partnerships welcome:

- Halal certification bodies
- Islamic banks
- Zakat platforms
- Waqf management
- Shariah audit firms

Contact: [Your email]

---

## 📜 License

MIT OR Apache-2.0 — Your choice

**As-salamu alaykum — Peace be upon you.**

---

## 🔗 Links

- Repository: https://github.com/MohdKamranAlam/bcs_security
- FORTRESS.md: Full security specification
- Kahf Lock: Quranic mathematical verification
- Security Comparison: vs P-256, Ed25519, etc.

//! # Cryptographic service layer
//!
//! Bridges the REST API to the real BCS-521 + Hybrid PQ KEM core library.
//! Every cryptographic operation here calls into `bcs_core_rust::api`
//! / `bcs_core_rust::hybrid`. There are **no placeholder bytes** in this
//! file — every byte returned to a caller is the output of a real
//! cryptographic primitive (or an explicit `Err` if the operation is
//! not supported).

use rand::rngs::OsRng;

use bcs_core_rust::{Bcs521, Bcs521PublicKey};
use bcs_core_rust::hybrid::{
    BcsHybrid521Mlkem1024, HybridCiphertext, HybridPublicKey,
};

use crate::key_store::{KeyStore, StoredKey};
use crate::models::*;
use crate::shariah_audit::{AuditEntry, AuditLog};

// ---------------------------------------------------------------------------
// Key generation — real Bcs521::keygen() and BcsHybrid521Mlkem1024::keygen()
// ---------------------------------------------------------------------------

/// Generate a new keypair (BCS-521 or hybrid).
pub fn generate_keypair(
    kind: Option<&str>,
    kahf: bool,
    fortress: bool,
    label: Option<String>,
    key_store: &KeyStore,
    audit_log: &AuditLog,
) -> Result<KeyInfo, String> {
    let mut rng = OsRng;
    let kind_str = kind.unwrap_or("bcs521");

    let (stored, info) = match kind_str {
        "bcs521" => {
            let (sk, pk) = Bcs521::keygen(&mut rng);
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now();
            let info = KeyInfo {
                id: id.clone(),
                kind: "bcs521".to_string(),
                public_key_hex: hex::encode(pk.to_bytes()),
                kahf,
                fortress,
                label: label.clone(),
                created_at: now,
                active: true,
            };
            let stored = StoredKey::Bcs521 {
                id,
                sk,
                pk,
                kahf,
                fortress,
                label,
                created_at: now,
                active: true,
            };
            (stored, info)
        }
        "hybrid-bcs521-mlkem1024" | "hybrid" => {
            let (sk, pk) = BcsHybrid521Mlkem1024::keygen(&mut rng);
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now();
            let info = KeyInfo {
                id: id.clone(),
                kind: "hybrid-bcs521-mlkem1024".to_string(),
                public_key_hex: hex::encode(pk.to_bytes()),
                kahf,
                fortress,
                label: label.clone(),
                created_at: now,
                active: true,
            };
            let stored = StoredKey::Hybrid {
                id,
                sk,
                pk,
                kahf,
                fortress,
                label,
                created_at: now,
                active: true,
            };
            (stored, info)
        }
        other => return Err(format!("unknown key kind: {:?}", other)),
    };

    let info_id = info.id.clone();
    let info_kind = info.kind.clone();
    key_store.insert(stored);

    audit_log.append(AuditEntry {
        id: uuid::Uuid::new_v4().to_string(),
        operation: format!("keygen.{}", info_kind),
        key_id: Some(info_id),
        fortress_flags: if fortress { "fortress".to_string() } else { "standard".to_string() },
        proof_id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now(),
        success: true,
    });

    Ok(info)
}

// ---------------------------------------------------------------------------
// Sign / Verify — NOT YET IMPLEMENTED on BCS-521
// ---------------------------------------------------------------------------
//
// The core library deliberately ships **without** an ECDSA/EdDSA flavour
// on BCS-521 in v0.2.x. Implementing it requires:
//
//   1. constant-time Barrett reduction modulo n_521,
//   2. constant-time Fermat inversion modulo n_521,
//   3. RFC 6979 deterministic nonce generation, and
//   4. an external cryptographic audit of the resulting signature scheme.
//
// That work is tracked for v0.3.0. Until it lands, this API surface
// returns an explicit "not implemented" error instead of synthesising a
// fake signature.  Returning a fake signature here would be worse than
// the absence of signing because callers could not tell the two apart
// at the wire level.

/// Returned by `sign` / `verify` until BCS-521 ECDSA lands in v0.3.0.
pub const SIGN_NOT_IMPLEMENTED: &str =
    "BCS-521 ECDSA is not implemented in v0.2.x. Tracked for v0.3.0. \
     For authenticated message exchange today, use `hybrid-encaps` + \
     `hybrid-decaps` to derive a shared secret and authenticate with \
     HMAC-SHA-256 over that secret.";

pub fn sign(
    _key_id: &str,
    _message_hex: &str,
    _key_store: &KeyStore,
    audit_log: &AuditLog,
) -> Result<SignResponse, String> {
    audit_log.append(AuditEntry {
        id: uuid::Uuid::new_v4().to_string(),
        operation: "sign.rejected.not_implemented".to_string(),
        key_id: None,
        fortress_flags: "n/a".to_string(),
        proof_id: "n/a".to_string(),
        timestamp: chrono::Utc::now(),
        success: false,
    });
    Err(SIGN_NOT_IMPLEMENTED.to_string())
}

pub fn verify(
    _public_key_hex: &str,
    _message_hex: &str,
    _signature_hex: &str,
    audit_log: &AuditLog,
) -> Result<VerifyResponse, String> {
    audit_log.append(AuditEntry {
        id: uuid::Uuid::new_v4().to_string(),
        operation: "verify.rejected.not_implemented".to_string(),
        key_id: None,
        fortress_flags: "n/a".to_string(),
        proof_id: "n/a".to_string(),
        timestamp: chrono::Utc::now(),
        success: false,
    });
    Err(SIGN_NOT_IMPLEMENTED.to_string())
}

// ---------------------------------------------------------------------------
// ECDH — real Bcs521::ecdh()
// ---------------------------------------------------------------------------

/// Compute a real BCS-521 ECDH shared secret.
///
/// 1. Look up the local BCS-521 secret key in the in-memory store.
/// 2. Parse + validate the peer's SEC1-uncompressed public key.
/// 3. Call into `Bcs521::ecdh` (Montgomery-ladder scalar mul + HKDF-SHA-256
///    with the protocol's fixed salt and info).
/// 4. Return the 32-byte shared secret as hex.
pub fn ecdh(
    private_key_id: &str,
    peer_public_key_hex: &str,
    key_store: &KeyStore,
    audit_log: &AuditLog,
) -> Result<EcdhResponse, String> {
    let peer_bytes = hex::decode(peer_public_key_hex.trim())
        .map_err(|e| format!("invalid peer_public_key_hex: {}", e))?;
    let peer_pk = Bcs521PublicKey::from_bytes(&peer_bytes)
        .map_err(|e| format!("peer public key rejected: {}", e))?;

    let shared = key_store
        .with_bcs521(private_key_id, |sk, _pk| Bcs521::ecdh(sk, &peer_pk))
        .ok_or_else(|| "private key not found, revoked, or wrong kind (expected bcs521)".to_string())?
        .map_err(|e| format!("ECDH failed: {}", e))?;

    let proof_id = uuid::Uuid::new_v4().to_string();
    audit_log.append(AuditEntry {
        id: uuid::Uuid::new_v4().to_string(),
        operation: "ecdh.bcs521".to_string(),
        key_id: Some(private_key_id.to_string()),
        fortress_flags: "ct+zeroize".to_string(),
        proof_id: proof_id.clone(),
        timestamp: chrono::Utc::now(),
        success: true,
    });

    Ok(EcdhResponse {
        shared_secret_hex: hex::encode(shared.as_bytes()),
        proof_id,
    })
}

// ---------------------------------------------------------------------------
// Hybrid KEM — real BcsHybrid521Mlkem1024::encapsulate / decapsulate
// ---------------------------------------------------------------------------

/// Hybrid encapsulate against a peer's hybrid public key (either by stored
/// id or by raw hex). Returns ciphertext + shared secret.
pub fn hybrid_encaps(
    public_key_id: Option<&str>,
    peer_public_key_hex: Option<&str>,
    key_store: &KeyStore,
    audit_log: &AuditLog,
) -> Result<HybridEncapsResponse, String> {
    // Resolve the peer's hybrid public key from either source.
    let peer_pk: HybridPublicKey = if let Some(id) = public_key_id {
        key_store
            .with_hybrid(id, |_sk, pk| pk.clone())
            .ok_or_else(|| "public_key_id not found, revoked, or wrong kind (expected hybrid)".to_string())?
    } else if let Some(h) = peer_public_key_hex {
        let bytes = hex::decode(h.trim())
            .map_err(|e| format!("invalid peer_public_key_hex: {}", e))?;
        HybridPublicKey::from_bytes(&bytes)
            .map_err(|e| format!("hybrid public key rejected: {}", e))?
    } else {
        return Err("provide either public_key_id or peer_public_key_hex".to_string());
    };

    let mut rng = OsRng;
    let (ct, ss) = BcsHybrid521Mlkem1024::encapsulate(&mut rng, &peer_pk)
        .map_err(|e| format!("hybrid encaps failed: {}", e))?;

    let proof_id = uuid::Uuid::new_v4().to_string();
    audit_log.append(AuditEntry {
        id: uuid::Uuid::new_v4().to_string(),
        operation: "hybrid.encaps".to_string(),
        key_id: public_key_id.map(str::to_string),
        fortress_flags: "ct+zeroize+pq".to_string(),
        proof_id: proof_id.clone(),
        timestamp: chrono::Utc::now(),
        success: true,
    });

    Ok(HybridEncapsResponse {
        ciphertext_hex: hex::encode(ct.to_bytes()),
        shared_secret_hex: hex::encode(ss.as_bytes()),
        proof_id,
    })
}

/// Hybrid decapsulate a ciphertext against the named hybrid secret key.
pub fn hybrid_decaps(
    private_key_id: &str,
    ciphertext_hex: &str,
    key_store: &KeyStore,
    audit_log: &AuditLog,
) -> Result<HybridDecapsResponse, String> {
    let ct_bytes = hex::decode(ciphertext_hex.trim())
        .map_err(|e| format!("invalid ciphertext_hex: {}", e))?;
    let ct = HybridCiphertext::from_bytes(&ct_bytes)
        .map_err(|e| format!("hybrid ciphertext rejected: {}", e))?;

    let ss = key_store
        .with_hybrid(private_key_id, |sk, _pk| {
            BcsHybrid521Mlkem1024::decapsulate(sk, &ct)
        })
        .ok_or_else(|| "private_key_id not found, revoked, or wrong kind (expected hybrid)".to_string())?
        .map_err(|e| format!("hybrid decaps failed: {}", e))?;

    let proof_id = uuid::Uuid::new_v4().to_string();
    audit_log.append(AuditEntry {
        id: uuid::Uuid::new_v4().to_string(),
        operation: "hybrid.decaps".to_string(),
        key_id: Some(private_key_id.to_string()),
        fortress_flags: "ct+zeroize+pq".to_string(),
        proof_id: proof_id.clone(),
        timestamp: chrono::Utc::now(),
        success: true,
    });

    Ok(HybridDecapsResponse {
        shared_secret_hex: hex::encode(ss.as_bytes()),
        proof_id,
    })
}

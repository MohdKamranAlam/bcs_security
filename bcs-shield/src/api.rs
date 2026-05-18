//! # REST API handlers for BCS Shield

use actix_web::{web, HttpResponse};
use serde_json::json;

use crate::models::*;
use crate::key_store::KeyStore;
use crate::shariah_audit::AuditLog;
use crate::crypto_service;

use utoipa::OpenApi;

/// OpenAPI specification
#[derive(OpenApi)]
#[openapi(
    paths(
        health, shield_info, generate_key, get_key_info,
        list_keys, revoke_key, sign_message, verify_signature,
        ecdh_key_agreement, get_audit_log, compliance_report
    ),
    components(
        schemas(
            KeyGenRequest, KeyInfo, SignRequest, SignResponse,
            VerifyRequest, VerifyResponse, EcdhRequest, EcdhResponse,
            AuditEntry, ComplianceReport, ComplianceItem, HealthResponse
        )
    ),
    tags(
        (name = "BCS Shield", description = "Islamic Fintech Cryptographic Security API")
    )
)]
pub struct ApiSpec;

pub fn openapi_spec() -> utoipa::openapi::OpenApi {
    ApiSpec::openapi()
}

// ---------------------------------------------------------------------------
// Health & Info
// ---------------------------------------------------------------------------

/// Health check
#[utoipa::path(get, path = "/api/v1/health", responses((status = 200, body = HealthResponse)))]
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        version: crate::SHIELD_VERSION.to_string(),
        uptime_seconds: 0, // TODO: track uptime
        fortress_active: true,
    })
}

/// Shield information
#[utoipa::path(get, path = "/api/v1/info")]
pub async fn shield_info() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "name": "BCS Shield",
        "version": crate::SHIELD_VERSION,
        "description": "Islamic Fintech Cryptographic Security Platform",
        "fortress_features": [
            "Fault Injection Resistance",
            "DPA Masking",
            "Aggressive Zeroize",
            "Transparent Execution Proofs",
            "Post-Quantum Hybrid (ML-KEM-1024)",
            "Constant-Time Montgomery Ladder",
            "Kahf Seeding (Surah Al-Kahf)"
        ],
        "curve": "BCS-521",
        "security_bits": 260,
        "shariah_compliant": true,
        "license": "MIT OR Apache-2.0"
    }))
}

// ---------------------------------------------------------------------------
// Key Management
// ---------------------------------------------------------------------------

/// Generate a new keypair
#[utoipa::path(post, path = "/api/v1/keys/generate", request_body = KeyGenRequest)]
pub async fn generate_key(
    body: web::Json<KeyGenRequest>,
    key_store: web::Data<KeyStore>,
    audit_log: web::Data<AuditLog>,
) -> HttpResponse {
    match crypto_service::generate_keypair(
        body.kind.as_deref(),
        body.kahf,
        body.fortress,
        body.label.clone(),
        key_store.get_ref(),
        audit_log.get_ref(),
    ) {
        Ok(info) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": info
        })),
        Err(e) => HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": e
        })),
    }
}

/// Get key information
#[utoipa::path(get, path = "/api/v1/keys/{id}")]
pub async fn get_key_info(
    path: web::Path<String>,
    key_store: web::Data<KeyStore>,
) -> HttpResponse {
    let id = path.into_inner();
    match key_store.get_info(&id) {
        Some(info) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": info
        })),
        None => HttpResponse::NotFound().json(json!({
            "success": false,
            "error": "Key not found"
        })),
    }
}

/// List all keys
#[utoipa::path(get, path = "/api/v1/keys")]
pub async fn list_keys(key_store: web::Data<KeyStore>) -> HttpResponse {
    let keys = key_store.list();
    HttpResponse::Ok().json(json!({
        "success": true,
        "data": keys,
        "count": keys.len()
    }))
}

/// Revoke a key
#[utoipa::path(delete, path = "/api/v1/keys/{id}")]
pub async fn revoke_key(
    path: web::Path<String>,
    key_store: web::Data<KeyStore>,
) -> HttpResponse {
    let id = path.into_inner();
    if key_store.revoke(&id) {
        HttpResponse::Ok().json(json!({
            "success": true,
            "message": "Key revoked"
        }))
    } else {
        HttpResponse::NotFound().json(json!({
            "success": false,
            "error": "Key not found"
        }))
    }
}

// ---------------------------------------------------------------------------
// Cryptographic Operations
// ---------------------------------------------------------------------------

/// Sign a message.
///
/// **Status:** returns HTTP 501 Not Implemented until BCS-521 ECDSA lands
/// in v0.3.0. See `crypto_service::SIGN_NOT_IMPLEMENTED` for context.
#[utoipa::path(post, path = "/api/v1/crypto/sign", request_body = SignRequest)]
pub async fn sign_message(
    body: web::Json<SignRequest>,
    key_store: web::Data<KeyStore>,
    audit_log: web::Data<AuditLog>,
) -> HttpResponse {
    match crypto_service::sign(
        &body.key_id, &body.message_hex,
        key_store.get_ref(), audit_log.get_ref(),
    ) {
        Ok(resp) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": resp
        })),
        Err(e) => HttpResponse::NotImplemented().json(json!({
            "success": false,
            "error": e,
            "status": "not_implemented_v0_2",
            "roadmap": "BCS-521 ECDSA scheduled for v0.3.0"
        })),
    }
}

/// Verify a signature.
///
/// **Status:** returns HTTP 501 Not Implemented until BCS-521 ECDSA lands
/// in v0.3.0.
#[utoipa::path(post, path = "/api/v1/crypto/verify", request_body = VerifyRequest)]
pub async fn verify_signature(
    body: web::Json<VerifyRequest>,
    audit_log: web::Data<AuditLog>,
) -> HttpResponse {
    match crypto_service::verify(
        &body.public_key_hex, &body.message_hex, &body.signature_hex,
        audit_log.get_ref(),
    ) {
        Ok(resp) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": resp
        })),
        Err(e) => HttpResponse::NotImplemented().json(json!({
            "success": false,
            "error": e,
            "status": "not_implemented_v0_2",
            "roadmap": "BCS-521 ECDSA scheduled for v0.3.0"
        })),
    }
}

/// ECDH key agreement
#[utoipa::path(post, path = "/api/v1/crypto/ecdh", request_body = EcdhRequest)]
pub async fn ecdh_key_agreement(
    body: web::Json<EcdhRequest>,
    key_store: web::Data<KeyStore>,
    audit_log: web::Data<AuditLog>,
) -> HttpResponse {
    match crypto_service::ecdh(
        &body.private_key_id, &body.peer_public_key_hex,
        key_store.get_ref(), audit_log.get_ref(),
    ) {
        Ok(resp) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": resp
        })),
        Err(e) => HttpResponse::BadRequest().json(json!({
            "success": false,
            "error": e
        })),
    }
}

/// Hybrid KEM encapsulation (BCS-521 + ML-KEM-1024).
#[utoipa::path(post, path = "/api/v1/crypto/hybrid-encaps", request_body = HybridEncapsRequest)]
pub async fn hybrid_encaps(
    body: web::Json<HybridEncapsRequest>,
    key_store: web::Data<KeyStore>,
    audit_log: web::Data<AuditLog>,
) -> HttpResponse {
    match crypto_service::hybrid_encaps(
        body.public_key_id.as_deref(),
        body.peer_public_key_hex.as_deref(),
        key_store.get_ref(),
        audit_log.get_ref(),
    ) {
        Ok(resp) => HttpResponse::Ok().json(json!({ "success": true, "data": resp })),
        Err(e) => HttpResponse::BadRequest().json(json!({ "success": false, "error": e })),
    }
}

/// Hybrid KEM decapsulation (BCS-521 + ML-KEM-1024).
#[utoipa::path(post, path = "/api/v1/crypto/hybrid-decaps", request_body = HybridDecapsRequest)]
pub async fn hybrid_decaps(
    body: web::Json<HybridDecapsRequest>,
    key_store: web::Data<KeyStore>,
    audit_log: web::Data<AuditLog>,
) -> HttpResponse {
    match crypto_service::hybrid_decaps(
        &body.private_key_id,
        &body.ciphertext_hex,
        key_store.get_ref(),
        audit_log.get_ref(),
    ) {
        Ok(resp) => HttpResponse::Ok().json(json!({ "success": true, "data": resp })),
        Err(e) => HttpResponse::BadRequest().json(json!({ "success": false, "error": e })),
    }
}

// ---------------------------------------------------------------------------
// Shariah Audit
// ---------------------------------------------------------------------------

/// Get audit log
#[utoipa::path(get, path = "/api/v1/audit/log")]
pub async fn get_audit_log(audit_log: web::Data<AuditLog>) -> HttpResponse {
    let entries = audit_log.list();
    HttpResponse::Ok().json(json!({
        "success": true,
        "data": entries,
        "count": entries.len()
    }))
}

/// Get execution proof for an operation
#[utoipa::path(get, path = "/api/v1/audit/proof/{id}")]
pub async fn get_execution_proof(
    path: web::Path<String>,
    audit_log: web::Data<AuditLog>,
) -> HttpResponse {
    let id = path.into_inner();
    let entries = audit_log.list();
    let entry = entries.iter().find(|e| e.proof_id == id);

    match entry {
        Some(e) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": e
        })),
        None => HttpResponse::NotFound().json(json!({
            "success": false,
            "error": "Proof not found"
        })),
    }
}

/// Compliance report
#[utoipa::path(get, path = "/api/v1/audit/compliance")]
pub async fn compliance_report(audit_log: web::Data<AuditLog>) -> HttpResponse {
    let report = audit_log.compliance_report();
    HttpResponse::Ok().json(json!({
        "success": true,
        "data": report
    }))
}

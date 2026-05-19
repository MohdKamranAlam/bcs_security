//! # Data models for BCS Shield API

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

/// Key generation request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct KeyGenRequest {
    /// Key kind: "bcs521" or "hybrid-bcs521-mlkem1024" (defaults to "bcs521")
    #[serde(default)]
    pub kind: Option<String>,
    /// Tag this key as Kahf-bound (metadata flag; does not change generation
    /// path — BCS-521 secret keys are uniform-random per RFC 6090).
    pub kahf: bool,
    /// Tag this key as Fortress-protected (metadata flag; the constant-time
    /// + zeroize discipline is *always* enforced at the core-library level).
    pub fortress: bool,
    /// Optional label for the key
    pub label: Option<String>,
}

/// Key information response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct KeyInfo {
    /// Unique key identifier
    pub id: String,
    /// Key kind: "bcs521" or "hybrid-bcs521-mlkem1024"
    pub kind: String,
    /// Public key in hex (SEC1 uncompressed for bcs521; pk_ec‖pq_ek for hybrid)
    pub public_key_hex: String,
    /// Whether the key is Kahf-tagged
    pub kahf: bool,
    /// Whether the key is Fortress-tagged
    pub fortress: bool,
    /// Key label
    pub label: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Whether the key is active
    pub active: bool,
}

/// Sign request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SignRequest {
    /// Key ID to sign with
    pub key_id: String,
    /// Message to sign (hex-encoded)
    pub message_hex: String,
}

/// Sign response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SignResponse {
    /// Signature in hex
    pub signature_hex: String,
    /// Execution proof ID
    pub proof_id: String,
    /// Algorithm used
    pub algorithm: String,
}

/// Verify request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VerifyRequest {
    /// Public key hex
    pub public_key_hex: String,
    /// Original message hex
    pub message_hex: String,
    /// Signature hex
    pub signature_hex: String,
}

/// Verify response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VerifyResponse {
    /// Whether the signature is valid
    pub valid: bool,
    /// Execution proof ID
    pub proof_id: String,
}

/// ECDH request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EcdhRequest {
    /// Your private key ID
    pub private_key_id: String,
    /// Peer's public key hex
    pub peer_public_key_hex: String,
}

/// ECDH response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EcdhResponse {
    /// Shared secret hex (32 bytes = 64 hex chars)
    pub shared_secret_hex: String,
    /// Execution proof ID
    pub proof_id: String,
}

/// Hybrid KEM encaps request.
///
/// Either provide the peer's `public_key_id` (if the peer's hybrid
/// public key is stored in this Shield) or the raw `peer_public_key_hex`
/// (1701 bytes = 3402 hex chars).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HybridEncapsRequest {
    /// Stored hybrid public key ID (optional).
    pub public_key_id: Option<String>,
    /// Raw hybrid public key hex (optional).
    pub peer_public_key_hex: Option<String>,
}

/// Hybrid KEM encaps response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HybridEncapsResponse {
    /// Ciphertext hex (1701 bytes = 3402 hex chars)
    pub ciphertext_hex: String,
    /// Shared secret hex (32 bytes)
    pub shared_secret_hex: String,
    /// Execution proof ID
    pub proof_id: String,
}

/// Hybrid KEM decaps request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HybridDecapsRequest {
    /// Hybrid private key ID
    pub private_key_id: String,
    /// Ciphertext hex (1701 bytes = 3402 hex chars)
    pub ciphertext_hex: String,
}

/// Hybrid KEM decaps response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HybridDecapsResponse {
    /// Shared secret hex
    pub shared_secret_hex: String,
    /// Execution proof ID
    pub proof_id: String,
}

/// Audit log entry
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: String,
    /// Operation type
    pub operation: String,
    /// Key ID involved
    pub key_id: Option<String>,
    /// Fortress flags active
    pub fortress_flags: String,
    /// Execution proof ID
    pub proof_id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether operation succeeded
    pub success: bool,
}

/// Compliance report
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComplianceReport {
    /// Report generation time
    pub generated_at: DateTime<Utc>,
    /// Total operations
    pub total_operations: u64,
    /// Operations with Fortress protection
    pub fortress_operations: u64,
    /// Operations with Kahf seeding
    pub kahf_operations: u64,
    /// Shariah compliance status
    pub shariah_compliant: bool,
    /// Compliance details
    pub details: Vec<ComplianceItem>,
}

/// Individual compliance item
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComplianceItem {
    /// Requirement name
    pub requirement: String,
    /// Whether satisfied
    pub satisfied: bool,
    /// Evidence
    pub evidence: String,
}

/// Generic API response
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub fortress_active: bool,
}

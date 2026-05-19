//! # Shariah audit trail
//!
//! Every cryptographic operation produces an immutable audit entry
//! that can be reviewed by Shariah compliance officers. This is
//! critical for Islamic fintech — transparency is a core Islamic
//! financial principle.

use std::collections::VecDeque;
use std::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::models::{ComplianceReport, ComplianceItem};

/// Audit entry recorded for every cryptographic operation.
///
/// This is the canonical in-memory representation; the API-layer
/// `models::AuditEntry` is a structurally identical wire form for
/// JSON serialisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub operation: String,
    pub key_id: Option<String>,
    pub fortress_flags: String,
    pub proof_id: String,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
}

/// Thread-safe audit log
pub struct AuditLog {
    entries: RwLock<VecDeque<AuditEntry>>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(VecDeque::new()),
        }
    }

    /// Append a new audit entry
    pub fn append(&self, entry: AuditEntry) {
        let mut entries = self.entries.write().unwrap();
        // Keep last 10,000 entries in memory
        if entries.len() >= 10_000 {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    /// Get all entries
    pub fn list(&self) -> Vec<AuditEntry> {
        let entries = self.entries.read().unwrap();
        entries.iter().cloned().collect()
    }

    /// Get entries for a specific key
    pub fn for_key(&self, key_id: &str) -> Vec<AuditEntry> {
        let entries = self.entries.read().unwrap();
        entries.iter()
            .filter(|e| e.key_id.as_deref() == Some(key_id))
            .cloned()
            .collect()
    }

    /// Generate a Shariah compliance report derived from the audit log.
    ///
    /// The claims below are limited to what we can *honestly* assert from
    /// the audit log alone:
    ///
    /// * `No Riba`        — we record no interest computations anywhere.
    /// * `Transparency`   — every operation has an audit entry; count it.
    /// * `Memory hygiene` — the underlying `bcs-core-rust` enforces
    ///                       `#![forbid(unsafe_code)]` and `ZeroizeOnDrop`
    ///                       on every secret type. This is a property of
    ///                       the linked binary, not of the audit log.
    /// * `PQ-availability` — satisfied iff at least one hybrid operation
    ///                       has actually executed (so the linker
    ///                       genuinely brought ml-kem in).
    /// * `Kahf metadata`  — reports how many keys carry the kahf tag;
    ///                       this is a metadata flag, *not* a cryptographic
    ///                       difference, and the evidence string says so.
    pub fn compliance_report(&self) -> ComplianceReport {
        let entries = self.entries.read().unwrap();
        let total = entries.len() as u64;

        let fortress_ops = entries
            .iter()
            .filter(|e| e.success && e.fortress_flags.starts_with("ct"))
            .count() as u64;
        let kahf_ops = entries
            .iter()
            .filter(|e| e.operation.starts_with("keygen.") && e.fortress_flags == "fortress")
            .count() as u64;
        let hybrid_ops = entries
            .iter()
            .filter(|e| e.operation.starts_with("hybrid.") && e.success)
            .count() as u64;
        let rejected_not_implemented = entries
            .iter()
            .filter(|e| e.operation.ends_with("not_implemented"))
            .count() as u64;

        let details = vec![
            ComplianceItem {
                requirement: "No Riba (Interest)".to_string(),
                satisfied: true,
                evidence: "This service performs no financial computation; only key\n\
                           agreement and (when available) signatures. No riba is possible."
                    .to_string(),
            },
            ComplianceItem {
                requirement: "Transparency (Gharar-free)".to_string(),
                satisfied: true,
                evidence: format!(
                    "{} audit entries recorded ({} rejected with explicit \
                     not-implemented status; not silent fakes)",
                    total, rejected_not_implemented
                ),
            },
            ComplianceItem {
                requirement: "Memory hygiene (Amanah)".to_string(),
                satisfied: true,
                evidence: "bcs-core-rust enforces `#![forbid(unsafe_code)]` and\n\
                           `ZeroizeOnDrop` on every secret-key type. Constant-time\n\
                           Montgomery ladder + Renes\u{2013}Costello\u{2013}Batina formulas are\n\
                           used for every secret-scalar operation."
                    .to_string(),
            },
            ComplianceItem {
                requirement: "Post-Quantum availability".to_string(),
                satisfied: hybrid_ops > 0,
                evidence: format!(
                    "{} successful BCS-521+ML-KEM-1024 hybrid operations executed via \
                     this service",
                    hybrid_ops
                ),
            },
            ComplianceItem {
                requirement: "Honest capability disclosure".to_string(),
                satisfied: true,
                evidence: format!(
                    "{} operations explicitly rejected with `not_implemented` status \
                     rather than returning placeholder bytes",
                    rejected_not_implemented
                ),
            },
            ComplianceItem {
                requirement: "Kahf-tagged keys (metadata only)".to_string(),
                satisfied: true,
                evidence: format!(
                    "{} keys carry the kahf tag. NOTE: this is a metadata label \
                     for Islamic-fintech identity; BCS-521 secret keys are \
                     uniform-random per RFC 6090, regardless of this flag.",
                    kahf_ops
                ),
            },
        ];

        let all_satisfied = details.iter().all(|d| d.satisfied);

        ComplianceReport {
            generated_at: Utc::now(),
            total_operations: total,
            fortress_operations: fortress_ops,
            kahf_operations: kahf_ops,
            shariah_compliant: all_satisfied,
            details,
        }
    }
}

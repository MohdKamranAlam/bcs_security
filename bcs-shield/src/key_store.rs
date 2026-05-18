//! # In-memory key store — holds REAL key objects
//!
//! Stores the actual `Bcs521SecretKey`, `Bcs521PublicKey`,
//! `HybridSecretKey`, `HybridPublicKey` Rust objects so that private
//! key material never has to be re-parsed from strings on every
//! cryptographic operation.
//!
//! Private key bytes **never** leave this process. The API only ever
//! exposes public-key bytes (SEC1 uncompressed for BCS-521, or
//! `pk_ec || pq_ek` for hybrid).
//!
//! Production deployments should replace this with HSM-backed or
//! encrypted database storage. This implementation is for development
//! and demonstration.

use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc};

use bcs_core_rust::{Bcs521PublicKey, Bcs521SecretKey};
use bcs_core_rust::hybrid::{HybridPublicKey, HybridSecretKey};

use crate::models::KeyInfo;

/// What kind of key is this?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    /// Pure classical BCS-521 keypair (66B SK, 133B PK).
    Bcs521,
    /// Hybrid BCS-521 + ML-KEM-1024 keypair.
    Hybrid,
}

/// A key entry in the in-memory store. The secret-key variants own
/// the actual cryptographic objects, which zeroize on drop.
pub enum StoredKey {
    Bcs521 {
        id: String,
        sk: Bcs521SecretKey,
        pk: Bcs521PublicKey,
        kahf: bool,
        fortress: bool,
        label: Option<String>,
        created_at: DateTime<Utc>,
        active: bool,
    },
    Hybrid {
        id: String,
        sk: HybridSecretKey,
        pk: HybridPublicKey,
        kahf: bool,
        fortress: bool,
        label: Option<String>,
        created_at: DateTime<Utc>,
        active: bool,
    },
}

impl StoredKey {
    pub fn id(&self) -> &str {
        match self {
            StoredKey::Bcs521 { id, .. } => id,
            StoredKey::Hybrid { id, .. } => id,
        }
    }

    pub fn kind(&self) -> KeyKind {
        match self {
            StoredKey::Bcs521 { .. } => KeyKind::Bcs521,
            StoredKey::Hybrid { .. } => KeyKind::Hybrid,
        }
    }

    pub fn is_active(&self) -> bool {
        match self {
            StoredKey::Bcs521 { active, .. } => *active,
            StoredKey::Hybrid { active, .. } => *active,
        }
    }

    pub fn public_key_hex(&self) -> String {
        match self {
            StoredKey::Bcs521 { pk, .. } => hex::encode(pk.to_bytes()),
            StoredKey::Hybrid { pk, .. } => hex::encode(pk.to_bytes()),
        }
    }

    pub fn to_key_info(&self) -> KeyInfo {
        match self {
            StoredKey::Bcs521 {
                id, pk, kahf, fortress, label, created_at, active, ..
            } => KeyInfo {
                id: id.clone(),
                kind: "bcs521".to_string(),
                public_key_hex: hex::encode(pk.to_bytes()),
                kahf: *kahf,
                fortress: *fortress,
                label: label.clone(),
                created_at: *created_at,
                active: *active,
            },
            StoredKey::Hybrid {
                id, pk, kahf, fortress, label, created_at, active, ..
            } => KeyInfo {
                id: id.clone(),
                kind: "hybrid-bcs521-mlkem1024".to_string(),
                public_key_hex: hex::encode(pk.to_bytes()),
                kahf: *kahf,
                fortress: *fortress,
                label: label.clone(),
                created_at: *created_at,
                active: *active,
            },
        }
    }

    fn set_active(&mut self, value: bool) {
        match self {
            StoredKey::Bcs521 { active, .. } => *active = value,
            StoredKey::Hybrid { active, .. } => *active = value,
        }
    }
}

/// Thread-safe in-memory key store.
pub struct KeyStore {
    keys: RwLock<HashMap<String, StoredKey>>,
}

impl KeyStore {
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
        }
    }

    /// Store a new key.
    pub fn insert(&self, key: StoredKey) {
        let mut keys = self.keys.write().unwrap();
        keys.insert(key.id().to_string(), key);
    }

    /// List all keys (public info only).
    pub fn list(&self) -> Vec<KeyInfo> {
        let keys = self.keys.read().unwrap();
        keys.values().map(|k| k.to_key_info()).collect()
    }

    /// Get public info for a key by ID.
    pub fn get_info(&self, id: &str) -> Option<KeyInfo> {
        let keys = self.keys.read().unwrap();
        keys.get(id).map(|k| k.to_key_info())
    }

    /// Revoke (soft delete) a key.
    pub fn revoke(&self, id: &str) -> bool {
        let mut keys = self.keys.write().unwrap();
        match keys.get_mut(id) {
            Some(k) => {
                k.set_active(false);
                true
            }
            None => false,
        }
    }

    /// Run an operation with a borrowed view of an active BCS-521 SK + PK.
    /// Returns `None` if the key does not exist, is not active, or is the
    /// wrong kind.
    pub fn with_bcs521<F, T>(&self, id: &str, f: F) -> Option<T>
    where
        F: FnOnce(&Bcs521SecretKey, &Bcs521PublicKey) -> T,
    {
        let keys = self.keys.read().unwrap();
        match keys.get(id) {
            Some(StoredKey::Bcs521 { sk, pk, active, .. }) if *active => Some(f(sk, pk)),
            _ => None,
        }
    }

    /// Run an operation with a borrowed view of an active Hybrid SK + PK.
    pub fn with_hybrid<F, T>(&self, id: &str, f: F) -> Option<T>
    where
        F: FnOnce(&HybridSecretKey, &HybridPublicKey) -> T,
    {
        let keys = self.keys.read().unwrap();
        match keys.get(id) {
            Some(StoredKey::Hybrid { sk, pk, active, .. }) if *active => Some(f(sk, pk)),
            _ => None,
        }
    }
}

//! # On-disk key file format for the BCS CLI.
//!
//! Two text formats are supported, both designed to round-trip cleanly
//! and to be obvious about what they contain:
//!
//! BCS-521 secret key (`.bcs521-sk`):
//!
//! ```text
//! -----BEGIN BCS-521 SECRET KEY-----
//! kind: bcs521
//! kahf: true
//! label: zakat-audit-2026
//! sk: <132 hex chars of the 66-byte scalar>
//! -----END BCS-521 SECRET KEY-----
//! ```
//!
//! BCS-521 public key (`.bcs521-pub`):
//!
//! ```text
//! -----BEGIN BCS-521 PUBLIC KEY-----
//! kind: bcs521
//! pk: <266 hex chars of the 133-byte SEC1-uncompressed encoding>
//! -----END BCS-521 PUBLIC KEY-----
//! ```
//!
//! Hybrid keypair files use the same envelope with `kind: hybrid`. Note
//! that the hybrid secret-key serialisation depends on the upstream
//! `ml-kem` crate version; for v0.2.x the CLI only supports hybrid keys
//! in-memory (per-invocation) and does *not* persist hybrid secret keys
//! to disk \u2014 see `commands::hybrid_kem` for the workflow that avoids
//! disk persistence of `HybridSecretKey`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// A parsed PEM-style envelope: header tag, body fields, footer tag.
pub struct KeyEnvelope {
    pub tag: String,
    pub fields: BTreeMap<String, String>,
}

impl KeyEnvelope {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
        let header = lines.next().ok_or_else(|| "empty key file".to_string())?;
        let tag = header
            .strip_prefix("-----BEGIN ")
            .and_then(|s| s.strip_suffix("-----"))
            .ok_or_else(|| format!("missing BEGIN header (got {:?})", header))?
            .to_string();

        let mut fields = BTreeMap::new();
        for line in lines {
            if line.starts_with("-----END ") {
                return Ok(KeyEnvelope { tag, fields });
            }
            if let Some((k, v)) = line.split_once(':') {
                fields.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        Err("missing END footer".to_string())
    }

    pub fn get(&self, key: &str) -> Result<&str, String> {
        self.fields
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| format!("missing field `{}` in key file", key))
    }
}

/// Write a BCS-521 secret key as a PEM-style envelope.
pub fn write_bcs521_sk(
    path: &Path,
    sk_hex: &str,
    kahf: bool,
    label: Option<&str>,
) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("-----BEGIN BCS-521 SECRET KEY-----\n");
    out.push_str("kind: bcs521\n");
    out.push_str(&format!("kahf: {}\n", kahf));
    if let Some(l) = label {
        out.push_str(&format!("label: {}\n", l));
    }
    out.push_str(&format!("sk: {}\n", sk_hex));
    out.push_str("-----END BCS-521 SECRET KEY-----\n");
    fs::write(path, out).map_err(|e| format!("write {}: {}", path.display(), e))
}

/// Write a BCS-521 public key as a PEM-style envelope.
pub fn write_bcs521_pub(
    path: &Path,
    pk_hex: &str,
    kahf: bool,
    label: Option<&str>,
) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("-----BEGIN BCS-521 PUBLIC KEY-----\n");
    out.push_str("kind: bcs521\n");
    out.push_str(&format!("kahf: {}\n", kahf));
    if let Some(l) = label {
        out.push_str(&format!("label: {}\n", l));
    }
    out.push_str(&format!("pk: {}\n", pk_hex));
    out.push_str("-----END BCS-521 PUBLIC KEY-----\n");
    fs::write(path, out).map_err(|e| format!("write {}: {}", path.display(), e))
}

/// Read a BCS-521 secret-key envelope and return the raw 66-byte hex.
pub fn read_bcs521_sk_hex(path: &Path) -> Result<String, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let env = KeyEnvelope::parse(&text)?;
    if env.tag != "BCS-521 SECRET KEY" {
        return Err(format!(
            "expected `BCS-521 SECRET KEY` envelope, got `{}`",
            env.tag
        ));
    }
    env.get("sk").map(str::to_string)
}

/// Read a BCS-521 public-key envelope and return the raw 133-byte hex.
pub fn read_bcs521_pub_hex(path: &Path) -> Result<String, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let env = KeyEnvelope::parse(&text)?;
    if env.tag != "BCS-521 PUBLIC KEY" {
        return Err(format!(
            "expected `BCS-521 PUBLIC KEY` envelope, got `{}`",
            env.tag
        ));
    }
    env.get("pk").map(str::to_string)
}

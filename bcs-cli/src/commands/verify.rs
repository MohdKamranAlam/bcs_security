//! `bcs verify` — BCS-521 ECDSA deterministic signature verification.
//!
//! Accepts the 132-byte signature as hex on the command line or reads it
//! from a file (if the `--signature` value is a path that exists on disk).
//!
//! Exit codes: 0 = valid, 1 = invalid / error, 2 = not-yet-CT warning.

use std::path::PathBuf;
use std::{fs, io};

use bcs_core_rust::{ecdsa_verify, Bcs521Signature};

use crate::keyfile::read_bcs521_pub_hex;

pub fn run(key: PathBuf, message: Option<String>, file: Option<PathBuf>, signature: String) {
    // Load the message bytes.
    let msg_bytes: Vec<u8> = match (message, file) {
        (Some(m), _) => m.into_bytes(),
        (None, Some(f)) => match fs::read(&f) {
            Ok(b) => b,
            Err(e) => fatal(format!("read file {:?}: {}", f, e)),
        },
        (None, None) => {
            use io::Read;
            let mut buf = Vec::new();
            io::stdin().read_to_end(&mut buf).unwrap_or_else(|e| fatal(format!("stdin: {}", e)));
            buf
        }
    };

    // Load public key (SEC1 uncompressed 133 bytes).
    let pk_hex = match read_bcs521_pub_hex(&key) {
        Ok(h) => h,
        Err(e) => fatal(format!("public key: {}", e)),
    };
    let pk_bytes = match hex::decode(&pk_hex) {
        Ok(b) => b,
        Err(e) => fatal(format!("public key hex: {}", e)),
    };

    // Parse signature hex (could also be a file path).
    let sig_hex = if std::path::Path::new(&signature).exists() {
        match fs::read_to_string(&signature) {
            Ok(s) => s.trim().to_string(),
            Err(e) => fatal(format!("read signature file: {}", e)),
        }
    } else {
        signature.trim().to_string()
    };
    let sig_bytes = match hex::decode(&sig_hex) {
        Ok(b) => b,
        Err(e) => fatal(format!("signature hex: {}", e)),
    };
    let sig = match Bcs521Signature::from_bytes(&sig_bytes) {
        Some(s) => s,
        None => fatal("signature must be exactly 132 bytes (264 hex chars)".to_string()),
    };

    match ecdsa_verify(&pk_bytes, &msg_bytes, &sig) {
        Ok(true) => {
            println!("VALID   algorithm: BCS-521-ECDSA-RFC6979-SHA256-v1");
            std::process::exit(0);
        }
        Ok(false) => {
            eprintln!("INVALID signature does not verify");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

fn fatal(msg: String) -> ! {
    eprintln!("error: {}", msg);
    std::process::exit(1);
}

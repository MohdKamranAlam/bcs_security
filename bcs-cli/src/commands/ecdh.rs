//! `bcs ecdh` — real BCS-521 ECDH + HKDF-SHA-256 key agreement.

use std::fs;
use std::path::PathBuf;

use bcs_core_rust::{Bcs521, Bcs521PublicKey, Bcs521SecretKey};

use crate::keyfile::{read_bcs521_pub_hex, read_bcs521_sk_hex};

/// Compute a real BCS-521 ECDH shared secret and write it (32 raw bytes).
pub fn run(private: PathBuf, public: PathBuf, output: PathBuf) {
    println!("BCS-521 ECDH");

    // Load and decode the local secret key.
    let sk_hex = match read_bcs521_sk_hex(&private) {
        Ok(h) => h,
        Err(e) => fatal(format!("private key: {}", e)),
    };
    let sk_bytes = match hex::decode(sk_hex) {
        Ok(b) => b,
        Err(e) => fatal(format!("private key hex: {}", e)),
    };
    if sk_bytes.len() != 66 {
        fatal(format!(
            "private key must be exactly 66 bytes, got {}",
            sk_bytes.len()
        ));
    }
    let mut sk_arr = [0u8; 66];
    sk_arr.copy_from_slice(&sk_bytes);
    let sk = match Bcs521SecretKey::from_bytes(&sk_arr) {
        Ok(s) => s,
        Err(e) => fatal(format!("private key rejected: {}", e)),
    };

    // Load and decode the peer's public key.
    let pk_hex = match read_bcs521_pub_hex(&public) {
        Ok(h) => h,
        Err(e) => fatal(format!("public key: {}", e)),
    };
    let pk_bytes = match hex::decode(pk_hex) {
        Ok(b) => b,
        Err(e) => fatal(format!("public key hex: {}", e)),
    };
    let peer_pk = match Bcs521PublicKey::from_bytes(&pk_bytes) {
        Ok(p) => p,
        Err(e) => fatal(format!("public key rejected: {}", e)),
    };

    // Real ECDH + HKDF-SHA-256.
    let shared = match Bcs521::ecdh(&sk, &peer_pk) {
        Ok(s) => s,
        Err(e) => fatal(format!("ECDH failed: {}", e)),
    };

    if let Err(e) = fs::write(&output, shared.as_bytes()) {
        fatal(format!("write {}: {}", output.display(), e));
    }

    println!(
        "wrote 32-byte shared secret: {} (algorithm: BCS-521-ECDH-HKDF-SHA256-v1)",
        output.display()
    );
}

fn fatal(msg: String) -> ! {
    eprintln!("error: {}", msg);
    std::process::exit(1);
}

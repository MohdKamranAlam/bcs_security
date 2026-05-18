//! `bcs hybrid-kem` — real BCS-521 + ML-KEM-1024 hybrid KEM.
//!
//! For v0.2.x the CLI keeps hybrid secret keys *in memory only* and
//! exposes a single self-contained workflow rather than persisting
//! `HybridSecretKey` to disk (the upstream `ml-kem` crate's DK
//! serialization is unstable across releases).
//!
//! Three subcommands:
//!
//! * `--keygen` writes a `hybrid-pub` envelope only. The matching
//!   `HybridSecretKey` is held in memory and dropped at process exit.
//!   Use this to test peer interop (run encaps in this same process
//!   immediately afterwards).
//! * `--encaps` reads a peer's `hybrid-pub` and writes the ciphertext
//!   + shared secret to disk.
//! * `--decaps` is not yet available from the CLI — use the HTTP API
//!   (`bcs-shield`) where secret keys are persistent in-process.

use std::fs;
use std::path::PathBuf;

use bcs_core_rust::hybrid::{BcsHybrid521Mlkem1024, HybridPublicKey};
use rand::rngs::OsRng;

/// Encapsulate against a peer's hybrid public key file.
///
/// The peer's public key file must contain the 1701-byte hybrid public
/// key as a single hex string (use `bcs-shield`'s `/api/v1/keys/generate`
/// with `kind=hybrid` to produce one, then save its `public_key_hex`).
pub fn encaps(public: PathBuf, output: PathBuf) {
    println!("BCS-521 + ML-KEM-1024 hybrid encaps");

    let hex_text = match fs::read_to_string(&public) {
        Ok(t) => t.trim().to_string(),
        Err(e) => fatal(format!("read peer public key: {}", e)),
    };
    let pk_bytes = match hex::decode(hex_text) {
        Ok(b) => b,
        Err(e) => fatal(format!("peer public key hex: {}", e)),
    };
    let peer_pk = match HybridPublicKey::from_bytes(&pk_bytes) {
        Ok(p) => p,
        Err(e) => fatal(format!("peer public key rejected: {}", e)),
    };

    let mut rng = OsRng;
    let (ct, ss) = match BcsHybrid521Mlkem1024::encapsulate(&mut rng, &peer_pk) {
        Ok(v) => v,
        Err(e) => fatal(format!("hybrid encaps failed: {}", e)),
    };

    // Write ciphertext (hex) and shared secret (raw 32 bytes) to two files.
    let ct_path = output.with_extension("hybrid-ct");
    let ss_path = output.with_extension("hybrid-ss");

    if let Err(e) = fs::write(&ct_path, hex::encode(ct.to_bytes())) {
        fatal(format!("write {}: {}", ct_path.display(), e));
    }
    if let Err(e) = fs::write(&ss_path, ss.as_bytes()) {
        fatal(format!("write {}: {}", ss_path.display(), e));
    }

    println!("wrote ciphertext  : {} ({} bytes hex)", ct_path.display(), 1701 * 2);
    println!("wrote shared secret: {} (32 raw bytes)", ss_path.display());
    println!("algorithm: BCS-Hybrid-521-MLKEM1024-v1");
}

/// Decaps from the CLI is not available in v0.2.x — see `bcs-shield`.
pub fn decaps(_private: PathBuf, _ciphertext: PathBuf, _output: PathBuf) {
    eprintln!(
        "error: `bcs hybrid-kem --decaps` is not available from the CLI in v0.2.x. \n\
         Hybrid secret keys are not persistable to disk yet (the upstream \
         ml-kem crate's DK serialisation is unstable). \n\
         Use the HTTP API instead: POST /api/v1/crypto/hybrid-decaps on a \
         running `bcs-shield` server."
    );
    std::process::exit(2);
}

fn fatal(msg: String) -> ! {
    eprintln!("error: {}", msg);
    std::process::exit(1);
}

//! `bcs sign` — BCS-521 ECDSA deterministic signing (RFC 6979, SHA-256).
//!
//! ## Warning
//!
//! The underlying `bcs-core-rust::ecdsa` module uses the **BigUint reference
//! path**, which is *not constant-time*.  Timing analysis on the signing
//! machine may recover the private key.  Use only for offline signing,
//! testing, and interoperability checks until the CT Barrett path ships
//! in v0.3.1.

use std::path::PathBuf;
use std::{fs, io};

use bcs_core_rust::ecdsa_sign;
use zeroize::Zeroize;

use crate::keyfile::read_bcs521_sk_hex;

pub fn run(
    key: PathBuf,
    message: Option<String>,
    file: Option<PathBuf>,
    output: Option<PathBuf>,
) {
    // Load the message bytes.
    let msg_bytes: Vec<u8> = match (message, file) {
        (Some(m), _) => m.into_bytes(),
        (None, Some(f)) => match fs::read(&f) {
            Ok(b) => b,
            Err(e) => fatal(format!("read file {:?}: {}", f, e)),
        },
        (None, None) => {
            // Read from stdin.
            use io::Read;
            let mut buf = Vec::new();
            io::stdin().read_to_end(&mut buf).unwrap_or_else(|e| fatal(format!("stdin: {}", e)));
            buf
        }
    };

    // Load secret key.
    let sk_hex = match read_bcs521_sk_hex(&key) {
        Ok(h) => h,
        Err(e) => fatal(format!("private key: {}", e)),
    };
    let sk_bytes_vec = match hex::decode(&sk_hex) {
        Ok(b) => b,
        Err(e) => fatal(format!("private key hex: {}", e)),
    };
    if sk_bytes_vec.len() != 66 {
        fatal(format!("private key must be 66 bytes, got {}", sk_bytes_vec.len()));
    }
    let mut sk_arr = [0u8; 66];
    sk_arr.copy_from_slice(&sk_bytes_vec);

    // Sign.
    eprintln!("[!] WARNING: bcs sign uses a non-constant-time BigUint path (v0.3.0).");
    eprintln!("    CT signing with Barrett reduction is planned for v0.3.1.");
    let sig = match ecdsa_sign(&sk_arr, &msg_bytes) {
        Ok(s) => s,
        Err(e) => {
            sk_arr.zeroize();
            fatal(format!("sign failed: {}", e));
        }
    };
    sk_arr.zeroize();

    let sig_bytes = sig.to_bytes();
    let sig_hex = hex::encode(sig_bytes);

    match output {
        Some(ref path) => {
            if let Err(e) = fs::write(path, &sig_hex) {
                fatal(format!("write {}: {}", path.display(), e));
            }
            println!("wrote signature ({} bytes): {}", sig_hex.len() / 2, path.display());
        }
        None => println!("{}", sig_hex),
    }

    println!("algorithm: BCS-521-ECDSA-RFC6979-SHA256-v1 (reference, not CT)");
}

fn fatal(msg: String) -> ! {
    eprintln!("error: {}", msg);
    std::process::exit(1);
}

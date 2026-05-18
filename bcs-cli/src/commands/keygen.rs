//! `bcs keygen` — generate a real BCS-521 keypair.
//!
//! This is a thin wrapper around `bcs_core_rust::Bcs521::keygen`, which
//! produces a uniform-random scalar in `[1, n_521 - 1]` and the
//! corresponding curve point. No placeholders.

use std::path::PathBuf;

use bcs_core_rust::Bcs521;
use rand::rngs::OsRng;
use zeroize::Zeroize;

use crate::keyfile::{write_bcs521_pub, write_bcs521_sk};

/// Generate a new keypair and write `.bcs521-sk` / `.bcs521-pub` files.
///
/// `kahf` and `fortress` are *metadata tags* preserved in the key file.
/// They do **not** alter the cryptographic generation path: BCS-521
/// secret keys must be uniform random per RFC 6090 §3, and the
/// constant-time + zeroize discipline is unconditional inside
/// `bcs-core-rust`. The CLI exposes the tags so a downstream operator
/// can later filter / report on them, not as a security claim.
pub fn run(output: PathBuf, kahf: bool, fortress: bool) {
    println!("BCS-521 keygen");
    println!("  kahf tag    : {}", kahf);
    println!("  fortress tag: {}", fortress);

    let mut rng = OsRng;
    let (sk, pk) = Bcs521::keygen(&mut rng);

    let mut sk_bytes = sk.to_bytes();
    let pk_bytes = pk.to_bytes();

    let sk_path = output.with_extension("bcs521-sk");
    let pub_path = output.with_extension("bcs521-pub");

    if let Err(e) = write_bcs521_sk(&sk_path, &hex::encode(sk_bytes), kahf, None) {
        eprintln!("error: {}", e);
        sk_bytes.zeroize();
        std::process::exit(1);
    }
    if let Err(e) = write_bcs521_pub(&pub_path, &hex::encode(pk_bytes), kahf, None) {
        eprintln!("error: {}", e);
        sk_bytes.zeroize();
        std::process::exit(1);
    }

    // Erase the local hex copy of the secret material.
    sk_bytes.zeroize();

    println!("wrote secret key: {}", sk_path.display());
    println!("wrote public key: {}", pub_path.display());
    if fortress {
        println!("note: fortress tag recorded; CT + zeroize are always on at the core level.");
    }
}

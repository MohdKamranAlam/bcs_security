//! Coverage-guided fuzz target exercising the full ECDH round-trip with
//! adversarial sk and pk inputs.
//!
//! Run with:
//!     cargo +nightly fuzz run fuzz_ecdh_round_trip
//!
//! Contracts:
//! * If both inputs validate, ECDH never panics.
//! * Both directions of ECDH agree (commutativity).

#![no_main]

use bcs_core_rust::{Bcs521, Bcs521SecretKey};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Need 132 bytes: 66 for sk_a, 66 for sk_b.
    if data.len() < 132 {
        return;
    }
    let mut sk_a_bytes = [0u8; 66];
    let mut sk_b_bytes = [0u8; 66];
    sk_a_bytes.copy_from_slice(&data[..66]);
    sk_b_bytes.copy_from_slice(&data[66..132]);

    let sk_a = match Bcs521SecretKey::from_bytes(&sk_a_bytes) {
        Ok(sk) => sk,
        Err(_) => return,
    };
    let sk_b = match Bcs521SecretKey::from_bytes(&sk_b_bytes) {
        Ok(sk) => sk,
        Err(_) => return,
    };
    let pk_a = sk_a.public_key();
    let pk_b = sk_b.public_key();

    let ss_ab = Bcs521::ecdh(&sk_a, &pk_b).expect("ECDH must succeed for valid keys");
    let ss_ba = Bcs521::ecdh(&sk_b, &pk_a).expect("ECDH must succeed for valid keys");
    assert_eq!(
        ss_ab.as_bytes(),
        ss_ba.as_bytes(),
        "ECDH commutativity violated"
    );
});

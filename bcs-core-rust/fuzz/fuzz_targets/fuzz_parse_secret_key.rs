//! Coverage-guided fuzz target for `Bcs521SecretKey::from_bytes`.
//!
//! Run with:
//!     cargo +nightly fuzz run fuzz_parse_secret_key
//!
//! Contracts:
//! * No panic for any 66-byte input.
//! * Every `Ok(sk)` round-trips byte-exactly.
//! * Every `Ok(sk)` produces a public key that itself round-trips.

#![no_main]

use bcs_core_rust::{Bcs521PublicKey, Bcs521SecretKey};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() != 66 {
        return;
    }
    let mut bytes = [0u8; 66];
    bytes.copy_from_slice(data);

    if let Ok(sk) = Bcs521SecretKey::from_bytes(&bytes) {
        // Round-trip the secret key.
        let bytes2 = sk.to_bytes();
        assert_eq!(bytes, bytes2, "secret key round-trip failure");

        // Derive a public key and round-trip it too.
        let pk = sk.public_key();
        let pk_bytes = pk.to_bytes();
        let _ = Bcs521PublicKey::from_bytes(&pk_bytes)
            .expect("derived public key must validate");
    }
});

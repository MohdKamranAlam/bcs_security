//! Coverage-guided fuzz target for `Bcs521PublicKey::from_bytes`.
//!
//! Run with:
//!     cargo +nightly fuzz run fuzz_parse_public_key
//!
//! The contract under test:
//!
//! * **No panic** for any byte string of any length.
//! * Every `Ok(pk)` must round-trip: `from_bytes(pk.to_bytes()) == Ok(pk_eq)`.

#![no_main]

use bcs_core_rust::Bcs521PublicKey;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(pk) = Bcs521PublicKey::from_bytes(data) {
        let bytes = pk.to_bytes();
        let pk2 = Bcs521PublicKey::from_bytes(&bytes)
            .expect("re-parse of a just-emitted public key must succeed");
        assert_eq!(bytes, pk2.to_bytes(), "round-trip mismatch");
    }
});

//! Property-based fuzz tests for the BCS-521 public API.
//!
//! Uses [`proptest`](https://docs.rs/proptest) to generate adversarial
//! inputs and check that the API:
//!
//! 1. **Never panics** on arbitrary byte input (no unwraps escape).
//! 2. **Round-trips** every value it produced itself.
//! 3. Satisfies basic algebraic properties (ECDH commutativity).
//!
//! These tests run inside `cargo test --features ct`, which makes them
//! cheap CI gates.  For deeper, coverage-guided fuzzing see
//! [`fuzz/fuzz_targets/*`] (libfuzzer via `cargo fuzz`).

#![cfg(feature = "ct")]

use bcs_core_rust::{
    Bcs521, Bcs521Error, Bcs521PublicKey, Bcs521SecretKey,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// 1. Robustness: parsers must NEVER panic on adversarial input.
// ---------------------------------------------------------------------------

proptest! {
    /// `Bcs521SecretKey::from_bytes` must return cleanly for any 66-byte
    /// input — never panic, never abort.  Acceptance is permitted only
    /// for values strictly in `[1, n_521 − 1]`.
    #[test]
    fn secret_key_parser_never_panics(bytes in prop::array::uniform32(any::<u8>())) {
        // Extend 32 random bytes to the 66 required, with high half zero
        // so we cover the "value < n" subspace densely.
        let mut full = [0u8; 66];
        full[34..].copy_from_slice(&bytes);
        let _ = Bcs521SecretKey::from_bytes(&full);
    }

    /// `Bcs521PublicKey::from_bytes` must handle any byte slice of any
    /// length without panicking.  The vast majority of inputs are
    /// rejected; the test only checks the panic-freedom invariant.
    #[test]
    fn public_key_parser_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..200)) {
        let _ = Bcs521PublicKey::from_bytes(&bytes);
    }

    /// Even random 133-byte buffers with the correct tag byte must be
    /// rejected without panic.
    #[test]
    fn public_key_parser_rejects_random_133b(payload in prop::array::uniform32(any::<u8>())) {
        let mut bytes = [0u8; 133];
        bytes[0] = 0x04;
        // Spray the tag and payload into the buffer.
        for (i, b) in payload.iter().enumerate() {
            bytes[1 + i] = *b;
            bytes[1 + 32 + i] = b.wrapping_add(11);
        }
        // 133 random bytes are statistically not on the curve; expect
        // an Err but absolutely no panic.
        let res = Bcs521PublicKey::from_bytes(&bytes);
        prop_assert!(
            res.is_err(),
            "random 133-byte buffer accepted as a public key — collision space too dense"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Round-trip: every output of `to_bytes` must be accepted by `from_bytes`.
// ---------------------------------------------------------------------------

proptest! {
    /// Every secret key generated from a 32-byte random seed (which
    /// always falls within `[1, n)`) must serialize and deserialize to
    /// itself.
    #[test]
    fn secret_key_round_trip(seed in prop::array::uniform32(any::<u8>())) {
        let mut bytes = [0u8; 66];
        bytes[34..].copy_from_slice(&seed);
        // Skip the all-zero case (rejected by spec).
        prop_assume!(bytes.iter().any(|b| *b != 0));

        let sk = Bcs521SecretKey::from_bytes(&bytes).expect("non-zero & < n");
        let bytes2 = sk.to_bytes();
        prop_assert_eq!(bytes, bytes2);
    }

    /// Every public key produced by `sk.public_key()` must round-trip.
    #[test]
    fn public_key_round_trip(seed in prop::array::uniform32(any::<u8>())) {
        let mut bytes = [0u8; 66];
        bytes[34..].copy_from_slice(&seed);
        prop_assume!(bytes.iter().any(|b| *b != 0));

        let sk = Bcs521SecretKey::from_bytes(&bytes).expect("non-zero & < n");
        let pk = sk.public_key();
        let pk_bytes = pk.to_bytes();
        let pk_again = Bcs521PublicKey::from_bytes(&pk_bytes)
            .expect("just-produced public key must validate");
        prop_assert_eq!(pk_bytes, pk_again.to_bytes());
    }
}

// ---------------------------------------------------------------------------
// 3. Algebraic property: ECDH is commutative across legitimate keypairs.
// ---------------------------------------------------------------------------

proptest! {
    /// `ECDH(sk_a, pk_b) == ECDH(sk_b, pk_a)` for any two legitimate
    /// keypairs.  Both sides derive the same 32-byte symmetric secret.
    #[test]
    fn ecdh_commutativity(
        seed_a in prop::array::uniform32(any::<u8>()),
        seed_b in prop::array::uniform32(any::<u8>()),
    ) {
        let mut bytes_a = [0u8; 66];
        let mut bytes_b = [0u8; 66];
        bytes_a[34..].copy_from_slice(&seed_a);
        bytes_b[34..].copy_from_slice(&seed_b);
        prop_assume!(bytes_a.iter().any(|b| *b != 0));
        prop_assume!(bytes_b.iter().any(|b| *b != 0));

        let sk_a = Bcs521SecretKey::from_bytes(&bytes_a).expect("non-zero");
        let sk_b = Bcs521SecretKey::from_bytes(&bytes_b).expect("non-zero");
        let pk_a = sk_a.public_key();
        let pk_b = sk_b.public_key();

        let ss_ab = Bcs521::ecdh(&sk_a, &pk_b).unwrap();
        let ss_ba = Bcs521::ecdh(&sk_b, &pk_a).unwrap();

        prop_assert_eq!(ss_ab.as_bytes(), ss_ba.as_bytes());
    }

    /// `ECDH(sk, pk)` is deterministic — calling twice yields the same
    /// shared secret.
    #[test]
    fn ecdh_determinism(
        seed_a in prop::array::uniform32(any::<u8>()),
        seed_b in prop::array::uniform32(any::<u8>()),
    ) {
        let mut bytes_a = [0u8; 66];
        let mut bytes_b = [0u8; 66];
        bytes_a[34..].copy_from_slice(&seed_a);
        bytes_b[34..].copy_from_slice(&seed_b);
        prop_assume!(bytes_a.iter().any(|b| *b != 0));
        prop_assume!(bytes_b.iter().any(|b| *b != 0));

        let sk_a = Bcs521SecretKey::from_bytes(&bytes_a).expect("non-zero");
        let pk_b = Bcs521SecretKey::from_bytes(&bytes_b).expect("non-zero").public_key();

        let ss1 = Bcs521::ecdh(&sk_a, &pk_b).unwrap();
        let ss2 = Bcs521::ecdh(&sk_a, &pk_b).unwrap();
        prop_assert_eq!(ss1.as_bytes(), ss2.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// 4. Targeted edge-case strategies.
// ---------------------------------------------------------------------------

/// Generate adversarial public-key encodings designed to hit specific
/// rejection paths.
fn evil_public_key_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        // Wrong length, all zeros.
        Just(vec![0u8; 100]),
        Just(vec![0u8; 132]),
        Just(vec![0u8; 134]),
        Just(vec![]),
        Just(vec![0x04]),
        // Right length, wrong tag.
        Just({
            let mut v = vec![0u8; 133];
            v[0] = 0x02;
            v
        }),
        Just({
            let mut v = vec![0u8; 133];
            v[0] = 0x03;
            v
        }),
        Just({
            let mut v = vec![0u8; 133];
            v[0] = 0x06;
            v
        }),
        // Right tag, all-zero coords (off-curve).
        Just({
            let mut v = vec![0u8; 133];
            v[0] = 0x04;
            v
        }),
        // Right tag, max-value X (out of range).
        Just({
            let mut v = vec![0xFFu8; 133];
            v[0] = 0x04;
            v
        }),
    ]
}

proptest! {
    #[test]
    fn evil_public_keys_all_rejected(bytes in evil_public_key_strategy()) {
        let res = Bcs521PublicKey::from_bytes(&bytes);
        prop_assert!(
            res.is_err(),
            "deliberately-malformed public key was accepted: {:?}",
            res.map(|pk| pk.to_bytes())
        );
    }
}

/// Generate adversarial secret-key encodings.
fn evil_secret_key_strategy() -> impl Strategy<Value = [u8; 66]> {
    prop_oneof![
        // All zeros — rejected by spec.
        Just([0u8; 66]),
        // All ones — > n_521.
        Just([0xFFu8; 66]),
        // Just the high bit set in the leading byte (still > n_521 for some patterns).
        Just({
            let mut v = [0u8; 66];
            v[0] = 0xFF;
            v
        }),
    ]
}

proptest! {
    #[test]
    fn evil_secret_keys_all_rejected(bytes in evil_secret_key_strategy()) {
        let res = Bcs521SecretKey::from_bytes(&bytes);
        prop_assert!(matches!(
            res,
            Err(Bcs521Error::SecretKeyIsZero) | Err(Bcs521Error::SecretKeyOutOfRange)
        ));
    }
}

//! Integration tests for BCS-521 ECDSA (reference path).
//!
//! Run with: `cargo test --features ecdsa -- test_ecdsa`
//! Or all features: `cargo test --features fortress`

#[cfg(feature = "ecdsa")]
mod ecdsa_tests {
    use bcs_core_rust::{
        bcs521, biguint_to_be_bytes, ecdsa_sign, ecdsa_verify, Bcs521Signature, Point,
    };
    use num_bigint::BigUint;

    fn small_sk(v: u64) -> [u8; 66] {
        let mut b = [0u8; 66];
        b[58..].copy_from_slice(&v.to_be_bytes());
        b
    }

    fn pk_for_sk(sk_bytes: &[u8; 66]) -> Vec<u8> {
        let curve = bcs521();
        let sk = BigUint::from_bytes_be(sk_bytes);
        let pk = curve.public_key(&sk).expect("public_key");
        let (x, y) = match pk {
            Point::Affine { x, y } => (x, y),
            _ => panic!("pk is infinity"),
        };
        let mut out = vec![0x04u8];
        out.extend_from_slice(&biguint_to_be_bytes(&x, 66));
        out.extend_from_slice(&biguint_to_be_bytes(&y, 66));
        out
    }

    // ---------------------------------------------------------------
    // Correctness
    // ---------------------------------------------------------------

    #[test]
    fn sign_verify_basic() {
        let sk = small_sk(0x1234_5678_9abc_def0u64);
        let pk = pk_for_sk(&sk);
        let msg = b"Bismillah";
        let sig = ecdsa_sign(&sk, msg).expect("sign");
        let ok = ecdsa_verify(&pk, msg, &sig).expect("verify");
        assert!(ok, "valid signature must verify");
    }

    #[test]
    fn wrong_message_rejected() {
        let sk = small_sk(42);
        let pk = pk_for_sk(&sk);
        let sig = ecdsa_sign(&sk, b"correct").expect("sign");
        let ok = ecdsa_verify(&pk, b"tampered", &sig).expect("verify");
        assert!(!ok, "tampered message must not verify");
    }

    #[test]
    fn wrong_key_rejected() {
        let sk_a = small_sk(1);
        let sk_b = small_sk(2);
        let pk_b = pk_for_sk(&sk_b);
        let sig_a = ecdsa_sign(&sk_a, b"msg").expect("sign");
        let ok = ecdsa_verify(&pk_b, b"msg", &sig_a).expect("verify");
        assert!(!ok, "signature under key-A must not verify with key-B public key");
    }

    #[test]
    fn mutated_r_rejected() {
        let sk = small_sk(7);
        let pk = pk_for_sk(&sk);
        let msg = b"test";
        let mut sig = ecdsa_sign(&sk, msg).expect("sign");
        sig.r[32] ^= 0x01; // flip one bit in r
        let ok = ecdsa_verify(&pk, msg, &sig).expect("verify");
        assert!(!ok, "mutated r must not verify");
    }

    #[test]
    fn mutated_s_rejected() {
        let sk = small_sk(99);
        let pk = pk_for_sk(&sk);
        let msg = b"hello";
        let mut sig = ecdsa_sign(&sk, msg).expect("sign");
        sig.s[10] ^= 0xFF;
        let ok = ecdsa_verify(&pk, msg, &sig).expect("verify");
        assert!(!ok, "mutated s must not verify");
    }

    // ---------------------------------------------------------------
    // RFC 6979 determinism
    // ---------------------------------------------------------------

    #[test]
    fn deterministic_same_output() {
        let sk = small_sk(13);
        let msg = b"determinism";
        let s1 = ecdsa_sign(&sk, msg).expect("sign 1");
        let s2 = ecdsa_sign(&sk, msg).expect("sign 2");
        assert_eq!(s1, s2, "RFC 6979: same (sk, msg) must always produce same signature");
    }

    #[test]
    fn different_messages_different_nonce() {
        let sk = small_sk(13);
        let s1 = ecdsa_sign(&sk, b"msg1").expect("sign 1");
        let s2 = ecdsa_sign(&sk, b"msg2").expect("sign 2");
        // r values come from the nonce point; they must differ per RFC 6979.
        assert_ne!(s1.r, s2.r, "different messages must produce different r values");
    }

    // ---------------------------------------------------------------
    // Encoding
    // ---------------------------------------------------------------

    #[test]
    fn signature_encoding_roundtrip() {
        let sk = small_sk(5);
        let sig = ecdsa_sign(&sk, b"encode").expect("sign");
        let bytes = sig.to_bytes();
        assert_eq!(bytes.len(), 132);
        let decoded = Bcs521Signature::from_bytes(&bytes).expect("decode");
        assert_eq!(sig, decoded);
    }

    #[test]
    fn from_bytes_wrong_length_returns_none() {
        assert!(Bcs521Signature::from_bytes(&[0u8; 131]).is_none());
        assert!(Bcs521Signature::from_bytes(&[0u8; 133]).is_none());
    }

    // ---------------------------------------------------------------
    // Edge / error cases
    // ---------------------------------------------------------------

    #[test]
    fn zero_sk_rejected() {
        assert!(ecdsa_sign(&[0u8; 66], b"x").is_err());
    }

    #[test]
    fn verify_bad_pk_length() {
        let sk = small_sk(3);
        let sig = ecdsa_sign(&sk, b"x").expect("sign");
        let err = ecdsa_verify(&[0u8; 100], b"x", &sig);
        assert!(err.is_err(), "wrong-length pk must error");
    }

    #[test]
    fn verify_bad_pk_tag() {
        let sk = small_sk(3);
        let pk = pk_for_sk(&sk);
        let mut bad = pk.clone();
        bad[0] = 0x03; // compressed tag — not supported
        let sig = ecdsa_sign(&sk, b"x").expect("sign");
        assert!(ecdsa_verify(&bad, b"x", &sig).is_err());
    }
}

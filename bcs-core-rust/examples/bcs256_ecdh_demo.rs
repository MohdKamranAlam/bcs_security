//! BCS-256 end-to-end ECDH demo.
//!
//! Run with:
//!   cargo run --release --example bcs256_ecdh_demo

use bcs_core_rust::{bcs256, hkdf_sha256, Point};
use num_bigint::BigUint;
use num_traits::Zero;

fn print_point(label: &str, p: &Point) {
    match p {
        Point::Infinity => println!("{} = O (infinity)", label),
        Point::Affine { x, y } => {
            println!("{}.x = {}", label, x);
            println!("{}.y = {}", label, y);
        }
    }
}

fn main() {
    let curve = bcs256();
    println!("=== {} ECDH demo ===", curve.name);
    println!("p bits = {}", curve.p.bits());
    println!("n bits = {}", curve.n.bits());

    let alice_sk = curve.generate_private_key();
    let alice_pk = curve.public_key(&alice_sk).unwrap();
    let bob_sk = curve.generate_private_key();
    let bob_pk = curve.public_key(&bob_sk).unwrap();

    print_point("Alice public", &alice_pk);
    print_point("Bob public", &bob_pk);

    let ss_alice = curve.ecdh(&alice_sk, &bob_pk).unwrap();
    let ss_bob = curve.ecdh(&bob_sk, &alice_pk).unwrap();
    assert_eq!(ss_alice, ss_bob);
    println!("[OK] shared_x = {}", hex::encode(&ss_alice));

    let mut ss32 = [0u8; 32];
    ss32.copy_from_slice(&ss_alice);
    let key = hkdf_sha256(&ss32, b"BCS-256 demo salt", b"BCS-256 ECDH v1");
    println!("HKDF-SHA-256 key = {}", hex::encode(key));

    // Reject off-curve attacker key
    let bad = Point::Affine {
        x: BigUint::zero(),
        y: BigUint::from(3u32),
    };
    assert!(curve.ecdh(&alice_sk, &bad).is_err());
    println!("[OK] off-curve key rejected");
}

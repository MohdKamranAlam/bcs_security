//! BCS-521 end-to-end ECDH demo.
//!
//! Run with:
//!   cargo run --release --example bcs521_ecdh_demo
//!
//! Demonstrates:
//!   1. Curve constants and generator validation
//!   2. Alice & Bob keypair generation
//!   3. Mutual public-key validation
//!   4. ECDH key agreement on both sides
//!   5. HKDF-SHA-512 expansion to a 64-byte symmetric secret
//!   6. Rejection of an invalid (off-curve) peer public key

use bcs_core_rust::{bcs521, hkdf_sha512_64, Point};
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
    let curve = bcs521();
    println!("=== {} ECDH demo ===", curve.name);
    println!("p bits = {}", curve.p.bits());
    println!("n bits = {}", curve.n.bits());
    println!("field_bytes = {}", curve.field_bytes);
    println!();

    // ----- Generator sanity -----
    assert!(curve.is_on_curve(&curve.g), "G must be on curve");
    println!("[OK] Generator G is on curve");
    print_point("G", &curve.g);
    println!();

    // ----- Step 1: Alice -----
    let alice_sk = curve.generate_private_key();
    let alice_pk = curve.public_key(&alice_sk).expect("Alice public key");
    println!("Alice private (hex): {:x}", alice_sk);
    print_point("Alice public", &alice_pk);
    println!();

    // ----- Step 2: Bob -----
    let bob_sk = curve.generate_private_key();
    let bob_pk = curve.public_key(&bob_sk).expect("Bob public key");
    println!("Bob private (hex): {:x}", bob_sk);
    print_point("Bob public", &bob_pk);
    println!();

    // ----- Step 3: validate received keys -----
    curve
        .validate_public_key(&bob_pk)
        .expect("Bob public must validate");
    curve
        .validate_public_key(&alice_pk)
        .expect("Alice public must validate");
    println!("[OK] Both public keys passed strict validation");

    // ----- Step 4: ECDH on both sides -----
    let ss_alice = curve.ecdh(&alice_sk, &bob_pk).expect("Alice ECDH");
    let ss_bob = curve.ecdh(&bob_sk, &alice_pk).expect("Bob ECDH");
    assert_eq!(ss_alice, ss_bob, "ECDH shared secrets must match");
    assert_eq!(ss_alice.len(), 66);
    println!("[OK] ECDH shared secret agrees on both sides");
    println!("shared_x (66 bytes hex):");
    println!("  {}", hex::encode(&ss_alice));
    println!();

    // ----- Step 5: derive a 64-byte symmetric key via HKDF-SHA-512 -----
    let key = hkdf_sha512_64(&ss_alice, b"BCS-521 demo salt", b"BCS-521 ECDH v1");
    println!("HKDF-SHA-512 derived key (64 bytes hex):");
    println!("  {}", hex::encode(key));
    println!();

    // ----- Step 6: malicious peer key must be rejected -----
    let bad = Point::Affine {
        x: BigUint::zero(),
        y: BigUint::from(3u32), // Not on curve (since (0,2) and (0,-2) are the only x=0 points)
    };
    assert!(
        curve.ecdh(&alice_sk, &bad).is_err(),
        "Off-curve point must be rejected"
    );
    println!("[OK] Off-curve attacker key correctly rejected");

    println!("\n=== BCS-521 ECDH demo complete ===");
}

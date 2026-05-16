//! Smoke tests for the constant-time core (feature `ct`).
//!
//! Runs only with:
//!   cargo test --features ct
//!
//! These tests exercise the API surface that lands first
//! (Fp521 add/sub/neg/conditional_swap, Scalar bit iteration).
//! The full byte-level parity vs the BigUint reference impl lives in
//! `tests/test_ct_parity.rs` once Montgomery multiplication is wired.

#![cfg(feature = "ct")]

use bcs_core_rust::ct::{Fp521, Scalar};

#[test]
fn fp521_add_then_sub_is_identity() {
    let mut a_bytes = [0u8; 66];
    a_bytes[65] = 0x11;
    let a = Fp521::from_bytes_be(&a_bytes).unwrap();

    let mut b_bytes = [0u8; 66];
    b_bytes[65] = 0x22;
    let b = Fp521::from_bytes_be(&b_bytes).unwrap();

    let sum  = a + b;
    let back = sum - b;
    assert_eq!(back, a);
}

#[test]
fn fp521_neg_neg_round_trip() {
    let mut bytes = [0u8; 66];
    bytes[65] = 0x42;
    let a = Fp521::from_bytes_be(&bytes).unwrap();
    assert_eq!(-(-a), a);
}

#[test]
fn scalar_bit_iter_yields_521_bits() {
    let mut bytes = [0u8; 66];
    bytes[65] = 0xAB;
    let s = Scalar::from_bytes_be(&bytes).unwrap();
    assert_eq!(s.bits_msb_first().count(), 521);
}

#[test]
fn scalar_bit_iter_msb_first_lsb_last() {
    // s = 0b0...0011  →  MSB-first yields 519 zeros, then 1, 1.
    let mut bytes = [0u8; 66];
    bytes[65] = 0b11;
    let s = Scalar::from_bytes_be(&bytes).unwrap();
    let bits: Vec<u8> = s.bits_msb_first().collect();
    assert_eq!(bits.len(), 521);
    // First 519 bits must be zero.
    assert!(bits[..519].iter().all(|&b| b == 0));
    // Last two bits must be 1.
    assert_eq!(bits[519], 1);
    assert_eq!(bits[520], 1);
}

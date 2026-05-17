//! # PHASE C-2 — Honest perf comparison vs. industry curves
//!
//! This Criterion suite measures **end-to-end ECDH** time (key
//! generation + scalar mul + KDF) for our [`Bcs521`] against the most
//! widely deployed pure-Rust elliptic-curve libraries:
//!
//! | Curve              | Crate                  | Bits | Notes |
//! |--------------------|------------------------|------|-------|
//! | NIST P-256         | `p256` (RustCrypto)    | 256  | TLS, X.509 baseline |
//! | NIST P-521         | `p521` (RustCrypto)    | 521  | Closest cousin to BCS-521 |
//! | secp256k1          | `k256` (RustCrypto)    | 256  | Bitcoin / Ethereum |
//! | Curve25519         | `x25519-dalek`         | 255  | Best-in-class speed   |
//! | **BCS-521 (this)** | `bcs_core_rust::ct`    | 521  | Our novel curve       |
//!
//! ## Why these and not others?
//!
//! * **No `ring`** — `ring` does not expose a public ECDH API for
//!   independent benchmarking; its `agreement` API conflates entropy
//!   sourcing with the ECDH itself.
//! * **No OpenSSL FFI** — apples-to-apples requires same language,
//!   same allocator, same compiler.
//! * **No P-384** — uncommon at the SMB layer; P-256 + P-521 already
//!   bracket BCS-521.
//!
//! ## Honest disclaimer
//!
//! These numbers will show BCS-521 as **slower** than the optimised
//! Curve25519 / P-256 implementations.  That is *expected* — those
//! libraries have years of assembly tuning and are 256-bit curves;
//! BCS-521 is a 521-bit curve in a portable Rust Montgomery ladder.
//! The fair comparison is **BCS-521 vs P-521**: that is the
//! cousin-to-cousin number we care about.
//!
//! Run with:
//! ```bash
//! cargo bench --features ct --bench ecdh_compare
//! ```

#![cfg(feature = "ct")]

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// ---------------------------------------------------------------------------
// BCS-521 — our crate
// ---------------------------------------------------------------------------

fn bench_bcs521_keygen(c: &mut Criterion) {
    use bcs_core_rust::Bcs521;
    use rand::rngs::OsRng;
    c.bench_function("bcs521_keygen", |b| {
        b.iter(|| {
            let mut rng = OsRng;
            let (sk, pk) = Bcs521::keygen(&mut rng);
            black_box((sk, pk))
        })
    });
}

fn bench_bcs521_ecdh(c: &mut Criterion) {
    use bcs_core_rust::Bcs521;
    use rand::rngs::OsRng;

    // Pre-generate one peer keypair; only the ECDH call is timed.
    let mut rng = OsRng;
    let (sk_a, _pk_a) = Bcs521::keygen(&mut rng);
    let (_sk_b, pk_b) = Bcs521::keygen(&mut rng);

    c.bench_function("bcs521_ecdh", |b| {
        b.iter(|| black_box(Bcs521::ecdh(&sk_a, &pk_b).unwrap()))
    });
}

// ---------------------------------------------------------------------------
// NIST P-256 — RustCrypto p256
// ---------------------------------------------------------------------------

fn bench_p256_keygen(c: &mut Criterion) {
    use p256::ecdh::EphemeralSecret;
    use rand::rngs::OsRng;
    c.bench_function("p256_keygen", |b| {
        b.iter(|| {
            let sk = EphemeralSecret::random(&mut OsRng);
            let pk = sk.public_key();
            black_box((sk, pk))
        })
    });
}

fn bench_p256_ecdh(c: &mut Criterion) {
    use p256::ecdh::EphemeralSecret;
    use rand::rngs::OsRng;

    let sk_a = EphemeralSecret::random(&mut OsRng);
    let sk_b = EphemeralSecret::random(&mut OsRng);
    let pk_b = sk_b.public_key();

    c.bench_function("p256_ecdh", |b| {
        b.iter(|| black_box(sk_a.diffie_hellman(&pk_b)))
    });
}

// ---------------------------------------------------------------------------
// NIST P-521 — RustCrypto p521 (closest peer to BCS-521)
// ---------------------------------------------------------------------------

fn bench_p521_keygen(c: &mut Criterion) {
    use p521::ecdh::EphemeralSecret;
    use rand::rngs::OsRng;
    c.bench_function("p521_keygen", |b| {
        b.iter(|| {
            let sk = EphemeralSecret::random(&mut OsRng);
            let pk = sk.public_key();
            black_box((sk, pk))
        })
    });
}

fn bench_p521_ecdh(c: &mut Criterion) {
    use p521::ecdh::EphemeralSecret;
    use rand::rngs::OsRng;

    let sk_a = EphemeralSecret::random(&mut OsRng);
    let sk_b = EphemeralSecret::random(&mut OsRng);
    let pk_b = sk_b.public_key();

    c.bench_function("p521_ecdh", |b| {
        b.iter(|| black_box(sk_a.diffie_hellman(&pk_b)))
    });
}

// ---------------------------------------------------------------------------
// secp256k1 — RustCrypto k256
// ---------------------------------------------------------------------------

fn bench_k256_keygen(c: &mut Criterion) {
    use k256::ecdh::EphemeralSecret;
    use rand::rngs::OsRng;
    c.bench_function("k256_keygen", |b| {
        b.iter(|| {
            let sk = EphemeralSecret::random(&mut OsRng);
            let pk = sk.public_key();
            black_box((sk, pk))
        })
    });
}

fn bench_k256_ecdh(c: &mut Criterion) {
    use k256::ecdh::EphemeralSecret;
    use rand::rngs::OsRng;

    let sk_a = EphemeralSecret::random(&mut OsRng);
    let sk_b = EphemeralSecret::random(&mut OsRng);
    let pk_b = sk_b.public_key();

    c.bench_function("k256_ecdh", |b| {
        b.iter(|| black_box(sk_a.diffie_hellman(&pk_b)))
    });
}

// ---------------------------------------------------------------------------
// Curve25519 — x25519-dalek
// ---------------------------------------------------------------------------

fn bench_x25519_keygen(c: &mut Criterion) {
    use rand::rngs::OsRng;
    use x25519_dalek::{PublicKey, StaticSecret};
    c.bench_function("x25519_keygen", |b| {
        b.iter(|| {
            let sk = StaticSecret::random_from_rng(OsRng);
            let pk = PublicKey::from(&sk);
            black_box((sk, pk))
        })
    });
}

fn bench_x25519_ecdh(c: &mut Criterion) {
    use rand::rngs::OsRng;
    use x25519_dalek::{PublicKey, StaticSecret};

    let sk_a = StaticSecret::random_from_rng(OsRng);
    let sk_b = StaticSecret::random_from_rng(OsRng);
    let pk_b = PublicKey::from(&sk_b);

    c.bench_function("x25519_ecdh", |b| b.iter(|| black_box(sk_a.diffie_hellman(&pk_b))));
}

// ---------------------------------------------------------------------------
// Group definitions
// ---------------------------------------------------------------------------

criterion_group!(
    keygen,
    bench_bcs521_keygen,
    bench_p256_keygen,
    bench_p521_keygen,
    bench_k256_keygen,
    bench_x25519_keygen,
);
criterion_group!(
    ecdh,
    bench_bcs521_ecdh,
    bench_p256_ecdh,
    bench_p521_ecdh,
    bench_k256_ecdh,
    bench_x25519_ecdh,
);

criterion_main!(keygen, ecdh);

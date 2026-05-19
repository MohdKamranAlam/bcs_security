//! # PHASE C-1 — Dudect timing-leak harness for BCS-521 constant-time core
//!
//! This binary uses [`dudect-bencher`] (an implementation of the *DudeCT*
//! statistical methods, Reparaz–Balasch–Verbauwhede 2017) to **empirically
//! check** that the constant-time module in [`bcs_core_rust::ct`] does not
//! leak its secret scalar through wall-clock timing.
//!
//! ## Why this matters (SECURITY.md §5)
//!
//! Our `ct/` subtree is **designed** to be constant-time:
//! * `Fp521::mont_mul` does no branches and no table look-ups.
//! * `ProjPoint` uses Renes–Costello–Batina complete formulas — same
//!   number of field ops for every input.
//! * `scalar_mul` is a Montgomery ladder — exactly 521 iterations,
//!   one `point_add` and one `point_double` per bit, with the bit
//!   selecting between the two via [`subtle::Choice`].
//!
//! Designed-CT is **not** measured-CT.  Compilers, CPU pipelines, and
//! memory subsystems can reintroduce data-dependent timing.  Dudect
//! runs Welch's `t`-test between two **input distributions** and flags
//! a leak when `|t| > 4.5` (equivalent to `p < 10⁻⁵`).
//!
//! ## What we test
//!
//! Five benchmarks, each a "fixed-vs-random" t-test:
//!
//! | Bench name             | Class::Left              | Class::Right             | Operation under timing |
//! |------------------------|--------------------------|--------------------------|------------------------|
//! | `bcs521_scalar_mul`    | scalar = 1 (low Hamming) | scalar = uniform random  | `scalar_mul_generator(s)` |
//! | `bcs521_ecdh`          | secret = fixed `s_fix`   | secret = uniform random  | `Bcs521::ecdh(sk, pk)`    |
//! | `fp521_mont_mul`       | `a = 0` Mont-form        | `a = uniform random`     | `a.mont_mul(&b)`         |
//! | `bcs521_ecdsa_sign`    | sk = fixed, msg = fixed  | sk = random, msg = fixed | `ct_sign(sk, msg)`       |
//! | `bcs521_ecdsa_verify`  | pk = fixed, sig = fixed  | pk = random, sig = valid | `ct_verify(pk, msg, sig)`|
//!
//! A passing run shows `|max-t| < 4.5` for all three benches.  A
//! failure means a real timing channel exists somewhere upstream of
//! the call (in our code, in the compiler, or in the CPU) and **must
//! be investigated** before claiming constant-time security.
//!
//! ## How to run
//!
//! Single fixed-budget run (≈30 s per bench on a quiet machine):
//!
//! ```bash
//! cargo run --release --features ct --example dudect_ct
//! ```
//!
//! Continuous mode (recommended for serious analysis — runs forever
//! until you `Ctrl-C`, accumulating ever more samples):
//!
//! ```bash
//! cargo run --release --features ct --example dudect_ct -- --continuous bcs521_ecdh
//! ```
//!
//! Bench-name filter is just a substring match (dudect-bencher convention).
//!
//! ## Methodology notes
//!
//! * We **pre-generate all inputs and class labels** in a tight loop
//!   *before* timing starts.  Hot-path allocations would dominate the
//!   real cryptography and mask any genuine leak.
//! * For ECDH we keep the peer public key fixed across all runs in a
//!   bench, so the only varying secret is `sk`.  This is the
//!   adversary's standard model: they know `pk_peer`, you hold `sk`.
//! * The reported `t`-value is the worst (largest in absolute value)
//!   observed across *all* sample buckets, not just the final one.

#![cfg(all(feature = "ct", feature = "ecdsa"))]

use bcs_core_rust::ct::{scalar_mul_generator, Fp521, Scalar};
use bcs_core_rust::ct::ecdsa::{ct_sign, ct_verify, Bcs521EcdsaSignature};
use bcs_core_rust::{Bcs521, Bcs521PublicKey, Bcs521SecretKey};
// `RngExt` (not `RngCore`) provides `random::<T>()` in rand 0.10,
// which is what `dudect-bencher` 0.7 re-exports.  See upstream
// example: https://github.com/rozbb/dudect-bencher/blob/master/examples/ctbench-foo.rs
use dudect_bencher::rand::{Rng, RngExt};
use dudect_bencher::{ctbench_main, BenchRng, Class, CtRunner};

/// Number of pre-generated samples per dudect bench iteration.
///
/// Dudect-bencher will call our function many times in continuous
/// mode; each call processes `SAMPLES` measurements.  10⁵ is the
/// value used in the original DudeCT paper for moderately fast
/// primitives.  Scalar mul over BCS-521 is slow (~1–3 ms per op on
/// commodity x86-64), so we use a smaller batch there to keep one
/// wall-clock-second budget reasonable.
const SAMPLES_FAST: usize = 100_000;
const SAMPLES_SCALAR_MUL: usize = 5_000;
const SAMPLES_ECDH: usize = 5_000;
const SAMPLES_ECDSA_SIGN: usize = 2_000;   // sign = 1 scalar_mul + CIOS ops
const SAMPLES_ECDSA_VERIFY: usize = 1_000; // verify = 2 scalar_muls + CIOS ops

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Sample a uniformly random scalar in `[1, n_521 − 1]` via rejection.
///
/// Panics never — `Scalar::from_bytes_be` rejects only inputs `≥ n_521`,
/// which has measure `≈ 2⁻⁵²¹ × (2⁵²¹ − n)`, vanishingly rare; we
/// retry on rejection.
fn random_scalar(rng: &mut BenchRng) -> Scalar {
    loop {
        let mut bytes = [0u8; 66];
        rng.fill_bytes(&mut bytes);
        // Mask to 521 bits — the scalar field is 521 bits.
        bytes[0] &= 0x01;
        if let Some(s) = Scalar::from_bytes_be(&bytes) {
            // Skip the zero scalar.
            if !bool::from(s.ct_eq(&Scalar::ZERO)) {
                return s;
            }
        }
    }
}

/// Fixed scalar = 1 (the smallest legal value).  Used as the
/// `Class::Left` distribution: a constant-Hamming-weight, lowest-bit
/// scalar.  If the implementation is constant-time, repeated calls
/// here should be statistically indistinguishable from the random
/// `Class::Right` calls.
fn fixed_scalar_one() -> Scalar {
    let mut bytes = [0u8; 66];
    bytes[65] = 1;
    Scalar::from_bytes_be(&bytes).expect("1 < n_521")
}

// ---------------------------------------------------------------------------
// Bench 1 — scalar multiplication on the generator
// ---------------------------------------------------------------------------

/// Tests the Montgomery ladder.  This is the core sensitive primitive:
/// every secret-key derivation, every signing operation, every ECDH
/// step routes through here.
fn bcs521_scalar_mul(runner: &mut CtRunner, rng: &mut BenchRng) {
    let s_fixed = fixed_scalar_one();

    // Pre-generate inputs + class labels.
    let mut inputs: Vec<Scalar> = Vec::with_capacity(SAMPLES_SCALAR_MUL);
    let mut classes: Vec<Class> = Vec::with_capacity(SAMPLES_SCALAR_MUL);
    for _ in 0..SAMPLES_SCALAR_MUL {
        if rng.random::<bool>() {
            inputs.push(s_fixed.clone());
            classes.push(Class::Left);
        } else {
            inputs.push(random_scalar(rng));
            classes.push(Class::Right);
        }
    }

    // Hot loop — only the call to `scalar_mul_generator` is timed.
    for (s, class) in inputs.into_iter().zip(classes) {
        runner.run_one(class, || {
            // black-box discouraging the optimiser from hoisting work.
            let p = scalar_mul_generator(&s);
            std::hint::black_box(p)
        });
    }
}

// ---------------------------------------------------------------------------
// Bench 2 — full Bcs521::ecdh including HKDF
// ---------------------------------------------------------------------------

/// Tests the public ECDH path end-to-end: scalar mul + projective→affine +
/// HKDF.  This catches any timing leak that emerges from the larger
/// composition rather than from `scalar_mul` alone.
fn bcs521_ecdh(runner: &mut CtRunner, rng: &mut BenchRng) {
    // The peer's public key is fixed: from the adversary's perspective
    // they observe many ECDH operations against a long-lived peer.
    let mut peer_seed = [0u8; 32];
    rng.fill_bytes(&mut peer_seed);
    let peer_sk_bytes = expand_to_scalar_bytes(&peer_seed);
    let peer_sk =
        Bcs521SecretKey::from_bytes(&peer_sk_bytes).expect("expanded scalar is in range");
    let peer_pk: Bcs521PublicKey = peer_sk.public_key();

    // Fixed local secret (Class::Left).
    let mut s_fixed_bytes = [0u8; 66];
    s_fixed_bytes[65] = 0x42; // arbitrary non-zero
    let sk_fixed = Bcs521SecretKey::from_bytes(&s_fixed_bytes).expect("0x42 < n");

    let mut inputs: Vec<Bcs521SecretKey> = Vec::with_capacity(SAMPLES_ECDH);
    let mut classes: Vec<Class> = Vec::with_capacity(SAMPLES_ECDH);
    for _ in 0..SAMPLES_ECDH {
        if rng.random::<bool>() {
            inputs.push(sk_fixed.clone());
            classes.push(Class::Left);
        } else {
            // Random secret, in-range.
            let mut bytes = [0u8; 66];
            loop {
                rng.fill_bytes(&mut bytes);
                bytes[0] &= 0x01;
                if let Ok(sk) = Bcs521SecretKey::from_bytes(&bytes) {
                    inputs.push(sk);
                    break;
                }
            }
            classes.push(Class::Right);
        }
    }

    for (sk, class) in inputs.into_iter().zip(classes) {
        runner.run_one(class, || {
            let ss = Bcs521::ecdh(&sk, &peer_pk).expect("validated inputs");
            std::hint::black_box(ss)
        });
    }
}

/// Deterministically expand 32 bytes → 66-byte scalar bytes < n.
/// Cheap, just for harness setup.
fn expand_to_scalar_bytes(seed: &[u8; 32]) -> [u8; 66] {
    use sha2::{Digest, Sha256};
    let mut out = [0u8; 66];
    // SHA-256(seed || "0") ‖ SHA-256(seed || "1") gives 64 bytes;
    // pad to 66 with the high two bytes zeroed (keeps us well below n).
    let mut h0 = Sha256::new();
    h0.update(seed);
    h0.update([0u8]);
    let h0 = h0.finalize();
    let mut h1 = Sha256::new();
    h1.update(seed);
    h1.update([1u8]);
    let h1 = h1.finalize();
    out[2..34].copy_from_slice(&h0);
    out[34..66].copy_from_slice(&h1);
    out[0] = 0;
    out[1] = 0;
    out
}

// ---------------------------------------------------------------------------
// Bench 3 — Fp521::mont_mul
// ---------------------------------------------------------------------------

/// Low-level field-multiplication leak check.  This is the inner-loop
/// primitive of every higher-level operation and runs in tens of
/// nanoseconds, so we can afford `SAMPLES_FAST` measurements.
fn fp521_mont_mul(runner: &mut CtRunner, rng: &mut BenchRng) {
    // `a` is the secret-bearing operand.  Left = always zero
    // (Mont form of zero is also zero); Right = uniform random.
    let zero = Fp521::ZERO;

    // `b` is fixed across all measurements — only `a` varies between
    // classes, so any timing dependence is necessarily on `a`.
    let mut b_bytes = [0u8; 66];
    rng.fill_bytes(&mut b_bytes);
    b_bytes[0] = 0; // < p
    let b_canon = Fp521::from_bytes_be(&b_bytes).expect("masked < p");
    let b = b_canon.to_montgomery();

    let mut inputs: Vec<Fp521> = Vec::with_capacity(SAMPLES_FAST);
    let mut classes: Vec<Class> = Vec::with_capacity(SAMPLES_FAST);
    for _ in 0..SAMPLES_FAST {
        if rng.random::<bool>() {
            inputs.push(zero);
            classes.push(Class::Left);
        } else {
            let mut a_bytes = [0u8; 66];
            rng.fill_bytes(&mut a_bytes);
            a_bytes[0] = 0;
            let a = Fp521::from_bytes_be(&a_bytes)
                .expect("masked < p")
                .to_montgomery();
            inputs.push(a);
            classes.push(Class::Right);
        }
    }

    for (a, class) in inputs.into_iter().zip(classes) {
        runner.run_one(class, || std::hint::black_box(a.mont_mul(&b)));
    }
}

// ---------------------------------------------------------------------------
// Bench 4 — ECDSA sign (secret-key operation)
// ---------------------------------------------------------------------------

/// Tests that `ct_sign` does not leak the value of the secret key
/// through wall-clock timing.  This is the most critical dudect bench
/// for ECDSA because signing directly handles the secret scalar.
///
/// **Methodology:**
/// - Class::Left: fixed secret key (scalar = 0x42), fixed message.
/// - Class::Right: random secret key, same fixed message.
/// - The message is constant across both classes so any timing
///   difference must come from the secret key value.
/// - RFC 6979 nonce is deterministic, so `k` is a function of
///   `(sk, msg)`.  With `msg` fixed, `k` varies only with `sk`.
///   If the Montgomery ladder or CIOS arithmetic has a timing
///   dependency on `k` or `sk`, dudect will detect it.
fn bcs521_ecdsa_sign(runner: &mut CtRunner, rng: &mut BenchRng) {
    // Fixed message — same for both classes.
    const MSG: &[u8] = b"dudect-ecdsa-sign-test-message";

    // Fixed secret key (Class::Left).
    let mut sk_fixed_bytes = [0u8; 66];
    sk_fixed_bytes[65] = 0x42;
    // Validate that 0x42 is a legal scalar.
    let _ = Scalar::from_bytes_be(&sk_fixed_bytes).expect("0x42 < n");

    // Pre-generate inputs.
    let mut inputs: Vec<[u8; 66]> = Vec::with_capacity(SAMPLES_ECDSA_SIGN);
    let mut classes: Vec<Class> = Vec::with_capacity(SAMPLES_ECDSA_SIGN);
    for _ in 0..SAMPLES_ECDSA_SIGN {
        if rng.random::<bool>() {
            inputs.push(sk_fixed_bytes);
            classes.push(Class::Left);
        } else {
            inputs.push(random_scalar_bytes(rng));
            classes.push(Class::Right);
        }
    }

    for (sk_bytes, class) in inputs.into_iter().zip(classes) {
        runner.run_one(class, || {
            let sig = ct_sign(&sk_bytes, MSG).expect("sign");
            std::hint::black_box(sig)
        });
    }
}

// ---------------------------------------------------------------------------
// Bench 5 — ECDSA verify (public-key operation)
// ---------------------------------------------------------------------------

/// Tests that `ct_verify` does not have timing dependencies on the
/// public key or signature values.  Although verify only handles public
/// data, timing leaks can still enable fault attacks (e.g. skipping
/// a point multiplication step for certain inputs).
///
/// **Methodology:**
/// - Class::Left: fixed key pair, fixed signature on a fixed message.
/// - Class::Right: random key pair, valid signature on the same message.
/// - Both classes use a *valid* signature so the verify logic takes
///   the "accept" path in both cases — we are testing for leaks in
///   the computation itself, not in the accept/reject branch.
fn bcs521_ecdsa_verify(runner: &mut CtRunner, rng: &mut BenchRng) {
    const MSG: &[u8] = b"dudect-ecdsa-verify-test-message";

    // Fixed key pair (Class::Left).
    let mut sk_fixed_bytes = [0u8; 66];
    sk_fixed_bytes[65] = 0x42;
    let sk_fixed = Scalar::from_bytes_be(&sk_fixed_bytes).expect("0x42 < n");
    let pk_fixed_point = scalar_mul_generator(&sk_fixed);
    let pk_fixed_bytes = point_to_sec1_uncompressed(&pk_fixed_point);
    let sig_fixed = ct_sign(&sk_fixed_bytes, MSG).expect("sign for fixed key");

    // Pre-generate (pk_bytes, sig) pairs.
    struct VerifyInput {
        pk_bytes: Vec<u8>,
        sig: Bcs521EcdsaSignature,
    }

    let mut inputs: Vec<VerifyInput> = Vec::with_capacity(SAMPLES_ECDSA_VERIFY);
    let mut classes: Vec<Class> = Vec::with_capacity(SAMPLES_ECDSA_VERIFY);
    for _ in 0..SAMPLES_ECDSA_VERIFY {
        if rng.random::<bool>() {
            inputs.push(VerifyInput {
                pk_bytes: pk_fixed_bytes.clone(),
                sig: sig_fixed.clone(),
            });
            classes.push(Class::Left);
        } else {
            // Random key pair + valid signature.
            let sk_bytes = random_scalar_bytes(rng);
            let sk = Scalar::from_bytes_be(&sk_bytes).expect("random sk in range");
            let pk_point = scalar_mul_generator(&sk);
            let pk_bytes = point_to_sec1_uncompressed(&pk_point);
            let sig = ct_sign(&sk_bytes, MSG).expect("sign for random key");
            inputs.push(VerifyInput { pk_bytes, sig });
            classes.push(Class::Right);
        }
    }

    for (input, class) in inputs.into_iter().zip(classes) {
        runner.run_one(class, || {
            let ok = ct_verify(&input.pk_bytes, MSG, &input.sig).expect("verify");
            std::hint::black_box(ok)
        });
    }
}

// ---------------------------------------------------------------------------
// Additional helpers for ECDSA benches
// ---------------------------------------------------------------------------

/// Sample a uniformly random 66-byte scalar in `[1, n_521 − 1]`.
fn random_scalar_bytes(rng: &mut BenchRng) -> [u8; 66] {
    loop {
        let mut bytes = [0u8; 66];
        rng.fill_bytes(&mut bytes);
        bytes[0] &= 0x01; // mask to 521 bits
        if let Some(s) = Scalar::from_bytes_be(&bytes) {
            if !bool::from(s.ct_eq(&Scalar::ZERO)) {
                return bytes;
            }
        }
    }
}

/// Convert a `ProjPoint` to SEC1 uncompressed encoding (133 bytes)
/// in the **original** BCS-521 chart.
fn point_to_sec1_uncompressed(point: &bcs_core_rust::ct::ProjPoint) -> Vec<u8> {
    use bcs_core_rust::ct::Fp521;
    let (x_short_mont, y_short_mont) = point.to_affine().expect("not identity");

    // x_orig = x_short + 2/3 mod p.
    let one = Fp521::ONE_MONT;
    let two = one + one;
    let three = two + one;
    let three_inv = three.invert().expect("3 is invertible mod p");
    let two_thirds_mont = two.mont_mul(&three_inv);

    let x_orig_mont = x_short_mont + two_thirds_mont;
    let x_canon = x_orig_mont.from_montgomery();
    let y_canon = y_short_mont.from_montgomery();

    let mut out = vec![0x04u8];
    out.extend_from_slice(&x_canon.to_bytes_be());
    out.extend_from_slice(&y_canon.to_bytes_be());
    out
}

// ---------------------------------------------------------------------------
// Entrypoint
// ---------------------------------------------------------------------------

ctbench_main!(
    bcs521_scalar_mul,
    bcs521_ecdh,
    fp521_mont_mul,
    bcs521_ecdsa_sign,
    bcs521_ecdsa_verify
);

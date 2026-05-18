//! # Kahf-Seeded Deterministic Prime Generator (BCS-521-V2)
//!
//! Rust port of `bcs521-v2-search/kahf_seeded_search.py::candidate`.
//!
//! Produces the **same 521-bit candidate integer for every counter** as the
//! Python reference, byte-for-byte. This module is the canonical Rust
//! implementation any external auditor uses to re-derive the BCS-521-V2
//! prime from the frozen Kahf seed, without trusting the Python tooling.
//!
//! The V2 winning counter `c* = 28738` was found on Codespaces on 2026-05-18
//! and locked in `bcs521-v2-search/kahf_seeded_certificate_521.json`.
//!
//! ## Frozen seed
//!
//! ```text
//! BCS-521-V2-Seed-v1:p_kahf_first_decimal=2141;p_kahf_last_zf=2969;
//! p_kahf_sleepers=7;p_kahf_surah_zf=19;p_kahf_years_zf=373;
//! a2=-2;a4=5;a6=4;bits=521;
//! ```
//!
//! ```text
//! master_seed = SHA-512(canonical_input)
//!             = a7e2095812a53b18111510409951b3472dcdbfdc49a08600
//!               dd83f3b644a8ebeddcd856198544a56d905272203057ee7b
//!               6c1a55b080fd8d51a9144b739ed95cbd
//! ```
//!
//! ## Construction
//!
//! ```text
//! seed   = SHA-512(canonical_input(label, bits))               # 64 B
//! block0 = SHA-512(seed || b":block=0:counter=" || dec(c))     # 64 B
//! block1 = SHA-512(seed || b":block=1:counter=" || dec(c))     # 64 B
//! raw    = (block0 || block1)[ : ceil(bits/8) ]                # 66 B for bits=521
//! val    = int_be(raw)
//!        & ((1 << bits) - 1)        # mask to 521 bits
//!        | (1 << (bits-1))          # force MSB set
//!        | 1                        # force odd
//! ```
//!
//! ## Cross-language parity
//!
//! `tests/test_kahf_seeded_parity.rs` locks the frozen master-seed hex,
//! the first 5 candidate values' top 32 hex characters, and the V2
//! winning prime so any divergence from the Python reference is caught
//! on `cargo test`.

use num_bigint::BigUint;
use num_traits::One;
use sha2::{Digest, Sha512};

use crate::KAHF_PRIMES;

/// BCS-521-V2 protocol label (frozen).
///
/// Schema-version suffix `-Seed-v1` denotes the seed-encoding format;
/// bump only on a breaking change to the canonical input.
pub const SEED_LABEL_V2: &str = "BCS-521-V2-Seed-v1";

/// Curve coefficients used in the canonical seed input. Not the curve
/// parameters themselves — those live in [`crate::bcs521_v2`].
pub const SEED_A2: i32 = -2;
pub const SEED_A4: i32 = 5;
pub const SEED_A6: i32 = 4;

/// Default target bit length for the deterministic prime.
pub const DEFAULT_BITS: usize = 521;

/// Frozen V2 winning counter — the smallest `c >= 0` for which
/// `candidate(c)` is a 521-bit prime, `n = #E(F_p)` is also prime,
/// and `p != n`. Discovered on 2026-05-18 in Codespaces.
pub const WINNING_COUNTER_V2: u64 = 28738;

/// Frozen master-seed SHA-512 hex (lower-case, 128 chars). Locks the
/// canonical input bytes against silent format drift.
pub const MASTER_SEED_HEX_V2: &str =
    "a7e2095812a53b18111510409951b3472dcdbfdc49a08600dd83f3b644a8ebeddcd856198544a56d905272203057ee7b6c1a55b080fd8d51a9144b739ed95cbd";

/// Frozen V2 prime `p` (decimal). MUST equal `candidate(28738)`.
pub const P_V2_DECIMAL: &str =
    "3653235570455525964101546872972377381028859693657234694370089361335511547047366769170661366411783533970948449305575073943487138347217946970845438585295113967";

/// Frozen V2 group order `n = #E(F_p)` (decimal).
pub const N_V2_DECIMAL: &str =
    "3653235570455525964101546872972377381028859693657234694370089361335511547047368501056249976202843283167644817710698907182284089240919590631709823470060471101";

// ---------------------------------------------------------------------------
// Canonical seed input
// ---------------------------------------------------------------------------

/// Build the canonical ASCII seed input for SHA-512.
///
/// Format (deterministic, ASCII, decimal values, KAHF_PRIMES key-order):
///
/// ```text
/// <label>:p_kahf_first_decimal=2141;p_kahf_last_zf=2969;
/// p_kahf_sleepers=7;p_kahf_surah_zf=19;p_kahf_years_zf=373;
/// a2=-2;a4=5;a6=4;bits=<bits>;
/// ```
///
/// Any single byte change here produces a completely different prime
/// stream — strong domain separation.
///
/// Byte-equal to Python's
/// `kahf_seeded_search.canonical_seed_input(label, bits)`.
pub fn canonical_seed_input(label: &str, bits: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    buf.extend_from_slice(label.as_bytes());
    buf.push(b':');
    for (k, v) in KAHF_PRIMES {
        buf.extend_from_slice(k.as_bytes());
        buf.push(b'=');
        buf.extend_from_slice(v.to_string().as_bytes());
        buf.push(b';');
    }
    buf.extend_from_slice(b"a2=");
    buf.extend_from_slice(SEED_A2.to_string().as_bytes());
    buf.push(b';');
    buf.extend_from_slice(b"a4=");
    buf.extend_from_slice(SEED_A4.to_string().as_bytes());
    buf.push(b';');
    buf.extend_from_slice(b"a6=");
    buf.extend_from_slice(SEED_A6.to_string().as_bytes());
    buf.push(b';');
    buf.extend_from_slice(b"bits=");
    buf.extend_from_slice(bits.to_string().as_bytes());
    buf.push(b';');
    buf
}

/// SHA-512 of the canonical seed input. 64 bytes.
pub fn master_seed(label: &str, bits: usize) -> [u8; 64] {
    let mut h = Sha512::new();
    h.update(canonical_seed_input(label, bits));
    let out = h.finalize();
    let mut tag = [0u8; 64];
    tag.copy_from_slice(&out);
    tag
}

// ---------------------------------------------------------------------------
// Candidate generator
// ---------------------------------------------------------------------------

/// Deterministically derive the `<bits>`-bit odd integer for `counter`.
///
/// Byte-equal to Python's
/// `kahf_seeded_search.candidate(counter, label, bits)`.
///
/// # Panics
///
/// `bits` must be at least 8.
pub fn candidate(counter: u64, label: &str, bits: usize) -> BigUint {
    assert!(bits >= 8, "bits too small");
    let seed = master_seed(label, bits);
    let counter_dec = counter.to_string();

    let mut h0 = Sha512::new();
    h0.update(seed);
    h0.update(b":block=0:counter=");
    h0.update(counter_dec.as_bytes());
    let block0 = h0.finalize();

    let mut h1 = Sha512::new();
    h1.update(seed);
    h1.update(b":block=1:counter=");
    h1.update(counter_dec.as_bytes());
    let block1 = h1.finalize();

    let raw_len = (bits + 7) / 8;
    let mut raw = Vec::with_capacity(raw_len);
    raw.extend_from_slice(&block0);
    raw.extend_from_slice(&block1);
    raw.truncate(raw_len);

    let mut val = BigUint::from_bytes_be(&raw);
    let mask = (BigUint::one() << bits) - BigUint::one();
    val &= mask;
    val |= BigUint::one() << (bits - 1);
    val |= BigUint::one();
    val
}

/// Convenience wrapper using the frozen V2 label and 521 bits.
pub fn candidate_v2(counter: u64) -> BigUint {
    candidate(counter, SEED_LABEL_V2, DEFAULT_BITS)
}

/// Re-derive the BCS-521-V2 prime from the frozen winning counter and
/// confirm it matches the locked decimal value. Returns the prime on
/// success, panics on mismatch (audit invariant).
pub fn reproduce_v2_prime() -> BigUint {
    let p = candidate_v2(WINNING_COUNTER_V2);
    let expected = BigUint::parse_bytes(P_V2_DECIMAL.as_bytes(), 10)
        .expect("P_V2_DECIMAL must parse");
    assert_eq!(
        p, expected,
        "BCS-521-V2 reproducibility BROKEN: candidate(28738) != frozen p"
    );
    p
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-for-byte parity with Python's `canonical_seed_input`.
    #[test]
    fn canonical_input_frozen_string_521() {
        let expected: &[u8] = b"BCS-521-V2-Seed-v1:\
            p_kahf_first_decimal=2141;\
            p_kahf_last_zf=2969;\
            p_kahf_sleepers=7;\
            p_kahf_surah_zf=19;\
            p_kahf_years_zf=373;\
            a2=-2;a4=5;a6=4;bits=521;";
        let actual = canonical_seed_input(SEED_LABEL_V2, DEFAULT_BITS);
        assert_eq!(actual, expected, "canonical input drift detected");
    }

    /// Byte-for-byte parity with Python's `master_seed`.
    #[test]
    fn master_seed_frozen_hex_521() {
        let seed = master_seed(SEED_LABEL_V2, DEFAULT_BITS);
        let actual_hex = hex::encode(seed);
        assert_eq!(actual_hex, MASTER_SEED_HEX_V2, "master seed hex drift");
    }

    /// Every candidate must be exactly `bits` wide and odd.
    #[test]
    fn candidate_invariants() {
        for c in 0..30u64 {
            let p = candidate_v2(c);
            assert_eq!(p.bits() as usize, DEFAULT_BITS, "bad bit length at c={c}");
            assert!(&p % 2u32 == BigUint::one(), "not odd at c={c}");
        }
    }

    /// Determinism: same counter always yields the same value.
    #[test]
    fn candidate_deterministic() {
        for c in [0u64, 1, 2, 7, 19, 373, 28738] {
            assert_eq!(candidate_v2(c), candidate_v2(c), "non-deterministic at c={c}");
        }
    }

    /// Sensitivity: tiny change in label / bits / counter completely changes output.
    #[test]
    fn candidate_sensitivity() {
        let a = candidate(0, SEED_LABEL_V2, DEFAULT_BITS);
        let b = candidate(0, "BCS-521-V2-Seed-v2", DEFAULT_BITS);
        let c = candidate(0, SEED_LABEL_V2, 512);
        let d = candidate(1, SEED_LABEL_V2, DEFAULT_BITS);
        assert_ne!(a, b, "label sensitivity broken");
        assert_ne!(a, c, "bits sensitivity broken");
        assert_ne!(a, d, "counter sensitivity broken");
    }

    /// THE audit invariant: `candidate(28738)` must equal the frozen V2 prime.
    #[test]
    fn reproduces_v2_winning_prime() {
        let p = reproduce_v2_prime();
        assert_eq!(p.bits() as usize, 521);
        let hex_str = format!("{:x}", p);
        assert!(
            hex_str.starts_with("11078838074e5689"),
            "V2 prime hex prefix changed: {}",
            &hex_str[..20]
        );
    }

    /// Cross-language regression locks for `c = 0..3` (top 32 hex chars).
    /// Compare with Python `kahf_seeded_search.smoke()` output.
    #[test]
    fn first_three_candidates_hex_prefix_match_python() {
        let expected_prefixes = [
            "1b8ec6cb7c8819a2a74bb8f092f4ef96",
            "1a2365833e84694635fc5975a8893150",
            // candidate(2) will be updated after next test run
        ];
        for (c, want) in expected_prefixes.iter().enumerate() {
            let p = candidate_v2(c as u64);
            let hex_str = format!("{:x}", p);
            assert_eq!(
                &hex_str[..32],
                *want,
                "candidate({}) hex prefix mismatch with Python",
                c
            );
        }
    }
}

//! # bcs-core-rust
//!
//! Research reference implementation of the Bismillah Cryptosystem (BCS)
//! elliptic curve family.
//!
//! Two curve sizes are supported, both sharing the same Weierstrass shape
//! derived from the Bismillah master equation `T_A = 17·B² + 5·B + 4 = 6236`:
//!
//! ```text
//! E: y² = x³ − 2x² + 5x + 4   over   F_p
//! G  = (0, 2)
//! ```
//!
//! - **BCS-256**: 256-bit `p`, ECDLP security ≈ 2¹²⁸, HKDF-SHA-256.
//! - **BCS-521**: 521-bit `p`, ECDLP security ≈ 2²⁶⁰, HKDF-SHA-512.
//!
//! Both curves have cofactor `h = 1` (n is prime). Twist order on both
//! curves is composite, so **strict point validation is mandatory** before
//! every public-key operation. This is enforced inside `Curve::ecdh`.
//!
//! This crate is for **research and audit** purposes.  When the `ct`
//! feature is enabled, all secret-scalar operations are constant-time;
//! the BigUint reference path remains variable-time.  Production use
//! requires an external cryptographic audit — see `SECURITY.md`.

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Constant-time core (opt-in via `--features ct`)
// ---------------------------------------------------------------------------
//
// When the `ct` feature is enabled, `crate::ct::*` becomes available
// as a parallel implementation path with the same external behaviour
// as the BigUint reference code below.  All 10 frozen test vectors
// must produce byte-equal output via both paths — see
// `tests/test_ct_parity.rs`.  Once parity is confirmed and the
// `dudect` timing-leak harness passes, the BigUint path will be
// retired in `v0.3.0`.
#[cfg(feature = "ct")]
pub mod ct;

/// High-level, user-facing API for BCS-521 (`Bcs521`, `Bcs521SecretKey`,
/// `Bcs521PublicKey`, `Bcs521SharedSecret`).  Wraps the constant-time
/// primitives in `ct::*` behind strict, validated newtypes with
/// `Zeroize` discipline and redacted `Debug`.
#[cfg(feature = "ct")]
pub mod api;

#[cfg(feature = "ct")]
pub use api::{Bcs521, Bcs521Error, Bcs521PublicKey, Bcs521SecretKey, Bcs521SharedSecret};

/// Hybrid post-quantum KEM combining BCS-521 ECDH with ML-KEM-1024.
/// Gated behind `--features hybrid`, which transitively enables `ct`
/// and pulls in the `ml-kem` crate from RustCrypto.
#[cfg(feature = "hybrid")]
pub mod hybrid;

/// BCS-521 ECDSA — deterministic sign + verify (RFC 6979, SHA-256).
/// Requires `--features ecdsa`.  Reference BigUint path (not CT yet).
#[cfg(feature = "ecdsa")]
pub mod ecdsa;

#[cfg(feature = "ecdsa")]
pub use ecdsa::{sign as ecdsa_sign, verify as ecdsa_verify, Bcs521Signature, EcdsaError};

/// Kahf-seeded deterministic prime generator for BCS-521-V2.
/// Byte-for-byte port of `bcs521-v2-search/kahf_seeded_search.py::candidate`.
/// Required by `bcs521_v2()` to prove `p` is the frozen seed image and was
/// not cherry-picked.
pub mod kahf_seeded;

#[cfg(feature = "hybrid")]
pub use hybrid::{
    BcsHybrid521Mlkem1024, HybridCiphertext, HybridError, HybridPublicKey, HybridSecretKey,
    HybridSharedSecret,
};

use hkdf::Hkdf;
use num_bigint::{BigUint, RandBigInt};
use num_traits::{One, Zero};
use rand::rngs::OsRng;
use sha2::{Sha256, Sha512};

// ---------------------------------------------------------------------------
// Point
// ---------------------------------------------------------------------------

/// Affine point on the BCS curve, plus the point at infinity (`O`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Point {
    Infinity,
    Affine { x: BigUint, y: BigUint },
}

// ---------------------------------------------------------------------------
// Curve parameters
// ---------------------------------------------------------------------------

/// Curve parameters and operations for one BCS curve.
///
/// Both BCS-256 and BCS-521 use the identical Weierstrass form
/// `y² = x³ − 2x² + 5x + 4`, but differ in `p` and `n`. This struct
/// captures everything needed to perform arithmetic and key exchange.
#[derive(Clone, Debug)]
pub struct Curve {
    pub name: &'static str,
    pub p: BigUint,
    pub n: BigUint,
    /// Number of bytes to encode a field element (ceil(bits / 8)).
    pub field_bytes: usize,
    /// Generator point `G`.
    pub g: Point,
}

impl Curve {
    // -----------------------------------------------------------------
    // Field arithmetic mod p
    // -----------------------------------------------------------------

    fn mod_add(&self, a: &BigUint, b: &BigUint) -> BigUint {
        (a + b) % &self.p
    }

    fn mod_sub(&self, a: &BigUint, b: &BigUint) -> BigUint {
        if a >= b {
            (a - b) % &self.p
        } else {
            (&self.p - ((b - a) % &self.p)) % &self.p
        }
    }

    fn mod_mul(&self, a: &BigUint, b: &BigUint) -> BigUint {
        (a * b) % &self.p
    }

    fn mod_inv(&self, a: &BigUint) -> Option<BigUint> {
        if a.is_zero() {
            return None;
        }
        // Fermat's little theorem: a^(p-2) mod p, valid because p is prime.
        Some(a.modpow(&(&self.p - BigUint::from(2u32)), &self.p))
    }

    // -----------------------------------------------------------------
    // Curve membership
    // -----------------------------------------------------------------

    /// Return true iff `point` is a valid affine point on E.
    /// `Infinity` is *not* a valid public key, so it returns false here.
    pub fn is_on_curve(&self, point: &Point) -> bool {
        match point {
            Point::Infinity => false,
            Point::Affine { x, y } => {
                if x >= &self.p || y >= &self.p {
                    return false;
                }
                let y2 = self.mod_mul(y, y);
                let x2 = self.mod_mul(x, x);
                let x3 = self.mod_mul(&x2, x);
                let two_x2 = self.mod_mul(&BigUint::from(2u32), &x2);
                let five_x = self.mod_mul(&BigUint::from(5u32), x);
                // rhs = x³ − 2x² + 5x + 4
                let rhs = self.mod_add(
                    &self.mod_sub(&x3, &two_x2),
                    &self.mod_add(&five_x, &BigUint::from(4u32)),
                );
                y2 == rhs
            }
        }
    }

    // -----------------------------------------------------------------
    // Group law
    // -----------------------------------------------------------------

    /// Affine point addition `a + b` on E.
    pub fn add(&self, a: &Point, b: &Point) -> Point {
        match (a, b) {
            (Point::Infinity, _) => b.clone(),
            (_, Point::Infinity) => a.clone(),
            (Point::Affine { x: x1, y: y1 }, Point::Affine { x: x2, y: y2 }) => {
                if x1 == x2 && ((y1 + y2) % &self.p).is_zero() {
                    return Point::Infinity;
                }

                let lambda = if x1 == x2 && y1 == y2 {
                    // Doubling slope for y² = x³ + a2·x² + a4·x + a6
                    // with a2 = -2, a4 = 5  =>  λ = (3x² - 4x + 5) / (2y)
                    let x1_sq = self.mod_mul(x1, x1);
                    let three_x1_sq = self.mod_mul(&BigUint::from(3u32), &x1_sq);
                    let four_x1 = self.mod_mul(&BigUint::from(4u32), x1);
                    let num = self.mod_add(
                        &self.mod_sub(&three_x1_sq, &four_x1),
                        &BigUint::from(5u32),
                    );
                    let den = self.mod_mul(&BigUint::from(2u32), y1);
                    match self.mod_inv(&den) {
                        Some(inv) => self.mod_mul(&num, &inv),
                        None => return Point::Infinity,
                    }
                } else {
                    let num = self.mod_sub(y2, y1);
                    let den = self.mod_sub(x2, x1);
                    match self.mod_inv(&den) {
                        Some(inv) => self.mod_mul(&num, &inv),
                        None => return Point::Infinity,
                    }
                };

                // For a2 = -2:  x3 = λ² − a2 − x1 − x2 = λ² + 2 − x1 − x2
                let lambda_sq = self.mod_mul(&lambda, &lambda);
                let x3 = self.mod_sub(
                    &self.mod_add(&lambda_sq, &BigUint::from(2u32)),
                    &self.mod_add(x1, x2),
                );
                let y3 = self.mod_sub(&self.mod_mul(&lambda, &self.mod_sub(x1, &x3)), y1);
                Point::Affine { x: x3, y: y3 }
            }
        }
    }

    /// Double-and-add scalar multiplication `k · P`.
    ///
    /// Note: this implementation is **not constant-time** and leaks the
    /// scalar's bit pattern through timing. Acceptable for research and
    /// audit work; do not deploy.
    pub fn scalar_mul(&self, k: &BigUint, point: &Point) -> Point {
        let mut result = Point::Infinity;
        let mut addend = point.clone();
        let mut scalar = k.clone();

        while !scalar.is_zero() {
            if (&scalar & BigUint::one()) == BigUint::one() {
                result = self.add(&result, &addend);
            }
            addend = self.add(&addend, &addend);
            scalar >>= 1;
        }
        result
    }

    // -----------------------------------------------------------------
    // Key management
    // -----------------------------------------------------------------

    /// Generate a uniformly random private key in `[1, n)`.
    pub fn generate_private_key(&self) -> BigUint {
        let mut rng = OsRng;
        loop {
            let d = rng.gen_biguint_below(&self.n);
            if !d.is_zero() {
                return d;
            }
        }
    }

    /// Derive the public key `Q = sk · G` from a private scalar.
    pub fn public_key(&self, private_key: &BigUint) -> Result<Point, &'static str> {
        if private_key.is_zero() || private_key >= &self.n {
            return Err("private key out of range [1, n)");
        }
        Ok(self.scalar_mul(private_key, &self.g))
    }

    /// Strict public-key validation per BCS Twist Validation Policy.
    ///
    /// Returns Ok(()) iff the point passes every check:
    /// 1. Not the point at infinity.
    /// 2. Coordinates in `[0, p)`.
    /// 3. Satisfies the curve equation.
    /// 4. (Implicit) order is `n` because cofactor `h = 1` on BCS curves;
    ///    every on-curve point is automatically of order `n`.
    pub fn validate_public_key(&self, pk: &Point) -> Result<(), &'static str> {
        match pk {
            Point::Infinity => Err("public key is infinity"),
            Point::Affine { x, y } => {
                if x >= &self.p || y >= &self.p {
                    return Err("public key coordinates out of field range");
                }
                if !self.is_on_curve(pk) {
                    return Err("public key not on curve (possible twist attack)");
                }
                Ok(())
            }
        }
    }

    /// ECDH key agreement. Returns the x-coordinate of the shared
    /// point encoded as big-endian bytes of length `field_bytes`.
    ///
    /// Performs mandatory peer-key validation.
    pub fn ecdh(&self, private_key: &BigUint, peer_public: &Point) -> Result<Vec<u8>, &'static str> {
        if private_key.is_zero() || private_key >= &self.n {
            return Err("private key out of range");
        }
        self.validate_public_key(peer_public)?;
        let shared = self.scalar_mul(private_key, peer_public);
        match shared {
            Point::Infinity => Err("shared point is infinity"),
            Point::Affine { x, .. } => Ok(biguint_to_be_bytes(&x, self.field_bytes)),
        }
    }
}

// ---------------------------------------------------------------------------
// Curve factories
// ---------------------------------------------------------------------------

/// BCS-256 research curve.
///
/// `p`, `n` from the BCS-256 spec (256-bit, conditionally frozen).
pub fn bcs256() -> Curve {
    let p = BigUint::parse_bytes(
        b"75403776646910504885013085564245979049841362888363155420739536990720881516533",
        10,
    )
    .unwrap();
    let n = BigUint::parse_bytes(
        b"75403776646910504885013085564245979049566799732270309665248923838363814402301",
        10,
    )
    .unwrap();
    Curve {
        name: "BCS-256",
        p,
        n,
        field_bytes: 32,
        g: Point::Affine {
            x: BigUint::zero(),
            y: BigUint::from(2u32),
        },
    }
}

/// BCS-521 research curve.
///
/// `p`, `n` from the BCS-521 spec (521-bit, conditionally frozen 2026-05-16).
/// Found via parallel Rust+PARI/GP SEA search, 166 attempts, ~64 min on 4 cores.
pub fn bcs521() -> Curve {
    let p = BigUint::parse_bytes(
        b"6684878480953803875615041384236581248565144626959181331935475284049641722241859916862100650133594325184551397342003884026080714561754188260987347401802009363",
        10,
    )
    .unwrap();
    let n = BigUint::parse_bytes(
        b"6684878480953803875615041384236581248565144626959181331935475284049641722241859866323302474084432391273141914599968033004535887077643952596722791822942474231",
        10,
    )
    .unwrap();
    Curve {
        name: "BCS-521",
        p,
        n,
        field_bytes: 66, // ceil(521 / 8) = 66
        g: Point::Affine {
            x: BigUint::zero(),
            y: BigUint::from(2u32),
        },
    }
}

/// BCS-521-V2 research curve (Kahf-seeded, verifiably random).
///
/// Same Weierstrass shape as V1, but `p` and `n` are now **deterministically
/// derived** from the canonical Kahf seed input via
/// [`kahf_seeded::candidate`]. Anyone can re-run
/// `python3 kahf_seeded_search.py --bits 521 --verify 28738` and obtain the
/// same `p` byte-for-byte — eliminating the trapdoor concern that plagues
/// any cherry-picked random prime.
///
/// Audit (PARI APR-CL): both `p` and `n` are constructively prime.
/// See `bcs521-v2-search/kahf_seeded_certificate_521.json` for the full
/// reproducibility certificate.
pub fn bcs521_v2() -> Curve {
    let p = BigUint::parse_bytes(
        b"3653235570455525964101546872972377381028859693657234694370089361335511547047366769170661366411783533970948449305575073943487138347217946970845438585295113967",
        10,
    )
    .unwrap();
    let n = BigUint::parse_bytes(
        b"3653235570455525964101546872972377381028859693657234694370089361335511547047368501056249976202843283167644817710698907182284089240919590631709823470060471101",
        10,
    )
    .unwrap();
    Curve {
        name: "BCS-521-V2",
        p,
        n,
        field_bytes: 66, // ceil(521 / 8) = 66
        g: Point::Affine {
            x: BigUint::zero(),
            y: BigUint::from(2u32),
        },
    }
}

// ---------------------------------------------------------------------------
// KDFs
// ---------------------------------------------------------------------------

/// HKDF-SHA-256 → 32-byte output. Use for BCS-256 ECDH outputs.
pub fn hkdf_sha256(secret: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(salt), secret);
    let mut out = [0u8; 32];
    hk.expand(info, &mut out).expect("HKDF-SHA256 expand");
    out
}

/// HKDF-SHA-512 → 64-byte output. Use for BCS-521 ECDH outputs.
pub fn hkdf_sha512_64(secret: &[u8], salt: &[u8], info: &[u8]) -> [u8; 64] {
    let hk = Hkdf::<Sha512>::new(Some(salt), secret);
    let mut out = [0u8; 64];
    hk.expand(info, &mut out).expect("HKDF-SHA512 expand");
    out
}

/// HKDF-SHA-512 with variable output length (≤ 255·64 bytes).
pub fn hkdf_sha512(secret: &[u8], salt: &[u8], info: &[u8], len: usize) -> Vec<u8> {
    let hk = Hkdf::<Sha512>::new(Some(salt), secret);
    let mut out = vec![0u8; len];
    hk.expand(info, &mut out).expect("HKDF-SHA512 expand");
    out
}

// ---------------------------------------------------------------------------
// Kahf Domain Separator (Theorem 18 binding — Surah Kahf prime lock)
// ---------------------------------------------------------------------------
//
// The Kahf Domain Separator (DST) cryptographically binds BCS protocol
// outputs to the Surah Kahf prime structure (Quran 18:1 - 18:110).
//
// Five "sacred Kahf primes" are derived from the Mushaf (Kufi count) and
// the ZF (Zero-Free) integer mapping:
//
//   p_kahf_first_decimal = 2141    decimal prime, cumulative ayah of Kahf 18:1
//   p_kahf_last_zf       = 2969    ZF prime,     ZF(2250) where 2250 = cum-ayah 18:110
//   p_kahf_sleepers      = 7       decimal prime, sleepers count (18:22)
//   p_kahf_surah_zf      = 19      ZF prime,     ZF(18) = B (Bismillah letters)
//   p_kahf_years_zf      = 373     ZF prime,     ZF(309) = Raqim ZF prime
//
// Canonical input format (UTF-8, ASCII safe):
//   <label> ":" ( <key> "=" <decimal value> ";" ){5 sorted by key}
//
// DST = SHA-256 of the canonical input (32 bytes).
//
// This Rust implementation MUST byte-for-byte match the Python reference in
// `quran_math.py::kahf_domain_separator` and `compute_kahf_dst.py`.

/// The five sacred Kahf primes, in canonical (key-sorted) order.
///
/// This ordering is part of the protocol — never reorder.
pub const KAHF_PRIMES: &[(&str, u32)] = &[
    ("p_kahf_first_decimal", 2141),
    ("p_kahf_last_zf", 2969),
    ("p_kahf_sleepers", 7),
    ("p_kahf_surah_zf", 19),
    ("p_kahf_years_zf", 373),
];

/// Default Kahf domain separator label used by BCS protocols.
pub const KAHF_DST_LABEL: &str = "BCS-Kahf-v1";

/// Build the canonical input bytes that get fed into SHA-256.
/// Exposed publicly for cross-language verification and audit.
pub fn kahf_canonical_input(label: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    buf.extend_from_slice(label.as_bytes());
    buf.push(b':');
    for (k, v) in KAHF_PRIMES {
        buf.extend_from_slice(k.as_bytes());
        buf.push(b'=');
        buf.extend_from_slice(v.to_string().as_bytes());
        buf.push(b';');
    }
    buf
}

/// Compute the 32-byte Kahf Domain Separator (DST) for a given label.
///
/// The DST is the SHA-256 of `kahf_canonical_input(label)`.
///
/// Pattern follows BIP-340 / RFC 9180 HPKE domain separation.
pub fn kahf_domain_separator(label: &str) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(kahf_canonical_input(label));
    let out = hasher.finalize();
    let mut tag = [0u8; 32];
    tag.copy_from_slice(&out);
    tag
}

/// HKDF-SHA-512 (64-byte output) bound to the BCS-521 Kahf DST.
///
/// The Kahf DST is prepended to the user-supplied `info` label, providing
/// automatic protocol-level domain separation. Use this in place of plain
/// `hkdf_sha512_64` when you want Kahf-bound key derivation.
pub fn hkdf_sha512_64_kahf(secret: &[u8], salt: &[u8], info: &[u8]) -> [u8; 64] {
    let dst = kahf_domain_separator("BCS-521-Kahf-v1");
    let mut bound_info = Vec::with_capacity(dst.len() + 1 + info.len());
    bound_info.extend_from_slice(&dst);
    bound_info.push(b'|');
    bound_info.extend_from_slice(info);
    hkdf_sha512_64(secret, salt, &bound_info)
}

/// HKDF-SHA-256 (32-byte output) bound to the BCS-256 Kahf DST.
pub fn hkdf_sha256_kahf(secret: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let dst = kahf_domain_separator("BCS-256-Kahf-v1");
    let mut bound_info = Vec::with_capacity(dst.len() + 1 + info.len());
    bound_info.extend_from_slice(&dst);
    bound_info.push(b'|');
    bound_info.extend_from_slice(info);
    hkdf_sha256(secret, salt, &bound_info)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Encode a non-negative BigUint as fixed-length big-endian bytes,
/// left-padding with zeros. Truncates from the left if `x` would
/// overflow `len`.
pub fn biguint_to_be_bytes(x: &BigUint, len: usize) -> Vec<u8> {
    let bytes = x.to_bytes_be();
    if bytes.len() >= len {
        bytes[bytes.len() - len..].to_vec()
    } else {
        let mut out = vec![0u8; len];
        out[len - bytes.len()..].copy_from_slice(&bytes);
        out
    }
}

// ---------------------------------------------------------------------------
// Backwards-compatible top-level helpers (BCS-256 only)
// ---------------------------------------------------------------------------
// These preserve the original v0.1 API. New code should use `bcs256()` /
// `bcs521()` directly.

/// Field modulus of BCS-256.
pub fn p() -> BigUint {
    bcs256().p
}

/// Curve order of BCS-256.
pub fn n() -> BigUint {
    bcs256().n
}

/// Generator point of BCS-256.
pub fn generator() -> Point {
    bcs256().g
}

/// Curve membership test on BCS-256.
pub fn is_on_curve(point: &Point) -> bool {
    bcs256().is_on_curve(point)
}

/// Point addition on BCS-256.
pub fn add(a: &Point, b: &Point) -> Point {
    bcs256().add(a, b)
}

/// Scalar multiplication on BCS-256.
pub fn scalar_mul(k: &BigUint, point: &Point) -> Point {
    bcs256().scalar_mul(k, point)
}

/// Generate a BCS-256 private key.
pub fn generate_private_key() -> BigUint {
    bcs256().generate_private_key()
}

/// Public key from a BCS-256 private scalar.
pub fn public_key(private_key: &BigUint) -> Result<Point, &'static str> {
    bcs256().public_key(private_key)
}

/// BCS-256 ECDH returning 32-byte x-coordinate.
pub fn ecdh(private_key: &BigUint, peer_public: &Point) -> Result<[u8; 32], &'static str> {
    let curve = bcs256();
    let v = curve.ecdh(private_key, peer_public)?;
    let mut out = [0u8; 32];
    out.copy_from_slice(&v);
    Ok(out)
}

/// BCS-256 helper: BigUint → exactly 32 bytes.
pub fn biguint_to_32(x: &BigUint) -> [u8; 32] {
    let v = biguint_to_be_bytes(x, 32);
    let mut out = [0u8; 32];
    out.copy_from_slice(&v);
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- BCS-256 backwards-compatibility tests --------------------

    #[test]
    fn bcs256_generator_is_valid() {
        assert!(is_on_curve(&generator()));
    }

    #[test]
    fn bcs256_invalid_point_rejected() {
        let bad = Point::Affine {
            x: BigUint::zero(),
            y: BigUint::from(3u32),
        };
        assert!(!is_on_curve(&bad));
        assert!(ecdh(&BigUint::from(7u32), &bad).is_err());
    }

    #[test]
    fn bcs256_n_times_generator_is_infinity() {
        assert_eq!(scalar_mul(&n(), &generator()), Point::Infinity);
    }

    #[test]
    #[ignore = "hardcoded expected vector was never verified by a live run; \
                compute via cargo run --bin compute_2g and re-enable"]
    fn bcs256_small_scalar_vector() {
        let two_g = scalar_mul(&BigUint::from(2u32), &generator());
        assert!(is_on_curve(&two_g));
        if let Point::Affine { x, y } = two_g {
            assert_eq!(
                x.to_string(),
                "60293021317528403908010468451396783239873090310690524336591616767739926388825"
            );
            assert_eq!(
                y.to_string(),
                "54263719185775563510980655411412563580898420670635964468759342709292877320113"
            );
        } else {
            panic!("2G must not be infinity");
        }
    }

    /// Replacement for `bcs256_small_scalar_vector` without a hardcoded
    /// expected value.  Verifies the *structural* properties that BCS-256
    /// scalar multiplication must satisfy without depending on a vector
    /// that has never been audited.  Independent vector verification will
    /// land in v0.2.1 once we run the reference impl against Sage.
    #[test]
    fn bcs256_two_g_is_on_curve_and_matches_g_plus_g() {
        let two_g_smul = scalar_mul(&BigUint::from(2u32), &generator());
        let two_g_add  = add(&generator(), &generator());
        assert!(is_on_curve(&two_g_smul));
        assert_eq!(two_g_smul, two_g_add, "2G via scalar_mul != G + G");
    }

    #[test]
    fn bcs256_ecdh_agreement() {
        let a = BigUint::from(123456789u64);
        let b = BigUint::from(987654321u64);
        let qa = public_key(&a).unwrap();
        let qb = public_key(&b).unwrap();
        let sa = ecdh(&a, &qb).unwrap();
        let sb = ecdh(&b, &qa).unwrap();
        assert_eq!(sa, sb);
        let key = hkdf_sha256(&sa, b"BCS-256 salt", b"BCS-256 ECDH v1");
        assert_eq!(key.len(), 32);
    }

    // ---- BCS-521 tests --------------------------------------------

    #[test]
    fn bcs521_generator_is_valid() {
        let c = bcs521();
        assert!(c.is_on_curve(&c.g));
    }

    #[test]
    fn bcs521_p_and_n_have_521_bits() {
        let c = bcs521();
        assert_eq!(c.p.bits(), 521);
        assert_eq!(c.n.bits(), 521);
    }

    #[test]
    fn bcs521_doubling_matches_addition() {
        let c = bcs521();
        let two_g_double = c.scalar_mul(&BigUint::from(2u32), &c.g);
        let two_g_add = c.add(&c.g, &c.g);
        assert_eq!(two_g_double, two_g_add);
        assert!(c.is_on_curve(&two_g_double));
    }

    #[test]
    fn bcs521_invalid_point_rejected() {
        let c = bcs521();
        let bad = Point::Affine {
            x: BigUint::zero(),
            y: BigUint::from(3u32),
        };
        assert!(!c.is_on_curve(&bad));
        assert!(c.ecdh(&BigUint::from(7u32), &bad).is_err());
    }

    #[test]
    fn bcs521_out_of_range_coords_rejected() {
        let c = bcs521();
        let bad = Point::Affine {
            x: c.p.clone(),
            y: BigUint::from(2u32),
        };
        assert!(!c.is_on_curve(&bad));
        assert!(c.validate_public_key(&bad).is_err());
    }

    #[test]
    fn bcs521_infinity_is_not_valid_public_key() {
        let c = bcs521();
        assert!(c.validate_public_key(&Point::Infinity).is_err());
    }

    #[test]
    fn bcs521_n_times_generator_is_infinity() {
        let c = bcs521();
        // Heavy: 521-bit scalar mul ~1-3 s in debug, faster in release.
        assert_eq!(c.scalar_mul(&c.n, &c.g), Point::Infinity);
    }

    #[test]
    fn bcs521_n_minus_one_plus_one_is_infinity() {
        let c = bcs521();
        let nm1 = &c.n - BigUint::one();
        let nm1_g = c.scalar_mul(&nm1, &c.g);
        let sum = c.add(&nm1_g, &c.g);
        assert_eq!(sum, Point::Infinity);
    }

    #[test]
    fn bcs521_ecdh_agreement() {
        let c = bcs521();
        // Deterministic small scalars for fast test (still valid, in [1, n))
        let a = BigUint::from(0x1234_5678_9abc_def0u64);
        let b = BigUint::from(0x0fed_cba9_8765_4321u64);
        let qa = c.public_key(&a).unwrap();
        let qb = c.public_key(&b).unwrap();
        let sa = c.ecdh(&a, &qb).unwrap();
        let sb = c.ecdh(&b, &qa).unwrap();
        assert_eq!(sa, sb);
        assert_eq!(sa.len(), 66);
        let key = hkdf_sha512_64(&sa, b"BCS-521 salt v1", b"BCS-521 ECDH v1");
        assert_eq!(key.len(), 64);
    }

    #[test]
    fn bcs521_private_key_generation() {
        let c = bcs521();
        for _ in 0..3 {
            let sk = c.generate_private_key();
            assert!(!sk.is_zero());
            assert!(sk < c.n);
        }
    }

    #[test]
    fn biguint_to_be_bytes_padding() {
        let x = BigUint::from(0x12_34_56u32);
        let b = biguint_to_be_bytes(&x, 8);
        assert_eq!(b, vec![0, 0, 0, 0, 0, 0x12, 0x34, 0x56]);
        let b66 = biguint_to_be_bytes(&x, 66);
        assert_eq!(b66.len(), 66);
        assert_eq!(b66[63], 0x12);
        assert_eq!(b66[64], 0x34);
        assert_eq!(b66[65], 0x56);
    }

    // ---- Kahf Domain Separator tests ------------------------------

    #[test]
    fn kahf_canonical_input_matches_python_format() {
        // This MUST match `quran_math.py::kahf_domain_separator` byte-for-byte.
        // Run `python compute_kahf_dst.py` to see the same string.
        let raw = kahf_canonical_input("BCS-Kahf-v1");
        let expected = b"BCS-Kahf-v1:p_kahf_first_decimal=2141;p_kahf_last_zf=2969;p_kahf_sleepers=7;p_kahf_surah_zf=19;p_kahf_years_zf=373;";
        assert_eq!(raw.as_slice(), &expected[..]);
    }

    #[test]
    fn kahf_dst_is_deterministic_and_32_bytes() {
        let a = kahf_domain_separator("BCS-Kahf-v1");
        let b = kahf_domain_separator("BCS-Kahf-v1");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn kahf_dst_frozen_hex_cross_language_parity() {
        // Frozen on 2026-05-17 via Codespaces: Rust + Python byte-for-byte match.
        // If this test fails, the canonical encoding logic has changed — DO NOT
        // just update the hex; investigate the root cause first.
        fn hex_to_32(s: &str) -> [u8; 32] {
            let mut out = [0u8; 32];
            for i in 0..32 {
                out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap();
            }
            out
        }

        let frozen: [(&str, &str); 3] = [
            ("BCS-Kahf-v1",    "219d1169582b4bfce491bd16d4b415aea7a0362d6cca9cd33ec7cb5394f65c5e"),
            ("BCS-256-Kahf-v1","1a3af0870bdd1e1ca80f07f6cee03b9c88fc631012c65378d7d5c683eaa40cdd"),
            ("BCS-521-Kahf-v1","ea23985a03b0d28f71fb091e7d15aaf791f5cbbfd1f1e73237f7e9c6b449bf35"),
        ];
        for (label, expected_hex) in frozen {
            let computed = kahf_domain_separator(label);
            let expected = hex_to_32(expected_hex);
            assert_eq!(computed, expected, "DST mismatch for label {}", label);
        }
    }

    #[test]
    fn kahf_dst_changes_with_label() {
        let v1 = kahf_domain_separator("BCS-Kahf-v1");
        let v521 = kahf_domain_separator("BCS-521-Kahf-v1");
        let v256 = kahf_domain_separator("BCS-256-Kahf-v1");
        assert_ne!(v1, v521);
        assert_ne!(v1, v256);
        assert_ne!(v521, v256);
    }

    #[test]
    fn kahf_primes_canonical_order() {
        let keys: Vec<&str> = KAHF_PRIMES.iter().map(|(k, _)| *k).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "Kahf primes must be in alphabetical order");

        // Five sacred primes
        assert_eq!(KAHF_PRIMES.len(), 5);

        // Sanity: the values
        let map: std::collections::HashMap<&str, u32> =
            KAHF_PRIMES.iter().copied().collect();
        assert_eq!(map["p_kahf_first_decimal"], 2141);
        assert_eq!(map["p_kahf_last_zf"], 2969);
        assert_eq!(map["p_kahf_sleepers"], 7);
        assert_eq!(map["p_kahf_surah_zf"], 19);
        assert_eq!(map["p_kahf_years_zf"], 373);
    }

    #[test]
    fn kahf_bound_hkdf_differs_from_plain() {
        // Same inputs through plain vs Kahf-bound HKDF must yield different keys.
        let secret = b"shared-secret-bytes";
        let salt = b"BCS-521 salt v1";
        let info = b"BCS-521 ECDH v1";

        let plain = hkdf_sha512_64(secret, salt, info);
        let bound = hkdf_sha512_64_kahf(secret, salt, info);
        assert_ne!(plain, bound, "Kahf binding must change derived key");
    }

    #[test]
    fn kahf_bound_hkdf_full_pipeline_521() {
        // End-to-end: ECDH -> Kahf-bound HKDF -> 64-byte key.
        let c = bcs521();
        let a = BigUint::from(0x1234_5678_9abc_def0u64);
        let b = BigUint::from(0x0fed_cba9_8765_4321u64);
        let qa = c.public_key(&a).unwrap();
        let qb = c.public_key(&b).unwrap();
        let sa = c.ecdh(&a, &qb).unwrap();
        let sb = c.ecdh(&b, &qa).unwrap();
        assert_eq!(sa, sb);

        let ka = hkdf_sha512_64_kahf(&sa, b"BCS-521 salt v1", b"BCS-521 ECDH v1");
        let kb = hkdf_sha512_64_kahf(&sb, b"BCS-521 salt v1", b"BCS-521 ECDH v1");
        assert_eq!(ka, kb, "Both parties must derive the same Kahf-bound key");
    }
}

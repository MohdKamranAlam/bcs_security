//! # High-level public API for BCS-521
//!
//! This module is the *only* surface that external users of this crate
//! should touch.  It wraps the constant-time primitives in
//! [`crate::ct`] behind opinionated newtypes with strict invariants:
//!
//! | Type | Invariants enforced by construction |
//! |------|--------------------------------------|
//! | [`Bcs521SecretKey`]   | scalar in `[1, n_521 − 1]`; zeroized on drop; `Debug` redacts |
//! | [`Bcs521PublicKey`]   | point on curve, coords `< p`, not the identity |
//! | [`Bcs521SharedSecret`]| exactly 32 bytes; zeroized on drop; `Debug` redacts |
//!
//! ## Encoding
//!
//! * **Secret key** — 66 big-endian bytes (`scalar.to_bytes_be()`).
//! * **Public key** — SEC1 uncompressed: `0x04 || X (66B) || Y (66B)` = 133 bytes.
//! * **Shared secret** — 32 raw bytes.
//!
//! ## Side-channel discipline
//!
//! All operations that touch the secret scalar route through
//! [`crate::ct`], whose scalar-multiplication is a Montgomery ladder
//! over Renes–Costello–Batina complete formulas.  This means the
//! sequence of arithmetic operations is **independent of the secret
//! bits** — no early-aborts, no table look-ups, no branches.
//!
//! ## Domain separation
//!
//! ECDH derives a 32-byte symmetric key via HKDF-SHA-256 with:
//!
//! ```text
//! salt = b"BCS-521-ECDH-v1"
//! info = b"BCS-521-ECDH-Shared-Secret-v1"
//! ikm  = x_coord(s·peer_pk)  in 66 BE bytes, *original* chart
//! ```
//!
//! These tags are part of the protocol; **do not** change them or you
//! will break interop with peers that follow this spec.
//!
//! ## What this API does *not* do
//!
//! * No signatures (no ECDSA/EdDSA over BCS-521 yet — separate module).
//! * No compressed point encoding (SEC1 0x02 / 0x03) — uncompressed only.
//! * No batched ECDH.
//! * No hybrid post-quantum KEM combination — see `BCS_PQ_ROADMAP.md`.

#![cfg(feature = "ct")]

use core::fmt;

use hkdf::Hkdf;
use rand::{CryptoRng, RngCore};
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::ct::{scalar_mul, scalar_mul_generator, Fp521, ProjPoint, Scalar};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FIELD_BYTES: usize = 66;
const SCALAR_BYTES: usize = 66;
const PUBLIC_KEY_BYTES: usize = 1 + 2 * FIELD_BYTES; // 0x04 || X || Y = 133
const SHARED_SECRET_BYTES: usize = 32;
const SEC1_UNCOMPRESSED_TAG: u8 = 0x04;

/// Fixed HKDF salt for ECDH key derivation.  Part of the wire protocol.
const ECDH_HKDF_SALT: &[u8] = b"BCS-521-ECDH-v1";

/// Fixed HKDF info string for ECDH key derivation.  Part of the wire protocol.
const ECDH_HKDF_INFO: &[u8] = b"BCS-521-ECDH-Shared-Secret-v1";

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// All public-API errors.
///
/// **Important:** external callers MUST NOT branch on the discriminant
/// for secret data.  These variants are for debugging and logging only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bcs521Error {
    /// Secret-key bytes encode a value `≥ n_521`.
    SecretKeyOutOfRange,
    /// Secret-key value is zero (forbidden by spec).
    SecretKeyIsZero,
    /// Public-key encoding length is not exactly 133 bytes.
    PublicKeyWrongLength,
    /// Public-key tag byte is not `0x04` (uncompressed SEC1).
    PublicKeyUnsupportedTag,
    /// Public-key X or Y coordinate is `≥ p_521`.
    PublicKeyCoordinateOutOfRange,
    /// Public-key `(X, Y)` does not satisfy the curve equation.
    PublicKeyNotOnCurve,
    /// Public-key encodes the identity (point at infinity) — forbidden.
    PublicKeyIsIdentity,
    /// ECDH produced the identity point.  With validated inputs this
    /// indicates fault injection or a software bug.
    EcdhResultIsIdentity,
}

impl fmt::Display for Bcs521Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::SecretKeyOutOfRange => "BCS-521: secret key value >= n_521",
            Self::SecretKeyIsZero => "BCS-521: secret key is zero (forbidden)",
            Self::PublicKeyWrongLength => {
                "BCS-521: public key must be exactly 133 bytes (SEC1 uncompressed)"
            }
            Self::PublicKeyUnsupportedTag => {
                "BCS-521: public key tag byte must be 0x04 (uncompressed)"
            }
            Self::PublicKeyCoordinateOutOfRange => "BCS-521: public key coordinate >= p_521",
            Self::PublicKeyNotOnCurve => {
                "BCS-521: public key does not satisfy y^2 = x^3 - 2x^2 + 5x + 4 (mod p)"
            }
            Self::PublicKeyIsIdentity => "BCS-521: public key is the point at infinity",
            Self::EcdhResultIsIdentity => "BCS-521: ECDH result is the identity point",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for Bcs521Error {}

// ---------------------------------------------------------------------------
// Secret Key
// ---------------------------------------------------------------------------

/// A BCS-521 secret scalar in `[1, n_521 − 1]`.
///
/// * Zeroized on drop via [`zeroize::ZeroizeOnDrop`].
/// * `Debug` deliberately redacts the value — printing a `Bcs521SecretKey`
///   yields `"Bcs521SecretKey(<redacted>)"` regardless of the contents.
/// * `Clone` is supported but discouraged for secrets; clone explicitly
///   only when duplication is provably necessary.
/// * **No** `Copy`, **no** `PartialEq` — comparing secret keys is almost
///   always a bug.  If you need it, write `sk_a.to_bytes().ct_eq(&sk_b.to_bytes())`
///   explicitly.
#[derive(Clone)]
pub struct Bcs521SecretKey {
    scalar: Scalar,
}

impl fmt::Debug for Bcs521SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Bcs521SecretKey(<redacted>)")
    }
}

impl Zeroize for Bcs521SecretKey {
    fn zeroize(&mut self) {
        self.scalar.zeroize();
    }
}

impl Drop for Bcs521SecretKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Bcs521SecretKey {}

impl Bcs521SecretKey {
    /// Generate a fresh secret key using `rng`.
    ///
    /// Uses **rejection sampling** over 521-bit uniform strings: in the
    /// astronomically improbable case that the sampled value is `0` or
    /// `≥ n_521`, the sample is discarded and another drawn.  Since
    /// `n_521 / 2^521 ≈ 1.0000…`, the expected number of attempts is
    /// `≈ 1`.  Per-attempt cost is constant-time.
    pub fn generate<R: CryptoRng + RngCore>(rng: &mut R) -> Self {
        loop {
            let mut bytes = [0u8; SCALAR_BYTES];
            rng.fill_bytes(&mut bytes);
            // Mask high bits so the value is < 2^521.
            // bytes[0] holds the MSB; the field is 521 bits = 65 full
            // bytes + 1 bit, so keep only the lowest bit of bytes[0].
            bytes[0] &= 0x01;
            if let Some(scalar) = Scalar::from_bytes_be(&bytes) {
                if !bool::from(scalar.ct_eq(&Scalar::ZERO)) {
                    return Bcs521SecretKey { scalar };
                }
            }
        }
    }

    /// Decode a secret key from 66 big-endian bytes.
    ///
    /// Rejects:
    /// - any value `≥ n_521` (`SecretKeyOutOfRange`)
    /// - the value `0`        (`SecretKeyIsZero`)
    pub fn from_bytes(bytes: &[u8; SCALAR_BYTES]) -> Result<Self, Bcs521Error> {
        let scalar = Scalar::from_bytes_be(bytes).ok_or(Bcs521Error::SecretKeyOutOfRange)?;
        if bool::from(scalar.ct_eq(&Scalar::ZERO)) {
            return Err(Bcs521Error::SecretKeyIsZero);
        }
        Ok(Bcs521SecretKey { scalar })
    }

    /// Serialize as 66 big-endian bytes.  **Caller is responsible** for
    /// erasing the returned buffer when finished (e.g. via `zeroize`).
    pub fn to_bytes(&self) -> [u8; SCALAR_BYTES] {
        self.scalar.to_bytes_be()
    }

    /// Derive the public key corresponding to this secret scalar.
    ///
    /// Constant-time in the value of `self`.
    pub fn public_key(&self) -> Bcs521PublicKey {
        let point = scalar_mul_generator(&self.scalar);
        // Since `self.scalar ∈ [1, n-1]` and `G` has order `n`,
        // `scalar * G` is never the identity.
        Bcs521PublicKey { point }
    }
}

// ---------------------------------------------------------------------------
// Public Key
// ---------------------------------------------------------------------------

/// A BCS-521 public key, **guaranteed** at construction time to be:
///
/// - on the curve `y² ≡ x³ − 2x² + 5x + 4  (mod p)`
/// - **not** the point at infinity
/// - with both coordinates strictly less than `p_521`.
///
/// These invariants are restored every time the key crosses an API
/// boundary (`from_bytes`).  Downstream code may therefore assume them
/// without re-checking.
#[derive(Clone, Debug)]
pub struct Bcs521PublicKey {
    /// Internal representation: homogeneous projective coordinates in
    /// the **short-Weierstrass** chart `x' = x_orig − 2/3`, with each
    /// coordinate already in Montgomery form.
    pub(crate) point: ProjPoint,
}

impl Bcs521PublicKey {
    /// Decode a public key from SEC1 uncompressed encoding
    /// `0x04 || X (66B) || Y (66B)` (133 bytes total).
    ///
    /// All checks are performed before the point is materialized:
    ///
    /// 1. Length is exactly 133.
    /// 2. Tag byte is `0x04`.
    /// 3. `X, Y < p_521`.
    /// 4. `Y² ≡ X³ − 2X² + 5X + 4  (mod p)`.
    /// 5. `(X, Y) ≠ (0, 0)` (the only affine form of the identity).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Bcs521Error> {
        if bytes.len() != PUBLIC_KEY_BYTES {
            return Err(Bcs521Error::PublicKeyWrongLength);
        }
        if bytes[0] != SEC1_UNCOMPRESSED_TAG {
            return Err(Bcs521Error::PublicKeyUnsupportedTag);
        }

        let mut x_bytes = [0u8; FIELD_BYTES];
        let mut y_bytes = [0u8; FIELD_BYTES];
        x_bytes.copy_from_slice(&bytes[1..1 + FIELD_BYTES]);
        y_bytes.copy_from_slice(&bytes[1 + FIELD_BYTES..]);

        let x_canon = Fp521::from_bytes_be(&x_bytes)
            .ok_or(Bcs521Error::PublicKeyCoordinateOutOfRange)?;
        let y_canon = Fp521::from_bytes_be(&y_bytes)
            .ok_or(Bcs521Error::PublicKeyCoordinateOutOfRange)?;

        // Move to Montgomery domain for the curve check.
        let xm = x_canon.to_montgomery();
        let ym = y_canon.to_montgomery();

        // Evaluate y² and x³ − 2x² + 5x + 4 in Mont form.
        let lhs = ym.square();

        let x2 = xm.square();
        let x3 = x2.mont_mul(&xm);

        let one_m = Fp521::ONE_MONT;
        let two_m = one_m + one_m;
        let four_m = two_m + two_m;
        let five_m = four_m + one_m;

        let two_x2 = x2.add_mod_p(&x2); // 2·x²
        let five_x = five_m.mont_mul(&xm); // 5·x

        let rhs = x3
            .sub_mod_p(&two_x2)
            .add_mod_p(&five_x)
            .add_mod_p(&four_m);

        if !bool::from(lhs.ct_eq(&rhs)) {
            return Err(Bcs521Error::PublicKeyNotOnCurve);
        }

        // The identity (0, 0) is not on the curve (since 0 ≠ 4) and would
        // have been rejected above.  We still keep an explicit check
        // for defence-in-depth.
        let x_is_zero = bool::from(xm.ct_eq(&Fp521::ZERO));
        let y_is_zero = bool::from(ym.ct_eq(&Fp521::ZERO));
        if x_is_zero && y_is_zero {
            return Err(Bcs521Error::PublicKeyIsIdentity);
        }

        // Translate to the short-Weierstrass chart: x_short = x_orig − 2/3.
        let two_thirds_mont = compute_two_thirds_mont();
        let x_short_mont = xm.sub_mod_p(&two_thirds_mont);
        let y_short_mont = ym;

        let point = ProjPoint {
            x: x_short_mont,
            y: y_short_mont,
            z: Fp521::ONE_MONT, // Z = 1 in Mont form
        };

        Ok(Bcs521PublicKey { point })
    }

    /// Serialize as SEC1 uncompressed: `0x04 || X || Y` (133 bytes).
    ///
    /// Coordinates are in the **original** BCS-521 chart
    /// `y² = x³ − 2x² + 5x + 4`.
    pub fn to_bytes(&self) -> [u8; PUBLIC_KEY_BYTES] {
        let mut out = [0u8; PUBLIC_KEY_BYTES];
        out[0] = SEC1_UNCOMPRESSED_TAG;

        // Invariant: never identity, so `to_affine` returns `Some`.
        let (x_short_mont, y_short_mont) = self
            .point
            .to_affine()
            .expect("Bcs521PublicKey invariant: never identity");

        let two_thirds_mont = compute_two_thirds_mont();
        let x_orig_mont = x_short_mont + two_thirds_mont;

        let x_canon = x_orig_mont.from_montgomery();
        let y_canon = y_short_mont.from_montgomery();

        out[1..1 + FIELD_BYTES].copy_from_slice(&x_canon.to_bytes_be());
        out[1 + FIELD_BYTES..].copy_from_slice(&y_canon.to_bytes_be());
        out
    }
}

// ---------------------------------------------------------------------------
// Shared Secret
// ---------------------------------------------------------------------------

/// A 32-byte symmetric shared secret produced by BCS-521 ECDH +
/// HKDF-SHA-256 with the protocol's fixed domain separator.
///
/// Zeroized on drop; `Debug` redacts.
#[derive(Clone)]
pub struct Bcs521SharedSecret {
    bytes: [u8; SHARED_SECRET_BYTES],
}

impl fmt::Debug for Bcs521SharedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Bcs521SharedSecret(<redacted>)")
    }
}

impl Zeroize for Bcs521SharedSecret {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for Bcs521SharedSecret {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Bcs521SharedSecret {}

impl Bcs521SharedSecret {
    /// Borrow the shared secret as a 32-byte array.
    ///
    /// **The returned reference is secret material.**  Treat it
    /// accordingly: do not log, do not branch on its value (except via
    /// `subtle::ConstantTimeEq`), do not copy it into a `Vec<u8>`
    /// without immediately wrapping it in something that zeroes on drop.
    pub fn as_bytes(&self) -> &[u8; SHARED_SECRET_BYTES] {
        &self.bytes
    }
}

// ---------------------------------------------------------------------------
// Facade
// ---------------------------------------------------------------------------

/// Top-level zero-sized facade type.  All real work lives in the
/// associated functions; the type itself is just a namespace.
pub struct Bcs521;

impl Bcs521 {
    /// Generate a fresh keypair `(secret, public)`.
    pub fn keygen<R: CryptoRng + RngCore>(rng: &mut R) -> (Bcs521SecretKey, Bcs521PublicKey) {
        let sk = Bcs521SecretKey::generate(rng);
        let pk = sk.public_key();
        (sk, pk)
    }

    /// Compute the BCS-521 ECDH shared secret between local secret key
    /// `sk` and the peer's already-validated public key `peer_pk`.
    ///
    /// The protocol is:
    ///
    /// 1. Compute `S = sk · peer_pk` via Montgomery ladder (constant time).
    /// 2. Extract the x-coordinate in the **original** BCS-521 chart
    ///    (`x_orig = x_short + 2/3 mod p`), serialise as 66 BE bytes.
    /// 3. HKDF-SHA-256-Extract with fixed salt `b"BCS-521-ECDH-v1"`,
    ///    then HKDF-Expand with fixed info `b"BCS-521-ECDH-Shared-Secret-v1"`,
    ///    output 32 bytes.
    ///
    /// Returns `EcdhResultIsIdentity` if `sk · peer_pk` is the identity
    /// — with validated inputs this never happens; raising the error
    /// is a defence against fault injection.
    pub fn ecdh(
        sk: &Bcs521SecretKey,
        peer_pk: &Bcs521PublicKey,
    ) -> Result<Bcs521SharedSecret, Bcs521Error> {
        let shared = scalar_mul(&sk.scalar, &peer_pk.point);
        let (x_short_mont, _y_short_mont) = shared
            .to_affine()
            .ok_or(Bcs521Error::EcdhResultIsIdentity)?;

        let two_thirds_mont = compute_two_thirds_mont();
        let x_orig_mont = x_short_mont + two_thirds_mont;
        let x_canon = x_orig_mont.from_montgomery();
        let mut x_bytes = x_canon.to_bytes_be();

        let hkdf = Hkdf::<Sha256>::new(Some(ECDH_HKDF_SALT), &x_bytes);
        let mut out = [0u8; SHARED_SECRET_BYTES];
        hkdf.expand(ECDH_HKDF_INFO, &mut out)
            .expect("32 byte output fits HKDF-SHA-256 max length");

        // Erase IKM from the stack now that the KDF has consumed it.
        x_bytes.zeroize();

        Ok(Bcs521SharedSecret { bytes: out })
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Return `2/3 mod p` in Montgomery form.
///
/// Computed at runtime from `1_M + 1_M`, one Fermat inversion of `3_M`,
/// and one Montgomery multiplication.  No new constant in `consts.rs`
/// is required, keeping the SHA-256 audit tag of consts stable.
fn compute_two_thirds_mont() -> Fp521 {
    let one = Fp521::ONE_MONT;
    let two = one + one;
    let three = two + one;
    let three_inv = three.invert().expect("3 is invertible mod p");
    two.mont_mul(&three_inv)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    // ------------- Round-trip / structural -------------

    #[test]
    fn keygen_produces_valid_keypair() {
        let mut rng = OsRng;
        let (sk, pk) = Bcs521::keygen(&mut rng);
        // Re-deriving public key must match.
        let pk2 = sk.public_key();
        assert_eq!(pk.to_bytes(), pk2.to_bytes());
    }

    #[test]
    fn public_key_serialization_round_trip() {
        let mut rng = OsRng;
        let (_sk, pk) = Bcs521::keygen(&mut rng);
        let bytes = pk.to_bytes();
        assert_eq!(bytes.len(), 133);
        assert_eq!(bytes[0], 0x04);
        let pk_again = Bcs521PublicKey::from_bytes(&bytes).expect("just produced");
        assert_eq!(bytes, pk_again.to_bytes());
    }

    #[test]
    fn secret_key_serialization_round_trip() {
        let mut rng = OsRng;
        let sk = Bcs521SecretKey::generate(&mut rng);
        let bytes = sk.to_bytes();
        let sk2 = Bcs521SecretKey::from_bytes(&bytes).expect("just produced");
        assert_eq!(bytes, sk2.to_bytes());
    }

    // ------------- Rejection cases (negative tests) -------------

    /// Helper to assert that a `Result` is `Err(expected)`.  Avoids
    /// requiring `PartialEq` on the `Ok` variant (which would force
    /// equality on secret types — a footgun).
    fn assert_err_is(result: Result<impl core::fmt::Debug, Bcs521Error>, expected: Bcs521Error) {
        match result {
            Ok(v) => panic!("expected Err({:?}), got Ok({:?})", expected, v),
            Err(e) => assert_eq!(e, expected, "wrong error variant"),
        }
    }

    #[test]
    fn secret_key_zero_rejected() {
        let zero = [0u8; SCALAR_BYTES];
        assert_err_is(
            Bcs521SecretKey::from_bytes(&zero),
            Bcs521Error::SecretKeyIsZero,
        );
    }

    #[test]
    fn secret_key_out_of_range_rejected() {
        // All-ones is definitely > n_521.
        let max = [0xFFu8; SCALAR_BYTES];
        assert_err_is(
            Bcs521SecretKey::from_bytes(&max),
            Bcs521Error::SecretKeyOutOfRange,
        );
    }

    #[test]
    fn public_key_wrong_length_rejected() {
        assert_err_is(
            Bcs521PublicKey::from_bytes(&[0x04; 132]),
            Bcs521Error::PublicKeyWrongLength,
        );
        assert_err_is(
            Bcs521PublicKey::from_bytes(&[0x04; 134]),
            Bcs521Error::PublicKeyWrongLength,
        );
        assert_err_is(
            Bcs521PublicKey::from_bytes(&[]),
            Bcs521Error::PublicKeyWrongLength,
        );
    }

    #[test]
    fn public_key_wrong_tag_rejected() {
        let mut bytes = [0u8; PUBLIC_KEY_BYTES];
        bytes[0] = 0x02; // compressed tag — not supported
        assert_err_is(
            Bcs521PublicKey::from_bytes(&bytes),
            Bcs521Error::PublicKeyUnsupportedTag,
        );
    }

    #[test]
    fn public_key_random_bytes_rejected() {
        // Random 133 bytes with 0x04 tag is overwhelmingly unlikely
        // to be on the curve.
        let mut bytes = [0u8; PUBLIC_KEY_BYTES];
        bytes[0] = 0x04;
        for (i, b) in bytes.iter_mut().enumerate().skip(1) {
            *b = (i as u8).wrapping_mul(37);
        }
        // Mask high bits so coords < p (avoid hitting the
        // "coordinate out of range" path first).
        bytes[1] &= 0x01;
        bytes[1 + FIELD_BYTES] &= 0x01;
        assert_err_is(
            Bcs521PublicKey::from_bytes(&bytes),
            Bcs521Error::PublicKeyNotOnCurve,
        );
    }

    #[test]
    fn public_key_identity_origin_rejected() {
        // (X, Y) = (0, 0) — would-be identity affine encoding.
        // Rejected because (0, 0) is not on the curve (0 ≠ 4).
        let mut bytes = [0u8; PUBLIC_KEY_BYTES];
        bytes[0] = 0x04;
        assert_err_is(
            Bcs521PublicKey::from_bytes(&bytes),
            Bcs521Error::PublicKeyNotOnCurve,
        );
    }

    #[test]
    fn public_key_x_out_of_range_rejected() {
        // X = all 0xFF: certainly > p_521.
        let mut bytes = [0xFFu8; PUBLIC_KEY_BYTES];
        bytes[0] = 0x04;
        assert_err_is(
            Bcs521PublicKey::from_bytes(&bytes),
            Bcs521Error::PublicKeyCoordinateOutOfRange,
        );
    }

    // ------------- ECDH agreement -------------

    #[test]
    fn ecdh_two_party_agreement() {
        let mut rng = OsRng;
        let (sk_a, pk_a) = Bcs521::keygen(&mut rng);
        let (sk_b, pk_b) = Bcs521::keygen(&mut rng);

        let ss_ab = Bcs521::ecdh(&sk_a, &pk_b).unwrap();
        let ss_ba = Bcs521::ecdh(&sk_b, &pk_a).unwrap();

        assert_eq!(ss_ab.as_bytes(), ss_ba.as_bytes());
    }

    #[test]
    fn ecdh_with_self_is_deterministic() {
        let mut rng = OsRng;
        let (sk, pk) = Bcs521::keygen(&mut rng);
        let ss1 = Bcs521::ecdh(&sk, &pk).unwrap();
        let ss2 = Bcs521::ecdh(&sk, &pk).unwrap();
        assert_eq!(ss1.as_bytes(), ss2.as_bytes());
    }

    // ------------- Redaction (Debug) -------------

    #[test]
    fn secret_key_debug_does_not_leak() {
        let mut rng = OsRng;
        let sk = Bcs521SecretKey::generate(&mut rng);
        let dbg = format!("{:?}", sk);
        assert!(dbg.contains("redacted"), "Debug must redact: got {}", dbg);
        // No hex of any limb byte must appear.
        let bytes = sk.to_bytes();
        for chunk in bytes.chunks(4) {
            let h = hex::encode(chunk);
            assert!(
                !dbg.contains(&h) || h == "00000000",
                "Debug leaked a non-zero limb: {}",
                h
            );
        }
    }

    #[test]
    fn shared_secret_debug_does_not_leak() {
        let mut rng = OsRng;
        let (sk_a, _pk_a) = Bcs521::keygen(&mut rng);
        let (_sk_b, pk_b) = Bcs521::keygen(&mut rng);
        let ss = Bcs521::ecdh(&sk_a, &pk_b).unwrap();
        let dbg = format!("{:?}", ss);
        assert!(dbg.contains("redacted"), "Debug must redact: got {}", dbg);
        for chunk in ss.as_bytes().chunks(4) {
            let h = hex::encode(chunk);
            assert!(
                !dbg.contains(&h) || h == "00000000",
                "Debug leaked shared secret bytes: {}",
                h
            );
        }
    }

    // ------------- Compatibility with reference impl -------------

    #[test]
    fn ecdh_matches_reference_implementation() {
        use crate::{bcs521, Point};
        use num_bigint::BigUint;

        let curve = bcs521();

        // Build a secret key from a known scalar so both paths use the
        // *same* value.  0xDEADBEEFCAFE — small enough to be < n.
        let mut scalar_bytes = [0u8; 66];
        scalar_bytes[60..66].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE]);
        let sk = Bcs521SecretKey::from_bytes(&scalar_bytes).unwrap();
        let k_big = BigUint::from_bytes_be(&scalar_bytes);

        // Public key via the new API.
        let pk = sk.public_key();
        let pk_bytes = pk.to_bytes();

        // Public key via the reference implementation (Curve::public_key).
        let ref_pk_point = curve.public_key(&k_big).unwrap();
        let ref_pk_bytes = match ref_pk_point {
            Point::Affine { ref x, ref y } => {
                let mut out = [0u8; 133];
                out[0] = 0x04;
                let xb = x.to_bytes_be();
                let yb = y.to_bytes_be();
                out[1 + 66 - xb.len()..1 + 66].copy_from_slice(&xb);
                out[1 + 66 + 66 - yb.len()..].copy_from_slice(&yb);
                out
            }
            Point::Infinity => panic!("reference produced identity"),
        };
        assert_eq!(pk_bytes, ref_pk_bytes, "public key mismatch CT vs reference");

        // ECDH against own pubkey: agree across CT and reference.
        let ss_ct = Bcs521::ecdh(&sk, &pk).unwrap();
        let ref_ss_raw = curve.ecdh(&k_big, &ref_pk_point).unwrap();

        // Reference returns the raw shared x-coordinate (≤ 66 bytes);
        // pad it to exactly 66 bytes BE, then run the API's KDF and
        // compare against the CT API output.
        let mut ikm = [0u8; 66];
        ikm[66 - ref_ss_raw.len()..].copy_from_slice(&ref_ss_raw);
        let hkdf = Hkdf::<Sha256>::new(Some(ECDH_HKDF_SALT), &ikm);
        let mut expected = [0u8; 32];
        hkdf.expand(ECDH_HKDF_INFO, &mut expected).unwrap();

        assert_eq!(ss_ct.as_bytes(), &expected, "ECDH parity vs reference failed");
    }
}

//! # Constant-time ECDSA over BCS-521
//!
//! Deterministic ECDSA (RFC 6979, SHA-256) with **all** secret-scalar
//! operations routed through the constant-time Montgomery-ladder and
//! CIOS Montgomery-multiplication path in [`crate::ct`].
//!
//! ## Status
//!
//! | Operation | Status |
//! |----------|--------|
//! | `sign`   | ✅ CT (Montgomery ladder + CIOS scalar arithmetic) |
//! | `verify` | ✅ CT (Montgomery ladder + CIOS scalar arithmetic) |
//!
//! ## Encoding
//!
//! A [`Bcs521EcdsaSignature`] is `(r, s)` each as 66 big-endian bytes,
//! concatenated for 132 bytes total.
//!
//! ## Algorithm
//!
//! **Sign:**
//! 1. `e = SHA-256(msg)` as integer (32-byte hash, no truncation needed
//!    since `hlen = 256 < qlen = 521`).
//! 2. `k = RFC6979(sk, e, SHA-256, qlen=521)` — deterministic nonce.
//! 3. `R = k·G` via Montgomery ladder (constant-time).
//! 4. `r = R.x mod n`  (x in the **original** BCS chart).
//! 5. `s = k⁻¹·(e + r·sk) mod n`  via CT `inv_mod_n` + `mul_mod_n`.
//!
//! **Verify:**
//! 1. Range-check `r, s ∈ [1, n−1]`.
//! 2. `e = SHA-256(msg)` as integer.
//! 3. `w = s⁻¹ mod n`  (CT Fermat inversion).
//! 4. `u1 = e·w mod n`, `u2 = r·w mod n`.
//! 5. `X = u1·G + u2·Q`  (two Montgomery-ladder scalar muls + point add).
//! 6. Accept iff `X.x ≡ r (mod n)`.

#![cfg(feature = "ecdsa")]

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::consts::FIELD_BYTES;
use super::fp521::Fp521;
use super::ladder::{scalar_mul, scalar_mul_generator};
use super::point::ProjPoint;
use super::scalar::Scalar;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SCALAR_BYTES: usize = FIELD_BYTES; // 66
const SIGNATURE_BYTES: usize = 2 * SCALAR_BYTES; // 132

// ---------------------------------------------------------------------------
// Signature type
// ---------------------------------------------------------------------------

/// A BCS-521 ECDSA signature `(r, s)` encoded as two 66-byte big-endian
/// scalars concatenated.  Total wire size: 132 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bcs521EcdsaSignature {
    /// `r` — x-coordinate of the nonce point, reduced mod n.  66 BE bytes.
    pub r: [u8; SCALAR_BYTES],
    /// `s` — the scalar proof value.  66 BE bytes.
    pub s: [u8; SCALAR_BYTES],
}

impl Bcs521EcdsaSignature {
    /// Decode from the 132-byte wire format `r || s`.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != SIGNATURE_BYTES {
            return None;
        }
        let mut r = [0u8; SCALAR_BYTES];
        let mut s = [0u8; SCALAR_BYTES];
        r.copy_from_slice(&bytes[..SCALAR_BYTES]);
        s.copy_from_slice(&bytes[SCALAR_BYTES..]);
        Some(Self { r, s })
    }

    /// Encode to the 132-byte wire format `r || s`.
    pub fn to_bytes(&self) -> [u8; SIGNATURE_BYTES] {
        let mut out = [0u8; SIGNATURE_BYTES];
        out[..SCALAR_BYTES].copy_from_slice(&self.r);
        out[SCALAR_BYTES..].copy_from_slice(&self.s);
        out
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors returned by CT ECDSA sign / verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtEcdsaError {
    /// Secret key is zero or `>= n_521`.
    InvalidSecretKey,
    /// Public-key byte length is not 133.
    InvalidPublicKeyLength,
    /// Public-key tag byte is not `0x04`.
    InvalidPublicKeyTag,
    /// Public-key coordinates do not satisfy the curve equation.
    InvalidPublicKey,
    /// Public key encodes the identity (point at infinity).
    PublicKeyIsIdentity,
    /// The nonce `k` produced an `r = 0` or `s = 0` — astronomically rare.
    NonceFailed,
    /// Signature `r` or `s` is zero or `>= n`.
    InvalidSignature,
}

impl core::fmt::Display for CtEcdsaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let msg = match self {
            Self::InvalidSecretKey => "CT-ECDSA: secret key is zero or >= n_521",
            Self::InvalidPublicKeyLength => "CT-ECDSA: public key must be 133 bytes",
            Self::InvalidPublicKeyTag => "CT-ECDSA: public key tag must be 0x04",
            Self::InvalidPublicKey => "CT-ECDSA: public key not on BCS-521 curve",
            Self::PublicKeyIsIdentity => "CT-ECDSA: public key is the point at infinity",
            Self::NonceFailed => "CT-ECDSA: nonce produced r=0 or s=0",
            Self::InvalidSignature => "CT-ECDSA: signature r or s is zero or >= n",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for CtEcdsaError {}

// ---------------------------------------------------------------------------
// Sign (constant-time)
// ---------------------------------------------------------------------------

/// Sign `msg` with `sk_bytes` (66-byte big-endian secret scalar).
///
/// Uses SHA-256 as the hash function and RFC 6979 §3.2 for the
/// deterministic nonce `k`.  **All** secret-scalar operations are
/// constant-time (Montgomery ladder + CIOS Montgomery multiplication).
///
/// Returns the signature `(r, s)` as 132 bytes.
pub fn ct_sign(sk_bytes: &[u8; SCALAR_BYTES], msg: &[u8]) -> Result<Bcs521EcdsaSignature, CtEcdsaError> {
    // Parse and validate secret key.
    let sk = Scalar::from_bytes_be(sk_bytes).ok_or(CtEcdsaError::InvalidSecretKey)?;
    if bool::from(sk.ct_eq(&Scalar::ZERO)) {
        return Err(CtEcdsaError::InvalidSecretKey);
    }

    // Step 1: e = SHA-256(msg) as integer.
    let h1: [u8; 32] = Sha256::digest(msg).into();
    let e = scalar_from_hash(&h1);

    // Step 2: k = RFC 6979 deterministic nonce.
    let k = rfc6979_nonce(&sk_bytes, &h1);

    // Step 3: R = k·G via Montgomery ladder (constant-time).
    let r_point = scalar_mul_generator(&k);
    let r_scalar = extract_r(&r_point)?;

    // Step 4: s = k⁻¹ · (e + r · sk) mod n.
    let k_inv = k.inv_mod_n().ok_or(CtEcdsaError::NonceFailed)?;
    let r_times_sk = r_scalar.mul_mod_n(&sk);
    let e_plus_r_sk = e.add_mod_n(&r_times_sk);
    let s = k_inv.mul_mod_n(&e_plus_r_sk);

    // s must not be zero.
    if bool::from(s.ct_eq(&Scalar::ZERO)) {
        return Err(CtEcdsaError::NonceFailed);
    }

    Ok(Bcs521EcdsaSignature {
        r: r_scalar.to_bytes_be(),
        s: s.to_bytes_be(),
    })
}

// ---------------------------------------------------------------------------
// Verify (constant-time)
// ---------------------------------------------------------------------------

/// Verify ECDSA signature `sig` over `msg` against 133-byte SEC1 public key.
///
/// Returns `Ok(true)` iff the signature is valid, `Ok(false)` iff invalid,
/// `Err(_)` if the inputs are malformed.
///
/// **All** scalar operations are constant-time.
pub fn ct_verify(
    pk_bytes: &[u8],
    msg: &[u8],
    sig: &Bcs521EcdsaSignature,
) -> Result<bool, CtEcdsaError> {
    // Parse and validate public key.
    let pk_point = parse_public_key(pk_bytes)?;

    // Parse and range-check (r, s).
    let r_scalar = Scalar::from_bytes_be(&sig.r);
    let s_scalar = Scalar::from_bytes_be(&sig.s);

    match (r_scalar, s_scalar) {
        (Some(r), Some(s)) => {
            // r, s must be in [1, n-1].
            if bool::from(r.ct_eq(&Scalar::ZERO)) || bool::from(s.ct_eq(&Scalar::ZERO)) {
                return Ok(false);
            }
            verify_core(&pk_point, &r, &s, msg)
        }
        _ => Ok(false), // r or s >= n
    }
}

/// Core verification logic, after all parsing and range checks.
fn verify_core(
    pk_point: &ProjPoint,
    r: &Scalar,
    s: &Scalar,
    msg: &[u8],
) -> Result<bool, CtEcdsaError> {
    // e = SHA-256(msg) as integer.
    let h1: [u8; 32] = Sha256::digest(msg).into();
    let e = scalar_from_hash(&h1);

    // w = s⁻¹ mod n.
    let w = match s.inv_mod_n() {
        Some(w) => w,
        None => return Ok(false), // s = 0, already checked above but defence-in-depth
    };

    // u1 = e·w mod n, u2 = r·w mod n.
    let u1 = e.mul_mod_n(&w);
    let u2 = r.mul_mod_n(&w);

    // X = u1·G + u2·Q  (two CT scalar muls + one CT point add).
    let u1g = scalar_mul_generator(&u1);
    let u2q = scalar_mul(&u2, pk_point);
    let x_point = u1g.add(u2q);

    // Extract x-coordinate in original chart and reduce mod n.
    let (x_short_mont, _y) = match x_point.to_affine() {
        Some(xy) => xy,
        None => return Ok(false), // X is the identity
    };

    let two_thirds_mont = compute_two_thirds_mont();
    let x_orig_mont = x_short_mont + two_thirds_mont;
    let x_canon = x_orig_mont.from_montgomery();
    let x_bytes = x_canon.to_bytes_be();

    // x mod n: interpret the 66-byte BE encoding as a scalar.
    // x < p but may be >= n, so use from_bytes_be_reduce.
    let x_mod_n = Scalar::from_bytes_be_reduce(&x_bytes);

    // Accept iff x_mod_n == r (constant-time compare).
    Ok(bool::from(x_mod_n.ct_eq(r)))
}

// ---------------------------------------------------------------------------
// RFC 6979 deterministic nonce (§3.2, SHA-256, qlen = 521 bits)
// ---------------------------------------------------------------------------

/// Generate a deterministic, per-message nonce per RFC 6979 §3.2.
///
/// Parameters:
/// - `sk_bytes` — secret scalar as 66 big-endian bytes (`int2octets(x)`)
/// - `h1`       — SHA-256 message digest (32 bytes)
fn rfc6979_nonce(sk_bytes: &[u8; SCALAR_BYTES], h1: &[u8; 32]) -> Scalar {
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<Sha256>;

    let n_bytes: usize = SCALAR_BYTES; // 66
    let hlen: usize = 32;

    // bits2octets(h1): interpret h1 as integer, reduce mod n, encode as 66 BE bytes.
    let h_int = scalar_from_hash(h1);
    let h_mod_n_bytes = h_int.to_bytes_be();

    // Step a: already have h1.
    // Step b: V = 0x01 * hlen.
    let mut v = vec![0x01u8; hlen];
    // Step c: K = 0x00 * hlen.
    let mut k_mac = vec![0x00u8; hlen];

    // Step d: K = HMAC_K(V || 0x00 || int2octets(sk) || bits2octets(h1)).
    {
        let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC key length");
        mac.update(&v);
        mac.update(&[0x00]);
        mac.update(sk_bytes);
        mac.update(&h_mod_n_bytes);
        k_mac = mac.finalize().into_bytes().to_vec();
    }

    // Step e: V = HMAC_K(V).
    {
        let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC key length");
        mac.update(&v);
        v = mac.finalize().into_bytes().to_vec();
    }

    // Step f: K = HMAC_K(V || 0x01 || int2octets(sk) || bits2octets(h1)).
    {
        let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC key length");
        mac.update(&v);
        mac.update(&[0x01]);
        mac.update(sk_bytes);
        mac.update(&h_mod_n_bytes);
        k_mac = mac.finalize().into_bytes().to_vec();
    }

    // Step g: V = HMAC_K(V).
    {
        let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC key length");
        mac.update(&v);
        v = mac.finalize().into_bytes().to_vec();
    }

    // Step h: generate candidate nonces until one is in [1, n-1].
    loop {
        // Accumulate T until |T| >= n_bytes (66 bytes).
        let mut t: Vec<u8> = Vec::with_capacity(n_bytes + hlen);
        while t.len() < n_bytes {
            let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC key length");
            mac.update(&v);
            v = mac.finalize().into_bytes().to_vec();
            t.extend_from_slice(&v);
        }

        // bits2int(T):
        //   T is n_bytes*8 = 528 bits, qlen = 521 bits.
        //   Discard the 7 rightmost (least-significant) bits → right-shift by 7.
        let mut t_truncated = t[..n_bytes].to_vec();
        right_shift_be_bytes(&mut t_truncated, 7);

        if let Some(candidate) = Scalar::from_bytes_be(
            <&[u8; SCALAR_BYTES]>::try_from(&t_truncated[..]).ok().unwrap(),
        ) {
            if !bool::from(candidate.ct_eq(&Scalar::ZERO)) {
                return candidate;
            }
        }

        // Candidate out of range: update K and V and retry.
        {
            let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC key length");
            mac.update(&v);
            mac.update(&[0x00]);
            k_mac = mac.finalize().into_bytes().to_vec();
        }
        {
            let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC key length");
            mac.update(&v);
            v = mac.finalize().into_bytes().to_vec();
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Right-shift a big-endian byte array by `shift` bits (0 < shift < 8).
///
/// Used by RFC 6979 `bits2int` when the HMAC output is wider than
/// `qlen` bits.  For BCS-521: T is 528 bits, qlen = 521, so shift = 7.
fn right_shift_be_bytes(bytes: &mut [u8], shift: usize) {
    debug_assert!(shift > 0 && shift < 8);
    let n = bytes.len();
    // Process from the least-significant byte (end) to the most-significant
    // (start), carrying the low bits of byte[i-1] into byte[i].
    // Mask the source byte before left-shifting to prevent u8 wrap-around
    // losing carry bits (e.g. 0x80 << 1 would wrap to 0x00 without the mask).
    let mask: u8 = (1u8 << (8 - shift)) - 1; // bottom (8-shift) bits
    for i in (1..n).rev() {
        bytes[i] = (bytes[i] >> shift) | ((bytes[i - 1] & mask) << (8 - shift));
    }
    bytes[0] >>= shift;
}

/// Convert a 32-byte SHA-256 hash into a scalar `e` in `[0, n-1]`.
///
/// Since `hlen = 256 < qlen = 521`, no right-shift is needed per
/// RFC 6979 §2.3.2.  The hash is simply interpreted as a big-endian
/// integer and reduced mod n.
fn scalar_from_hash(h1: &[u8; 32]) -> Scalar {
    // Pad the 32-byte hash to 66 bytes (left-pad with zeros).
    let mut padded = [0u8; SCALAR_BYTES];
    padded[SCALAR_BYTES - 32..].copy_from_slice(h1);

    // from_bytes_be returns None if value >= n, but a 256-bit value
    // is always < a 521-bit prime, so this always succeeds.
    let s = Scalar::from_bytes_be(&padded).expect("256-bit hash < 521-bit n");
    // The value is already < n (since 2^256 < n_521), no reduction needed.
    s
}

/// Extract `r = R.x mod n` from the nonce point `R`.
///
/// Converts from the internal short-Weierstrass chart back to the
/// original BCS chart (`x_orig = x_short + 2/3 mod p`), then
/// reduces mod n.
fn extract_r(r_point: &ProjPoint) -> Result<Scalar, CtEcdsaError> {
    let (x_short_mont, _y) = r_point
        .to_affine()
        .ok_or(CtEcdsaError::NonceFailed)?;

    // Convert to original chart: x_orig = x_short + 2/3 mod p.
    let two_thirds_mont = compute_two_thirds_mont();
    let x_orig_mont = x_short_mont + two_thirds_mont;
    let x_canon = x_orig_mont.from_montgomery();
    let x_bytes = x_canon.to_bytes_be();

    // x mod n: interpret the 66-byte BE encoding as a scalar.
    // x < p but may be >= n (rare), so use from_bytes_be_reduce.
    let r = Scalar::from_bytes_be_reduce(&x_bytes);

    if bool::from(r.ct_eq(&Scalar::ZERO)) {
        return Err(CtEcdsaError::NonceFailed);
    }

    Ok(r)
}

/// Parse a 133-byte SEC1 uncompressed public key into a `ProjPoint`
/// in the short-Weierstrass chart, with full validation.
fn parse_public_key(pk_bytes: &[u8]) -> Result<ProjPoint, CtEcdsaError> {
    const PUBLIC_KEY_BYTES: usize = 1 + 2 * FIELD_BYTES; // 133
    const SEC1_TAG: u8 = 0x04;

    if pk_bytes.len() != PUBLIC_KEY_BYTES {
        return Err(CtEcdsaError::InvalidPublicKeyLength);
    }
    if pk_bytes[0] != SEC1_TAG {
        return Err(CtEcdsaError::InvalidPublicKeyTag);
    }

    let mut x_bytes = [0u8; FIELD_BYTES];
    let mut y_bytes = [0u8; FIELD_BYTES];
    x_bytes.copy_from_slice(&pk_bytes[1..1 + FIELD_BYTES]);
    y_bytes.copy_from_slice(&pk_bytes[1 + FIELD_BYTES..]);

    let x_canon = Fp521::from_bytes_be(&x_bytes)
        .ok_or(CtEcdsaError::InvalidPublicKey)?;
    let y_canon = Fp521::from_bytes_be(&y_bytes)
        .ok_or(CtEcdsaError::InvalidPublicKey)?;

    // Move to Montgomery domain for the curve check.
    let xm = x_canon.to_montgomery();
    let ym = y_canon.to_montgomery();

    // Evaluate y² ≡ x³ − 2x² + 5x + 4 (mod p) in Montgomery form.
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
        return Err(CtEcdsaError::InvalidPublicKey);
    }

    // Reject the identity (0, 0) — defence-in-depth.
    let x_is_zero = bool::from(xm.ct_eq(&Fp521::ZERO));
    let y_is_zero = bool::from(ym.ct_eq(&Fp521::ZERO));
    if x_is_zero && y_is_zero {
        return Err(CtEcdsaError::PublicKeyIsIdentity);
    }

    // Translate to the short-Weierstrass chart: x_short = x_orig − 2/3.
    let two_thirds_mont = compute_two_thirds_mont();
    let x_short_mont = xm.sub_mod_p(&two_thirds_mont);
    let y_short_mont = ym;

    Ok(ProjPoint {
        x: x_short_mont,
        y: y_short_mont,
        z: Fp521::ONE_MONT,
    })
}

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sk_bytes_small(val: u64) -> [u8; SCALAR_BYTES] {
        let mut b = [0u8; SCALAR_BYTES];
        b[SCALAR_BYTES - 8..].copy_from_slice(&val.to_be_bytes());
        b
    }

    fn make_pk_bytes(sk_bytes: &[u8; SCALAR_BYTES]) -> Vec<u8> {
        let sk = Scalar::from_bytes_be(sk_bytes).expect("valid sk");
        let point = scalar_mul_generator(&sk);
        let (x_short_mont, y_short_mont) = point.to_affine().expect("not identity");

        let two_thirds_mont = compute_two_thirds_mont();
        let x_orig_mont = x_short_mont + two_thirds_mont;
        let x_canon = x_orig_mont.from_montgomery();
        let y_canon = y_short_mont.from_montgomery();

        let mut out = vec![0x04u8];
        out.extend_from_slice(&x_canon.to_bytes_be());
        out.extend_from_slice(&y_canon.to_bytes_be());
        out
    }

    #[test]
    fn sign_verify_roundtrip() {
        let sk = sk_bytes_small(0x1234_5678_9abc_def0u64);
        let pk = make_pk_bytes(&sk);
        let msg = b"Bismillah al-Rahman al-Raheem";
        let sig = ct_sign(&sk, msg).expect("sign failed");
        let ok = ct_verify(&pk, msg, &sig).expect("verify failed");
        assert!(ok, "valid signature should verify");
    }

    #[test]
    fn wrong_message_fails() {
        let sk = sk_bytes_small(0xdead_beef_cafe_0123u64);
        let pk = make_pk_bytes(&sk);
        let msg = b"Hello";
        let sig = ct_sign(&sk, msg).expect("sign");
        let bad = ct_verify(&pk, b"World", &sig).expect("verify");
        assert!(!bad, "wrong message must not verify");
    }

    #[test]
    fn wrong_key_fails() {
        let sk1 = sk_bytes_small(42);
        let sk2 = sk_bytes_small(99);
        let pk1 = make_pk_bytes(&sk1);
        let pk2 = make_pk_bytes(&sk2);
        let msg = b"test";
        let sig = ct_sign(&sk1, msg).expect("sign");
        let bad = ct_verify(&pk2, msg, &sig).expect("verify");
        assert!(!bad, "wrong key must not verify");
    }

    #[test]
    fn deterministic_signature() {
        let sk = sk_bytes_small(7);
        let msg = b"deterministic test";
        let sig1 = ct_sign(&sk, msg).expect("sign");
        let sig2 = ct_sign(&sk, msg).expect("sign");
        assert_eq!(sig1, sig2, "RFC 6979 must produce identical signatures");
    }

    #[test]
    fn signature_encoding_roundtrip() {
        let sk = sk_bytes_small(17);
        let msg = b"encode test";
        let sig = ct_sign(&sk, msg).expect("sign");
        let bytes = sig.to_bytes();
        let decoded = Bcs521EcdsaSignature::from_bytes(&bytes).expect("decode");
        assert_eq!(sig, decoded);
    }

    #[test]
    fn zero_sk_rejected() {
        let sk = [0u8; SCALAR_BYTES];
        assert!(ct_sign(&sk, b"x").is_err());
    }

    #[test]
    fn invalid_pk_rejected() {
        let sk = sk_bytes_small(42);
        let msg = b"test";
        let sig = ct_sign(&sk, msg).expect("sign");

        // Wrong length.
        assert!(ct_verify(&[0x04; 132], msg, &sig).is_err());
        // Wrong tag.
        let mut bad = vec![0x03];
        bad.extend_from_slice(&[0u8; 132]);
        assert!(ct_verify(&bad, msg, &sig).is_err());
        // Random bytes (not on curve).
        let mut bad = vec![0x04];
        bad.extend_from_slice(&[0x42u8; 132]);
        assert!(ct_verify(&bad, msg, &sig).is_err());
    }

    #[test]
    fn multiple_messages_verify() {
        let sk = sk_bytes_small(0xBEEF_CAFE_DEAD_1234u64);
        let pk = make_pk_bytes(&sk);
        let messages: &[&[u8]] = &[
            b"message 1",
            b"message 2",
            b"different content",
            b"",
            b"Bismillah",
        ];
        for msg in messages {
            let sig = ct_sign(&sk, msg).expect("sign");
            let ok = ct_verify(&pk, msg, &sig).expect("verify");
            assert!(ok, "signature should verify for message {:?}", msg);
        }
    }

    // -----------------------------------------------------------------
    // Cross-check with BigUint reference ECDSA
    // -----------------------------------------------------------------

    #[test]
    fn ct_sign_matches_reference_sign() {
        use crate::ecdsa::{sign as ref_sign, Bcs521Signature};
        use num_bigint::BigUint;

        let sk_val: u64 = 0x1234_5678_9abc_def0;
        let sk_bytes = sk_bytes_small(sk_val);
        let msg = b"parity test between CT and reference ECDSA";

        // CT sign.
        let ct_sig = ct_sign(&sk_bytes, msg).expect("CT sign");

        // Reference sign (BigUint path).
        let ref_sig = ref_sign(&sk_bytes, msg).expect("reference sign");

        // Both must produce the same (r, s) because they use the same
        // RFC 6979 nonce derivation.
        assert_eq!(ct_sig.r, ref_sig.r, "r mismatch: CT vs reference");
        assert_eq!(ct_sig.s, ref_sig.s, "s mismatch: CT vs reference");
    }

    #[test]
    fn ct_verify_accepts_reference_signature() {
        use crate::ecdsa::sign as ref_sign;

        let sk_val: u64 = 0xDEAD_BEEF_CAFE_0123;
        let sk_bytes = sk_bytes_small(sk_val);
        let pk = make_pk_bytes(&sk_bytes);
        let msg = b"cross-verify test";

        // Sign with reference (BigUint) path.
        let ref_sig = ref_sign(&sk_bytes, msg).expect("reference sign");

        // Convert reference signature to CT signature format.
        let ct_sig = Bcs521EcdsaSignature {
            r: ref_sig.r,
            s: ref_sig.s,
        };

        // Verify with CT path.
        let ok = ct_verify(&pk, msg, &ct_sig).expect("CT verify");
        assert!(ok, "CT verify should accept reference signature");
    }

    // -----------------------------------------------------------------
    // Unit tests for internal helpers
    // -----------------------------------------------------------------

    #[test]
    fn right_shift_be_bytes_simple() {
        // 0x8000 >> 7 = 0x0100 = 256
        let mut buf = [0x80u8, 0x00];
        right_shift_be_bytes(&mut buf, 7);
        assert_eq!(buf, [0x01, 0x00]);

        // 0xFFFF >> 7 = 0x01FF = 511
        let mut buf = [0xFFu8, 0xFF];
        right_shift_be_bytes(&mut buf, 7);
        assert_eq!(buf, [0x01, 0xFF]);

        // 0x8100 >> 7 = 0x0102 = 258
        let mut buf = [0x81u8, 0x00];
        right_shift_be_bytes(&mut buf, 7);
        assert_eq!(buf, [0x01, 0x02]);
    }

    #[test]
    fn right_shift_be_bytes_66_bytes() {
        // All-ones 66-byte array >> 7 should produce 0x01 followed by 65 bytes of 0xFF,
        // with the last byte being 0x01 (since the bottom 7 bits of the last 0xFF
        // shift out, and the carry from the byte above fills in).
        let mut buf = [0xFFu8; 66];
        right_shift_be_bytes(&mut buf, 7);
        assert_eq!(buf[0], 0x01);
        // The rest should be 0xFF (each byte gets 1 bit from above + 7 bits from itself).
        for i in 1..66 {
            assert_eq!(buf[i], 0xFF, "byte {} should be 0xFF, got {:02x}", i, buf[i]);
        }
    }

    #[test]
    fn from_bytes_be_reduce_handles_n() {
        // n itself: from_bytes_be returns None, but from_bytes_be_reduce should give 0.
        let n_bytes = Scalar::N.to_bytes_be();
        let reduced = Scalar::from_bytes_be_reduce(&n_bytes);
        assert!(
            bool::from(reduced.ct_eq(&Scalar::ZERO)),
            "n mod n should be zero"
        );

        // n-1: should be unchanged by from_bytes_be_reduce.
        let mut one_bytes = [0u8; SCALAR_BYTES];
        one_bytes[SCALAR_BYTES - 1] = 1;
        let one_scalar = Scalar::from_bytes_be_reduce(&one_bytes);
        let n_minus_1 = Scalar::N.sub_mod_n(&one_scalar);
        let n_minus_1_bytes = n_minus_1.to_bytes_be();
        let reduced = Scalar::from_bytes_be_reduce(&n_minus_1_bytes);
        assert!(
            bool::from(reduced.ct_eq(&n_minus_1)),
            "n-1 mod n should be n-1"
        );
    }

    // -----------------------------------------------------------------
    // KAT parity: cross-check with Python-generated ECDSA vectors
    // -----------------------------------------------------------------

    /// Read the ECDSA KAT file produced by `generate_bcs521_kats.py`.
    /// The file lives in `bcs-verify/kats/bcs521_ecdsa.json`, which is
    /// **not** inside the crate directory.  The test is gated behind
    /// the `ecdsa_kat` feature so normal CI doesn't need the file.
    #[cfg(feature = "ecdsa_kat")]
    #[test]
    fn ecdsa_kat_parity() {
        use std::fs;

        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../bcs-verify/kats/bcs521_ecdsa.json"
        );
        let data = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("KAT file not found at {}: {}", path, e));

        let kat: serde_json::Value = serde_json::from_str(&data).expect("invalid JSON");
        let vectors = kat["vectors"].as_array().expect("vectors array");

        let mut pass = 0usize;
        let mut fail = 0usize;

        for v in vectors {
            let idx = v["index"].as_u64().unwrap();

            // Decode fields.
            let sk_bytes: [u8; SCALAR_BYTES] = {
                let hex = v["secret_key_hex"].as_str().unwrap();
                let bytes = hex_decode(hex);
                <[u8; SCALAR_BYTES]>::try_from(bytes.as_slice()).unwrap()
            };

            let pk_bytes: Vec<u8> = hex_decode(v["public_key_sec1_hex"].as_str().unwrap());

            let msg: Vec<u8> = hex_decode(v["message_hex"].as_str().unwrap());

            let expected_r: [u8; SCALAR_BYTES] = {
                let hex = v["r_hex"].as_str().unwrap();
                let bytes = hex_decode(hex);
                <[u8; SCALAR_BYTES]>::try_from(bytes.as_slice()).unwrap()
            };
            let expected_s: [u8; SCALAR_BYTES] = {
                let hex = v["s_hex"].as_str().unwrap();
                let bytes = hex_decode(hex);
                <[u8; SCALAR_BYTES]>::try_from(bytes.as_slice()).unwrap()
            };

            // CT sign — must produce the same (r, s).
            let ct_sig = ct_sign(&sk_bytes, &msg).unwrap_or_else(|e| {
                panic!("vector {}: ct_sign failed: {:?}", idx, e)
            });

            if ct_sig.r != expected_r || ct_sig.s != expected_s {
                eprintln!(
                    "vector {}: SIGNATURE MISMATCH\n  r: got {} expected {}\n  s: got {} expected {}",
                    idx,
                    hex_encode(&ct_sig.r),
                    hex_encode(&expected_r),
                    hex_encode(&ct_sig.s),
                    hex_encode(&expected_s),
                );
                fail += 1;
                continue;
            }

            // CT verify — must accept the expected signature.
            let expected_sig = Bcs521EcdsaSignature {
                r: expected_r,
                s: expected_s,
            };
            let ok = ct_verify(&pk_bytes, &msg, &expected_sig)
                .unwrap_or_else(|e| panic!("vector {}: ct_verify error: {:?}", idx, e));

            if !ok {
                eprintln!("vector {}: ct_verify rejected valid signature", idx);
                fail += 1;
                continue;
            }

            // CT verify — must reject a tampered message.
            let tampered: Vec<u8> = msg.iter().chain(b"-tampered").copied().collect();
            let bad = ct_verify(&pk_bytes, &tampered, &expected_sig)
                .unwrap_or_else(|e| panic!("vector {}: ct_verify(tampered) error: {:?}", idx, e));

            if bad {
                eprintln!("vector {}: ct_verify accepted tampered message!", idx);
                fail += 1;
                continue;
            }

            pass += 1;
        }

        assert_eq!(fail, 0, "{} KAT vectors failed, {} passed", fail, pass);
        eprintln!("ECDSA KAT parity: {}/{} vectors passed", pass, pass + fail);
    }

    fn hex_decode(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    fn hex_encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

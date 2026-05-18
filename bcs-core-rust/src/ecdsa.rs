//! # BCS-521 ECDSA — Reference implementation (v0.3.0)
//!
//! Deterministic ECDSA (RFC 6979) over the BCS-521 curve using SHA-256
//! as the hash function.
//!
//! ## Status
//!
//! | Operation | Status |
//! |-----------|--------|
//! | `sign`    | ✅ Reference (BigUint path, **not constant-time**) |
//! | `verify`  | ✅ Reference (BigUint path, **not constant-time**) |
//! | CT sign   | 🚧 v0.3.1 — requires Barrett reduction + Fermat inversion in `ct::scalar` |
//!
//! ## Warning
//!
//! The BigUint scalar-multiplication path is **not constant-time** — timing
//! analysis can recover the secret scalar.  Use this implementation only for
//! offline key generation, test vectors, and interoperability checks.  A
//! constant-time sign path (Montgomery-ladder + Barrett-reduced scalar
//! arithmetic) is tracked for v0.3.1.
//!
//! ## Encoding
//!
//! A `Bcs521Signature` is a pair `(r, s)` each serialized as 66 big-endian
//! bytes (521-bit scalars), concatenated for a total of 132 bytes.
//!
//! ## Algorithm outline
//!
//! 1. `e = SHA-256(msg)` as a `BigUint` (32-byte hash, treated as integer).
//! 2. `k = RFC6979(sk, e, SHA-256, qlen=521)` — deterministic, per §3.2.
//! 3. `R = k·G`, `r = R.x mod n`.
//! 4. `s = k⁻¹·(e + r·sk) mod n`  (Fermat inverse via `k^(n−2) mod n`).
//! 5. Signature = `(r, s)`.
//!
//! Verification: `w = s⁻¹`, `u1 = e·w`, `u2 = r·w`, check `(u1·G + u2·Q).x ≡ r (mod n)`.

#![cfg(feature = "ecdsa")]

use num_bigint::BigUint;
use num_traits::{One, Zero};
use sha2::{Digest, Sha256};

use crate::{bcs521, biguint_to_be_bytes, Point};

// ---------------------------------------------------------------------------
// Signature type
// ---------------------------------------------------------------------------

/// A BCS-521 ECDSA signature `(r, s)` encoded as two 66-byte big-endian
/// scalars concatenated.  Total wire size: 132 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bcs521Signature {
    /// `r` — x-coordinate of the nonce point, reduced mod n.  66 big-endian bytes.
    pub r: [u8; 66],
    /// `s` — the scalar proof value.  66 big-endian bytes.
    pub s: [u8; 66],
}

impl Bcs521Signature {
    /// Decode from the 132-byte wire format `r || s`.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 132 {
            return None;
        }
        let mut r = [0u8; 66];
        let mut s = [0u8; 66];
        r.copy_from_slice(&bytes[..66]);
        s.copy_from_slice(&bytes[66..]);
        Some(Self { r, s })
    }

    /// Encode to the 132-byte wire format `r || s`.
    pub fn to_bytes(&self) -> [u8; 132] {
        let mut out = [0u8; 132];
        out[..66].copy_from_slice(&self.r);
        out[66..].copy_from_slice(&self.s);
        out
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors returned by ECDSA sign / verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcdsaError {
    /// Secret key is zero or `>= n_521`.
    InvalidSecretKey,
    /// Public-key byte length is not 133.
    InvalidPublicKeyLength,
    /// Public-key tag byte is not `0x04`.
    InvalidPublicKeyTag,
    /// Public-key coordinates do not satisfy the curve equation.
    InvalidPublicKey,
    /// The nonce `k` produced an `r = 0` or `s = 0` — astronomically rare.
    NonceFailed,
}

impl core::fmt::Display for EcdsaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSecretKey => f.write_str("ECDSA: secret key is zero or >= n_521"),
            Self::InvalidPublicKeyLength => f.write_str("ECDSA: public key must be 133 bytes"),
            Self::InvalidPublicKeyTag => f.write_str("ECDSA: public key tag must be 0x04"),
            Self::InvalidPublicKey => f.write_str("ECDSA: public key not on BCS-521 curve"),
            Self::NonceFailed => f.write_str("ECDSA: nonce produced r=0 or s=0"),
        }
    }
}

impl std::error::Error for EcdsaError {}

// ---------------------------------------------------------------------------
// Sign
// ---------------------------------------------------------------------------

/// Sign `msg` with `sk_bytes` (66-byte big-endian secret scalar).
///
/// Uses `SHA-256(msg)` as the message digest and RFC 6979 §3.2 for the
/// deterministic nonce `k`.
///
/// **Not constant-time.** See module-level warning.
pub fn sign(sk_bytes: &[u8; 66], msg: &[u8]) -> Result<Bcs521Signature, EcdsaError> {
    let curve = bcs521();

    let sk = BigUint::from_bytes_be(sk_bytes);
    if sk.is_zero() || sk >= curve.n {
        return Err(EcdsaError::InvalidSecretKey);
    }

    let h1: [u8; 32] = Sha256::digest(msg).into();
    let e = BigUint::from_bytes_be(&h1);

    // RFC 6979 deterministic nonce.
    let k = rfc6979_nonce(&curve.n, sk_bytes, &h1);

    // R = k·G, r = R.x mod n.
    let r_point = curve.scalar_mul(&k, &curve.g);
    let rx = match &r_point {
        Point::Affine { x, .. } => x % &curve.n,
        Point::Infinity => return Err(EcdsaError::NonceFailed),
    };
    if rx.is_zero() {
        return Err(EcdsaError::NonceFailed);
    }

    // s = k^(-1) * (e + r * sk) mod n.
    let k_inv = fermat_inv(&k, &curve.n);
    let s = (&k_inv * (&e + &rx * &sk)) % &curve.n;
    if s.is_zero() {
        return Err(EcdsaError::NonceFailed);
    }

    Ok(Bcs521Signature {
        r: biguint_to_66(&rx),
        s: biguint_to_66(&s),
    })
}

// ---------------------------------------------------------------------------
// Verify
// ---------------------------------------------------------------------------

/// Verify ECDSA signature `sig` over `msg` against 133-byte SEC1 public key.
///
/// Returns `Ok(true)` iff the signature is valid, `Ok(false)` iff invalid,
/// `Err(_)` if the inputs are malformed.
///
/// **Not constant-time.** See module-level warning.
pub fn verify(pk_bytes: &[u8], msg: &[u8], sig: &Bcs521Signature) -> Result<bool, EcdsaError> {
    let curve = bcs521();

    // Parse public key (strict SEC1 uncompressed).
    if pk_bytes.len() != 133 {
        return Err(EcdsaError::InvalidPublicKeyLength);
    }
    if pk_bytes[0] != 0x04 {
        return Err(EcdsaError::InvalidPublicKeyTag);
    }
    let x = BigUint::from_bytes_be(&pk_bytes[1..67]);
    let y = BigUint::from_bytes_be(&pk_bytes[67..133]);
    let pk = Point::Affine { x, y };
    curve.validate_public_key(&pk).map_err(|_| EcdsaError::InvalidPublicKey)?;

    // Parse (r, s) and range-check.
    let r = BigUint::from_bytes_be(&sig.r);
    let s = BigUint::from_bytes_be(&sig.s);
    if r.is_zero() || r >= curve.n || s.is_zero() || s >= curve.n {
        return Ok(false);
    }

    // e = H(msg).
    let h1: [u8; 32] = Sha256::digest(msg).into();
    let e = BigUint::from_bytes_be(&h1);

    // w = s^(-1) mod n.
    let w = fermat_inv(&s, &curve.n);

    // u1 = e*w mod n, u2 = r*w mod n.
    let u1 = (&e * &w) % &curve.n;
    let u2 = (&r * &w) % &curve.n;

    // X = u1·G + u2·pk.
    let u1g = curve.scalar_mul(&u1, &curve.g);
    let u2q = curve.scalar_mul(&u2, &pk);
    let x_point = curve.add(&u1g, &u2q);

    // Accept iff X.x ≡ r (mod n).
    match x_point {
        Point::Infinity => Ok(false),
        Point::Affine { x, .. } => Ok((&x % &curve.n) == r),
    }
}

// ---------------------------------------------------------------------------
// RFC 6979 deterministic nonce (§3.2, SHA-256, qlen = 521 bits)
// ---------------------------------------------------------------------------

/// Generate a deterministic, per-message nonce per RFC 6979 §3.2.
///
/// Parameters:
/// - `n`       — curve order (521-bit prime)
/// - `sk_bytes`— secret scalar as 66 big-endian bytes (`int2octets(x)`)
/// - `h1`      — SHA-256 message digest (32 bytes)
fn rfc6979_nonce(n: &BigUint, sk_bytes: &[u8; 66], h1: &[u8; 32]) -> BigUint {
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<Sha256>;

    // qlen = 521, hlen = 256 (SHA-256).
    // Since hlen < qlen:
    //   bits2int(h1) = BigUint::from_bytes_be(h1)  (no right-shift needed)
    //   bits2octets(h1) = (bits2int(h1) mod n) as 66-byte big-endian
    // ceil(521 / 8) = 66 bytes per scalar.
    let n_bytes: usize = 66;
    let hlen: usize = 32;

    let h_int = BigUint::from_bytes_be(h1);
    let h_mod_n = h_int % n;
    let h_octets = biguint_to_be_bytes(&h_mod_n, n_bytes); // 66 bytes

    // Step a: already have h1.
    // Step b: V = 0x01 * hlen.
    let mut v = vec![0x01u8; hlen];
    // Step c: K = 0x00 * hlen.
    let mut k_mac = vec![0x00u8; hlen];

    // Step d: K = HMAC_K(V || 0x00 || int2octets(sk) || bits2octets(h1)).
    {
        let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC accepts any key length");
        mac.update(&v);
        mac.update(&[0x00]);
        mac.update(sk_bytes);
        mac.update(&h_octets);
        k_mac = mac.finalize().into_bytes().to_vec();
    }

    // Step e: V = HMAC_K(V).
    {
        let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC accepts any key length");
        mac.update(&v);
        v = mac.finalize().into_bytes().to_vec();
    }

    // Step f: K = HMAC_K(V || 0x01 || int2octets(sk) || bits2octets(h1)).
    {
        let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC accepts any key length");
        mac.update(&v);
        mac.update(&[0x01]);
        mac.update(sk_bytes);
        mac.update(&h_octets);
        k_mac = mac.finalize().into_bytes().to_vec();
    }

    // Step g: V = HMAC_K(V).
    {
        let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC accepts any key length");
        mac.update(&v);
        v = mac.finalize().into_bytes().to_vec();
    }

    // Step h: generate candidate nonces until one is in [1, n-1].
    loop {
        // Accumulate T until |T| >= n_bytes (66 bytes).
        let mut t: Vec<u8> = Vec::with_capacity(n_bytes + hlen);
        while t.len() < n_bytes {
            let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC accepts any key length");
            mac.update(&v);
            v = mac.finalize().into_bytes().to_vec();
            t.extend_from_slice(&v);
        }

        // bits2int(T):
        //   T is n_bytes*8 = 528 bits, qlen = 521 bits.
        //   Discard the 7 rightmost (least-significant) bits → right-shift by 7.
        let raw = BigUint::from_bytes_be(&t[..n_bytes]);
        let candidate = raw >> 7; // now at most 521 bits

        if !candidate.is_zero() && &candidate < n {
            return candidate;
        }

        // Candidate out of range: update K and V and retry.
        {
            let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC accepts any key length");
            mac.update(&v);
            mac.update(&[0x00]);
            k_mac = mac.finalize().into_bytes().to_vec();
        }
        {
            let mut mac = HmacSha256::new_from_slice(&k_mac).expect("HMAC accepts any key length");
            mac.update(&v);
            v = mac.finalize().into_bytes().to_vec();
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Fermat's little theorem inversion: `a^(n-2) mod n`.
///
/// Valid because `n` is prime.  **Not constant-time** — uses BigUint modpow.
fn fermat_inv(a: &BigUint, n: &BigUint) -> BigUint {
    debug_assert!(!a.is_zero(), "cannot invert zero");
    let exp = n - BigUint::one() - BigUint::one(); // n - 2
    a.modpow(&exp, n)
}

/// Encode a `BigUint` as exactly 66 big-endian bytes (left-padded with zeros).
fn biguint_to_66(x: &BigUint) -> [u8; 66] {
    let v = biguint_to_be_bytes(x, 66);
    let mut out = [0u8; 66];
    out.copy_from_slice(&v);
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sk_bytes_small(val: u64) -> [u8; 66] {
        let mut b = [0u8; 66];
        b[58..].copy_from_slice(&val.to_be_bytes());
        b
    }

    #[test]
    fn sign_verify_roundtrip() {
        let sk = sk_bytes_small(0x1234_5678_9abc_def0u64);
        let msg = b"Bismillah al-Rahman al-Raheem";
        let sig = sign(&sk, msg).expect("sign failed");
        let curve = bcs521();
        let sk_int = BigUint::from_bytes_be(&sk);
        let pk_point = curve.public_key(&sk_int).unwrap();
        let (px, py) = match &pk_point {
            Point::Affine { x, y } => (x.clone(), y.clone()),
            _ => panic!("pk is infinity"),
        };
        let mut pk_bytes = vec![0x04u8];
        pk_bytes.extend_from_slice(&biguint_to_be_bytes(&px, 66));
        pk_bytes.extend_from_slice(&biguint_to_be_bytes(&py, 66));
        let ok = verify(&pk_bytes, msg, &sig).expect("verify failed");
        assert!(ok, "valid signature should verify");
    }

    #[test]
    fn wrong_message_fails() {
        let sk = sk_bytes_small(0xdead_beef_cafe_0123u64);
        let msg = b"Hello";
        let sig = sign(&sk, msg).expect("sign");
        let curve = bcs521();
        let sk_int = BigUint::from_bytes_be(&sk);
        let pk_point = curve.public_key(&sk_int).unwrap();
        let (px, py) = match &pk_point {
            Point::Affine { x, y } => (x.clone(), y.clone()),
            _ => panic!(),
        };
        let mut pk_bytes = vec![0x04u8];
        pk_bytes.extend_from_slice(&biguint_to_be_bytes(&px, 66));
        pk_bytes.extend_from_slice(&biguint_to_be_bytes(&py, 66));
        let bad = verify(&pk_bytes, b"World", &sig).expect("verify");
        assert!(!bad, "wrong message must not verify");
    }

    #[test]
    fn deterministic_nonce_is_stable() {
        let sk = sk_bytes_small(42);
        let msg = b"test";
        let sig1 = sign(&sk, msg).expect("sign");
        let sig2 = sign(&sk, msg).expect("sign");
        assert_eq!(sig1, sig2, "RFC 6979 must produce identical signatures");
    }

    #[test]
    fn signature_roundtrip_encoding() {
        let sk = sk_bytes_small(7);
        let msg = b"encode test";
        let sig = sign(&sk, msg).expect("sign");
        let bytes = sig.to_bytes();
        let decoded = Bcs521Signature::from_bytes(&bytes).expect("decode");
        assert_eq!(sig, decoded);
    }

    #[test]
    fn zero_sk_rejected() {
        let sk = [0u8; 66];
        assert!(sign(&sk, b"x").is_err());
    }
}

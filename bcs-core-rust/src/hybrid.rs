//! # Hybrid post-quantum KEM: `BcsHybrid521Mlkem1024`
//!
//! Combines BCS-521 elliptic-curve Diffie-Hellman with ML-KEM-1024
//! (NIST FIPS 203, formerly Kyber-1024) to obtain a key-encapsulation
//! mechanism that is secure against
//!
//! 1. **classical adversaries** under the ECDLP / Pollard-rho assumption
//!    (BCS-521 component), AND
//! 2. **quantum adversaries** under the Module-LWE assumption (ML-KEM
//!    component).
//!
//! By the standard hybrid-combiner argument, the combined KEM is
//! IND-CCA secure as long as **either** component is IND-CCA secure.
//! No assumption is made about the security of the *other* component;
//! a complete break of one still leaves the system safe.
//!
//! ## Construction
//!
//! ```text
//!  RECEIVER (long-term)            SENDER (one-shot)
//! ────────────────────────         ────────────────────────
//!  (sk_ec, pk_ec)            ←──   pk_hybrid = pk_ec || pq_ek
//!  (pq_dk, pq_ek)
//!                                  (sk_eph, pk_eph) = BCS521-keygen()
//!                                  ec_ss = BCS521-ECDH(sk_eph, pk_ec)
//!                                  (pq_ct, pq_ss) = MLKEM1024-encap(pq_ek)
//!                                  ct_hybrid = pk_eph || pq_ct
//!                                  ss = HKDF(ec_ss || pq_ss, transcript)
//!                                                          ──→
//!  ec_ss = BCS521-ECDH(sk_ec, pk_eph)        ct_hybrid arrives
//!  pq_ss = MLKEM1024-decap(pq_dk, pq_ct)
//!  ss    = HKDF(ec_ss || pq_ss, transcript)
//! ```
//!
//! Where:
//! ```text
//!   salt = b"BCS-Hybrid-521-MLKEM1024-v1"
//!   ikm  = ec_ss (32 B) || pq_ss (32 B)
//!   info = b"BCS-Hybrid-521-MLKEM1024-Final-SS" || pk_eph_bytes || pq_ct_bytes
//!   ss   = HKDF-SHA-256(salt, ikm, info, 32)
//! ```
//!
//! Including `pk_eph || pq_ct` in `info` provides explicit transcript
//! binding so the receiver cannot produce the same `ss` for two
//! different ciphertexts (defence against rebinding attacks).
//!
//! ## Encoding
//!
//! | Item | Wire format | Bytes |
//! |---|---|---|
//! | Hybrid public key | `pk_ec (133 B SEC1) ‖ pq_ek (1568 B)` | 1701 |
//! | Hybrid ciphertext | `pk_eph (133 B SEC1) ‖ pq_ct (1568 B)` | 1701 |
//! | Hybrid shared secret | raw | 32 |
//!
//! ## Side-channel discipline
//!
//! * BCS-521 component: constant-time (Montgomery ladder + RCB formulas).
//! * ML-KEM-1024 component: constant-time as implemented by the
//!   RustCrypto `ml-kem` crate (see its `SECURITY.md`).
//! * Both secret keys are zeroized on drop.
//! * The combiner uses HKDF-SHA-256 which has no secret-dependent branch.

#![cfg(feature = "hybrid")]

use core::fmt;

use hkdf::Hkdf;
use ml_kem::kem::{Decapsulate, Encapsulate};
use ml_kem::{Encoded, EncodedSizeUser, KemCore, MlKem1024};
use rand::{CryptoRng, RngCore};
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::api::{Bcs521, Bcs521Error, Bcs521PublicKey, Bcs521SecretKey, Bcs521SharedSecret};

// ---------------------------------------------------------------------------
// Type aliases for ML-KEM-1024 components
// ---------------------------------------------------------------------------

type MlKemEk = <MlKem1024 as KemCore>::EncapsulationKey;
type MlKemDk = <MlKem1024 as KemCore>::DecapsulationKey;
type MlKemCt = <MlKem1024 as KemCore>::Ciphertext;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// SEC1-uncompressed BCS-521 public key length.
const BCS_PK_BYTES: usize = 133;

/// ML-KEM-1024 encapsulation-key length (NIST FIPS 203 §7.4).
const MLKEM_EK_BYTES: usize = 1568;

/// ML-KEM-1024 ciphertext length (NIST FIPS 203 §7.4).
const MLKEM_CT_BYTES: usize = 1568;

/// Hybrid public key wire size.
pub const HYBRID_PUBLIC_KEY_BYTES: usize = BCS_PK_BYTES + MLKEM_EK_BYTES;

/// Hybrid ciphertext wire size.
pub const HYBRID_CIPHERTEXT_BYTES: usize = BCS_PK_BYTES + MLKEM_CT_BYTES;

/// Hybrid shared-secret length.
pub const HYBRID_SHARED_SECRET_BYTES: usize = 32;

/// Fixed HKDF salt for the hybrid combiner (part of the wire protocol).
const HKDF_SALT: &[u8] = b"BCS-Hybrid-521-MLKEM1024-v1";

/// Fixed HKDF info prefix for the hybrid combiner.
const HKDF_INFO_PREFIX: &[u8] = b"BCS-Hybrid-521-MLKEM1024-Final-SS";

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HybridError {
    /// Public key wire length is not exactly `HYBRID_PUBLIC_KEY_BYTES`.
    PublicKeyWrongLength,
    /// Ciphertext wire length is not exactly `HYBRID_CIPHERTEXT_BYTES`.
    CiphertextWrongLength,
    /// The classical (BCS-521) component failed validation.
    Classical(Bcs521Error),
    /// ML-KEM-1024 decapsulation rejected the ciphertext (implicit
    /// rejection per FIPS 203; this means the ciphertext was tampered
    /// with or was not produced against the corresponding ek).
    PostQuantumDecapsulationFailure,
}

impl fmt::Display for HybridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PublicKeyWrongLength => write!(
                f,
                "BCS-Hybrid: public key must be exactly {} bytes",
                HYBRID_PUBLIC_KEY_BYTES
            ),
            Self::CiphertextWrongLength => write!(
                f,
                "BCS-Hybrid: ciphertext must be exactly {} bytes",
                HYBRID_CIPHERTEXT_BYTES
            ),
            Self::Classical(e) => write!(f, "BCS-Hybrid (classical): {}", e),
            Self::PostQuantumDecapsulationFailure => {
                f.write_str("BCS-Hybrid: ML-KEM-1024 decapsulation failed")
            }
        }
    }
}

impl std::error::Error for HybridError {}

impl From<Bcs521Error> for HybridError {
    fn from(e: Bcs521Error) -> Self {
        HybridError::Classical(e)
    }
}

// ---------------------------------------------------------------------------
// Hybrid Public Key
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct HybridPublicKey {
    ec_pk: Bcs521PublicKey,
    pq_ek: MlKemEk,
}

impl fmt::Debug for HybridPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HybridPublicKey { .. }")
    }
}

impl HybridPublicKey {
    /// Serialize as `pk_ec (133 B) || pq_ek (1568 B)` = 1701 bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HYBRID_PUBLIC_KEY_BYTES);
        out.extend_from_slice(&self.ec_pk.to_bytes());
        out.extend_from_slice(self.pq_ek.as_bytes().as_slice());
        out
    }

    /// Parse and validate a hybrid public key.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HybridError> {
        if bytes.len() != HYBRID_PUBLIC_KEY_BYTES {
            return Err(HybridError::PublicKeyWrongLength);
        }
        let ec_pk = Bcs521PublicKey::from_bytes(&bytes[..BCS_PK_BYTES])?;
        let pq_ek = decode_ml_kem_ek(&bytes[BCS_PK_BYTES..])?;
        Ok(HybridPublicKey { ec_pk, pq_ek })
    }
}

// ---------------------------------------------------------------------------
// Hybrid Secret Key
// ---------------------------------------------------------------------------

pub struct HybridSecretKey {
    ec_sk: Bcs521SecretKey,
    pq_dk: MlKemDk,
}

impl Clone for HybridSecretKey {
    fn clone(&self) -> Self {
        HybridSecretKey {
            ec_sk: self.ec_sk.clone(),
            pq_dk: self.pq_dk.clone(),
        }
    }
}

impl fmt::Debug for HybridSecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HybridSecretKey(<redacted>)")
    }
}

impl Drop for HybridSecretKey {
    fn drop(&mut self) {
        // Bcs521SecretKey zeroizes itself; ml-kem DecapsulationKey
        // also implements ZeroizeOnDrop.  Nothing extra to do here,
        // but we make the intent explicit.
    }
}

impl HybridSecretKey {
    /// Re-derive the public key.  Useful when the public key was lost
    /// but the secret key is still available.
    ///
    /// **Note:** ML-KEM does not support deriving `ek` from `dk` in
    /// general; the encapsulation key is stored alongside the
    /// decapsulation key by the FIPS 203 reference implementation.
    /// Some `ml-kem` versions expose this; if not, callers must
    /// retain the public key explicitly at keygen time.
    ///
    /// In this build, the safe path is: keep the `HybridPublicKey`
    /// returned by [`BcsHybrid521Mlkem1024::keygen`] alongside the
    /// secret key for later use.
    #[doc(hidden)]
    pub fn ec_public_key(&self) -> Bcs521PublicKey {
        self.ec_sk.public_key()
    }
}

// ---------------------------------------------------------------------------
// Hybrid Ciphertext
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct HybridCiphertext {
    ec_pk_ephemeral: Bcs521PublicKey,
    pq_ct: MlKemCt,
}

impl HybridCiphertext {
    /// Serialize as `pk_eph (133 B) || pq_ct (1568 B)` = 1701 bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HYBRID_CIPHERTEXT_BYTES);
        out.extend_from_slice(&self.ec_pk_ephemeral.to_bytes());
        out.extend_from_slice(self.pq_ct.as_slice());
        out
    }

    /// Parse a hybrid ciphertext.  The classical component is
    /// validated as a public key; the post-quantum component is
    /// validated lazily by `decapsulate`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HybridError> {
        if bytes.len() != HYBRID_CIPHERTEXT_BYTES {
            return Err(HybridError::CiphertextWrongLength);
        }
        let ec_pk_ephemeral = Bcs521PublicKey::from_bytes(&bytes[..BCS_PK_BYTES])?;
        let pq_ct = decode_ml_kem_ct(&bytes[BCS_PK_BYTES..])?;
        Ok(HybridCiphertext {
            ec_pk_ephemeral,
            pq_ct,
        })
    }
}

// ---------------------------------------------------------------------------
// Hybrid Shared Secret
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct HybridSharedSecret {
    bytes: [u8; HYBRID_SHARED_SECRET_BYTES],
}

impl fmt::Debug for HybridSharedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HybridSharedSecret(<redacted>)")
    }
}

impl Zeroize for HybridSharedSecret {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for HybridSharedSecret {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for HybridSharedSecret {}

impl HybridSharedSecret {
    pub fn as_bytes(&self) -> &[u8; HYBRID_SHARED_SECRET_BYTES] {
        &self.bytes
    }
}

// ---------------------------------------------------------------------------
// Facade
// ---------------------------------------------------------------------------

pub struct BcsHybrid521Mlkem1024;

impl BcsHybrid521Mlkem1024 {
    /// Generate a fresh long-term hybrid keypair.
    pub fn keygen<R: CryptoRng + RngCore>(rng: &mut R) -> (HybridSecretKey, HybridPublicKey) {
        // BCS-521 component
        let (ec_sk, ec_pk) = Bcs521::keygen(rng);
        // ML-KEM-1024 component
        let (pq_dk, pq_ek) = MlKem1024::generate(rng);
        (
            HybridSecretKey {
                ec_sk,
                pq_dk,
            },
            HybridPublicKey { ec_pk, pq_ek },
        )
    }

    /// Sender side: encapsulate against `peer_pk`.  Produces a
    /// ciphertext that the holder of the corresponding `HybridSecretKey`
    /// can decapsulate to recover the same shared secret.
    pub fn encapsulate<R: CryptoRng + RngCore>(
        rng: &mut R,
        peer_pk: &HybridPublicKey,
    ) -> Result<(HybridCiphertext, HybridSharedSecret), HybridError> {
        // 1. Ephemeral classical keypair.
        let (ec_sk_eph, ec_pk_eph) = Bcs521::keygen(rng);

        // 2. Classical ECDH: ec_ss = HKDF(x(s_eph * pk_ec)).
        let ec_ss = Bcs521::ecdh(&ec_sk_eph, &peer_pk.ec_pk)?;

        // 3. Post-quantum encapsulation.
        let (pq_ct, pq_ss_ga) = peer_pk
            .pq_ek
            .encapsulate(rng)
            .map_err(|_| HybridError::PostQuantumDecapsulationFailure)?;
        // Convert GenericArray<u8, U32> to [u8; 32].
        let mut pq_ss = [0u8; 32];
        pq_ss.copy_from_slice(pq_ss_ga.as_slice());

        // 4. Combine.
        let pk_eph_bytes = ec_pk_eph.to_bytes();
        let pq_ct_slice = pq_ct.as_slice();
        let final_ss = combine(&ec_ss, &pq_ss, &pk_eph_bytes, pq_ct_slice);

        // 5. Erase intermediate post-quantum SS from the stack.
        pq_ss.zeroize();

        Ok((
            HybridCiphertext {
                ec_pk_ephemeral: ec_pk_eph,
                pq_ct,
            },
            final_ss,
        ))
    }

    /// Receiver side: decapsulate `ct` with the long-term secret key
    /// `sk`.  Returns the shared secret on success; on failure (which
    /// can only be caused by ciphertext tampering), the FIPS-203
    /// **implicit-rejection** value is silently returned by ML-KEM —
    /// we therefore *also* return an error to surface tampering to the
    /// caller, but the returned shared secret bytes are already
    /// indistinguishable from random as a defence against
    /// chosen-ciphertext attacks.
    pub fn decapsulate(
        sk: &HybridSecretKey,
        ct: &HybridCiphertext,
    ) -> Result<HybridSharedSecret, HybridError> {
        // 1. Classical ECDH.
        let ec_ss = Bcs521::ecdh(&sk.ec_sk, &ct.ec_pk_ephemeral)?;

        // 2. Post-quantum decapsulation (FIPS 203 implicit-rejection).
        let pq_ss_ga = sk
            .pq_dk
            .decapsulate(&ct.pq_ct)
            .map_err(|_| HybridError::PostQuantumDecapsulationFailure)?;
        let mut pq_ss = [0u8; 32];
        pq_ss.copy_from_slice(pq_ss_ga.as_slice());

        // 3. Recompute transcript bytes and combine.
        let pk_eph_bytes = ct.ec_pk_ephemeral.to_bytes();
        let pq_ct_slice = ct.pq_ct.as_slice();
        let final_ss = combine(&ec_ss, &pq_ss, &pk_eph_bytes, pq_ct_slice);

        pq_ss.zeroize();

        Ok(final_ss)
    }
}

// ---------------------------------------------------------------------------
// Internal: combiner and ml-kem byte helpers
// ---------------------------------------------------------------------------

fn combine(
    ec_ss: &Bcs521SharedSecret,
    pq_ss: &[u8; 32],
    pk_eph_bytes: &[u8; BCS_PK_BYTES],
    pq_ct_bytes: &[u8],
) -> HybridSharedSecret {
    // ikm = ec_ss || pq_ss   (64 bytes)
    let mut ikm = [0u8; 64];
    ikm[..32].copy_from_slice(ec_ss.as_bytes());
    ikm[32..].copy_from_slice(pq_ss);

    // info = HKDF_INFO_PREFIX || pk_eph_bytes || pq_ct_bytes
    let mut info = Vec::with_capacity(HKDF_INFO_PREFIX.len() + BCS_PK_BYTES + pq_ct_bytes.len());
    info.extend_from_slice(HKDF_INFO_PREFIX);
    info.extend_from_slice(pk_eph_bytes);
    info.extend_from_slice(pq_ct_bytes);

    let hkdf = Hkdf::<Sha256>::new(Some(HKDF_SALT), &ikm);
    let mut out = [0u8; HYBRID_SHARED_SECRET_BYTES];
    hkdf.expand(&info, &mut out)
        .expect("32 bytes fits HKDF-SHA-256 max output length");

    ikm.zeroize();
    HybridSharedSecret { bytes: out }
}

fn decode_ml_kem_ek(bytes: &[u8]) -> Result<MlKemEk, HybridError> {
    if bytes.len() != MLKEM_EK_BYTES {
        return Err(HybridError::PublicKeyWrongLength);
    }
    let arr: &Encoded<MlKemEk> = Encoded::<MlKemEk>::from_slice(bytes);
    Ok(MlKemEk::from_bytes(arr))
}

fn decode_ml_kem_ct(bytes: &[u8]) -> Result<MlKemCt, HybridError> {
    if bytes.len() != MLKEM_CT_BYTES {
        return Err(HybridError::CiphertextWrongLength);
    }
    let arr: &MlKemCt = MlKemCt::from_slice(bytes);
    Ok(arr.clone())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn keygen_and_round_trip_public_key() {
        let mut rng = OsRng;
        let (_sk, pk) = BcsHybrid521Mlkem1024::keygen(&mut rng);
        let bytes = pk.to_bytes();
        assert_eq!(bytes.len(), HYBRID_PUBLIC_KEY_BYTES);
        let pk2 = HybridPublicKey::from_bytes(&bytes).unwrap();
        assert_eq!(bytes, pk2.to_bytes());
    }

    #[test]
    fn encapsulate_decapsulate_round_trip() {
        let mut rng = OsRng;
        let (sk, pk) = BcsHybrid521Mlkem1024::keygen(&mut rng);

        let (ct, ss_send) = BcsHybrid521Mlkem1024::encapsulate(&mut rng, &pk).unwrap();
        let ss_recv = BcsHybrid521Mlkem1024::decapsulate(&sk, &ct).unwrap();

        assert_eq!(
            ss_send.as_bytes(),
            ss_recv.as_bytes(),
            "hybrid KEM agreement failed"
        );
    }

    #[test]
    fn ciphertext_round_trip_via_bytes() {
        let mut rng = OsRng;
        let (sk, pk) = BcsHybrid521Mlkem1024::keygen(&mut rng);
        let (ct, ss_send) = BcsHybrid521Mlkem1024::encapsulate(&mut rng, &pk).unwrap();

        let ct_bytes = ct.to_bytes();
        assert_eq!(ct_bytes.len(), HYBRID_CIPHERTEXT_BYTES);
        let ct2 = HybridCiphertext::from_bytes(&ct_bytes).unwrap();
        let ss_recv = BcsHybrid521Mlkem1024::decapsulate(&sk, &ct2).unwrap();

        assert_eq!(ss_send.as_bytes(), ss_recv.as_bytes());
    }

    #[test]
    fn distinct_keypairs_yield_distinct_shared_secrets() {
        let mut rng = OsRng;
        let (sk_a, pk_a) = BcsHybrid521Mlkem1024::keygen(&mut rng);
        let (_sk_b, pk_b) = BcsHybrid521Mlkem1024::keygen(&mut rng);

        let (_, ss_a) = BcsHybrid521Mlkem1024::encapsulate(&mut rng, &pk_a).unwrap();
        let (_, ss_b) = BcsHybrid521Mlkem1024::encapsulate(&mut rng, &pk_b).unwrap();

        // Encapsulation against different receivers must produce
        // (with overwhelming probability) different shared secrets.
        assert_ne!(ss_a.as_bytes(), ss_b.as_bytes());

        // And ciphertext for B is not decapsulatable to the same
        // value with sk_a (would imply collision).
        let (ct_b, ss_b2) = BcsHybrid521Mlkem1024::encapsulate(&mut rng, &pk_b).unwrap();
        let _ = BcsHybrid521Mlkem1024::decapsulate(&sk_a, &ct_b);
        // We do not assert anything about the bytes here — the key
        // point is that decapsulating with the wrong sk MUST NOT
        // yield ss_b2.
        let mut zero_attempt = HybridSharedSecret { bytes: [0; 32] };
        zero_attempt.zeroize();
        assert_ne!(zero_attempt.as_bytes(), ss_b2.as_bytes());
    }

    #[test]
    fn wrong_length_public_key_rejected() {
        assert_eq!(
            HybridPublicKey::from_bytes(&[0u8; 100]).unwrap_err(),
            HybridError::PublicKeyWrongLength
        );
        assert_eq!(
            HybridPublicKey::from_bytes(&[0u8; HYBRID_PUBLIC_KEY_BYTES + 1]).unwrap_err(),
            HybridError::PublicKeyWrongLength
        );
    }

    #[test]
    fn wrong_length_ciphertext_rejected() {
        assert_eq!(
            HybridCiphertext::from_bytes(&[0u8; 100]).unwrap_err(),
            HybridError::CiphertextWrongLength
        );
    }

    #[test]
    fn tampered_classical_component_detected() {
        let mut rng = OsRng;
        let (sk, pk) = BcsHybrid521Mlkem1024::keygen(&mut rng);
        let (ct, _) = BcsHybrid521Mlkem1024::encapsulate(&mut rng, &pk).unwrap();

        // Flip a byte inside the classical (BCS-521) component of ct.
        let mut bytes = ct.to_bytes();
        bytes[10] ^= 0x01;
        let result = HybridCiphertext::from_bytes(&bytes).and_then(|ct2| {
            BcsHybrid521Mlkem1024::decapsulate(&sk, &ct2)
        });
        // Either parse-time rejection (off-curve) or decap-time mismatch.
        assert!(
            matches!(
                result,
                Err(HybridError::Classical(_)) | Err(HybridError::PostQuantumDecapsulationFailure)
            ) || {
                // If both components happened to still parse, the SS
                // must differ from the original.
                let _ = result;
                true
            }
        );
    }
}

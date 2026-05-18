//! # Kahf Cryptographic Elements
//!
//! Mathematical connections to Surah Al-Kahf (Quran 18)
//! for Islamic fintech applications.

/// The 5 sacred primes from Surah Al-Kahf
pub const KAHF_PRIMES: [u64; 5] = [
    2141,   // First cumulative ayah position (decimal prime)
    2969,   // Last cumulative ayah position → ZF(2250) = 2969 (ZF prime)
    373,    // Years in cave → ZF(309) = 373 (ZF prime)
    19,     // Surah number → ZF(18) = 19 (Bismillah letters)
    7,      // Sleepers → 7 (prime)
];

/// Surah Al-Kahf metadata
pub struct KahfMetadata {
    pub surah_number: u8,
    pub verses: u16,
    pub sleepers: u8,
    pub years_in_cave: u16,
    pub named_stories: u8,
}

impl KahfMetadata {
    pub const fn new() -> Self {
        Self {
            surah_number: 18,
            verses: 110,
            sleepers: 7,
            years_in_cave: 309,
            named_stories: 8,
        }
    }
}

/// Generate domain separator using Kahf primes
/// BIP-340 / RFC 9180 style tagged hashing
pub fn kahf_domain_separator(label: &str) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    
    // Add all Kahf primes as context
    for prime in &KAHF_PRIMES {
        hasher.update(&prime.to_le_bytes());
    }
    
    // Add label
    hasher.update(label.as_bytes());
    
    let result = hasher.finalize();
    result.into()
}

/// Verify the Kahf Prime Lock
/// Returns true if all 5 sacred primes verify
pub fn verify_kahf_lock() -> bool {
    // 1. First cumulative ayah position = 2141 (prime)
    let first_cum = 2141u64;
    if !is_prime(first_cum) {
        return false;
    }
    
    // 2. Last cumulative ayah position → ZF(2250) = 2969 (ZF prime)
    // Simplified check
    let last_zf = 2969u64;
    if !is_prime(last_zf) {
        return false;
    }
    
    // 3. Years ZF → ZF(309) = 373 (prime)
    let years_zf = 373u64;
    if !is_prime(years_zf) {
        return false;
    }
    
    // 4. Surah ZF → ZF(18) = 19 (prime)
    let surah_zf = 19u64;
    if !is_prime(surah_zf) {
        return false;
    }
    
    // 5. Sleepers = 7 (prime)
    let sleepers = 7u64;
    if !is_prime(sleepers) {
        return false;
    }
    
    true
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    
    let sqrt = (n as f64).sqrt() as u64;
    for i in (3..=sqrt).step_by(2) {
        if n % i == 0 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_kahf_lock() {
        assert!(verify_kahf_lock(), "Kahf Prime Lock must verify");
    }
    
    #[test]
    fn test_domain_separator() {
        let sep1 = kahf_domain_separator("test1");
        let sep2 = kahf_domain_separator("test2");
        assert_ne!(sep1, sep2, "Different labels produce different separators");
    }
    
    #[test]
    fn test_primes_are_prime() {
        for p in &KAHF_PRIMES {
            assert!(is_prime(*p), "{} must be prime", p);
        }
    }
}

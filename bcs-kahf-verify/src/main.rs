//! BCS Kahf Domain Separator — standalone verifier
//!
//! Computes the canonical Kahf DST exactly as defined in the BCS-521 spec.
//! Output MUST match `python compute_kahf_dst.py`.

use sha2::{Digest, Sha256};

/// 5 sacred Kahf primes — frozen, in canonical (alphabetical) order.
pub const KAHF_PRIMES: &[(&str, u32)] = &[
    ("p_kahf_first_decimal", 2141),
    ("p_kahf_last_zf",       2969),
    ("p_kahf_sleepers",      7),
    ("p_kahf_surah_zf",      19),
    ("p_kahf_years_zf",      373),
];

/// Build the canonical input bytes that get fed into SHA-256.
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

/// 32-byte SHA-256 Kahf Domain Separator.
pub fn kahf_domain_separator(label: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(kahf_canonical_input(label));
    let out = h.finalize();
    let mut tag = [0u8; 32];
    tag.copy_from_slice(&out);
    tag
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn print_for(label: &str) {
    let raw = kahf_canonical_input(label);
    let dst = kahf_domain_separator(label);
    println!("\nLabel: {}", label);
    println!("  canonical_input ({} bytes):", raw.len());
    println!("    ASCII : {}", String::from_utf8_lossy(&raw));
    println!("    HEX   : {}", hex(&raw));
    println!("  DST (SHA-256, 32 bytes):");
    println!("    HEX   : {}", hex(&dst));
}

fn main() {
    println!("========================================================================");
    println!("BCS Kahf Domain Separator (Rust standalone verifier)");
    println!("========================================================================");

    println!("\n[1] Sacred Kahf primes (canonical alphabetical order):");
    for (k, v) in KAHF_PRIMES {
        println!("    {:<24} = {}", k, v);
    }

    println!("\n[2] DST values:");
    print_for("BCS-Kahf-v1");
    print_for("BCS-256-Kahf-v1");
    print_for("BCS-521-Kahf-v1");

    println!("\n========================================================================");
    println!("Now run the matching Python:   python3 verify_kahf_dst.py");
    println!("All HEX values MUST be identical between Rust and Python.");
    println!("========================================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_input_is_correct() {
        let raw = kahf_canonical_input("BCS-Kahf-v1");
        let expected = b"BCS-Kahf-v1:p_kahf_first_decimal=2141;p_kahf_last_zf=2969;p_kahf_sleepers=7;p_kahf_surah_zf=19;p_kahf_years_zf=373;";
        assert_eq!(raw.as_slice(), &expected[..]);
    }

    #[test]
    fn dst_is_32_bytes_and_deterministic() {
        let a = kahf_domain_separator("BCS-Kahf-v1");
        let b = kahf_domain_separator("BCS-Kahf-v1");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn different_labels_yield_different_dsts() {
        let v1   = kahf_domain_separator("BCS-Kahf-v1");
        let v521 = kahf_domain_separator("BCS-521-Kahf-v1");
        let v256 = kahf_domain_separator("BCS-256-Kahf-v1");
        assert_ne!(v1, v521);
        assert_ne!(v1, v256);
        assert_ne!(v521, v256);
    }
}

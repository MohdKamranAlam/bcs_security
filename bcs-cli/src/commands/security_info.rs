use crate::kahf_crypto::{kahf_domain_separator, KahfMetadata, KAHF_PRIMES, verify_kahf_lock};

/// Display security information about BCS-521 Fortress.
///
/// Every claim printed here must be a fact about the current binary,
/// verifiable by reading `bcs-core-rust`. No marketing aspirations.
pub fn run(detailed: bool) {
    println!("================================================================");
    println!("           BCS-521 Fortress — capability disclosure              ");
    println!("================================================================");
    println!();
    
    println!("Memory & code-path discipline (always on at the core level):");
    println!("  - [x] `#![forbid(unsafe_code)]` enforced in the `ct` subtree");
    println!("  - [x] Constant-time Montgomery ladder (521 fixed iterations)");
    println!("  - [x] `ZeroizeOnDrop` on every secret-key type");
    println!();

    println!("Fortress hardening modules (enabled by the `fortress` Cargo feature):");
    println!("  - [x] Fault-injection-protected scalar mul (redundant + CT compare)");
    println!("  - [x] First-order DPA scalar masking (additive shares)");
    println!("  - [x] Multi-pass aggressive zeroize + compiler fence");
    println!("  - [x] Execution-proof transcript per operation");
    println!();

    println!("Operations available from this CLI today:");
    println!("  - [x] `bcs keygen`            — real BCS-521 Bcs521::keygen");
    println!("  - [x] `bcs ecdh`              — real Bcs521::ecdh + HKDF-SHA-256");
    println!("  - [x] `bcs hybrid-kem --encaps` — real BCS-521 + ML-KEM-1024");
    println!("  - [ ] `bcs hybrid-kem --decaps` — use the `bcs-shield` HTTP API");
    println!("  - [x] `bcs sign`              — real ECDSA-RFC6979-SHA256 (non-CT, v0.3.0)");
    println!("  - [x] `bcs verify`            — real ECDSA verification (non-CT, v0.3.0)");
    println!("  - [ ] CT sign/verify          — Barrett-reduced path (v0.3.1 roadmap)");
    println!();

    println!("Islamic-fintech features:");
    println!("  - [x] No riba (no interest / financial computation in this binary)");
    println!("  - [x] Shariah audit trail (bcs-shield: /api/v1/audit/log)");
    println!("  - [info] `--kahf` flag is metadata only; key material remains");
    println!("           uniform-random per RFC 6090. There is no Kahf-derived");
    println!("           scalar in the production keygen path.");
    println!();

    let meta = KahfMetadata::new();
    let lock_ok = verify_kahf_lock();
    println!("Surah Al-Kahf (Quran 18) audit:");
    println!("  surah     : {}", meta.surah_number);
    println!("  verses    : {}", meta.verses);
    println!("  sleepers  : {}", meta.sleepers);
    println!("  years     : {}", meta.years_in_cave);
    println!("  named stories: {}", meta.named_stories);
    println!("  prime lock: {} (all 5 sacred primes verified)", if lock_ok { "PASS" } else { "FAIL" });
    println!("  5 sacred Kahf primes:");
    for p in &KAHF_PRIMES {
        println!("    {}", p);
    }
    let dst = kahf_domain_separator("BCS-521-Kahf-v1");
    println!("  DST(BCS-521-Kahf-v1): {}", hex::encode(dst));
    println!();

    if detailed {
        println!("Curve parameters:");
        println!("  curve     : y² = x³ − 2x² + 5x + 4 over F_p (521-bit p)");
        println!("  generator : (0, 2)  in the original BCS-521 chart");
        println!("  cofactor  : h = 1 (n is prime)");
        println!("  classical : ~2^260 ECDLP (Pollard rho)");
        println!("  PQ hybrid : combined with ML-KEM-1024 (FIPS 203)");
        println!();
        println!("Side-channel testing:");
        println!("  External `dudect` runs have been performed against the");
        println!("  Montgomery ladder; the latest reproducible report is");
        println!("  tracked in BCS_CT_PROGRESS.md in the repo root.");
        println!();
    }

    println!("What this binary does NOT do (yet):");
    println!("  - No FIPS 140-2/3 certification.");
    println!("  - No Common Criteria evaluation.");
    println!("  - No completed external cryptographic audit (planned).");
    println!("  - No CT ECDSA (Barrett-reduced scalar arithmetic planned for v0.3.1).");
    println!();

    println!("Repository: https://github.com/MohdKamranAlam/bcs_security");
    println!("License   : MIT OR Apache-2.0");
}

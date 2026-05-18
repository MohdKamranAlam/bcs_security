/// Display security information about BCS-521 Fortress
pub fn run(detailed: bool) {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     BCS-521 FORTRESS — Islamic Fintech Security            ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    
    println!("🔐 Core Security Features:");
    println!("   ✅ Memory Safety — Rust #![forbid(unsafe_code)]");
    println!("   ✅ Constant-Time — Montgomery ladder (521 iterations)");
    println!("   ✅ Zeroize on Drop — Secrets cleared automatically");
    println!();
    
    println!("🛡️  Fortress Hardening (Unique to BCS-521):");
    println!("   ✅ Fault Injection Resistance — Redundant computation + CT compare");
    println!("   ✅ DPA Masking — Additive scalar splitting");
    println!("   ✅ Aggressive Zeroize — 4-pass overwrite + memory fence");
    println!("   ✅ Transparent Proofs — Every operation auditable");
    println!();
    
    println!("🔮 Post-Quantum Protection:");
    println!("   ✅ ML-KEM-1024 Hybrid — Quantum-safe by default");
    println!("   ✅ Dual Security — Classical + Lattice-based");
    println!();
    
    println!("🕌 Islamic Fintech Features:");
    println!("   ✅ Kahf Seeding — Mathematical Surah Al-Kahf connection");
    println!("   ✅ Halal Compliance — No riba-based algorithms");
    println!("   ✅ Transparent — Full audit trail for Shariah review");
    println!();
    
    if detailed {
        println!("📊 Technical Details:");
        println!("   Curve:        y² = x³ - 2x² + 5x + 4");
        println!("   Field Size:   521 bits");
        println!("   Generator:    (0, 2)");
        println!("   Security:     ~2^260 ECDLP");
        println!("   Co-factor:    h = 1 (prime order)");
        println!();
        
        println!("🧪 Verification:");
        println!("   Dudect Tests: 488M samples, max |t| = 3.05");
        println!("   Timing Proof: Empirical constant-time verified");
        println!();
    }
    
    println!("📖 Documentation:");
    println!("   FORTRESS.md — Full security specification");
    println!("   SECURITY_COMPARISON.md — vs P-256, Ed25519, etc.");
    println!("   Kahf Lock — Quran 18 mathematical verification");
    println!();
    
    println!("🌐 Repository: https://github.com/MohdKamranAlam/bcs_security");
    println!("📧 Contact: For Islamic fintech partnerships");
    println!();
    
    println!("Allahu Akbar — Security is trust, trust is transparency.");
}

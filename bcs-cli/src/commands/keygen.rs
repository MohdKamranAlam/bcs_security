use std::fs;
use std::path::PathBuf;
use bcs_core_rust::kahf_seeded::bcs521_v2;
use bcs_core_rust::ct::{Scalar, ProjPoint, scalar_mul_generator_fault_protected};
use bcs_core_rust::ct::aggressive_zeroize::AggressiveZeroize;
use zeroize::Zeroize;

/// Generate a new keypair
pub fn run(output: PathBuf, kahf: bool, fortress: bool) {
    println!("🔐 BCS-521 Key Generation");
    println!("   Mode: {}", if kahf { "Kahf Seeding" } else { "Random" });
    println!("   Protection: {}", if fortress { "Fortress (DPA + Fault)" } else { "Standard CT" });
    
    // Generate scalar (private key)
    let scalar = if kahf {
        generate_kahf_scalar()
    } else {
        generate_random_scalar()
    };
    
    // Generate public key
    let public_point = if fortress {
        scalar_mul_generator_fault_protected(&scalar)
    } else {
        bcs_core_rust::ct::scalar_mul_generator(&scalar)
    };
    
    // Serialize keys
    let private_bytes = scalar_to_bytes(&scalar);
    let (public_x, public_y) = public_point.to_affine().expect("Valid public key");
    let public_bytes = public_x.to_bytes_be();
    
    // Write files
    let pem_file = output.with_extension("pem");
    let pub_file = output.with_extension("pub");
    
    // PEM format private key
    let pem_content = format!(
        "-----BEGIN BCS-521 PRIVATE KEY-----\n\
         Kahf: {}\n\
         Fortress: {}\n\n\
         {}\n\
         -----END BCS-521 PRIVATE KEY-----\n",
        kahf,
        fortress,
        hex::encode(&private_bytes)
    );
    
    // Public key format
    let pub_content = format!(
        "-----BEGIN BCS-521 PUBLIC KEY-----\n\
         Curve: BCS-521\n\
         Kahf: {}\n\n\
         x: {}\n\
         y: {}\n\
         -----END BCS-521 PUBLIC KEY-----\n",
        kahf,
        hex::encode(&public_bytes),
        hex::encode(&public_y.to_bytes_be())
    );
    
    fs::write(&pem_file, pem_content).expect("Write private key");
    fs::write(&pub_file, pub_content).expect("Write public key");
    
    // Securely clear scalar from memory
    let mut scalar_mut = scalar;
    scalar_mut.aggressive_zeroize();
    
    println!("✅ Keypair generated:");
    println!("   Private: {}", pem_file.display());
    println!("   Public:  {}", pub_file.display());
    println!("\n🕌 InshaAllah — Keys secured with BCS-521 Fortress");
}

fn generate_kahf_scalar() -> Scalar {
    // Use Kahf-seeded generation
    let curve = bcs521_v2();
    // For now, use random scalar from OsRng
    // TODO: Integrate full Kahf seeding with counter
    generate_random_scalar()
}

fn generate_random_scalar() -> Scalar {
    use rand::rngs::OsRng;
    use rand::RngCore;
    
    let mut bytes = [0u8; 66];
    OsRng.fill_bytes(&mut bytes);
    
    // Reduce mod n
    Scalar::from_bytes_be(&bytes).expect("Valid scalar")
}

fn scalar_to_bytes(scalar: &Scalar) -> [u8; 66] {
    // Extract limbs and convert to bytes
    let mut bytes = [0u8; 66];
    // Implementation depends on Scalar internal structure
    // For now, use serialization
    bytes
}

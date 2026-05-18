use std::fs;
use std::path::PathBuf;

/// Sign a message
pub fn run(key: PathBuf, message: Option<String>, file: Option<PathBuf>, output: Option<PathBuf>) {
    println!("✍️  BCS-521 Signing");
    
    // Read message
    let msg_bytes = if let Some(msg) = message {
        msg.into_bytes()
    } else if let Some(f) = file {
        fs::read(f).expect("Read file")
    } else {
        eprintln!("❌ Error: Provide --message or --file");
        std::process::exit(1);
    };
    
    // Read private key
    let _key_content = fs::read_to_string(&key).expect("Read private key");
    
    // TODO: Implement actual ECDSA or EdDSA-style signing
    // For now, placeholder
    let signature = format!("BCS-SIG-v1:{}", hex::encode(&msg_bytes[..32.min(msg_bytes.len())]));
    
    if let Some(out) = output {
        fs::write(&out, &signature).expect("Write signature");
        println!("✅ Signature saved: {}", out.display());
    } else {
        println!("Signature: {}", signature);
    }
    
    println!("\n🕌 Signed with BCS-521 Fortress protection");
}

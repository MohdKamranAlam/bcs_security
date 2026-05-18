//! # BCS-521 Islamic Fintech CLI
//!
//! A command-line tool for BCS-521 Fortress Edition cryptography,
//! designed for Islamic fintech applications with maximum security.
//!
//! ## Features
//!
//! - **Kahf Seeding**: Mathematical connection to Surah Al-Kahf (Quran 18)
//! - **Fortress Hardening**: DPA masking, fault injection resistance, aggressive zeroize
//! - **Post-Quantum Hybrid**: ML-KEM-1024 + BCS-521 by default
//! - **Halal Compliance**: No riba-based algorithms, transparent audit trail
//!
//! ## Usage
//!
//! ```bash
//! # Generate a keypair with Kahf seeding
//! bcs keygen --kahf --output mykey
//!
//! # Sign a message
//! bcs sign --key mykey.pem --message "Hello World"
//!
//! # Verify a signature
//! bcs verify --key mykey.pub --message "Hello World" --signature sig.hex
//!
//! # ECDH key agreement
//! bcs ecdh --private mykey.pem --public peer.pub --output shared.raw
//!
//! # Hybrid KEM (post-quantum safe)
//! bcs hybrid-kem --encaps --public peer.pub --output ct.raw
//! bcs hybrid-kem --decaps --private mykey.pem --ciphertext ct.raw
//! ```

use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

mod commands;
mod kahf_crypto;

use commands::*;

/// BCS-521 Islamic Fintech CLI — Fortress Edition
#[derive(Parser)]
#[command(
    name = "bcs",
    version = "0.1.0-fortress",
    about = "BCS-521 Islamic Fintech CLI with Fortress security",
    long_about = "A command-line tool for BCS-521 Fortress Edition cryptography.\n\
                  Designed for Islamic fintech with post-quantum security."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new keypair
    Keygen {
        /// Output file prefix (e.g., mykey creates mykey.pem and mykey.pub)
        #[arg(short, long)]
        output: PathBuf,
        /// Use Kahf seeding (Surah Al-Kahf mathematical derivation)
        #[arg(long)]
        kahf: bool,
        /// Use Fortress mode (DPA masking + fault protection)
        #[arg(long)]
        fortress: bool,
    },

    /// Sign a message
    Sign {
        /// Private key file
        #[arg(short, long)]
        key: PathBuf,
        /// Message to sign (or use --file)
        #[arg(short, long)]
        message: Option<String>,
        /// File to sign
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Output file for signature
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Verify a signature
    Verify {
        /// Public key file
        #[arg(short, long)]
        key: PathBuf,
        /// Original message
        #[arg(short, long)]
        message: Option<String>,
        /// Original file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Signature file or hex string
        #[arg(short, long)]
        signature: String,
    },

    /// ECDH key agreement
    Ecdh {
        /// Your private key
        #[arg(short, long)]
        private: PathBuf,
        /// Peer public key
        #[arg(short, long)]
        public: PathBuf,
        /// Output file for shared secret
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Hybrid KEM (Post-Quantum + Classical)
    HybridKem {
        /// Encapsulate mode
        #[arg(long, group = "mode")]
        encaps: bool,
        /// Decapsulate mode
        #[arg(long, group = "mode")]
        decaps: bool,
        /// Public key file (for encaps)
        #[arg(short, long)]
        public: Option<PathBuf>,
        /// Private key file (for decaps)
        #[arg(short, long)]
        private: Option<PathBuf>,
        /// Ciphertext file (for decaps)
        #[arg(short, long)]
        ciphertext: Option<PathBuf>,
        /// Output file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Show security information
    SecurityInfo {
        /// Show detailed Fortress features
        #[arg(long)]
        fortress: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Keygen { output, kahf, fortress } => {
            keygen::run(output, kahf, fortress);
        }
        Commands::Sign { key, message, file, output } => {
            sign::run(key, message, file, output);
        }
        Commands::Verify { key, message, file, signature } => {
            verify::run(key, message, file, signature);
        }
        Commands::Ecdh { private, public, output } => {
            ecdh::run(private, public, output);
        }
        Commands::HybridKem { encaps, decaps, public, private, ciphertext, output } => {
            if encaps {
                hybrid_kem::encaps(public.unwrap(), output);
            } else if decaps {
                hybrid_kem::decaps(private.unwrap(), ciphertext.unwrap(), output);
            }
        }
        Commands::SecurityInfo { fortress } => {
            security_info::run(fortress);
        }
    }
}

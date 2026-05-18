//! End-to-end ECDH agreement test for `bcs keygen` + `bcs ecdh`.
//!
//! Test plan:
//!   1. Alice keygen  → alice.bcs521-sk / alice.bcs521-pub
//!   2. Bob   keygen  → bob.bcs521-sk   / bob.bcs521-pub
//!   3. Alice ECDH (alice-sk, bob-pub)  → alice_shared.raw
//!   4. Bob   ECDH (bob-sk,   alice-pub)→ bob_shared.raw
//!   5. Assert alice_shared == bob_shared (byte-equal, 32 bytes)
//!
//! Run with: `cargo test --features ecdsa -- --test-threads=1`
//! (single-thread avoids temp-dir name collisions on parallel runs)

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::{env, fs};

/// Return the path to the `bcs` binary built in the current profile.
fn bcs_bin() -> PathBuf {
    // In integration tests the binary is placed at:
    //   <workspace>/target/<profile>/bcs[.exe]
    let mut p = env::current_exe()
        .expect("cannot resolve test binary path")
        .parent()
        .expect("test binary has no parent")
        .to_path_buf();

    // The test binary sits in  target/<profile>/deps/;
    // the CLI binary is one level up in  target/<profile>/.
    if p.ends_with("deps") {
        p = p.parent().unwrap().to_path_buf();
    }

    p.join(if cfg!(windows) { "bcs.exe" } else { "bcs" })
}

fn run(bin: &PathBuf, args: &[&str]) {
    let status = Command::new(bin)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .unwrap_or_else(|e| panic!("failed to run bcs {:?}: {}", args, e));
    assert!(
        status.success(),
        "bcs {} exited with {:?}",
        args.join(" "),
        status
    );
}

#[test]
fn ecdh_alice_bob_shared_secret_byte_equal() {
    let bin = bcs_bin();
    assert!(
        bin.exists(),
        "bcs binary not found at {:?}. Run `cargo build --features ecdsa` first.",
        bin
    );

    // Temporary directory for key files and shared secrets.
    let tmp = env::temp_dir().join(format!(
        "bcs_e2e_ecdh_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    fs::create_dir_all(&tmp).expect("create tmp dir");

    let alice      = tmp.join("alice");
    let bob        = tmp.join("bob");
    let alice_sk   = tmp.join("alice.bcs521-sk");
    let alice_pub  = tmp.join("alice.bcs521-pub");
    let bob_sk     = tmp.join("bob.bcs521-sk");
    let bob_pub    = tmp.join("bob.bcs521-pub");
    let alice_sh   = tmp.join("alice_shared.raw");
    let bob_sh     = tmp.join("bob_shared.raw");

    // 1. Alice keygen
    run(&bin, &["keygen", "--output", alice.to_str().unwrap()]);
    assert!(alice_sk.exists(),  "alice.bcs521-sk missing");
    assert!(alice_pub.exists(), "alice.bcs521-pub missing");

    // 2. Bob keygen
    run(&bin, &["keygen", "--output", bob.to_str().unwrap()]);
    assert!(bob_sk.exists(),  "bob.bcs521-sk missing");
    assert!(bob_pub.exists(), "bob.bcs521-pub missing");

    // 3. Alice ECDH: alice-sk × bob-pub → alice_shared.raw
    run(&bin, &[
        "ecdh",
        "--private", alice_sk.to_str().unwrap(),
        "--public",  bob_pub.to_str().unwrap(),
        "--output",  alice_sh.to_str().unwrap(),
    ]);
    assert!(alice_sh.exists(), "alice_shared.raw missing");

    // 4. Bob ECDH: bob-sk × alice-pub → bob_shared.raw
    run(&bin, &[
        "ecdh",
        "--private", bob_sk.to_str().unwrap(),
        "--public",  alice_pub.to_str().unwrap(),
        "--output",  bob_sh.to_str().unwrap(),
    ]);
    assert!(bob_sh.exists(), "bob_shared.raw missing");

    // 5. Compare.
    let alice_bytes = fs::read(&alice_sh).expect("read alice_shared.raw");
    let bob_bytes   = fs::read(&bob_sh).expect("read bob_shared.raw");

    assert_eq!(
        alice_bytes.len(),
        32,
        "shared secret must be exactly 32 bytes (HKDF-SHA-256 output)"
    );
    assert_eq!(
        bob_bytes.len(),
        32,
        "shared secret must be exactly 32 bytes (HKDF-SHA-256 output)"
    );
    assert_eq!(
        alice_bytes, bob_bytes,
        "Alice and Bob derived different shared secrets — ECDH agreement FAILED"
    );

    // Clean up.
    let _ = fs::remove_dir_all(&tmp);

    println!("✓ ECDH agreement: alice and bob shared secrets are byte-equal (32 bytes)");
}

// ---------------------------------------------------------------------------
// E2E sign + verify round-trip
// ---------------------------------------------------------------------------

#[test]
fn ecdsa_sign_verify_roundtrip() {
    let bin = bcs_bin();
    assert!(
        bin.exists(),
        "bcs binary not found at {:?}. Run `cargo build --features ecdsa` first.",
        bin
    );

    let tmp = env::temp_dir().join(format!(
        "bcs_e2e_ecdsa_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    fs::create_dir_all(&tmp).expect("create tmp dir");

    let alice     = tmp.join("alice");
    let alice_sk  = tmp.join("alice.bcs521-sk");
    let alice_pub = tmp.join("alice.bcs521-pub");
    let sig_file  = tmp.join("msg.sig");

    // Keygen
    run(&bin, &["keygen", "--output", alice.to_str().unwrap()]);

    // Sign "Hello BCS-521"
    let message = "Hello BCS-521";
    run(&bin, &[
        "sign",
        "--key",     alice_sk.to_str().unwrap(),
        "--message", message,
        "--output",  sig_file.to_str().unwrap(),
    ]);
    assert!(sig_file.exists(), "sig file missing after sign");

    // Verify
    let status = Command::new(&bin)
        .args(&[
            "verify",
            "--key",       alice_pub.to_str().unwrap(),
            "--message",   message,
            "--signature", sig_file.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .expect("run bcs verify");
    assert!(status.success(), "bcs verify exited non-zero for a valid signature");

    // Tampered message must fail (exit 1)
    let bad = Command::new(&bin)
        .args(&[
            "verify",
            "--key",       alice_pub.to_str().unwrap(),
            "--message",   "tampered message",
            "--signature", sig_file.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run bcs verify (tampered)");
    assert!(
        !bad.success(),
        "verify of tampered message must fail, but it succeeded"
    );

    let _ = fs::remove_dir_all(&tmp);
    println!("✓ ECDSA sign+verify round-trip passed; tampered message correctly rejected");
}

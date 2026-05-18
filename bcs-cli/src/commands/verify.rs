//! `bcs verify` — NOT YET IMPLEMENTED (mirror of `bcs sign`).
//!
//! Returning `valid = true` for *any* input — as the previous
//! placeholder did — would be a security catastrophe. We therefore
//! explicitly fail rather than ever return a verification verdict for
//! a signature scheme that does not yet exist.

use std::path::PathBuf;

pub fn run(_key: PathBuf, _message: Option<String>, _file: Option<PathBuf>, _signature: String) {
    eprintln!(
        "error: `bcs verify` is not implemented in v0.2.x. \n\
         BCS-521 ECDSA verification is tracked for v0.3.0."
    );
    std::process::exit(2);
}

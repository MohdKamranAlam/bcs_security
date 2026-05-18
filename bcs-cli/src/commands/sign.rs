//! `bcs sign` — NOT YET IMPLEMENTED.
//!
//! BCS-521 ECDSA requires constant-time scalar arithmetic mod n_521
//! (Barrett reduction + Fermat inversion) that the v0.2.x core library
//! deliberately does not yet ship. Implementing this without the
//! supporting primitives would mean either (a) using a non-constant-time
//! path or (b) returning placeholder bytes — we will do neither.
//!
//! Tracked for v0.3.0. For authenticated message exchange today, use
//! `bcs hybrid-kem --encaps/--decaps` to derive a shared secret and
//! authenticate with HMAC-SHA-256.

use std::path::PathBuf;

pub fn run(
    _key: PathBuf,
    _message: Option<String>,
    _file: Option<PathBuf>,
    _output: Option<PathBuf>,
) {
    eprintln!(
        "error: `bcs sign` is not implemented in v0.2.x. \n\
         BCS-521 ECDSA is tracked for v0.3.0. \n\
         For authenticated message exchange today, derive a shared secret \
         via `bcs hybrid-kem` and authenticate with HMAC-SHA-256."
    );
    std::process::exit(2);
}

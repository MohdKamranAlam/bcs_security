//! # Constant-time core for BCS-521  (`--features ct`)
//!
//! This module is the constant-time replacement for the BigUint-based
//! reference implementation in the top-level `lib.rs`.  All public
//! operations on secrets execute in time independent of the secret
//! value.
//!
//! See `BCS_CT_DESIGN.md` for the full specification.
//!
//! ## Module layout
//!
//! ```text
//! ct/
//! ├── mod.rs        ← this file (re-exports)
//! ├── consts.rs     ← p_521, n_521, R, R^2, G, addition-chain table
//! ├── fp521.rs      ← 9-limb field with Montgomery multiplication
//! ├── scalar.rs     ← Z / n_521 Z with Zeroize
//! ├── point.rs      ← Jacobian-projective point + complete formulas
//! └── ladder.rs     ← Montgomery ladder scalar multiplication
//! ```
//!
//! ## Stability
//!
//! This module is **experimental** in `v0.2.x`.  Public API is subject
//! to change until the side-channel tests in `dudect/` consistently
//! pass and the parity tests in `tests/test_ct_parity.rs` confirm
//! byte-equal output with the reference implementation for all 10
//! frozen test vectors.
//!
//! ## Safety
//!
//! `#[forbid(unsafe_code)]` is enforced for the entire `ct` subtree.

#![forbid(unsafe_code)]

pub mod consts;
pub mod fp521;
pub mod scalar;
// pub mod point;     // ← TODO: enable when point.rs lands
// pub mod ladder;    // ← TODO: enable when ladder.rs lands

pub use consts::*;
pub use fp521::Fp521;
pub use scalar::Scalar;

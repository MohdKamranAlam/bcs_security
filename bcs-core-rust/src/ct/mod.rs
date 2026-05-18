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
//! ├── mod.rs               ← this file (re-exports)
//! ├── consts.rs            ← p_521, n_521, R, R^2, G, addition-chain table
//! ├── fp521.rs             ← 9-limb field with Montgomery multiplication
//! ├── scalar.rs            ← Z / n_521 Z with Zeroize
//! ├── point.rs             ← Jacobian-projective point + complete formulas
//! ├── ladder.rs            ← Montgomery ladder scalar multiplication
//! ├── fault_injection.rs   ← Redundant computation + CT compare (Fortress)
//! ├── masking.rs           ← DPA additive scalar masking (Fortress)
//! ├── execution_proof.rs   ← Transparent proof system (Fortress)
//! └── aggressive_zeroize.rs ← Multi-pass clear + fence (Fortress)
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
pub mod point;
pub mod ladder;
pub mod fault_injection;
pub mod masking;
pub mod execution_proof;
pub mod aggressive_zeroize;

pub use consts::*;
pub use fp521::Fp521;
pub use scalar::Scalar;
pub use point::ProjPoint;
pub use ladder::{scalar_mul, scalar_mul_generator};
pub use fault_injection::{scalar_mul_fault_protected, scalar_mul_generator_fault_protected};
pub use masking::{scalar_mul_masked, scalar_mul_generator_masked, MaskedScalar};
pub use execution_proof::{ExecutionProof, FortressFlags, OperationKind};
pub use aggressive_zeroize::{aggressive_clear, aggressive_clear_u64, AggressiveZeroize};

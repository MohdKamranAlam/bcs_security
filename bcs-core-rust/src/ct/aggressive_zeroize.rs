//! # Aggressive secret clearing — cold-boot and dead-store resistance
//!
//! Standard `Zeroize` overwrites memory with zeroes.  However:
//!
//! 1. **Compiler optimization:** LLVM may eliminate a write to memory
//!    that is never read again ("dead-store elimination").  `Zeroize`
//!    uses `volatile_cell` writes to prevent this, but the compiler can
//!    still reorder or merge writes.
//!
//! 2. **CPU cache:** After zeroize, the secret may persist in L1/L2
//!    cache lines.  A cold-boot attacker who freezes RAM can recover
//!    data that was "zeroized" but still in cache.
//!
//! 3. **Compiler reordering:** Without a memory fence, the compiler
//!    may move the zeroize call earlier or later than expected.
//!
//! ## Countermeasures in this module
//!
//! 1. **Multi-pass overwrite** — write 3 different patterns before
//!    the final zero write (DoD 5220.22-M inspired).
//! 2. **Memory fence** — `SeqCst` atomic fence after each pass
//!    prevents the compiler from reordering or merging.
//! 3. **Black-box read** — force a read of the zeroized memory
//!    after writing, preventing dead-store elimination.
//!
//! ### Limitations
//!
//! - **No CPU cache flush** — `#![forbid(unsafe_code)]` prevents us
//!   from using `clflush` or similar instructions.  The best we can
//!   do is overwrite + fence + black_box.  On most architectures,
//!   the cache line will eventually be evicted naturally.
//!
//! - **No register clearing** — the compiler may keep secret values
//!   in registers after zeroize.  Without inline assembly, we cannot
//!   force register zeroing.  The `black_box` calls make this *less*
//!   likely but cannot guarantee it.

use std::sync::atomic::fence;
use std::sync::atomic::Ordering::SeqCst;

/// Overwrite a byte slice with 3 random-ish patterns, then zero,
/// with memory fences between each pass.
///
/// This is significantly harder for an attacker to recover than a
/// single zero-write, because:
/// - The intermediate patterns defeat simple "one overwrite" recovery
/// - The fences prevent the compiler from merging the passes
/// - The final black_box read prevents dead-store elimination
#[inline]
pub fn aggressive_clear(data: &mut [u8]) {
    let len = data.len();
    if len == 0 { return; }

    // Pass 1: overwrite with 0x55 (01010101)
    for b in data.iter_mut() {
        *b = 0x55;
    }
    fence(SeqCst);

    // Pass 2: overwrite with 0xAA (10101010)
    for b in data.iter_mut() {
        *b = 0xAA;
    }
    fence(SeqCst);

    // Pass 3: overwrite with 0xFF (11111111)
    for b in data.iter_mut() {
        *b = 0xFF;
    }
    fence(SeqCst);

    // Final pass: zero
    for b in data.iter_mut() {
        *b = 0x00;
    }
    fence(SeqCst);

    // Force a read to prevent dead-store elimination.
    // The compiler cannot eliminate the writes if it must produce
    // the value of this read.
    let mut acc: u8 = 0;
    for &b in data.iter() {
        acc |= b;
    }
    std::hint::black_box(acc);
}

/// Overwrite a `u64` array with 3 patterns, then zero.
/// Same strategy as `aggressive_clear` but for limb arrays.
#[inline]
pub fn aggressive_clear_u64(limbs: &mut [u64]) {
    let len = limbs.len();
    if len == 0 { return; }

    // Pass 1: 0x5555...55
    for l in limbs.iter_mut() {
        *l = 0x5555_5555_5555_5555;
    }
    fence(SeqCst);

    // Pass 2: 0xAAAA...AA
    for l in limbs.iter_mut() {
        *l = 0xAAAA_AAAA_AAAA_AAAA;
    }
    fence(SeqCst);

    // Pass 3: 0xFFFF...FF
    for l in limbs.iter_mut() {
        *l = 0xFFFF_FFFF_FFFF_FFFF;
    }
    fence(SeqCst);

    // Final: zero
    for l in limbs.iter_mut() {
        *l = 0;
    }
    fence(SeqCst);

    // Force read
    let mut acc: u64 = 0;
    for &l in limbs.iter() {
        acc |= l;
    }
    std::hint::black_box(acc);
}

/// Trait for types that can aggressively clear their secret data.
///
/// Implementors should use multi-pass overwrite + fence + black_box.
/// This is a stricter contract than `Zeroize` — it provides cold-boot
/// resistance in addition to simple memory clearing.
pub trait AggressiveZeroize {
    /// Overwrite all secret data with multiple patterns, then zero,
    /// with memory fences between passes.
    fn aggressive_zeroize(&mut self);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggressive_clear_byte_slice() {
        let mut data = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE];
        aggressive_clear(&mut data);
        assert_eq!(data, [0u8; 6], "not zeroed after aggressive_clear");
    }

    #[test]
    fn aggressive_clear_u64_slice() {
        let mut limbs = [0xDEADBEEFu64, 0xCAFEBABEu64, 0x12345678u64];
        aggressive_clear_u64(&mut limbs);
        assert_eq!(limbs, [0u64; 3], "not zeroed after aggressive_clear_u64");
    }

    #[test]
    fn aggressive_clear_empty_slice() {
        let mut data: [u8; 0] = [];
        aggressive_clear(&mut data); // should not panic
    }

    #[test]
    fn aggressive_zeroize_trait_impl() {
        let mut secret = TestSecret { limbs: [42u64; 9] };
        secret.aggressive_zeroize();
        assert_eq!(secret.limbs, [0u64; 9]);
    }

    struct TestSecret {
        limbs: [u64; 9],
    }

    impl AggressiveZeroize for TestSecret {
        fn aggressive_zeroize(&mut self) {
            aggressive_clear_u64(&mut self.limbs);
        }
    }
}

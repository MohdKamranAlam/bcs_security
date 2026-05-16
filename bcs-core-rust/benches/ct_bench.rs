//! Criterion benchmarks for the constant-time core.
//!
//! Run with:
//!   cargo bench --features ct
//!
//! ## Current coverage
//!
//! - `fp521_add`   : Fp521::add_mod_p
//! - `fp521_sub`   : Fp521::sub_mod_p
//! - `fp521_swap`  : Fp521::conditional_swap (Choice = 0 and Choice = 1)
//!
//! Mul / square / inversion benchmarks land with the Montgomery
//! milestone in `src/ct/fp521.rs`.

#![cfg(feature = "ct")]

use bcs_core_rust::ct::Fp521;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use subtle::Choice;

fn bench_fp521(c: &mut Criterion) {
    // Use a non-trivial element so the optimizer cannot fold it.
    let mut bytes = [0u8; 66];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(31).wrapping_add(7);
    }
    // Top byte must keep us below p; clear high bits.
    bytes[0] = 0;
    let a = Fp521::from_bytes_be(&bytes).expect("test vector below p");
    let mut bytes2 = bytes;
    bytes2[65] ^= 0xA5;
    let b = Fp521::from_bytes_be(&bytes2).expect("test vector below p");

    c.bench_function("fp521_add", |bench| {
        bench.iter(|| black_box(black_box(a).add_mod_p(black_box(&b))))
    });
    c.bench_function("fp521_sub", |bench| {
        bench.iter(|| black_box(black_box(a).sub_mod_p(black_box(&b))))
    });
    c.bench_function("fp521_swap_choice_0", |bench| {
        bench.iter(|| {
            let mut x = a;
            let mut y = b;
            Fp521::conditional_swap(&mut x, &mut y, Choice::from(0));
            black_box((x, y))
        })
    });
    c.bench_function("fp521_swap_choice_1", |bench| {
        bench.iter(|| {
            let mut x = a;
            let mut y = b;
            Fp521::conditional_swap(&mut x, &mut y, Choice::from(1));
            black_box((x, y))
        })
    });
}

criterion_group!(ct, bench_fp521);
criterion_main!(ct);

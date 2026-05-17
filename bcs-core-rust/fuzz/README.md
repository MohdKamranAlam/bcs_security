# BCS-521 fuzzing harness

Coverage-guided fuzzing of the public API surface using
[`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html) +
libFuzzer.

## Prerequisites

```bash
rustup toolchain install nightly
cargo install cargo-fuzz
```

## Running a target

```bash
cd bcs-core-rust
cargo +nightly fuzz run fuzz_parse_public_key
cargo +nightly fuzz run fuzz_parse_secret_key
cargo +nightly fuzz run fuzz_ecdh_round_trip
```

The harness will run forever; press `Ctrl-C` after a chosen wall-clock
budget (e.g. one hour) and inspect the corpus under
`fuzz/corpus/<target>/`.

## Targets

| Target | Surface | Contracts |
|---|---|---|
| `fuzz_parse_public_key` | `Bcs521PublicKey::from_bytes` | no panic; `Ok` round-trips |
| `fuzz_parse_secret_key` | `Bcs521SecretKey::from_bytes` | no panic; `Ok` round-trips; derived `pk` round-trips |
| `fuzz_ecdh_round_trip` | `Bcs521::ecdh` | no panic on valid inputs; commutativity |

## What this does NOT cover

* Timing side-channels — see `dudect/` (planned).
* Curve formula correctness — see `tests/test_ct_parity.rs`.
* Memory safety — Rust handles this; we still fuzz to catch logic bugs.

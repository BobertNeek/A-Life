#!/bin/bash
set -euo pipefail
BASE=/workspace/scratch/ee5e08a3ca9b
ROOT=$BASE/recovered-toolchain-rust-1.99.1-20261107
export CARGO_HOME=$ROOT/cargo RUSTUP_HOME=$ROOT/rustup
export PATH=$ROOT/rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin:$ROOT/cargo/bin:$PATH
export CARGO_TARGET_DIR=$BASE/thermal-contact-target-v2 CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 RUST_MIN_STACK=33554432
cd "$BASE/thermal-contact-candidate-v2"
cargo fmt --all --check
cargo test --offline -p alife_world --lib thermal_contact_candidate -- --nocapture --test-threads=1
cargo test --offline -p alife_world --lib -- --test-threads=1
cargo test --offline -p alife_world --test task_3_2a_atomic_action --test save_load_roundtrip --test atomic_portable_save --test v11_cognitive_biology -- --test-threads=1
cargo test --offline -p alife_core --test biochemistry_development --test thermal_reproductive_drive_audit -- --test-threads=1
cargo clippy --offline -p alife_core -p alife_world --all-targets -- -D warnings
git diff --check

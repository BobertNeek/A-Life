# Reproduction commands

Compiler: official Rust 1.99.1. Set CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,
CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0. Use an offline populated
Cargo cache and a target directory reserved for the checked source.

Production/body test source: 0a4ed58317a48a090a4511b09ca8a1883239aeba.
Final public audit example: 3007e0ab2d8a0a47f5e4f0f57478fb21562d8909.
Later commits contain documentation/evidence only.

```sh
cargo test --offline -p alife_core -p alife_world --lib -- --test-threads=1 --nocapture
cargo clippy --offline -p alife_core -p alife_world --all-targets -- -D warnings
cargo test --offline -p alife_core \
  --test biochemical_graph_v2 --test biochemical_material_baselines \
  --test biochemical_reaction_conservation --test biochemical_valuation \
  --test biochemistry_development --test evolutionary_genetics \
  --test measured_physiology_v2 --test newborn_nociception \
  --test chemistry_homeostasis --test thermal_reproductive_drive_audit \
  -- --test-threads=1
cargo test --offline -p alife_world \
  --test canonical_new_game --test awake_rest_legacy_wire \
  --test developmental_rest_recovery --test save_load_roundtrip \
  --test atomic_portable_save --test world_organism_registry_persistence \
  -- --test-threads=1
cargo fmt --all -- --check
bash scripts/check.sh --quick
cargo run --offline -p alife_world --example recovery_timescale_legacy_wire -- OUTPUT_DIRECTORY
```

For the cross-source comparison, run the exact same audit example in a separate
worktree of accepted main 68beb760f61fddd9f838221f75b006748845d821 and its own target.
Use separate fresh output directories. Compare each of the six JSON files as
bytes, then compare the complete stdout digest records. Only generated ordinary
and awake-rest control saves are inputs; no retained private life or checkpoint.
Do not replace, move or clean any existing user save or target.

The first run with only the initial six world fixtures is retained in
first-fixed-tests.log. The separate 1100-spaced schedule was declared before its
implementation. strict-clippy.log retains the first failure, an audit example
attempting to call a private API; strict-clippy-final.log records the corrected
public-API run. No biological parameter was fitted to that failure or any result.

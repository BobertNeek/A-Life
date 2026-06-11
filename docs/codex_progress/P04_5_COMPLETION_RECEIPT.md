# Completion receipt

Plan: P04.5 - Performance contract, GPU/CPU boundary, and population budget amendment

Branch: codex/P04.5-performance-contract

Files changed:

- Added `docs/architecture/P04_5_performance_contract.md`.
- Added `crates/alife_tools/tests/performance_contract.rs`.
- Updated `docs/codex_progress/PLAN_PROGRESS.md`.
- Updated `docs/codex_progress/SPEC_TRACEABILITY.md`.
- Updated `docs/codex_progress/DECISION_LOG.md`.
- Added `docs/codex_progress/P04_5_COMPLETION_RECEIPT.md`.

Public APIs changed:

- None.

Tests added/changed:

- Added `crates/alife_tools/tests/performance_contract.rs`.
- The test parses the machine-readable ledger block in `docs/architecture/P04_5_performance_contract.md` and recomputes dense totals, sparse per-creature live totals, shared template totals, and population totals from explicit fields and formulas.

Commands run:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `& 'C:\Program Files\Git\bin\bash.exe' scripts/check.sh`
- `& 'C:\Program Files\Git\bin\bash.exe' scripts/check_core_boundaries.sh`

Results:

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace --all-targets`: passed.
- `cargo test --workspace --all-targets`: passed, including the new performance ledger validation test.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- Git Bash `scripts/check.sh`: passed.
- Git Bash `scripts/check_core_boundaries.sh`: passed.

Invariant checks:

- P04.5 did not add neural kernels, GPU shaders, Bevy/Avian adapters, world simulation, playground code, SLM code, D2NWG, ETF tooling, or runtime neural loops.
- `GLOBAL_INVARIANTS.md` was not changed because existing invariants already cover the P04.5 transfer, sparse-buffer, parity, and structured-action rules.

Deviations:

- `scripts/check.sh` and `scripts/check_core_boundaries.sh` were run through Git Bash because this Windows PowerShell environment does not directly execute `.sh` files and default `bash` resolves to unavailable WSL.

Known limitations:

- The ledger uses v1 conservative defaults. P05/P06/P14/P20/P24 may amend formulas only by updating the contract and validation.

Next plan(s): P05, P06, P07, P08, P09

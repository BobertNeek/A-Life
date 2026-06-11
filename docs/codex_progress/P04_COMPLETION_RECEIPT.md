# Completion receipt

Plan: P04 - ABI versioning, validation framework, error model

Branch: codex/P04-abi-validation-errors

Files changed:

- Added `crates/alife_core/src/version.rs`.
- Added `crates/alife_core/src/validation.rs`.
- Added `crates/alife_core/src/diagnostics.rs`.
- Updated `crates/alife_core/src/error.rs`.
- Updated `crates/alife_core/src/action_abi.rs`.
- Updated `crates/alife_core/src/sensory_abi.rs`.
- Updated `crates/alife_core/src/action.rs`.
- Updated `crates/alife_core/src/experience.rs`.
- Updated `crates/alife_core/src/traits.rs`.
- Updated `crates/alife_core/src/genome.rs`.
- Updated `crates/alife_core/src/lineage.rs`.
- Updated `crates/alife_core/src/lib.rs`.
- Added `crates/alife_core/tests/abi_validation_errors.rs`.
- Updated `docs/architecture/schema_versioning.md`.
- Updated `docs/codex_progress/PLAN_PROGRESS.md`.
- Updated `docs/codex_progress/DECISION_LOG.md`.
- Updated `docs/codex_progress/SPEC_TRACEABILITY.md`.
- Added `docs/codex_progress/P04_COMPLETION_RECEIPT.md`.

Public APIs changed:

- Added `SchemaKind`, `ContractVersion`, `SchemaVersions`, `require_current_version`, and `require_version`.
- Added `Validate`, `Validated<T>`, and `ensure_current_version`.
- Added `ContractDiagnostic` and `DiagnosticCode`.
- Expanded `ScaffoldContractError` with missing phase data, out-of-range drive/hormone, incompatible ABI, packed-log schema mismatch, and backend parity variants.
- Existing placeholder ABI constants now point at `SchemaVersions::CURRENT`.

Tests added/changed:

- Added `crates/alife_core/tests/abi_validation_errors.rs` covering central version sharing, incompatible ABI rejection, typed diagnostics, `Validated<T>`, and validation of existing headers/manifests.

Commands run:

- `cargo fmt --all -- --check`
- `cargo test -p alife_core --tests`
- `cargo check --workspace --all-targets`
- `cargo test --workspace --all-targets`
- `& 'C:\Program Files\Git\bin\bash.exe' scripts/check_core_boundaries.sh`
- `& 'C:\Program Files\Git\bin\bash.exe' scripts/check.sh`
- `cargo clippy --workspace --all-targets -- -D warnings`

Results:

- Formatting passed after applying rustfmt.
- Core tests passed: 15 tests.
- Workspace tests passed: 18 tests total.
- Workspace check passed.
- Core boundary check passed.
- Aggregate local gate passed under Git Bash.
- Clippy with `-D warnings` passed.

Invariant checks:

- `alife_core` remains engine-independent.
- Errors are typed and testable.
- Version rejection returns structured errors and compact diagnostics.
- No runtime neural kernels, Bevy runtime behavior, GPU shader work, SLM work, D2NWG work, or playground work was added.

Deviations:

- None.

Known limitations:

- Packed-log, neural projection, save, and teacher/school schemas are versioned in the registry but their executable readers/writers are deferred to their owning plans.
- Backend parity errors are represented as a top-level variant; detailed parity payloads are deferred to GPU parity plans.

Next plan(s): P05, P06, P07, P08, P09

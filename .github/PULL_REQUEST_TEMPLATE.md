## Plan

- Plan ID:
- Branch:
- Next plan(s):

## Changes

- Files changed:
- Public APIs changed:
- Tests added/changed:

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --workspace --all-targets`
- [ ] `cargo test --workspace --all-targets`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] Windows: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`; non-Windows/Git Bash: run `scripts/check_core_boundaries.sh`
- [ ] Windows: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`; non-Windows/Git Bash: run `scripts/docs_check.sh`

## Invariants

- [ ] `alife_core` remains engine-independent.
- [ ] No Unity/C#/HLSL production files.
- [ ] Runtime cognition and packed logging remain separate.
- [ ] Teacher/semantic systems cannot bypass perception or action arbitration.
- [ ] GPU work, if any, is gated by CPU parity.

## Deviations and Limitations

- Deviations:
- Known limitations:

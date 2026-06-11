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
- [ ] `bash scripts/check_core_boundaries.sh` or documented local substitute
- [ ] `bash scripts/docs_check.sh` or documented local substitute

## Invariants

- [ ] `alife_core` remains engine-independent.
- [ ] No Unity/C#/HLSL production files.
- [ ] Runtime cognition and packed logging remain separate.
- [ ] Teacher/semantic systems cannot bypass perception or action arbitration.
- [ ] GPU work, if any, is gated by CPU parity.

## Deviations and Limitations

- Deviations:
- Known limitations:

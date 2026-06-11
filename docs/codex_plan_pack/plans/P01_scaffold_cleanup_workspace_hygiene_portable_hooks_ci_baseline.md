# P01 - Scaffold cleanup, workspace hygiene, portable hooks, CI baseline

Group: Group 0 - Baseline serial

Branch: `codex/P01-scaffold-cleanup`

Prerequisites: P00

Concurrency: No. Run after P00 before feature branches.

Next plan(s): P02

## Purpose

Remove avoidable scaffold hazards before feature work begins. This keeps future branches from fighting duplicate files, non-portable hooks, broken workspace membership, or inconsistent validation scripts.

## Owned scope

- Workspace manifests, repo scripts, CI configs, docs cleanup, `.codex` hooks.

## Required implementation steps

1. Remove or archive duplicate nested scaffold/spec-pack files if P00 confirmed they are accidental duplicates. Preserve original specs under a clear `docs/specs/` location if they are intended to remain in-repo.
2. Replace any absolute-user-path Codex/Graphify hook with a portable script invocation. A hook may call `scripts/graphify.sh` or equivalent, but it must not point at a local Windows profile path.
3. Normalize workspace member paths and crate names. Ensure every crate either compiles or is explicitly gated as placeholder with tests/documentation explaining why.
4. Add or fix root scripts: `scripts/check.sh`, `scripts/test.sh`, `scripts/check_core_boundaries.sh`, and any existing graph/doc script. Scripts must fail loudly on real errors and skip optional tools cleanly.
5. Add CI or local validation config that runs format, check, tests, and core boundary checks. If GitHub Actions exists, update it. If not, create a conservative workflow.
6. Add minimal crate-level README/module docs if missing so future agents understand ownership boundaries.
7. Do not add new runtime contracts except what is necessary to make existing placeholder code compile.
8. Update traceability rows for dependency-boundary and validation infrastructure.

## Required tests and validation

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace --all-targets`
- `bash scripts/check_core_boundaries.sh` if shell scripts are supported

## Acceptance criteria

- No accidental duplicate scaffold tree remains in the active workspace.
- All hooks are portable or disabled with explanation.
- CI/local scripts express the same validation gate Codex will use later.
- `alife_core` still has no Bevy/wgpu/Avian dependency.

## Failure handling

- If a placeholder crate cannot compile without large scope expansion, gate it behind a feature or fix the minimal manifest/module issue. Do not implement that crate here.
- If removing duplicate files would risk deleting source specs, move them to `docs/specs/archive/` instead.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P01 - Scaffold cleanup, workspace hygiene, portable hooks, CI baseline
Branch: codex/P01-scaffold-cleanup
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P02
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.

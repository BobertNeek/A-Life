# P00 - Operating model, repo audit, and plan wiring

Group: Group 0 - Baseline serial

Branch: `codex/P00-operating-model`

Prerequisites: None.

Concurrency: No. Run first and serially.

Next plan(s): P01

## Purpose

Make Codex stop guessing. This plan establishes the real repository state, places the plan pack somewhere stable, and creates the progress/decision artifacts that every later branch will update.

## Owned scope

- Repository root docs and scripts only unless fixing broken plan-pack paths.

## Required implementation steps

1. Create `docs/codex_plan_pack/` if the pack was not already placed there. Copy or preserve this entire pack under that path so every future branch can read it.
2. Audit the repository tree and record the actual crate names, module paths, current branch, current commit, and whether the previously observed crates exist.
3. Identify current scaffold state: which contracts are real, which are placeholders, which tests exist, which docs/specs are duplicated, and whether any generated nested spec pack should be removed in P01.
4. Create `docs/codex_progress/` if missing. Seed `PLAN_PROGRESS.md`, `DECISION_LOG.md`, and `SPEC_TRACEABILITY.md` from templates if equivalent files do not already exist.
5. Create a `docs/codex_progress/REPO_AUDIT_P00.md` report that lists current workspace members, dependencies per crate, important scripts, CI files, and obvious hazards.
6. Do not implement runtime features in this plan. The only code changes allowed are tiny script/path fixes needed to make validation commands runnable.
7. Record whether Cargo/Rust tooling is available and which validation commands can run in the current environment.
8. Update the progress log with P00 status and exact next plan P01.

## Required tests and validation

- Run `git status --short` before and after.
- Run `cargo metadata --no-deps` if Cargo is available; save a short summary in the audit.
- Run `cargo check --workspace --all-targets` only if the workspace is already expected to compile; otherwise record why it does not yet run.

## Acceptance criteria

- Future Codex agents can find the plan pack at a stable path.
- The repo audit exists and does not make stale assumptions about scaffold state.
- Progress, decision, and traceability logs exist.
- No runtime architecture was changed.

## Failure handling

- If the repository structure differs from this pack, do not rename everything. Record the real structure and adapt future plan file paths in the progress note.
- If Cargo is unavailable, do not claim validation passed. Record the missing tool and continue only with non-build checks.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P00 - Operating model, repo audit, and plan wiring
Branch: codex/P00-operating-model
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P01
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.

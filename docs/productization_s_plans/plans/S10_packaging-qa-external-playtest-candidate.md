# S10 - Packaging, QA, external playtest candidate

Branch: `codex/S10-packaging-qa-external-playtest`

Dependencies:
- S09

Recommended model/reasoning: GPT-5.5 High or Extra High

Next plan(s): S11

## Purpose

Prepare an external playtest candidate: packaging smoke, clean checkout instructions, known issues, test checklist, and artifact discipline.

## Owned scope

- package/run scripts
- external tester README
- known issues
- release candidate checklist
- S10 report

## Likely files/crates to inspect or touch

- scripts/**
- docs/productization/**
- README.md
- docs/playable_sim_spec/**
- crates/alife_game_app/**

## Forbidden scope

- release tag without approval
- store packaging
- signing automation unless explicitly approved
- committing dist artifacts

## Implementation milestones

1. Define clean-checkout runbook.
2. Validate package/dry-run scripts.
3. Confirm asset bundle discipline.
4. Create external playtest checklist.
5. Update known issues with honest limitations.
6. Run final product smoke commands.

## Required tests and evidence

- platform-package smoke
- product QA smoke
- release-candidate smoke
- no tracked generated artifacts
- docs path/command checks

## Acceptance criteria

- An external tester can follow documented instructions to run the supported build.
- No release tag is created.
- Known issues are explicit.

## Focused commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
```
```powershell
cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke
```
```powershell
cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke
```
```powershell
git ls-files target dist target/artifacts graphify-out
```

## Computer-use / manual evidence

- Create `S10_EXTERNAL_PLAYTEST_CANDIDATE_REPORT.md` and tester checklist.
- Do not commit builds/screenshots unless user explicitly approves.

## Failure handling

- If package runbook fails, stop before S11.
- If artifacts are tracked, remove before merge.

## Review checklist

- The plan implemented only `S10` scope.
- Runtime/code changes match the plan's owned scope.
- `alife_core` remains engine-independent.
- Headless CPU path remains green.
- Optional graphics/GPU/semantic/school systems remain optional unless explicitly hardened.
- No P37/G25/new automatic chain was created.
- Product claims match actual evidence.
- Reports under `docs/productization/` are honest about unavailable manual evidence.



## Global invariants

Read and obey:

- `docs/productization_s_plans/GLOBAL_INVARIANTS.md` if imported there, or the imported equivalent under the productization plan pack.
- Existing repo invariants in `AGENTS.md`.
- Existing P36/R24 validation discipline.

## Standard validation

Use Windows wrappers. Do not run plain `bash scripts/check.sh`.

Run the standard validation set from `VALIDATION_PROTOCOL.md`, plus each plan's focused commands.

## Completion receipt

```text
Completion receipt
Plan:
Branch:
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s):
Stopped:
```


## Required receipt override

```text
Completion receipt
Plan: S10 - Packaging, QA, external playtest candidate
Branch: codex/S10-packaging-qa-external-playtest
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s): S11
Stopped: yes
```

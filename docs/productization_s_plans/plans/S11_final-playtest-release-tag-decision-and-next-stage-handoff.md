# S11 - Final playtest, release/tag decision, and next-stage handoff

Branch: `codex/S11-final-playtest-release-decision`

Dependencies:
- S10

Recommended model/reasoning: GPT-5.5 Extra High preferred; High acceptable

Next plan(s): None - release decision / next-stage handoff

## Purpose

Run the final productization playtest review and decide whether to tag, defer, or start a new explicit phase. This is a decision gate, not automatic release.

## Owned scope

- final productization report
- release/tag proposal
- blocker list
- next-stage roadmap
- user decision packet

## Likely files/crates to inspect or touch

- docs/productization/**
- docs/playable_sim_spec/**
- README.md if factual update needed

## Forbidden scope

- creating release tag without explicit user approval
- starting a new implementation chain
- hiding limitations
- new runtime features

## Implementation milestones

1. Aggregate S01-S10 evidence.
2. Classify current game status: release candidate / alpha / prototype / not ready.
3. List blockers/high/medium/low issues.
4. Create release/tag proposal only.
5. Create next-stage roadmap after S11.
6. Stop for user decision.

## Required tests and evidence

- full validation
- final smoke suite
- stale command audit
- no G25/P37/new chain audit
- artifact tracking audit

## Acceptance criteria

- User receives a clear release/no-release decision packet.
- No release tag created unless explicitly approved.
- Future work is roadmap/backlog only.

## Focused commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke
cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke
cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke
cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun
```

## Computer-use / manual evidence

- Create `S11_FINAL_PRODUCTIZATION_REPORT.md`, `S11_RELEASE_DECISION_PACKET.md`, and `S11_NEXT_STAGE_ROADMAP.md`.
- Include screenshot/evidence references from S01-S10.

## Failure handling

- If blockers remain, recommend deferral and exact next fix phase.
- If all gates pass but manual evidence is missing, recommend alpha/prototype only unless user accepts limitations.

## Review checklist

- The plan implemented only `S11` scope.
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
Plan: S11 - Final playtest, release/tag decision, and next-stage handoff
Branch: codex/S11-final-playtest-release-decision
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s): None
Stopped: yes
```

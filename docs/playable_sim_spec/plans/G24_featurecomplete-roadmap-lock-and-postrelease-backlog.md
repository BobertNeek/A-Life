# G24 - Feature-complete roadmap lock and post-release backlog

Branch: `codex/G24-feature-complete-roadmap-lock`

Prerequisites: G23

Concurrency: Final serial lock.

Recommended model/reasoning: GPT-5.5 High or Pro High

Next plan(s): None - final game-phase lock

## Purpose

Lock the playable-sim game phase: confirm feature completeness against the spec, generate final report, produce backlog/issues for future work, and ensure no hidden implementation plans remain.


## Global constraints inherited from P01-P36

- Do not create P37. This is the `Gxx` playable-sim product phase.
- Keep `alife_core` engine-independent. No Bevy, Avian, wgpu, renderer, ECS, OS-windowing, Python runtime objects, Unity, C#, or HLSL production dependencies in core.
- Headless CPU remains the correctness oracle and default CI path.
- Bevy/Avian, GPU, semantic/SLM, school UI, and research tooling remain optional or feature-gated unless a plan explicitly hardens them.
- Preserve P09 structured action arbitration. No direct motor bypass from memory, topology, teacher, semantic provider, UI, or debug tools.
- Preserve sealed three-phase `ExperiencePatch` before learning, memory, topology, logging, or school verification.
- Preserve P34 stable-ID save/load policy. Do not persist Bevy `Entity`, Avian handles, wgpu handles, renderer handles, or OS window handles.
- Do not claim GPU performance without measured hardware evidence. CPU fallback reports are not GPU timing claims.
- Do not commit large logs, generated tensors, GPU captures, or benchmark artifacts.
- Do not weaken existing P36 release gates or golden trace policy.


## Owned scope

- final game-phase status report
- feature-complete checklist
- backlog/issues docs
- tag proposal docs

## Likely files/crates to inspect or touch

- docs/playable_sim_spec/**
- docs/final_game_status_report.md
- README.md if factual status update needed

## Forbidden scope

- Adding features
- Creating P37
- Tagging release without explicit user approval
- Hiding known limitations

## Implementation milestones

1. Compare final implementation against product game spec.
2. Classify every feature as complete, partial/manual, intentionally deferred, or blocked.
3. Create final backlog with prioritized future work and issue templates.
4. Create tag/release proposal, not the tag.
5. Run final validation and audit no new plan exists.

## Required tests and evidence

- Final validation suite.
- Docs path/command checks.
- No P37 search.
- Release report contains known limitations and tag proposal.

## Acceptance criteria

- Feature-complete status is explicit and defensible.
- No hidden blockers or untracked future plans.
- Next work is issues/backlog or explicit release/tag decision, not Codex autoplan.

## Failure handling

- If validation fails, classify the failure before editing: compile/API mismatch, feature-gating issue, nondeterminism, dependency leak, scope leak, missing existing contract, manual hardware limitation, or test expectation problem.
- Apply the smallest local repair. Do not rewrite completed P01-P36 systems unless the current plan exposes a direct blocker and the fix is clearly scoped.
- If a plan depends on unavailable local hardware or graphics, keep a CI-safe headless/default test and record an exact manual command. Do not fabricate results.
- If the work appears to require a later G-plan, stop and write an integration note rather than implementing the later scope.
- Rerun the narrow failed test first, then the full validation set.


## Standard validation commands

Run these before the completion receipt. On Windows, never run plain `bash scripts/check.sh`; use the PowerShell wrappers.

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

If a graphical/GPU/manual command is required and the local machine cannot run it, keep a CI-safe smoke test and record the exact manual command and limitation. Do not claim hardware evidence that was not measured.


## Review checklist

- The plan implemented only `G24` scope.
- `alife_core` remains engine-independent.
- Optional systems remain optional unless this plan explicitly hardens them.
- Existing P36 release gates are not weakened.
- New public schemas/configs/assets are versioned and validated.
- Tests cover the main behavior and failure paths.
- Manual/hardware limitations are documented with exact commands.
- Docs/progress files under `docs/playable_sim_spec/` are updated.
- No large generated artifacts are committed.
- No P37 was created.

## Required completion receipt

```text
Completion receipt
Plan: G24 - Feature-complete roadmap lock and post-release backlog
Branch: codex/G24-feature-complete-roadmap-lock
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): None - final game-phase lock
```

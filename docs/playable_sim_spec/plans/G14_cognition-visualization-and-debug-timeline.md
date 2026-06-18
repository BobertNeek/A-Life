# G14 - Cognition visualization and debug timeline

Branch: `codex/G14-cognition-visualization`

Prerequisites: G05, G03

Concurrency: Serial visualization contract.

Recommended model/reasoning: GPT-5.5 High

Next plan(s): G15

## Purpose

Expose readable cognition/debug views for players/developers: experience timeline, drives/hormones, memory expectancy, topology gaps, action arbitration, sleep consolidation, and optional neural/GPU summaries without active neural readback.


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

- debug UI panels
- timeline/log viewers
- offline log viewer bridge
- read-only cognition summaries

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_tools/**
- crates/alife_core summary types only if needed
- docs/playable_sim_spec/**

## Forbidden scope

- Bulk active neural readback
- Mutating cognition through debug panels
- Overclaiming internal mental state
- Implementing offline research dashboards beyond integrated summaries

## Implementation milestones

1. Add sealed patch timeline view.
2. Add action arbitration/proposal summary view.
3. Add memory expectancy and topology gap summaries.
4. Add sleep/consolidation summary.
5. Add optional GPU/backend diagnostics summary at allowed boundaries.
6. Add export button/command for packed logs if cheap.

## Required tests and evidence

- Timeline uses sealed patches only.
- Inspector does not mutate state.
- Memory/topology output is bias-only.
- GPU diagnostics only boundary-scoped.

## Acceptance criteria

- Player/developer can understand why a creature did something.
- Views are read-only and bounded.
- No forbidden readback or action bypass.

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

- The plan implemented only `G14` scope.
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
Plan: G14 - Cognition visualization and debug timeline
Branch: codex/G14-cognition-visualization
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): G15
```

# G03 - Live brain loop bridge and game tick scheduler

Branch: `codex/G03-live-brain-loop-bridge`

Prerequisites: G02

Concurrency: Serial high-risk runtime integration.

Recommended model/reasoning: GPT-5.5 High or Pro High

Next plan(s): G04

## Purpose

Run the existing CPU reference creature loop inside the visible app scheduler with strict causal order: gather sensory, tick brain, execute action, observe outcome, seal patch, update logs. This converts visible scene from display to live sim.


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

- game app scheduler systems
- adapter bridge between Bevy scene and P15/P17 contracts
- tick/pause/step controls at runtime level
- integration tests

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_bevy_adapter/src/plugin.rs
- crates/alife_world/**
- crates/alife_core/src/reference_brain.rs

## Forbidden scope

- Replacing the CPU oracle
- Bypassing action arbitration
- Learning from unsealed patches
- Adding GPU as default runtime
- Implementing G04 rendering polish or G06 full gameplay loop

## Implementation milestones

1. Define the app tick schedule and explicitly align it with P15/P21 causal sets.
2. Bridge Bevy visible objects into sensory snapshots through adapter contracts.
3. Run `CreatureMind` for at least one creature in app state.
4. Execute selected `ActionCommand` through adapter/world legality, then observe outcome.
5. Return sealed `ExperiencePatch` and optional packed log summary.
6. Implement pause, single-step, and fixed-rate run modes.
7. Add diagnostics for tick status, selected action, and patch seal status.

## Required tests and evidence

- Headless equivalent and app-bridge tick produce compatible patch summaries on tiny fixture.
- Invalid action path creates recoverable failure, not panic.
- Pause/step does not advance hidden state unexpectedly.
- No direct teacher/memory/topology action bypass.

## Acceptance criteria

- A visible app tick produces sealed patches from a real P15 brain path.
- No update occurs from invalid/unsealed experience.
- Pause/step/run modes are deterministic.
- Core remains engine-independent.

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

- The plan implemented only `G03` scope.
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
Plan: G03 - Live brain loop bridge and game tick scheduler
Branch: codex/G03-live-brain-loop-bridge
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): G04
```

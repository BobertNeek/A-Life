# G02 - Visible world binding and stable entity presentation

Branch: `codex/G02-visible-world-binding`

Prerequisites: G01

Concurrency: Serial after app shell.

Recommended model/reasoning: GPT-5.5 High or Pro High

Next plan(s): G03

## Purpose

Bind the deterministic headless world data to a visible Bevy scene with stable entity mapping, simple geometry, and debug-safe presentation. This is visual world display, not yet live cognition stepping.


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

- game app world presentation modules
- adapter-local entity mapping use
- small placeholder assets/materials
- world spawn/despawn tests and docs

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_bevy_adapter/**
- crates/alife_world/**
- examples/p35/** or docs/playable_sim_spec/**

## Forbidden scope

- Changing `alife_core` IDs to suit Bevy
- Serializing Bevy entities in saves
- Implementing live brain loop G03
- Overbuilding art pipeline

## Implementation milestones

1. Create visual representations for ground, food, hazards, obstacles, and one creature using simple meshes/materials.
2. Load initial visible world from P34/P17/P18 tiny fixtures.
3. Map stable `WorldEntityId` to Bevy entities via adapter-local tables only.
4. Add deterministic spawn ordering and stable debug labels.
5. Implement visual reset/reload path from config.
6. Add world signature comparison between headless and visible representation.

## Required tests and evidence

- Stable ID to Bevy entity mapping round-trip in adapter/app tests.
- Scene build from tiny fixture contains expected food/hazard/creature placeholder objects.
- Reset with same seed produces same stable signature.
- Headless smoke still passes without Bevy.

## Acceptance criteria

- The visible world shows the same stable objects as the headless fixture.
- Engine-local IDs never enter portable saves or core.
- Object placement is deterministic from seed/config.
- No live creature cognition is required yet.

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

- The plan implemented only `G02` scope.
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
Plan: G02 - Visible world binding and stable entity presentation
Branch: codex/G02-visible-world-binding
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): G03
```

# G13 - Player world editing and scenario sandbox tools

Branch: `codex/G13-world-editing-tools`

Prerequisites: G07, P34 complete

Concurrency: Can have docs helper after tool API stabilized.

Recommended model/reasoning: GPT-5.5 High

Next plan(s): G14

## Purpose

Add simple in-game/dev sandbox tools for placing/removing food, hazards, obstacles, and creatures; changing resource rates; and saving edited worlds through P34 stable IDs.


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

- world editor UI
- placement/removal systems
- sandbox config/save integration
- small editor tests

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_world/**
- docs/playable_sim_spec/**

## Forbidden scope

- Full content-authoring pipeline G16
- Engine-local IDs in saved worlds
- Unbounded spawning
- Changing core to support editor shortcuts

## Implementation milestones

1. Add editor mode toggle separate from simulation mode.
2. Implement place/remove/move for core object types.
3. Validate edits against world bounds/entity caps.
4. Pause simulation while editing or define deterministic edit tick boundary.
5. Save edited world through P34 references.
6. Add undo/reset if cheap; otherwise document reset path.

## Required tests and evidence

- Place/remove entity deterministic.
- Invalid edit rejected.
- Edited world save/load roundtrip.
- Simulation resumes with sealed patch path intact.

## Acceptance criteria

- Player/developer can create a small playable world without hand-editing JSON.
- Saved edits use stable IDs and validate.
- Editing does not mutate cognition directly.

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

- The plan implemented only `G13` scope.
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
Plan: G13 - Player world editing and scenario sandbox tools
Branch: codex/G13-world-editing-tools
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): G14
```

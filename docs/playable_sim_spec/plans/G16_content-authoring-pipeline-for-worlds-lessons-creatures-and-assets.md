# G16 - Content authoring pipeline for worlds, lessons, creatures, and assets

Branch: `codex/G16-content-authoring`

Prerequisites: G15, G10, G13

Concurrency: Can use Spark for docs/fixtures after schemas fixed.

Recommended model/reasoning: GPT-5.5 High

Next plan(s): G17

## Purpose

Create a small authoring pipeline so designers/developers can add worlds, lessons, creature presets, generated weight refs, semantic assets, and scenario packs without editing code.


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

- content schema docs
- validation CLI extensions
- sample content packs
- authoring tests

## Likely files/crates to inspect or touch

- crates/alife_tools/**
- crates/alife_world/**
- crates/alife_school/**
- docs/playable_sim_spec/**
- content/fixtures/** if small

## Forbidden scope

- Heavy editor UI beyond simple tools
- Unversioned content
- Large assets
- Runtime dependence on research tooling

## Implementation milestones

1. Define content pack manifest that references P34 assets/configs.
2. Add validation commands for world/lesson/creature presets.
3. Add sample small content pack.
4. Add docs for adding actions/sensory channels/lessons/assets.
5. Integrate with world editor export if feasible.
6. Reject missing/large/incompatible content clearly.

## Required tests and evidence

- Content pack manifest validates.
- Missing asset rejects.
- Lesson pack remains perception-only.
- Creature preset genome valid.
- No huge assets tracked.

## Acceptance criteria

- A non-core code change can add a small world/lesson/preset and validate it.
- Content remains versioned and stable-ID based.

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

- The plan implemented only `G16` scope.
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
Plan: G16 - Content authoring pipeline for worlds, lessons, creatures, and assets
Branch: codex/G16-content-authoring
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): G17
```

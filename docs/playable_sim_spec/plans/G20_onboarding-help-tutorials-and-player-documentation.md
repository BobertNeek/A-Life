# G20 - Onboarding, help, tutorials, and player documentation

Branch: `codex/G20-onboarding-help`

Prerequisites: G15, G19

Concurrency: Can run docs-heavy after feature set stabilizes.

Recommended model/reasoning: Codex Spark or GPT-5.5 Medium acceptable; High for UI integration changes.

Next plan(s): G21

## Purpose

Make the playable sim understandable: first-run instructions, controls, inspector help, tutorial prompts, troubleshooting, and developer extension documentation.


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

- in-game help text
- docs/tutorials
- controls reference
- troubleshooting docs
- sample guided scenarios

## Likely files/crates to inspect or touch

- docs/**
- crates/alife_game_app/** help data
- examples/**

## Forbidden scope

- Changing runtime behavior to fit docs
- Overclaiming maturity
- Adding release packaging
- Making optional demos mandatory

## Implementation milestones

1. Write first-run guide and controls reference.
2. Add in-game help overlay or help panel if app UI supports it.
3. Add tutorial scenario script for food/hazard/sleep/inspection.
4. Document common errors: GPU unavailable, graphics unavailable, schema mismatch, save failure, nondeterminism.
5. Document mod/content extension path from G16.

## Required tests and evidence

- Docs path/command validation.
- Tutorial config loads.
- Help text references existing controls.
- Windows wrapper commands used.

## Acceptance criteria

- A new developer/player can run the sim and understand controls and limitations.
- Docs match actual commands and files.

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

- The plan implemented only `G20` scope.
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
Plan: G20 - Onboarding, help, tutorials, and player documentation
Branch: codex/G20-onboarding-help
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): G21
```

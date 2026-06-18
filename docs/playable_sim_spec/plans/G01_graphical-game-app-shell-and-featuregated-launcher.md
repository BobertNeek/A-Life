# G01 - Graphical game app shell and feature-gated launcher

Branch: `codex/G01-game-app-shell`

Prerequisites: G00

Concurrency: Serial product foundation.

Recommended model/reasoning: GPT-5.5 High or Pro High

Next plan(s): G02

## Purpose

Create the minimal Bevy application shell for the playable sim without yet implementing the full visible world. The app must start, load configuration, expose a deterministic headless fallback, and preserve all optional feature boundaries.


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

- new or existing game app crate such as `alife_game_app` or `alife_playground_app`
- example launchers under `examples/` if appropriate
- P34 config loading glue
- docs/playable_sim_spec progress

## Likely files/crates to inspect or touch

- Cargo.toml workspace
- crates/alife_game_app/** or examples/alife_game_app.rs
- crates/alife_bevy_adapter/**
- crates/alife_world/tests/fixtures/p34/**
- docs/playable_sim_spec/**

## Forbidden scope

- Adding gameplay systems beyond boot/loading/empty world shell
- Putting Bevy types into `alife_core`
- Making GPU/semantic/school mandatory
- Adding release packaging
- Implementing G02+ visible world content

## Implementation milestones

1. Choose whether to create a dedicated game app crate or an example binary; record rationale.
2. Add feature-gated Bevy app shell that compiles with existing Bevy adapter features.
3. Add app states: Boot, LoadConfig, MainMenu or DevMenu, Running, Paused, Shutdown.
4. Load a tiny P34 config/asset manifest and report a validated app startup summary.
5. Add a headless fallback command/path for CI when graphics is unavailable.
6. Add manual run command for the graphical app and CI-safe smoke command.
7. Document feature flags and default behavior.

## Required tests and evidence

- App-shell config loads P34 tiny config.
- Headless shell smoke passes in CI.
- Feature-gated Bevy app compiles where features are enabled.
- Invalid config rejects with clear diagnostics.
- Core boundary scripts pass.

## Acceptance criteria

- A developer can launch a minimal app or headless app-shell smoke without touching core.
- Default CI path does not require graphics/GPU hardware.
- P34 config validation is used, not bypassed.
- No gameplay claims beyond app shell.

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

- The plan implemented only `G01` scope.
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
Plan: G01 - Graphical game app shell and feature-gated launcher
Branch: codex/G01-game-app-shell
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): G02
```

# G18 - Population performance, LOD, and scalability pass

Branch: `codex/G18-population-performance`

Prerequisites: G08, G12, G17

Concurrency: Serial performance gate.

Recommended model/reasoning: GPT-5.5 High or Pro High

Next plan(s): G19

## Purpose

Make the game run acceptably with growing populations by adding simulation/render LOD, cadence throttling, benchmark integration, and honest CPU/GPU performance evidence.


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

- performance policies
- LOD/cadence systems
- benchmark reports
- render/sim decimation configs
- performance docs

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_tools/src/benchmark.rs
- crates/alife_gpu_backend/**
- docs/playable_sim_spec/**

## Forbidden scope

- Weakening correctness to hit FPS
- Removing parity/golden/soak gates
- Claiming unmeasured GPU performance
- Making GPU mandatory

## Implementation milestones

1. Define target tiers and minimum playable target for v1.
2. Integrate P20/P29 benchmarks with visible app/headless population scenarios.
3. Add render LOD and nonessential cognition cadence controls.
4. Protect sensory/motor/homeostasis priority.
5. Record CPU and available GPU performance honestly.
6. Add performance overlay and report generation.

## Required tests and evidence

- Tier 1/10 CI smoke.
- Manual 50/100/250/500 commands documented.
- Throttle protects sensory/motor.
- Golden behavior not corrupted by LOD.

## Acceptance criteria

- Tier targets are measured or explicitly unknown.
- Performance throttling preserves behavior-critical systems.
- Population demo remains stable.

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

- The plan implemented only `G18` scope.
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
Plan: G18 - Population performance, LOD, and scalability pass
Branch: codex/G18-population-performance
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): G19
```

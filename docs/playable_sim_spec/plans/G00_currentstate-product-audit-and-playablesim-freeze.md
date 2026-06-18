# G00 - Current-state product audit and playable-sim freeze

Branch: `codex/G00-product-audit`

Prerequisites: P36 complete on main

Concurrency: Serial. This is the entry gate for the game phase.

Recommended model/reasoning: GPT-5.5 High or Pro High

Next plan(s): G01

## Purpose

Create a truthful product-readiness audit that separates real working backend behavior from scaffold contracts, diagnostic paths, fake providers, scripted fixtures, and missing player-facing systems. This plan imports/validates the new game-phase spec and freezes the baseline before any gameplay implementation.


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

- docs/playable_sim_spec/ audit documents
- game-phase progress file
- backend confidence matrix
- expanded plan verification notes

## Likely files/crates to inspect or touch

- README.md
- docs/final_status_report.md
- docs/playground_examples.md
- docs/release_checklist.md
- docs/architecture_decisions.md
- crates/alife_core/**
- crates/alife_world/**
- crates/alife_bevy_adapter/**
- crates/alife_gpu_backend/**
- crates/alife_school/**
- crates/alife_semantic/**
- crates/alife_tools/**
- examples/**

## Forbidden scope

- Implementing game code
- Creating P37
- Changing P01-P36 historical plan files except factual cross-reference
- Weakening P36 gates
- Claiming backend intelligence/gameplay maturity without evidence

## Implementation milestones

1. Verify current main is clean, pushed, and at/after the post-P36 consistency fix.
2. Import this spec pack under `docs/playable_sim_spec/` if not already present.
3. Produce `G00_backend_confidence_audit.md` classifying every subsystem as real implementation, contract/scaffold only, fixture/scripted behavior, diagnostic/manual only, missing for product gameplay, and integration risk.
4. Produce `G00_backend_confidence_matrix.md` with current evidence, test evidence, dummy/scaffold pieces, product requirement, confidence rating, and the G-plan that closes each gap.
5. Produce `GAME_PHASE_PROGRESS.md` initialized to G00 in-progress/complete.
6. Verify the expanded G-plans are present and detailed enough before implementation begins.
7. Generate the next prompt for G01 only.

## Required tests and evidence

- Docs/check validation passes.
- No runtime code changed unless strictly needed for docs import hygiene.
- Search confirms no P37 plan introduced.
- `alife_core` dependency boundary remains clean.

## Acceptance criteria

- Backend confidence audit exists and is candid about dummy/scaffold/manual behavior.
- Every major subsystem has a confidence rating and a closure plan.
- No code implementation occurred.
- P01-P36 remain historical foundation, not reopened as P37.
- Validation passes.

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

- The plan implemented only `G00` scope.
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
Plan: G00 - Current-state product audit and playable-sim freeze
Branch: codex/G00-product-audit
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): G01
```

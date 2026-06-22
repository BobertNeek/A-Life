# S08 - GPU, graphics, and performance evidence pass

Branch: `codex/S08-gpu-graphics-performance-evidence`

Dependencies:
- S07

Recommended model/reasoning: GPT-5.5 High or Extra High

Next plan(s): S09

## Purpose

Gather honest GPU, graphics, and performance evidence, improve settings/fallback UX, and keep claims bounded by actual hardware measurements.

## Owned scope

- GPU runtime evidence
- graphics smoke evidence
- performance reports
- settings/fallback display
- S08 report

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_gpu_backend/**
- crates/alife_tools/**
- scripts/run_graphical_playground.ps1
- docs/productization/**

## Forbidden scope

- claiming GPU performance from CPU fallback
- requiring GPU for default path
- active neural readback
- hardware-specific hacks

## Implementation milestones

1. Run CPU benchmark smoke and optional GPU runtime report.
2. If hardware flags available, record real GPU backend/timing.
3. Show backend/fallback/unknown status in UI/report.
4. Verify no active readback.
5. Measure launch/window smoke timing where possible.
6. Document 60 FPS target status.

## Required tests and evidence

- CPU fallback works
- GPU unavailable fallback is honest
- no active readback
- settings/status display reflects backend
- manual GPU command documented

## Acceptance criteria

- Performance evidence is measured or explicitly unknown/manual.
- No false GPU claims.
- Settings/status surface gives player/tester clear backend state.

## Focused commands

```powershell
cargo run -p alife_tools --bin benchmark_tiers
```
```powershell
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```
```powershell
ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
```
```powershell
cargo run -p alife_game_app --bin alife_game_app -- gpu-product-smoke
```

## Computer-use / manual evidence

- Hardware ID, backend requested/selected, fallback, timings, FPS target status.
- Create `S08_GPU_GRAPHICS_PERFORMANCE_REPORT.md`.

## Failure handling

- If hardware unavailable, mark MANUAL_EVIDENCE_MISSING, not pass.
- If GPU selected path fails, preserve CPU fallback and report blocker/high depending on release target.

## Review checklist

- The plan implemented only `S08` scope.
- Runtime/code changes match the plan's owned scope.
- `alife_core` remains engine-independent.
- Headless CPU path remains green.
- Optional graphics/GPU/semantic/school systems remain optional unless explicitly hardened.
- No P37/G25/new automatic chain was created.
- Product claims match actual evidence.
- Reports under `docs/productization/` are honest about unavailable manual evidence.



## Global invariants

Read and obey:

- `docs/productization_s_plans/GLOBAL_INVARIANTS.md` if imported there, or the imported equivalent under the productization plan pack.
- Existing repo invariants in `AGENTS.md`.
- Existing P36/R24 validation discipline.

## Standard validation

Use Windows wrappers. Do not run plain `bash scripts/check.sh`.

Run the standard validation set from `VALIDATION_PROTOCOL.md`, plus each plan's focused commands.

## Completion receipt

```text
Completion receipt
Plan:
Branch:
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s):
Stopped:
```


## Required receipt override

```text
Completion receipt
Plan: S08 - GPU, graphics, and performance evidence pass
Branch: codex/S08-gpu-graphics-performance-evidence
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s): S09
Stopped: yes
```

# CA44 - External Alpha/Beta Playtest Loop

## Status

`USER_ACTION_REQUIRED`

CA44 cannot honestly be marked complete yet. The repository contains local
Codex Computer Use evidence and local smoke evidence, but no independent human
external tester submission is available to triage.

The existing `docs/productization/EXTERNAL_ALPHA_TESTER_001_REPORT.md` is
explicit that it is a local Codex Computer Use playtest, not an independent
human external alpha pass. CA44 therefore stops before CA45 rather than
inventing a playtest result.

## Plan

- Plan: CA44
- Title: External alpha/beta playtest loop
- Branch: `codex/CA44-external-alpha-beta-playtest-loop`
- Result: blocker/status package only
- Next executable plan: CA44 after external tester evidence is provided

## Files Changed

- `docs/creatures_agi_roadmap_pack/templates/CA44_EXTERNAL_TESTER_INTAKE_REPORT.md`
- `docs/creatures_agi_roadmap_pack/status/CA44_EXTERNAL_ALPHA_BETA_PLAYTEST_LOOP.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`

## Tester Matrix Required

Minimum evidence before CA44 can complete:

| Tester Slot | Required Coverage | Required Evidence |
| --- | --- | --- |
| CA44-T01 | Independent human tester on Windows with GPU mode requested | Filled CA44 intake report, screenshot/video references, launcher output path |
| CA44-T02 | Independent human tester or maintainer on forced CPU fallback/degraded mode | Filled intake report showing fallback is explicit and not GPU evidence |
| CA44-T03 | Package runner test on a clean or near-clean Windows environment | Package runner command result, crash summary path if any failure occurs |

Optional but recommended:

| Tester Slot | Coverage | Evidence |
| --- | --- | --- |
| CA44-T04 | Second GPU/driver combination | Filled intake report with GPU/fallback status |
| CA44-T05 | Lower-resolution display or non-primary monitor | Readability notes and screenshot/video references |

## Intake Report

Use:

```text
docs/creatures_agi_roadmap_pack/templates/CA44_EXTERNAL_TESTER_INTAKE_REPORT.md
```

Raw evidence must remain untracked under a local path such as:

```text
target/playtest_evidence/alpha/TESTER_OR_RUN_ID/
```

Do not commit screenshots, videos, terminal logs, crash dumps, package
artifacts, model files, or generated captures.

## Severity Triage

CA44 uses this triage policy:

- `BLOCKER`: fix before CA45; CA44 cannot complete.
- `HIGH`: fix before CA45; CA44 cannot complete.
- `MEDIUM`: either fix before CA45 or explicitly defer with user approval.
- `LOW`: may defer to backlog if documented.
- `MANUAL_EVIDENCE_MISSING`: gather the missing evidence or stop with
  `USER_ACTION_REQUIRED`.

No fixes were implemented in this CA44 branch because there are no new external
tester findings to reproduce.

## Beta Criteria

Do not proceed to CA45 release-candidate gate until all are true:

- at least the required tester matrix slots are completed or explicitly waived
  by the user;
- no unresolved `BLOCKER` or `HIGH` findings remain;
- all `MEDIUM` findings are fixed or explicitly accepted/deferred;
- graphical GPU-first launch evidence is captured on the exact tested SHA;
- forced CPU fallback/degraded-mode evidence is captured;
- package runner evidence is captured;
- validation is rerun on the exact SHA considered for beta/release-candidate
  review;
- no release tag is created by CA44.

## Commands To Run After Evidence Is Available

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
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Invariant Checks

- No S12, G25, P37, or hidden continuation plan.
- No release tag.
- No screenshots, videos, logs, captures, target artifacts, generated media,
  model weights, model caches, llama.cpp binaries, or downloaded model files
  are tracked.
- No Bevy, wgpu, GPU, model-runtime, UI, or app dependency added to
  `alife_core`.
- CPU fallback preserved.
- CPU shadow parity preserved.
- Product claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- Full action-authoritative GPU runtime is not claimed.
- Semantic and SLM systems remain perception/context only.

## Known Limitations

- Independent human external tester evidence is still missing.
- No blocker/high/medium external findings can be fixed until evidence is
  provided.
- This branch prepares the CA44 loop and records the stop condition; it does
  not complete the loop.

## Main Status

This blocker/status package is intended to be merged after validation. The
post-merge receipt records the exact main branch status.

Next plan: CA44 after user provides external tester evidence. Do not start CA45.

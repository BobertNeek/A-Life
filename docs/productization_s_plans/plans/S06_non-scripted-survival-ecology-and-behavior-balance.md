# S06 - Non-scripted survival, ecology, and behavior balance

Branch: `codex/S06-nonscripted-ecology-balance`

Dependencies:
- S05

Recommended model/reasoning: GPT-5.5 High or Extra High

Next plan(s): S07

## Purpose

Move beyond smoke-scripted behavior by improving non-scripted survival/ecology dynamics and balance evidence, while preserving deterministic metrics.

## Owned scope

- balance configs
- long-run smoke scenarios
- behavior metrics reports
- degenerate behavior log
- S06 report

## Likely files/crates to inspect or touch

- crates/alife_game_app/**
- crates/alife_world/**
- crates/alife_tools/**
- docs/productization/**

## Forbidden scope

- changing core contracts to force behavior
- hiding failure metrics
- overfitting golden traces
- unbounded CI tests

## Implementation milestones

1. Define non-scripted behavior criteria for food seeking, hazard avoidance, sleep/rest, reproduction/death bounds, resource stability.
2. Add deterministic balance smoke with less scripted hazard contact if feasible.
3. Add or improve ignored extended balance command.
4. Tune configs where possible, not invariants.
5. Record degenerate behaviors honestly.
6. Create balance report.

## Required tests and evidence

- fast balance smoke
- deterministic replay
- bounded population/resources
- no NaN/invalid IDs/unsealed learning
- ignored extended balance command documented

## Acceptance criteria

- Fast long-run smoke remains bounded and reproducible.
- At least one non-scripted behavior metric improves or is honestly reported as not yet emergent.
- Known limitations are explicit.

## Focused commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke
```
```powershell
cargo test -p alife_game_app --test app_shell g19_manual_extended_balance_run -- --ignored --nocapture
```

## Computer-use / manual evidence

- Balance metric report with seeds/configs.
- Create `S06_BALANCE_ECOSYSTEM_REPORT.md`.

## Failure handling

- If behavior remains heavily scripted, label it clearly and recommend next balancing work.
- If tuning destabilizes tests, revert and record gap.

## Review checklist

- The plan implemented only `S06` scope.
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
Plan: S06 - Non-scripted survival, ecology, and behavior balance
Branch: codex/S06-nonscripted-ecology-balance
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Computer-use / manual evidence:
Deviations:
Known limitations:
Next plan(s): S07
Stopped: yes
```

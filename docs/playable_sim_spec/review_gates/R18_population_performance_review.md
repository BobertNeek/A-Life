# R18 - Population Performance Review

## Review ID

R18 - Population/performance/scalability review before G19

## Branch

`codex/R18-population-performance-review`

## Dependencies

- G18 complete on `main`.
- G07, G08, G09, G12, and G17 results available for review.

## Purpose

R18 is a hard stop before G19 long-run balance work. It checks whether population scale, LOD, GPU fallback, and user-visible playability evidence are strong enough to tune the ecosystem without hiding scalability or performance gaps.

## Owned Scope

- Review only.
- Population scale and LOD audit.
- Performance/scalability evidence audit.
- Long-run stability readiness audit.
- Review report and progress/traceability updates needed to record the gate result.

## Forbidden Scope

- No G19 implementation.
- No new runtime features.
- No GPU architecture changes.
- No benchmark result fabrication.
- No weakening of performance, determinism, or boundary tests.

## Exact Review Checklist

- Population caps, update cadence, and LOD policies are deterministic and bounded.
- Sensory/motor and survival-critical work retain priority over non-essential visual or cognitive work.
- G18 performance claims are backed by measured data or clearly marked unknown/manual.
- CPU fallback remains available and correct.
- GPU acceleration remains optional and does not require active neural readback.
- Ecology/social/lifecycle loops remain stable under the reviewed population counts.
- Long-run memory/topology/log growth remains bounded or has explicit caps.
- The sim is fun/playable enough to justify G19 tuning rather than merely passing tests.
- No false GPU performance claims appear in docs, logs, or release notes.
- No optional GPU/graphics path becomes required by headless tests.
- `alife_core` remains engine-independent.

## Validation Commands

Run on Windows with wrappers, not plain `bash scripts/check.sh`:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
```

Run G18 performance smoke/manual commands that are documented by G18. If hardware is unavailable, record the exact limitation.

## Hard Stop Condition

Stop after the R18 report. Do not start G19 automatically. If the report is `FIX_REQUIRED` or `BLOCKER`, G19 may not proceed until the exact fix prompt is completed and validated.

## Completion Receipt

```text
R18 review receipt
Review: R18 - Population/performance/scalability review before G19
Branch: codex/R18-population-performance-review
Verdict: PASS / FIX_REQUIRED / BLOCKER
Findings:
G19 may proceed: yes/no
Commands run:
Results:
Performance evidence:
Invariant checks:
Fix prompt if needed:
Next plan: G19
Stopped before G19: yes
```

## Next Plan

G19, only after explicit user authorization following the R18 receipt.

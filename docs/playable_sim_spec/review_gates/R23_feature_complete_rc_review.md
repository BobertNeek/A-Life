# R23 - Feature-Complete Release-Candidate Review

## Review ID

R23 - Feature-complete release-candidate review before G24

## Branch

`codex/R23-feature-complete-rc-review`

## Dependencies

- G23 complete on `main`.
- G01-G23 product work available for review.

## Purpose

R23 is a hard stop before G24 roadmap lock. It determines whether the playable sim is actually feature-complete enough to lock the roadmap, or whether release-candidate gaps need explicit fixes first.

## Owned Scope

- Review only.
- Feature-complete release-candidate audit.
- UX, save/load, graphics, school, semantic, GPU, packaging, and known-limitation audit.
- Review report and progress/traceability updates needed to record the gate result.

## Forbidden Scope

- No G24 implementation.
- No new runtime features.
- No release-tagging or packaging automation beyond review evidence.
- No hiding release-candidate gaps.
- No weakening of validation or acceptance tests.

## Exact Review Checklist

- Core user loops are feature-complete and documented.
- UX surfaces are coherent, navigable, and do not expose misleading backend-only promises.
- Save/load UX uses P34 stable IDs and versioned schemas.
- Visual loop, camera, inspector, creature presentation, and feedback readability are integrated.
- School/teacher mode remains perception-only and does not bypass arbitration.
- Semantic/Gaussian/SLM provider remains optional, bounded, and non-authoritative.
- GPU path remains optional with CPU fallback and no active neural readback.
- Packaging smoke and asset bundle discipline are documented and tested where portable.
- Known limitations are explicit and not contradicted by quickstart or release-candidate docs.
- No huge generated assets, logs, or tensors are committed.
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

Run G23 release-candidate smoke/manual commands documented by G23. Record hardware or graphics limitations honestly.

## Hard Stop Condition

Stop after the R23 report. Do not start G24 automatically. If the report is `FIX_REQUIRED` or `BLOCKER`, G24 may not proceed until the exact fix prompt is completed and validated.

## Completion Receipt

```text
R23 review receipt
Review: R23 - Feature-complete release-candidate review before G24
Branch: codex/R23-feature-complete-rc-review
Verdict: PASS / FIX_REQUIRED / BLOCKER
Findings:
G24 may proceed: yes/no
Commands run:
Results:
Feature-complete evidence:
Invariant checks:
Fix prompt if needed:
Next plan: G24
Stopped before G24: yes
```

## Next Plan

G24, only after explicit user authorization following the R23 receipt.

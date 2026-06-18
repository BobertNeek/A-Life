# R13 - Retrospective Product Boundary Review

## Review ID

R13 - Retrospective product boundary review after G13 before G14

## Branch

`codex/R13-retrospective-product-boundary-review`

## Dependencies

- G01 through G13 complete on `main`.
- P36 release gates remain intact.
- Post-P36 playable-sim progress marks G13 complete.

## Purpose

R13 is a hard stop before G14 cognition visualization work. It retrospectively audits G01-G13 because the intended human checkpoints after G03, G06, and G12 were missed. It does not rewrite history or insert R11/R12 as active gates.

## Owned Scope

- Review only.
- Product boundary audit for G01-G13.
- Maintainability audit for `alife_game_app` before additional large UX/debug additions.
- Review report and progress/traceability updates needed to record the gate result.

## Forbidden Scope

- No new runtime features.
- No G14 implementation.
- No GPU changes.
- No save/schema redesign.
- No `alife_core` changes unless a release-blocking dependency leak is found.
- No P37 creation.

## Exact Review Checklist

- G01 app shell remains feature-gated and headless-safe.
- G02 visible world uses stable IDs and no engine-local persistence.
- G03 live brain loop bridge preserves CPU oracle and sealed patch order.
- G04 creature visuals are presentation only and do not mutate cognition.
- G05 inspector is read-only and stable-ID based.
- G06 survival loop is not overclaimed as full gameplay.
- G07 ecology is deterministic/bounded and not unbounded simulation.
- G08 population/social loop stays bounded and perception/modulatory.
- G09 lifecycle/lineage keeps genetic/lifetime separation.
- G10 school mode remains perception-only and verifier uses sealed patches.
- G11 semantic/SLM provider is optional, bounded, non-authoritative, cannot act, and cannot mutate weights.
- G12 GPU product hardening is optional/fallback-safe, has no active neural readback, and makes no false hardware claims.
- G13 world editor uses stable IDs, bounded edits, P34 save/load, and no cognition mutation.
- `alife_game_app` module organization is still maintainable. If `lib.rs` has become too large or monolithic, recommend a G14-safe module split before further large additions.
- No P37 exists.
- P36 gates remain intact.
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

Add focused G01-G13 smoke commands where the review findings require them.

## Hard Stop Condition

Stop after the R13 report. Do not start G14 automatically. If the report is `FIX_REQUIRED` or `BLOCKER`, G14 may not proceed until the exact fix prompt is completed and validated.

## Completion Receipt

```text
R13 review receipt
Review: R13 - Retrospective product boundary review after G13 before G14
Branch: codex/R13-retrospective-product-boundary-review
Verdict: PASS / FIX_REQUIRED / BLOCKER
Findings:
G14 may proceed: yes/no
Module split required before G14: yes/no
Commands run:
Results:
Invariant checks:
Fix prompt if needed:
Next plan: G14
Stopped before G14: yes
```

## Next Plan

G14, only after explicit user authorization following the R13 receipt.

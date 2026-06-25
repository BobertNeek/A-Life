# CA15 - Allocation-bounded endocrine/homeostasis runtime

Status: complete on `codex/CA15-endocrine-homeostasis-runtime`.

## Scope

CA15 adds a bounded app-level presentation for endocrine and homeostatic state.
It reuses the existing `alife_core::HomeostaticSnapshot` and
`ChemistryModulation` helpers; it does not change core chemistry semantics.

## Evidence Added

- fixed five-register homeostasis presentation:
  - energy
  - hunger
  - fatigue
  - pain
  - stress
- salience and learning modulation lines exposed in runtime panels,
- runtime panel updates the presentation after live ticks,
- read-only inspector shows compact homeostasis and modulation status,
- deterministic `homeostasis-runtime-smoke` command,
- app-shell tests for finite/bounded values, fixed register count, and
  stable-ID-safe overlay text.

## Boundaries

- No Bevy, wgpu, renderer, or GPU dependency was added to `alife_core`.
- The presentation does not emit actions and does not bypass P09 arbitration.
- GPU claims remain unchanged; CPU shadow parity remains the gate.
- The homeostasis bars mirror model state and remain display-only.
- No active bulk neural readback was added.

## Focused Commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- homeostasis-runtime-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo test -p alife_game_app --test app_shell ca15 -- --nocapture
```

Graphical changes are covered by the standard graphical smoke and forced
fallback smoke for this tranche.

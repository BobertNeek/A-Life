You are continuing the A-Life playable-sim product phase.

Use only `docs/playable_sim_spec/` as the canonical G-plan source.

Prerequisite:
- G00 complete on `codex/G00-product-audit`.

Do not create P37.
Do not weaken P36 release gates.
Do not implement G02 or later.

Start from clean main after G00 is merged.
Create branch `codex/G01-game-app-shell`.

Read:
- docs/playable_sim_spec/README.md
- docs/playable_sim_spec/GLOBAL_INVARIANTS.md
- docs/playable_sim_spec/VALIDATION_PROTOCOL.md
- docs/playable_sim_spec/G00_backend_confidence_audit.md
- docs/playable_sim_spec/G00_backend_confidence_matrix.md
- docs/playable_sim_spec/plans/G01_graphical-game-app-shell-and-featuregated-launcher.md

Implement G01 only. Build the graphical game app shell and feature-gated
launcher described by the G01 plan while preserving the headless CPU path as
the default validation path. Stop before G02.

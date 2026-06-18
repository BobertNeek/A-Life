# A-Life Playable Sim Game Completion Spec v2

This pack expands the compact G00-G24 product phase into Codex-ready implementation plans. It starts from the validated P01-P36 backend/headless scaffold and defines the path to a polished, feature-complete simulation game.

This is **not P37**. P01-P36 remain the completed scaffold/release-gate phase. These `Gxx` plans are the new playable-sim product phase.

## How to use

1. Copy this directory into the repository as `docs/playable_sim_spec/`.
2. Commit the spec import only if desired.
3. Start with `plans/G00_currentstate-product-audit-and-playablesim-freeze.md`.
4. Do not implement `G01` until the G00 backend confidence audit is complete.
5. Use `prompts/GOAL_MODE_DRIVER_PROMPT.md` only if you want Codex to work plan-by-plan with strict gates.
6. Use `prompts/GOAL_MODE_DRIVER_PROMPT_REVIEW_GATED.md` for automated Goal Mode after G13. Review gates with `Rxx` IDs are manifest-visible executable hard stops, not internal notes.

## Review gates

The manifest includes explicit review plans so Goal Mode cannot run past product checkpoints without producing a review receipt and stopping:

- `R13` after G13 and before G14. This retrospectively audits G01-G13, including the missed G03/G06/G12 checkpoints.
- `R18` after G18 and before G19.
- `R23` after G23 and before G24.
- `R24` after G24 as the final playable-sim roadmap lock review.

Do not add active R11/R12 gates because G12 and G13 are already merged. R13 is the corrective review gate for that history.

## Why this pack exists

The current project is validated as backend/headless/tooling/persistence/developer playground. The missing layer is the player-facing graphical sim game: Bevy app, visible world, creature inspection, live brain loop, ecology, lifecycle, school UX, semantic provider UX, save/load UX, performance hardening, and playtest/release-candidate discipline.

## Key rule

Never weaken the P36 release gates to make the graphical game easier. The headless backend stays the oracle.

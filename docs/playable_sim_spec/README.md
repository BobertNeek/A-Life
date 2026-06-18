# A-Life Playable Sim Game Completion Spec v2

This pack expands the compact G00-G24 product phase into Codex-ready implementation plans. It starts from the validated P01-P36 backend/headless scaffold and defines the path to a polished, feature-complete simulation game.

This is **not P37**. P01-P36 remain the completed scaffold/release-gate phase. These `Gxx` plans are the new playable-sim product phase.

## How to use

1. Copy this directory into the repository as `docs/playable_sim_spec/`.
2. Commit the spec import only if desired.
3. Start with `plans/G00_currentstate-product-audit-and-playablesim-freeze.md`.
4. Do not implement `G01` until the G00 backend confidence audit is complete.
5. Use `prompts/GOAL_MODE_DRIVER_PROMPT.md` only if you want Codex to work plan-by-plan with strict gates.

## Why this pack exists

The current project is validated as backend/headless/tooling/persistence/developer playground. The missing layer is the player-facing graphical sim game: Bevy app, visible world, creature inspection, live brain loop, ecology, lifecycle, school UX, semantic provider UX, save/load UX, performance hardening, and playtest/release-candidate discipline.

## Key rule

Never weaken the P36 release gates to make the graphical game easier. The headless backend stays the oracle.

# A-Life Productization Plan Pack S01-S11

Version: `1.0-productization`  
Created: `2026-06-21`

This pack starts after the locked P01-P36 and G00-G24/R24 phases.

Current baseline:

- Supported validated playable path: headless CPU playground plus deterministic product smoke suite.
- Graphical app evidence: S00 found no persistent player-facing game window; the graphical launcher ran a visible-world smoke and exited.
- GPU performance: manual/unknown unless hardware evidence is explicitly gathered.
- No next automatic implementation plan exists in the locked roadmap.

This pack is the next explicit phase to reach a polished playable graphical game.

## S-plan sequence

1. S01 - Persistent graphical app window and launch stabilization
2. S02 - Minimal interactive player loop and runtime controls
3. S03 - Camera, selection, creature inspector, and screenshot evidence
4. S04 - Visual world readability and feedback polish
5. S05 - Player-facing save/load/menu UX
6. S06 - Non-scripted survival, ecology, and behavior balance
7. S07 - Social, lifecycle, school, and semantic gameplay UX
8. S08 - GPU, graphics, and performance evidence pass
9. S09 - Content, tutorial, scenario, and world authoring pass
10. S10 - Packaging, QA, external playtest candidate
11. S11 - Final playtest, release/tag decision, and next-stage handoff

## Control policy

Use `prompts/GOAL_MODE_S01_TO_S11.md` for Codex Goal Mode.

The goal prompt allows sequential execution but requires per-plan review and full validation. It stops on validation failure, FIX_REQUIRED/BLOCKER, architecture ambiguity, or after S11.

## Evidence policy

Each S-plan must produce or update a report under `docs/productization/`. GUI screenshots and logs stay in `target/playtest_evidence/` and are referenced by path unless the user explicitly asks to commit them.

## Review policy

The goal mode run may proceed automatically through S01-S11 only when each plan passes implementation review and validation. If a plan finds a blocker or requires a user product decision, it must stop.

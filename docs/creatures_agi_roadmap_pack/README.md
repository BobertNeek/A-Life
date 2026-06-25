# A-Life Creatures-to-AGI Roadmap Pack v1

Purpose: continue from the current GPU-backed graphical alpha into a playable **Creatures-inspired but smarter** artificial-life game, then into a bounded AGI research roadmap.

This pack is intended to be imported into the repository under:

`docs/creatures_agi_roadmap_pack/`

It does not replace the historical P/G/R roadmap. P01-P36, G00-G24, R24, S/productization work, and the current GPU graphical alpha remain historical baseline. This pack starts a new explicit roadmap.

## Current assumed baseline

Expected repository state at import:

- GPU graphical alpha exists.
- Current product claim is `CpuShadowGuardedStaticPlusLiveHShadow`.
- CPU fallback and CPU shadow parity remain safety/correctness mechanisms.
- `alife_core` must remain engine/GPU/UI independent.
- Current visible game still needs stronger player-facing gameplay, visual behavior, UX, ecology, school mode, content, packaging, and broader hardware/playtest evidence.

## Core principle

Build the game as GPU-first for players, but keep CPU oracle/fallback internally.

Do not remove CPU shadow parity until a later plan explicitly proves sampled/action-authoritative behavior over diverse workloads.

## How to use

1. Copy this directory into the repository as `docs/creatures_agi_roadmap_pack/`.
2. Commit the imported pack as a docs-only import.
3. Start Codex Goal Mode with `prompts/GOAL_MODE_DRIVER_PROMPT_4000.md`.
4. Codex should follow `plan_manifest.json` in order.
5. Codex must stop at review gates and user-decision gates.
6. Paste review receipts back into ChatGPT before continuing past major gates.

## Roadmap shape

- CA00-CA12: product surface, UI, assets, save/load, sandbox.
- CA13-CA22: real gameplay loop, ecology, population, tuning.
- CA23-CA31: school, semantic systems, cognition visualization.
- CA32-CA36: GPU/CPU parity, performance, soak.
- CA37-CA45: polish, packaging, external alpha/beta, release candidate.
- CA46-CA59: cognitive-school and theoretical AGI research track.

This pack deliberately separates **game shipping** from **AGI research**. The game must become playable before later cognitive claims are expanded.

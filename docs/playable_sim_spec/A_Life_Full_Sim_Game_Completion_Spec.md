# A-Life Full Sim Game Completion Spec

## Product target

A polished A-Life sim game is a graphical, inspectable, persistent Bevy/WebGPU simulation where autonomous creatures perceive a living world, act through structured arbitration, learn from sealed experience, sleep/consolidate, socialize, reproduce, respond to perception-only teaching, and can be observed and influenced by the player through safe UI tools.

## Baseline

The baseline is the completed P01-P36 scaffold/release-gate phase. It includes engine-independent core contracts, headless simulation, persistence, optional adapters, GPU diagnostics/fallback, offline tools, and P35 developer playground commands. It is not yet a finished playable game.

## Completion definition

The game phase is complete when:

- A player can launch a visible game app.
- A small world loads from versioned configs/assets.
- Creatures visibly perceive, act, eat, avoid hazards, sleep, socialize, reproduce, and die.
- The player can inspect cognition summaries without mutating core state.
- School/teacher and semantic systems are visible/product-safe and optional.
- Save/load works through user-facing slots.
- GPU acceleration is optional, honest, and falls back to CPU.
- Long-run soak and performance gates pass or record exact manual limitations.
- The default headless CPU path remains deterministic and validated.

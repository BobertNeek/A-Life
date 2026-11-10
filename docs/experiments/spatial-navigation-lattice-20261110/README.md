# Experimental spatial lattice and terrain-vision probe

This is source-only research, based on the opt-in authored experimental V2 brain. It does not read or replace acquired lives or optimizer histories. Terrain vision intentionally changes sensory semantics and does not expose an innate GPS; genetic weights are transferred to a fresh research phenotype only after verifying every synapse coordinate. This is not a migration.

The probe tests a clear target and mirrored opaque-wall detours. It asserts the food is absent from initial action candidates when hidden. The research candidate adds lateral visual-to-context-to-turn paths, an exploratory visible-object approach bias, and a 4x4 local lattice using a reciprocal predictive/context connection block. This is not yet a learned occupancy map, goal-conditioned pathfinder, or demonstrated maze solver.

A detached replay-only pulse/ablation diagnostic injects a pulse into one neuron and compares the response with the local lattice edges silenced. It does not feed information into a live creature. One smoke trial showed a small response in neighboring context cells that was nearly absent under ablation and decayed over later replay steps. Eight-decision runs are not behavioral validation. A preceding 120-decision test of the simpler trace-only edit found no food consumption in two clear and four wall scenes per arm; the combined and lattice candidates require longer comparison and safety controls.

Reproduce the smoke test from the repository root, using a fresh output directory:

```sh
ALIFE_RESEARCH_NAV_TERRAIN=1 ALIFE_RESEARCH_GRID_PULSE=1 cargo run -p alife_training --features cpu-backend --example probe_spatial_navigation -- <new-output-dir> 8 0 nav-lattice-factor 1 2026111821 maze-clear 0.4 0 0
```

Inspiration: https://arxiv.org/abs/2301.06820 and https://arxiv.org/html/2410.18832v2. Both papers use supplied spatial representations; this probe must derive sensory data from the simulated creature’s view and cannot assume a hidden complete map. CPU probes are not RTX or gameplay qualification.

## Egocentric input and care probes

A later candidate connects eight distinct observed range-ray encoder channels into four adjacent lattice input cells and increases local self-recurrence. A detached pulse/ablation smoke test showed a small, decaying local response. This is an egocentric sensory trace, not an allocated global map, occupancy mask, goal gradient, or validated maze solution. Normal mirrored-wall tests still hide the target from initial visual candidates. A stronger visible-object exploration edit ate far food in one 120-decision paired open trial where the baseline did not, while neither ate the neutral target or solved either wall in that small trial; this is not a reliability or safety claim. Contact-bias alternatives and actual hazardous food are under test. Never promote these results into a trained life or infer learning without controlled retention tests.

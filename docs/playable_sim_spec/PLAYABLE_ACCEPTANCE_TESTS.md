# Playable Acceptance Tests

These tests define what the new game phase must eventually prove.

1. Launch graphical app/manual smoke: window opens, tiny world loads, creature visible.
2. Headless fallback smoke: same config runs without graphics/GPU.
3. Spawn one creature, food, hazard, obstacle, and rest/sleep affordance.
4. Select creature and inspect drives, hormones, action, memory expectancy, topology, sleep state, and last sealed patch.
5. Eating food reduces hunger or improves energy/reward.
6. Hazard produces pain/negative valence and future danger bias without replaying actions.
7. Sleep/rest is visible, logged, and does not mutate genetic baseline.
8. Save/load restores a visible world through stable IDs, not engine-local IDs.
9. Teacher cue enters through perception only and cannot select actions directly.
10. Semantic provider missing is tolerated; fake/provider context is optional.
11. GPU backend falls back to CPU when unavailable and never requires active neural readback.
12. Multi-creature run remains deterministic and bounded.
13. Reproduction produces valid genomes and lineage records.
14. Long-run soak has no NaN, invalid IDs, unsealed learning, or unbounded memory/topology growth.
15. Product docs and commands match actual executable paths.

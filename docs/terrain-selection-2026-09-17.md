# World-owned terrain — 2026-09-17

Highlands remains the default new-game asset. Movement, editor placement,
spawning, visual grounding, picking and camera bounds query the world's selected
heightfield. The existing motor calculation and blocked-action feedback remain
authoritative; terrain queries do not choose destinations or navigate for the brain.

`TerrainData` is a serializable heightfield (dimensions, origin, spacing, samples,
solid obstacle bounds and optional water level). Load it through
`WorldTerrain::new(data, LocomotionLimits::default())`, then call
`HeadlessWorld::enable_terrain_for_new_game(terrain, origin)` before advancing the
world. This converts a flat scenario's X/Y placement to terrain X/Z. Failed initial
placement leaves the world untouched. Cloned worlds share immutable surface arrays.

`LocomotionLimits` supplies body clearance, maximum rise/run and wading depth.
These shared body defaults are explicit and saved; this change introduces no
genetics subsystem. Water height belongs to the map. Smaller body radii use finer
movement sweep samples; collision indexing queries the actual clearance radius.

Custom saves embed terrain data and limits. Their binding hashes the complete
terrain payload, including water and obstacles, and reload checks the data against
that binding. Legacy Highlands binding-only saves use an explicit compatibility
loader. Missing custom data or mismatched identities are rejected. Flat legacy
saves remain flat. Headless signature schema 6 also includes movement limits;
old causal-signature receipts must not be treated as fresh schema-6 evidence.

The renderer shares the selected surface for grounding and picking. Custom maps
render the same heightfield diagonal in bounded 32-cell meshes, with water and
solid obstacle bounds. Highlands alone selects its existing authored GLBs and LODs.
This is a heightfield implementation: caves, bridges with traversable undersides,
and stacked support surfaces remain outside this representation.

Architecture: AOA-WORLD-001/003 retain geometric observables and measured action
outcomes; AOA-BODY-001 keeps physical movement limits outside the cognitive policy.
No neural state, learning rules, or GPU execution paths change.

Focused verification covers different maps outside Highlands' bounds, identical
motor commands, terrain-dependent heights/blocking, water and body limits,
non-Highlands save/reload movement, invalid bindings, shared arrays, and matching
rendered/physical triangles. Existing flat-world and Highlands tests are retained.

Results: 84 world-library, signature and save/load tests passed. The Bevy/GPU
app and its tests compile; boundary checks and all 79 documentation assertions
pass. The two new renderer tests were compiled but not executed: their full
graphics test-binary build was stopped to keep this repair bounded. No visual
playtest or navigation-learning claim is made.

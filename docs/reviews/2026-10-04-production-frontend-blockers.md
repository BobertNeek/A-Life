# Production frontend admission and test repairs — 2026-10-04–05

This report covers PR #10 on the combined PRs #3–#10 candidate. Its source
baseline is main `482948ec116bad505cee0225cad74c1ab110acb7`. The coordinated
candidate retains all eight reviewed heads, including the separate GeneForge
retirement. The repair changes presentation and its tests; it does not change
organism genomes, acquired state, neural execution, persistence publication,
terrain height/collision, or exported art bytes. Unsupported procedural camera
streaming is retired; static unbound scenes retain an explicit diagnostic role.

## Composite founder admission

Current curated founders archive the complete content-addressed `CreatureGenome`,
its expressed source brain genome, and the phenotype produced by
`N512FounderFoundationProjection`. Enter now loads and expresses that composite,
requires that its brain genome equals the archived source, and reconstructs the
same foundation projection used by curated staging and live runtime admission.
A generic compiler cannot reproduce that phenotype from the brain genome alone.

Exact foundation bytes and identity, phenotype hash, persistent address map,
language, sensor, and live-agent binding checks remain enforced. Missing composite
payloads fail explicitly. Negative tests reject a changed source brain genome,
a changed manifest phenotype hash, missing or ambiguous live agents, sensor
mismatches, and an incompatible N2048 foundation. The test archive uses actual
`prepare_composite_birth_batch` / `commit_composite_birth_batch` records.

This follows `AOA-GEN-008` and `AOA-PRIOR-008`; it introduces no foundation format,
compiler bypass, or substitute scaffold. Sources:
[lineage admission](../../crates/alife_game_app/src/production_conversation_lineage_ui.rs),
[curated staging](../../crates/alife_game_app/src/curated_founder_staging.rs), and
[live admission](../../crates/alife_game_app/src/gpu_live_runtime.rs).

The existing reset recovery test now expects `retryable=false` after a durable
publication whose refresh failed. The runtime clears the retained operation and
requires explicit manual recovery; the authority test still rejects retry with
`NoRetainedOperation`. Hand-contact enumeration fixes strict Clippy without
changing contact values, poses, or the approved hand asset.

## Current saves and CPU scene failures

Renderer launch sites now retain `CurrentProductionLaunchFixture`, which writes
and validates 30 current untrained founders. Tests no longer launch from a
historical save missing `genetic_biochemical_graph`. No legacy migration or
production validation relaxation is introduced.

The earlier CPU scene run reached 36 cases: 16 passed and 20 failed. These were
actual fixture/contract defects, not evidence that projection or inspectors
require a GPU: their read-only ECS readers are registered in the CPU scene mode.
The 20 failures were traced as follows:

| Original failing cases | Count | Actual cause and final resolution |
| --- | ---: | --- |
| Modular hierarchy; GeneForge children/head; cute-biped profile; low-poly modular rig; sixteen separate species meshes; authored separate head; seven body parts | 8 | Retired generated-part expectations conflict with the approved shared Hearthling skin. Tests now check 30 unique stable roots, saved species/body-plan identity, inherited proportions, absence of retired parts, and the shipping skin/material/animation contract. |
| Corrupt derived-runtime rebuild | 1 | `runtime_save_path` is the authoritative resume/manual-save/autosave checkpoint. Rebuilding corrupt authority would weaken persistence. The test instead requires exact schema rejection and unchanged corrupt and source bytes. |
| Both live inspectors | 2 | The test never opened the debug inspector; refresh correctly returns while its panel is closed. Tests explicitly open it and retain live tick/position and identity-mismatch assertions. |
| Both live world projection cases | 2 | Production still swapped Y and Z for unbound terrain, although current world objects are always Y-up. The renderer now preserves authoritative X/Z continuously, including startup, live roots, newborn tile lookup, care objects and hand ground coordinates. Identity mismatch and non-agent rejection remain checked. |
| Selection marker | 1 | The retired tile ring is hidden by current selection highlighting. Test checks the selected stable ID's current root position and hidden ring. |
| Camera follow | 1 | Obsolete absolute follow matrix. Test checks continuous root/camera displacement, unchanged framing and focus on the same selected root. |
| World label | 1 | Obsolete absolute height. Test checks the current root-relative label height and matching stable ID. |
| Compact player HUD | 1 | Obsolete debug headings, recovery-key text and assumed energy. Tests check current compact controls, actual needs, explicit unavailable energy, and no invented learning/social readings. |
| Profile lighting | 1 | Obsolete 6,000-lux limit; current configured sun is 8,500 lux. Profile shadow, grounding, contact and capability budgets remain checked. |
| Terrain/world colors | 1 | White material tint was mistaken for terrain albedo. Tests read actual mesh vertex colors, lit white tint, cool ambient and warm sun. |
| Terrain dressing | 1 | The procedural camera stream loses anchors, but has no current supported island role: New Game enables the authored island and `SelectedTerrain` bypasses this system. Retire the stream, its cache and the stale procedural dressing test; add no replacement planner machinery. |
| **Total** | **20** | **17 stale fixture/contract cases; two coordinate cases; one obsolete procedural-path case retired under island-only scope.** |

Current world coordinate evidence is in
[headless movement and bearing](../../crates/alife_world/src/headless.rs).
The repair preserves the existing grounding-height calculation and terrain
backend. Terrain binding selects the height source, not a coordinate convention.
The supported New Game path calls `enable_island_for_new_game`; the authored
island's `SelectedTerrain` bypasses the retired camera stream. Its Blender prop
exclusion, authored chunk culling and placements are unchanged. Sources:
[New Game](../../crates/alife_game_app/src/new_game_lifecycle.rs),
[island presentation](../../crates/alife_game_app/src/production_voxel_renderer/island.rs),
and [Blender landscape exclusion](../../crates/alife_game_app/src/production_voxel_renderer/landscape.rs).

The procedural streaming module, its retained backend cache, and obsolete
procedural dressing scene assertion are removed. The existing current-format unbound fixture is explicitly described as
isolated CPU/ECS and causal diagnostics. Its static terrain checks still cover
meshing, layer budgets, vertex colors, stable identity and save correctness;
these are not supported island scenery or graphical acceptance.

A care-object fixture also expected zero elevation after its position changed
to Y-up height 4. Its corrected assertion preserves both terrain-bound absolute
height and the unbound surface offset, movement, consumption and unchanged
authoritative tick. The affected case passes.

Hearthling checks inspect the shipping GLB JSON bytes: one skin, matching inverse
bind/joint counts, six primitives/materials, authored head/face/feet/tail joints,
position/normal/UV/color/joint/weight attributes, and the three existing clips.
They are static CPU asset checks, not proof of scene loading or rendering.

The presentation changes follow `AOA-INV-003`, `AOA-INV-011` and `AOA-OBS-004`:
views remain derived from authoritative identity, positions and ticks. The
checkpoint test preserves `AOA-INV-010` explicit failure. These checks do not
certify all player-facing capabilities in `AOA-OBS-001`.

## Validation and limits

| Check | Result |
| --- | --- |
| CPU scene target, `bevy-app,production-assets,vfx-hanabi` | 35 passed, zero failed, zero ignored; retired procedural assertions are removed |
| Production-feature exact corrupt-checkpoint rejection | One passed; rejects before backend creation and retains exact corrupt/source bytes |
| Production-feature library coverage before procedural retirement | Full run: 336 passed, one care-height fixture failed, two existing ignored CPU benchmarks. Corrected care-height test: one passed; all 337 active cases covered successfully. |
| Final relevant production renderer library tests | 47 passed, zero failed, zero ignored |
| Strict app/training production and GPU-test all-target Clippy | Passed with `-D warnings` |
| Formatting, patch whitespace and docs gate | Passed; documentation assertions 77/77 |
| Combined production manifest | All 48 paths, sizes and FNV digests valid; 16,112,065 bytes |

Manifest SHA-256:
`9e3512abe67c25386d10d0120b70e284858911c572d331f5687773ad62ba11e8`.

```sh
cargo test -p alife_game_app --features bevy-app,production-assets,vfx-hanabi --test fvr03_voxel_renderer
cargo test -p alife_game_app --features production-voxel-frontend --test fvr03_voxel_renderer runtime_checkpoint_corruption_is_explicit_and_never_overwritten
cargo test -p alife_game_app --features production-voxel-frontend --lib production_voxel_renderer::
cargo clippy -p alife_game_app -p alife_training --features alife_game_app/production-voxel-frontend,alife_game_app/gpu-tests,alife_training/gpu-tests --all-targets -- -D warnings
```

The earlier production-feature three-target rerun reported 35
`NeuralBackendUnavailable` failures. Those runs are **Blocked** environment
checks, not accepted gameplay. They were not repeated to obtain the same unavailable
backend failures. The focused corrupt-checkpoint test must reject before backend
creation. CPU scene mode has no neural fallback and performs no graphics work.

Physical GPU gameplay, graphical asset loading, animation appearance, save/resume
with a live GPU, and GPU reset acceptance remain **Blocked/Unrun**. No training,
physical GPU workloads, user-PC activity or new asset download was performed.
The requested refined Blender ZIP integration remains blocked by the previously
reported Library download DNS failure; this follow-up changes no art bytes.
PRs #3, #6 and #10 remain held for owner review and are not merged by this task.

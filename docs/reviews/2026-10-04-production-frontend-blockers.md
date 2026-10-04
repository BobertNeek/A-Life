# Production frontend admission and test repairs — 2026-10-04

The production-feature validation of PRs #3–#7 exposed two library failures and
one strict Clippy failure. Their relevant source functions are byte-identical to
main `482948ec116bad505cee0225cad74c1ab110acb7`; these defects predate the combined
candidate `7bc05bd49b1121bf4f3f07251e8acfedd9af0d3f`. Cassidy authorized fixing
discovered issues. This repair is separate from all five candidate changes.

## Selected resolution

The lineage Enter path requires the exact checked builtin Nano512 foundation.
Its generic canonical compiler call rejected that asset's explicitly identified
ABI before a valid archived founder could enter. The path now uses the existing
live-runtime construction helper after checking the asset's exact bytes and
identity. That helper performs the established builtin admission and normalizes
construction maturation to 1.0 while preserving world developmental state. The
fixture uses the builtin graph's fixed coordinate seed. The N2048 mismatch case
still uses its existing runtime construction route and remains rejected by the
Nano512-only Enter contract. Exact genome, phenotype, foundation, address-map,
language, sensor and live-agent binding checks remain enforced.

This changes no compiler validation or foundation format. It follows
`AOA-GEN-008` and `AOA-PRIOR-008`: the checked content-addressed asset must match,
and incompatible genome/asset combinations still fail explicitly. Sources:
[lineage admission](../../crates/alife_game_app/src/production_conversation_lineage_ui.rs),
[live construction](../../crates/alife_game_app/src/gpu_live_runtime.rs), and
[immutable Nano512 ABI tests](../../crates/alife_core/tests/n512_legacy_compatibility_abi.rs).

The renderer reset test formerly expected a retained-operation retry after a
successful durable publication whose refresh failed. The runtime deliberately
clears that operation, projects Unknown and requires manual recovery with
`retryable=false`. The assertion now matches that existing behavior, consistent
with `AOA-INV-010`. The authority test already verifies that a retry receives
`NoRetainedOperation`; production recovery behavior is unchanged.

The hand pose test enumerates the existing contact array rather than indexing it
with a range. The poses, transforms, contact points and approved hand assets are
unchanged.

## Validation

Validated on the combined candidate plus this repair:

| Check | Result |
| --- | --- |
| Production-feature library tests | 334 passed, 0 failed, 2 existing ignored benchmarks |
| Production-feature all targets, `--no-fail-fast` | All 30 targets executed; 398 passed, 36 failed, 2 ignored |
| Strict production-feature all-target Clippy | Passed with `-D warnings` |
| Focused lineage Enter/archive mapping | Passed, including live binding, sensor and foundation rejection cases |
| Formatting and patch whitespace | Passed |

Before the renderer fixture follow-up below, the complete app command exited
101 with three integration targets failing.
`canonical_new_game_lifecycle` and `phase3_capture_presentation` each have one
failure reporting `NeuralBackendUnavailable`. `fvr03_voxel_renderer` has 34
failures before its renderer assertions because the shipped fixture lacks the
authoritative `genetic_biochemical_graph`; three tests in that target pass.
At that point, those three test files and the fixture were byte-identical to main. They remain
separate inherited blockers; this result does not establish graphical or
production integration acceptance. The old renderer expectations, including
GeneForge expectations, were not rewritten during this focused repair.

The combined candidate's preceding default full workspace validation passed
1,670 tests with zero failures and 20 existing ignored tests. The feature repair
was then checked with:

```sh
cargo test -p alife_game_app --features production-voxel-frontend --all-targets --no-fail-fast
cargo clippy -p alife_game_app --features production-voxel-frontend --all-targets -- -D warnings
cargo fmt --all --check
git diff --check
```

No training, user-PC activity, art transfer, graphical acceptance run or live
neural execution was performed. GPU-dependent acceptance remains unverified in
this executor. The repair and all candidate PRs remain unmerged.

## Follow-up: current renderer fixture

The 34 shared renderer launch sites now use the existing
`CurrentProductionLaunchFixture`, which creates and validates a fresh file-backed
population of 30 current untrained founders. Each test retains the fixture owner
until its assertions finish. The historical source save is no longer their
launch input. No migration or production validation rule changes are made.
All 323 original assertion macros are unchanged. The one test calling a GPU
runtime method directly now declares its `gpu-runtime` feature prerequisite;
it remains enabled in the production-feature run and is not ignored.

The targeted production rerun executes all three previously failing targets:

| Target | Result | Remaining failure class |
| --- | --- | --- |
| `canonical_new_game_lifecycle` | 1 passed, 1 failed | `NeuralBackendUnavailable`: environment |
| `phase3_capture_presentation` | 0 passed, 1 failed | `NeuralBackendUnavailable`: environment |
| `fvr03_voxel_renderer` | 3 passed, 34 failed | 33 unavailable GPU backends; 1 corrupt-derived-save contract failure |

There are zero `genetic_biochemical_graph` errors. The separate derived-save
test deliberately writes `stale-derived-runtime`. The durable publisher loads
the existing public manifest before publishing and rejects it with
`Schema { expected: "alife.p34.save_file.v1", actual: "stale-derived-runtime" }`.
That mismatch between the test's rebuild expectation and strict publication is
a code/test contract failure, not an environment limitation. Its assertion and
the publisher's explicit rejection remain intact, consistent with `AOA-INV-010`.

A CPU scene run with `bevy-app,production-assets,vfx-hanabi` and no GPU runtime
executes 36 cases: 16 pass, 20 fail, zero are ignored. It uses MinimalPlugins and
performs no graphical or neural execution. This mode reaches the assertions
below, but cannot establish behavior of the absent GPU-owned live projection
systems. The direct GPU-action case is outside this feature configuration.

| Failing CPU scene case | Exact failure or mismatched value |
| --- | --- |
| `dry_run_rebuilds_an_incompatible_derived_runtime_save_before_gpu_launch` | Save schema is `stale-derived-runtime` |
| `fvr03_geneforge_children_preserve_real_asset_groups_and_one_coat_handle` | `!parts.is_empty()` |
| `fvr03_geneforge_head_owns_face_and_renderer_is_display_only` | `!head_bounds.is_empty()` |
| `fvr04_camera_follow_recomputes_from_the_moved_projected_creature_root` | Camera transform tolerance `< 1.0e-5` |
| `fvr04_live_creature_inspectors_reject_stable_id_mismatch` | Inspector lacks explicit stable-ID mismatch |
| `fvr04_live_creature_inspectors_report_current_authoritative_position_and_tick` | Inspector lacks live tick |
| `fvr04_live_world_label_tracks_projected_root_by_stable_id` | Height `2.03`, expected `2.35` |
| `fvr04_live_world_projection_ignores_unmatched_and_non_agent_objects` | Yaw `0.0 == 0.0` where change is required |
| `fvr04_live_world_projection_moves_matching_creature_and_creates_newborn_by_stable_id` | Stable creature 4 stays at `(-3.000,0.480,0.000)`, expected `(-2.500,0.480,1.500)` |
| `fvr04_selection_marker_follows_the_moved_projected_creature_root` | `(-4.5,0.5)`, expected `(-2.5,1.5)` |
| `fvr09_creatures_are_cute_bipedal_real_state_visuals` | `approved-hearthling-skinned-v2`, expected `modular-heritable-part-assembly-v1` |
| `fvr10_creature_mesh_is_readable_low_poly_rig_not_cuboid_stack` | Same approved Hearthling versus modular assembly mismatch |
| `fvr10_creatures_use_all_selected_bipedal_caveman_species_not_color_swaps` | `approved-blender-vertex-color-v2`, expected `modular-textured-part-material-v1` |
| `fvr10_product_camera_and_authored_heads_are_composed_for_readable_creatures` | No legacy `CreaturePartSlot::Head` marker |
| `fvr10_scene_dressing_uses_composite_vertical_props_not_unit_debug_cubes` | Hero prop count `0` |
| `fvr11_profile_lighting_preserves_minimum_floor_and_comfort_depth` | Minimum sun illuminance exceeds `6_000.0` |
| `fvr12_creatures_render_only_the_seven_authored_body_parts` | Legacy part count `0`, expected `210` |
| `modular_creature_renderer_spawns_shared_heritable_part_hierarchies` | Legacy part hierarchy count `0`, expected `30` |
| `v0_default_player_view_is_compact_and_uses_real_selected_creature_state` | Card reports `Energy unavailable` |
| `v0_render_world_direction_is_warm_readable_and_creature_led` | Grass green-channel ratio assertion fails |

The explicit old modular/GeneForge expectations conflict with the approved
Hearthling renderer metadata. The lighting, dressing and color failures are
non-environment assertion mismatches in this CPU mode. The live projection/HUD
failures require checking the corresponding runtime-enabled systems; they are
not proven production defects by a mode that omits those systems. No failing
assertion was weakened or blanket-ignored to claim acceptance.

Strict production/GPU-test all-target Clippy passes after the fixture repair.

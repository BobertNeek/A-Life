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

The complete app command exited 101 with three integration targets failing.
`canonical_new_game_lifecycle` and `phase3_capture_presentation` each have one
failure reporting `NeuralBackendUnavailable`. `fvr03_voxel_renderer` has 34
failures before its renderer assertions because the shipped fixture lacks the
authoritative `genetic_biochemical_graph`; three tests in that target pass.
These three test files and the fixture are byte-identical to main. They remain
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

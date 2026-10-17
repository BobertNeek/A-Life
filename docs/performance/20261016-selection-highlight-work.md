# Selection highlight work removal — 2026-10-16

## Scope and source

This is a Bevy presentation-only optimization of `god_hand::highlight`.
The reviewed baseline is main `624bb5b355aa93bfd2c986351e556810b5b1dd25`,
tree `a19f721e0f9510c5c935d5cae550a50f726989fe`. The highlight source is
blob `b162cff20aeed53b96e835058e2cb71a9164f6fa`, unchanged from the earlier
main `03a9e249` used by the frozen test oracle.

No world state, cognition, neural execution, simulation cadence, animation,
selection policy, art asset, render quality, persistence or GPU ABI changes.

## Removed work

The previous pass collected the selected hierarchy into a vector, scanned every
material-bearing entity, and ran a linear hierarchy-membership test for each.
It rewrote the same highlighted or original handle on every surviving mesh
previously selected, even when the handle was already correct.

The Island source contains 3,716 prop placements and 120 active terrain chunks,
plus its water entity. All 3,837 unrelated material entities were included in
that per-render-frame scan. The approved Hearthling has 36 authored glTF nodes
and six material primitives. Including the assembly root, scene wrapper and
Bevy glTF world root gives a source-derived 45-entity selected hierarchy:
172,665 failed entity comparisons per frame for the unrelated Island art alone.
These are source-derived operation counts, not a live frame-time measurement.

The new pass resolves the same target and freshly traverses its hierarchy each
Update. It uses a membership set, directly queries selected entities, and visits
only persistent `SelectedTint` entities for restoration. It compares handles
before assigning. The work now scales with the selected hierarchy and surviving
historical tint count, independently of untouched Island props and chunks.

This also avoids redundant Bevy component changes. In pinned Bevy PBR 0.18.1,
`mark_meshes_as_changed_if_their_materials_changed` uses changed material
components to mark meshes changed, and `extract_mesh_materials` re-extracts
changed material instances. No GPU-byte or timing improvement is claimed here.

## Preserved behavior

- Creature-root lookup still takes priority over care-object fallback
- The selected root itself and all current descendants are considered
- Fresh traversal covers late glTF meshes, material components added later,
  reparenting and replacement roots for an unchanged selected stable ID
- First selection clones each loaded material with the existing emissive tint
- `SelectedTint` remains a persistent cache after deselection. External handle
  replacements still yield to its original/highlighted handles, as before
- Missing source material assets are retried on a later Update without caching
- Despawned entities and removed material components are safely skipped
- The old selection marker remains hidden

## Verification and limits

Two independent source reviews found no correctness blocker. Formatting,
whitespace, static core-boundary and documentation checks are recorded with the
qualification result. The shared `Island output-change frontend CPU regression`
job executes the new frozen-baseline parity/change-detection tests and neighboring
pointer tests with `production-voxel-frontend`, without launching a renderer or GPU.

At draft publication, the isolated local frontend build reached final linking
but was interrupted under memory pressure; the new Rust tests had **not executed
locally**. The first attempt reused a stale
October 13 core artifact because its dep-info pointed to a different checkout.
The fresh attempt recompiles local crates into a separate target; only immutable
third-party libraries are reused, with a compiler guard rejecting dependency
rebuilds. Hosted exact-head execution is required before merge.

Publication resumed against main `73f8ecf03a4bbd90018c0fae7c732c6ec488e1d9`
(tree `d5507cba2f7222217560d7250c8b6a7b5a9d3eda`). The reviewed Rust
files are unchanged. The integration retains the existing Island and camera CPU
suites and appends the highlight suite to their shared frontend job, reusing the
same compiled test executable instead of adding another large compile.

The fixtures cover 3,716 unrelated materials, unchanged selection, switching and
deselection, late descendants, reparenting, replacement roots/despawns, missing
material retry, target priority, asset edits, removed material components and
external material replacement. They compare
rendered material values and persistent tint caches with a frozen copy of the
old pass and observe changed material components after warm-up.

Game FPS/TPS, physical GPU traffic, visual acceptance and the 50-creature target
remain **Unknown**. No PC access, GPU run or training campaign was performed.

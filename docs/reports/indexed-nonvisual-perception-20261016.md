# Indexed nonvisual perception, 2026-10-16

> Historical original-patch evidence only. This report describes quarantined
> patch `25e554263e578535afa4ff2e7de73c659eb35e065f68b9444688ee6bf4cb6fba`.
> Its test, Clippy, rustfmt and work-count results do not qualify the separately
> rebuilt density fallback added on 2026-10-17. The original also regressed dense
> CPU totals 4.34x–8.05x in independent review. See
> `indexed-nonvisual-density-rebuilt-20261017.md` for the rebuilt source's current
> evidence and open qualification gates. No old densityfix bytes were recovered.

## Scope and provenance

Narrow optimization of GroundedTerrainVisionV1 smell/contact candidate enumeration
against source commit `73f8ecf03a4bbd90018c0fae7c732c6ec488e1d9`.
The existing batch index is extended during its existing object pass. No optical,
ray/sphere, candidate-ranking, neural, GPU, training, persistence-schema, or
carried-pose behavior is changed.

## Mechanism and equivalence obligations

- Store the maximum unconsumed contact radius and ascending owner/carried-ID
  lists in the immutable perception batch, with no cross-tick cache.
- Query the union of the observer's contact cube and both actual f32-rounded
  lateral odor sample cubes. Radius-8 odor samples have the required 8.25-unit
  extent from the observer at ordinary positions.
- Compute outward-expanded cell bounds in f64. Eight f32 epsilons cover
  distance-operation rounding; effective radii are at least 0.75. Saturating
  cell conversion can overinclude candidates. It must never exclude support.
- Check cell volume before traversal. Fall back to the canonical iterator above
  4,096 cells, when cell probes outnumber world objects, on volume overflow,
  nonfinite sample centers, or unconsumed nonfinite radius/odor chemistry.
  Nonfinite remote chemistry cannot be omitted because infinity/NaN times zero
  can change the canonical result.
- Union the observer's carried IDs independently of distance, then sort ascending
  entity ID and deduplicate. The canonical smell accumulation order is retained.
- Run the original nonvisual arithmetic on those references. Four odor bands,
  both sides, contact/temperature, possession, ground, care, taste, pain, sensor
  gains, and clamping remain in the same kernel. Self/consumed filtering remains
  in that kernel.

The index still has its existing same-snapshot lifetime contract. Seed/tick/count
checks do not detect same-tick movement, radius, or ownership changes. Rebuild
following mutation; this change introduces no persistent invalidation scheme.

## Focused CPU verification

The exact core/world sources and embedded foundation bytes were tested in an
isolated two-crate workspace, using the pinned Cargo.lock dependency versions,
offline cached dependencies, one build job, no incremental output or debug
symbols, and test package opt-level 0. The workspace narrowing and test profile
are verification-only and are not part of this patch.

Final checks:

- 11 new indexed/canonical unit tests passed, including all sensory float bits,
  complete draft equality, candidate/slot equality, and serialized packet equality
- Existing perception_candidates integration suite: 20 passed; the batch equality
  profile list now includes GroundedTerrainVisionV1
- Existing carried_food_terrain_vision integration suite: 5 passed unchanged
- Existing terrain_vision_tests unit group: 9 passed unchanged
- Existing carry_pose_tests unit group: 7 passed unchanged
- Focused Clippy with `-D warnings` and rustfmt passed

Coverage includes lateral cell-two odor on both sides; rear/occluded odor and
contact; positive/negative surface temperatures; far oversized contact; far owned
items and duplicate membership; self/consumed objects; mixed-sign four-band sums
in entity order; fresh indexes after movement/radius/ownership changes; huge
radius/small-world fallbacks; nonfinite chemistry/radius/sample cases; neighboring
float values at signed cell boundaries; saturated extreme coordinates.

An initial optimized test attempt did not complete. Final successful checks used
the bounded unoptimized CPU profile above. These are focused checks, not full
workspace qualification. No GPU, PC, training, full build, package installation,
publication, or live-frame-rate acceptance was performed.

## Work-count evidence

The test-only counter increments once in the actual nonvisual object loop. Each
such evaluation executes two odor distances and one contact distance. With one
local source, one far-owned source, and increasing remote objects:

| Remote objects | Canonical objects | Indexed objects | Canonical distances | Indexed distances |
| ---: | ---: | ---: | ---: | ---: |
| 128 | 130 | 2 | 390 | 6 |
| 512 | 514 | 2 | 1,542 | 6 |
| 2,048 | 2,050 | 2 | 6,150 | 6 |

These are exact kernel work counts, not timing or FPS measurements. The index
build adds scalar/ownership bookkeeping; cell lookups, ID sorting, and object
lookups also cost work. Dense or oversized scenes can retain the full scan, so
no universal wall-clock speedup is claimed.

## Architecture trace

- AOA-WORLD-001, AOA-WORLD-002, AOA-SENSE-001, AOA-SENSE-002: preserve grounded
  physical channels and unscored transport/candidates; no semantic filter
- AOA-TIME-001: retain canonical entity order and exact sensory float results
- AOA-AUTH-002: metadata stays batch-owned and read-only under the existing
  authoritative same-snapshot contract
- AOA-INV-005: bound cell traversal explicitly and use conservative fallback

This report records only the narrow optimization evidence. It does not certify
full compliance with these requirements or revise the controlling architecture.

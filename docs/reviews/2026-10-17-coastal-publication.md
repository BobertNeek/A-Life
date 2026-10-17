# CoastalShrub publication on integrated main, 2026-10-17

This asset-only successor is based on main
`28770f3b7f2692be02c0aa686c554309bc4703b9` (tree
`fd4eb4767f16245ce8781813a1c2945703f25f12`), which already contains Fern #72.
It publishes the exact previously reviewed CoastalShrub folded-leaf runtime,
editable Blender source and baseline, with the corrected offline preview
renderers and the combined source/art gate. No Fern runtime/native file,
placement, runtime Rust code, shader or organism behavior changes.

## Preservation and current integration

The corrected 41-path package is retained with its historical profiles, reports,
receipts and images unchanged. All 140 art/world files and 48 manifest entries
pass the combined audit. The only new production-manifest change relative to
current main is CoastalShrub digest, size and generator. All 170 shrub placements,
Fern and Broadleaf geometry, other art, metadata and world data are retained.

The small `scripts/fern-coastal-main-integration-profile.json` records the exact
historical/current CI source-pin transition. The historical corrected profile
is byte-retained; only its CI source pin is superseded in memory after fixed
profile validation. The checker also validates the separately pinned integration
tests. The existing `broadleaf-canopy` job ID/name stays; only the plant audit/test
commands change, retaining every unrelated current-main workflow step.

## Fresh qualification

- Combined stdlib positive/negative suite: 74/74
- Authored-preview rotation guards: 4/4; current-main integration guards: 3/3
- Shipping island audit and boulder placement regressions: 6/6
- Documentation assertions: 77/77
- Blender 4.3.2 native reload/export: exact source/runtime identity, all 108
  blade roots at original attachments, unchanged L2 and BerryBush
- Both current exporter orders reproduce the exact Fern/Coastal runtimes after
  historical override exports; all audited files and native sources retained
- Corrected authored-yaw matrices: 24/24, with 24 historical negative controls;
  maximum matrix error 5.96e-8, no rendering

Fresh source-bound receipts are in the adjacent dated directory. L0/L1 shrub
triangles are 672/204, down from 1152/506; L2 remains 218. Runtime size is 38,552
bytes, down from 44,788. These measurements establish no FPS improvement.

## Visual and native evidence limits

The historical standalone close/reverse L0 images were rendered from the exact
published runtime SHA256 `7a2636250308b67ba4794914b8edc0b1738646233a249c78a5f42573944f2a18`,
with matching cameras/materials and Cycles CPU 24 samples. Their close views
remain valid geometry evidence. Historical authored-patch panels have the known
inactive-yaw defect and are retained only as historical evidence; they are not
proof of corrected placements or of the combined Fern/shrub scene. The renderer
fix is verified through the fresh no-render matrix checks.

No new images, corrected combined visual acceptance, live gameplay, dynamic LOD
transition, physical-GPU/FPS result or whole-workspace build is claimed. Exact
published-head CI must be checked separately after publication.

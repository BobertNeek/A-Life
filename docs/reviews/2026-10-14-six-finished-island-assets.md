# Six finished island assets: source-aware reconciliation

The six saved Oct 9 refinements are retained as later intentional work. Current
repository paths still held their Oct 1 originals, with no subsequent six-asset
replacement. This is not a choice between competing newer versions.

The Oct 4 corrected source pack differs from those installed bytes. Both exact
baselines are preserved under `finished-island-refinements/`; `lineage.json`
records archive identity, SHA-256, FNV-1a64, stable asset IDs, native sources and
all 18 LOD contracts. The small runtime bundle is not treated as a blind install.

- Flowers: retain broader petals, orchid/ivory separation and amber centres
- Grass: retain fuller folded blades, darker bases and pale growing tips
- BoulderA: retain tall basalt shelves, fractures and directional moss
- BoulderB: retain medium, leaning basalt silhouette and matching mineral palette
- BoulderC: retain squat layered basalt silhouette and matching mineral palette
- Cliff: retain sea-gray columns; its Oct 9 pass changes only color bytes versus
  the corrected source-pack baseline

Every installed AABB, pivot, dimension, node/mesh name, shared material and LOD
triangle budget is retained. Flowers use 848/373/155 triangles, Grass 288/128/57,
each boulder 312/140/62 and Cliff 728/326/144. Vertex counts and positions are
not claimed identical to the older installed assets.

The six editable Blender 4.3.2 sources were fetched from the preserved authoring
archives. Their unchanged native exports, passed through a portable guarded
layout adapter, freshly reproduce all six delivered GLBs byte-for-byte. The
adapter never reapplies the sculpt/paint operation and preserves the source-pack
JSON/index layout. It transfers positions, normals and colors for the five
geometry refinements, and colors only for Cliff.

Only six existing manifest records change digest, size and generator provenance.
All other fields/records, including approved Conifer, remain exact. The original
four/five-asset historical profiles are retained unchanged. A separate fixed
Conifer-plus-six successor checks the complete current art tree and all 48
registrations, plus existing structural checks for shrubs, Fern, Sapling and
Conifer. It is not a broader permission to change unreviewed art.

A fresh paired CPU-only neutral-lighting comparison of actual LOD0 GLBs confirms
the intended silhouettes and palette. This is an offline inspection, not game
lighting, runtime LOD-transition, GPU/performance or whole-workspace acceptance.

# Licensed Creature Mesh Sources

This directory contains the whole-animal OBJ sources used only by the
deterministic `alife_tools::creature_part_builder` pipeline. Runtime packaging
must not include these files. Production uses generated named-part OBJ packs
under `production_voxel_v1/creature_parts/generated/` plus the retained source
textures.

The meshes are derived from **Quirky Series - FREE Animals** by **Omabuarts
Studio** under Creative Commons Attribution 4.0 International (`CC-BY-4.0`).

- Source: https://sketchfab.com/3d-models/quirky-series-free-animals-pack-19e91ef86cd0448f9cbb5d6c538dade2
- Creator: https://www.omabuarts.com/product/quirky-series-free-animals/
- License: https://creativecommons.org/licenses/by/4.0/legalcode

Family IDs, source paths, canonical transforms, cut volumes, sockets, and
generated output paths are append-only catalog data in
`production_voxel_v1/creature_parts/catalog.json`. New meshes are compatible
with the pipeline by adding a new stable family entry and cut profile; renderer
code must not gain a family-specific branch.

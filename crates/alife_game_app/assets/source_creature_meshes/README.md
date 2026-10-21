# Licensed Creature Mesh Sources

This directory contains the whole-animal OBJ sources used only by the
offline `alife_tools::creature_part_builder` pipeline. The current game ships the
Hearthling GLB; these donor meshes and optional generated part packs are authoring
inputs, outside the runtime package.

The meshes are derived from **Quirky Series - FREE Animals** by **Omabuarts
Studio** under Creative Commons Attribution 4.0 International (`CC-BY-4.0`).

- Source: https://sketchfab.com/3d-models/quirky-series-free-animals-pack-19e91ef86cd0448f9cbb5d6c538dade2
- Creator: https://www.omabuarts.com/product/quirky-series-free-animals/
- License: https://creativecommons.org/licenses/by/4.0/legalcode

Family IDs, source paths, transforms, cut volumes, sockets, and output paths are
recorded in `production_voxel_v1/creature_parts/catalog.json`. Family IDs also
appear in saved appearances; preserve those identities when adding mesh sources,
or migrate the saved identities explicitly.

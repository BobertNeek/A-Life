# Creature Source And Production Asset Attribution

The production creature roster is generated from the GeneForge Norn, Ettin,
and Grendel assets by Eem Foo.

- Source: https://eem.foo/geneforge/
- Permission record: `GENEFORGE_LICENSE_RECEIPT.md`
- License asserted by the A-Life project owner: MIT
- Blender version: 5.1.0
- Importer: `alife.geneforge_importer.v2`
- Recipe and source SHA-256 values:
  `../creature_parts/geneforge_recipes.json`

A-Life selects named neutral adult geometry, separates reusable head, torso,
arm, leg, and tail groups, prepares canonical and cross-torso sockets, creates
Full/Compact/Impostor LODs, and generates semantic/anatomy masks used to bake
one cohesive inherited coat per assembled creature. The committed production
pack contains only generated OBJ, socket JSON, and PNG masks. It excludes
source `.blend` files, archives, source textures, previews, and screenshots.

The developer-only Quirky animal source pack remains documented under
`assets/source_creature_meshes/`; it is not part of the production roster or
production asset manifest.

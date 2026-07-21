# GeneForge Runtime Asset License Receipt

## Permission Evidence

The A-Life project owner explicitly stated in the project work thread that the
GeneForge Norn, Ettin, and Grendel source assets are MIT licensed. The supplied
source bundle did not include a standalone upstream license file, and no
verbatim license text was available on the supplied project page during asset
preparation. This receipt records the owner's permission statement without
inventing an upstream copyright notice.

- Source project: https://eem.foo/geneforge/
- Source author credited by the supplied catalog: Eem Foo
- License asserted by the project owner: MIT
- Source SHA-256 values: recorded per donor in
  `../creature_parts/geneforge_recipes.json`

## Production Modifications

A-Life imports the source `.blend` files with Blender 5.1.0, selects named
adult neutral body geometry, normalizes and separates reusable anatomical
groups, prepares canonical and cross-torso attachment transforms, generates
three LODs, and bakes semantic/anatomy masks for cohesive inherited coats.
Only generated runtime OBJ, socket JSON, and PNG mask outputs are distributed
in the production pack. Source `.blend` files, source textures, archives,
previews, and screenshots are not included.

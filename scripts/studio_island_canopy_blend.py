"""Editable packed Blender source for the actual runtime crown pack; no rendering or testing."""

import argparse
from pathlib import Path
import sys
import bpy

cli = argparse.ArgumentParser(description=__doc__)
cli.add_argument("asset", type=Path)
cli.add_argument("out", type=Path)
args = cli.parse_args(sys.argv[sys.argv.index("--") + 1 :])
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(args.asset.resolve()))
scene = bpy.context.scene
scene.unit_settings.system = "METRIC"
scene.name = "Qualia | established maritime spruce"
source = scene.collection
families = {}
for obj in list(bpy.data.objects):
    if obj.type != "MESH" or not obj.name.startswith("PineForm"):
        continue
    form, lod = (int(n) for n in obj.name[len("PineForm") :].split("_L"))
    category = ("Wind-sheared", "Sheltered", "Tall leader")[(form - 1) // 4]
    key = (category, lod)
    if key not in families:
        c = bpy.data.collections.new(f"{category} | LOD {lod}")
        source.children.link(c)
        families[key] = c
        c.hide_viewport = lod != 0
        c.hide_render = lod != 0
    for c in list(obj.users_collection):
        c.objects.unlink(obj)
    families[key].objects.link(obj)
    # Native mesh coordinates remain untouched. The scene only spreads the
    # objects on a workbench so an artist can compare individual silhouettes.
    obj.location = ((form - 1) % 4 * 6.0, (form - 1) // 4 * 8.0, 0.0)
    obj["island_native_form"] = form
    obj["island_native_lod"] = lod
    obj["native_root"] = "Unchanged original Conifer.glb bark coordinates through 3.2 m"
for c in list(bpy.data.collections):
    if c != source and not c.objects and not c.children:
        bpy.data.collections.remove(c)
for im in bpy.data.images:
    if im.source == "FILE" and im.has_data and not im.packed_file:
        im.pack()
scene["source"] = (
    "grove-crowns.glb; twelve source-bound families, no placement data or simulation state."
)
scene["palette"] = "The wood and needles reuse the single packed native PBR surface."
scene["editing"] = (
    "LOD 0 visible. Display other collections to edit L1 or L2. Mesh data origins are original trunk contacts."
)
args.out.parent.mkdir(exist_ok=True, parents=True)
bpy.ops.wm.save_as_mainfile(filepath=str(args.out.resolve()), compress=True)

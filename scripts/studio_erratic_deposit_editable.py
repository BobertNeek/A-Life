"""Modeler's native-scale desk for the separate A/B/C deposits.

Original and revised forms stay separate and reopen with their actual
metre-scale minerals. Presentation offsets live on parent empties only.
"""

from pathlib import Path
import sys
import bpy
from mathutils import Euler

raw = sys.argv[sys.argv.index("--") + 1 :]
previous, revised, out = map(Path, raw[:3])
talus = "--scree" in raw[3:]


def annotate_fragments(o):
    """Select a whole real clast even across duplicated glTF UV/normal seams."""
    mesh = o.data
    keys = [tuple(round(v, 6) for v in vert.co) for vert in mesh.vertices]
    parent = {v: v for v in keys}

    def root(v):
        while parent[v] != v:
            parent[v] = parent[parent[v]]
            v = parent[v]
        return v

    for edge in mesh.edges:
        a, b = (root(keys[i]) for i in edge.vertices)
        if a != b:
            parent[b] = a
    clasts = {}
    for i, p in enumerate(keys):
        clasts.setdefault(root(p), []).append(i)
    for k, ids in enumerate(
        sorted(clasts.values(), key=lambda q: min(keys[v][0] for v in q))
    ):
        o.vertex_groups.new(name=f"Clast {k+1:02d}").add(ids, 1.0, "REPLACE")
    o["selectable_fragments"] = len(clasts)


bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.unit_settings.system = "METRIC"
scene.unit_settings.scale_length = 1.0
active = None
for source, tag in (
    (previous, "Original height bands"),
    (revised, "Cleaved talus and weather lips" if talus else "Glacial rub faces"),
):
    master = bpy.data.collections.new(tag)
    scene.collection.children.link(master)
    for i, kind in enumerate(
        ("Scree",) if talus else ("BoulderA", "BoulderB", "BoulderC")
    ):
        group = bpy.data.collections.new(tag + " | " + kind)
        master.children.link(group)
        offset = bpy.data.objects.new(kind + " · optional bench position", None)
        group.objects.link(offset)
        offset.location.x = 0 if talus else (i - 1) * 6.5
        offset.empty_display_size = 0.30
        old = set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=str((source / (kind + ".glb")).resolve()))
        for o in set(bpy.data.objects) - old:
            for collection in list(o.users_collection):
                collection.objects.unlink(o)
            group.objects.link(o)
            if not o.parent:
                o.parent = offset
            level = next((k for k in range(3) if f"_L{k}" in o.name), None)
            if level is not None:
                o.name = f"{tag} | {kind} | L{level}"
                o.hide_viewport = level != 0
                o.hide_render = level != 0
                if talus and o.type == "MESH":
                    annotate_fragments(o)
                if level == 0 and (kind == "BoulderB" or talus) and source == revised:
                    active = o
    if source == previous:
        master.hide_viewport = True
        master.hide_render = True
bpy.ops.object.select_all(action="DESELECT")
if active:
    active.select_set(True)
    bpy.context.view_layer.objects.active = active
for area in bpy.context.screen.areas if bpy.context.screen else []:
    if area.type == "VIEW_3D":
        sp = area.spaces.active
        sp.region_3d.view_location = (0, 0, 0.23 if talus else 1.05)
        sp.region_3d.view_distance = 6.8 if talus else 15.8
        sp.region_3d.view_rotation = Euler((1.25, 0, -0.15)).to_quaternion()
        sp.shading.type = "MATERIAL"
scene["guide"] = (
    "Native metres. Three separate rock bodies in each A/B/C model. Original forms and all LODs are in the outliner. Gallery parents alone are offset. The appearance study applies no collision or terrain changes."
)
if talus:
    scene["guide"] = (
        "Native metres. Every near/medium/original overview can be inspected individually. Clast vertex groups respect the real disconnected stone bodies while retaining their UV seams. The artist library cannot change a game placement, collision or save."
    )
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(out.resolve()), compress=True)

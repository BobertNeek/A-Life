"""Author the lower-valley botany on each original measured native footprint.

No terrain, transform or game object is generated. Reopen the editable
Blender file to export the two ordinary prop libraries after an artist edits it.
"""

import argparse
import sys
from pathlib import Path

import bpy

here = Path(__file__).resolve().parent
sys.path.insert(0, str(here))
from island_meadow_sedges import sedges
from island_meadow_bells import bellflowers

FAMILIES = {
    "Grass": "sweep",
    "GrassSilver": "silver",
    "GrassAmber": "amber",
    "Flowers": None,
}


def bounds(vertices):
    return [
        (min(p[k] for p in vertices), max(p[k] for p in vertices)) for k in range(3)
    ]


def srgb(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def author(source, dest):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    for original in ("Grass", "Flowers"):
        bpy.ops.import_scene.gltf(
            filepath=str((source / (original + ".glb")).resolve())
        )
    # Untextured vertex material deliberately stays compatible with the
    # renderer's other opaque ordinary plants. No bespoke alpha/image slots.
    mat = bpy.data.materials.new("Coastal sedge and pale calyx")
    mat.use_nodes = True
    graph = mat.node_tree
    v = graph.nodes.new("ShaderNodeVertexColor")
    v.layer_name = "ArtColor"
    sh = graph.nodes.get("Principled BSDF")
    graph.links.new(v.outputs["Color"], sh.inputs["Base Color"])
    sh.inputs["Roughness"].default_value = 0.82
    mat.use_backface_culling = False
    # Remember the old bounds *before* substituting the original Grass mesh.
    targets = {
        (o, k): bounds([p.co for p in bpy.data.objects[f"{o}_L{k}"].data.vertices])
        for o in ("Grass", "Flowers")
        for k in range(3)
    }
    for name, habit in FAMILIES.items():
        original = "Flowers" if name == "Flowers" else "Grass"
        for level in range(3):
            title = f"{name}_L{level}"
            ob = bpy.data.objects.get(title)
            if ob is None:
                ob = bpy.data.objects[f"{original}_L{level}"].copy()
                bpy.context.scene.collection.objects.link(ob)
                ob.name = title
            component = (
                bellflowers(level) if name == "Flowers" else sedges(level, habit)
            )
            limits = bounds(component.points)
            target = targets[original, level]

            def fit(p):
                return tuple(
                    target[j][0]
                    + (p[j] - limits[j][0])
                    * (target[j][1] - target[j][0])
                    / (limits[j][1] - limits[j][0])
                    for j in (0, 1, 2)
                )

            me = bpy.data.meshes.new(title + " • drawn")
            me.from_pydata([fit(p) for p in component.points], [], component.faces)
            me.update()
            me.materials.append(mat)
            ink = me.color_attributes.new(
                name="ArtColor", type="BYTE_COLOR", domain="POINT"
            )
            for k, c in zip(ink.data, component.ink):
                k.color = (*(srgb(v) for v in c), 1)
            for p in me.polygons:
                p.use_smooth = True
            if original == "Grass":
                # A softly sky-facing vertex frame approximates transmitted
                # daylight on a folded leaf instead of a black paper edge.
                from mathutils import Vector

                soft = [
                    (v.normal * 0.38 + Vector((0.045, -0.085, 0.79))).normalized()
                    for v in me.vertices
                ]
                me.normals_split_custom_set_from_vertices(soft)
            old = ob.data
            ob.data = me
            if old.users == 0:
                bpy.data.meshes.remove(old)
            me.name = title
            ob["anchor"] = (
                "Same inherited local ground and measured LOD bounds as " + original
            )
            ob["source"] = "Original Qualia meadow artist set"
            ob.hide_render = level != 0
            ob.hide_set(level != 0)
    notes = bpy.data.texts.new("Wild meadow • artist notes")
    notes.write(
        "The four botanical portraits share their inherited original centre. Solo a family in the Outliner before editing. Grass, silver sedge and ripe fox-tail are alternatives for real original grass anchors. None of the new flowers are player interactables. Save the .blend and re-export with studio_meadow_flowers.py.\n"
    )
    bpy.context.scene.unit_settings.system = "METRIC"
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(dest.resolve()), compress=True)


def export(blend, out):
    bpy.ops.wm.open_mainfile(filepath=str(blend.resolve()))
    for title, plants in (
        ("Grass", ["Grass", "GrassSilver", "GrassAmber"]),
        ("Flowers", ["Flowers"]),
    ):
        bpy.ops.object.select_all(action="DESELECT")
        for name in plants:
            for level in range(3):
                ob = bpy.data.objects[f"{name}_L{level}"]
                ob.hide_set(False)
                ob.hide_render = False
                ob.select_set(True)
        bpy.ops.export_scene.gltf(
            filepath=str((out / (title + ".glb")).resolve()),
            export_format="GLB",
            use_selection=True,
            export_animations=False,
            export_lights=False,
            export_cameras=False,
            export_vertex_color="ACTIVE",
        )


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("out", type=Path)
    cli.add_argument("--author-from", type=Path)
    args = cli.parse_args(sys.argv[sys.argv.index("--") + 1 :])
    args.out.mkdir(exist_ok=True, parents=True)
    blend = args.out / "wild-sedge-and-bells.blend"
    if args.author_from:
        author(args.author_from, blend)
    export(blend, args.out)

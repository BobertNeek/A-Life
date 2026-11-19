"""Author or reopen the divided woodland fern and export its ordinary three meshes."""

import argparse
from pathlib import Path
import sys
import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
from island_fern_fans import levels
from compact_glb_colors import compact_colors


def bounds(points):
    return [(min(q[k] for q in points), max(q[k] for q in points)) for k in range(3)]


def author(source, blend):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(source))
    for level in range(3):
        name = f"Fern_L{level}"
        ob = bpy.data.objects[name]
        target = bounds([v.co for v in ob.data.vertices])
        earlier = ob.data
        mat = earlier.materials[0]
        brush = levels(level)
        outer = bounds(brush.v)

        def fit(p):
            return [
                (
                    target[k][0]
                    + ((p[k] - outer[k][0]) / (outer[k][1] - outer[k][0]))
                    * (target[k][1] - target[k][0])
                )
                for k in range(3)
            ]

        mesh = bpy.data.meshes.new(name + " new")
        mesh.from_pydata([fit(q) for q in brush.v], [], brush.faces)
        mesh.update()
        mesh.materials.append(mat)
        paint = mesh.color_attributes.new(
            name="ArtColor", type="BYTE_COLOR", domain="POINT"
        )
        for element, color in zip(paint.data, brush.rgb, strict=True):
            element.color = (*color, 1)
        mesh.color_attributes.active_color = paint
        ob.data = mesh
        bpy.data.meshes.remove(earlier)
        mesh.name = name
        ob["art_owner"] = "fern-sori.blend"
        ob["art_pass"] = (
            "Divided tapered native woodland leaf, not an umbrella-green hood"
        )
        ob["anchor"] = "Original ground-centered stock prop and per-LOD extents"
        ob.hide_set(level != 0)
        ob.hide_render = level != 0
    bpy.context.scene.unit_settings.system = "METRIC"
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(blend))


def export(blend, out):
    bpy.ops.wm.open_mainfile(filepath=str(blend))
    bpy.ops.object.select_all(action="DESELECT")
    for level in range(3):
        ob = bpy.data.objects[f"Fern_L{level}"]
        ob.hide_set(False)
        ob.hide_render = False
        ob.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=str(out),
        export_format="GLB",
        use_selection=True,
        export_animations=False,
        export_cameras=False,
        export_lights=False,
        export_vertex_color="ACTIVE",
    )
    compact_colors(out)


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("out", type=Path)
    cli.add_argument("--author-from", type=Path)
    a = cli.parse_args(sys.argv[sys.argv.index("--") + 1 :])
    a.out.mkdir(exist_ok=True, parents=True)
    source = a.out / "fern-sori.blend"
    if a.author_from:
        author(a.author_from.resolve(), source.resolve())
    export(source.resolve(), (a.out / "Fern.glb").resolve())

"""Author or reopen the divided woodland fern and export its ordinary three meshes."""

import argparse
from pathlib import Path
import sys
import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
from island_fern_blades import leaves


def bounds(points):
    return [(min(q[k] for q in points), max(q[k] for q in points)) for k in range(3)]


def author(source, blend):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(source))
    atlas = bpy.data.images.load(str(blend.parent / "lady-fronds.png"))
    atlas.pack()
    mat = bpy.data.materials.new("Lace of the island • woodland lady fern")
    mat.use_nodes = True
    tree = mat.node_tree
    bsdf = tree.nodes.get("Principled BSDF")
    bsdf.inputs["Roughness"].default_value = 0.77
    tex = tree.nodes.new("ShaderNodeTexImage")
    tex.image = atlas
    clip = tree.nodes.new("ShaderNodeMath")
    clip.operation = "GREATER_THAN"
    clip.inputs[1].default_value = 0.45
    tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    tree.links.new(tex.outputs["Alpha"], clip.inputs[0])
    tree.links.new(clip.outputs[0], bsdf.inputs["Alpha"])
    mat.use_backface_culling = False
    for level in range(3):
        name = f"Fern_L{level}"
        ob = bpy.data.objects[name]
        target = bounds([v.co for v in ob.data.vertices])
        earlier = ob.data
        positions, uvs, faces = leaves(level)
        outer = bounds(positions)

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
        mesh.from_pydata([fit(q) for q in positions], [], faces)
        mesh.update()
        mesh.materials.append(mat)
        uv = mesh.uv_layers.new(name="Leaf colour and cut edge")
        for loop in mesh.loops:
            uv.data[loop.index].uv = uvs[loop.vertex_index]
        for polygon in mesh.polygons:
            polygon.use_smooth = True
        ob.data = mesh
        bpy.data.meshes.remove(earlier)
        mesh.name = name
        ob["art_owner"] = "fern-sori.blend"
        ob["art_pass"] = (
            "Nine arching and twisting fronds; four painted pinna and sori charts"
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

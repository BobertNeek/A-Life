"""Rebuild the juvenile crown on fourteen actual original forks plus its leader."""

import argparse, sys, math
from pathlib import Path
from collections import defaultdict
import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from island_sapling_shoots import Growth, crown


def timber(mesh, b):
    ink = mesh.color_attributes.active_color
    parent = {}
    stock = []

    def key(i):
        return tuple(round(t, 5) for t in mesh.vertices[i].co)

    def find(p):
        if p not in parent:
            parent[p] = p
        if parent[p] != p:
            parent[p] = find(parent[p])
        return parent[p]

    for po in mesh.polygons:

        def value(loop):
            return ink.data[
                loop.index if ink.domain == "CORNER" else loop.vertex_index
            ].color

        loops = [mesh.loops[k] for k in po.loop_indices]
        c = value(loops[0])
        if c[1] > c[0] * 1.12:
            continue
        a = find(key(loops[0].vertex_index))
        stock.append([key(l.vertex_index) for l in loops])
        tri = []
        for l in loops:
            parent[find(key(l.vertex_index))] = a
            tri.append(
                b.p(
                    mesh.vertices[l.vertex_index].co,
                    value(l)[:3],
                    mesh.corner_normals[l.index].vector,
                )
            )
        b.tri(*tri)
    groups = defaultdict(set)
    for union in stock:
        groups[find(union[0])].update(union)
    all = []
    secondaries = []
    for pts in groups.values():
        hi = max(v[2] for v in pts)
        lo = min(v[2] for v in pts)
        end = [Vector(v) for v in pts if v[2] > hi - 0.048]
        tip = sum(end, Vector()) / len(end)
        all.append(("leader" if lo < 0.1 else "fork", tip))
        if lo > 0.1:
            base = [Vector(p) for p in pts if p[2] < lo + 0.050]
            root = sum(base, Vector()) / len(base)
            # Only five naked historic rods carry a lower lateral. It grows
            # directly off its actual original shaft, not off a guessed trunk.
            az = math.atan2(tip.y, tip.x)
            if math.sin(3 * az + 0.43) > 0.32:
                mid = root.lerp(tip, 0.66 + 0.09 * math.sin(az * 2))
                secondaries.append(("lateral", mid))
    all.extend(secondaries)
    all.sort(key=lambda t: (t[0], round(math.atan2(t[1].y, t[1].x), 1)))
    return [(i, *v) for i, v in enumerate(all)]


def author(src, out):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(src.resolve()))
    mat = bpy.data.materials.new("Young saltwood | living paint")
    mat.use_nodes = True
    mat.use_backface_culling = False
    attr = mat.node_tree.nodes.new("ShaderNodeVertexColor")
    attr.layer_name = "ArtColor"
    sh = mat.node_tree.nodes.get("Principled BSDF")
    mat.node_tree.links.new(attr.outputs["Color"], sh.inputs["Base Color"])
    sh.inputs["Roughness"].default_value = 0.78
    for level in (0, 1, 2):
        ob = bpy.data.objects[f"Sapling_L{level}"]
        old = ob.data
        b = Growth()
        ends = timber(old, b)
        wood = len(b.v)
        if level == 0:
            near = ends
        if level == 2:
            ends = near
        crown(b, ends, level)
        # Keep old grounded timber bit for bit in geometry. Only decorative
        # distal tissue receives an envelope fit; fork roots stay attached.
        limits = [
            (min(p.co[k] for p in old.vertices), max(p.co[k] for p in old.vertices))
            for k in range(3)
        ]
        limits[2] = (1.94, limits[2][1])
        v = b.v[wood:]
        origin = [p for _, _, p in ends]
        actual = [(min(p[k] for p in v), max(p[k] for p in v)) for k in range(3)]
        for p in v:
            d = min((p - e).length for e in origin)
            a = min(1, d / 0.34)
            a = a * a * (3 - 2 * a)
            p[:] = p.lerp(
                Vector(
                    [
                        lo
                        + (p[k] - actual[k][0])
                        * (hi - lo)
                        / (actual[k][1] - actual[k][0])
                        for k, (lo, hi) in enumerate(limits)
                    ]
                ),
                a,
            )
        mesh = bpy.data.meshes.new("Rooted juvenile, original wood")
        mesh.from_pydata(b.v, [], b.faces)
        mesh.update()
        mesh.materials.append(mat)
        paint = mesh.color_attributes.new(
            name="ArtColor", type="BYTE_COLOR", domain="POINT"
        )
        for slot, colour in zip(paint.data, b.c):
            slot.color = (*colour, 1)
        for poly in mesh.polygons:
            poly.use_smooth = True
        mesh.normals_split_custom_set(
            [b.normals.get(l.vertex_index, (0, 0, 0)) for l in mesh.loops]
        )
        ob.data = mesh
        bpy.data.meshes.remove(old)
        mesh.name = ob.name
        ob.hide_render = level != 0
        ob.hide_set(level != 0)
        ob["scope"] = (
            "All original woody geometry retained. New leaf-bearing twigs are not collisions."
        )
    bpy.context.preferences.filepaths.save_version = 0
    bpy.context.scene.unit_settings.system = "METRIC"
    bpy.ops.wm.save_as_mainfile(
        filepath=str((out / "young-saltwood.blend").resolve()), compress=True
    )


def export(out):
    bpy.ops.wm.open_mainfile(filepath=str((out / "young-saltwood.blend").resolve()))
    bpy.ops.object.select_all(action="DESELECT")
    for j in range(3):
        m = bpy.data.objects[f"Sapling_L{j}"]
        m.hide_set(False)
        m.hide_render = False
        m.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=str((out / "Sapling.glb").resolve()),
        export_format="GLB",
        use_selection=True,
        export_animations=False,
        export_lights=False,
        export_cameras=False,
        export_vertex_color="ACTIVE",
    )


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("out", type=Path)
    ap.add_argument("--author-from", type=Path)
    args = ap.parse_args(sys.argv[sys.argv.index("--") + 1 :])
    args.out.mkdir(exist_ok=True, parents=True)
    if args.author_from:
        author(args.author_from, args.out)
    export(args.out)

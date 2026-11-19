"""Author a maritime hardwood around the existing stock broadleaf's literal wood.

The previous tree is not remeshed wholesale: every original bole/branch/root
triangle keeps its absolute local coordinates. The new foliage can be edited
in the native blend and that file can then be exported without regenerating.
"""

import argparse, math, sys
from collections import defaultdict
from pathlib import Path
import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from island_alder_bough import Brush, crown, distant


def wood_and_tips(mesh, b):
    colour = mesh.color_attributes.active_color
    timber = []
    parent = {}

    def named(v):
        return tuple(round(q, 5) for q in mesh.vertices[v].co)

    def root(k):
        if k not in parent:
            parent[k] = k
        if parent[k] != k:
            parent[k] = root(parent[k])
        return parent[k]

    for poly in mesh.polygons:
        r, g, _, _ = colour.data[
            poly.loop_start if colour.domain == "CORNER" else poly.vertices[0]
        ].color
        if g >= r:
            continue
        ids = [named(v) for v in poly.vertices]
        key = root(ids[0])
        for q in ids[1:]:
            parent[root(q)] = key
        timber.append(tuple(poly.vertices))
    islands = defaultdict(list)
    for tri in timber:
        islands[root(named(tri[0]))].append(tri)
    ends = []
    for tris in islands.values():
        ids = {v for tri in tris for v in tri}
        pts = [mesh.vertices[v].co for v in ids]
        zmin = min(p.z for p in pts)
        zmax = max(p.z for p in pts)
        bottom = [p for p in pts if p.z < zmin + 0.11]
        high = [p for p in pts if p.z > zmax - 0.065]
        lo = sum(bottom, Vector()) / len(bottom)
        hi = sum(high, Vector()) / len(high)
        if zmin > 1.4 and zmax > 3.6:
            target = zmin + 0.62 * (zmax - zmin)
            dz = min(abs(p.z - target) for p in pts)
            ring = [p for p in pts if abs(p.z - target) <= dz + 0.085]
            middle = sum(ring, Vector()) / len(ring)
            ends.append((zmin, (hi.copy(), middle)))
        for tri in tris:
            coords = [mesh.vertices[j].co for j in tri]
            angles = []
            for p in coords:
                t = min(1, max(0, (p.z - zmin) / max(zmax - zmin, 0.001)))
                c = lo.lerp(hi, t)
                angles.append((0.5 + math.atan2(p.y - c.y, p.x - c.x) / math.tau) % 1)
            if max(angles) - min(angles) > 0.53:
                angles = [q + 1 if q < 0.5 else q for q in angles]
            # Face-local wrap never crosses into the leaf panels or far swatches.
            new = [
                b.add(
                    p,
                    (
                        0.14 + 0.47 * a,
                        0.5095 + 0.0715 * (p.z - zmin) / max(zmax - zmin, 0.001),
                    ),
                )
                for p, a in zip(coords, angles)
            ]
            b.tri(*new)
    return [p for _, p in sorted(ends, key=lambda pair: pair[0])]


def material(path):
    mat = bpy.data.materials.new("Salt alder • cut leaves and dappled bark")
    mat.use_nodes = True
    mat.use_backface_culling = False
    tex = mat.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = bpy.data.images.load(str(path.resolve()))
    tex.image.pack()
    clip = mat.node_tree.nodes.new("ShaderNodeMath")
    clip.operation = "GREATER_THAN"
    clip.inputs[1].default_value = 0.40
    sh = mat.node_tree.nodes.get("Principled BSDF")
    sh.inputs["Roughness"].default_value = 0.81
    for a, b in [
        (tex.outputs["Color"], sh.inputs["Base Color"]),
        (tex.outputs["Alpha"], clip.inputs[0]),
        (clip.outputs[0], sh.inputs["Alpha"]),
    ]:
        mat.node_tree.links.new(a, b)
    return mat


def author(src, out):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(src.resolve()))
    mat = material(out / "alder-ink.png")
    near = None
    for level in range(3):
        ob = bpy.data.objects[f"Broadleaf_L{level}"]
        old = ob.data
        b = Brush()
        ends = wood_and_tips(old, b)
        wood = len(b.v)
        if level == 0:
            near = ends
        if level == 2:
            ends = near
        if len(ends) != 8:
            raise ValueError("Source needs the eight established fork ends")
        tips = [h for h, _ in ends]
        center = sum(tips, Vector()) / len(tips)
        for i, (h, middle) in enumerate(ends):
            if level < 2:
                crown(b, h, center, level, i, middle)
            else:
                distant(b, h, i)
        # All source wood stays exactly where it was. Bring only terminal leaf
        # positions into the historical crown envelope. Blend that correction
        # away near the original fork so the new woody arms still connect.
        foliage = b.v[wood:]
        lo = [min(p[k] for p in foliage) for k in range(3)]
        hi = [max(p[k] for p in foliage) for k in range(3)]
        tlo = (-3, -2.75, 2.77)
        thi = (3, 2.75, 8)
        for p in foliage:
            mapped = Vector(
                [
                    tlo[k] + (p[k] - lo[k]) * (thi[k] - tlo[k]) / (hi[k] - lo[k])
                    for k in range(3)
                ]
            )
            closest = min((p - h).length for pair in ends for h in pair)
            w = min(1, closest / 0.51)
            w = w * w * (3 - 2 * w)
            # mathutils Vector.lerp returns a copy; write into the authoring list.
            p[:] = p.lerp(mapped, w)
        me = bpy.data.meshes.new(ob.name + " branch-borne alder")
        me.from_pydata(b.v, [], b.f)
        me.update()
        me.materials.append(mat)
        uv = me.uv_layers.new(name="Alder leaves and weathered bark")
        for loop in me.loops:
            uv.data[loop.index].uv = b.uv[loop.vertex_index]
        for po in me.polygons:
            po.use_smooth = True
        ob.data = me
        bpy.data.meshes.remove(old)
        ob["art_owner"] = "alder-boughs.blend"
        ob["wood"] = "all original wood triangles in their original coordinates"
        ob.hide_set(level != 0)
        ob.hide_render = level != 0
    bpy.context.scene.unit_settings.system = "METRIC"
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(
        filepath=str((out / "alder-boughs.blend").resolve()), compress=True
    )


def export(out):
    bpy.ops.wm.open_mainfile(filepath=str((out / "alder-boughs.blend").resolve()))
    bpy.ops.object.select_all(action="DESELECT")
    for level in range(3):
        ob = bpy.data.objects[f"Broadleaf_L{level}"]
        ob.hide_set(False)
        ob.hide_render = False
        ob.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=str((out / "Broadleaf.glb").resolve()),
        export_format="GLB",
        use_selection=True,
        export_animations=False,
        export_cameras=False,
        export_lights=False,
    )


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("out", type=Path)
    cli.add_argument("--author-from", type=Path)
    a = cli.parse_args(sys.argv[sys.argv.index("--") + 1 :])
    a.out.mkdir(exist_ok=True, parents=True)
    if a.author_from:
        author(a.author_from, a.out)
    export(a.out)

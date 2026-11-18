"""Build the playable nursery's shallow moss wash and two low understory species in Blender."""

from math import cos, floor, pi, sin, sqrt, tau
from pathlib import Path
from random import Random
from struct import unpack_from

import bpy

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates/alife_game_app/assets/care/nursery"
PAINT = ROOT / "target/artifacts/nursery-understory/floor.png"
CENTER = (81.25, 250.0)
SPAN = 25.0
LIFT = 0.004


class Ground:
    def __init__(self):
        data = (ROOT / "crates/alife_world/assets/island-v1.bin").read_bytes()
        if data[:8] != b"HLAND001":
            raise ValueError("Unsupported island heightfield")
        self.w, d, self.x, self.z, self.spacing, _ = unpack_from("<IIfffI", data, 8)
        self.h = unpack_from(f"<{self.w * d}f", data, 32)

    def height(self, x, z):
        u, v = (x - self.x) / self.spacing, (z - self.z) / self.spacing
        i, j = floor(u), floor(v)
        a = j * self.w + i
        h00, h10 = self.h[a : a + 2]
        h01, h11 = self.h[a + self.w : a + self.w + 2]
        u, v = u - i, v - j
        if v >= u:
            return h00 * (1 - v) + h01 * (v - u) + h11 * u
        return h00 * (1 - u) + h10 * (u - v) + h11 * v

    def here(self, x, z, lift=0):
        return (
            x,
            -z,
            self.height(CENTER[0] + x, CENTER[1] + z) - self.height(*CENTER) + lift,
        )


def object_from(name, vertices, faces, material, colors=None):
    data = bpy.data.meshes.new(name)
    data.from_pydata(vertices, [], faces)
    data.update()
    obj = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(obj)
    data.materials.append(material)
    if colors:
        layer = data.color_attributes.new(
            name="Pigment", type="FLOAT_COLOR", domain="POINT"
        )
        for dst, color in zip(layer.data, colors, strict=True):
            dst.color = (*color, 1)
        for face in data.polygons:
            face.use_smooth = True
    return obj


def surface(name):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    main = mat.node_tree.nodes.get("Principled BSDF")
    main.inputs["Roughness"].default_value = 0.91
    return mat, main


def ground_wash(ground):
    mat, bsdf = surface("Earth between the moss cushions")
    mat.surface_render_method = "BLENDED"
    pigment = mat.node_tree.nodes.new("ShaderNodeTexImage")
    pigment.image = bpy.data.images.load(str(PAINT))
    pigment.image.pack()
    for dst, src in (("Base Color", "Color"), ("Alpha", "Alpha")):
        mat.node_tree.links.new(pigment.outputs[src], bsdf.inputs[dst])
    # This grid shares both the island's exact coordinates and its diagonal.
    # It is a transparent coat, not another playable surface.
    count = round(SPAN / ground.spacing)
    if abs(count * ground.spacing - SPAN) > 1e-5:
        raise ValueError("The ground wash no longer aligns to the playable island")
    vertices, faces = [], []
    for j in range(count + 1):
        for i in range(count + 1):
            vertices.append(
                ground.here((i / count - 0.5) * SPAN, (j / count - 0.5) * SPAN, LIFT)
            )
            if i < count and j < count:
                a = j * (count + 1) + i
                b, d, c = a + 1, a + count + 1, a + count + 2
                faces.extend(((a, c, b), (a, d, c)))
    obj = object_from("Patchwork earth", vertices, faces, mat)
    uv = obj.data.uv_layers.new(name="Clearing")
    for loop in obj.data.loops:
        x, y, _ = vertices[loop.vertex_index]
        uv.data[loop.index].uv = (0.5 + x / SPAN, 0.5 + y / SPAN)
    return obj


class Leaves:
    def __init__(self, name, ground, material):
        self.name, self.ground, self.material = name, ground, material
        self.vertices, self.faces, self.colors = [], [], []

    def shoot(self, x, z, angle, length, width, height, color, shape):
        ca, sa = cos(angle), sin(angle)
        start = len(self.vertices)
        for i in range(8):
            t = i / 7
            w, rise = shape(t)
            for side in (-1, 0, 1):
                u, v = length * t, width * max(w, 0.012) * side
                a, b = x + ca * u - sa * v, z + sa * u + ca * v
                ridge = (1 - abs(side)) * width * w * 0.32
                self.vertices.append(
                    self.ground.here(a, b, 0.012 + height * rise + ridge)
                )
                brightness = (0.82 + 0.21 * t) * (1.23 if not side else 0.93)
                self.colors.append(tuple(c * brightness for c in color))
            if i:
                k = start + (i - 1) * 3
                self.faces.extend(
                    ((k, k + 1, k + 4, k + 3), (k + 1, k + 2, k + 5, k + 4))
                )

    def finish(self):
        return object_from(
            self.name, self.vertices, self.faces, self.material, self.colors
        )


def foliage(ground):
    mat, bsdf = surface("Sage, celadon and shell pink")
    attr = mat.node_tree.nodes.new("ShaderNodeVertexColor")
    attr.layer_name = "Pigment"
    mat.node_tree.links.new(attr.outputs["Color"], bsdf.inputs["Base Color"])
    sorrel = Leaves("Folded moon sorrel", ground, mat)
    ribbons = Leaves("Nacre ribbon grass", ground, mat)
    rng = Random(85403)

    def heart(t):
        return (sin(pi * t) ** 0.44 * (0.36 + 0.85 * t), 0.32 + sin(t * 2.58) * 0.74)

    def blade(t):
        return (
            sin(pi * (t * 0.9 + 0.1)) ** 0.65 * (1.05 - 0.62 * t),
            0.11 + 0.89 * sin(t * 2),
        )

    # Composition is anchored to the nursery, never the camera or a creature.
    # Keep the Hand landing and path open; longer leaves stay in the banks.
    drifts = [
        (-1.62, -1.53, 1.22, 0.51),
        (2.33, 1.58, 1.51, 0.57),
        (3.41, -2.32, 0.62, 1.20),
        (-1.15, 3.41, 1.14, 0.53),
        (-4.42, 0.4, 0.79, 1.72),
        (0.92, -4.46, 1.38, 0.86),
        (5.13, 3.76, 0.96, 1.65),
        (-5.08, -4.50, 1.61, 0.91),
    ]
    for ix, (cx, cz, rx, rz) in enumerate(drifts):
        for i in range(19):
            a, r = rng.uniform(0, tau), sqrt(rng.random())
            x, z = cx + cos(a) * r * rx, cz + sin(a) * r * rz
            size = rng.uniform(0.78, 1.37)
            if i % 5:
                for leaf in range(3):
                    sorrel.shoot(
                        x,
                        z,
                        a + leaf * tau / 3,
                        0.13 * size,
                        0.056 * size,
                        0.073 * size,
                        (0.14, 0.30 + (ix % 3) * 0.036, 0.17),
                        heart,
                    )
            else:
                for leaf in range(6):
                    angle = a + leaf * 2.39996
                    c = (0.32, 0.42, 0.31) if leaf % 4 else (0.45, 0.33, 0.29)
                    ribbons.shoot(
                        x,
                        z,
                        angle,
                        rng.uniform(0.14, 0.28) * size,
                        0.022 * size,
                        rng.uniform(0.10, 0.22) * size,
                        c,
                        blade,
                    )
    return sorrel.finish(), ribbons.finish()


def main():
    if not PAINT.is_file():
        raise SystemExit("Paint the ground with scripts/paint_nursery_floor.py first")
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.preferences.filepaths.save_version = 0
    ground = Ground()
    ground_wash(ground)
    foliage(ground)
    OUT.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT / "understory.blend"), compress=True)
    bpy.ops.export_scene.gltf(
        filepath=str(OUT / "understory.glb"),
        export_format="GLB",
        export_vertex_color="ACTIVE",
        export_all_vertex_colors=True,
        export_normals=True,
        export_texcoords=True,
    )


if __name__ == "__main__":
    main()

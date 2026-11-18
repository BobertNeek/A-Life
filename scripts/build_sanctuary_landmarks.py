"""Author the two grounded nursery body silhouettes. Run in Blender's Python."""

from math import cos, sin, tau
from pathlib import Path

import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates/alife_game_app/assets/care/landmarks"
STONE = (0.16, 0.24, 0.24)
CELADON = (0.59, 0.76, 0.67)


def mix(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))


class Sculpture:
    def __init__(self):
        self.points, self.colors, self.faces, self.flat = [], [], [], set()

    def shell(self, rings, tint, steps=48, ripple=0.0, lean=0.0, faceted=False):
        """Smooth, closed lathe with a hand-cut asymmetry and a stable footprint."""
        start = len(self.points)
        first_face = len(self.faces)
        for j, (z, rx, ry, x, y) in enumerate(rings):
            t = j / (len(rings) - 1)
            for i in range(steps):
                a = tau * i / steps
                texture = 1 + ripple * (0.6 * cos(3 * a + t) + 0.4 * sin(5 * a - 2 * t))
                self.points.append(
                    (
                        x + rx * texture * cos(a),
                        y + ry * texture * sin(a),
                        z + lean * rx * cos(a),
                    )
                )
                self.colors.append((*tint(t, a), 1.0))
                if j:
                    k = start + j * steps + i
                    n = start + j * steps + (i + 1) % steps
                    self.faces.append((k - steps, n - steps, n, k))
        self.faces.append(tuple(start + i for i in reversed(range(steps))))
        self.faces.append(
            tuple(start + (len(rings) - 1) * steps + i for i in range(steps))
        )
        if faceted:
            self.flat.update(range(first_face, len(self.faces)))

    def seam(self, path, radius, color, sides=8):
        pts = [Vector(p) for p in path]
        start = len(self.points)
        for j, p in enumerate(pts):
            tangent = (pts[min(j + 1, len(pts) - 1)] - pts[max(0, j - 1)]).normalized()
            d = tangent.cross(Vector((0, 0, 1)))
            if d.length < 0.01:
                d = tangent.cross(Vector((1, 0, 0)))
            d.normalize()
            e = tangent.cross(d).normalized()
            for i in range(sides):
                a = tau * i / sides
                self.points.append(tuple(p + radius * (cos(a) * d + sin(a) * e)))
                self.colors.append((*color, 1.0))
                if j:
                    k = start + j * sides + i
                    n = start + j * sides + (i + 1) % sides
                    self.faces.append((k - sides, n - sides, n, k))
        self.faces.append(tuple(start + i for i in reversed(range(sides))))
        self.faces.append(
            tuple(start + (len(pts) - 1) * sides + i for i in range(sides))
        )

    def finish(self, name):
        mesh = bpy.data.meshes.new(name)
        mesh.from_pydata(self.points, [], self.faces)
        mesh.update()
        colors = mesh.color_attributes.new(
            name="Kiln glaze", type="FLOAT_COLOR", domain="POINT"
        )
        for vert, color in zip(colors.data, self.colors):
            vert.color_srgb = color
        for face in mesh.polygons:
            face.use_smooth = len(face.vertices) == 4 and face.index not in self.flat
        obj = bpy.data.objects.new(name, mesh)
        bpy.context.collection.objects.link(obj)
        return obj


def kiln_ribbon(model, shell, angle, width, pigment):
    """Paint a narrow mineral stratum on the actual stone surface."""
    points = []
    for i, (z, rx, ry, x, y) in enumerate(shell):
        a = angle + 0.13 * sin(i * 0.86 + angle)
        points.append((x + (rx + 0.009) * cos(a), y + (ry + 0.009) * sin(a), z))
    model.seam(points, width, pigment)


def waykeeper():
    s = Sculpture()
    # Two fractured strata share a buried shoulder. A forked, off-centre
    # crown reads as a large fixed body even in silhouette, not as a switch.
    s.shell(
        [
            (-0.17, 0.94, 0.94, 0, 0),
            (0.04, 0.95, 0.95, 0, 0),
            (0.145, 0.83, 0.79, -0.035, 0.012),
            (0.24, 0.65, 0.58, -0.08, 0.03),
            (0.29, 0.44, 0.36, -0.12, 0.05),
        ],
        lambda t, a: mix(
            (0.33, 0.41, 0.39), (0.65, 0.67, 0.56), 0.17 + 0.47 * t + 0.15 * cos(3 * a)
        ),
        steps=17,
        ripple=0.066,
        lean=0.025,
        faceted=True,
    )
    for x, y, rx, ry, height in [
        (-0.42, -0.52, 0.31, 0.24, 0.29),
        (0.48, 0.36, 0.32, 0.27, 0.34),
        (0.45, -0.38, 0.29, 0.22, 0.23),
        (-0.49, 0.32, 0.34, 0.24, 0.27),
    ]:
        s.shell(
            [
                (-0.045, rx, ry, x, y),
                (0.09, rx, ry, x, y),
                (height, rx * 0.69, ry * 0.58, x * 0.94, y * 0.94),
                (height + 0.035, rx * 0.34, ry * 0.28, x * 0.88, y * 0.88),
            ],
            lambda t, a: mix((0.33, 0.40, 0.38), (0.66, 0.61, 0.45), 0.13 + 0.45 * t),
            steps=8,
            ripple=0.06,
            lean=0.10,
            faceted=True,
        )
    blade = [
        (0.24, 0.46, 0.56, -0.21, 0.03),
        (0.36, 0.52, 0.57, -0.22, 0.035),
        (0.64, 0.50, 0.52, -0.29, 0.045),
        (0.98, 0.43, 0.45, -0.34, 0.055),
        (1.24, 0.345, 0.37, -0.32, 0.07),
        (1.53, 0.25, 0.29, -0.24, 0.07),
        (1.74, 0.13, 0.18, -0.12, 0.06),
        (1.85, 0.029, 0.045, -0.035, 0.05),
    ]
    s.shell(
        blade,
        lambda t, a: mix(
            (0.42, 0.58, 0.55), (0.93, 0.88, 0.68), 0.17 + 0.68 * t + 0.05 * cos(3 * a)
        ),
        steps=13,
        ripple=0.035,
        lean=0.075,
        faceted=True,
    )
    wing = [
        (0.25, 0.35, 0.40, 0.30, -0.065),
        (0.37, 0.40, 0.44, 0.33, -0.07),
        (0.64, 0.34, 0.375, 0.405, -0.09),
        (0.87, 0.24, 0.29, 0.45, -0.095),
        (1.08, 0.14, 0.18, 0.465, -0.10),
        (1.24, 0.026, 0.055, 0.425, -0.105),
    ]
    s.shell(
        wing,
        lambda t, a: mix((0.31, 0.43, 0.43), (0.77, 0.78, 0.59), 0.22 + 0.63 * t),
        steps=11,
        ripple=0.028,
        lean=0.065,
        faceted=True,
    )
    for angle, width in [(-1.38, 0.022), (1.57, 0.018)]:
        kiln_ribbon(s, blade[1:-1], angle, width, (0.70, 0.47, 0.27))
        kiln_ribbon(s, blade[2:-1], angle + 0.16, 0.011, (0.93, 0.83, 0.59))
    kiln_ribbon(s, wing[:-1], -1.15, 0.019, (0.72, 0.76, 0.57))
    # A small low shard ties the hard base to the cut upright silhouettes.
    s.shell(
        [
            (0.22, 0.22, 0.19, -0.48, -0.36),
            (0.34, 0.25, 0.19, -0.48, -0.38),
            (0.55, 0.14, 0.12, -0.51, -0.41),
            (0.66, 0.025, 0.038, -0.48, -0.42),
        ],
        lambda t, a: mix(STONE, CELADON, 0.25 + 0.55 * t),
        steps=8,
        faceted=True,
    )
    return s.finish("Weathered waykeeper")


def caution_bloom():
    s = Sculpture()
    s.shell(
        [
            (-0.16, 0.93, 0.92, 0, 0),
            (0.04, 0.95, 0.94, 0, 0),
            (0.14, 0.81, 0.79, 0.005, 0.015),
            (0.21, 0.61, 0.57, -0.06, 0.035),
            (0.245, 0.38, 0.35, -0.10, 0.04),
        ],
        lambda t, a: mix(
            (0.31, 0.29, 0.32), (0.53, 0.43, 0.35), 0.14 + 0.43 * t + 0.13 * cos(3 * a)
        ),
        steps=17,
        ripple=0.064,
        lean=0.028,
        faceted=True,
    )
    # A broken, radial mineral fan rather than a regular ring of identical teeth.
    for a, r, h, width in [
        (0.05, 0.52, 0.58, 0.22),
        (0.89, 0.50, 0.71, 0.205),
        (1.74, 0.54, 0.57, 0.19),
        (2.52, 0.49, 0.67, 0.21),
        (3.38, 0.56, 0.51, 0.195),
        (4.12, 0.48, 0.77, 0.22),
        (5.01, 0.51, 0.56, 0.20),
        (5.82, 0.46, 0.65, 0.19),
    ]:
        co, si = cos(a), sin(a)
        rings = [
            (0.13, width * 1.2, width * 0.83, r * 0.66),
            (0.25, width * 1.12, width * 0.85, r * 0.87),
            (h * 0.64, width * 0.84, width * 0.61, r + 0.035),
            (h * 0.78, width * 0.59, width * 0.43, r + 0.11),
            (h * 0.95, width * 0.26, width * 0.23, r + 0.20),
            (h, width * 0.075, width * 0.065, r + 0.23),
        ]
        s.shell(
            [(z, rx, ry, d * co, d * si) for z, rx, ry, d in rings],
            lambda t, a: mix(
                (0.28, 0.22, 0.27), (0.94, 0.50, 0.29), 0.06 + 0.86 * t**2
            ),
            steps=7,
            ripple=0.02,
            lean=0.085,
            faceted=True,
        )
    s.shell(
        [
            (0.18, 0.51, 0.49, -0.05, 0),
            (0.34, 0.44, 0.39, -0.085, 0.025),
            (0.44, 0.29, 0.28, -0.14, 0.045),
            (0.46, 0.14, 0.13, -0.15, 0.055),
        ],
        lambda t, a: mix(
            (0.49, 0.26, 0.23), (0.91, 0.55, 0.33), 0.38 + 0.25 * t + 0.12 * cos(2 * a)
        ),
        steps=11,
        ripple=0.055,
        lean=0.065,
        faceted=True,
    )
    for x, y, size in [
        (0.23, -0.20, 0.115),
        (-0.32, 0.14, 0.095),
        (-0.15, -0.14, 0.125),
    ]:
        s.shell(
            [
                (0.27, size, size, x, y),
                (0.37, size * 1.04, size * 0.76, x, y),
                (0.47, size * 0.53, size * 0.41, x + 0.025, y),
                (0.49, size * 0.12, size * 0.09, x + 0.04, y + 0.014),
            ],
            lambda t, a: mix((0.67, 0.32, 0.20), (0.96, 0.69, 0.41), t),
            steps=6,
            faceted=True,
        )
    return s.finish("Emberstone outcrop")


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.preferences.filepaths.save_version = 0
    OUT.mkdir(parents=True, exist_ok=True)
    objects = [("waykeeper", waykeeper()), ("emberstone", caution_bloom())]
    for stem, obj in objects:
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        bpy.ops.export_scene.gltf(
            filepath=str(OUT / f"{stem}.glb"),
            use_selection=True,
            export_format="GLB",
            export_all_vertex_colors=True,
            export_materials="NONE",
            export_yup=True,
        )
    for i, (_, obj) in enumerate(objects):
        obj.location.x = i * 3.2
    # The editable source opens as an art study rather than two unlit gray shells.
    scene = bpy.context.scene
    lens = bpy.data.cameras.new("Clay study lens")
    camera = bpy.data.objects.new("Clay study lens", lens)
    bpy.context.collection.objects.link(camera)
    center = Vector((1.5, 0, 0.66))
    camera.location = center + Vector((3.7, -8.7, 4.25))
    camera.rotation_euler = (
        (center - camera.location).to_track_quat("-Z", "Y").to_euler()
    )
    lens.type = "ORTHO"
    lens.ortho_scale = 6.4
    scene.camera = camera
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.render.resolution_x = 880
    scene.render.resolution_y = 400
    scene.render.resolution_percentage = 100
    for view in [scene.display.shading] + [
        space.shading
        for screen in bpy.data.screens
        for area in screen.areas
        for space in area.spaces
        if space.type == "VIEW_3D"
    ]:
        view.light = "STUDIO"
        view.color_type = "VERTEX"
        view.show_shadows = True
        view.show_cavity = True
        view.cavity_type = "WORLD"
        view.background_type = "VIEWPORT"
        view.background_color = (0.085, 0.115, 0.13)
    proof = ROOT / "target/artifacts/sanctuary-landmarks/studio.png"
    proof.parent.mkdir(parents=True, exist_ok=True)
    scene.render.filepath = "//studio.png"
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT / "landmarks.blend"))
    scene.render.filepath = str(proof)
    bpy.ops.render.render(write_still=True)


if __name__ == "__main__":
    main()

"""Build the four living silhouettes of the lantern pear in Blender."""

from math import cos, pi, sin, tau
from pathlib import Path

import bpy
from mathutils import Matrix, Vector

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates/alife_game_app/assets/care/orchard"
OUT.mkdir(parents=True, exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.context.preferences.filepaths.save_version = 0
SOIL = (0.13, 0.074, 0.052)
JADE = (0.12, 0.40, 0.28)
PALE = (0.32, 0.67, 0.43)
BRASS = (0.75, 0.39, 0.14)
GLAZE = (0.19, 0.37, 0.37)
COROLLA = (0.98, 0.77, 0.56)
FRUIT = (1.0, 0.39, 0.12)


def mesh(name, vertices, faces, colors):
    geo = bpy.data.meshes.new(name)
    geo.from_pydata(vertices, [], faces)
    geo.update()
    pigment = geo.color_attributes.new(
        name="Enamel", type="FLOAT_COLOR", domain="POINT"
    )
    for vert, color in zip(pigment.data, colors):
        vert.color = (*color, 1.0)
    for poly in geo.polygons:
        poly.use_smooth = True
    o = bpy.data.objects.new(name, geo)
    bpy.context.collection.objects.link(o)
    return o


def spun(name, profile, xyz, color, n=20):
    vertices, colors = [], []
    for j, (r, z) in enumerate(profile):
        for i in range(n):
            a = tau * i / n
            vertices.append((xyz[0] + r * cos(a), xyz[1] + r * sin(a), xyz[2] + z))
            shade = 0.83 + 0.17 * j / max(1, len(profile) - 1) + 0.04 * cos(a - 0.7)
            colors.append(tuple(min(1.0, c * shade) for c in color))
    faces = [
        (j * n + i, j * n + (i + 1) % n, (j + 1) * n + (i + 1) % n, (j + 1) * n + i)
        for j in range(len(profile) - 1)
        for i in range(n)
    ]
    return mesh(name, vertices, faces, colors)


def leaf(origin, angle, length, width, lift, color, curl=0.0):
    vertices, colors, faces = [], [], []
    for j in range(8):
        t = j / 7
        half = width * (0.03 + sin(pi * t) ** 0.7)
        reach = length * t
        height = lift * sin(t * pi * 0.68) + curl * t * t
        for side in (-1, 0, 1):
            v = (
                reach * cos(angle) - side * half * sin(angle),
                reach * sin(angle) + side * half * cos(angle),
                height + (0.035 * (1 - abs(side)) * sin(t * pi)),
            )
            vertices.append(tuple(a + b for a, b in zip(origin, v)))
            k = 0.80 + 0.18 * t + 0.16 * (1 - abs(side))
            colors.append(tuple(min(1.0, c * k) for c in color))
    for j in range(7):
        for i in (0, 1):
            a = j * 3 + i
            faces.append((a, a + 3, a + 4, a + 1))
    # Mirror the surface just below the fold for a readable silhouette from any angle.
    n = len(vertices)
    vertices += [(x, y, z - 0.008) for x, y, z in vertices]
    colors += [tuple(c * 0.68 for c in rgb) for rgb in colors]
    faces += [tuple(n + i for i in reversed(face)) for face in faces]
    return mesh("Folded leaf", vertices, faces, colors)


def stem(a, b, radius):
    va, vb = Vector(a), Vector(b)
    length = (vb - va).length
    o = spun("Stem", [(radius, 0), (radius * 0.69, length)], (0, 0, 0), JADE, 9)
    o.data.transform(
        Matrix.Translation(va) @ (vb - va).to_track_quat("Z", "Y").to_matrix().to_4x4()
    )
    return o


def plant(stage):
    parts = [
        spun(
            "Rain bowl",
            [
                (0.01, 0),
                (0.30, 0.018),
                (0.38, 0.077),
                (0.365, 0.115),
                (0.315, 0.098),
                (0.28, 0.065),
                (0.01, 0.062),
            ],
            (0, 0, 0),
            GLAZE,
        ),
        spun(
            "Copper lip",
            [(0.37, 0.084), (0.382, 0.099), (0.36, 0.116)],
            (0, 0, 0),
            BRASS,
        ),
        spun(
            "Loam",
            [(0.325, 0.075), (0.31, 0.10), (0.16, 0.11), (0.001, 0.112)],
            (0, 0, 0),
            SOIL,
        ),
    ]
    if stage == 0:
        parts.append(
            spun(
                "Exposed pip",
                [(0.002, 0.08), (0.071, 0.096), (0.062, 0.15), (0.007, 0.20)],
                (0, 0, 0),
                BRASS,
                14,
            )
        )
    top = (0.05 * stage, -0.016 * stage, (0.14, 0.37, 0.65, 0.72)[stage])
    knee = (-0.026 * stage, 0.012 * stage, top[2] * 0.57)
    parts += [stem((0, 0, 0.092), knee, 0.036), stem(knee, top, 0.029)]
    for i in range(7):
        a = tau * i / 7
        parts.append(
            spun(
                "Rain gauge rivet",
                [(0.008, 0), (0.018, 0.003), (0.009, 0.013)],
                (0.343 * cos(a), 0.343 * sin(a), 0.105),
                BRASS,
                9,
            )
        )
    count = (2, 4, 6, 7)[stage]
    for i in range(count):
        angle = i * 2.399 + 0.15
        fraction = i / count
        origin = (
            knee[0] + (top[0] - knee[0]) * fraction,
            knee[1] + (top[1] - knee[1]) * fraction,
            0.11 + (top[2] - 0.11) * fraction,
        )
        parts.append(
            leaf(
                origin,
                angle,
                (0.16, 0.28, 0.40, 0.43)[stage],
                (0.041, 0.09, 0.127, 0.145)[stage],
                0.08 + 0.035 * (i % 3),
                PALE if i % 3 == 0 else JADE,
                -0.06 if stage > 1 else 0.02,
            )
        )
    if stage >= 2:
        for i, a in enumerate((0.25, 2.55, 4.35)):
            end = (0.28 * cos(a), 0.28 * sin(a), top[2] + (0.02, 0.07, -0.01)[i])
            parts.append(stem((top[0] * 0.5, top[1] * 0.5, top[2] - 0.19), end, 0.021))
            if stage == 2:
                for k in range(5):
                    parts.append(
                        leaf(end, k * tau / 5 + a, 0.13, 0.045, 0.064, COROLLA, -0.01)
                    )
                parts.append(
                    spun(
                        "Pollen",
                        [(0.005, -0.005), (0.032, 0.012), (0.009, 0.042)],
                        end,
                        BRASS,
                        12,
                    )
                )
            else:
                parts.append(
                    spun(
                        "Lantern pear",
                        [
                            (0.003, -0.23),
                            (0.070, -0.214),
                            (0.114, -0.153),
                            (0.106, -0.081),
                            (0.068, -0.039),
                            (0.023, 0),
                            (0.008, 0.008),
                        ],
                        end,
                        FRUIT,
                        18,
                    )
                )
                for k in range(3):
                    parts.append(
                        leaf(end, k * tau / 3 + a, 0.067, 0.021, 0.016, PALE, -0.04)
                    )
    return parts


for i, name in enumerate(("seed", "bud", "bloom", "ripe")):
    parts = plant(i)
    bpy.ops.object.select_all(action="DESELECT")
    for part in parts:
        part.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
    bpy.ops.object.join()
    obj = bpy.context.object
    obj.name = obj.data.name = f"Lantern pear · {name}"
    bpy.ops.export_scene.gltf(
        filepath=str(OUT / f"{name}.glb"),
        use_selection=True,
        export_format="GLB",
        export_materials="NONE",
        export_vertex_color="ACTIVE",
        export_all_vertex_colors=True,
        export_normals=True,
        export_texcoords=False,
    )
    obj.location.x = i * 1.2
bpy.ops.wm.save_as_mainfile(filepath=str(OUT / "lantern-orchard.blend"))

"""Editable Broadleaf canopy override, Blender 4.3.2; CPU authoring only.

Normal invocation exports the saved source. --author creates the deliberate
2026-10-16 replacement once; it never edits the multi-asset source library.
The original GLB is retained beside the override for review and regression QA.
"""
import argparse
import hashlib
import json
import math
import random
import sys
from pathlib import Path

import bpy
from mathutils import Vector, noise

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_island_assets import Geometry, overview_tree
from island_art_common import export, fit_dimensions, material, mesh

ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / 'crates/alife_game_app/assets/landscape/island'
SOURCE = HERE / 'broadleaf-canopy.blend'
OUTPUT = HERE / 'Broadleaf.glb'
BASELINE_SHA256 = '819a9104b4b001cb381dac0848215e77242ca930364f0e8797f1301b59d05f0f'

# Shared by near and middle LODs. Broad, overlapping leaf fans form a single
# asymmetric crown, with daylight gaps beneath the boughs rather than a ball.
CROWNS = [
    ((-2.00, -.10, 4.18), (1.12, .91, .34)),
    ((1.78, .08, 4.78), (1.33, 1.10, .43)),
    ((-.15, 1.30, 5.50), (1.45, .95, .43)),
    ((-1.22, -.72, 5.93), (1.54, 1.02, .47)),
    ((1.29, -.49, 6.02), (1.17, .94, .39)),
    ((.91, 1.01, 6.73), (1.10, .87, .49)),
    ((-1.91, .55, 6.57), (.91, .78, .34)),
    ((-.61, .15, 7.18), (1.43, 1.08, .54)),
]


def tint(position, crown, growth=0):
    """Large painted leaf patches survive distance; no textures or alpha cards."""
    x, y, z = position
    light = .88 + .10 * math.sin(x * 2.4 + y * 1.7 + crown * .8)
    upper = max(0., min(1., (z - 3.6) / 4.5))
    # Jade shadows, warm young leaves, and a quiet golden upper rim.
    warm = .5 + .5 * math.sin(crown * 2.3)
    return ((.105 + .055 * upper + .030 * warm + growth * .05) * light,
            (.255 + .105 * upper + .025 * warm + growth * .07) * light,
            (.105 + .035 * upper - .020 * warm) * light)


def crown(g, center, size, index, level):
    import bmesh
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=2, radius=1)
    bm.verts.ensure_lookup_table()
    bm.verts.index_update()
    base = len(g.v)
    for vert in bm.verts:
        v = vert.co
        angle = math.atan2(v.y, v.x)
        # Scalloped fan outline and shallow folds, rather than pebble spheres.
        folds = math.sin(angle * 3 + index * .7) * .17 + math.sin(angle * 7 + index) * .06
        radial = (1 + folds * (1 - v.z * v.z)) * .995
        p = Vector(center) + Vector((v.x * size[0] * radial,
                                    v.y * size[1] * radial,
                                    v.z * size[2] + v.x * .18 + v.y * .08))
        g.v.append(tuple(p))
        color = tint(p, index)
        shade = .93 + .09 * v.z + .035 * noise.noise_vector(v * 4 + Vector((index, 0, 0))).x
        g.c.append(tuple(c * shade for c in color))
    for face in bm.faces:
        g.f.append(tuple(base + v.index for v in face.verts))
    bm.free()
    if level == 0:
        rng = random.Random(1016 + index)
        leaf_count = 100
        for leaf in range(leaf_count):
            # Evenly distributed broad leaves hug each crown. Their overlapping
            # folded planes create real foliage edges and painted light breaks.
            z = 1 - 2 * (leaf + .5) / leaf_count
            a = leaf * 2.39996323 + index * .71
            radial = math.sqrt(1 - z * z)
            direction = Vector((radial * math.cos(a), radial * math.sin(a), z))
            radial = 1 + (math.sin(a * 3 + index * .7) * .17 + math.sin(a * 7 + index) * .06) * (1 - z * z)
            p = Vector(center) + Vector((direction.x * size[0] * radial,
                                        direction.y * size[1] * radial,
                                        direction.z * size[2] + direction.x * .18 + direction.y * .08))
            normal = Vector(tuple(direction[k] / size[k] for k in range(3))).normalized()
            along = Vector((math.cos(a + .5), math.sin(a + .5), -.25))
            along = (along - normal * along.dot(normal)).normalized()
            across = normal.cross(along).normalized()
            length = .62 + rng.random() * .20
            width = .13 + rng.random() * .07
            start = len(g.v)
            shape = [p - along * length * .65,
                     p - across * width,
                     p + normal * .012,
                     p + across * width,
                     p + along * length * .65]
            base_tint = tint(p, index, .55)
            for k, point in enumerate(shape):
                g.v.append(tuple(point))
                shade = (.91, .98, 1.06, .95, 1.04)[k]
                g.c.append(tuple(c * shade for c in base_tint))
            g.f.extend(tuple(start + k for k in face) for face in
                       [(0, 1, 2), (0, 2, 3), (1, 4, 2), (2, 4, 3)])
    g.crowns.append(list(range(base, len(g.v))))


def geometry(level):
    g = Geometry()
    # First 40 vertices are the trunk; the overview envelope's explicit source
    # contract uses these four rings. Buttressed base and a readable curved fork.
    bark = (.235, .142, .075)
    g.tube([(0, 0, 0), (-.16, .04, .52), (.04, .08, 2.22), (-.15, .12, 4.30)],
           [.44, .30, .235, .095], bark, 10)
    branch_targets = [c[0] for c in CROWNS]
    for i, target in enumerate(branch_targets):
        tx, ty, tz = target
        start = Vector((-.02, .10, 2.15 + i * .17))
        end = Vector(target)
        middle = start.lerp(end, .52) + Vector((-.16 if i % 2 else .13, .04, .28))
        if level == 0:
            points = [start, start.lerp(middle, .55), middle, end]
            radii = [.17 - i * .008, .125, .075, .018]
        else:
            points, radii = [start, middle, end], [.16 - i * .008, .085, .018]
        g.tube(points, radii, bark, 7 if level == 0 else 4)
    for i in range(5):
        a = i * math.tau / 5 + .4
        root = [(math.cos(a) * .85, math.sin(a) * .72, .035),
                (math.cos(a) * .35, math.sin(a) * .32, .20), (0, 0, .62)]
        g.tube(root if level == 0 else [root[0], root[2]],
               [.025, .10, .11] if level == 0 else [.025, .11], bark,
               5 if level == 0 else 3)
    for i, (center, size) in enumerate(CROWNS):
        crown(g, center, size, i, level)
    return g


def author():
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)
    noise.seed_set(20261016)
    mat = material('Island shared painted surface', (1, 1, 1), True)
    mat.use_backface_culling = False
    collection = bpy.context.scene.collection
    levels = []
    for level in range(2):
        g = geometry(level)
        ob = mesh('Broadleaf_L' + str(level), g.v, g.f, mat, g.c, True)
        fit_dimensions(ob, (6, 5.5, 8))
        levels.append(ob)
        if level == 0:
            distant = overview_tree(ob, g, 'Broadleaf', collection)
            fit_dimensions(distant, (6, 5.5, 8))
    levels.append(distant)
    for level, ob in enumerate(levels):
        ob['art_owner'] = 'broadleaf-canopy.blend'
        ob['art_pass'] = '2026-10-16 layered leaf fans and curved load-bearing boughs'
        ob['origin_contract'] = 'ground-centred; metres; Z-up source / Y-up runtime'
        ob.hide_set(level != 0)
        ob.hide_render = level != 0
    bpy.context.scene.unit_settings.system = 'METRIC'
    bpy.context.scene.unit_settings.scale_length = 1
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE), compress=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--author', action='store_true')
    args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    assert bpy.app.version == (4, 3, 2), 'Use the verified Blender 4.3.2 exporter'
    assert hashlib.sha256((HERE / 'broadleaf-canopy/Broadleaf.baseline.glb').read_bytes()).hexdigest() == BASELINE_SHA256
    if args.author:
        author()
    else:
        bpy.ops.wm.open_mainfile(filepath=str(SOURCE))
    levels = [bpy.data.objects['Broadleaf_L' + str(i)] for i in range(3)]
    for ob, ceiling in zip(levels, (4820, 1038, 88)):
        assert tuple(ob.location) == (0, 0, 0) and tuple(ob.scale) == (1, 1, 1)
        assert tuple(ob.rotation_euler) == (0, 0, 0)
        count = sum(len(p.vertices) - 2 for p in ob.data.polygons)
        assert count <= ceiling, (ob.name, count, ceiling)
        ob.hide_set(False)
        ob.hide_render = False
        print(ob.name, count, 'triangles', flush=True)
    export(levels, OUTPUT, export_animations=False, export_cameras=False, export_lights=False)
    # A full base-library rebuild reinstalls the old Broadleaf counts. Reapply
    # only this override's metadata along with its GLB, keeping every other
    # species and the base-library provenance intact.
    specification = HERE / 'asset-specification.json'
    data = json.loads(specification.read_text())
    counts = [sum(len(p.vertices) - 2 for p in ob.data.polygons) for ob in levels]
    dimensions = [round(max(v.co[k] for v in levels[0].data.vertices) -
                        min(v.co[k] for v in levels[0].data.vertices), 6) for k in range(3)]
    dimensions = [int(v) if v.is_integer() else v for v in dimensions]
    data['assets']['Broadleaf'].update(
        width_depth_height_m=dimensions,
        near_triangles=counts[0], far_triangles=counts[1], overview_triangles=counts[2],
        editable_override='broadleaf-canopy.blend', export_recipe='scripts/build_broadleaf_canopy.py',
        override_blender='4.3.2')
    specification.write_text(json.dumps(data, indent=2) + '\n')


if __name__ == '__main__':
    main()

"""Matched CPU material reviews; not an in-engine or GPU performance capture.

blender -b -t 4 --factory-startup --python scripts/render_broadleaf_canopy.py --
  --asset <Broadleaf.glb> --output <folder> [--grove]
The grove uses the unchanged Island terrain and exact authored prop transforms.
"""
import argparse
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from island_art_common import camera, material, mesh, render, setup_scene

ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / 'crates/alife_game_app/assets/landscape/island'


def load(path):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path))
    return [o for o in bpy.data.objects if o not in before and o.type == 'MESH']


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--asset', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--grove', action='store_true')
    a = p.parse_args(sys.argv[sys.argv.index('--') + 1:])
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)
    setup_scene()
    scene = bpy.context.scene
    scene.cycles.use_denoising = False
    scene.cycles.samples = 48
    tree = {o.name.rsplit('.', 1)[0]: o for o in load(a.asset)}
    for ob in tree.values():
        ob.hide_render = True
    if a.grove:
        focus = Vector((55, -181, 12.4))
        terrain = load(HERE / 'terrain-chunks.glb')
        for ob in terrain:
            center = sum((ob.matrix_world @ Vector(corner) for corner in ob.bound_box), Vector()) / 8
            ob.hide_render = not (ob.name.endswith('_L0') and
                                  abs(center.x - focus.x) < 120 and abs(center.y - focus.y) < 120)
        props = json.loads((HERE / 'props.json').read_text())
        libraries = {'Broadleaf': tree['Broadleaf_L0']}
        count = 0
        for prop in props:
            x, y, z = prop['position']
            if abs(x - focus.x) > 60 or abs(-z - focus.y) > 60:
                continue
            kind = prop['kind']
            if kind not in libraries:
                imported = load(HERE / (kind + '.glb'))
                for ob in imported:
                    ob.hide_render = True
                libraries[kind] = next(o for o in imported if o.name.rsplit('.', 1)[0] == kind + '_L0')
            ob = libraries[kind].copy()
            scene.collection.objects.link(ob)
            ob.hide_render = False
            ob.location = (x, -z, y)
            ob.rotation_euler = (0, 0, prop['yaw'])
            ob.scale = (prop['scale'],) * 3
            count += kind == 'Broadleaf'
        cam = camera('Authored grove', focus + Vector((48, -65, 56)), focus + Vector((0, 0, 5)), 64)
        render(cam, a.output / 'grove.png', (1280, 900))
        print('Unchanged authored grove: Broadleaf instances', count)
    else:
        mesh('Review ground', [(-9, -9, -.01), (9, -9, -.01), (9, 9, -.01), (-9, 9, -.01)],
             [(0, 1, 2, 3)], material('Review ground', (.13, .17, .145)))
        cam = camera('Matched three-quarter', (13, -18, 12), (0, 0, 3.9), 10)
        for level in range(3):
            for name, ob in tree.items():
                ob.hide_render = name != 'Broadleaf_L' + str(level)
            render(cam, a.output / ('close-L' + str(level) + '.png'), (900, 900))
        # Also inspect the foliage profile from the other side.
        cam = camera('Matched reverse', (-15, 12, 10), (0, 0, 3.9), 10)
        for name, ob in tree.items():
            ob.hide_render = name != 'Broadleaf_L0'
        render(cam, a.output / 'reverse-L0.png', (900, 900))


if __name__ == '__main__':
    main()

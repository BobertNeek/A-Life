"""Reconstruct a color-only Fern override from the exact committed GLB.

Blender 4.3.2 CPU authoring. Normal export loads current editable source.
The one-time --author-from option accepts only the pinned untouched baseline.
Geometry, normals, hierarchy and material parameters are deliberately untouched.
"""
import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
from compact_glb_colors import compact_colors
from retain_fern_geometry import retain_geometry

BASE_SHA256 = '6dcccfdb1044e35d60cf0b14ebbe9de2ad220be590d11511d0b9e3d161cd62f3'
ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'crates/alife_game_app/assets/landscape/island'
RECEIPTS = ROOT / 'target/artifacts/fern-readability/export'


def clamp(x):
    return max(0., min(1., x))


def paint(ob):
    mesh = ob.data
    attr = mesh.color_attributes.active_color
    assert attr is not None and attr.domain == 'CORNER'
    original = [tuple(c.color) for c in attr.data]
    attr.name = 'ArtColor'
    # Preserve the original authored edge / ridge / growth gradient. Expand its
    # narrow luminance range, then give each arch a small deterministic tint.
    # All LODs use the same function in source-space metres.
    for loop, entry, old in zip(mesh.loops, attr.data, original):
        x, y, z = mesh.vertices[loop.vertex_index].co
        growth = clamp((old[1] * 255 - 57) / 24) ** .85
        lower = (.024, .140, .070)
        upper = (.155, .430, .190)
        color = [a * (1 - growth) + b * growth for a, b in zip(lower, upper)]
        azimuth = math.atan2(y, x)
        variation = 1 + .055 * math.sin(azimuth * 3 + .45)
        # Strongest near the upper frond layer. Warm new growth remains green.
        height = clamp(z / .75)
        color[0] *= variation * (.97 + .06 * height)
        color[1] *= variation
        color[2] *= 1 + .025 * math.cos(azimuth * 3 + .45)
        entry.color = (*color, 1)
    ob['art_owner'] = 'fern-readability.blend'
    ob['art_pass'] = '2026-10-09 color-only folded-leaf readability'
    ob['origin_contract'] = 'ground-centred; metres; Z-up source / Y-up runtime'
    ob['baseline_sha256'] = BASE_SHA256


def author(baseline, source):
    assert hashlib.sha256(baseline.read_bytes()).hexdigest() == BASE_SHA256, (
        'Author only from the original committed Fern GLB; do not reapply the pass')
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)
    bpy.ops.import_scene.gltf(filepath=str(baseline))
    for level in range(3):
        paint(bpy.data.objects['Fern_L' + str(level)])
    shared = bpy.data.materials['Island shared painted surface']
    bsdf = next(n for n in shared.node_tree.nodes if n.type == 'BSDF_PRINCIPLED')
    node = shared.node_tree.nodes.new('ShaderNodeVertexColor')
    node.layer_name = 'ArtColor'
    shared.node_tree.links.new(node.outputs['Color'], bsdf.inputs['Base Color'])
    bpy.context.scene.unit_settings.system = 'METRIC'
    bpy.context.scene.unit_settings.scale_length = 1
    bpy.context.preferences.filepaths.save_version = 0
    for level in range(3):
        ob = bpy.data.objects['Fern_L' + str(level)]
        ob.hide_set(level != 0)
        ob.hide_render = level != 0
    source.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(source))


def export(output, baseline, receipts):
    objects = [bpy.data.objects['Fern_L' + str(i)] for i in range(3)]
    bpy.ops.object.select_all(action='DESELECT')
    for ob in objects:
        assert tuple(ob.location) == (0., 0., 0.)
        assert tuple(ob.scale) == (1., 1., 1.)
        assert all(abs(x) < 1e-7 for x in ob.rotation_euler)
        ob.hide_set(False)
        ob.hide_render = False
        ob.select_set(True)
    output.parent.mkdir(parents=True, exist_ok=True)
    receipts.mkdir(parents=True, exist_ok=True)
    native = receipts / 'Fern.native-export.glb'
    bpy.ops.export_scene.gltf(filepath=str(native), export_format='GLB',
        use_selection=True, export_animations=False, export_vertex_color='ACTIVE')
    compact_colors(native)
    retained = retain_geometry(baseline, native, output)
    (receipts / 'source-export-adapter.json').write_text(json.dumps(retained, indent=2) + '\n')
    print('FERN_READABILITY_EXPORT', json.dumps({
        'path': str(output), 'bytes': output.stat().st_size,
        'triangles': [sum(len(p.vertices) - 2 for p in o.data.polygons) for o in objects]
    }), flush=True)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source', type=Path, default=ASSETS / 'fern-readability.blend')
    p.add_argument('--output', type=Path, default=ASSETS / 'Fern.glb')
    p.add_argument('--baseline', type=Path, default=ASSETS / 'fern-readability/Fern.baseline.glb')
    p.add_argument('--author-from', type=Path)
    p.add_argument('--receipt-dir', type=Path, default=RECEIPTS)
    args = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
    a = p.parse_args(args)
    if a.author_from:
        author(a.author_from, a.source)
    else:
        bpy.ops.wm.open_mainfile(filepath=str(a.source))
    assert hashlib.sha256(a.baseline.read_bytes()).hexdigest() == BASE_SHA256
    export(a.output, a.baseline, a.receipt_dir)


if __name__ == '__main__':
    main()

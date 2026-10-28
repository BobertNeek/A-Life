"""Apply the raccoon-shaped hand/foot pass once to an accepted checkpoint.

Run Blender 5.2.1 with --python scripts/refine_hearthling_extremities.py --
--source /path/to/pre-pass/hearthling.blend. Optional --output-dir isolates a
review export. The full source exporter also calls the same geometry functions.
"""
import argparse
from array import array
import runpy
import sys
from pathlib import Path

import bpy

ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / 'crates/alife_game_app/assets/creatures/hearthling'


def refine_extremities():
    rig = bpy.data.objects['Hearthling_Rig']
    assert rig.get('selected_reference_revision') == 10
    assert rig.get('cream_transition_revision') == 1
    assert not rig.get('raccoon_hand_revision'), 'Use the pre-extremity checkpoint'
    assert not rig.get('raccoon_foot_revision'), 'Use the pre-extremity checkpoint'
    # New smooth skin uses the unused flat corner of the accepted normal atlas.
    atlas = bpy.data.images['Hearthling_FurNormal']
    assert tuple(atlas.size) == (2048, 2048), 'Expected the authoring normal atlas'
    pixels = array('f', [0]) * (2048 * 2048 * 4)
    atlas.pixels.foreach_get(pixels)
    assert all(abs(pixels[4 * (y * 2048 + x) + k] - expected) < .01
               for y in range(8, 57) for x in range(8, 57)
               for k, expected in enumerate((.5, .5, 1))), 'Neutral UV patch is occupied'
    from hearthling_raccoon_hands import refine_hands
    from hearthling_raccoon_feet import refine_feet
    result = {'hands': refine_hands(), 'feet': refine_feet()}
    mesh = bpy.data.objects['Hearthling_Body'].data
    mesh.calc_loop_triangles()
    uv = mesh.uv_layers.active
    for name in ['RaccoonHandRegion', 'RaccoonFootRegion']:
        tag = mesh.attributes[name]
        triangles = [t for t in mesh.loop_triangles if tag.data[t.polygon_index].value]
        assert triangles, ('Missing new anatomy', name)
        assert all(t.area > 1e-12 for t in triangles), ('Degenerate surface triangle', name)
        for triangle in triangles:
            a, b, c = [uv.data[i].uv for i in triangle.loops]
            assert abs((b-a).cross(c-a)) * .5 > 1e-12, ('Degenerate UV triangle', name)
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', type=Path, default=HERE / 'hearthling.blend')
    parser.add_argument('--output-dir', type=Path, default=HERE)
    args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    sys.path.insert(0, str(ROOT / 'scripts'))
    bpy.ops.wm.open_mainfile(filepath=str(args.source))
    print('RACCOON_EXTREMITIES', refine_extremities(), flush=True)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(args.output_dir / 'hearthling.blend'))
    runpy.run_path(str(ROOT / 'scripts/optimize_hearthling_runtime.py'),
                   init_globals={'HEARTHLING_ASSET_DIR': args.output_dir.resolve()})

"""Apply the coat follow-up once to an accepted editable Hearthling checkpoint.

The full source exporter includes this same pass. This entry point preserves
an existing checkpoint's exact face, UVs, weights and authored animation data.
Run with Blender 5.2.1; optional: -- --source /path/to/pre-pass/hearthling.blend.
"""
import argparse
import runpy
import sys
from pathlib import Path

import bpy

ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / 'crates/alife_game_app/assets/creatures/hearthling'
parser = argparse.ArgumentParser()
parser.add_argument('--source', type=Path, default=HERE / 'hearthling.blend')
args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
bpy.ops.wm.open_mainfile(filepath=str(args.source))
rig = bpy.data.objects['Hearthling_Rig']
assert rig.get('selected_reference_revision') == 10
assert not rig.get('cream_transition_revision'), 'Use the pre-pass checkpoint'
sys.path.insert(0, str(ROOT / 'scripts'))
from refine_hearthling_selected import soften_cream_transitions

previous = rig.data.pose_position
rig.data.pose_position = 'REST'
soften_cream_transitions()
rig['cream_transition_revision'] = 1
rig.data.pose_position = previous
bpy.context.view_layer.update()
bpy.ops.wm.save_as_mainfile(filepath=str(HERE / 'hearthling.blend'))
runpy.run_path(str(ROOT / 'scripts/optimize_hearthling_runtime.py'))

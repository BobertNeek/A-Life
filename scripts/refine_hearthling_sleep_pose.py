"""Keep the existing left hand clear of the folded knee in the sleep clip.

This is a pose correction, not another extremity geometry pass. Run on the
accepted post-extremity native checkpoint with --source and --output-dir, or
through export_hearthling.py for full regeneration.
"""
import argparse
from pathlib import Path
import runpy

import bpy

ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / 'crates/alife_game_app/assets/creatures/hearthling'
SLEEP_LEFT_ARM_CLEARANCE = .18


def refine_sleep_hand_clearance():
    rig = bpy.data.objects['Hearthling_Rig']
    assert rig.get('raccoon_hand_revision') == 1
    assert rig.get('raccoon_foot_revision') == 1
    assert not rig.get('sleep_hand_clearance_revision'), 'Use the pre-clearance checkpoint'
    action = bpy.data.actions['Hearthling_Sleep']
    bone = rig.pose.bones['upper_arm.L']
    assert bone.rotation_mode == 'XYZ'
    scene = bpy.context.scene
    saved_action = rig.animation_data.action
    saved_frame = scene.frame_current
    tracks = [(track, track.mute) for track in rig.animation_data.nla_tracks]
    try:
        for track, _ in tracks:
            track.mute = True
        rig.animation_data.action = action
        for frame in range(1, 74):
            scene.frame_set(frame)
            # A small existing-joint lift/abduction clears the left thigh at
            # compact, neutral and broad paw scales. Do not translate fingers
            # or disturb the support pose, right hand, idle or walking clips.
            bone.rotation_euler.x += SLEEP_LEFT_ARM_CLEARANCE
            bone.keyframe_insert('rotation_euler', index=0, frame=frame, group=bone.name)
        rig['sleep_hand_clearance_revision'] = 1
    finally:
        rig.animation_data.action = saved_action
        for track, mute in tracks:
            track.mute = mute
        scene.frame_set(saved_frame)
    return {'clip': action.name, 'bone': bone.name,
            'rotation_x_delta_radians': SLEEP_LEFT_ARM_CLEARANCE, 'samples': 73}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', type=Path, default=HERE / 'hearthling.blend')
    parser.add_argument('--output-dir', type=Path, default=HERE)
    import sys
    args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    bpy.ops.wm.open_mainfile(filepath=str(args.source))
    print('SLEEP_HAND_CLEARANCE', refine_sleep_hand_clearance(), flush=True)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(args.output_dir / 'hearthling.blend'))
    runpy.run_path(str(ROOT / 'scripts/optimize_hearthling_runtime.py'),
                   init_globals={'HEARTHLING_ASSET_DIR': args.output_dir.resolve()})

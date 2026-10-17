"""No-render Blender CPU regression for the exact authored-preview assignments.

Run with Blender 4.3.2 --background --factory-startup --python-exit-code 1
--python scripts/check_authored_preview_rotation.py -- --output receipt.json.
Imports baseline/current GLBs read-only; never exports, saves, or renders.
"""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
HERE = ROOT / 'crates/alife_game_app/assets/landscape/island'
RENDERERS = ('scripts/render_coastal_leaf_sprays.py', 'scripts/render_fern_fronds_patch.py')


def target_attribute(statement):
    if isinstance(statement, ast.Assign) and len(statement.targets) == 1:
        target = statement.targets[0]
        if isinstance(target, ast.Attribute) and isinstance(target.value, ast.Name) and target.value.id == 'o':
            return target.attr
    return None


def placement_statements(source):
    """Extract the actual contiguous location/mode/yaw/scale statement block."""
    tree = ast.parse(source)
    matches = []
    for node in ast.walk(tree):
        for _, value in ast.iter_fields(node):
            if not isinstance(value, list):
                continue
            for index, statement in enumerate(value):
                if target_attribute(statement) != 'rotation_euler':
                    continue
                if not any(isinstance(child, ast.Subscript)
                           and isinstance(child.value, ast.Name) and child.value.id == 'p'
                           and isinstance(child.slice, ast.Constant) and child.slice.value == 'yaw'
                           for child in ast.walk(statement.value)):
                    continue
                start = index
                while start and target_attribute(value[start]) != 'location':
                    start -= 1
                end = index
                while end + 1 < len(value) and target_attribute(value[end]) != 'scale':
                    end += 1
                block = value[start:end + 1]
                assert target_attribute(block[0]) == 'location', 'Missing placement location'
                assert target_attribute(block[-1]) == 'scale', 'Missing placement scale'
                assert all(target_attribute(row) in {'location', 'rotation_mode', 'rotation_euler', 'scale'}
                           for row in block), 'Unexpected placement statement'
                matches.append(block)
    assert len(matches) == 1, ('Expected one authored-yaw placement block', len(matches))
    return matches[0]


def placement_code(source, remove_mode=False):
    block = placement_statements(source)
    if remove_mode:
        block = [row for row in block if target_attribute(row) != 'rotation_mode']
    return compile(ast.Module(body=block, type_ignores=[]), '<renderer placement statements>', 'exec')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run():
    import bpy
    from mathutils import Matrix
    assert bpy.app.version == (4, 3, 2), 'Use reviewed Blender 4.3.2'
    bpy.ops.wm.read_factory_settings(use_empty=True)
    props = json.loads((HERE / 'props.json').read_text())
    files = [('Fern', 'baseline', HERE / 'fern-connected-fronds/Fern.baseline.glb'),
             ('Fern', 'candidate', HERE / 'Fern.glb'),
             ('CoastalShrub', 'baseline', HERE / 'coastal-leaf-sprays/CoastalShrub.baseline.glb'),
             ('CoastalShrub', 'candidate', HERE / 'CoastalShrub.glb')]
    inputs = {path.relative_to(ROOT).as_posix(): sha(path) for _, _, path in files}
    rows = []
    error = lambda a, b: max(abs(a[r][c] - b[r][c]) for r in range(4) for c in range(4))
    for species, variant, path in files:
        old = set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=str(path))
        imported = [o for o in bpy.data.objects if o not in old and o.type == 'MESH']
        assert len(imported) == 3, ('Expected all three plant LODs', path)
        placement = next(p for p in props if p['kind'] == species and abs(p['yaw']) > .3)
        scale = placement['scale']
        expected = (Matrix.Translation((3, -7, 2.25))
                    @ Matrix.Rotation(placement['yaw'], 4, 'Z')
                    @ Matrix.Diagonal((scale, scale, scale, 1)))
        no_yaw = Matrix.Translation((3, -7, 2.25)) @ Matrix.Diagonal((scale, scale, scale, 1))
        for src in imported:
            assert src.rotation_mode == 'QUATERNION', ('Import mode changed; re-review fixture', src.name)
            for renderer in RENDERERS:
                source = (ROOT / renderer).read_text()
                outcomes = {}
                for label, remove_mode in [('corrected', False), ('historical_negative_control', True)]:
                    obj = src.copy()
                    bpy.context.scene.collection.objects.link(obj)
                    namespace = dict(o=obj, x=3, y=99, z=7, ground_height=lambda x, z: 2.25,
                                     kind=species, cover={'Fern', 'CoastalShrub'}, p=placement)
                    exec(placement_code(source, remove_mode), namespace)
                    bpy.context.view_layer.update()
                    actual = obj.matrix_world.copy()
                    outcomes[label] = {'rotation_mode': obj.rotation_mode,
                                       'matrix': [list(row) for row in actual],
                                       'error_vs_authored_yaw': error(actual, expected),
                                       'error_vs_no_yaw': error(actual, no_yaw)}
                    bpy.data.objects.remove(obj, do_unlink=True)
                assert outcomes['corrected']['rotation_mode'] == 'XYZ', 'Renderer left inactive Euler mode'
                assert outcomes['corrected']['error_vs_authored_yaw'] < 2e-6, 'Renderer ignored authored yaw'
                assert outcomes['historical_negative_control']['rotation_mode'] == 'QUATERNION'
                assert outcomes['historical_negative_control']['error_vs_no_yaw'] < 2e-6
                assert outcomes['historical_negative_control']['error_vs_authored_yaw'] > .1, \
                    'Negative control no longer distinguishes the original bug'
                rows.append({'renderer': renderer, 'species': species, 'variant': variant,
                             'mesh': src.name, 'import_mode': src.rotation_mode,
                             'authored_yaw_rad': placement['yaw'], 'uniform_scale': scale,
                             'expected_matrix': [list(row) for row in expected], 'outcomes': outcomes})
        for obj in imported:
            bpy.data.objects.remove(obj, do_unlink=True)
    assert len(rows) == 24
    assert all(sha(ROOT / name) == digest for name, digest in inputs.items()), 'Input asset changed'
    return {'result': 'PASS', 'blender': bpy.app.version_string,
            'checks': len(rows), 'historical_negative_controls': len(rows),
            'maximum_corrected_matrix_error': max(row['outcomes']['corrected']['error_vs_authored_yaw']
                                                  for row in rows),
            'input_sha256': inputs, 'renderer_sha256': {name: sha(ROOT / name) for name in RENDERERS},
            'proofs': rows, 'rendered': False, 'asset_or_native_written': False,
            'combined_visual_composition_review': False, 'physical_gpu': False}


if __name__ == '__main__':
    sys.dont_write_bytecode = True
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    result = run()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: value for key, value in result.items() if key != 'proofs'}))

"""CoastalShrub successor contracts and retained historical negative controls.

Focused temporary fixtures avoid cloning the complete art pack for each fault.
Geometry mutations bypass the fixed file hash, so mesh checks are exercised.
"""
import copy
from collections import Counter
import json
import math
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import check_coastal_leaf_sprays as check


class CoastalLeafSprayTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = (check.ROOT / check.RUNTIME).read_bytes()
        size = struct.unpack_from('<I', cls.raw, 12)[0]
        cls.doc = json.loads(cls.raw[20:20 + size])
        cls.binary = cls.raw[28 + size:]
        cls.before = json.loads((check.ROOT / check.PREDECESSOR).read_text())

    def profile(self):
        return json.loads((check.ROOT / check.PROFILE).read_text())

    def mutated(self, edit_doc=None, edit_binary=None):
        doc, binary = copy.deepcopy(self.doc), bytearray(self.binary)
        if edit_doc:
            edit_doc(doc)
        if edit_binary:
            edit_binary(doc, binary)
        metadata = json.dumps(doc, separators=(',', ':')).encode()
        metadata += b' ' * (-len(metadata) % 4)
        return (struct.pack('<III', 0x46546c67, 2, 28 + len(metadata) + len(binary)) +
                struct.pack('<II', len(metadata), 0x4e4f534a) + metadata +
                struct.pack('<II', len(binary), 0x004e4942) + binary)

    def rejects_geometry(self, raw, message=None):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'CoastalShrub.glb'
            path.write_bytes(raw)
            context = self.assertRaisesRegex(AssertionError, message) if message else self.assertRaises(AssertionError)
            with context:
                check.geometry(path)

    @staticmethod
    def offset(doc, level, semantic):
        primitive = doc['meshes'][level]['primitives'][0]
        index = primitive['indices'] if semantic == 'indices' else primitive['attributes'][semantic]
        accessor = doc['accessors'][index]
        view = doc['bufferViews'][accessor['bufferView']]
        return accessor, view.get('byteOffset', 0) + accessor.get('byteOffset', 0)

    def test_complete_art_manifest_and_source_audit(self):
        result = check.check()
        self.assertEqual([r['triangles'] for r in result['lods']], [672, 204, 218])
        self.assertEqual([r['vertices'] for r in result['lods']], [600, 276, 163])
        self.assertEqual(result['runtime_bytes'], 38552)
        self.assertEqual(result['manifest_entries_valid'], 48)
        self.assertEqual(result['other_manifest_records_retained'], 47)
        self.assertEqual(result['coastal_shrub_placements'], 170)
        self.assertEqual(result['retained_broadleaf_triangles'], [4462, 916, 86])
        self.assertTrue(result['l2_attributes_and_indices_retained'])
        self.assertFalse(result['game_rendering_proof'])
        self.assertFalse(result['physical_gpu_proof'])

    def test_clean_stdlib_only_audit(self):
        result = subprocess.run([sys.executable, '-S', str(check.ROOT / 'scripts/check_coastal_leaf_sprays.py')],
                                capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['result'], 'PASS')

    def test_optimized_python_cannot_disable_checks(self):
        result = subprocess.run([sys.executable, '-O', str(check.ROOT / 'scripts/check_coastal_leaf_sprays.py')],
                                capture_output=True, text=True, check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('require normal Python mode', result.stderr)

    def test_rejects_extra_texture(self):
        self.rejects_geometry(self.mutated(lambda d: d.update(textures=[{'source': 0}])))

    def test_rejects_material_change(self):
        self.rejects_geometry(self.mutated(lambda d: d['materials'][0].update(alphaMode='BLEND')), 'Binding changed')

    def test_rejects_node_transform(self):
        self.rejects_geometry(self.mutated(lambda d: d['nodes'][0].update(translation=[0, 1, 0])), 'Binding changed')

    def test_rejects_scene_binding_change(self):
        self.rejects_geometry(self.mutated(lambda d: d['scenes'][0].update(nodes=[0, 1])), 'Binding changed')

    def test_rejects_lod_binding_change(self):
        self.rejects_geometry(self.mutated(lambda d: d['meshes'][0].update(name='Wrong_L0')), 'LOD binding')

    def test_rejects_extra_primitive(self):
        self.rejects_geometry(self.mutated(lambda d: d['meshes'][0]['primitives'].append(
            copy.deepcopy(d['meshes'][0]['primitives'][0]))), 'One primitive')

    def test_rejects_nontriangle_mode(self):
        self.rejects_geometry(self.mutated(lambda d: d['meshes'][0]['primitives'][0].update(mode=1)))

    def test_rejects_nonfinite_position(self):
        def edit(doc, binary):
            _, offset = self.offset(doc, 0, 'POSITION')
            struct.pack_into('<f', binary, offset, math.nan)
        self.rejects_geometry(self.mutated(edit_binary=edit), 'Nonfinite geometry')

    def test_rejects_nonfinite_normal(self):
        def edit(doc, binary):
            _, offset = self.offset(doc, 0, 'NORMAL')
            struct.pack_into('<f', binary, offset, math.inf)
        self.rejects_geometry(self.mutated(edit_binary=edit), 'Nonfinite geometry')

    def test_rejects_nonunit_normal(self):
        def edit(doc, binary):
            _, offset = self.offset(doc, 0, 'NORMAL')
            struct.pack_into('<fff', binary, offset, 0, 0, 0)
        self.rejects_geometry(self.mutated(edit_binary=edit), 'Nonunit normal')

    def test_rejects_changed_bounds(self):
        def edit(doc, binary):
            _, offset = self.offset(doc, 0, 'POSITION')
            struct.pack_into('<f', binary, offset, 2)
        self.rejects_geometry(self.mutated(edit_binary=edit), 'LOD bounds changed')

    def test_rejects_false_accessor_bounds(self):
        def edit(doc):
            a, _ = self.offset(doc, 0, 'POSITION')
            a['max'][0] += .01
        self.rejects_geometry(self.mutated(edit_doc=edit), 'Accessor bounds changed')

    def test_rejects_degenerate_triangle(self):
        def edit(doc, binary):
            a, offset = self.offset(doc, 0, 'indices')
            fmt = {5121: 'B', 5123: 'H', 5125: 'I'}[a['componentType']]
            indices = list(struct.unpack_from('<' + fmt * a['count'], binary, offset))
            uses = Counter(indices)
            for start in range(0, len(indices), 3):
                if uses[indices[start + 1]] > 1:
                    # Collapse one corner while every exported vertex remains used.
                    indices[start + 1] = indices[start]
                    struct.pack_into('<' + fmt * len(indices), binary, offset, *indices)
                    return
            self.fail('No multiply used corner found')
        self.rejects_geometry(self.mutated(edit_binary=edit), 'Degenerate triangle')

    def test_rejects_out_of_range_index(self):
        def edit(doc, binary):
            a, offset = self.offset(doc, 0, 'indices')
            fmt = {5121: 'B', 5123: 'H', 5125: 'I'}[a['componentType']]
            struct.pack_into('<' + fmt, binary, offset, 65535)
        self.rejects_geometry(self.mutated(edit_binary=edit), 'Unused or invalid')

    def test_rejects_unused_vertex_at_unchanged_counts(self):
        def edit(doc, binary):
            a, offset = self.offset(doc, 0, 'indices')
            fmt = {5121: 'B', 5123: 'H', 5125: 'I'}[a['componentType']]
            indices = list(struct.unpack_from('<' + fmt * a['count'], binary, offset))
            victim = max(indices)
            struct.pack_into('<' + fmt * len(indices), binary, offset,
                             *(0 if i == victim else i for i in indices))
        self.rejects_geometry(self.mutated(edit_binary=edit), 'Unused or invalid')

    def test_rejects_unnormalized_color(self):
        def edit(doc):
            a, _ = self.offset(doc, 0, 'COLOR_0')
            a['normalized'] = False
        self.rejects_geometry(self.mutated(edit_doc=edit), 'Normalized U8 RGB')

    def test_rejects_rgba_color(self):
        def edit(doc):
            a, _ = self.offset(doc, 0, 'COLOR_0')
            a['type'] = 'VEC4'
        self.rejects_geometry(self.mutated(edit_doc=edit), 'Normalized U8 RGB')

    def test_rejects_l2_color_change(self):
        def edit(doc, binary):
            _, offset = self.offset(doc, 2, 'COLOR_0')
            binary[offset] ^= 1
        self.rejects_geometry(self.mutated(edit_binary=edit), 'L2 attribute changed')

    def test_rejects_l2_position_change_with_same_bounds(self):
        def edit(doc, binary):
            a, offset = self.offset(doc, 2, 'POSITION')
            # Find an interior coordinate; bounds and all topology stay fixed.
            for i in range(a['count']):
                x = struct.unpack_from('<f', binary, offset + 12 * i)[0]
                if -.3 < x < .3:
                    struct.pack_into('<f', binary, offset + 12 * i, x + 1e-6)
                    return
            self.fail('No interior position found')
        self.rejects_geometry(self.mutated(edit_binary=edit), 'L2 attribute changed')

    def test_rejects_l2_normal_change_with_unit_length(self):
        def edit(doc, binary):
            _, offset = self.offset(doc, 2, 'NORMAL')
            x, y, z = struct.unpack_from('<fff', binary, offset)
            struct.pack_into('<fff', binary, offset, -x, -y, -z)
        self.rejects_geometry(self.mutated(edit_binary=edit), 'L2 attribute changed')

    def test_rejects_l2_index_winding_change(self):
        def edit(doc, binary):
            a, offset = self.offset(doc, 2, 'indices')
            fmt = {5121: 'B', 5123: 'H', 5125: 'I'}[a['componentType']]
            a, b, c = struct.unpack_from('<' + fmt * 3, binary, offset)
            struct.pack_into('<' + fmt * 3, binary, offset, a, c, b)
        self.rejects_geometry(self.mutated(edit_binary=edit), 'L2 indices changed')

    def test_rejects_truncated_glb(self):
        self.rejects_geometry(self.raw[:-4], 'Invalid GLB header')

    def test_rejects_changed_fixed_profile(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / check.PROFILE).parent.mkdir(parents=True)
            (root / check.PROFILE).write_bytes((check.ROOT / check.PROFILE).read_bytes() + b' ')
            with self.assertRaisesRegex(AssertionError, 'Fixed CoastalShrub profile changed'):
                check.check(root)

    def test_rejects_changed_historical_profiles(self):
        for changed, message in ((check.PREDECESSOR, 'Historical Broadleaf'),
                                 (check.CONSOLIDATED, 'Historical consolidated')):
            with self.subTest(profile=changed), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for name in (check.PROFILE, check.PREDECESSOR, check.CONSOLIDATED):
                    (root / name).parent.mkdir(parents=True, exist_ok=True)
                    (root / name).write_bytes((check.ROOT / name).read_bytes() + (b' ' if name == changed else b''))
                with self.assertRaisesRegex(AssertionError, message):
                    check.load_profiles(root)

    def test_rejects_changed_selected_and_imported_sources(self):
        for changed in ('selected', 'imported'):
            with self.subTest(tree=changed), tempfile.TemporaryDirectory() as directory:
                selected, imported = Path(directory) / 'selected', Path(directory) / 'imported'
                selected.mkdir()
                imported.mkdir()
                for label, tree in (('selected', selected), ('imported', imported)):
                    (tree / 'helper.py').write_bytes(b'changed' if label == changed else b'original')
                with self.assertRaisesRegex(AssertionError, 'Source changed'):
                    check.validate_source_trees(selected, {'helper.py': check.sha(b'original')}, [imported])

    def test_rejects_source_parent_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'real').mkdir()
            (root / 'real/helper.py').write_bytes(b'original')
            try:
                (root / 'alias').symlink_to(root / 'real', target_is_directory=True)
            except OSError:
                self.skipTest('Symlinks unavailable on this platform')
            with self.assertRaisesRegex(AssertionError, 'Source symlink forbidden'):
                check.validate_source(root, 'alias/helper.py', check.sha(b'original'))

    def test_rejects_missing_retained_helper_pin(self):
        profile = self.profile()
        profile['retained_sources'].pop('scripts/check_island_art.py')
        consolidated = json.loads((check.ROOT / check.CONSOLIDATED).read_text())
        with self.assertRaisesRegex(AssertionError, 'Historical source inventory changed'):
            check.validate_profile_sources(profile, self.before, consolidated)

    def test_rejects_changed_unrelated_art_record(self):
        profile = self.profile()
        record = next(r for r in profile['expected_art_files'] if r['path'].endswith('/Fern.glb'))
        record['sha256'] = '0' * 64
        with self.assertRaisesRegex(AssertionError, 'Unrelated art profile changed'):
            check.validate_retention(profile, self.before)

    def test_rejects_removed_historical_art_record(self):
        profile = self.profile()
        profile['expected_art_files'] = [r for r in profile['expected_art_files'] if not r['path'].endswith('/Fern.glb')]
        with self.assertRaisesRegex(AssertionError, 'Art path retention changed'):
            check.validate_retention(profile, self.before)

    def test_rejects_extra_art_record(self):
        profile = self.profile()
        profile['expected_art_files'].append(dict(profile['expected_art_files'][0], path='unexpected.glb'))
        with self.assertRaisesRegex(AssertionError, 'Art path retention changed'):
            check.validate_retention(profile, self.before)

    def test_rejects_replaced_baseline(self):
        profile = self.profile()
        record = next(r for r in profile['expected_art_files'] if r['path'] == check.BASELINE.as_posix())
        record['sha256'] = check.RUNTIME_SHA256
        with self.assertRaisesRegex(AssertionError, 'Original CoastalShrub not retained'):
            check.validate_retention(profile, self.before)

    def test_rejects_changed_native_asset(self):
        profile = self.profile()
        record = next(r for r in profile['expected_art_files'] if r['path'] == check.SOURCE.as_posix())
        record['sha256'] = '0' * 64
        with self.assertRaisesRegex(AssertionError, 'Approved native source changed'):
            check.validate_retention(profile, self.before)

    def test_art_tree_path_bytes_mode_and_symlink_negative_controls(self):
        for fault, message in (('extra', 'path set changed'), ('missing', 'path set changed'),
                               ('bytes', 'Art bytes changed'), ('mode', 'Art mode changed'),
                               ('symlink', 'Art symlink forbidden')):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                path = root / check.ART_ROOTS[0] / 'retained.bin'
                path.parent.mkdir(parents=True)
                (root / check.ART_ROOTS[1]).mkdir(parents=True)
                path.write_bytes(b'original')
                record = {'path': path.relative_to(root).as_posix(), 'bytes': 8,
                          'sha256': check.sha(b'original'), 'executable': False}
                if fault == 'extra':
                    path.with_name('extra.bin').write_bytes(b'extra')
                elif fault == 'missing':
                    path.unlink()
                elif fault == 'bytes':
                    path.write_bytes(b'altered!')
                elif fault == 'mode':
                    path.chmod(0o755)
                else:
                    original = root / 'original.bin'
                    path.rename(original)
                    try:
                        path.symlink_to(original)
                    except OSError:
                        continue
                with self.assertRaisesRegex(AssertionError, message):
                    check.validate_files(root, [record])

    def test_rejects_unrelated_specification_change(self):
        profile = self.profile()
        spec = copy.deepcopy(profile['baseline_specification'])
        spec['assets']['CoastalShrub'] = profile['coastal_specification']
        spec['assets']['BerryBush']['near_triangles'] += 1
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / check.SPECIFICATION).parent.mkdir(parents=True)
            (root / check.SPECIFICATION).write_text(json.dumps(spec))
            with self.assertRaisesRegex(AssertionError, 'Unrelated specification'):
                check.validate_specification(root, profile)

    def test_rejects_changed_baseline_specification(self):
        profile = self.profile()
        profile['baseline_specification']['assets']['Fern']['near_triangles'] += 1
        with self.assertRaisesRegex(AssertionError, 'Historical specification changed'):
            check.validate_specification(check.ROOT, profile)

    def test_rejects_changed_shrub_dimensions(self):
        profile = self.profile()
        profile['coastal_specification']['width_depth_height_m'][0] += .1
        with self.assertRaises(AssertionError):
            check.validate_specification(check.ROOT, profile)

    def test_rejects_changed_placement_count(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / check.PLACEMENTS).parent.mkdir(parents=True)
            (root / check.PLACEMENTS).write_text(json.dumps([{'kind': 'CoastalShrub'}] * 169))
            with self.assertRaisesRegex(AssertionError, 'placement count changed'):
                check.validate_placements(root)

    def test_same_count_placement_motion_is_rejected_by_retention(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / check.PLACEMENTS
            path.parent.mkdir(parents=True)
            (root / check.ART_ROOTS[1]).mkdir(parents=True)
            props = [{'kind': 'CoastalShrub', 'position': [0, 0, 0]} for _ in range(170)]
            raw = json.dumps(props).encode()
            record = {'path': check.PLACEMENTS.as_posix(), 'bytes': len(raw),
                      'sha256': check.sha(raw), 'executable': False}
            props[0]['position'][0] = 1
            path.write_text(json.dumps(props))
            self.assertEqual(check.validate_placements(root), 170)
            with self.assertRaisesRegex(AssertionError, 'Art bytes changed'):
                check.validate_files(root, [record])

    def test_manifest_registration_and_payload_negative_controls(self):
        expected = {'entries': [{'local_path': f'asset-{i}.bin', 'size_bytes': 8,
                                'digest': check.predecessor.digest_payload('', b'original')}
                               for i in range(48)]}
        for fault, message in (('changed', 'Unreviewed manifest changes'),
                               ('missing', '48 unique registrations'),
                               ('duplicate', '48 unique registrations'),
                               ('payload', 'Manifest digest mismatch')):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                actual = copy.deepcopy(expected)
                if fault == 'changed':
                    actual['entries'][1]['size_bytes'] += 1
                elif fault == 'missing':
                    actual['entries'].pop()
                elif fault == 'duplicate':
                    actual['entries'][1] = copy.deepcopy(actual['entries'][0])
                else:
                    for entry in actual['entries']:
                        (root / entry['local_path']).write_bytes(b'original')
                    (root / 'asset-0.bin').write_bytes(b'altered!')
                with self.assertRaisesRegex(AssertionError, message):
                    check.validate_manifest_records(root, actual, expected)


def load_tests(loader, tests, pattern):
    # Earlier full-art positive tests describe a frozen historical snapshot.
    # Retain their negative geometry/profile cases against unchanged Broadleaf.
    import test_broadleaf_canopy as historical
    names = [name for name in loader.getTestCaseNames(historical.BroadleafCanopyTests)
             if name.startswith('test_rejects_')]
    tests.addTests(historical.BroadleafCanopyTests(name) for name in names)
    return tests


if __name__ == '__main__':
    unittest.main()

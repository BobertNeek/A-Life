"""Positive/negative Fern successor tests, retaining historical Broadleaf geometry tests."""
import copy
import hashlib
import json
import math
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import check_fern_fronds as check


class FernFrondTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = (check.ROOT / check.RUNTIME).read_bytes()
        size = struct.unpack_from('<I', cls.raw, 12)[0]
        cls.doc = json.loads(cls.raw[20:20 + size])
        cls.binary = cls.raw[28 + size:]

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

    def rejects(self, raw):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'Fern.glb'
            path.write_bytes(raw)
            with self.assertRaises(AssertionError):
                check.geometry(path)

    def test_complete_art_manifest_and_source_audit(self):
        result = check.check()
        self.assertEqual([r['triangles'] for r in result['lods']], [700, 294, 154])
        self.assertEqual(result['other_manifest_records_retained'], 47)
        self.assertEqual(result['retained_broadleaf_triangles'], [4462, 916, 86])
        self.assertFalse(result['game_rendering_proof'])

    def test_clean_stdlib_only_audit(self):
        result = subprocess.run([sys.executable, '-S', str(check.ROOT / 'scripts/check_fern_fronds.py')],
                                capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['result'], 'PASS')

    def test_rejects_changed_fixed_profile(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / check.PROFILE).parent.mkdir(parents=True)
            (root / check.PROFILE).write_bytes((check.ROOT / check.PROFILE).read_bytes() + b' ')
            with self.assertRaisesRegex(AssertionError, 'Fixed Fern profile changed'):
                check.check(root)

    def test_rejects_changed_pinned_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'source.blend').write_bytes(b'changed')
            with self.assertRaisesRegex(AssertionError, 'Source changed'):
                check.validate_source(root, 'source.blend', hashlib.sha256(b'original').hexdigest())

    def test_rejects_source_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'source.blend').write_bytes(b'original')
            try:
                (root / 'alias.blend').symlink_to(root / 'source.blend')
            except OSError:
                self.skipTest('Symlinks unavailable on this platform')
            with self.assertRaisesRegex(AssertionError, 'Source symlink forbidden'):
                check.validate_source(root, 'alias.blend', hashlib.sha256(b'original').hexdigest())

    def test_rejects_extra_texture(self):
        self.rejects(self.mutated(lambda d: d.update(textures=[{'source': 0}])))

    def test_rejects_material_change(self):
        self.rejects(self.mutated(lambda d: d['materials'][0].update(alphaMode='BLEND')))

    def test_rejects_node_transform(self):
        self.rejects(self.mutated(lambda d: d['nodes'][0].update(translation=[0, 1, 0])))

    def test_rejects_lod_binding_change(self):
        self.rejects(self.mutated(lambda d: d['meshes'][0].update(name='Wrong_L0')))

    def test_rejects_nonfinite_position(self):
        def edit(doc, binary):
            a = doc['accessors'][doc['meshes'][0]['primitives'][0]['attributes']['POSITION']]
            v = doc['bufferViews'][a['bufferView']]
            struct.pack_into('<f', binary, v.get('byteOffset', 0) + a.get('byteOffset', 0), math.nan)
        self.rejects(self.mutated(edit_binary=edit))

    def test_rejects_degenerate_triangle(self):
        def edit(doc, binary):
            a = doc['accessors'][doc['meshes'][0]['primitives'][0]['indices']]
            v = doc['bufferViews'][a['bufferView']]
            fmt = {5121: 'B', 5123: 'H', 5125: 'I'}[a['componentType']]
            struct.pack_into('<' + fmt * 3, binary, v.get('byteOffset', 0), 0, 0, 0)
        self.rejects(self.mutated(edit_binary=edit))

    def test_rejects_unnormalized_color(self):
        def edit(doc):
            a = doc['meshes'][0]['primitives'][0]['attributes']['COLOR_0']
            doc['accessors'][a]['normalized'] = False
        self.rejects(self.mutated(edit_doc=edit))

    def test_rejects_unused_vertex(self):
        def edit(doc):
            a = doc['meshes'][0]['primitives'][0]['indices']
            doc['accessors'][a]['count'] -= 3
        self.rejects(self.mutated(edit_doc=edit))


def load_tests(loader, tests, pattern):
    # The earlier full-art positive tests belong to their pinned snapshot.
    # Keep their geometry-negative cases running against retained Broadleaf.
    import test_broadleaf_canopy as historical
    names = [name for name in loader.getTestCaseNames(historical.BroadleafCanopyTests)
             if name.startswith('test_rejects_')]
    tests.addTests(historical.BroadleafCanopyTests(name) for name in names)
    return tests


if __name__ == '__main__':
    unittest.main()

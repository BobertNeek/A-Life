"""Focused positive/negative tests for the fixed Broadleaf successor contract."""
import copy
import json
import math
import struct
import tempfile
import unittest
from pathlib import Path

import check_broadleaf_canopy as check


class BroadleafCanopyTests(unittest.TestCase):
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
            path = Path(directory) / 'Broadleaf.glb'
            path.write_bytes(raw)
            with self.assertRaises(AssertionError):
                check.geometry(path)

    def test_complete_successor_audit(self):
        result = check.check()
        self.assertEqual(result['triangles'], [4462, 916, 86])
        self.assertEqual(result['other_manifest_records_retained'], 47)
        self.assertFalse(result['game_rendering_proof'])

    def test_rejects_extra_texture(self):
        self.rejects(self.mutated(lambda doc: doc.update(textures=[{'source': 0}])))

    def test_rejects_alpha_blending(self):
        self.rejects(self.mutated(lambda doc: doc['materials'][0].update(alphaMode='BLEND')))

    def test_rejects_node_translation(self):
        self.rejects(self.mutated(lambda doc: doc['nodes'][0].update(translation=[0, 1, 0])))

    def test_rejects_wrong_lod_name(self):
        self.rejects(self.mutated(lambda doc: doc['meshes'][0].update(name='NotBroadleaf_L0')))

    def test_rejects_nonfinite_position(self):
        def mutate(doc, binary):
            accessor = doc['accessors'][doc['meshes'][0]['primitives'][0]['attributes']['POSITION']]
            view = doc['bufferViews'][accessor['bufferView']]
            offset = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
            struct.pack_into('<f', binary, offset, math.nan)
        self.rejects(self.mutated(edit_binary=mutate))

    def test_rejects_degenerate_triangle(self):
        def mutate(doc, binary):
            accessor = doc['accessors'][doc['meshes'][0]['primitives'][0]['indices']]
            view = doc['bufferViews'][accessor['bufferView']]
            offset = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
            fmt = {5121: 'B', 5123: 'H', 5125: 'I'}[accessor['componentType']]
            struct.pack_into('<' + fmt * 3, binary, offset, 0, 0, 0)
        self.rejects(self.mutated(edit_binary=mutate))

    def test_rejects_changed_fixed_profile(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / check.PROFILE).parent.mkdir(parents=True)
            (root / check.PROFILE).write_bytes((check.ROOT / check.PROFILE).read_bytes() + b' ')
            with self.assertRaisesRegex(AssertionError, 'Fixed Broadleaf profile changed'):
                check.check(root)


if __name__ == '__main__':
    unittest.main()

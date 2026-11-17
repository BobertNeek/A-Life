"""Color compaction must never reinterpret another consumer's buffer bytes."""
import copy
import json
import struct
import tempfile
import unittest
from pathlib import Path

from compact_glb_colors import compact_colors


def write_glb(path, doc, binary):
    metadata = json.dumps(doc).encode()
    metadata += b' ' * (-len(metadata) % 4)
    binary += b'\0' * (-len(binary) % 4)
    path.write_bytes(
        struct.pack('<III', 0x46546c67, 2, 28 + len(metadata) + len(binary))
        + struct.pack('<II', len(metadata), 0x4e4f534a) + metadata
        + struct.pack('<II', len(binary), 0x004e4942) + binary)


class ColorCompactionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / 'fixture.glb'
        self.binary = struct.pack('<4f', 0.0, 0.5, 1.0, 1.0)
        self.doc = {
            'asset': {'version': '2.0'},
            'buffers': [{'byteLength': len(self.binary)}],
            'bufferViews': [{'buffer': 0, 'byteOffset': 0, 'byteLength': 16}],
            'accessors': [{'bufferView': 0, 'componentType': 5126,
                           'type': 'VEC4', 'count': 1}],
            'meshes': [{'primitives': [{'attributes': {'COLOR_0': 0}}]}],
        }

    def assert_rejected_unchanged(self):
        write_glb(self.path, self.doc, self.binary)
        original = self.path.read_bytes()
        with self.assertRaises(ValueError):
            compact_colors(self.path)
        self.assertEqual(self.path.read_bytes(), original)

    def test_other_accessor_sharing_color_view_is_not_corrupted(self):
        self.doc['accessors'].append(copy.deepcopy(self.doc['accessors'][0]))
        self.doc['meshes'][0]['primitives'][0]['attributes']['TANGENT'] = 1
        self.assert_rejected_unchanged()

    def test_same_accessor_used_as_tangent_is_not_reinterpreted(self):
        self.doc['meshes'][0]['primitives'][0]['attributes']['TANGENT'] = 0
        self.assert_rejected_unchanged()

    def test_shared_image_view_is_not_rewritten(self):
        self.doc['images'] = [{'bufferView': 0, 'mimeType': 'image/png'}]
        self.assert_rejected_unchanged()

    def test_other_accessor_sparse_payload_is_not_rewritten(self):
        self.doc['accessors'].append({
            'componentType': 5126, 'type': 'VEC4', 'count': 1,
            'sparse': {'count': 1, 'indices': {'bufferView': 0, 'componentType': 5121},
                       'values': {'bufferView': 0}}})
        self.assert_rejected_unchanged()

    def test_sparse_color_is_rejected_instead_of_losing_overrides(self):
        self.doc['accessors'][0]['sparse'] = {
            'count': 1, 'indices': {'bufferView': 0, 'componentType': 5121},
            'values': {'bufferView': 0}}
        self.assert_rejected_unchanged()

    def test_exclusive_color_compacts_and_is_idempotent(self):
        write_glb(self.path, self.doc, self.binary)
        compact_colors(self.path)
        raw = self.path.read_bytes()
        size = struct.unpack_from('<I', raw, 12)[0]
        doc = json.loads(raw[20:20 + size])
        self.assertEqual(doc['accessors'][0]['componentType'], 5121)
        self.assertTrue(doc['accessors'][0]['normalized'])
        self.assertEqual(raw[28 + size:], bytes([0, 128, 255, 255]))
        compact_colors(self.path)
        self.assertEqual(self.path.read_bytes(), raw)

    def test_shared_accessor_used_by_two_primitives_is_safe(self):
        self.doc['meshes'][0]['primitives'].append({'attributes': {'COLOR_0': 0}})
        write_glb(self.path, self.doc, self.binary)
        compact_colors(self.path)
        raw = self.path.read_bytes()
        size = struct.unpack_from('<I', raw, 12)[0]
        self.assertEqual(raw[28 + size:], bytes([0, 128, 255, 255]))


if __name__ == '__main__':
    unittest.main()

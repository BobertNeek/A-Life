"""Focused source/baseline and combined-profile negative regression tests."""
import copy
import json
import shutil
import tempfile
import unittest
from pathlib import Path

import check_finished_island_assets as finished
import check_consolidated_island_art as consolidated


class FinishedAssetTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        lineage = json.loads((finished.ROOT / finished.SOURCE / 'lineage.json').read_text())
        self.asset = next(a for a in lineage['assets'] if a['name'] == 'Flowers')
        files = [finished.SOURCE / self.asset[k]['file']
                 for k in ('preintegration', 'source_pack_baseline', 'editable_source')]
        files.append(Path(self.asset['runtime_path']))
        for relative in files:
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(finished.ROOT / relative, target)

    def corrupt(self, relative):
        path = self.root / relative
        raw = path.read_bytes()
        path.write_bytes(raw[:-1] + bytes([raw[-1] ^ 1]))

    def test_exact_source_aware_flower_contract(self):
        result = finished.check_asset(self.root, self.asset)
        self.assertEqual(len(result['lods']), 3)
        self.assertTrue(result['installed_baseline_reconciled'])

    def test_same_size_runtime_mutation_rejected(self):
        self.corrupt(self.asset['runtime_path'])
        with self.assertRaisesRegex(AssertionError, 'Exact asset bytes differ'):
            finished.check_asset(self.root, self.asset)

    def test_installed_baseline_guard_remains_required(self):
        self.corrupt(finished.SOURCE / self.asset['preintegration']['file'])
        with self.assertRaisesRegex(AssertionError, 'Exact asset bytes differ'):
            finished.check_asset(self.root, self.asset)

    def test_source_pack_guard_remains_required(self):
        self.corrupt(finished.SOURCE / self.asset['source_pack_baseline']['file'])
        with self.assertRaisesRegex(AssertionError, 'Exact asset bytes differ'):
            finished.check_asset(self.root, self.asset)

    def test_native_source_mutation_rejected(self):
        self.corrupt(finished.SOURCE / self.asset['editable_source']['file'])
        with self.assertRaisesRegex(AssertionError, 'Editable source changed'):
            finished.check_asset(self.root, self.asset)


class CombinedManifestTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.base = {'schema': 'fixture', 'entries': []}
        self.lineage = {'assets': []}
        for i in range(48):
            name = 'fixture/' + str(i) + '.glb'
            raw = ('reviewed:' + name).encode()
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw)
            entry = {'asset_id': 'fixture-' + str(i), 'local_path': name,
                     'size_bytes': len(raw), 'digest': finished.digest(raw),
                     'generator': {'date': 'base'}, 'license': 'retained'}
            self.base['entries'].append(entry)
            if i < 6:
                self.lineage['assets'].append({'name': 'Asset' + str(i),
                    'asset_id': entry['asset_id'], 'runtime_path': name,
                    'runtime': {'bytes': len(raw), 'fnv1a64': finished.digest(raw)}})
        self.profile = {'base_manifest': self.base}
        self.actual = copy.deepcopy(self.base)
        for i, asset in enumerate(self.lineage['assets']):
            self.actual['entries'][i]['generator'] = {
                'config_path': 'scripts/build_finished_island_assets.py',
                'date': '2026-10-09', 'seed': 'saved-finished-island-refinements/' + asset['name'],
                'tool': 'Blender-4.3.2'}
        self.write_manifest()

    def write_manifest(self):
        path = self.root / consolidated.historical.MANIFEST
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.actual))

    def validate(self):
        return consolidated.validate_manifest(self.root, self.profile, self.lineage)

    def test_exact_six_delta_preserves_42_complete_records(self):
        result = self.validate()
        self.assertEqual(result['other_complete_registrations_retained'], 42)

    def test_unrelated_registration_change_rejected(self):
        self.actual['entries'][-1]['generator'] = {'changed': True}
        self.write_manifest()
        with self.assertRaisesRegex(AssertionError, 'Manifest differs'):
            self.validate()

    def test_changed_stable_asset_id_rejected(self):
        self.actual['entries'][0]['asset_id'] = 'wrong-id'
        self.write_manifest()
        with self.assertRaisesRegex(AssertionError, 'Manifest differs'):
            self.validate()

    def test_duplicate_registration_rejected(self):
        self.actual['entries'][-1] = copy.deepcopy(self.actual['entries'][0])
        self.write_manifest()
        with self.assertRaisesRegex(AssertionError, 'Registration set changed'):
            self.validate()

    def test_same_size_unrelated_payload_mutation_rejected(self):
        path = self.root / self.actual['entries'][-1]['local_path']
        raw = path.read_bytes()
        path.write_bytes(bytes([raw[0] ^ 1]) + raw[1:])
        with self.assertRaisesRegex(AssertionError, 'Manifest digest mismatch'):
            self.validate()

    def test_art_snapshot_rejects_unknown_addition(self):
        art_root = self.root / 'isolated-art-tree'
        directory = Path(consolidated.historical.ART_ROOTS[0])
        path = art_root / directory / 'known.glb'
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b'known')
        records = [{'path': path.relative_to(art_root).as_posix(), 'bytes': 5,
                    'sha256': consolidated.sha(b'known'), 'executable': False}]
        self.assertEqual(consolidated.validate_files(art_root, records), 1)
        (path.parent / 'unknown.glb').write_bytes(b'unknown')
        with self.assertRaisesRegex(AssertionError, 'path set changed'):
            consolidated.validate_files(art_root, records)

    def test_art_snapshot_rejects_symlink(self):
        path = self.root / consolidated.historical.ART_ROOTS[0] / 'link.glb'
        path.parent.mkdir(parents=True, exist_ok=True)
        path.symlink_to(self.root / 'fixture/0.glb')
        with self.assertRaisesRegex(AssertionError, 'symlink forbidden'):
            consolidated.validate_files(self.root, [])


if __name__ == '__main__':
    unittest.main()

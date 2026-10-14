"""Focused CPU regression tests; synthetic fixtures are not game evidence."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import check_composed_island_graphics as composed
import check_island_biological_shrubs as shrubs


def blob_id(payload):
    return hashlib.sha1(b'blob ' + str(len(payload)).encode() + b'\0' + payload).hexdigest()


class ComposedGraphicsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.paths = list(composed.CHANGES) + [
            'crates/alife_game_app/assets/protected-%02d.glb' % i for i in range(44)]
        self.old_payloads = {p: ('baseline:' + p).encode() for p in self.paths}
        self.payloads = {p: (('reviewed:' + p).encode() if p in composed.CHANGES else raw)
                         for p, raw in self.old_payloads.items()}
        self.baseline = {'schema': 'fixture', 'entries': [self.entry(p, self.old_payloads[p]) for p in self.paths]}
        self.reviewed = copy.deepcopy(self.baseline)
        for entry in self.reviewed['entries']:
            p = entry['local_path']
            if p in composed.CHANGES:
                entry.update(digest=composed.digest_payload(p, self.payloads[p]),
                             size_bytes=len(self.payloads[p]), generator={'version': 'reviewed'})
        self.reviewed_payloads = {p: self.payloads[p] for p in composed.CHANGES}
        for p, raw in self.payloads.items():
            self.write(p, raw)
        self.write_manifest(self.reviewed)

    def entry(self, path, payload):
        return {'asset_id': 'fixture-' + str(self.paths.index(path)), 'local_path': path,
                'digest': composed.digest_payload(path, payload), 'size_bytes': len(payload),
                'generator': {'version': 'baseline'}, 'license': 'fixture'}

    def write(self, path, payload):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(payload)

    def write_manifest(self, manifest):
        self.write(composed.MANIFEST, json.dumps(manifest).encode())

    def validate(self):
        return composed.validate_manifest(self.root, self.baseline, self.reviewed, self.reviewed_payloads)

    def test_exact_composition_accepts_four_changes_and_protects_44(self):
        report = self.validate()
        self.assertEqual(report['manifest_entries_valid'], 48)
        self.assertEqual(report['unchanged_registrations'], 44)
        self.assertEqual(report['authorized_runtime_paths'], sorted(composed.CHANGES))

    def test_unrelated_asset_mutation_fails(self):
        self.write(self.paths[-1], b'unrelated mutation')
        with self.assertRaises(AssertionError):
            self.validate()

    def test_unrelated_mutation_and_reregistered_digest_fail(self):
        path, payload = self.paths[-1], b'new unrelated asset'
        self.write(path, payload)
        actual = copy.deepcopy(self.reviewed)
        actual['entries'][-1].update(digest=composed.digest_payload(path, payload), size_bytes=len(payload))
        self.write_manifest(actual)
        with self.assertRaisesRegex(AssertionError, 'Candidate manifest differs'):
            self.validate()

    def test_digest_mismatch_with_same_size_fails(self):
        path = self.paths[-1]
        payload = self.payloads[path]
        self.write(path, bytes([payload[0] ^ 1]) + payload[1:])
        with self.assertRaisesRegex(AssertionError, 'Digest mismatch'):
            self.validate()

    def test_omitted_expected_registration_fails(self):
        actual = copy.deepcopy(self.reviewed)
        actual['entries'].pop(2)
        self.write_manifest(actual)
        with self.assertRaisesRegex(AssertionError, 'Registration set changed'):
            self.validate()

    def test_omitted_expected_asset_file_fails(self):
        (self.root / self.paths[2]).unlink()
        with self.assertRaises(FileNotFoundError):
            self.validate()

    def test_duplicate_registration_fails(self):
        actual = copy.deepcopy(self.reviewed)
        actual['entries'][-1] = copy.deepcopy(actual['entries'][-2])
        self.write_manifest(actual)
        with self.assertRaisesRegex(AssertionError, 'Registration set changed'):
            self.validate()

    def test_changed_id_fails(self):
        actual = copy.deepcopy(self.reviewed)
        actual['entries'][0]['asset_id'] = 'different-id'
        self.write_manifest(actual)
        with self.assertRaisesRegex(AssertionError, 'Candidate manifest differs'):
            self.validate()

    def test_manifest_metadata_change_fails(self):
        actual = copy.deepcopy(self.reviewed)
        actual['schema'] = 'different-schema'
        self.write_manifest(actual)
        with self.assertRaisesRegex(AssertionError, 'Candidate manifest differs'):
            self.validate()

    def test_exact_reviewed_bytes_required_even_with_matching_digest(self):
        # Simulate a digest collision: exact reviewed bytes remain independently checked.
        path = self.paths[0]
        payload = self.payloads[path]
        self.write(path, bytes([payload[0] ^ 1]) + payload[1:])
        with patch.object(composed, 'digest_payload', side_effect=lambda p, raw: next(
                e['digest'] for e in self.reviewed['entries'] if e['local_path'] == p)):
            with self.assertRaisesRegex(AssertionError, 'Not the exact reviewed revision'):
                self.validate()

    def test_source_profile_cannot_omit_a_reviewed_change(self):
        reviewed = copy.deepcopy(self.reviewed)
        reviewed['entries'][2] = copy.deepcopy(self.baseline['entries'][2])
        with self.assertRaisesRegex(AssertionError, 'exceed or omit'):
            composed.validate_manifest(self.root, self.baseline, reviewed, self.reviewed_payloads)

    def test_art_snapshot_rejects_unregistered_mutation_and_addition(self):
        reviewed_tree = {p.relative_to(self.root).as_posix(): ('100644', blob_id(p.read_bytes()))
                         for p in self.root.rglob('*') if p.is_file()}
        self.assertEqual(composed.validate_candidate_tree(self.root, reviewed_tree), 49)
        extra = 'crates/alife_world/assets/unregistered.bin'
        self.write(extra, b'unregistered')
        with self.assertRaisesRegex(AssertionError, 'art paths differ'):
            composed.validate_candidate_tree(self.root, reviewed_tree)
        reviewed_tree[extra] = ('100644', blob_id(b'unregistered'))
        self.write(extra, b'changed')
        with self.assertRaisesRegex(AssertionError, 'Art blob changed'):
            composed.validate_candidate_tree(self.root, reviewed_tree)

    def test_reviewed_tree_scope_is_exact(self):
        baseline = {p: ('100644', blob_id(self.old_payloads[p])) for p in self.paths}
        baseline.update({str(composed.MANIFEST): ('100644', 'old-manifest'),
                         str(composed.HERE / 'README.md'): ('100644', 'old-doc')})
        reviewed = dict(baseline)
        reviewed.update({p: ('100644', blob_id(self.payloads[p])) for p in composed.CHANGES})
        reviewed.update({str(composed.MANIFEST): ('100644', 'new-manifest'),
                         str(composed.HERE / 'README.md'): ('100644', 'new-doc')})
        reviewed.update({p: ('100644', 'new-artifact') for p in composed.NEW_ARTIFACTS})
        composed.validate_reviewed_trees(baseline, reviewed)
        reviewed[self.paths[-1]] = ('100644', 'unexpected-mutation')
        with self.assertRaisesRegex(AssertionError, 'exceed or omit'):
            composed.validate_reviewed_trees(baseline, reviewed)

    def test_standalone_shrub_gate_still_rejects_fern(self):
        # The unmodified historical retention block must run after geometry checks.
        with patch.object(shrubs, 'ROOT', self.root), \
             patch.object(shrubs, 'check_shrub_structures', return_value={}), \
             patch.object(shrubs, 'baseline', side_effect=lambda p: self.old_payloads[str(p)]), \
             patch.object(shrubs.subprocess, 'check_output', return_value='\n'.join(self.paths)):
            with self.assertRaisesRegex(AssertionError, 'Fern.glb'):
                shrubs.check()

    def profile_trees(self):
        self.write(composed.HERE / 'README.md', b'reviewed documentation')
        for name in composed.NEW_ARTIFACTS:
            self.write(name, ('reviewed artifact:' + name).encode())
        reviewed = {p.relative_to(self.root).as_posix(): ('100644', blob_id(p.read_bytes()))
                    for p in self.root.rglob('*') if p.is_file()}
        baseline = {p: value for p, value in reviewed.items() if p not in composed.NEW_ARTIFACTS}
        for p in composed.CHANGES:
            baseline[p] = ('100644', blob_id(self.old_payloads[p]))
        baseline[str(composed.MANIFEST)] = ('100644', blob_id(json.dumps(self.baseline).encode()))
        baseline[str(composed.HERE / 'README.md')] = ('100644', blob_id(b'baseline documentation'))
        return baseline, reviewed

    def mocked_profile(self, shrub_error=None):
        trees = self.profile_trees()
        def reader(root, revision, path):
            if str(path) == str(composed.MANIFEST):
                return json.dumps(self.baseline if revision == composed.BASE else self.reviewed).encode()
            self.assertEqual(revision, composed.CHANGES[str(path)])
            return self.reviewed_payloads[str(path)]
        return (patch.object(composed, 'git_art_tree', side_effect=trees),
                patch.object(composed, 'git_bytes', side_effect=reader),
                patch.object(composed, 'check_shrub_structures', side_effect=shrub_error,
                             return_value={'result': 'fixture-shrubs'}),
                patch.object(composed, 'check_fern', return_value={'result': 'fixture-fern'}),
                patch.object(composed, 'check_sapling', return_value={'result': 'fixture-sapling'}))

    def test_composition_invokes_all_three_structural_checks(self):
        a, b, c, d, e = self.mocked_profile()
        with a, b, c as shrub, d as fern, e as sapling:
            report = composed.check(self.root)
            self.assertEqual(report['result'], 'PASS')
            shrub.assert_called_once()
            fern.assert_called_once_with(self.root / composed.HERE / 'fern-readability/Fern.baseline.glb',
                                         self.root / composed.HERE / 'Fern.glb')
            sapling.assert_called_once_with(self.root / composed.HERE / 'sapling-color/Sapling.baseline.glb',
                                           self.root / composed.HERE / 'Sapling.glb')

    def test_structural_failure_cannot_be_reported_as_pass(self):
        a, b, c, d, e = self.mocked_profile(AssertionError('structure changed'))
        with a, b, c, d, e:
            with self.assertRaisesRegex(AssertionError, 'structure changed'):
                composed.check(self.root)


if __name__ == '__main__':
    unittest.main()

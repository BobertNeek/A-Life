"""Focused CPU regressions; synthetic art/Git fixtures are not render evidence."""
import copy
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import ExitStack
from pathlib import Path
from unittest.mock import patch

import check_composed_conifer_graphics as composed

historical = composed.historical
conifer = composed.conifer


class ComposedConiferTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in {**composed.HISTORICAL_HELPERS, **composed.CONIFER_SCRIPTS}:
            self.write(name, (composed.ROOT / name).read_bytes())
        self.paths = list(historical.CHANGES) + [composed.CONIFER] + [
            'crates/alife_game_app/assets/protected-%02d.glb' % i for i in range(43)]
        self.old_payloads = {p: ('parent:' + p).encode() for p in self.paths}
        self.payloads = dict(self.old_payloads)
        self.payloads[composed.CONIFER] = b'fixture-conifer' + bytes(112240 - len(b'fixture-conifer'))
        self.parent_manifest = {'schema': 'fixture', 'entries': [self.entry(p) for p in self.paths]}
        self.actual_manifest = copy.deepcopy(self.parent_manifest)
        self.actual_manifest['entries'][4].update(
            digest=historical.digest_payload(composed.CONIFER, self.payloads[composed.CONIFER]),
            size_bytes=112240, generator=conifer.GENERATOR)
        self.parent_raw = json.dumps(self.parent_manifest).encode()
        for p, raw in self.payloads.items():
            self.write(p, raw)
        self.write_manifest(self.actual_manifest)
        self.write(historical.HERE / 'README.md', b'fixture-conifer-doc')
        for p in historical.NEW_ARTIFACTS:
            self.write(p, ('fixture-existing-source:' + p).encode())
        for i in range(58):
            self.write('crates/alife_world/assets/unregistered-%02d.bin' % i, b'original source')
        self.parent_tree = self.tree()
        for p in [composed.CONIFER, str(composed.MANIFEST), str(composed.HERE / 'README.md')]:
            old = self.old_payloads.get(p, self.parent_raw if p == str(composed.MANIFEST) else b'parent-doc')
            self.parent_tree[p] = ('100644', composed.blob_id(old))
        self.synthetic_overlay = {}
        for p, (old, _) in composed.ART_OVERLAY.items():
            if old is None:
                self.write(p, ('fixture-new-source:' + p).encode())
            self.synthetic_overlay[p] = (self.parent_tree[p][1] if old is not None else None,
                                         composed.blob_id((self.root / p).read_bytes()))
        self.baseline_tree = {p: v for p, v in self.parent_tree.items() if p not in historical.NEW_ARTIFACTS}
        for p in set(historical.CHANGES) | {str(composed.MANIFEST), str(composed.HERE / 'README.md')}:
            self.baseline_tree[p] = ('100644', 'historical-preimage:' + p)
        self.expected_tree = self.tree()
        self.assertEqual(len(self.parent_tree), 113)
        self.assertEqual(len(self.expected_tree), 115)

    def entry(self, p):
        raw = self.old_payloads[p]
        return {'asset_id': 'approved-island-conifer' if p == composed.CONIFER else 'fixture-' + str(self.paths.index(p)),
                'local_path': p, 'digest': historical.digest_payload(p, raw), 'size_bytes': len(raw),
                'generator': {'version': 'parent'}, 'license': 'retained', 'policy': 'retained'}

    def write(self, p, raw):
        dest = self.root / p
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(raw)

    def write_manifest(self, manifest):
        self.write(composed.MANIFEST, json.dumps(manifest).encode())

    def tree(self):
        return {p.relative_to(self.root).as_posix(): ('100644', composed.blob_id(p.read_bytes()))
                for art in historical.ART_ROOTS for p in (self.root / art).rglob('*') if p.is_file()}

    def manifest_context(self):
        stack = ExitStack()
        stack.enter_context(patch.object(conifer, 'BASE_MANIFEST_SHA256', hashlib.sha256(self.parent_raw).hexdigest()))
        stack.enter_context(patch.object(conifer, 'FIXED_SHA256', hashlib.sha256(self.payloads[composed.CONIFER]).hexdigest()))
        return stack

    def validate_manifest(self):
        with self.manifest_context():
            return composed.validate_manifest(self.root, self.parent_raw)

    def profile_context(self, structural_failure=None):
        stack = self.manifest_context()
        stack.enter_context(patch.object(composed, 'ART_OVERLAY', self.synthetic_overlay))
        stack.enter_context(patch.object(historical, 'git_art_tree', side_effect=[self.baseline_tree, self.parent_tree]))
        stack.enter_context(patch.object(historical, 'git_bytes', return_value=self.parent_raw))
        for target, name in [(historical, 'check_shrub_structures'), (historical, 'check_fern'),
                             (historical, 'check_sapling'), (conifer, 'check')]:
            failure = AssertionError('fixture structure failed') if name == structural_failure else None
            stack.enter_context(patch.object(target, name, return_value={'fixture': name}, side_effect=failure))
        return stack

    def test_exact_composition_checks_115_blobs_and_all_48_payloads(self):
        with self.profile_context():
            report = composed.check(self.root)
        self.assertEqual(report['result'], 'PASS')
        self.assertEqual(report['reviewed_art_blobs_valid'], 115)
        self.assertEqual(report['manifest_entries_valid'], 48)
        self.assertEqual(report['complete_registrations_unchanged_from_pr24'], 47)
        self.assertEqual(report['reviewed_conifer_script_blobs_valid'], 6)

    def test_historical_profile_still_rejects_conifer_overlay(self):
        historical.validate_reviewed_trees(self.baseline_tree, self.parent_tree)
        with self.assertRaisesRegex(AssertionError, 'art paths differ'):
            historical.validate_candidate_tree(self.root, self.parent_tree)

    def test_fixed_parent_preimages_and_added_paths_are_required(self):
        with patch.object(composed, 'ART_OVERLAY', self.synthetic_overlay):
            for p in composed.ART_OVERLAY:
                with self.subTest(path=p):
                    parent = dict(self.parent_tree)
                    parent[p] = ('100644', 'wrong-preimage')
                    # Preserve the required count for the two addition cases.
                    if p not in self.parent_tree:
                        parent.pop('crates/alife_world/assets/unregistered-00.bin')
                    with self.assertRaises(AssertionError):
                        composed.expected_art_tree(parent)

    def test_conifer_omission_baseline_and_same_size_mutation_fail(self):
        for raw in [None, self.old_payloads[composed.CONIFER], b'X' + self.payloads[composed.CONIFER][1:]]:
            with self.subTest(payload='omitted' if raw is None else len(raw)):
                if raw is None:
                    (self.root / composed.CONIFER).unlink()
                else:
                    self.write(composed.CONIFER, raw)
                with self.profile_context(), self.assertRaises((AssertionError, FileNotFoundError)):
                    composed.check(self.root)

    def test_fake_matching_fnv_cannot_hide_conifer_byte_mutation(self):
        self.write(composed.CONIFER, b'X' + self.payloads[composed.CONIFER][1:])
        with self.profile_context(), patch.object(historical, 'digest_payload', return_value='fnv1a64:fake'):
            with self.assertRaisesRegex(AssertionError, 'Art blob changed'):
                composed.check(self.root)

    def test_previous_reviewed_runtime_assets_and_original_sources_remain_exact(self):
        paths = list(historical.CHANGES) + [str(composed.HERE / 'island-biological-shrubs.blend')]
        for p in paths:
            with self.subTest(path=p):
                old = (self.root / p).read_bytes()
                self.write(p, b'changed previously reviewed art')
                try:
                    with self.profile_context(), self.assertRaisesRegex(AssertionError, 'Art blob changed'):
                        composed.check(self.root)
                finally:
                    self.write(p, old)

    def test_both_new_source_artifacts_cannot_be_omitted_or_mutated(self):
        for p, (old, _) in composed.ART_OVERLAY.items():
            if old is not None:
                continue
            for omit in [True, False]:
                with self.subTest(path=p, omitted=omit):
                    raw = (self.root / p).read_bytes()
                    if omit:
                        (self.root / p).unlink()
                    else:
                        self.write(p, b'changed source')
                    try:
                        with self.profile_context(), self.assertRaises(AssertionError):
                            composed.check(self.root)
                    finally:
                        self.write(p, raw)

    def test_unregistered_mutation_addition_deletion_and_executable_mode_fail(self):
        p = 'crates/alife_world/assets/unregistered-00.bin'
        raw = (self.root / p).read_bytes()
        for change in ['mutation', 'addition', 'deletion', 'mode']:
            with self.subTest(change=change):
                extra = 'crates/alife_world/assets/unreviewed.bin'
                if change == 'mutation':
                    self.write(p, b'changed')
                elif change == 'addition':
                    self.write(extra, b'new')
                elif change == 'deletion':
                    (self.root / p).unlink()
                else:
                    (self.root / p).chmod(0o755)
                try:
                    with self.profile_context(), self.assertRaises(AssertionError):
                        composed.check(self.root)
                finally:
                    self.write(p, raw)
                    (self.root / p).chmod(0o644)
                    (self.root / extra).unlink(missing_ok=True)

    def test_art_file_symlink_and_art_root_ancestor_symlink_fail(self):
        p = self.root / composed.HERE / 'island-biological-shrubs.blend'
        raw = p.read_bytes()
        target = self.root / 'outside.blend'
        target.write_bytes(raw)
        p.unlink()
        p.symlink_to(target)
        with self.profile_context(), self.assertRaisesRegex(AssertionError, 'symlink'):
            composed.check(self.root)
        p.unlink()
        p.write_bytes(raw)
        original = self.root / 'crates/alife_world'
        relocated = self.root / 'relocated-world'
        original.rename(relocated)
        original.symlink_to(relocated, target_is_directory=True)
        with self.profile_context(), self.assertRaisesRegex(AssertionError, 'symlink'):
            composed.check(self.root)

    def test_each_conifer_script_dependency_and_recipe_is_pinned(self):
        for p in composed.CONIFER_SCRIPTS:
            with self.subTest(path=p):
                raw = (self.root / p).read_bytes()
                self.write(p, raw + b'changed')
                try:
                    with self.assertRaisesRegex(AssertionError, 'source blob changed'):
                        composed.validate_script_identities(self.root)
                finally:
                    self.write(p, raw)

    def test_checker_script_symlink_mode_and_extra_conifer_source_fail(self):
        p = self.root / 'scripts/conifer_glb.py'
        p.chmod(0o755)
        with self.assertRaisesRegex(AssertionError, 'source mode changed'):
            composed.validate_script_identities(self.root)
        p.chmod(0o644)
        raw = p.read_bytes()
        target = self.root / 'outside.py'
        target.write_bytes(raw)
        p.unlink()
        p.symlink_to(target)
        with self.assertRaisesRegex(AssertionError, 'symlink'):
            composed.validate_script_identities(self.root)
        p.unlink()
        p.write_bytes(raw)
        self.write('scripts/unreviewed_conifer.py', b'new source')
        with self.assertRaisesRegex(AssertionError, 'Unexpected Conifer source paths'):
            composed.validate_script_identities(self.root)

    def test_historical_checker_dependency_mutation_fails(self):
        self.write('scripts/check_island_art.py', b'changed historical dependency')
        with self.assertRaisesRegex(AssertionError, 'source blob changed'):
            composed.validate_script_identities(self.root)

    def test_other_complete_registration_records_cannot_change(self):
        for index in [0, 1, 2, 3, 5, 47]:
            with self.subTest(index=index):
                actual = copy.deepcopy(self.actual_manifest)
                actual['entries'][index]['generator'] = {'version': 'plausible refresh'}
                self.write_manifest(actual)
                with self.assertRaisesRegex(AssertionError, 'sole expected PR24 Conifer update'):
                    self.validate_manifest()

    def test_unrelated_runtime_mutation_and_reregistration_fail(self):
        p = self.paths[-1]
        raw = b'reregistered unrelated asset'
        self.write(p, raw)
        with self.assertRaises(AssertionError):
            self.validate_manifest()
        actual = copy.deepcopy(self.actual_manifest)
        actual['entries'][-1].update(size_bytes=len(raw), digest=historical.digest_payload(p, raw))
        self.write_manifest(actual)
        with self.assertRaisesRegex(AssertionError, 'sole expected PR24 Conifer update'):
            self.validate_manifest()

    def test_registration_set_order_identity_metadata_and_conifer_policy_are_fixed(self):
        for change in ['add', 'remove', 'duplicate-id', 'duplicate-path', 'order', 'id', 'path',
                       'metadata', 'license', 'policy', 'generator', 'omit-conifer-refresh']:
            with self.subTest(change=change):
                actual = copy.deepcopy(self.actual_manifest)
                entries = actual['entries']
                if change == 'add':
                    entries.append(copy.deepcopy(entries[-1]))
                elif change == 'remove':
                    entries.pop()
                elif change == 'duplicate-id':
                    entries[-1]['asset_id'] = entries[-2]['asset_id']
                elif change == 'duplicate-path':
                    entries[-1]['local_path'] = entries[-2]['local_path']
                elif change == 'order':
                    entries[0], entries[1] = entries[1], entries[0]
                elif change == 'id':
                    entries[4]['asset_id'] = 'different-id'
                elif change == 'path':
                    entries[4]['local_path'] = 'crates/alife_game_app/assets/unreviewed.glb'
                elif change == 'metadata':
                    actual['schema'] = 'different schema'
                elif change == 'omit-conifer-refresh':
                    entries[4] = copy.deepcopy(self.parent_manifest['entries'][4])
                else:
                    entries[4][change] = 'unapproved change'
                self.write_manifest(actual)
                with self.assertRaises(AssertionError):
                    self.validate_manifest()

    def test_wrong_pinned_manifest_and_wrong_conifer_baseline_fail(self):
        with self.manifest_context(), self.assertRaisesRegex(AssertionError, 'exact pinned PR24 manifest'):
            composed.validate_manifest(self.root, self.parent_raw + b' ')
        p = self.root / composed.HERE / 'conifer-leader/Conifer.baseline.glb'
        p.write_bytes(b'wrong baseline')
        with self.profile_context(), self.assertRaisesRegex(AssertionError, 'Art blob changed'):
            composed.check(self.root)

    def test_registered_json_uses_historical_newline_normalization(self):
        self.assertEqual(historical.digest_payload('x.json', b'a\r\nb\rc'),
                         historical.digest_payload('x.json', b'a\nb\nc'))
        self.assertNotEqual(historical.digest_payload('x.glb', b'a\r\n'),
                            historical.digest_payload('x.glb', b'a\n'))

    def test_all_four_structural_checks_run_and_each_failure_propagates(self):
        with self.profile_context(), patch.object(historical, 'check_shrub_structures') as shrubs, \
             patch.object(historical, 'check_fern') as fern, patch.object(historical, 'check_sapling') as sapling, \
             patch.object(conifer, 'check') as leader:
            composed.check(self.root)
            for worker in [shrubs, fern, sapling, leader]:
                worker.assert_called_once()
        for name in ['check_shrub_structures', 'check_fern', 'check_sapling', 'check']:
            with self.subTest(validator=name), self.profile_context(name):
                with self.assertRaisesRegex(AssertionError, 'fixture structure failed'):
                    composed.check(self.root)

    def test_missing_pinned_git_history_fails_closed(self):
        with patch.object(historical, 'git_art_tree', side_effect=subprocess.CalledProcessError(128, 'git')):
            with self.assertRaises(subprocess.CalledProcessError):
                composed.check(self.root)

    def test_optimized_python_refuses_before_pass_or_output(self):
        script = composed.ROOT / 'scripts/check_composed_conifer_graphics.py'
        for flags, optimize in [(['-O'], None), (['-OO'], None), ([], '1')]:
            with self.subTest(flags=flags, env=optimize):
                env = dict(os.environ)
                if optimize is not None:
                    env['PYTHONOPTIMIZE'] = optimize
                else:
                    env.pop('PYTHONOPTIMIZE', None)
                output = self.root / 'must-not-be-created.json'
                result = subprocess.run([sys.executable, *flags, str(script), '--root', str(self.root),
                                         '--output', str(output)], capture_output=True, text=True, env=env)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('requires normal Python mode', result.stderr)
                self.assertNotIn('PASS', result.stdout)
                self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()

"""Combined positive/negative audit plus both retained plant geometry suites.

Only the independent full-tree positive tests are superseded. All other Fern,
CoastalShrub and Broadleaf negative contracts run without rewriting those files.
"""
import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import check_fern_coastal_composition as check


class FernCoastalCompositionTests(unittest.TestCase):
    def profiles(self):
        return check.load_profiles(check.ROOT)

    def test_complete_union_art_manifest_source_and_geometry(self):
        result = check.check()
        self.assertEqual(result['result'], 'PASS')
        self.assertEqual(result['art_blobs_audited'], 140)
        self.assertEqual(result['manifest_entries_valid'], 48)
        self.assertEqual(result['other_manifest_records_retained'], 46)
        self.assertEqual([r['triangles'] for r in result['fern_lods']], [700, 294, 154])
        self.assertEqual([r['triangles'] for r in result['coastal_lods']], [672, 204, 218])
        self.assertEqual(result['retained_broadleaf_triangles'], [4462, 916, 86])
        self.assertEqual(result['coastal_shrub_placements'], 170)
        self.assertFalse(result['game_rendering_proof'])
        self.assertFalse(result['physical_gpu_proof'])

    def test_clean_stdlib_only_combined_audit(self):
        result = subprocess.run([sys.executable, '-S', str(check.ROOT / 'scripts/check_fern_coastal_composition.py')],
                                capture_output=True, text=True, check=False, timeout=60)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['result'], 'PASS')

    def test_optimized_python_cannot_disable_combined_checks(self):
        result = subprocess.run([sys.executable, '-O', str(check.ROOT / 'scripts/check_fern_coastal_composition.py')],
                                capture_output=True, text=True, check=False, timeout=60)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('require normal Python mode', result.stderr)

    def test_rejects_changed_combined_profile(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / check.PROFILE).parent.mkdir(parents=True)
            (root / check.PROFILE).write_bytes((check.ROOT / check.PROFILE).read_bytes() + b' ')
            with self.assertRaisesRegex(AssertionError, 'Fixed combined profile changed'):
                check.load_profiles(root)

    def test_retains_both_independent_profiles_as_historical_snapshots(self):
        # Their rejection proves neither stale full-art gate was weakened to
        # accept the new composition. The new successor is the positive gate.
        for module in (check.fern, check.coastal):
            with self.subTest(profile=module.PROFILE), self.assertRaises(AssertionError):
                module.check()

    def test_rejects_missing_retained_pins_from_either_upgrade(self):
        for name in ('scripts/build_fern_fronds.py', 'scripts/build_coastal_leaf_sprays.py',
                     'scripts/check_fern_fronds.py', 'scripts/check_coastal_leaf_sprays.py',
                     check.fern.PROFILE.as_posix(), check.coastal.PROFILE.as_posix()):
            profile, fp, cp, before, consolidated = self.profiles()
            del profile['retained_sources'][name]
            with self.subTest(source=name), self.assertRaisesRegex(AssertionError, 'Combined retained source'):
                check.validate_sources(profile, fp, cp, before, consolidated)

    def test_rejects_conflicting_retained_source_pins(self):
        with self.assertRaisesRegex(AssertionError, 'Conflicting source pins'):
            check.merge_pins({'source.py': 'a'}, {'source.py': 'b'})

    def test_rejects_removed_or_extra_current_checker_sources(self):
        profile, fp, cp, before, consolidated = self.profiles()
        for fault in ('missing', 'extra'):
            changed = copy.deepcopy(profile)
            if fault == 'missing':
                del changed['sources']['scripts/test_fern_coastal_composition.py']
            else:
                changed['sources']['scripts/unreviewed.py'] = '0' * 64
            with self.subTest(fault=fault), self.assertRaisesRegex(AssertionError, 'Combined successor source'):
                check.validate_sources(changed, fp, cp, before, consolidated)

    def test_rejects_changed_runtime_native_and_snapshot_of_either_upgrade(self):
        for name in sorted(check.RUNTIMES | check.ADDED_ART):
            profile, fp, cp, before, _ = self.profiles()
            row = next(row for row in profile['expected_art_files'] if row['path'] == name)
            row['sha256'] = '0' * 64
            with self.subTest(asset=name), self.assertRaisesRegex(AssertionError, 'Approved .* art changed'):
                check.validate_retention(profile, fp, cp, before)

    def test_rejects_unrelated_art_and_source_removal(self):
        for fault in ('changed', 'missing', 'extra'):
            profile, fp, cp, before, _ = self.profiles()
            if fault == 'changed':
                row = next(row for row in profile['expected_art_files'] if row['path'].endswith('/BerryBush.glb'))
                row['sha256'] = '0' * 64
                message = 'Unrelated combined art record changed'
            elif fault == 'missing':
                profile['expected_art_files'] = [row for row in profile['expected_art_files']
                                                 if row['path'] != check.fern.SOURCE.as_posix()]
                message = 'Combined art path retention changed'
            else:
                profile['expected_art_files'].append(dict(profile['expected_art_files'][0], path='unexpected.glb'))
                message = 'Combined art path retention changed'
            with self.subTest(fault=fault), self.assertRaisesRegex(AssertionError, message):
                check.validate_retention(profile, fp, cp, before)

    def test_rejects_duplicate_art_records(self):
        profile, fp, cp, before, _ = self.profiles()
        profile['expected_art_files'].append(copy.deepcopy(profile['expected_art_files'][0]))
        with self.assertRaisesRegex(AssertionError, 'Duplicate art path'):
            check.validate_retention(profile, fp, cp, before)

    def test_rejects_each_species_or_unrelated_specification_change(self):
        for species in ('Fern', 'CoastalShrub', 'BerryBush'):
            profile, _, cp, _, _ = self.profiles()
            spec = json.loads((check.ROOT / check.SPECIFICATION).read_text())
            spec['assets'][species]['near_triangles'] += 1
            with self.subTest(species=species), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                path = root / check.SPECIFICATION
                path.parent.mkdir(parents=True)
                path.write_text(json.dumps(spec))
                with self.assertRaisesRegex(AssertionError, 'Combined specification'):
                    check.validate_specification(root, profile, cp)

    def test_rejects_changed_fern_specification_pin(self):
        profile, _, cp, _, _ = self.profiles()
        profile['fern_specification']['width_depth_height_m'][0] += .1
        with self.assertRaisesRegex(AssertionError, 'Combined Fern specification'):
            check.validate_specification(check.ROOT, profile, cp)

    def test_rejects_choosing_either_complete_manifest(self):
        profile, fp, cp, before, consolidated = self.profiles()
        expected = check.expected_manifest(check.ROOT, profile, fp, cp, before, consolidated)
        baseline = check.coastal.predecessor.expected_manifest(before, consolidated, check.ROOT)
        originals = {row['local_path']: row for row in baseline['entries']}
        for name in sorted(check.RUNTIMES):
            actual = copy.deepcopy(expected)
            for i, row in enumerate(actual['entries']):
                if row['local_path'] == name:
                    actual['entries'][i] = originals[name]
            with self.subTest(lost_registration=name), self.assertRaisesRegex(AssertionError, 'Unreviewed manifest'):
                check.coastal.validate_manifest_records(check.ROOT, actual, expected)

    def test_rejects_unreviewed_manifest_override(self):
        profile, fp, cp, before, consolidated = self.profiles()
        profile['manifest_overrides'][check.fern.RUNTIME]['generator']['seed'] = 'unreviewed'
        with self.assertRaisesRegex(AssertionError, 'Combined manifest overrides'):
            check.expected_manifest(check.ROOT, profile, fp, cp, before, consolidated)

    def test_same_existing_ci_lane_runs_combined_successor(self):
        text = (check.ROOT / '.github/workflows/ci.yml').read_text()
        self.assertEqual(text.count('\n  broadleaf-canopy:\n'), 1)
        self.assertIn('name: Broadleaf canopy CPU regression', text)
        self.assertIn('python -S scripts/check_fern_coastal_composition.py', text)
        self.assertIn("python -S -m unittest discover -s scripts -p 'test_fern_coastal_composition.py' -v", text)


def load_tests(loader, tests, pattern):
    import test_fern_fronds as fern_tests
    import test_coastal_leaf_sprays as coastal_tests
    import test_broadleaf_canopy as broadleaf_tests
    superseded = {'test_complete_art_manifest_and_source_audit', 'test_clean_stdlib_only_audit'}
    for case in (fern_tests.FernFrondTests, coastal_tests.CoastalLeafSprayTests):
        tests.addTests(case(name) for name in loader.getTestCaseNames(case) if name not in superseded)
    tests.addTests(broadleaf_tests.BroadleafCanopyTests(name)
                   for name in loader.getTestCaseNames(broadleaf_tests.BroadleafCanopyTests)
                   if name.startswith('test_rejects_'))
    return tests


if __name__ == '__main__':
    unittest.main()

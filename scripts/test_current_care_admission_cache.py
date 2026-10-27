"""Native admission is mocked; publication/recovery and file mutations are real."""
from pathlib import Path
import os
import tempfile
import unittest
from unittest.mock import patch

import current_checkpoint_store as cp
import run_current_care as current


class AdmissionCacheTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.binary = self.root / 'tool'
        self.binary.write_bytes(b'pinned-native-tool')
        self.path = self.root / 'candidate'
        self.path.write_bytes(b'complete-current-state')
        self.report = {'kind': 'single-current-training-checkpoint', 'external_dependencies': []}
        mock = patch.object(current, 'tool', return_value=self.report)
        self.native = mock.start()
        self.addCleanup(mock.stop)
        self.validate = current.validator(self.binary)

    def test_identical_relocated_bytes_are_admitted_once_but_changes_are_fresh(self):
        self.validate(self.path)
        other = self.root / 'relocated'
        other.write_bytes(self.path.read_bytes())
        self.validate(other)
        self.assertEqual(self.native.call_count, 1)
        other.write_bytes(b'new-Adam-private-or-seed-state')
        self.validate(other)
        self.assertEqual(self.native.call_count, 2)

    def test_bare_relative_inspector_invokes_the_exact_absolute_file_hashed(self):
        previous = Path.cwd()
        try:
            os.chdir(self.root)
            validate = current.validator(Path(self.binary.name))
            validate(self.path)
        finally:
            os.chdir(previous)
        self.native.assert_called_once_with(self.binary.resolve(strict=True), '--inspect-current', self.path)

    def test_cached_native_report_is_returned_and_isolated_from_caller_mutation(self):
        self.report.update(policy_version=42, seeds={'pending_range': [100, 101]})
        first = self.validate(self.path)
        first['policy_version'] = 999
        first['seeds']['pending_range'][0] = 1
        self.assertEqual(self.report['policy_version'], 42)
        self.assertEqual(self.report['seeds']['pending_range'], [100, 101])
        cached = self.validate(self.path)
        self.assertEqual(cached['policy_version'], 42)
        self.assertEqual(cached['seeds']['pending_range'], [100, 101])
        cached['seeds']['pending_range'][1] = 999
        self.assertEqual(self.validate(self.path)['seeds']['pending_range'], [100, 101])
        self.assertEqual(self.native.call_count, 1)

    def test_native_failure_is_never_cached_and_invalidates_prior_reuse(self):
        self.validate(self.path)
        other = self.root / 'different'
        other.write_bytes(b'not admitted')
        self.native.side_effect = ValueError('native rejection')
        with self.assertRaisesRegex(ValueError, 'native rejection'):
            self.validate(other)
        self.native.side_effect = None
        self.validate(self.path)
        self.assertEqual(self.native.call_count, 3)

    def test_changed_during_admission_is_rejected_not_cached(self):
        def mutate(*args):
            self.path.write_bytes(b'different bytes during native read')
            return self.report
        self.native.side_effect = mutate
        with self.assertRaisesRegex(ValueError, 'changed during native admission'):
            self.validate(self.path)
        self.native.side_effect = None
        self.validate(self.path)
        self.assertEqual(self.native.call_count, 2)

    def test_tool_change_rejects_even_a_cached_checkpoint(self):
        self.validate(self.path)
        self.binary.write_bytes(b'changed tool')
        with self.assertRaisesRegex(ValueError, 'admission binary changed'):
            self.validate(self.path)
        self.assertEqual(self.native.call_count, 1)

    def test_binary_changed_during_native_call_is_rejected(self):
        def mutate(*args):
            self.binary.write_bytes(b'changed while inspecting')
            return self.report
        self.native.side_effect = mutate
        with self.assertRaisesRegex(ValueError, 'admission binary changed'):
            self.validate(self.path)

    def test_external_or_unrecognized_reports_always_revalidate(self):
        for report in ({'kind': 'single-current-training-checkpoint', 'external_dependencies': ['outside']},
                       {'kind': 'other', 'external_dependencies': []}, {'kind': 'single-current-training-checkpoint'}):
            self.native.return_value = report
            validate = current.validator(self.binary)
            before = self.native.call_count
            validate(self.path); validate(self.path)
            self.assertEqual(self.native.call_count-before, 2)

    def test_bounded_cache_evicts_old_bytes_and_a_new_validator_is_cold(self):
        for number in range(9):
            self.path.write_bytes(str(number).encode())
            self.validate(self.path)
        self.path.write_bytes(b'0')
        self.validate(self.path)
        self.assertEqual(self.native.call_count, 10)
        current.validator(self.binary)(self.path)
        self.assertEqual(self.native.call_count, 11)

    def test_cached_content_does_not_bypass_regular_file_guard(self):
        self.validate(self.path)
        with self.assertRaisesRegex(ValueError, 'bounded regular checkpoint'):
            self.validate(self.root)
        self.assertEqual(self.native.call_count, 1)

    def test_publication_copies_are_hash_checked_with_two_full_admissions(self):
        store_root = self.root / 'store'; store_root.mkdir()
        (store_root / cp.CURRENT).write_bytes(b'accepted-old-state')
        self.path.write_bytes(b'accepted-new-state')
        with cp.Store(store_root, self.validate) as store:
            before = cp.digest(store.current)
            store.publish(self.path, before)
        self.assertEqual((store_root / cp.CURRENT).read_bytes(), b'accepted-new-state')
        self.assertEqual(self.native.call_count, 2)
        self.assertEqual({p.name for p in store_root.iterdir()}, {cp.CURRENT, cp.LOCK})

    def test_process_restart_recovery_readmits_before_discarding_prior(self):
        store_root = self.root / 'store'; store_root.mkdir()
        (store_root / cp.CURRENT).write_bytes(b'accepted-old-state')
        self.path.write_bytes(b'accepted-new-state')
        class Crash(BaseException):
            pass
        def crash(stage):
            if stage == 'new-moved':
                raise Crash()
        with self.assertRaises(Crash):
            with cp.Store(store_root, self.validate, crash) as store:
                store.publish(self.path, cp.digest(store.current))
        self.assertTrue((store_root / cp.PREVIOUS).exists())
        before = self.native.call_count
        with cp.Store(store_root, current.validator(self.binary)):
            pass
        self.assertGreater(self.native.call_count, before)
        self.assertEqual((store_root / cp.CURRENT).read_bytes(), b'accepted-new-state')
        self.assertEqual({p.name for p in store_root.iterdir()}, {cp.CURRENT, cp.LOCK})


if __name__ == '__main__':
    unittest.main()

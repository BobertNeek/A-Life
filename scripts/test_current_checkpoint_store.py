"""Host crash/ownership tests with disposable bytes, not neural/GPU evidence."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import current_checkpoint_store as cp


class Crash(BaseException):
    pass


class StoreTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.store = self.root / 'store'
        self.store.mkdir()
        self.old = self.root / 'old'
        self.new = self.root / 'new'
        self.old.write_bytes(b'complete old compiler/actor/value/Adam')
        self.new.write_bytes(b'complete new compiler/actor/value/Adam')
        self.validate = lambda p: cp.require(p.read_bytes() in (self.old.read_bytes(), self.new.read_bytes()), 'invalid state')
        with cp.Store(self.store, self.validate) as store:
            store.publish(self.old, None)

    def test_every_publication_crash_preserves_one_complete_authority_and_cleans_transients(self):
        stages = ('candidate-written', 'candidate-verified', 'intent-prepared', 'intent-written',
                  'old-preserved', 'new-moved', 'new-verified', 'old-removed', 'intent-removed')
        for stage in stages:
            with self.subTest(stage=stage):
                with cp.Store(self.store, self.validate) as store:
                    store.publish(self.old, cp.digest(store.current))

                def crash(at):
                    self.validate(self.store / cp.CURRENT)  # Atomic replacement never exposes an absent/partial current.
                    if at == stage:
                        raise Crash()
                with self.assertRaises(Crash):
                    with cp.Store(self.store, self.validate, crash) as store:
                        store.publish(self.new, cp.digest(store.current))
                survivors = [self.store / n for n in (cp.CURRENT, cp.NEXT, cp.PREVIOUS)
                             if (self.store / n).exists()]
                self.assertTrue(any(p.read_bytes() in (self.old.read_bytes(), self.new.read_bytes()) for p in survivors))
                with cp.Store(self.store, self.validate) as store:
                    expected = self.old if stage in stages[:3] else self.new
                    self.assertEqual(store.current.read_bytes(), expected.read_bytes())
                self.assertEqual({p.name for p in self.store.iterdir()}, {cp.CURRENT, cp.LOCK})

    def test_corrupt_committed_candidate_never_deletes_old_state(self):
        def crash(stage):
            if stage == 'old-preserved':
                raise Crash()
        with self.assertRaises(Crash):
            with cp.Store(self.store, self.validate, crash) as store:
                store.publish(self.new, cp.digest(store.current))
        (self.store / cp.NEXT).write_bytes(b'truncated')
        before = {p.name: p.read_bytes() for p in self.store.iterdir()}
        with self.assertRaisesRegex(ValueError, 'missing or corrupt'):
            with cp.Store(self.store, self.validate):
                pass
        self.assertEqual({p.name: p.read_bytes() for p in self.store.iterdir()}, before)
        self.assertEqual((self.store / cp.PREVIOUS).read_bytes(), self.old.read_bytes())

    def test_invalid_candidate_or_changed_current_cannot_start_replacement(self):
        invalid = self.root / 'bad'
        invalid.write_bytes(b'corrupt')
        with cp.Store(self.store, self.validate) as store:
            before = cp.digest(store.current)
            with self.assertRaises(ValueError):
                store.publish(invalid, before)
            with self.assertRaises(ValueError):
                store.publish(self.new, '0' * 64)
            self.assertEqual(cp.digest(store.current), before)
        self.assertEqual({p.name for p in self.store.iterdir()}, {cp.CURRENT, cp.LOCK})

    def test_interrupted_initial_import_recovers_from_complete_owned_candidate(self):
        empty = self.root / 'new-store'
        empty.mkdir()
        run = empty / cp.RUN
        run.mkdir()
        candidate = run / 'import.alife-checkpoint'
        candidate.write_bytes(self.new.read_bytes())
        cp.write_json(run / 'run.json', {'schema': 1, 'kind': 'single-current-temporary-run',
            'mode': 'import', 'candidate_sha256': cp.digest(candidate), 'worker': None})
        def crash(stage):
            if stage == 'candidate-written':
                raise Crash()
        with self.assertRaises(Crash):
            with cp.Store(empty, self.validate, crash) as store:
                store.publish(candidate, None)
        with cp.Store(empty, self.validate) as store:
            self.assertEqual(store.current.read_bytes(), self.new.read_bytes())
        self.assertFalse((empty / cp.NEXT).exists())

    def test_partial_prior_copy_recovers_without_exposing_missing_current(self):
        def crash(stage):
            if stage == 'old-preserved':
                raise Crash()
        with self.assertRaises(Crash):
            with cp.Store(self.store, self.validate, crash) as store:
                store.publish(self.new, cp.digest(store.current))
        (self.store / cp.PREVIOUS).write_bytes(b'partial old copy')
        self.assertEqual((self.store / cp.CURRENT).read_bytes(), self.old.read_bytes())
        with cp.Store(self.store, self.validate) as store:
            self.assertEqual(store.current.read_bytes(), self.new.read_bytes())

    def run_state(self, worker=None):
        run = self.store / cp.RUN
        run.mkdir(exist_ok=True)
        (run / 'private-recovery').write_bytes(b'recoverable work')
        state = {'schema': 1, 'kind': 'single-current-temporary-run', 'worker': worker,
                 'reconciled_current': cp.digest(self.store / cp.CURRENT)}
        cp.write_json(run / 'run.json', state)
        return state

    def test_cleanup_requires_stopped_writer_and_verified_current(self):
        self.run_state({'pid': os.getpid(), 'stopped': False})
        with cp.Store(self.store, self.validate) as store:
            with self.assertRaisesRegex(ValueError, 'active'):
                store.remove_run(cp.digest(store.current))
            self.assertTrue((self.store / cp.RUN / 'private-recovery').exists())
            state = self.run_state({'pid': os.getpid(), 'stopped': True})
            with self.assertRaises(ValueError):
                store.remove_run('wrong')
            state['reconciled_current'] = 'wrong'
            cp.write_json(self.store / cp.RUN / 'run.json', state)
            with self.assertRaisesRegex(ValueError, 'not been reconciled'):
                store.remove_run(cp.digest(store.current))
            self.run_state({'pid': os.getpid(), 'stopped': True})
            store.remove_run(cp.digest(store.current))
            self.assertFalse((self.store / cp.RUN).exists())

    def test_unknown_launch_outcome_and_unknown_store_entries_fail_closed(self):
        self.run_state({'pid': None, 'stopped': False})
        with cp.Store(self.store, self.validate) as store:
            with self.assertRaisesRegex(ValueError, 'unknown writer'):
                store.remove_run(cp.digest(store.current))
        (self.store / 'other-owner').write_text('untouched')
        with self.assertRaisesRegex(ValueError, 'unknown store entries'):
            with cp.Store(self.store, self.validate):
                pass
        self.assertTrue((self.store / cp.RUN / 'private-recovery').exists())

    def test_oversized_metadata_update_preserves_the_last_readable_state(self):
        state = self.run_state()
        path = self.store / cp.RUN / 'run.json'
        before = path.read_bytes()
        with self.assertRaisesRegex(ValueError, 'metadata byte limit'):
            cp.write_json(path, {**state, 'oversized': 'x' * cp.MAX_METADATA_BYTES})
        self.assertEqual(path.read_bytes(), before)
        self.assertEqual(cp.read_json(path), state)
        self.assertFalse(path.with_name('run.json.tmp').exists())

    def test_os_lock_excludes_another_process_and_releases_after_exit(self):
        script = '''import pathlib,sys
import current_checkpoint_store as cp
try:
    with cp.writer_lock(pathlib.Path(sys.argv[1])): pass
except (ValueError, OSError): sys.exit(7)
'''
        environment = {**os.environ, 'PYTHONPATH': str(Path(cp.__file__).parent)}
        with cp.Store(self.store, self.validate):
            result = subprocess.run([sys.executable, '-c', script, str(self.store)], env=environment)
            self.assertEqual(result.returncode, 7)
        result = subprocess.run([sys.executable, '-c', script, str(self.store)], env=environment)
        self.assertEqual(result.returncode, 0)

    def test_cleanup_validates_current_before_touching_recovery(self):
        self.run_state()
        with cp.Store(self.store, self.validate) as store:
            store.current.write_bytes(b'corrupt')
            with self.assertRaises(ValueError):
                store.remove_run(cp.digest(store.current))
        self.assertTrue((self.store / cp.RUN / 'private-recovery').exists())

    @unittest.skipIf(os.name == 'nt', 'symlink creation requires Windows privilege')
    def test_symlink_run_cannot_delete_another_directory(self):
        outside = self.root / 'outside'
        outside.mkdir()
        (outside / 'learning').write_text('preserve')
        (self.store / cp.RUN).symlink_to(outside, target_is_directory=True)
        with cp.Store(self.store, self.validate) as store:
            with self.assertRaises(ValueError):
                store.remove_run(cp.digest(store.current))
        self.assertEqual((outside / 'learning').read_text(), 'preserve')


if __name__ == '__main__':
    unittest.main()

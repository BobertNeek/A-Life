"""Bounded archival preserves exact history and fails before acceptance/mutation."""
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import grab_food_seed_reservations as seeds
import mixed_care_ledger as store


class SegmentedLedgerTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.path = self.root / "ledger.json"
        self.value = {"schema": 1, "kind": store.LEDGER, "excluded_seed_ranges": [[10, 20]],
                      "assessment_seeds": [99], "operator_note": "keep this metadata", "runs": [
                          self.allocation(10), self.allocation(11), self.allocation(12, "pending")]}

    def allocation(self, seed, status="accepted"):
        return {"output": str(self.root / str(seed)), "source_binding": "source-" + str(seed),
                "attempts": [{"seed": seed, "status": status, "slot": 0, "optimizer_age": 2**60 + seed,
                              "receipt": {"note": "spaces stay\n?", "value": 0.125}}]}

    def legacy(self):
        seeds.write_document(self.path, self.value)
        return store.read(self.path)

    def migrated(self):
        state = self.legacy(); store.write(self.path, state)
        return store.read(self.path)

    def test_legacy_migration_is_lossless_and_exact_original_is_retained(self):
        state = self.legacy(); before = self.path.read_bytes()
        result = store.write(self.path, state); manifest = seeds._read(self.path)[0]
        self.assertEqual(result, hashlib.sha256(self.path.read_bytes()).hexdigest())
        self.assertEqual(manifest["kind"], store.MANIFEST)
        self.assertEqual(dict(store.read(self.path)), self.value)
        backup = store.folder(self.path) / ("legacy-" + hashlib.sha256(before).hexdigest() + ".json")
        self.assertEqual(backup.read_bytes(), before)
        self.assertEqual(len(manifest["current_runs"]), 1)
        self.assertEqual(manifest["archived_runs"], 2)

    def test_total_history_can_exceed_same_per_document_cap(self):
        state = store.State({**self.value, "runs": []})
        with patch.object(seeds, "MAX_DOCUMENT_BYTES", 2500):
            for seed in range(100, 130):
                state["runs"].append(self.allocation(seed))
                store.write(self.path, state)
            self.assertGreater(len(json.dumps(state).encode()), 2500)
            self.assertEqual(dict(store.read(self.path)), dict(state))
            self.assertLessEqual(self.path.stat().st_size, 2500)
            self.assertTrue(all(p.stat().st_size <= 2500 for p in store.folder(self.path).iterdir()))

    def test_pending_and_accepted_writes_hash_unchanged_prefix_once_each(self):
        state = self.migrated()
        state["runs"][-1]["attempts"][0]["status"] = "accepted"
        store.write(self.path, state)
        for seed in (13, 14):
            attempt = {"seed": seed, "slot": seed - 12, "status": "pending"}
            state["runs"][-1]["attempts"].append(attempt)
            for status in ("pending", "accepted"):
                with self.subTest(seed=seed, status=status):
                    attempt["status"] = status
                    with patch.object(store, "canonical_hash", wraps=store.canonical_hash) as hashed:
                        store.write(self.path, state)
                    hashed.assert_called_once_with(state["runs"][:2])
                    self.assertEqual(dict(store.read(self.path)), dict(state))

    def test_new_allocation_rebinds_prefix_and_nested_mutation_fails_before_commit(self):
        state = self.migrated()
        state["runs"][-1]["attempts"][0]["status"] = "accepted"
        store.write(self.path, state)
        state["runs"].append(self.allocation(13, "pending"))
        with patch.object(store, "canonical_hash", wraps=store.canonical_hash) as hashed:
            store.write(self.path, state)
        self.assertEqual([len(call.args[0]) for call in hashed.call_args_list], [2, 3])
        self.assertEqual(state.protected_prefix, 3)
        self.assertEqual(state.prefix_sha256, store.canonical_hash(state["runs"][:3]))
        self.assertEqual(dict(store.read(self.path)), dict(state))

        state["runs"][-1]["attempts"][0]["status"] = "abandoned"
        with patch.object(store, "canonical_hash", wraps=store.canonical_hash) as hashed:
            store.write(self.path, state)
        hashed.assert_called_once_with(state["runs"][:3])
        self.assertEqual(dict(store.read(self.path)), dict(state))

        before = self.path.read_bytes()
        state["runs"][2]["attempts"][0]["receipt"]["note"] = "changed nested receipt"
        with patch.object(store, "immutable") as publish, self.assertRaisesRegex(ValueError, "archived history"):
            store.write(self.path, state)
        publish.assert_not_called()
        self.assertEqual(self.path.read_bytes(), before)

    def test_each_fresh_read_hashes_the_full_protected_prefix(self):
        state = self.migrated()
        for _ in range(2):
            with patch.object(store, "canonical_hash", wraps=store.canonical_hash) as hashed:
                admitted = store.read(self.path)
            prefixes = [call.args[0] for call in hashed.call_args_list if call.args[0]]
            self.assertEqual(prefixes, [state["runs"][:2]])
            self.assertEqual(admitted.prefix_sha256, store.canonical_hash(state["runs"][:2]))
            self.assertEqual(dict(admitted), dict(state))

    def test_failed_manifest_replace_preserves_old_bytes_and_orphans_are_reusable(self):
        state = self.legacy(); before = self.path.read_bytes(); real = os.replace
        def fail(source, target):
            if Path(target) == self.path:
                raise OSError("fixture sharing failure")
            return real(source, target)
        with patch.object(seeds.os, "replace", side_effect=fail), self.assertRaises(OSError):
            store.write(self.path, state)
        self.assertEqual(self.path.read_bytes(), before)
        objects = {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in store.folder(self.path).iterdir()}
        store.write(self.path, state)
        self.assertEqual(dict(store.read(self.path)), self.value)
        self.assertEqual(objects, {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in objects})

    def test_failed_segment_publication_preserves_the_legacy_ledger(self):
        state = self.legacy(); before = self.path.read_bytes()
        with patch.object(store.os, "link", side_effect=OSError("publication unavailable")), self.assertRaises(OSError):
            store.write(self.path, state)
        self.assertEqual(self.path.read_bytes(), before)
        self.assertEqual(list(store.folder(self.path).glob(".publishing-*")), [])
        store.write(self.path, state)
        self.assertEqual(dict(store.read(self.path)), self.value)

    def test_segment_corruption_or_missing_file_blocks_read(self):
        self.migrated(); manifest = seeds._read(self.path)[0]
        segment = store.folder(self.path) / (manifest["head"] + ".json")
        original = segment.read_bytes(); segment.write_bytes(original.replace(b"source-11", b"source-99"))
        with self.assertRaisesRegex(ValueError, "segment changed"):
            store.read(self.path)
        segment.write_bytes(original); segment.unlink()
        with self.assertRaises(FileNotFoundError):
            store.read(self.path)

    def test_legacy_backup_corruption_blocks_read(self):
        self.migrated(); manifest = seeds._read(self.path)[0]
        backup = store.folder(self.path) / ("legacy-" + manifest["legacy_backup"] + ".json")
        backup.write_bytes(b"corrupt")
        with self.assertRaisesRegex(ValueError, "backup changed"):
            store.read(self.path)

    def test_archive_prefix_and_current_records_cannot_be_removed_or_changed(self):
        for mutate in [lambda s: s["runs"].pop(0),
                       lambda s: s["runs"][0].update(source_binding="changed"),
                       lambda s: s["runs"][-1]["attempts"].clear(),
                       lambda s: s["runs"][-1]["attempts"][0].update(seed=777),
                       lambda s: s["excluded_seed_ranges"].clear(),
                       lambda s: s["assessment_seeds"].clear()]:
            with self.subTest(mutate=mutate):
                state = self.migrated(); before = self.path.read_bytes(); mutate(state)
                with self.assertRaises(ValueError):
                    store.write(self.path, state)
                self.assertEqual(self.path.read_bytes(), before)

    def test_pending_resolution_is_preserved_and_never_reuses_seed(self):
        state = self.migrated(); state["runs"][-1]["attempts"][0]["status"] = "abandoned"
        store.write(self.path, state)
        state["runs"][-1]["attempts"].append({"seed": 13, "slot": 0, "status": "pending"})
        store.write(self.path, state)
        actual = store.read(self.path)
        self.assertEqual([x["seed"] for x in actual["runs"][-1]["attempts"]], [12, 13])
        self.assertEqual(actual["runs"][-1]["attempts"][0]["status"], "abandoned")

    def test_pending_allocation_cannot_enter_immutable_archive(self):
        state = self.migrated(); before = self.path.read_bytes(); state["runs"].append(self.allocation(13))
        with self.assertRaisesRegex(ValueError, "uncertain allocation"):
            store.write(self.path, state)
        self.assertEqual(self.path.read_bytes(), before)

    def test_snapshot_keeps_origin_and_validates_every_dependency(self):
        self.migrated(); snapshot = self.root / "snapshot.json"; snapshot.write_bytes(self.path.read_bytes())
        reference = store.snapshot_reference(snapshot, self.path)
        self.assertEqual(reference["format"], store.MANIFEST)
        self.assertEqual(dict(store.read(snapshot, allow_snapshot=True)), self.value)
        with self.assertRaisesRegex(ValueError, "origin changed"):
            store.read(snapshot)
        store.verify_snapshot(snapshot, self.path)
        manifest = seeds._read(self.path)[0]
        (store.folder(self.path) / (manifest["head"] + ".json")).unlink()
        with self.assertRaises(FileNotFoundError):
            store.verify_snapshot(snapshot, self.path)

    def test_changed_manifest_since_admission_cannot_be_overwritten(self):
        state = self.migrated(); self.path.write_bytes(self.path.read_bytes() + b" ")
        before = self.path.read_bytes()
        with self.assertRaisesRegex(ValueError, "changed since admission"):
            store.write(self.path, state)
        self.assertEqual(self.path.read_bytes(), before)

    def test_oversize_current_allocation_preserves_previous_manifest(self):
        state = self.migrated(); before = self.path.read_bytes()
        state["runs"][-1]["attempts"].append({"status": "pending", "blob": "x" * 5000})
        with patch.object(seeds, "MAX_DOCUMENT_BYTES", 2500), self.assertRaises(ValueError):
            store.write(self.path, state)
        self.assertEqual(self.path.read_bytes(), before)

    def test_malformed_count_origin_and_head_are_rejected(self):
        self.migrated(); original = seeds._read(self.path)[0]
        for change in [{"archived_runs": 3}, {"origin": "relative.json"}, {"head": "../escape"},
                       {"current_runs": original["current_runs"] * 2}]:
            with self.subTest(change=change):
                seeds.write_document(self.path, {**original, **change}, compact=True)
                with self.assertRaises(ValueError):
                    store.read(self.path)

    def test_segments_are_durable_before_manifest_commit(self):
        state = self.legacy(); events = []; real_link = os.link; real_replace = os.replace
        def link(source, destination):
            events.append("segment-publish"); return real_link(source, destination)
        def replace(source, destination):
            if Path(destination) == self.path:
                events.append("manifest-commit")
            return real_replace(source, destination)
        with patch.object(store.os, "link", side_effect=link), patch.object(store.os, "replace", side_effect=replace), \
                patch.object(store.os, "fsync", side_effect=lambda _: events.append("fsync")):
            store.write(self.path, state)
        self.assertEqual(events[0], "fsync")
        self.assertEqual(events.count("segment-publish"), 3)
        self.assertGreater(events.index("manifest-commit"), max(i for i, e in enumerate(events) if e == "segment-publish"))
        self.assertEqual(events[events.index("manifest-commit") - 1], "fsync")

    def test_post_commit_failure_is_reconciled_by_reading_the_durable_root(self):
        state = self.legacy(); real = seeds.write_document
        def committed_then_failed(path, value, **options):
            real(path, value, **options)
            raise OSError('uncertain caller outcome after commit')
        with patch.object(seeds, 'write_document', side_effect=committed_then_failed), self.assertRaises(OSError):
            store.write(self.path, state)
        self.assertEqual(dict(store.read(self.path)), self.value)
        with self.assertRaisesRegex(ValueError, 'changed since admission'):
            store.write(self.path, state)

    def test_abandoned_attempt_cannot_be_silently_promoted(self):
        state = self.migrated(); state['runs'][-1]['attempts'][0]['status'] = 'abandoned'
        store.write(self.path, state); before = self.path.read_bytes()
        state['runs'][-1]['attempts'][0]['status'] = 'accepted'
        with self.assertRaisesRegex(ValueError, 'attempt changed'):
            store.write(self.path, state)
        self.assertEqual(self.path.read_bytes(), before)

    def test_existing_corrupt_immutable_file_is_never_overwritten(self):
        path = self.root / 'immutable.json'; path.write_bytes(b'corrupt retained evidence')
        before = path.read_bytes()
        with self.assertRaisesRegex(ValueError, 'immutable archive changed'):
            store.immutable(path, b'expected original bytes')
        self.assertEqual(path.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()

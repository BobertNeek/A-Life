import hashlib
import errno
import os
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch

import training_artifact_store as store


class ArtifactStoreTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.objects = self.root / 'objects'
        self.data = b'accepted weights and full Adam history' * 100
        self.sha = hashlib.sha256(self.data).hexdigest()

    def artifact(self, name):
        directory = self.root / name
        directory.mkdir()
        path = directory / 'actor-checkpoint.json'
        path.write_bytes(self.data)
        return path

    def test_identical_snapshots_share_storage_keep_paths_and_become_readonly(self):
        a, b = self.artifact('a'), self.artifact('b')
        for path in (a, b):
            store.share_file(path, self.sha, self.objects)
            self.assertEqual(path.read_bytes(), self.data)
            self.assertFalse(path.stat().st_mode & stat.S_IWUSR)
        self.assertTrue(os.path.samefile(a, b))
        self.assertEqual(store.share_file(b, self.sha, self.objects), 0)
        # Each checkpoint survives removal of the store's directory entry.
        blob = self.objects / self.sha
        if os.name == 'nt':
            blob.chmod(blob.stat().st_mode | stat.S_IWUSR)
        blob.unlink()
        self.assertEqual(a.read_bytes(), self.data)
        self.assertEqual(b.read_bytes(), self.data)

    def test_corrupt_store_never_replaces_valid_original(self):
        path = self.artifact('a')
        self.objects.mkdir()
        (self.objects / self.sha).write_bytes(b'wrong')
        with self.assertRaises(ValueError):
            store.share_file(path, self.sha, self.objects)
        self.assertEqual(path.read_bytes(), self.data)

    def test_interrupted_replace_retains_original(self):
        path = self.artifact('a')
        store.share_file(self.artifact('other'), self.sha, self.objects)
        with patch.object(store.os, 'replace', side_effect=OSError('interrupted')):
            with self.assertRaises(OSError):
                store.share_file(path, self.sha, self.objects)
        self.assertEqual(path.read_bytes(), self.data)
        self.assertEqual(list(path.parent.iterdir()), [path])
        store.share_file(path, self.sha, self.objects)
        self.assertTrue(os.path.samefile(path, self.objects / self.sha))

    def test_link_limit_rolls_over_without_stopping_or_losing_bytes(self):
        a, b = self.artifact('a'), self.artifact('b')
        store.share_file(a, self.sha, self.objects)
        original_link = os.link
        def limited(source, target):
            if Path(source) == self.objects / self.sha:
                raise OSError(errno.EMLINK, 'filesystem link limit')
            original_link(source, target)
        with patch.object(store.os, 'link', side_effect=limited):
            store.share_file(b, self.sha, self.objects)
        self.assertEqual(a.read_bytes(), b.read_bytes())
        self.assertFalse(os.path.samefile(a, b))
        self.assertTrue(os.path.samefile(b, self.objects / (self.sha + '.1')))

    def test_bad_hash_and_escaping_member_leave_original_untouched(self):
        path = self.artifact('a')
        with self.assertRaises(ValueError):
            store.share_file(path, '0' * 64, self.objects)
        with self.assertRaises(ValueError):
            store.reuse_sealed_files(path.parent, {'../outside': self.sha}, self.objects)
        self.assertEqual(path.read_bytes(), self.data)

    @unittest.skipUnless(os.name == 'posix', 'symlink creation may need Windows privilege')
    def test_symlink_inputs_and_store_are_rejected(self):
        path = self.artifact('a')
        link = path.parent / 'link'
        link.symlink_to(path)
        with self.assertRaises(ValueError):
            store.share_file(link, self.sha, self.objects)
        self.objects.symlink_to(path.parent, target_is_directory=True)
        with self.assertRaises(ValueError):
            store.share_file(path, self.sha, self.objects)

    def test_cumulative_lineage_uses_one_physical_object_per_digest(self):
        unique = []
        for episode in range(12):
            directory = self.root / f'episode-{episode}'
            directory.mkdir()
            data = f'receipt-{episode}'.encode()
            unique.append((data, hashlib.sha256(data).hexdigest()))
            hashes = {}
            for data, sha in unique:
                name = sha + '.json'
                (directory / name).write_bytes(data)
                hashes[name] = sha
            store.reuse_sealed_files(directory, hashes, self.objects)
        paths = list(self.root.glob('episode-*/*.json'))
        self.assertEqual(len(paths), 78)
        self.assertEqual(len({(p.stat().st_dev, p.stat().st_ino) for p in paths}), 12)
        for p in paths:
            self.assertEqual(hashlib.sha256(p.read_bytes()).hexdigest(), p.stem)


if __name__ == '__main__':
    unittest.main()

"""Reuse sealed, hash-verified training files without changing their paths or bytes.

Call only after native acceptance, under the scheduler's existing owner lock.
Objects and their aliases become read-only. Live ledgers, logs, partial attempts
and unsealed files never enter the store. A checkpoint remains readable even if
the store directory is removed: hard links are ordinary independent directory
entries, not references that require a separate loader.
"""
import hashlib
import errno
import os
from pathlib import Path
import stat
import tempfile


def digest(path):
    result = hashlib.sha256()
    with open(path, 'rb') as stream:
        while block := stream.read(1024 * 1024):
            result.update(block)
    return result.hexdigest()


def regular(path):
    if not stat.S_ISREG(path.lstat().st_mode):
        raise ValueError(f'artifact must be a regular non-symlink file: {path}')


def readonly(path):
    path.chmod(path.stat().st_mode & ~(stat.S_IWUSR | stat.S_IWGRP | stat.S_IWOTH))


def share_file(path, expected, store):
    """Publish before replacing; a failed copy/link leaves the original intact."""
    path, store = Path(path), Path(store)
    if len(expected) != 64 or any(c not in '0123456789abcdef' for c in expected):
        raise ValueError('invalid artifact SHA256')
    regular(path)
    if digest(path) != expected:
        raise ValueError(f'sealed artifact changed: {path}')
    store.mkdir(parents=True, exist_ok=True)
    if store.is_symlink() or not store.is_dir():
        raise ValueError('artifact store must be a real directory')
    if any(parent.is_symlink() for parent in store.parents):
        raise ValueError('artifact store traverses a symlink')
    variant = 0
    while True:
        # Some filesystems limit hard links per inode. Roll over to another
        # verified object instead of stopping learning or dropping any receipt.
        blob = store / (expected if variant == 0 else f'{expected}.{variant}')
        if not blob.exists():
            if not path.stat().st_mode & stat.S_IWUSR:
                return 0  # Leave pre-existing read-only aliases untouched.
            try:
                os.link(path, blob)  # Atomic exclusive publication, no payload copy.
            except FileExistsError:
                pass
        regular(blob)
        if digest(blob) != expected:
            raise ValueError(f'artifact store corruption: {blob}')
        readonly(blob)
        if os.path.samefile(path, blob):
            return 0
        if not path.stat().st_mode & stat.S_IWUSR:
            return 0  # Do not alter pre-existing read-only aliases to replace them.
        if digest(path) != expected:
            raise ValueError(f'sealed artifact changed during sharing: {path}')
        size = path.stat().st_size
        descriptor, temporary = tempfile.mkstemp(prefix='.artifact-link-', dir=path.parent)
        os.close(descriptor)
        temporary = Path(temporary)
        temporary.unlink()
        try:
            try:
                os.link(blob, temporary)
            except OSError as error:
                if error.errno == errno.EMLINK or getattr(error, 'winerror', None) == 1142:
                    variant += 1
                    continue
                raise
            os.replace(temporary, path)
        finally:
            if temporary.exists():
                # Windows read-only attributes apply to every hard-link alias.
                if os.name == 'nt':
                    temporary.chmod(temporary.stat().st_mode | stat.S_IWUSR)
                temporary.unlink()
                readonly(blob)
        return size


def reuse_sealed_files(directory, hashes, store):
    """Share only explicit seal members. Return logical bytes linked, not freed."""
    directory = Path(directory)
    linked = 0
    for name, expected in hashes.items():
        relative = Path(name)
        if relative.is_absolute() or '..' in relative.parts:
            raise ValueError('artifact name escapes checkpoint')
        path = directory / relative
        if any(parent.is_symlink() for parent in path.parents):
            raise ValueError('artifact path traverses a symlink')
        linked += share_file(path, expected, store)
    return {'files': len(hashes), 'logical_bytes_linked': linked}


def cpu_sealed_files(receipt, directory):
    """Current CPU seal plus retained exact-byte source and ancestry blobs."""
    hashes = dict(receipt['native_output_sha256'])
    hashes.update(receipt['episode_artifact_sha256'])
    hashes.update({str(Path('source-native') / name): value
                   for name, value in receipt['source_native_sha256'].items()})
    for relative in ('source-lineage', 'source-native/source-lineage'):
        folder = Path(directory) / relative
        if not folder.exists():
            continue
        if folder.is_symlink():
            raise ValueError('lineage directory must not be a symlink')
        for blob in folder.iterdir():
            if blob.suffix != '.json':
                raise ValueError('unexpected lineage artifact')
            hashes[str(Path(relative) / blob.name)] = blob.stem
    return hashes

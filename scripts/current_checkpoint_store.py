"""One durable checkpoint, transient publication files and one owned temporary run.

The validator must perform native full-state admission. All mutations require the
OS writer lock; lock files are never unlinked (which would split ownership).
"""
import contextlib
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat

CURRENT = 'current.alife-checkpoint'
NEXT = '.current.next'
PREVIOUS = '.current.previous'
INTENT = '.current.publish.json'
INTENT_NEXT = '.current.intent.next'
LOCK = '.current.writer.lock'
RUN = '.run'
MAX_BYTES = 256 * 1024 * 1024
MAX_METADATA_BYTES = 1024 * 1024


def require(condition, message):
    if not condition:
        raise ValueError(message)


def real_directory(path):
    info = path.lstat()
    return stat.S_ISDIR(info.st_mode) and not getattr(info, 'st_file_attributes', 0) & 0x400


def regular(path):
    info = path.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_size <= MAX_BYTES and
            not getattr(info, 'st_file_attributes', 0) & 0x400,
            f'not a bounded regular checkpoint file: {path}')
    return info


def digest(path):
    regular(path)
    result = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(block)
    return result.hexdigest()


def sync_directory(path):
    if os.name != 'nt':
        descriptor = os.open(path, os.O_RDONLY | getattr(os, 'O_DIRECTORY', 0))
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)


def replace(source, target):
    if os.name == 'nt':
        # Flush payloads first, then request a write-through same-volume rename.
        import ctypes
        from ctypes import wintypes
        move = ctypes.WinDLL('kernel32', use_last_error=True).MoveFileExW
        move.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD]
        move.restype = wintypes.BOOL
        if not move(str(source), str(target), 0x1 | 0x8):
            raise ctypes.WinError(ctypes.get_last_error())
    else:
        os.replace(source, target)
    sync_directory(target.parent)


def write_new(path, data):
    with path.open('xb') as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    sync_directory(path.parent)


def encode_json(value):
    data = json.dumps(value, allow_nan=False, separators=(',', ':')).encode()
    require(len(data) <= MAX_METADATA_BYTES, 'temporary metadata byte limit exceeded')
    return data


def write_json(path, value):
    data = encode_json(value)  # Reject before touching the last readable state or its temporary.
    temporary = path.with_name(path.name + '.tmp')
    require(not temporary.is_symlink(), 'temporary metadata must not be a symlink')
    # Only this owned, locked run's fixed metadata temporary may be retried.
    if temporary.exists():
        regular(temporary)
        temporary.unlink()
    write_new(temporary, data)
    replace(temporary, path)


def read_json(path):
    regular(path)
    require(path.stat().st_size <= MAX_METADATA_BYTES, 'temporary metadata byte limit exceeded')
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate temporary metadata field')
            result[key] = value
        return result
    return json.loads(path.read_text(encoding='utf-8'), object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite temporary metadata'))


@contextlib.contextmanager
def writer_lock(root):
    require(real_directory(root), 'checkpoint store must be a real directory')
    path = root / LOCK
    descriptor = os.open(path, os.O_RDWR | os.O_CREAT | getattr(os, 'O_NOFOLLOW', 0), 0o600)
    acquired = False
    try:
        require(stat.S_ISREG(os.fstat(descriptor).st_mode) and not path.is_symlink(), 'invalid writer lock')
        if os.fstat(descriptor).st_size == 0:
            os.write(descriptor, b'\0')
            os.fsync(descriptor)
        os.lseek(descriptor, 0, os.SEEK_SET)
        if os.name == 'nt':
            import msvcrt
            msvcrt.locking(descriptor, msvcrt.LK_NBLCK, 1)
        else:
            import fcntl
            fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        acquired = True
        yield
    except (BlockingIOError, PermissionError) as error:
        if not acquired:
            raise ValueError('checkpoint writer is active; no files were cleaned') from error
        raise
    finally:
        os.close(descriptor)


def pid_alive(pid):
    require(type(pid) is int and pid > 0, 'unknown writer identity; reconcile without deleting its files')
    if os.name == 'nt':
        import ctypes
        kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        kernel.OpenProcess.restype = ctypes.c_void_p
        handle = kernel.OpenProcess(0x1000, False, pid)  # QUERY_LIMITED_INFORMATION
        if not handle:
            # ERROR_INVALID_PARAMETER means no process. Access denied is unknown/alive.
            return ctypes.get_last_error() != 87
        try:
            code = ctypes.c_ulong()
            if not kernel.GetExitCodeProcess(ctypes.c_void_p(handle), ctypes.byref(code)):
                return True
            return code.value == 259
        finally:
            kernel.CloseHandle(ctypes.c_void_p(handle))
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


class Store:
    def __init__(self, root, validate, hook=lambda _: None):
        self.root = Path(root)
        self.validate = validate
        self.hook = hook
        self.locked = False

    @property
    def current(self):
        return self.root / CURRENT

    def __enter__(self):
        self._guard = writer_lock(self.root)
        self._guard.__enter__()
        self.locked = True
        try:
            allowed = {CURRENT, NEXT, PREVIOUS, INTENT, INTENT_NEXT, LOCK, RUN}
            require(set(p.name for p in self.root.iterdir()) <= allowed, 'unknown store entries; refusing cleanup')
            self.recover()
            return self
        except BaseException:
            self.__exit__(None, None, None)
            raise

    def __exit__(self, *args):
        self.locked = False
        return self._guard.__exit__(*args)

    def recover(self):
        require(self.locked, 'writer lock required')
        current, pending, previous, intent = [self.root / n for n in (CURRENT, NEXT, PREVIOUS, INTENT)]
        if not intent.exists():
            require(not intent.is_symlink() and not previous.exists() and not previous.is_symlink(),
                    'unexplained prior checkpoint; refusing cleanup')
            if current.exists():
                self.validate(current)
            else:
                require(not current.is_symlink(), 'current symlink refused')
                if pending.exists():
                    state = self.run_state()
                    require(state.get('mode') == 'import' and 'candidate_sha256' in state,
                            'incomplete initial import; source retained, manual reconciliation required')
                    candidate = self.root / RUN / 'import.alife-checkpoint'
                    self.validate(candidate)
                    require(digest(candidate) == state['candidate_sha256'], 'initial import candidate changed')
                    regular(pending)
                    pending.unlink()  # Incomplete unpublished copy; complete candidate is retained.
                    if (self.root / INTENT_NEXT).exists():
                        regular(self.root / INTENT_NEXT)
                        (self.root / INTENT_NEXT).unlink()
                    self.publish(candidate, None)
                    return
            if pending.exists():
                regular(pending)
                pending.unlink()  # No commit intent; verified current remains authoritative.
                sync_directory(self.root)
            if (self.root / INTENT_NEXT).exists():
                regular(self.root / INTENT_NEXT)
                (self.root / INTENT_NEXT).unlink()
                sync_directory(self.root)
            return
        record = read_json(intent)
        require(set(record) == {'schema', 'before', 'after'} and record['schema'] == 1,
                'invalid publication intent')
        before, after = record['before'], record['after']
        require(isinstance(after, str) and len(after) == 64 and
                (before is None or isinstance(before, str) and len(before) == 64), 'invalid publication hashes')
        hashes = {p.name: digest(p) if p.exists() else None for p in (current, pending, previous)}
        require(not any(p.is_symlink() for p in (current, pending, previous)), 'publication symlink refused')
        if hashes[CURRENT] != after:
            require(hashes[NEXT] == after, 'committed candidate missing or corrupt; prior checkpoint preserved')
            self.validate(pending)
            if before is not None:
                if hashes[CURRENT] == before:
                    self.validate(current)
                    if hashes[PREVIOUS] != before:
                        # A crash while copying the transient prior leaves the
                        # verified current intact. Rebuild only that partial copy.
                        if previous.exists():
                            regular(previous)
                            previous.unlink()
                        with current.open('rb') as source, previous.open('xb') as target:
                            shutil.copyfileobj(source, target, 1024 * 1024)
                            target.flush()
                            os.fsync(target.fileno())
                        sync_directory(self.root)
                        require(digest(previous) == before, 'prior changed during preservation')
                    self.hook('old-preserved')
                else:
                    require(hashes[CURRENT] is None and hashes[PREVIOUS] == before,
                            'current/prior checkpoint changed; refusing recovery')
                    self.validate(previous)
            else:
                require(hashes[CURRENT] is None and hashes[PREVIOUS] is None, 'initial publication collision')
            replace(pending, current)
            self.hook('new-moved')
        else:
            require(hashes[NEXT] is None, 'ambiguous pending checkpoint')
        self.validate(current)
        require(digest(current) == after, 'current changed during admission')
        self.hook('new-verified')
        if previous.exists():
            require(before is not None and digest(previous) == before, 'prior checkpoint changed')
            previous.unlink()
            sync_directory(self.root)
        self.hook('old-removed')
        intent.unlink()
        sync_directory(self.root)
        self.hook('intent-removed')

    def publish(self, candidate, expected):
        require(self.locked, 'writer lock required')
        self.recover()
        actual = digest(self.current) if self.current.exists() else None
        require(actual == expected, 'accepted current changed before publication')
        candidate = Path(candidate)
        self.validate(candidate)
        after = digest(candidate)
        if actual == after:
            return actual
        pending = self.root / NEXT
        with candidate.open('rb') as source, pending.open('xb') as target:
            shutil.copyfileobj(source, target, 1024 * 1024)
            target.flush()
            os.fsync(target.fileno())
        sync_directory(self.root)
        self.hook('candidate-written')
        require(digest(pending) == after, 'candidate changed during publication')
        self.validate(pending)
        self.hook('candidate-verified')
        intent_next = self.root / INTENT_NEXT
        write_new(intent_next, json.dumps({'schema': 1, 'before': actual, 'after': after}).encode())
        self.hook('intent-prepared')
        replace(intent_next, self.root / INTENT)
        self.hook('intent-written')
        self.recover()
        return after

    def run_state(self):
        run = self.root / RUN
        require(real_directory(run), 'invalid temporary run directory')
        state = read_json(run / 'run.json')
        require(state.get('kind') == 'single-current-temporary-run' and state.get('schema') == 1,
                'unknown temporary run owner; refusing cleanup')
        worker = state.get('worker')
        if worker is not None:
            require(set(worker) == {'pid', 'stopped'} and type(worker['stopped']) is bool, 'invalid run writer')
            if not worker['stopped']:
                require(not pid_alive(worker['pid']), 'temporary run writer is active; no files were cleaned')
        return state

    def remove_run(self, expected):
        require(self.locked and digest(self.current) == expected, 'verified current required before cleanup')
        self.validate(self.current)
        state = self.run_state()
        require(state.get('reconciled_current') == expected, 'temporary run has not been reconciled')
        shutil.rmtree(self.root / RUN)
        sync_directory(self.root)

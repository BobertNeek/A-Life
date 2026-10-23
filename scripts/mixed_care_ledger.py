"""Bounded mixed-care documents with an immutable, hash-linked allocation archive.

The caller owns the existing lane lock. All history is reconstructed for the
scheduler's unchanged metadata/receipt checks. Nothing is deleted or retired.
Copied manifests retain an absolute origin: recovery copies must retain the
adjacent .segments directory too. Old readers reject the distinct manifest kind.
"""
import hashlib
import copy
import json
import os
from pathlib import Path
import tempfile

import grab_food_seed_reservations as seeds

LEDGER = "mixed-care-seed-ledger"
MANIFEST = "mixed-care-seed-ledger-manifest"
SEGMENT = "mixed-care-seed-ledger-segment"


def require(value, message):
    if not value:
        raise ValueError(message)


def canonical_hash(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, allow_nan=False).encode()).hexdigest()


def encoded(value):
    data = (json.dumps(value, separators=(",", ":"), allow_nan=False) + "\n").encode()
    require(len(data) <= seeds.MAX_DOCUMENT_BYTES, "mixed ledger document exceeds its bounded size")
    return data


def digest(value):
    return isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value)


def regular(path):
    require(not path.is_symlink() and not (hasattr(path, "is_junction") and path.is_junction()),
            "mixed ledger/archive must not be a symlink or junction")


def raw(path):
    regular(path)
    with path.open("rb") as stream:
        data = stream.read(seeds.MAX_DOCUMENT_BYTES + 1)
    require(len(data) <= seeds.MAX_DOCUMENT_BYTES, "mixed ledger document exceeds its bounded size")
    return data


def sync_directory(path):
    if os.name != "nt":
        descriptor = os.open(path, os.O_RDONLY)
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)


def folder(origin):
    path = origin.with_name(origin.name + ".segments")
    regular(path)
    require(not path.exists() or path.is_dir(), "mixed ledger archive must be a directory")
    return path


def immutable(path, data):
    """Publish a new immutable file exclusively, durably, without replacing one."""
    require(len(data) <= seeds.MAX_DOCUMENT_BYTES, "mixed ledger document exceeds its bounded size")
    expected = hashlib.sha256(data).hexdigest()
    regular(path)
    if path.exists():
        require(hashlib.sha256(raw(path)).hexdigest() == expected, "existing immutable archive changed")
        sync_directory(path.parent)
        return
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="wb", dir=path.parent, prefix=".publishing-", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(data); stream.flush(); os.fsync(stream.fileno())
        try:
            # Same-directory hard-link publication is create-if-absent on NTFS/POSIX.
            # Unsupported publication fails before the manifest changes; no fallback
            # overwrites an immutable file or changes filesystem/security settings.
            os.link(temporary, path)
        except FileExistsError:
            require(hashlib.sha256(raw(path)).hexdigest() == expected, "existing immutable archive changed")
        sync_directory(path.parent)
    finally:
        if temporary is not None and temporary.exists():
            temporary.unlink()


class State(dict):
    def __init__(self, value=None):
        super().__init__(value or {"schema": 1, "kind": LEDGER, "excluded_seed_ranges": [],
                                  "assessment_seeds": [], "runs": []})
        self.origin = None
        self.document_sha256 = None
        self.archived = 0
        self.head = None
        self.legacy_backup = None
        self.legacy = False
        self.prefix_sha256 = canonical_hash([])
        self.protected_prefix = 0
        self.current_before = None
        self.metadata_before = None

    def bind_history(self):
        self.protected_prefix = max(self.archived, max(0, len(self["runs"]) - 1))
        self.prefix_sha256 = canonical_hash(self["runs"][:self.protected_prefix])
        self.current_before = (copy.deepcopy(self["runs"][self.protected_prefix])
                               if len(self["runs"]) > self.protected_prefix else None)
        self.metadata_before = copy.deepcopy({k: v for k, v in self.items() if k != "runs"})


def append_only(state):
    runs = state["runs"]
    require(len(runs) >= state.protected_prefix and
            canonical_hash(runs[:state.protected_prefix]) == state.prefix_sha256,
            "archived history was changed or removed")
    if state.current_before is not None:
        require(len(runs) > state.protected_prefix, "current allocation was removed")
        old = state.current_before; current = runs[state.protected_prefix]
        require({k: v for k, v in old.items() if k != "attempts"} ==
                {k: v for k, v in current.items() if k != "attempts"}, "current allocation metadata changed")
        attempts = seeds._list(current.get("attempts"))
        require(len(attempts) >= len(old["attempts"]), "attempt history was removed")
        for before, after in zip(old["attempts"], attempts):
            if before["status"] == "pending":
                require(isinstance(after, dict) and after.get("status") in ("pending", "accepted", "abandoned") and
                        all(k in after and after[k] == v for k, v in before.items() if k != "status"),
                        "pending attempt identity changed")
            else:
                require(before == after, "sealed or abandoned attempt changed")
    if state.metadata_before is not None:
        before = state.metadata_before
        mutable = {"excluded_seed_ranges", "assessment_seeds"}
        require({k: v for k, v in before.items() if k not in mutable} ==
                {k: v for k, v in state.items() if k not in mutable | {"runs"}}, "ledger metadata changed")
        ranges = seeds._ranges(state["excluded_seed_ranges"])
        require(all(any(a <= first <= last <= b for a, b in ranges)
                    for first, last in seeds._ranges(before["excluded_seed_ranges"])), "seed exclusions were removed")
        require(set(before["assessment_seeds"]) <= set(state["assessment_seeds"]), "assessment exclusions were removed")


def manifest(value, path, *, allow_snapshot=False):
    require(set(value) == {"schema", "kind", "origin", "head", "archived_runs", "legacy_backup", "ledger", "current_runs"}
            and value["kind"] == MANIFEST, "invalid mixed ledger manifest")
    require(isinstance(value["origin"], str) and Path(value["origin"]).is_absolute(), "invalid ledger archive origin")
    origin = Path(value["origin"]).resolve()
    require(allow_snapshot or origin == path.resolve(), "ledger archive origin changed")
    count = value["archived_runs"]
    require(type(count) is int and 0 <= count <= seeds.MAX_ENTRIES, "invalid archived allocation count")
    require((value["head"] is None and count == 0) or (digest(value["head"]) and count > 0), "invalid archive head")
    require(value["legacy_backup"] is None or digest(value["legacy_backup"]), "invalid legacy backup reference")
    base = value["ledger"]
    require(isinstance(base, dict) and type(base.get("schema")) is int and base["schema"] == 1 and base.get("kind") == LEDGER
            and "runs" not in base, "invalid manifest ledger metadata")
    current = seeds._list(value["current_runs"])
    require(len(current) <= 1 and count + len(current) <= seeds.MAX_ENTRIES, "invalid current allocation count")
    folder(origin)
    return origin


def archive_documents(value, origin, read_document, verified=None):
    """Validate every dependency; optional deduplication lives in one admission only."""
    verified = {} if verified is None else verified
    directory = folder(origin)
    backup = value["legacy_backup"]
    if backup is not None:
        path = directory / ("legacy-" + backup + ".json")
        key = (str(path), backup)
        if key not in verified:
            require(hashlib.sha256(raw(path)).hexdigest() == backup, "legacy ledger backup changed")
            verified[key] = None
    head, index, seen = value["head"], value["archived_runs"] - 1, set()
    while head is not None:
        require(digest(head) and head not in seen and index >= 0, "archive cycle or count mismatch")
        seen.add(head)
        path = directory / (head + ".json"); regular(path)
        key = (str(path), head)
        if key not in verified:
            document, actual = read_document(path)
            require(actual == head, "immutable ledger segment changed")
            verified[key] = document
        document = verified[key]
        require(isinstance(document, dict) and set(document) == {"schema", "kind", "index", "previous", "run"}
                and document["kind"] == SEGMENT and type(document["index"]) is int
                and document["index"] == index, "invalid ledger segment order")
        run = document["run"]
        require(isinstance(run, dict) and all(isinstance(a, dict) and a.get("status") in ("accepted", "abandoned")
                                            for a in seeds._list(run.get("attempts"))),
                "archived allocation contains an uncertain attempt")
        yield run
        head = document["previous"]; index -= 1
    require(index == -1, "archive count mismatch")


def read(path, *, read_document=seeds._read, allow_snapshot=False):
    path = Path(path); regular(path)
    value, actual = read_document(path)
    if value.get("kind") == LEDGER:
        state = State(value); state.legacy = True; origin = path.resolve()
    else:
        origin = manifest(value, path, allow_snapshot=allow_snapshot)
        archived = list(archive_documents(value, origin, read_document))
        archived.reverse()
        state = State({**value["ledger"], "runs": archived + value["current_runs"]})
        state.archived = value["archived_runs"]; state.head = value["head"]
        state.legacy_backup = value["legacy_backup"]
    state.origin = origin; state.document_sha256 = actual
    state.bind_history()
    return state


def write(path, state):
    """Commit bounded segments first, then atomically replace the small manifest."""
    path = Path(path); regular(path)
    require(isinstance(state, State), "write requires a validated mixed ledger State")
    require(type(state.get("schema")) is int and state["schema"] == 1 and state.get("kind") == LEDGER,
            "invalid logical mixed ledger kind/schema")
    origin = path.resolve()
    require(state.origin is None or state.origin == origin, "cannot commit a foreign ledger snapshot")
    before = raw(path) if path.exists() else None
    require((before is None and state.document_sha256 is None) or
            (before is not None and hashlib.sha256(before).hexdigest() == state.document_sha256),
            "ledger changed since admission")
    runs = seeds._list(state["runs"])
    append_only(state)
    count = max(0, len(runs) - 1)
    require(count >= state.archived, "archived history cannot become mutable")
    head = state.head; documents = []
    for index in range(state.archived, count):
        run = runs[index]
        require(all(isinstance(a, dict) and a.get("status") in ("accepted", "abandoned")
                    for a in seeds._list(run.get("attempts"))),
                "cannot archive an uncertain allocation")
        data = encoded({"schema": 1, "kind": SEGMENT, "index": index, "previous": head, "run": run})
        head = hashlib.sha256(data).hexdigest(); documents.append((head + ".json", data))
    backup = state.legacy_backup
    if state.legacy:
        backup = state.document_sha256
        documents.insert(0, ("legacy-" + backup + ".json", before))
    value = {"schema": 1, "kind": MANIFEST, "origin": str(origin), "head": head, "archived_runs": count,
             "legacy_backup": backup, "ledger": {k: v for k, v in state.items() if k != "runs"},
             "current_runs": runs[count:]}
    data = encoded(value)  # All size/nonfinite checks precede publication.
    directory = folder(origin)
    if not directory.exists():
        directory.mkdir(); sync_directory(directory.parent)
    for name, payload in documents:
        immutable(directory / name, payload)
    # The existing owner lock is still mandatory. Detect unexpected external writes.
    require((before is None and not path.exists()) or (before is not None and raw(path) == before),
            "ledger changed before manifest commit")
    seeds.write_document(path, value, compact=True)
    state.origin = origin; state.document_sha256 = hashlib.sha256(data).hexdigest()
    state.archived = count; state.head = head; state.legacy_backup = backup; state.legacy = False
    state.bind_history()
    return state.document_sha256


def snapshot_reference(snapshot, source, *, read_document=seeds._read):
    """Tag manifest evidence so its immutable dependencies are checked too."""
    snapshot = Path(snapshot); document, actual = read_document(snapshot)
    result = {"path": str(snapshot.resolve()), "sha256": actual}
    if document.get("kind") == MANIFEST:
        verify_snapshot(snapshot, source, read_document=read_document)
        result["format"] = MANIFEST
    return result


def verify_snapshot(path, source, *, read_document=seeds._read, verified=None):
    value, _ = read_document(path)
    origin = manifest(value, Path(path), allow_snapshot=True)
    require(origin == Path(source).resolve(), "recovery snapshot archive origin changed")
    for _ in archive_documents(value, origin, read_document, verified):
        pass

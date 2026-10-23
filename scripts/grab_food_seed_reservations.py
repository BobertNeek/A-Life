"""Bounded, operator-attested seed history and lane-local reservations.

The caller must hold the lane's existing owner lock when committing. A plan
only calls prepare(); it neither creates nor changes the ledger.
"""
import datetime as dt
import bisect
import hashlib
import json
import os
from pathlib import Path
import tempfile

MAX_SEED = 2**64 - 1
MAX_ENTRIES = 100_000
MAX_DOCUMENT_BYTES = 64 * 1024 * 1024
LEDGER_NAME = ".grab-food-seed-reservations.json"
SOURCE_FILES = ("cycle.json", "trained.alife-foundation", "actor-checkpoint.json", "value-checkpoint.json")


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate seed-history/ledger key: {key}")
        result[key] = value
    return result


def _read(path):
    with Path(path).open("rb") as stream:
        data = stream.read(MAX_DOCUMENT_BYTES + 1)
    if len(data) > MAX_DOCUMENT_BYTES:
        raise ValueError("seed history/ledger exceeds its bounded document size")
    value = json.loads(data, object_pairs_hook=_unique_object)
    if not isinstance(value, dict) or type(value.get("schema")) is not int or value["schema"] != 1:
        raise ValueError("seed history/ledger requires schema 1 object")
    return value, hashlib.sha256(data).hexdigest()


def _seed(value):
    if type(value) is not int or not 1 <= value <= MAX_SEED:
        raise ValueError("seed reservation requires uint64 seeds in 1..2**64-1")
    return value


def _list(value):
    if not isinstance(value, list) or len(value) > MAX_ENTRIES:
        raise ValueError("seed history/ledger requires bounded lists")
    return value


def _ranges(value):
    ranges = []
    for pair in _list(value):
        if not isinstance(pair, list) or len(pair) != 2:
            raise ValueError("training seed ranges require inclusive [first, last] pairs")
        first, last = map(_seed, pair)
        if first > last:
            raise ValueError("training seed range is reversed")
        ranges.append([first, last])
    merged = []
    for first, last in sorted(ranges):
        if merged and first <= merged[-1][1] + 1:
            merged[-1][1] = max(merged[-1][1], last)
        else:
            merged.append([first, last])
    return merged


def _hashes(value):
    if (not isinstance(value, dict) or set(value) != set(SOURCE_FILES) or
            any(not isinstance(digest, str) or len(digest) != 64 or
                any(c not in "0123456789abcdef" for c in digest) for digest in value.values())):
        raise ValueError("seed history requires the exact four source SHA-256 digests")
    return value


def _contains(ranges, seed):
    index = bisect.bisect_right(ranges, [seed, MAX_SEED]) - 1
    return index >= 0 and ranges[index][0] <= seed <= ranges[index][1]


def prepare(history_path, ledger_path, source_hashes, source_seed, first, count, assessment):
    """Validate complete explicit history + surviving lane ledger; reserve nothing."""
    first = _seed(first); assessment = _seed(assessment); source_seed = _seed(source_seed)
    if type(count) is not int or not 1 <= count <= MAX_ENTRIES or first + count - 1 > MAX_SEED:
        raise ValueError("invalid or overflowing seed allocation")
    last = first + count - 1
    history, history_digest = _read(history_path)
    if (history.get("kind") != "grab-food-seed-history" or history.get("complete") is not True or
            not isinstance(history.get("evidence"), str) or not history["evidence"].strip()):
        raise ValueError("complete source-bound seed history with an evidence description is required")
    if _hashes(history.get("source_sha256")) != _hashes(source_hashes):
        raise ValueError("seed history does not match the admitted sealed source")
    training = _ranges(history.get("training_seed_ranges"))
    assessments = {_seed(seed) for seed in _list(history.get("assessment_seeds"))}
    if not _contains(training, source_seed):
        raise ValueError("complete seed history must include the admitted source seed")
    ledger_path = Path(ledger_path)
    if ledger_path.is_symlink():
        raise ValueError("seed ledger must be a regular lane-local file, not a symlink")
    ledger = {"schema": 1, "kind": "grab-food-seed-ledger", "training_seed_ranges": [],
              "assessment_seeds": [], "allocations": []}
    if ledger_path.exists():
        ledger, _ = _read(ledger_path)
        if ledger.get("kind") != "grab-food-seed-ledger":
            raise ValueError("invalid seed ledger kind")
    prior_ranges = _ranges(ledger.get("training_seed_ranges"))
    assessments.update(_seed(seed) for seed in _list(ledger.get("assessment_seeds")))
    allocations = _list(ledger.get("allocations"))
    if len(allocations) == MAX_ENTRIES:
        raise ValueError("seed ledger allocation bound reached; reconcile before continuing")
    # Allocation receipts are also exclusions, even if a top-level interval was
    # accidentally omitted. Invalid surviving receipts stop admission.
    for allocation in allocations:
        if not isinstance(allocation, dict):
            raise ValueError("invalid seed allocation receipt")
        prior_ranges.extend(_ranges([allocation.get("training_seed_range")]))
        assessments.add(_seed(allocation.get("assessment_seed")))
        _hashes(allocation.get("source_sha256"))
        if not all(isinstance(allocation.get(key), str) and allocation[key]
                   for key in ("output", "history_path", "history_sha256", "reserved_utc")):
            raise ValueError("seed allocation receipt lacks provenance")
    training = _ranges(training + prior_ranges)
    if any(first <= end and start <= last for start, end in training):
        raise ValueError("training seed range overlaps previously reserved training seeds")
    if any(first <= seed <= last for seed in assessments | {assessment}):
        raise ValueError("training seed range overlaps a reserved assessment seed")
    if any(_contains(training, seed) for seed in assessments | {assessment}):
        raise ValueError("assessment seed overlaps previously reserved training seeds")
    return {"schema": 1, "kind": "grab-food-seed-ledger",
            "training_seed_ranges": _ranges(training + [[first, last]]),
            "assessment_seeds": sorted(assessments | {assessment}), "allocations": allocations}, history_digest


def commit(path, prepared, allocation):
    """Commit the whole allocation under the caller's existing owner lock.

    A failed commit never authorizes dispatch. A surviving reservation is never
    rolled back, including after an uncertain or zero-training-dose attempt.
    """
    value = {**prepared, "allocations": prepared["allocations"] + [
        {**allocation, "reserved_utc": dt.datetime.now(dt.timezone.utc).isoformat()}]}
    write_document(path, value)


def write_document(path, value, *, compact=False):
    """Durably replace a bounded JSON document; caller owns its lane lock."""
    path = Path(path)
    if path.is_symlink():
        raise ValueError("durable document must not be a symlink")
    formatting = {"separators": (",", ":")} if compact else {"indent": 2}
    payload = (json.dumps(value, allow_nan=False, **formatting) + "\n").encode("utf-8")
    if len(payload) > MAX_DOCUMENT_BYTES:
        raise ValueError("seed ledger exceeds its bounded document size; reconcile before continuing")
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="wb", dir=path.parent,
                                         prefix=path.name + ".", suffix=".tmp", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(payload); stream.flush(); os.fsync(stream.fileno())
        os.replace(temporary, path)
        if os.name != "nt":
            descriptor = os.open(path.parent, os.O_RDONLY)
            try:
                os.fsync(descriptor)
            finally:
                os.close(descriptor)
    finally:
        if temporary is not None and temporary.exists():
            temporary.unlink()

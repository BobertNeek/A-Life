"""Opt-in timings for a host planner; never cache or skip its validations."""
from contextlib import contextmanager
from functools import wraps
from pathlib import Path
import json
import os
import sys
import time


def _write_report(destination, report):
    destination = Path(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(report, indent=2, allow_nan=False) + "\n")


@contextmanager
def profile_planning(mixed, destination, *, metadata=None):
    """Time selected boundaries and restore them even when planning fails."""
    calls = []
    original = []
    started = time.perf_counter()
    cpu_started = time.process_time()
    entered_ns = time.time_ns()
    depth = 0

    def instrument(owner, name, label, hashed_file=False):
        function = getattr(owner, name)
        @wraps(function)
        def measured(*args, **kwargs):
            nonlocal depth
            row = {"stage": label, "depth": depth, "started_unix_ns": time.time_ns()}
            if args and isinstance(args[0], (str, os.PathLike)):
                row["path"] = str(args[0])
                if hashed_file:
                    try:
                        row["hashed_file_bytes"] = Path(args[0]).stat().st_size
                    except OSError:
                        pass
            start, cpu = time.perf_counter(), time.process_time()
            depth += 1
            try:
                return function(*args, **kwargs)
            finally:
                depth -= 1
                row.update(wall_seconds=time.perf_counter() - start,
                           cpu_seconds=time.process_time() - cpu)
                calls.append(row)
        original.append((owner, name, function))
        setattr(owner, name, measured)

    for name in ("load", "read_ledger", "history", "recovery_attestation",
                 "reconcile_prior_runs", "reserve", "file_hashes", "object_hash", "main"):
        if hasattr(mixed, name):
            instrument(mixed, name, name)
    instrument(mixed.host, "sha256", "sha256", hashed_file=True)
    try:
        yield
    finally:
        error = sys.exc_info()[1]
        for owner, name, function in reversed(original):
            setattr(owner, name, function)
        report = {
            "schema": 1, "entered_unix_ns": entered_ns, "finished_unix_ns": time.time_ns(),
            "wall_seconds": time.perf_counter() - started,
            "cpu_seconds": time.process_time() - cpu_started,
            "timings_are_nested": True, "metadata": metadata or {},
            "exception_type": type(error).__name__ if error is not None else None,
            "calls": calls,
        }
        try:
            _write_report(destination, report)
        except Exception as report_error:
            # Diagnostic output must not change the planner's result or cleanup.
            try:
                print(f"host planner timing output unavailable: {report_error}", file=sys.stderr)
            except Exception:
                pass

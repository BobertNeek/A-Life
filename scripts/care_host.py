"""Shared care-host I/O, prior diagnostics and owned-process/resource guards.

Scheduling, seed reservation, checkpoint ownership and learner policy stay with
callers. Importing this module does not launch work or mutate a training lane.
"""
import csv
import ctypes
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time

UTC = dt.timezone.utc

MAX_SEED = 2**64 - 1

MIXED_LEDGER_NAME = ".mixed-care-seed-reservations.json"

def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))

def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()

def write_json(path, value):
    temporary = Path(str(path) + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    temporary.replace(path)

def cutoff_time(text):
    value = dt.datetime.fromisoformat(text.replace("Z", "+00:00"))
    if value.tzinfo is None:
        raise ValueError("cutoff must include a timezone, e.g. 2026-10-06T08:00:00Z")
    return value.astimezone(UTC)

def prior_environment():
    environment = os.environ.copy()
    environment.setdefault("ALIFE_SLM_PRIOR", "on")
    return environment

def assert_prior(receipt, expected_frames=None):
    """Prior diagnostics never reject a genuine, otherwise valid episode."""
    prior = receipt.get("semantic_prior") or {}
    issues = []
    if not isinstance(prior, dict):
        prior = {}
        issues.append("invalid optional prior receipt type")
    # Missing or malformed coverage is unknown, not a measured zero.
    counts = {}
    for key in ("decision_frames", "decision_frames_with_prior", "decision_frames_without_prior"):
        value = prior.get(key)
        if value is not None and (type(value) is not int or value < 0):
            value = None
            issues.append(f"invalid optional prior {key} count")
        counts[key] = value
    frames = counts["decision_frames"]
    inputs = prior.get("decision_inputs", [])
    if not isinstance(inputs, list) or any(not isinstance(row, dict) for row in inputs):
        inputs = []
        issues.append("invalid optional prior trace type")
    if not prior.get("rich_information_required") or not frames or frames != len(inputs):
        issues.append("prior receipt absent or its bounded trace incomplete")
    if expected_frames is not None and frames != expected_frames:
        issues.append("prior receipt count differs from captured learner decisions")
    # PriorDecisionInput identities are u64 wire values. Validate before hashing
    # so malformed optional identities can warn without rejecting the episode.
    valid_identities = all(type(row.get(key)) is int and 0 <= row[key] <= MAX_SEED
                           for row in inputs for key in ("organism", "sequence", "tick"))
    if not valid_identities or len({(row["organism"], row["sequence"]) for row in inputs}) != frames:
        issues.append("prior trace lacks unique confirmed decision identities")
    if any(prior.get(key, 0) for key in
           ("dropout_frames", "failures", "prime_timeouts", "decision_frames_without_prior")):
        issues.append("prior dropout, failure, timeout or missing delivery")
    if prior.get("decision_frames_with_prior") != frames or prior.get("provider_failures"):
        issues.append("prior did not reach every confirmed decision")
    if any(row.get("status") != "delivered" or not row.get("nonzero_prior_lanes") or
           not row.get("nonzero_encoded_prior_lanes") for row in inputs):
        issues.append("some decisions have no semantic encoder delivery")
    if issues:
        print("warning: continuing without complete language priors: " + "; ".join(issues), file=sys.stderr)
    return {"failure_policy": "log_and_continue_without_prior", "warnings": issues, **counts}

def assert_setup(receipt):
    setup = receipt.get("held_food_setup") or {}
    if receipt.get("lesson") != "eat_held_food" or not setup or not setup.get("sampling_seed"):
        raise ValueError("receipt lacks the randomized genuine held-food setup")
    if not 1 <= setup.get("sampling_attempts", 0) <= 32:
        raise ValueError("setup sampling exceeded its bound")
    for key in ("realized_body_position", "realized_grip_position"):
        coordinates(setup.get(key))
    if not math.isfinite(setup.get("realized_body_yaw", float("nan"))):
        raise ValueError("setup lacks its realized facing")
    if (setup.get("initial_sleeping") or setup.get("initial_hunger", 0) < .12 or
            not .2 < setup.get("initial_energy", 0) <= .9 or setup.get("initial_health", 0) < .95):
        raise ValueError("setup lacks healthy awake natural need")
    if not receipt.get("teacher_cue_frames") or receipt.get("vocabulary_token") is not None:
        raise ValueError("ordinary independent teacher cue exposure is missing")
    return setup

def coordinates(value):
    # Vec3f's current native serde shape is a named object, not an XYZ list.
    if not isinstance(value, dict) or set(value) != {"x", "y", "z"}:
        raise ValueError("setup lacks a native realized Vec3f pose")
    values = tuple(value[key] for key in ("x", "y", "z"))
    if not all(isinstance(number, (int, float)) and math.isfinite(number) for number in values):
        raise ValueError("setup lacks a finite realized pose")
    return values

def resource_snapshot(root):
    result = {"disk_free_bytes": shutil.disk_usage(root).free, "available_memory_bytes": None,
              "idle_seconds": None}
    if os.name == "nt":
        class Memory(ctypes.Structure):
            _fields_ = [("length", ctypes.c_ulong), ("load", ctypes.c_ulong)] + [
                (name, ctypes.c_ulonglong) for name in
                ("total_phys", "avail_phys", "total_page", "avail_page", "total_virtual", "avail_virtual", "extended")]
        memory = Memory(); memory.length = ctypes.sizeof(memory)
        if not ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(memory)):
            raise OSError("available-memory measurement failed")
        result["available_memory_bytes"] = memory.avail_phys
        class LastInput(ctypes.Structure):
            _fields_ = [("size", ctypes.c_uint), ("tick", ctypes.c_uint)]
        last = LastInput(); last.size = ctypes.sizeof(last)
        if not ctypes.windll.user32.GetLastInputInfo(ctypes.byref(last)):
            raise OSError("interactive-idle measurement failed")
        result["idle_seconds"] = ((ctypes.windll.kernel32.GetTickCount() & 0xffffffff) - last.tick) % 2**32 / 1000
    else:
        for line in Path("/proc/meminfo").read_text().splitlines():
            if line.startswith("MemAvailable:"):
                result["available_memory_bytes"] = int(line.split()[1]) * 1024
    return result

def stop_owned(process):
    if process.poll() is not None:
        return
    if os.name == "nt":
        result = subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"], capture_output=True)
        if result.returncode and process.poll() is None:
            result.check_returncode()
    else:
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass  # The child may have exited between poll and signal.
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    process.wait(timeout=10)

def run_child(argv, log, deadline, guard, environment, on_started=None):
    guard()
    if dt.datetime.now(UTC) >= deadline:
        raise TimeoutError("command budget exhausted before launch")
    kwargs = {"creationflags": subprocess.CREATE_NO_WINDOW | subprocess.BELOW_NORMAL_PRIORITY_CLASS | subprocess.CREATE_NEW_PROCESS_GROUP} if os.name == "nt" else {"start_new_session": True}
    with Path(str(log) + ".stdout.log").open("xb") as out, Path(str(log) + ".stderr.log").open("xb") as err:
        process = subprocess.Popen(argv, stdout=out, stderr=err, env=environment, **kwargs)
        try:
            if on_started is not None:
                on_started(process)
            while process.poll() is None:
                if dt.datetime.now(UTC) >= deadline:
                    raise TimeoutError("owned command exceeded its wall-time/cutoff budget")
                guard()
                time.sleep(.5)
            if process.returncode:
                raise RuntimeError(f"child exit {process.returncode}; see {log}.*.log")
        finally:
            stop_owned(process)

def assert_lane_idle():
    if os.name == "nt":
        result = subprocess.run(["tasklist", "/FO", "CSV", "/NH"], capture_output=True, text=True, check=True)
        names = [row[0].lower().removesuffix(".exe") for row in csv.reader(result.stdout.splitlines()) if row]
    else:
        names = []
        for path in Path("/proc").glob("[0-9]*/comm"):
            try:
                names.append(path.read_text().strip())
            except (FileNotFoundError, PermissionError):
                pass
    busy = [name for name in names if name in ("cargo", "rustc", "train_n2048_care", "train_n2048_car") or
            name.startswith(("alife_", "wgsl_foundation_trainer", "n2048_foundation_abi"))]
    if busy:
        raise RuntimeError(f"Cargo/game/GPU lane occupied: {busy}; no other process was stopped")

def estimate_capacity(samples, remaining_seconds, free_bytes, reserve_bytes):
    # Conservative maximum observed episode cost plus 25%; never promise an
    # unmeasured PC throughput. Include receipt/checkpoint storage in the bound.
    seconds = max(row["seconds"] for row in samples) * 1.25
    size = max(row["bytes"] for row in samples)
    return {"observed_episodes_per_hour": 3600 / seconds,
            "estimated_remaining_episodes": max(0, min(int(remaining_seconds / seconds),
                int(max(0, free_bytes - reserve_bytes) / max(1, size)))),
            "seconds_per_episode_with_margin": seconds, "max_observed_episode_bytes": size}

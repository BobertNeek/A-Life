#!/usr/bin/env python3
"""Read saved PPO targets from explicitly named cycle/attempt directories."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess
import sys

SIDECAR = "ppo-rollout-diagnostics.json"
MAX_BYTES = 1024 * 1024
ARTIFACTS = ("actor-checkpoint.json", "value-checkpoint.json", "trained.alife-foundation")
FILE_LIMITS = {"cycle.json": 16 * MAX_BYTES, SIDECAR: MAX_BYTES,
               ARTIFACTS[0]: 16 * MAX_BYTES, ARTIFACTS[1]: MAX_BYTES,
               ARTIFACTS[2]: 32 * MAX_BYTES}
INSPECTOR_TIMEOUT = 30


def require(condition, message):
    if not condition:
        raise ValueError(message)


def integer(value, label, maximum=2**64 - 1):
    require(type(value) is int and 0 <= value <= maximum, f"invalid {label}")


def number(value, label):
    require(type(value) in (int, float) and -3.5e38 <= value <= 3.5e38,
            f"nonfinite or invalid {label}")


def _float(text):
    value = float(text)
    require(math.isfinite(value), "nonfinite JSON number")
    return value


def _pairs(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def decode_json(data, label):
    value = json.loads(data, object_pairs_hook=_pairs, parse_float=_float,
                       parse_constant=lambda text: require(False, f"nonfinite JSON: {text}"))
    require(type(value) is dict, f"{label} must be an object")
    return value


def load_json(path, limit=MAX_BYTES):
    with path.open("rb") as stream:
        data = stream.read(limit + 1)
    require(len(data) <= limit, f"{path.name} exceeds byte limit")
    return decode_json(data, path.name), hashlib.sha256(data).hexdigest()


def snapshot(directory):
    result = {}
    for name, limit in FILE_LIMITS.items():
        path = directory / name
        require(path.is_file() and path.stat().st_size <= limit, f"invalid or oversized {name}")
        size, digest = 0, hashlib.sha256()
        with path.open("rb") as stream:
            while chunk := stream.read(65536):
                size += len(chunk)
                require(size <= limit, f"{name} exceeds byte limit")
                digest.update(chunk)
        result[name] = (size, digest.hexdigest())
    return result


def validate_checkpoint(directory, cycle, inspector, cycle_digest, diagnostic_digest):
    require(cycle.get("ppo_rollout_diagnostics_sha256") == diagnostic_digest,
            "sealed cycle diagnostic SHA256 mismatch or missing binding")
    require(inspector is not None and Path(inspector).is_file(),
            "--inspector PATH is required for a potentially committed diagnostic cycle")
    before = snapshot(directory)
    require(before["cycle.json"][1] == cycle_digest and before[SIDECAR][1] == diagnostic_digest,
            "cycle or diagnostic changed after JSON parsing")
    try:
        result = subprocess.run([str(Path(inspector).resolve()), "--validate-checkpoint",
                                 str(directory), "--expected-policy", str(cycle["policy_version"])],
                                capture_output=True, timeout=INSPECTOR_TIMEOUT, check=False)
    except subprocess.TimeoutExpired as error:
        raise ValueError("checkpoint inspector timed out") from error
    require(snapshot(directory) == before, "checkpoint files changed during inspection")
    require(len(result.stdout) <= 65536 and len(result.stderr) <= 65536,
            "checkpoint inspector output exceeds byte limit")
    require(result.returncode == 0, f"checkpoint inspector failed (exit {result.returncode})")
    inspection = decode_json(result.stdout, "checkpoint inspector output")
    files = inspection.get("file_digests")
    expected_files = {"cycle.json", *ARTIFACTS}
    require(type(files) is list and len(files) == len(expected_files),
            "inspector must bind exactly four checkpoint files")
    checked = set()
    for entry in files:
        require(type(entry) is dict, "invalid inspector file digest entry")
        name = entry.get("file_name")
        require(type(name) is str and name in expected_files and name not in checked,
                "inspector file digest names are missing, duplicated or unexpected")
        checked.add(name)
        integer(entry.get("encoded_bytes"), "inspector encoded_bytes")
        require((entry["encoded_bytes"], entry.get("sha256")) == before[name],
                f"inspector file digest mismatch: {name}")
    for key in ("expected_policy_version", "policy_version", "founder_seed_base",
                "actor_optimizer_step", "value_optimizer_step"):
        integer(inspection.get(key), f"inspector {key}")
        expected = cycle.get("policy_version" if key == "expected_policy_version" else key)
        integer(expected, f"cycle {key}")
        require(inspection[key] == expected, f"checkpoint inspector {key} mismatch")
    digest = inspection.get("foundation_digest")
    require(type(digest) is list and len(digest) == 32, "invalid inspector foundation_digest")
    for byte in digest:
        integer(byte, "inspector foundation_digest", 255)
    require(bytes(digest).hex() == cycle.get("new_asset_digest"), "inspector foundation digest mismatch")
    require(inspection.get("neural_execution") is False and inspection.get("weights_modified") is False,
            "checkpoint inspector must confirm no neural execution or weight modification")


def validate(diagnostic, cycle):
    require(type(diagnostic.get("schema_version")) is int
            and diagnostic["schema_version"] == 1, "unsupported schema_version")
    require(diagnostic.get("phase") == "prepared_before_optimizer_update"
            and diagnostic.get("training_commit_requires_cycle_receipt") is True,
            "unsupported diagnostic phase or commit contract")
    for key in ("sampling_temperature", "discount_half_life_seconds", "gae_half_life_seconds"):
        number(diagnostic.get(key), key)
        require(diagnostic[key] > 0, f"{key} must be positive")
    for key in ("policy_version", "seed", "total_rows", "emitted_rows"):
        integer(diagnostic.get(key), key)
    total, emitted = diagnostic["total_rows"], diagnostic["emitted_rows"]
    rows = diagnostic.get("rows")
    require(1 <= total <= 36000 and emitted == min(total, 64)
            and type(rows) is list and len(rows) == emitted, "invalid bounded row counts")
    require(type(diagnostic.get("rows_truncated")) is bool
            and diagnostic["rows_truncated"] == (emitted < total), "invalid rows_truncated")
    source = diagnostic.get("source")
    require(type(source) is dict, "missing source identity")
    require(type(source.get("source_revision")) is str and source["source_revision"],
            "invalid source_revision")
    integer(source.get("policy_version"), "source policy_version")
    require(source["policy_version"] == diagnostic["policy_version"], "source policy mismatch")
    for key, length, maximum in (("actor_checkpoint_digest", 32, 255),
                                 ("foundation_asset_digest", 32, 255),
                                 ("phenotype_hash", 4, 2**64 - 1),
                                 ("compiler_inputs_digest", 4, 2**64 - 1)):
        digest = source.get(key)
        require(type(digest) is list and len(digest) == length, f"invalid source {key}")
        for item in digest:
            integer(item, f"source {key}", maximum)
    previous_tick = -1
    for index, row in enumerate(rows):
        require(type(row) is dict, "invalid row object")
        for key in ("row", "step", "trajectory_id", "decision_tick"):
            integer(row.get(key), f"row {key}")
        require(row["row"] == index and row["step"] == index
                and row["trajectory_id"] == diagnostic["seed"], "row identity mismatch")
        require(row["decision_tick"] > previous_tick, "nonmonotonic decision ticks")
        previous_tick = row["decision_tick"]
        for key in ("reward", "old_value", "next_value", "elapsed_seconds", "gamma",
                    "lambda", "advantage", "return"):
            number(row.get(key), f"row {key}")
        require(row["elapsed_seconds"] > 0 and 0 <= row["gamma"] <= 1
                and 0 <= row["lambda"] <= 1, "invalid row time/discount")
        require(row.get("boundary") in ("continuing", "terminated", "truncated"),
                "invalid row boundary")
        require(index == total - 1 or row["boundary"] == "continuing",
                "premature row boundary")
    tick, value = diagnostic.get("bootstrap_decision_tick"), diagnostic.get("bootstrap_value")
    require((tick is None) == (value is None), "incomplete bootstrap identity")
    terminal = diagnostic.get("terminal_death_tick")
    require((terminal is None) == (tick is not None), "invalid terminal/bootstrap boundary")
    if terminal is not None:
        integer(terminal, "terminal_death_tick")
        require(terminal > previous_tick, "invalid terminal tick")
    if tick is not None:
        integer(tick, "bootstrap_decision_tick")
        number(value, "bootstrap_value")
        require(tick > previous_tick, "invalid bootstrap tick")
    if emitted == total:
        require(rows[-1]["boundary"] == ("terminated" if tick is None else "truncated"),
                "invalid final row boundary")
        require(tick is not None or rows[-1]["next_value"] == 0, "nonzero terminal next_value")
    if cycle is not None:
        for key in ("policy_version", "seed"):
            integer(cycle.get(key), f"cycle {key}")
            require(cycle[key] == diagnostic[key], f"cycle {key} mismatch")
        integer(cycle.get("trained_replay_rows"), "cycle trained_replay_rows")
        require(cycle["trained_replay_rows"] == total, "cycle trained row count mismatch")
        require(cycle.get("terminal_death_tick") == terminal, "cycle terminal boundary mismatch")
        require(bytes(source["foundation_asset_digest"]).hex() == cycle.get("old_asset_digest"),
                "source foundation digest mismatch")
    return rows


def summarize(directories, inspector=None):
    report = {"schema_version": 1, "committed_diagnostic_cycles": 0, "emitted_rows": 0,
              "total_rows_in_diagnostic_cycles": 0, "omitted_rows_unknown": 0,
              "truncated_diagnostic_cycles": 0, "reward_sum": 0.0, "advantage_sum": 0.0,
              "negative_advantage_rows": 0, "positive_reward_negative_advantage_rows": 0,
              "missing_diagnostics": [], "excluded_unsealed": [], "errors": [], "duplicate_aliases": [],
              "competence": "not_assessed; aggregates cover emitted committed targets only"}
    seen = {}
    for directory in sorted({Path(path).resolve() for path in directories}):
        try:
            require(directory.is_dir(), "input must be a cycle/attempt directory")
            cycle_path = directory / "cycle.json"
            cycle, cycle_digest = (load_json(cycle_path, FILE_LIMITS["cycle.json"])
                                   if cycle_path.is_file() else (None, None))
            diagnostic_path = directory / SIDECAR
            diagnostic, diagnostic_digest = (load_json(diagnostic_path)
                                             if diagnostic_path.is_file() else (None, None))
            rows = validate(diagnostic, cycle) if diagnostic is not None else None
            sealed = (cycle is not None and cycle.get("next_cohort_tick_captured") is True
                      and cycle.get("next_cohort_optimizer_rebound") is True
                      and all((directory / name).is_file() for name in ARTIFACTS))
            if not sealed:
                report["excluded_unsealed"].append(str(directory))
                continue
            if diagnostic is None:
                report["missing_diagnostics"].append(str(directory))
                continue
            validate_checkpoint(directory, cycle, inspector, cycle_digest, diagnostic_digest)
            source = diagnostic["source"]
            identity = (diagnostic["policy_version"], diagnostic["seed"],
                        tuple(source["foundation_asset_digest"]), tuple(source["actor_checkpoint_digest"]))
            digest = hashlib.sha256(json.dumps(diagnostic, sort_keys=True, separators=(",", ":"),
                                               allow_nan=False).encode()).hexdigest()
            if identity in seen:
                previous_digest, previous_directory = seen[identity]
                require(digest == previous_digest, f"conflicting diagnostics for learning identity: {previous_directory}")
                report["duplicate_aliases"].append({"directory": str(directory), "same_as": previous_directory})
                continue
            seen[identity] = (digest, str(directory))
            report["committed_diagnostic_cycles"] += 1
            report["emitted_rows"] += len(rows)
            report["total_rows_in_diagnostic_cycles"] += diagnostic["total_rows"]
            report["omitted_rows_unknown"] += diagnostic["total_rows"] - len(rows)
            report["truncated_diagnostic_cycles"] += int(diagnostic["rows_truncated"])
            report["reward_sum"] += math.fsum(row["reward"] for row in rows)
            report["advantage_sum"] += math.fsum(row["advantage"] for row in rows)
            report["negative_advantage_rows"] += sum(row["advantage"] < 0 for row in rows)
            report["positive_reward_negative_advantage_rows"] += sum(
                row["reward"] > 0 and row["advantage"] < 0 for row in rows)
        except (OSError, ValueError, TypeError, RecursionError, subprocess.SubprocessError) as error:
            report["errors"].append({"directory": str(directory), "error": str(error)})
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directories", nargs="+", type=Path,
                        help="Explicit cycle/attempt directories; no recursive scan or writes")
    parser.add_argument("--inspector", type=Path,
                        help="Required native inspect_archived_success binary for committed diagnostics")
    args = parser.parse_args()
    if len(args.directories) > 4096:
        parser.error("at most 4096 directories per report")
    report = summarize(args.directories, args.inspector)
    print(json.dumps(report, indent=2, allow_nan=False))
    return int(bool(report["errors"]))


if __name__ == "__main__":
    sys.exit(main())

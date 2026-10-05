#!/usr/bin/env python3
"""Plan by default; explicitly run fresh held-food episodes through the GPU CLI.

One sealed cycle is one prepared target opportunity. Fresh worlds retain the
offline actor/value/optimizer chain, not a creature's personal lifetime state.
Only standard-library host orchestration lives here; no learner policy does.
"""
import argparse
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
import time

UTC = dt.timezone.utc
MAX_SEED = 2**64 - 1


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


def source_info(source):
    seals = [name for name in ("cycle.json", "warmup.json", "adaptation.json") if (source / name).is_file()]
    if len(seals) != 1:
        raise ValueError("source must contain exactly one unambiguous sealed handoff receipt")
    for name in ("cycle.json", "warmup.json", "adaptation.json"):
        if (source / name).is_file():
            receipt = read_json(source / name)
            founder = receipt.get("founder_seed_base") or receipt.get("seed")
            if not isinstance(founder, int) or not 1 <= founder <= MAX_SEED:
                raise ValueError("source has no valid founder seed")
            files = [source / name, source / "trained.alife-foundation"]
            if name != "adaptation.json":
                files += [source / "actor-checkpoint.json", source / "value-checkpoint.json"]
            return founder, {str(path): sha256(path) for path in files}
    raise ValueError("source must be a sealed cycle, warm-up or explicit adaptation")


def assert_prior(receipt, expected_frames=None):
    prior = receipt.get("semantic_prior") or {}
    frames = prior.get("decision_frames", 0)
    inputs = prior.get("decision_inputs", [])
    if not prior.get("rich_information_required") or not frames or frames != len(inputs):
        raise ValueError("rich prior receipt is absent or its bounded trace is incomplete")
    if expected_frames is not None and frames != expected_frames:
        raise ValueError("prior receipt count does not match the captured learner decisions")
    if len({(row.get("organism"), row.get("sequence")) for row in inputs}) != frames or any(
            row.get("organism") is None or row.get("sequence") is None or row.get("tick") is None for row in inputs):
        raise ValueError("prior receipts lack unique confirmed decision identities")
    if any(prior.get(key, 0) for key in
           ("dropout_frames", "failures", "prime_timeouts", "decision_frames_without_prior")):
        raise ValueError("rich prior has dropout, failure, timeout or missing delivery")
    if prior.get("decision_frames_with_prior") != frames or prior.get("provider_failures"):
        raise ValueError("rich prior did not reach every confirmed decision")
    if any(row.get("status") != "delivered" or not row.get("nonzero_prior_lanes") or
           not row.get("nonzero_encoded_prior_lanes") for row in inputs):
        raise ValueError("actual semantic encoder delivery is missing")


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


def validate_probe(directory):
    receipt = read_json(directory / "pilot.json")
    setup = assert_setup(receipt)
    assert_prior(receipt, receipt["ticks"])
    if receipt.get("held_food_terminal_death_tick") is not None:
        raise ValueError("assessment learner died")
    trace_path = directory / "lesson-trace.json"
    if not trace_path.is_file():
        trace_path = directory / "lesson-diagnostic.json"
    trace = read_json(trace_path)
    # Eat action 210 is the registered world primitive. Require actual offered
    # support for the real held target, never infer it from task success.
    offered = False
    for step in trace.get("steps", []):
        masks = [step.get("representative_mask", 0)] + step.get("motor_masks", [])
        for candidate in step.get("candidates", []):
            if (step.get("lesson_target_held_before_decision") and candidate.get("action") == 210
                    and candidate.get("target") == setup["target"]
                    and any(mask & (1 << candidate["index"]) for mask in masks)):
                offered = True
    if not offered:
        raise ValueError("assessment never offered selectable Eat for the genuinely held target")
    return {"held_food_meals": receipt.get("held_food_consumption_events", 0),
            "decisions": receipt["ticks"], "directory": str(directory)}


def validate_cycle(directory, previous, seed):
    receipt = read_json(directory / "cycle.json")
    setup = assert_setup(receipt)
    assert_prior(receipt, receipt["captured_replay_rows"])
    if (receipt.get("seed") != seed or receipt.get("terminal_death_tick") is not None or
            not receipt.get("next_cohort_tick_captured") or
            not receipt.get("next_cohort_optimizer_rebound") or receipt.get("speech_target_rows")):
        raise ValueError("cycle did not seal the healthy nonlanguage optimizer handoff")
    if receipt.get("prepared_held_food_opportunities") != 1:
        raise ValueError("cycle must identify exactly one prepared opportunity")
    rows = receipt.get("trained_replay_rows", 0)
    bootstrap = receipt.get("bootstrap_replay_rows", 0)
    if (rows < 1 or rows != receipt.get("training_ticks") or
            receipt.get("captured_replay_rows") != rows + bootstrap or bootstrap != 1):
        raise ValueError("captured, loss and bootstrap row counts disagree")
    _, files = source_info(directory)
    if sha256(previous / "trained.alife-foundation") != sha256(directory / "initial.alife-foundation"):
        raise ValueError("cycle did not start from the exact preceding exported asset")
    if (previous / "cycle.json").is_file():
        old = read_json(previous / "cycle.json")
        if receipt.get("objective_state_reset") or receipt["policy_version"] != old["policy_version"] + 1:
            raise ValueError("continuation reset its objective or lost policy age")
        for head in ("actor", "value"):
            if receipt[f"{head}_optimizer_step"] - old[f"{head}_optimizer_step"] != receipt[f"{head}_optimizer_updates"]:
                raise ValueError("continuation lost its optimizer age")
    for head in ("actor", "value"):
        if receipt.get(f"{head}_optimizer_updates", 0) < 1:
            raise ValueError("cycle has no optimizer updates")
    return receipt, setup, files


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
        subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"], capture_output=True, check=True)
    else:
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
    process.wait(timeout=10)


def run_child(argv, log, deadline, guard, environment):
    guard()
    if dt.datetime.now(UTC) >= deadline:
        raise TimeoutError("command budget exhausted before launch")
    kwargs = {"creationflags": subprocess.BELOW_NORMAL_PRIORITY_CLASS | subprocess.CREATE_NEW_PROCESS_GROUP} if os.name == "nt" else {"start_new_session": True}
    with Path(str(log) + ".stdout.log").open("xb") as out, Path(str(log) + ".stderr.log").open("xb") as err:
        process = subprocess.Popen(argv, stdout=out, stderr=err, env=environment, **kwargs)
        try:
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
    busy = [name for name in names if name in ("cargo", "rustc", "train_n2048_care") or
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


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cutoff", type=cutoff_time, required=True)
    parser.add_argument("--seed", type=int, default=2026100502)
    parser.add_argument("--assessment-seed", type=int, default=2026100501)
    parser.add_argument("--target-episodes", type=int, default=3000)
    parser.add_argument("--batch-episodes", type=int, default=16)
    parser.add_argument("--decisions", type=int, default=16)
    parser.add_argument("--startup-decisions", type=int, default=128)
    parser.add_argument("--startup-seconds", type=int, default=2700)
    parser.add_argument("--command-seconds", type=int, default=1200)
    parser.add_argument("--assessment-seconds", type=int, default=600)
    parser.add_argument("--final-reserve-minutes", type=int, default=15)
    parser.add_argument("--min-disk-gib", type=float, default=8)
    parser.add_argument("--min-memory-gib", type=float, default=2)
    parser.add_argument("--idle-seconds", type=int, default=30)
    parser.add_argument("--startup-grace-seconds", type=int, default=30)
    parser.add_argument("--run", action="store_true", help="explicitly launch; default only writes a plan")
    return parser


def main(argv=None):
    parser = arguments(); args = parser.parse_args(argv)
    try:
        if not 1 <= args.target_episodes <= 100_000 or not 1 <= args.batch_episodes <= 64:
            raise ValueError("target must be 1..100000 episodes; batches 1..64")
        if not 1 <= args.decisions <= 128 or not 1 <= args.startup_decisions <= 128:
            raise ValueError("decision windows must be 1..128 to retain every bounded prior receipt")
        if not 1 <= args.seed <= MAX_SEED - args.target_episodes or not 1 <= args.assessment_seed <= MAX_SEED:
            raise ValueError("invalid or overflowing seed range")
        if args.seed <= args.assessment_seed < args.seed + args.target_episodes:
            raise ValueError("frozen assessment seed must be excluded from the training range")
        if (not 1 <= args.startup_seconds <= 3600 or not 1 <= args.command_seconds <= 3600 or not 1 <= args.assessment_seconds <= 1800 or
                not 1 <= args.final_reserve_minutes <= 60 or args.min_disk_gib <= 0 or
                args.min_memory_gib <= 0 or not math.isfinite(args.min_disk_gib) or
                not math.isfinite(args.min_memory_gib) or args.idle_seconds < 0 or not 0 <= args.startup_grace_seconds <= 300):
            raise ValueError("invalid command, reserve or resource bounds")
        if args.cutoff <= dt.datetime.now(UTC):
            raise ValueError("morning cutoff is already past")
        if args.cutoff - dt.datetime.now(UTC) > dt.timedelta(hours=24):
            raise ValueError("morning cutoff must be within the next 24 hours")
        args.binary = args.binary.resolve(strict=True); args.source = args.source.resolve(strict=True)
        founder, input_hashes = source_info(args.source)
        if (args.source / "cycle.json").is_file():
            source_cycle = read_json(args.source / "cycle.json")
            source_seed = source_cycle.get("seed")
            if source_seed is not None and (args.seed <= source_seed < args.seed + args.target_episodes or
                    source_cycle.get("lesson") == "eat_held_food" and args.seed <= source_seed):
                raise ValueError("training seed range repeats the source episode; select a fresh range")
        input_hashes[str(args.binary)] = sha256(args.binary)
        args.output = args.output.resolve()
        # Do not acquire the owner lock or start any executable in plan mode.
        args.output.mkdir(parents=True, exist_ok=False)
        state = {"schema": 1, "status": "planned", "lesson": "eat_held_food", "promoted": False,
                 "target_distinct_episodes": args.target_episodes, "batch_episodes": args.batch_episodes,
                 "decisions_per_episode": args.decisions, "startup_decisions": args.startup_decisions,
                 "startup_cap_seconds": args.startup_seconds,
                 "cutoff_utc": args.cutoff.isoformat(), "input_sha256": input_hashes,
                 "source": str(args.source), "latest_checkpoint": str(args.source),
                 "founder_seed_base": founder, "assessment_seed": args.assessment_seed,
                 "training_seed_range": [args.seed, args.seed + args.target_episodes - 1],
                 "distinct_completed_episodes": 0, "captured_decision_rows": 0, "trained_decision_rows": 0,
                 "bootstrap_rows": 0, "captured_held_meals": 0, "trained_row_held_meals": 0,
                 "completed_epoch_repeats": 0, "actor_optimizer_updates": 0, "value_optimizer_updates": 0,
                 "individual_lifetime_continues_between_episodes": False,
                 "trained_parameters_and_optimizer_continue": True, "episodes": [], "assessments": [],
                 "partial_episode": None, "error": None,
                 "resource_limits": {"min_disk_gib": args.min_disk_gib,
                                     "min_memory_gib": args.min_memory_gib, "idle_seconds": args.idle_seconds}}
        state["resource_limits"]["startup_grace_seconds"] = args.startup_grace_seconds
        state["first_episode_argv"] = [str(args.binary), "--resume-cycle", str(args.source),
            str(args.output / "episode-00000"), str(args.startup_decisions), "--seed", str(args.seed),
            "--lesson", "eat_held_food"]
        state["episode_schedule"] = "each accepted episode resumes the immediately preceding sealed cycle; one new seed and realized pose per episode"
        state_path = args.output / "night.json"
        write_json(state_path, state)
        if not args.run:
            print(json.dumps(state, indent=2)); return 0
        run_night(args, state, state_path, input_hashes)
        print(json.dumps({key: value for key, value in state.items() if key != "episodes"}, indent=2))
        return 0 if state["status"] == "completed" else 1
    except (ValueError, OSError) as error:
        parser.error(str(error))


def run_night(args, state, state_path, input_hashes):
    lock_path = args.output.parent / ".eat-held-food-owner.lock"
    descriptor = None
    environment = os.environ.copy(); environment["ALIFE_SLM_PRIOR"] = "on"
    samples = []; setup_keys = set(); previous = args.source
    checkpoint_hashes = input_hashes.copy()
    collection_cutoff = args.cutoff - dt.timedelta(minutes=args.final_reserve_minutes)
    startup_cutoff = min(collection_cutoff, dt.datetime.now(UTC) + dt.timedelta(seconds=args.startup_seconds))
    assessment_costs = []
    interactive_grace_until = dt.datetime.now(UTC) + dt.timedelta(seconds=args.startup_grace_seconds)

    def publish():
        state["updated_utc"] = dt.datetime.now(UTC).isoformat()
        write_json(state_path, state)

    def guard():
        if (args.output / "STOP").exists():
            raise RuntimeError("owner STOP file requested interruption")
        snapshot = resource_snapshot(args.output)
        state["last_resources"] = snapshot
        if snapshot["disk_free_bytes"] < args.min_disk_gib * 2**30:
            raise RuntimeError("disk reserve reached")
        if snapshot["available_memory_bytes"] is None or snapshot["available_memory_bytes"] < args.min_memory_gib * 2**30:
            raise RuntimeError("available-memory reserve reached or measurement unavailable")
        if (dt.datetime.now(UTC) >= interactive_grace_until and args.idle_seconds and
                os.name == "nt" and snapshot["idle_seconds"] < args.idle_seconds):
            raise RuntimeError("interactive PC use resumed; stopped only the owned child")

    def invoke(argv, name, seconds, cutoff):
        for path, expected in checkpoint_hashes.items():
            if sha256(path) != expected:
                raise RuntimeError(f"source/checkpoint/binary changed: {path}")
        assert_lane_idle()
        run_child(argv, args.output / name, min(cutoff, dt.datetime.now(UTC) + dt.timedelta(seconds=seconds)),
                  guard, environment)

    def probe(name, source, cutoff):
        directory = args.output / name
        started = time.monotonic()
        invoke([str(args.binary), "--evaluate", str(directory), str(source / "trained.alife-foundation"),
                "eat_held_food", "16", str(args.assessment_seed), str(state["founder_seed_base"])],
               name, args.assessment_seconds, cutoff)
        result = validate_probe(directory); state["assessments"].append(result); publish()
        assessment_costs.append(time.monotonic() - started)
        return result

    try:
        descriptor = os.open(lock_path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
        os.write(descriptor, json.dumps({"pid": os.getpid(), "output": str(args.output)}).encode())
        state["status"] = "running"; state["started_utc"] = dt.datetime.now(UTC).isoformat(); publish()
        # Explicit readiness/model hashes remain mandatory, not an assumption
        # derived from setting ALIFE_SLM_PRIOR=on in this owned child.
        claim = environment.get("ALIFE_SLM_PRIOR_MODEL_SHA256", "")
        if not environment.get("ALIFE_SLM_PRIOR_MODEL") or len(claim) != 64 or any(c not in "0123456789abcdefABCDEF" for c in claim):
            raise ValueError("set the verified local model alias and actual GGUF SHA256 before --run")
        repo = Path(__file__).resolve().parent.parent
        if Path.cwd().resolve() != repo:
            raise ValueError("run from the authoritative repository root")
        if args.output.parent != repo / "target" / "founder-training":
            raise ValueError("run output must be a fresh directory directly under target/founder-training for one shared owner lock")
        git = subprocess.run(["git", "status", "--porcelain"], capture_output=True, text=True, check=True)
        if git.stdout.strip():
            raise ValueError("run requires clean reviewed repository source")
        state["git_head"] = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
        before = probe("frozen-before", previous, startup_cutoff)
        observed_frozen_meal = bool(before["held_food_meals"])
        regressions = 0
        for index in range(args.target_episodes):
            if dt.datetime.now(UTC) >= collection_cutoff:
                break
            if index and samples:
                remaining = (collection_cutoff - dt.datetime.now(UTC)).total_seconds()
                # Replace the longer startup estimate once ordinary windows
                # have been measured; do not count assessment rows as examples.
                ordinary_samples = [{**sample, "seconds": sample["seconds"] + max(assessment_costs) / args.batch_episodes}
                                    for sample in (samples[1:] or samples)]
                estimate = estimate_capacity(ordinary_samples, remaining, shutil.disk_usage(args.output).free,
                                             args.min_disk_gib * 2**30)
                estimate["ordinary_episode_samples"] = max(0, len(samples) - 1)
                estimate["provisional"] = len(samples) - 1 < 3
                state["throughput_estimate"] = estimate
                state["target_attainable_at_measured_rate"] = None if estimate["provisional"] else (
                    estimate["estimated_remaining_episodes"] >= args.target_episodes - index)
                if remaining < estimate["seconds_per_episode_with_margin"]:
                    break
            guard()
            seed = args.seed + index
            directory = args.output / f"episode-{index:05d}"
            state["partial_episode"] = str(directory); publish()
            started = time.monotonic()
            invoke([str(args.binary), "--resume-cycle", str(previous), str(directory),
                    str(args.startup_decisions if index == 0 else args.decisions),
                    "--seed", str(seed), "--lesson", "eat_held_food"],
                   f"episode-{index:05d}", args.command_seconds,
                   startup_cutoff if index == 0 else collection_cutoff)
            receipt, setup, hashes = validate_cycle(directory, previous, seed)
            key = coordinates(setup["realized_body_position"]) + (setup["realized_body_yaw"],)
            if key in setup_keys:
                raise ValueError("episode repeated a realized location/facing setup")
            setup_keys.add(key)
            seconds = time.monotonic() - started
            size = sum(path.stat().st_size for path in directory.rglob("*") if path.is_file())
            samples.append({"seconds": seconds, "bytes": size})
            previous = directory
            checkpoint_hashes = {str(args.binary): input_hashes[str(args.binary)], **hashes}
            state["latest_checkpoint"] = str(directory); state["partial_episode"] = None
            state["distinct_completed_episodes"] += 1
            for total, field in (("captured_decision_rows", "captured_replay_rows"),
                                 ("trained_decision_rows", "trained_replay_rows"), ("bootstrap_rows", "bootstrap_replay_rows"),
                                 ("captured_held_meals", "held_food_consumption_events"),
                                 ("trained_row_held_meals", "trained_held_food_consumption_events"),
                                 ("completed_epoch_repeats", "completed_epochs"),
                                 ("actor_optimizer_updates", "actor_optimizer_updates"),
                                 ("value_optimizer_updates", "value_optimizer_updates")):
                state[total] += receipt[field]
            state["episodes"].append({"index": index, "seed": seed, "setup": setup, "seconds": seconds,
                                      "bytes": size, "input_sha256": hashes, "policy_version": receipt["policy_version"]})
            publish()
            if index == 0 or (index + 1) % args.batch_episodes == 0:
                current = probe(f"frozen-after-{index:05d}", previous,
                                startup_cutoff if index == 0 else collection_cutoff)
                if index == 0:
                    state["startup_qualified"] = True
                print(f"Held-food episodes {index + 1}/{args.target_episodes}; trained rows {state['trained_decision_rows']}; "
                      f"last sealed checkpoint {previous}; estimate {state.get('throughput_estimate', 'startup measurement')}", flush=True)
                regressions = regressions + 1 if observed_frozen_meal and not current["held_food_meals"] else 0
                observed_frozen_meal = observed_frozen_meal or bool(current["held_food_meals"])
                if regressions >= 2:
                    raise ValueError("two frozen batch checks lost previously observed eating")
        # A deadline interruption retains the last sealed source; never resume
        # partial collection. The reserved final probe is a separate assessment.
        probe("frozen-final", previous, args.cutoff)
        state["status"] = "completed" if state["distinct_completed_episodes"] == args.target_episodes else "incomplete"
    except KeyboardInterrupt:
        state["status"] = "interrupted"; state["error"] = "owner interrupted the runner"
    except TimeoutError as error:
        state["status"] = "interrupted"; state["error"] = str(error)
        if state.get("startup_qualified") and dt.datetime.now(UTC) < args.cutoff:
            try:
                probe("frozen-final-after-interruption", previous, args.cutoff)
            except Exception as assessment_error:
                state["final_assessment_error"] = str(assessment_error)
    except Exception as error:
        state["status"] = "failed"
        state["error"] = str(error)
    finally:
        try:
            state["ended_utc"] = dt.datetime.now(UTC).isoformat(); publish()
        finally:
            if descriptor is not None:
                os.close(descriptor); lock_path.unlink()


if __name__ == "__main__":
    raise SystemExit(main())

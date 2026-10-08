#!/usr/bin/env python3
"""Plan by default; explicitly continue sealed brains on reachable Grab-only lives.

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
import struct
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
    if seals != ["cycle.json"]:
        raise ValueError("Grab continuation requires an existing sealed actor/value cycle; adaptations and warm-ups are ineligible")
    receipt = read_json(source / "cycle.json")
    if (receipt.get("biological_objective_version") != 2 or receipt.get("objective_state_reset") or
            not receipt.get("next_cohort_tick_captured") or not receipt.get("next_cohort_optimizer_rebound")):
        raise ValueError("continuation requires compatible objective2 and completed optimizer rebind")
    founder = receipt.get("founder_seed_base")
    if not isinstance(founder, int) or not 1 <= founder <= MAX_SEED:
        raise ValueError("source has no valid founder seed")
    for head in ("actor", "value"):
        checkpoint = read_json(source / f"{head}-checkpoint.json")
        if checkpoint.get("optimizer_step") != receipt.get(f"{head}_optimizer_step"):
            raise ValueError(f"{head} checkpoint age disagrees with its sealed cycle")
    files = [source / name for name in ("cycle.json", "trained.alife-foundation",
        "actor-checkpoint.json", "value-checkpoint.json")]
    return founder, {str(path): sha256(path) for path in files}


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


def assert_held_setup(receipt):
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


def validate_retention_probe(directory):
    receipt = read_json(directory / "pilot.json")
    setup = assert_held_setup(receipt)
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


def assert_setup(receipt):
    setup = receipt.get("grab_food_setup") or {}
    if receipt.get("lesson") != "grab_food" or not setup or receipt.get("held_food_setup"):
        raise ValueError("receipt lacks genuine unheld Grab-only setup")
    if (not setup.get("sampling_seed") or not 1 <= setup.get("sampling_attempts", 0) <= 32 or
            setup.get("initial_owner") is not None or setup.get("initial_consumed")):
        raise ValueError("Grab setup was held/consumed or has invalid sampled provenance")
    position = coordinates(setup.get("realized_body_position"))
    food = coordinates(setup.get("food_position"))
    if not math.isclose(math.dist(position, food), .5, abs_tol=.0002):
        raise ValueError("Grab setup food is outside its prepared reach")
    if not math.isfinite(setup.get("realized_body_yaw", float("nan"))):
        raise ValueError("Grab setup lacks realized facing")
    if (setup.get("initial_sleeping") or setup.get("initial_hunger", 0) < .12 or
            not .2 < setup.get("initial_energy", 0) <= .9 or setup.get("initial_health", 0) < .95):
        raise ValueError("Grab setup lacks healthy awake natural need")
    if not receipt.get("teacher_cue_frames") or receipt.get("vocabulary_token") is not None:
        raise ValueError("ordinary independent Grab cue exposure is missing")
    if receipt.get("teacher_cue_tokens") != [9, 1]:
        raise ValueError("Grab teacher cue changed")
    return setup


def validate_acquisitions(receipt, setup, captured_rows):
    events = receipt.get("grab_food_acquisitions", [])
    rows = []
    for event in events:
        row = event.get("row")
        command = event.get("selected_command") or {}
        physical = event.get("physical") or {}
        target = command.get("target") or {}
        if (not isinstance(row, int) or not 0 <= row < captured_rows or
                event.get("organism") != setup["organism"] or event.get("target") != setup["target"] or
                event.get("owner_before") is not None or event.get("owner_after") != setup["organism"] or
                event.get("consumed_after") or command.get("channel") != "Manipulation" or
                command.get("primitive") != 211 or target.get("entity") != setup["target"] or
                physical.get("contact") != "Touch" or physical.get("target_entity") != setup["target"]):
            raise ValueError("Grab acquisition lacks exact sealed action/Touch/ownership evidence")
        rows.append(row)
    if rows != sorted(set(rows)):
        raise ValueError("Grab acquisition rows repeat or reorder")
    return rows


def validate_probe(directory):
    receipt = read_json(directory / "pilot.json")
    setup = assert_setup(receipt)
    assert_prior(receipt, receipt["ticks"])
    rows = validate_acquisitions(receipt, setup, receipt["ticks"])
    if receipt.get("teacher_mode"):
        raise ValueError("Grab assessment must be learner-selected")
    if bool(rows) != bool(receipt.get("lesson_completed")):
        raise ValueError("Grab assessment completion disagrees with acquisition evidence")
    trace_path = directory / "lesson-trace.json"
    if not trace_path.is_file():
        trace_path = directory / "lesson-diagnostic.json"
    trace = read_json(trace_path)
    offered = any(candidate.get("action") == 211 and candidate.get("target") == setup["target"]
        and any(mask & (1 << candidate["index"]) for mask in
            [step.get("representative_mask", 0)] + step.get("motor_masks", []))
        for step in trace.get("steps", []) for candidate in step.get("candidates", []))
    if not offered:
        raise ValueError("assessment never offered selectable Grab for its reachable unheld food")
    return {"lesson": "grab_food", "grab_acquisitions": len(rows), "first_acquisition_row": rows[0] if rows else None,
            "decisions": receipt["ticks"], "directory": str(directory)}


def validate_cycle(directory, previous, seed):
    receipt = read_json(directory / "cycle.json")
    setup = assert_setup(receipt)
    assert_prior(receipt, receipt["captured_replay_rows"])
    if (receipt.get("seed") != seed or receipt.get("terminal_death_tick") is not None or
            not receipt.get("next_cohort_tick_captured") or
            not receipt.get("next_cohort_optimizer_rebound") or receipt.get("speech_target_rows")):
        raise ValueError("cycle did not seal the healthy nonlanguage optimizer handoff")
    if receipt.get("prepared_grab_food_opportunities") != 1:
        raise ValueError("cycle must identify exactly one prepared opportunity")
    rows = receipt.get("trained_replay_rows", 0)
    bootstrap = receipt.get("bootstrap_replay_rows", 0)
    if (rows < 1 or rows != receipt.get("training_ticks") or
            receipt.get("captured_replay_rows") != rows + bootstrap or bootstrap != 1):
        raise ValueError("captured, loss and bootstrap row counts disagree")
    event_rows = validate_acquisitions(receipt, setup, rows + bootstrap)
    if receipt.get("trained_grab_food_acquisitions") != sum(row < rows for row in event_rows):
        raise ValueError("Grab trained acquisition count includes bootstrap or loses events")
    rewards = receipt.get("curriculum_reward_rows", [])
    if receipt.get("grab_food_curriculum_version") != 1 or len(rewards) != rows:
        raise ValueError("Grab curriculum components are missing")
    for index, reward in enumerate(rewards):
        credit = .25 if event_rows and event_rows[0] == index else 0.0
        if (reward.get("row") != index or reward.get("curriculum_reward") != credit or
                not all(math.isfinite(reward.get(key, float("nan"))) for key in
                    ("physiological_reward", "curriculum_reward", "combined_reward")) or
                not math.isclose(reward["combined_reward"],
                    reward["physiological_reward"] + credit, abs_tol=1e-6)):
            raise ValueError("Grab curriculum reward includes repeated/setup/bootstrap credit")
    if not math.isclose(receipt.get("curriculum_reward_total", float("nan")),
            sum(reward["curriculum_reward"] for reward in rewards), abs_tol=1e-6):
        raise ValueError("Grab curriculum total disagrees with its loss rows")
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


def sampling_temperature(text):
    value = float(text)
    minimum = struct.unpack("<f", struct.pack("<f", .0001))[0]
    if not math.isfinite(value) or not minimum <= value <= 128.0:
        raise argparse.ArgumentTypeError("sampling temperature must be finite in 0.0001..128")
    return struct.unpack("<f", struct.pack("<f", value))[0]


def validate_rehearsal(receipt, requested):
    rehearsal = receipt.get("acquisition_rehearsal")
    if not requested:
        if rehearsal is not None:
            raise ValueError("unexpected acquisition rehearsal in a PPO-only cycle")
        return 0
    credited = [row["row"] for row in receipt["curriculum_reward_rows"]
                if row["curriculum_reward"] > 0]
    expected = requested if credited else 0
    if (not isinstance(rehearsal, dict) or
            rehearsal.get("requested_epochs") != requested or
            rehearsal.get("completed_epochs") != expected or
            rehearsal.get("acquisition_row") != (credited[0] if credited else None) or
            rehearsal.get("loss_rows_per_epoch") != int(bool(credited)) or
            rehearsal.get("imitation_temperature") != 1.0 or
            rehearsal.get("value_checkpoint_unchanged") is not True):
        raise ValueError("rehearsal dose or credited acquisition disagrees with the sealed cycle")
    losses = rehearsal.get("action_losses", [])
    if (len(losses) != expected or any(not math.isfinite(loss) for loss in losses) or
            rehearsal.get("actor_optimizer_step_after") != receipt["actor_optimizer_step"] or
            rehearsal["actor_optimizer_step_after"] - rehearsal["actor_optimizer_step_before"] != expected):
        raise ValueError("rehearsal losses or actor optimizer delta disagree with completed epochs")
    return expected


def cooled_temperature(current, floor, grab, eating, baseline_meals):
    # Cool exploration only after ordinary-temperature behavior and retention pass.
    if grab["grab_acquisitions"] and eating["held_food_meals"] >= max(1, baseline_meals):
        return sampling_temperature(str(max(floor, current / 2)))
    return current


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cutoff", type=cutoff_time, required=True)
    parser.add_argument("--seed", type=int, default=202610061277)
    parser.add_argument("--assessment-seed", type=int, default=202610061201)
    parser.add_argument("--target-episodes", type=int, default=3000)
    parser.add_argument("--batch-episodes", type=int, default=16)
    parser.add_argument("--decisions", type=int, default=16)
    parser.add_argument("--startup-decisions", type=int, default=128)
    parser.add_argument("--sampling-temperature", type=sampling_temperature, default=1.0)
    parser.add_argument("--sampling-temperature-floor", type=sampling_temperature,
                        help="halve exploration after passing frozen Grab/Eat checks, down to this floor")
    parser.add_argument("--acquisition-rehearsal-epochs", type=int, default=0,
                        help="0 disables; 1..512 offline passes on each first real acquisition")
    parser.add_argument("--max-stalled-batches", type=int, default=0,
                        help="stop after this many full batches without frozen grabbing; 0 disables")
    parser.add_argument("--startup-seconds", type=int, default=2700)
    parser.add_argument("--command-seconds", type=int, default=1200)
    parser.add_argument("--assessment-seconds", type=int, default=600)
    parser.add_argument("--final-reserve-minutes", type=int, default=15)
    parser.add_argument("--min-disk-gib", type=float, default=8)
    parser.add_argument("--min-memory-gib", type=float, default=2)
    parser.add_argument("--idle-seconds", type=int, default=30)
    parser.add_argument("--startup-grace-seconds", type=int, default=30)
    parser.add_argument("--qualification", type=Path, help="completed focused CPU qualification pinning source and new binary")
    parser.add_argument("--run", action="store_true", help="explicitly launch; default only writes a plan")
    return parser


def main(argv=None):
    parser = arguments(); args = parser.parse_args(argv)
    try:
        if not 1 <= args.target_episodes <= 100_000 or not 1 <= args.batch_episodes <= 64:
            raise ValueError("target must be 1..100000 episodes; batches 1..64")
        if not 1 <= args.decisions <= 128 or not 1 <= args.startup_decisions <= 128:
            raise ValueError("decision windows must be 1..128 to retain every bounded prior receipt")
        if not 0 <= args.acquisition_rehearsal_epochs <= 512 or not 0 <= args.max_stalled_batches <= 64:
            raise ValueError("rehearsal must be 0..512; stalled-batch limit 0..64")
        if args.sampling_temperature_floor is None:
            args.sampling_temperature_floor = args.sampling_temperature
        if args.sampling_temperature_floor > args.sampling_temperature:
            raise ValueError("temperature floor cannot exceed initial exploration temperature")
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
                    source_cycle.get("lesson") in ("eat_held_food", "grab_food") and args.seed <= source_seed):
                raise ValueError("training seed range repeats the source episode; select a fresh range")
        input_hashes[str(args.binary)] = sha256(args.binary)
        if args.run:
            if args.qualification is None:
                raise ValueError("run requires completed focused CPU qualification for this new Grab/Eat binary")
            qualification = read_json(args.qualification)
            repo = Path(__file__).resolve().parent.parent
            actual_revision = subprocess.check_output(
                ["git", "-c", f"safe.directory={repo.as_posix()}", "rev-parse", "HEAD"],
                cwd=repo, text=True).strip()
            if (not qualification.get("ready") or qualification.get("source_revision") != actual_revision or
                    qualification.get("binary_sha256") != input_hashes[str(args.binary)] or
                    not qualification.get("focused_native_tests_passed") or
                    not qualification.get("focused_host_tests_passed")):
                raise ValueError("new Grab/Eat source and binary are not CPU-qualified")
        args.output = args.output.resolve()
        # Do not acquire the owner lock or start any executable in plan mode.
        args.output.mkdir(parents=True, exist_ok=False)
        state = {"schema": 1, "status": "planned", "lesson": "grab_food", "promoted": False,
                 "target_distinct_episodes": args.target_episodes, "batch_episodes": args.batch_episodes,
                 "decisions_per_episode": args.decisions, "startup_decisions": args.startup_decisions,
                 "sampling_temperature": args.sampling_temperature, "frozen_assessment_temperature": 1.0,
                 "sampling_temperature_floor": args.sampling_temperature_floor,
                 "acquisition_rehearsal_epochs": args.acquisition_rehearsal_epochs,
                 "offline_rehearsal_epochs_completed": 0,
                 "max_stalled_batches": args.max_stalled_batches,
                 "startup_cap_seconds": args.startup_seconds,
                 "cutoff_utc": args.cutoff.isoformat(), "input_sha256": input_hashes,
                 "source": str(args.source), "latest_checkpoint": str(args.source),
                 "founder_seed_base": founder, "assessment_seed": args.assessment_seed,
                 "training_seed_range": [args.seed, args.seed + args.target_episodes - 1],
                 "distinct_completed_episodes": 0, "captured_decision_rows": 0, "trained_decision_rows": 0,
                 "bootstrap_rows": 0, "captured_held_meals": 0, "trained_row_held_meals": 0,
                 "captured_grab_acquisitions": 0, "trained_grab_acquisitions": 0, "curriculum_reward_total": 0.0,
                 "completed_epoch_repeats": 0, "actor_optimizer_updates": 0, "value_optimizer_updates": 0,
                 "individual_lifetime_continues_between_episodes": False,
                 "trained_parameters_and_optimizer_continue": True, "episodes": [], "assessments": [],
                 "partial_episode": None, "error": None,
                 "resource_limits": {"min_disk_gib": args.min_disk_gib,
                                     "min_memory_gib": args.min_memory_gib, "idle_seconds": args.idle_seconds}}
        state["resource_limits"]["startup_grace_seconds"] = args.startup_grace_seconds
        state["first_episode_argv"] = [str(args.binary), "--resume-cycle", str(args.source),
            str(args.output / "episode-00000"), str(args.startup_decisions), "--seed", str(args.seed),
            "--lesson", "grab_food", "--sampling-temperature", str(args.sampling_temperature)]
        if args.acquisition_rehearsal_epochs:
            state["first_episode_argv"].extend(["--acquisition-rehearsal-epochs",
                                               str(args.acquisition_rehearsal_epochs)])
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
    temperature = args.sampling_temperature
    recipe_enabled = bool(args.acquisition_rehearsal_epochs or args.max_stalled_batches or
                          args.sampling_temperature_floor < temperature)
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
                "grab_food", "16", str(args.assessment_seed), str(state["founder_seed_base"])],
               name, args.assessment_seconds, cutoff)
        result = validate_probe(directory); state["assessments"].append(result); publish()
        assessment_costs.append(time.monotonic() - started)
        return result

    def retention_probe(name, source, cutoff):
        directory = args.output / name
        invoke([str(args.binary), "--evaluate", str(directory), str(source / "trained.alife-foundation"),
                "eat_held_food", "16", str(args.assessment_seed), str(state["founder_seed_base"])],
               name, args.assessment_seconds, cutoff)
        result = validate_retention_probe(directory)
        result["lesson"] = "eat_held_food"
        state.setdefault("retention_assessments", []).append(result); publish()
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
        git = subprocess.run(["git", "-c", f"safe.directory={repo.as_posix()}", "status", "--porcelain"], capture_output=True, text=True, check=True)
        if git.stdout.strip():
            raise ValueError("run requires clean reviewed repository source")
        state["git_head"] = subprocess.check_output(["git", "-c", f"safe.directory={repo.as_posix()}", "rev-parse", "HEAD"], text=True).strip()
        before = probe("frozen-before", previous, startup_cutoff)
        observed_frozen_grab = bool(before["grab_acquisitions"])
        eating_before = retention_probe("eating-retention-before", previous, startup_cutoff)
        baseline_meals = eating_before["held_food_meals"]
        regressions = 0
        stalled_batches = 0
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
            cycle_argv = [str(args.binary), "--resume-cycle", str(previous), str(directory),
                    str(args.startup_decisions if index == 0 else args.decisions),
                    "--seed", str(seed), "--lesson", "grab_food"]
            cycle_argv.extend(["--sampling-temperature", str(temperature)])
            if args.acquisition_rehearsal_epochs:
                cycle_argv.extend(["--acquisition-rehearsal-epochs", str(args.acquisition_rehearsal_epochs)])
            invoke(cycle_argv,
                   f"episode-{index:05d}", args.command_seconds,
                   startup_cutoff if index == 0 else collection_cutoff)
            receipt, setup, hashes = validate_cycle(directory, previous, seed)
            if receipt.get("sampling_temperature", 1.0) != temperature:
                raise ValueError("collected sampling/PPO temperature differs from the requested recipe")
            offline_epochs = validate_rehearsal(receipt, args.acquisition_rehearsal_epochs)
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
            state["offline_rehearsal_epochs_completed"] += offline_epochs
            for total, field in (("captured_decision_rows", "captured_replay_rows"),
                                 ("trained_decision_rows", "trained_replay_rows"), ("bootstrap_rows", "bootstrap_replay_rows"),
                                 ("captured_held_meals", "held_food_consumption_events"),
                                 ("trained_row_held_meals", "trained_held_food_consumption_events"),
                                 ("captured_grab_acquisitions", "grab_food_acquisitions"),
                                 ("trained_grab_acquisitions", "trained_grab_food_acquisitions"),
                                 ("curriculum_reward_total", "curriculum_reward_total"),
                                 ("completed_epoch_repeats", "completed_epochs"),
                                 ("actor_optimizer_updates", "actor_optimizer_updates"),
                                 ("value_optimizer_updates", "value_optimizer_updates")):
                state[total] += len(receipt[field]) if field == "grab_food_acquisitions" else receipt[field]
            state["episodes"].append({"index": index, "seed": seed, "setup": setup, "seconds": seconds,
                                      "bytes": size, "input_sha256": hashes, "policy_version": receipt["policy_version"],
                                      "sampling_temperature": temperature, "offline_rehearsal_epochs": offline_epochs})
            publish()
            if index == 0 or (index + 1) % args.batch_episodes == 0:
                current = probe(f"frozen-after-{index:05d}", previous,
                                startup_cutoff if index == 0 else collection_cutoff)
                if recipe_enabled:
                    eating = retention_probe(f"eating-retention-after-{index:05d}", previous,
                                             startup_cutoff if index == 0 else collection_cutoff)
                    if eating["held_food_meals"] < baseline_meals:
                        raise ValueError("frozen batch lost source eating retention")
                    temperature = cooled_temperature(temperature, args.sampling_temperature_floor,
                                                     current, eating, baseline_meals)
                    state["next_sampling_temperature"] = temperature
                    if (index + 1) % args.batch_episodes == 0:
                        stalled_batches = 0 if current["grab_acquisitions"] else stalled_batches + 1
                        state["stalled_batches"] = stalled_batches
                        if args.max_stalled_batches and stalled_batches >= args.max_stalled_batches:
                            raise ValueError("recipe stalled: full batches still fail frozen grabbing")
                    publish()
                if index == 0:
                    state["startup_qualified"] = True
                print(f"Grab-food episodes {index + 1}/{args.target_episodes}; trained rows {state['trained_decision_rows']}; "
                      f"last sealed checkpoint {previous}; estimate {state.get('throughput_estimate', 'startup measurement')}", flush=True)
                regressions = regressions + 1 if observed_frozen_grab and not current["grab_acquisitions"] else 0
                observed_frozen_grab = observed_frozen_grab or bool(current["grab_acquisitions"])
                if regressions >= 2:
                    raise ValueError("two frozen batch checks lost previously observed Grab acquisition")
        # A deadline interruption retains the last sealed source; never resume
        # partial collection. The reserved final probe is a separate assessment.
        probe("frozen-final", previous, args.cutoff)
        retention_probe("eating-retention-final", previous, args.cutoff)
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

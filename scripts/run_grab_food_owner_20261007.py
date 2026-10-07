#!/usr/bin/env python3
"""October 7 sole-owner launcher. Plan/check never start a provider or learner."""
import argparse
import datetime as dt
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import urllib.request

REPO = Path(__file__).resolve().parent.parent
WORKSPACE = REPO.parent
HERE = WORKSPACE / "night-20261007"
UTC = dt.timezone.utc
OWNER = "01a10a77-d76e-712d-9392-f1ef74c0a034"
NATIVE = REPO / "scripts" / "run_grab_food_night.py"
spec = importlib.util.spec_from_file_location("grab_food_native_night", NATIVE)
night = importlib.util.module_from_spec(spec)
spec.loader.exec_module(night)
REQUIRED_SOURCE_FILES = (
    "scripts/run_grab_food_owner_20261007.py", "scripts/run_grab_food_night.py",
    "scripts/start_llamacpp_slm_prior.ps1",
    "crates/alife_world/src/headless.rs", "crates/alife_game_app/src/foundation_grab_food.rs",
    "crates/alife_game_app/src/foundation_training.rs",
    "crates/alife_game_app/src/foundation_training_cycle.rs",
    "crates/alife_game_app/src/foundation_training_warmup.rs",
    "crates/alife_game_app/src/bin/train_n2048_care.rs", "crates/alife_game_app/src/lib.rs",
)

def utc():
    return dt.datetime.now(UTC)

def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))

def hidden():
    return {"creationflags": subprocess.CREATE_NO_WINDOW} if os.name == "nt" else {}

def revision():
    return subprocess.check_output(
        [r"C:\Program Files\Git\cmd\git.exe", "-c", f"safe.directory={REPO.as_posix()}",
         "rev-parse", "HEAD"], cwd=REPO, text=True, **hidden()).strip()

def assert_source(config):
    if night.sha256(config["snapshot_ps1"]) != config["snapshot_ps1_sha256"]:
        raise ValueError("approved read-only host snapshot script changed")
    source = Path(config["source"])
    founder, hashes = night.source_info(source)
    expected = config["source_sha256"]
    actual = {Path(path).name: digest for path, digest in hashes.items()}
    if actual != expected:
        raise ValueError("selected accepted checkpoint hashes changed")
    cycle = read(source / "cycle.json")
    if (config["owner_thread"] != OWNER or founder != config["founder_seed_base"] or
            cycle["biological_objective_version"] != 2 or
            cycle["policy_version"] != config["source_policy_version"] or
            cycle["actor_optimizer_step"] != config["source_actor_age"] or
            cycle["value_optimizer_step"] != config["source_value_age"]):
        raise ValueError("selected trained continuation identity/objective/optimizer age mismatch")
    if config["lesson"] != "grab_food" or config["seed"] != 202610061277 or config["assessment_seed"] != 202610061201:
        raise ValueError("October7 lesson or frozen seed binding changed")
    if (config["work_cutoff_utc"] != "2026-10-07T14:55:00Z" or
            config["absolute_cutoff_utc"] != "2026-10-07T15:00:00Z" or
            config.get("production_budget_mode") != "until_work_cutoff" or
            config.get("full_resource_after_utc") != "2026-10-07T07:00:00Z" or
            "experiment_budget_seconds" in config or
            not 1 <= config["target_distinct_episodes"] <= 3000):
        raise ValueError("October7 production cutoff/bounded episode target changed")
    return hashes

def qualification(config, config_path):
    path = Path(config["qualification"])
    data = read(path)
    if not data.get("ready"):
        raise ValueError("CPU qualification blocked: " + data.get("blocker", "new Grab/Eat build and checks incomplete"))
    if (data.get("source_revision") != revision() or
            data.get("owner_config_sha256") != night.sha256(config_path) or
            not data.get("focused_native_tests_passed") or
            not data.get("focused_host_tests_passed")):
        raise ValueError("qualification does not bind current source/config and passed focused CPU checks")
    source_pins = data.get("source_files_sha256", {})
    if any(name not in source_pins for name in REQUIRED_SOURCE_FILES):
        raise ValueError("qualification omits a required Grab/Eat source file")
    for name, expected in source_pins.items():
        path = (REPO / name).resolve(strict=True)
        if not path.is_relative_to(REPO) or night.sha256(path) != expected:
            raise ValueError("CPU-qualified source changed: " + name)
    status = subprocess.check_output(
        [r"C:\Program Files\Git\cmd\git.exe", "-c", f"safe.directory={REPO.as_posix()}",
         "status", "--porcelain"], cwd=REPO, text=True, **hidden())
    if status.strip():
        raise ValueError("run requires clean committed qualified source before any provider work")
    pins = {str(REPO / name): digest for name, digest in source_pins.items()}
    for key, digest_key in (("binary", "binary_sha256"), ("prior_preflight", "prior_preflight_binary_sha256")):
        path = Path(config[key]).resolve(strict=True)
        if night.sha256(path) != data.get(digest_key):
            raise ValueError("new qualified executable digest changed: " + key)
        pins[str(path)] = data[digest_key]
    pins[str(Path(config_path).resolve())] = night.sha256(config_path)
    pins[str(Path(config["qualification"]).resolve())] = night.sha256(config["qualification"])
    return pins

def native_argv(config, output, cutoff, run=False):
    argv = ["--binary", config["binary"], "--source", config["source"], "--output", str(output),
            "--seed", str(config["seed"]), "--assessment-seed", str(config["assessment_seed"]),
            "--target-episodes", str(config["target_distinct_episodes"]),
            "--batch-episodes", "16", "--decisions", "16", "--startup-decisions", "16",
            "--startup-seconds", "1200", "--command-seconds", "1200", "--assessment-seconds", "300",
            "--final-reserve-minutes", "5", "--min-memory-gib", "8", "--min-disk-gib", "15",
            "--idle-seconds", "0", "--startup-grace-seconds", "0", "--cutoff", cutoff,
            "--qualification", config["qualification"]]
    argv.extend(["--sampling-temperature", str(config.get("sampling_temperature", 1.0))])
    if run:
        argv.append("--run")
    return argv

def prepare_plan(config, config_path, stamp):
    assert_source(config)
    outputs = REPO / "target" / "founder-training"
    plan_output = outputs / ("grab-food-plan-" + stamp)
    run_output = outputs / ("grab-food-run-" + stamp)
    if plan_output == run_output or plan_output.exists() or run_output.exists():
        raise ValueError("plan and run need distinct unused output paths")
    q = read(config["qualification"])
    return {
        "schema": 1, "owner_thread": OWNER, "prepared_utc": utc().isoformat(),
        "status": "source_prepared_unqualified", "training_started": False,
        "provider_started": False, "native_plan_executed": False, "promoted": False,
        "config": str(Path(config_path).resolve()), "config_sha256": night.sha256(config_path),
        "source": config["source"], "source_sha256": config["source_sha256"],
        "policy_version": config["source_policy_version"], "objective_version": 2,
        "actor_age": config["source_actor_age"], "value_age": config["source_value_age"],
        "qualification_ready": bool(q.get("ready")), "qualification_blocker": q.get("blocker"),
        "plan_output": str(plan_output), "run_output": str(run_output),
        "native_plan_argv": native_argv(config, plan_output, config["work_cutoff_utc"]),
        "native_run_argv": native_argv(config, run_output, config["work_cutoff_utc"], True),
        "production_budget_mode": config["production_budget_mode"],
        "collection_cutoff_utc": (night.cutoff_time(config["work_cutoff_utc"]) - dt.timedelta(minutes=5)).isoformat(),
        "work_cutoff_utc": config["work_cutoff_utc"], "absolute_cutoff_utc": config["absolute_cutoff_utc"],
        "stop_file": str(HERE / "STOP"), "attempt_marker": str(HERE / "RUN-ATTEMPT.json"),
        "owner_locks": config["owner_locks"],
        "resource_policy": config["resource_policy"],
        "full_resource_after_utc": config["full_resource_after_utc"],
        "cpu_checks_prepared_only": config["cpu_checks"],
        "provider": {key: config[key] for key in ("model_path", "model_sha256", "expected_alias",
                    "server", "server_sha256", "provider_port", "gpu_layers", "provider_threads",
                    "context_size", "parallel_slots")},
    }

def snapshot(config):
    result = subprocess.run(["pwsh", "-NoProfile", "-File", config["snapshot_ps1"]],
                            capture_output=True, text=True, timeout=15, check=True, **hidden())
    return json.loads(result.stdout)

def owned_pids(snap):
    owned = {os.getpid()}
    while True:
        expanded = owned | {int(p["ProcessId"]) for p in snap["processes"]
                            if int(p["ParentProcessId"]) in owned}
        if expanded == owned:
            return owned
        owned = expanded

def full_resource_allowed(snap, config):
    return (utc() >= night.cutoff_time(config["full_resource_after_utc"]) or
            not any(p["Name"].lower() == "robloxplayerbeta.exe" for p in snap["processes"]))

def check_resources(snap, config, external=None, admission=False):
    res = snap["resources"]
    limits = config["admission_limits"]
    if (res["free_ram_gib"] < limits["min_ram_gib"] or
            res["free_c_gib"] < limits["min_disk_gib"] or
            res["gpu_memory_mib"] > limits["max_vram_mib"] or
            res["gpu_temperature_c"] >= limits["max_temperature_c"]):
        raise RuntimeError("RAM/disk/VRAM/temperature distress")
    admission_limits = config["full_admission_limits"] if full_resource_allowed(snap, config) else limits
    if admission and (res["cpu_load_percent"] > admission_limits["max_cpu_percent"] or
                      res["gpu_util_percent"] > admission_limits["max_gpu_percent"]):
        raise RuntimeError("PC workload busy at admission")
    owned = owned_pids(snap)
    foreign = [p for p in snap["processes"] if re.match(
        r"^(cargo|rustc|train_n2048_care|alife_.*|balanced_practice_probe|captured_fit_diagnostic|counterfactual_replay|learnability_gradient_probe|llama-server)\.exe$",
        p["Name"], re.I) and int(p["ProcessId"]) not in owned
        and (external is None or int(p["ProcessId"]) != external["pid"])]
    if foreign:
        raise RuntimeError("competing workload PIDs " + ",".join(str(p["ProcessId"]) for p in foreign))
    if external is not None:
        live = snap.get("service") or {}
        if live.get("pid") != external["pid"] or live.get("created") != external.get("created"):
            raise RuntimeError("external prior listener identity changed")

def assert_listener(live, config):
    if (not live or not live.get("pid") or not live.get("created") or
            live.get("name") != "llama-server.exe" or
            live.get("alias") != config["expected_alias"] or
            Path(live.get("model_path", "")).resolve() != Path(config["model_path"]).resolve()):
        raise ValueError("prior listener model path/alias mismatch")

def claim_once(path, value):
    descriptor = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    try:
        os.write(descriptor, json.dumps(value).encode())
        os.fsync(descriptor)
    finally:
        os.close(descriptor)

def startup_recovery_reference(config, requested):
    marker = HERE / "RUN-ATTEMPT.json"
    if requested is None and marker.exists():
        raise RuntimeError("October7 sole attempt already claimed; explicit --recover-startup receipt required")
    candidates = sorted(HERE.glob("*-owner-run.json"))
    if not candidates:
        if requested is not None:
            raise ValueError("startup recovery has no prior owner receipt")
        return None
    latest = candidates[-1].resolve()
    if requested is None:
        raise RuntimeError("prior owner receipt exists; recovery requires explicit --recover-startup")
    if Path(requested).resolve() != latest or latest.parent != HERE.resolve():
        raise ValueError("startup recovery must name the latest local owner receipt")
    prior = read(latest)
    required = ("owner_thread", "owner_pid", "ended_utc", "training_process_started",
                "learner_capture_observed", "prepared_plan", "commands")
    if any(key not in prior for key in required):
        raise ValueError("startup receipt lacks explicit closed-owner/dose evidence")
    state = prior.get("native_state") or {}
    if (prior["owner_thread"] != OWNER or not prior["ended_utc"] or
            prior.get("status") not in ("blocked_or_stopped", "failed", "interrupted") or
            prior["training_process_started"] or prior["learner_capture_observed"] or
            prior.get("last_sealed_checkpoint") or prior.get("cleanup_blockers") or
            prior.get("source_preservation_error") or
            any(state.get(key, 0) for key in ("distinct_completed_episodes",
                "captured_decision_rows", "trained_decision_rows", "actor_optimizer_updates",
                "value_optimizer_updates"))):
        raise ValueError("startup recovery requires a closed zero-training-dose failure")
    prepared = prior["prepared_plan"]
    if prepared.get("source_sha256") != config["source_sha256"]:
        raise ValueError("startup recovery would change the selected trained checkpoint")
    output = Path(prepared["run_output"]).resolve()
    if output.parent != (REPO / "target/founder-training").resolve():
        raise ValueError("prior startup output is outside the registered owner namespace")
    for name in ("cycle.json", "actor-checkpoint.json", "value-checkpoint.json", "trained.alife-foundation"):
        if any(output.glob("episode-*/" + name)):
            raise ValueError("startup output contains training checkpoint artifacts; reconcile the chain")
    pids = {int(prior["owner_pid"])}
    if prior.get("provider_pid"):
        pids.add(int(prior["provider_pid"]))
    if prior.get("owned_listener_pid"):
        pids.add(int(prior["owned_listener_pid"]))
    pids.update(int(command["pid"]) for command in prior["commands"] if command.get("pid"))
    if any(pid <= 0 for pid in pids):
        raise ValueError("startup recovery lacks valid recorded process identities")
    marker_hash = None
    if marker.exists():
        bound = read(marker)
        if (bound.get("owner_thread") != OWNER or Path(bound.get("receipt", "")).resolve() != latest or
                bound.get("source_sha256") != config["source_sha256"] or
                bound.get("pid") != prior["owner_pid"]):
            raise ValueError("attempt marker is not bound to the closed startup receipt")
        marker_hash = night.sha256(marker)
    return {"previous_receipt": str(latest), "previous_receipt_sha256": night.sha256(latest),
            "previous_pids": sorted(pids), "marker_sha256": marker_hash}

def check_recovery_processes(reference, snap):
    if reference is not None:
        active = {int(process["ProcessId"]) for process in snap["processes"]}
        if active.intersection(reference["previous_pids"]):
            raise RuntimeError("prior startup owner/child process remains active; no recovery")

def archive_startup_recovery(reference, stamp, receipt_path):
    # Called only after both owner locks are exclusively acquired.
    if night.sha256(reference["previous_receipt"]) != reference["previous_receipt_sha256"]:
        raise ValueError("closed startup receipt changed before recovery")
    marker = HERE / "RUN-ATTEMPT.json"
    archived = None
    if reference["marker_sha256"] is not None:
        if not marker.is_file() or night.sha256(marker) != reference["marker_sha256"]:
            raise ValueError("startup attempt marker changed before recovery")
        archived = HERE / (stamp + "-startup-attempt-archived.json")
        if archived.exists() or archived.parent.resolve() != HERE.resolve():
            raise ValueError("startup archive path is not fresh and local")
        marker.rename(archived)  # Preserve the exact marker; never unlink it.
    elif marker.exists():
        raise ValueError("an unexpected attempt marker appeared before recovery")
    record = {**reference, "owner_thread": OWNER, "new_owner_pid": os.getpid(),
              "new_receipt": str(receipt_path), "recovered_utc": utc().isoformat(),
              "archived_marker": str(archived) if archived else None,
              "training_state_changed": False}
    path = HERE / (stamp + "-startup-recovery.json")
    claim_once(path, record)
    return {"receipt": str(path), "receipt_sha256": night.sha256(path),
            "archived_marker": record["archived_marker"]}

def execute(config, config_path, plan, recover_startup=None):
    # Fail closed before host admission, ownership claims, provider or GPU work.
    pins = assert_source(config)
    pins.update(qualification(config, config_path))
    if plan["config_sha256"] != night.sha256(config_path) or plan["source_sha256"] != config["source_sha256"]:
        raise ValueError("prepared plan no longer matches its pinned source/config")
    work_cutoff = night.cutoff_time(config["work_cutoff_utc"])
    if utc() >= work_cutoff or utc() - night.cutoff_time(plan["prepared_utc"]) > dt.timedelta(hours=24):
        raise TimeoutError("prepared October7 owner plan expired")
    output = Path(plan["run_output"]).resolve()
    planned_output = Path(plan["plan_output"]).resolve()
    parent = REPO / "target" / "founder-training"
    if (output.parent != parent or planned_output.parent != parent or output == planned_output or output.exists()):
        raise ValueError("actual run output must be fresh and distinct from the native plan output")
    recovery = startup_recovery_reference(config, recover_startup)
    if (HERE / "STOP").exists():
        raise RuntimeError("fresh October7 owner STOP exists")
    stamp = utc().strftime("%Y%m%dT%H%M%S%fZ")
    receipt_path = HERE / (stamp + "-owner-run.json")
    receipt = {"owner_thread": OWNER, "owner_pid": os.getpid(), "status": "admission", "prepared_plan": plan,
               "started_utc": utc().isoformat(), "training_process_started": False,
               "learner_capture_observed": False, "provider_started": False,
               "gpu_native_process_started": False, "promoted": False, "commands": []}
    provider = None
    owned_listener = None
    external = None
    locks = []
    cache = [0., None]
    distress_samples = [0]
    attempt_claimed = [False]
    def publish():
        receipt["updated_utc"] = utc().isoformat()
        night.write_json(receipt_path, receipt)
    def guard():
        if utc() >= work_cutoff:
            raise TimeoutError("owned production work cutoff reached")
        if (HERE / "STOP").exists():
            raise RuntimeError("October7 owner requested STOP")
        quick = night.resource_snapshot(REPO)
        if quick["available_memory_bytes"] < 8 * 2**30 or quick["disk_free_bytes"] < 15 * 2**30:
            raise RuntimeError("RAM/disk reserve reached")
        if time.monotonic() - cache[0] >= 5:
            snap = snapshot(config)
            check_resources(snap, config, external)
            cache[:] = [time.monotonic(), snap]
            receipt["last_resources"] = snap["resources"]
            full = full_resource_allowed(snap, config)
            receipt["resource_mode"] = "full" if full else "limited"
            res = snap["resources"]
            distress_samples[0] = (distress_samples[0] + 1 if
                not full and (res["cpu_load_percent"] >= 95 or res["gpu_util_percent"] >= 98) else 0)
            if distress_samples[0] >= 3:
                raise RuntimeError("sustained CPU/GPU saturation; stopped owned work to preserve headroom")
            active = receipt.get("active_training_output")
            if active and (Path(active) / "phase.txt").is_file():
                phase = (Path(active) / "phase.txt").read_text(encoding="utf-8")
                receipt["native_training_phase"] = phase
                captured = phase in ("first-capture", "replay-writer-ready", "collection-complete",
                                     "update-complete", "next-cohort-admitted") or bool(
                    re.match(r"^collecting world_tick=\d+ waking_records=[1-9]\d*$", phase))
                if captured and not receipt["learner_capture_observed"]:
                    receipt["learner_capture_observed"] = True
                    receipt["first_capture_observed_utc"] = utc().isoformat()
                    print(json.dumps({"learner_capture_observed": active, "phase": phase}), flush=True)
            publish()
    def assert_pins():
        for path, expected in pins.items():
            if night.sha256(path) != expected:
                raise ValueError("pinned input/executable changed: " + path)
    environment = os.environ.copy()
    environment.update({"ALIFE_SLM_PRIOR": "on", "ALIFE_SLM_PRIOR_MODEL": config["expected_alias"],
                        "ALIFE_SLM_PRIOR_MODEL_SHA256": config["model_sha256"],
                        "ALIFE_TRAINING_STAGE": "0", "PYTHONDONTWRITEBYTECODE": "1"})
    original_child = night.run_child
    original_cycle = night.validate_cycle
    original_probe = night.validate_probe
    original_retention = night.validate_retention_probe
    def checked_model(directory, filename):
        prior = read(directory / filename).get("semantic_prior") or {}
        if prior.get("model") != config["expected_alias"] or prior.get("model_sha256") != config["model_sha256"]:
            raise ValueError("runtime prior identity/hash mismatch")
    def checked_cycle(directory, previous, seed):
        result = original_cycle(directory, previous, seed)
        checked_model(directory, "cycle.json")
        if result[0].get("sampling_temperature", 1.0) != config.get("sampling_temperature", 1.0):
            raise ValueError("completed cycle sampling/PPO temperature differs from owner configuration")
        if result[1]["sampling_seed"] != seed:
            raise ValueError("prepared Grab setup seed mismatch")
        receipt["last_sealed_checkpoint"] = str(directory)
        receipt["last_sealed_policy"] = result[0]["policy_version"]
        receipt["last_sealed_actor_age"] = result[0]["actor_optimizer_step"]
        receipt["last_sealed_value_age"] = result[0]["value_optimizer_step"]
        publish()
        print(json.dumps({"validated_cycle": str(directory), "grab_acquisitions":
                         result[0]["trained_grab_food_acquisitions"]}), flush=True)
        return result
    def checked_probe(directory):
        result = original_probe(directory); checked_model(directory, "pilot.json")
        print(json.dumps({"validated_grab_probe": result}), flush=True)
        return result
    def checked_retention(directory):
        result = original_retention(directory); checked_model(directory, "pilot.json")
        print(json.dumps({"validated_eating_retention": result}), flush=True)
        return result
    def guarded_child(argv, log, deadline, native_guard, env, on_started=None):
        assert_pins()
        def both():
            guard(); native_guard()
        entry = {"argv": argv, "dispatched_utc": utc().isoformat(), "log": str(log)}
        receipt["commands"].append(entry); publish()
        def started(process):
            entry.update({"pid": process.pid, "actual_process_started_utc": utc().isoformat()})
            receipt["gpu_native_process_started"] = True
            if "--resume-cycle" in argv:
                receipt["training_process_started"] = True
                receipt["active_training_output"] = argv[3]
            publish()
            print(json.dumps({"actual_process_started": entry}), flush=True)
            if on_started is not None:
                on_started(process)
        try:
            if "--resume-cycle" in argv and not attempt_claimed[0]:
                claim_once(HERE / "RUN-ATTEMPT.json", {
                    "owner_thread": OWNER, "pid": os.getpid(), "receipt": str(receipt_path),
                    "source": config["source"], "source_sha256": config["source_sha256"],
                    "seed": config["seed"], "run_output": str(output), "claimed_utc": utc().isoformat()})
                attempt_claimed[0] = True
                receipt["training_attempt_claimed"] = True
                publish()
            original_child(argv, log, min(deadline, work_cutoff), both, env, started)
            entry["status"] = "passed"
        except Exception as error:
            entry["status"] = "stopped"; entry["error"] = str(error)
            raise
        finally:
            entry["ended_utc"] = utc().isoformat(); publish()
    try:
        initial = snapshot(config)
        live = initial.get("service")
        if live:
            assert_listener(live, config); external = live
        check_resources(initial, config, external, True)
        check_recovery_processes(recovery, initial)
        for path in config["owner_locks"]:
            lock = Path(path)
            value = {"owner_thread": OWNER, "pid": os.getpid(), "receipt": str(receipt_path),
                     "cutoff_utc": config["work_cutoff_utc"]}
            claim_once(lock, value)
            locks.append((lock, value))
        if recovery is not None:
            receipt["startup_recovery"] = archive_startup_recovery(recovery, stamp, receipt_path)
        guard()
        # Preparation is separate from native training/evaluation dose.
        receipt["status"] = "provider_preparation"; publish()
        for key, expected_key in (("model_path", "model_sha256"), ("server", "server_sha256")):
            guard()
            if night.sha256(config[key]) != config[expected_key]:
                raise ValueError("actual provider executable/model hash mismatch: " + key)
        if external is None:
            argv = ["pwsh", "-NoProfile", "-File", str(REPO / "scripts/start_llamacpp_slm_prior.ps1"),
                    "-LlamaServerPath", config["server"], "-ModelPath", config["model_path"],
                    "-ModelAlias", config["expected_alias"], "-Port", str(config["provider_port"]),
                    "-GpuLayers", "999", "-Threads", "4", "-ContextSize", "2048", "-ParallelSlots", "1"]
            guard()
            with (HERE / (stamp + "-provider.stdout.log")).open("xb") as out, (HERE / (stamp + "-provider.stderr.log")).open("xb") as err:
                kwargs = {"creationflags": subprocess.CREATE_NO_WINDOW | subprocess.BELOW_NORMAL_PRIORITY_CLASS |
                          subprocess.CREATE_NEW_PROCESS_GROUP} if os.name == "nt" else {"start_new_session": True}
                provider = subprocess.Popen(argv, stdout=out, stderr=err, env=environment, **kwargs)
            receipt["provider_started"] = True; receipt["provider_pid"] = provider.pid; publish()
            print(json.dumps({"actual_provider_process_started": provider.pid}), flush=True)
            readiness_cutoff = min(work_cutoff, utc() + dt.timedelta(seconds=120))
            while utc() < readiness_cutoff:
                guard()
                if provider.poll() is not None:
                    raise RuntimeError("owned provider exited before readiness")
                live = snapshot(config).get("service")
                if live:
                    break
                time.sleep(.5)
        served = snapshot(config)
        check_resources(served, config, external)
        live = served.get("service")
        assert_listener(live, config)
        if external is None:
            if live["pid"] not in owned_pids(served):
                raise ValueError("new provider listener is not an owned descendant")
            owned_listener = live
            receipt["owned_listener_pid"] = live["pid"]
        with urllib.request.urlopen(f"http://127.0.0.1:{config['provider_port']}/v1/models", timeout=5) as response:
            models = json.load(response)
        if config["expected_alias"] not in [model["id"] for model in models["data"]]:
            raise ValueError("served prior API alias mismatch")
        assert_pins()
        original_child([config["prior_preflight"]], HERE / (stamp + "-prior-preflight"),
                       min(work_cutoff, utc() + dt.timedelta(seconds=300)), guard, environment)
        ready = read(HERE / (stamp + "-prior-preflight.stdout.log"))
        if (not ready.get("ready") or ready.get("model") != config["expected_alias"] or
                not any(row.get("salience", 0) > 0 for row in
                        (ready.get("validated_prior") or {}).get("lexicon_associations", []))):
            raise ValueError("structured nonzero prior readiness failed")
        receipt["provider_ready"] = ready
        # All available production time remains until the fixed morning cutoff.
        # Batches/individual commands keep their own bounded limits.
        receipt["production_work_cutoff_utc"] = work_cutoff.isoformat()
        receipt["production_collection_cutoff_utc"] = (work_cutoff - dt.timedelta(minutes=5)).isoformat()
        receipt["production_available_seconds_at_admission"] = max(0., (work_cutoff - utc()).total_seconds())
        receipt["status"] = "native_production_admitted"; publish()
        night.run_child = guarded_child
        night.validate_cycle = checked_cycle
        night.validate_probe = checked_probe
        night.validate_retention_probe = checked_retention
        os.chdir(REPO); os.environ.update(environment)
        # Native plan and actual output differ because native plan creates its output.
        parent.mkdir(parents=True, exist_ok=True)
        plan_argv = native_argv(config, planned_output, work_cutoff.isoformat())
        if night.main(plan_argv) != 0:
            raise ValueError("qualified native plan failed")
        run_argv = native_argv(config, output, work_cutoff.isoformat(), True)
        result = night.main(run_argv)
        receipt["native_state"] = read(output / "night.json")
        receipt["status"] = receipt["native_state"]["status"]
        return result
    except Exception as error:
        receipt["status"] = "blocked_or_stopped"; receipt["error"] = str(error)
        print(json.dumps({"blocked_or_stopped": str(error), "action": "sole-owner bounded Grab continuation",
                          "target": str(output)}), file=sys.stderr, flush=True)
        return 1
    finally:
        night.run_child = original_child
        night.validate_cycle = original_cycle
        night.validate_probe = original_probe
        night.validate_retention_probe = original_retention
        try:
            if provider is not None:
                night.stop_owned(provider)
                # Match an orphaned owned server PID and creation time.
                if owned_listener is not None:
                    live = snapshot(config).get("service")
                    if (live and live.get("pid") == owned_listener["pid"] and
                            live.get("created") == owned_listener.get("created")):
                        subprocess.run(["taskkill", "/PID", str(live["pid"]), "/T", "/F"],
                                       capture_output=True, check=True, **hidden())
                receipt["owned_provider_cleaned_up"] = True
        finally:
            receipt["ended_utc"] = utc().isoformat()
            try:
                receipt["selected_source_preserved"] = assert_source(config)
            except Exception as error:
                receipt["source_preservation_error"] = str(error)
            for lock, value in reversed(locks):
                try:
                    if read(lock) == value:
                        lock.unlink()
                    else:
                        receipt.setdefault("cleanup_blockers", []).append(
                            "owner lock identity changed: " + str(lock))
                except Exception as error:
                    receipt.setdefault("cleanup_blockers", []).append(str(error))
            publish()
            print(json.dumps({"owner_receipt": str(receipt_path), "status": receipt["status"],
                              "training_process_started": receipt["training_process_started"]}), flush=True)

def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("plan", "check", "run"))
    parser.add_argument("--config", type=Path, default=HERE / "owner-config-production-v4.json")
    parser.add_argument("--plan", type=Path)
    parser.add_argument("--recover-startup", type=Path, help="explicit closed zero-dose owner receipt; preserve/archive any old marker")
    args = parser.parse_args(argv)
    config = read(args.config)
    HERE.mkdir(parents=True, exist_ok=True)
    try:
        if args.mode == "run":
            if args.plan is None:
                raise ValueError("--run mode requires the concrete prepared --plan receipt")
            return execute(config, args.config, read(args.plan), args.recover_startup)
        stamp = utc().strftime("%Y%m%dT%H%M%S%fZ")
        plan = prepare_plan(config, args.config, stamp)
        if args.mode == "check":
            plan["host_snapshot"] = snapshot(config)
            try:
                check_resources(plan["host_snapshot"], config, admission=True)
                plan["host_resource_admission"] = "resource_candidate"
            except Exception as error:
                plan["host_resource_admission"] = "blocked"
                plan["host_blocker"] = str(error)
        path = HERE / (stamp + "-" + args.mode + "-plan.json")
        night.write_json(path, plan)
        print(json.dumps({"plan": str(path), **plan}, indent=2))
        return 0
    except Exception as error:
        print("BLOCKED: " + str(error), file=sys.stderr)
        return 1

if __name__ == "__main__":
    raise SystemExit(main())

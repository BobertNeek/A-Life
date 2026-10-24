#!/usr/bin/env python3
"""Compare two explicit frozen-collector rounds with matched serial learning.

--spec RECIPE.json validates and prints a plan without running executables or
creating files. --run requires a fresh output tree. Interrupted outputs, native
private lives, logs, and reservations are retained; this tool never resumes,
restarts a life, selects an accepted source, or promotes a result.

V1 accepts an operator's existing strict complete history and 1..32 decisions
per training episode: ordinary serial PPO then has one 32-tick replay window
and a batch of one, matching the cooperative learner's fixed 32/8/1 settings.
Both independent experimental arms intentionally use the same recipe seeds.
Their paired outcomes are descriptive, not a statistical claim of synergy.
"""
import argparse
import copy
import hashlib
import json
import math
import os
from pathlib import Path
import re
import struct
import subprocess
import sys

import grab_food_seed_reservations as seeds

MIB = 1024 * 1024
MAX_JSON_BYTES = 256 * MIB
MAX_INPUT_BYTES = 2 * 1024 * MIB
MAX_INPUT_FILES = 10_000
LEDGER = ".grab-food-seed-reservations.json"
SOURCE_FILES = ("cpu-cycle.json", "trained.alife-foundation", "actor-checkpoint.json",
                "value-checkpoint.json")
LESSONS = {"eat_held_food", "eat_reachable_food", "feeding", "obstacle_navigation", "relocated_food"}
STRICT = {"policy": "strict", "historical_freshness": "operator-attested-complete"}
SETTINGS = {"window_ticks": 32, "burn_in": 8, "effective_batch_size": 1}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_json(path, limit=MAX_JSON_BYTES):
    with Path(path).open("rb") as stream:
        raw = stream.read(limit + 1)
    require(len(raw) <= limit, f"JSON exceeds byte bound: {path}")
    def finite_float(text):
        value = float(text)
        require(math.isfinite(value), "nonfinite JSON number")
        return value
    value = json.loads(raw, object_pairs_hook=seeds._unique_object, parse_float=finite_float,
                       parse_constant=lambda value: require(False, f"nonfinite JSON: {value}"))
    require(isinstance(value, dict), f"JSON object required: {path}")
    return value


def file_pin(path):
    path = Path(path)
    require(path.is_file() and not path.is_symlink(), f"regular nonsymlink file required: {path}")
    size, digest = 0, hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(MIB):
            size += len(chunk)
            require(size <= MAX_INPUT_BYTES, f"input exceeds byte bound: {path}")
            digest.update(chunk)
    return {"bytes": size, "sha256": digest.hexdigest()}


def raw_tree(root):
    root = Path(root)
    require(root.is_dir() and not root.is_symlink(), f"real input directory required: {root}")
    result, total = {}, 0
    for directory, directories, files in os.walk(root, followlinks=False):
        for name in directories:
            require(not (Path(directory) / name).is_symlink(), "input tree must not contain directory symlinks")
        for name in sorted(files):
            path = Path(directory) / name
            pin = file_pin(path)
            total += pin["bytes"]
            require(len(result) < MAX_INPUT_FILES and total <= MAX_INPUT_BYTES,
                    "input tree exceeds file/byte bound")
            result[str(path.relative_to(root))] = pin
    return result


def native_pins(source):
    require(not any((source / name).exists() for name in ("cycle.json", "group50.json")),
            "comparison V1 requires one explicit native CPU source")
    return {name: file_pin(source / name)["sha256"] for name in SOURCE_FILES}


def overlaps(left, right):
    return left == right or left in right.parents or right in left.parents


def explicit_path(value, label, *, exists=True):
    require(isinstance(value, str) and Path(value).is_absolute(), f"{label} requires an explicit absolute path")
    path = Path(value)
    require(not path.is_symlink(), f"{label} must not be a symlink")
    return path.resolve(strict=exists)


def history_inputs(path):
    """Verify the existing bounded import graph; leave native admission authoritative."""
    documents, pins, total = {}, {}, 0

    def visit(current, expected=None, depth=0):
        nonlocal total
        require(depth <= 6 and len(documents) < 124,
                "history must leave import depth/count for comparison and collector wrappers")
        current = current.resolve(strict=True)
        require(current not in documents, "duplicate/cyclic seed-history import")
        pin = file_pin(current)
        require(expected is None or pin["sha256"] == expected, "seed-history import raw bytes changed")
        total += pin["bytes"]
        require(total <= 16 * MIB, "comparison V1 history graph exceeds16 MiB")
        document = read_json(current, 16 * MIB)
        require(document.get("schema") == 1, "history imports require schema1")
        documents[current] = document
        pins[str(current)] = pin
        imports = document.get("imports", [])
        require(isinstance(imports, list) and len(imports) <= 128, "invalid bounded history imports")
        for reference in imports:
            require(isinstance(reference, dict) and set(reference) == {"path", "sha256"}
                    and isinstance(reference["path"], str), "invalid exact-byte history import")
            child = Path(reference["path"])
            visit(child if child.is_absolute() else current.parent / child, reference["sha256"], depth + 1)

    visit(path)
    return documents, pins


def exclusions(value):
    """Mirror native disclosed seed keys, retaining training/assessment distinction."""
    ranges, assessments = [], set()
    if isinstance(value, dict):
        for key, child in value.items():
            if key.endswith("seed_ranges") or key.endswith("seed_range"):
                require(isinstance(child, list), "invalid disclosed seed ranges")
                pairs = [child] if child and type(child[0]) is int else child
                ranges.extend(seeds._ranges(pairs))
            elif key == "assessment_seeds":
                assessments.update(seeds._seed(item) for item in seeds._list(child))
            elif key == "assessment_seed":
                assessments.add(seeds._seed(child))
            elif key in {"seed", "source_seed", "world_seed"}:
                seed = seeds._seed(child)
                ranges.append([seed, seed])
            else:
                more, declared = exclusions(child)
                ranges.extend(more)
                assessments.update(declared)
    elif isinstance(value, list):
        for child in value:
            more, declared = exclusions(child)
            ranges.extend(more)
            assessments.update(declared)
    return ranges, assessments


def check_lesson(row, *, training):
    keys = {"collector_run_id", "seed", "lesson", "decisions"} if training else {"seed", "lesson", "decisions"}
    require(isinstance(row, dict) and set(row) == keys, "invalid explicit lesson record")
    seeds._seed(row["seed"])
    require(row["lesson"] in LESSONS, "unsupported care lesson")
    maximum = 32 if training else 128
    require(type(row["decisions"]) is int and 1 <= row["decisions"] <= maximum,
            "training doses must be1..32 for one-window serial parity; assessments1..128")
    if training:
        name = row["collector_run_id"]
        require(isinstance(name, str) and re.fullmatch(r"[A-Za-z0-9_-]{1,64}", name)
                and name.upper() not in {"CON", "PRN", "AUX", "NUL"}
                and not re.fullmatch(r"(?:COM|LPT)[1-9]", name, re.IGNORECASE),
                "collector_run_id must be portable and nonreserved")


def source_seed_evidence(source):
    """Preflight a small ancestry set before parsing any retained receipt JSON."""
    paths = [source / "cpu-cycle.json"]
    lineage = source / "source-lineage"
    if lineage.exists():
        require(lineage.is_dir() and not lineage.is_symlink(), "real source-lineage directory required")
        for path in lineage.iterdir():
            if path.suffix == ".json":
                paths.append(path)
                require(len(paths) <= 124, "comparison V1 source ancestry exceeds124 receipt files")
    total = 0
    for path in paths:
        require(path.is_file() and not path.is_symlink(), "regular nonsymlink source receipt required")
        total += path.stat().st_size
        require(total <= 16 * MIB, "comparison V1 parsed source ancestry exceeds16 MiB")
    return [read_json(path, 16 * MIB) for path in [paths[0], *sorted(paths[1:])]]


def build_plan(spec_path):
    spec_path = spec_path.resolve(strict=True)
    spec = read_json(spec_path, MIB)
    required = {"schema", "previous", "seed_history", "output", "care_binary", "round_binary",
                "temperature", "rounds", "assessments"}
    require(required <= set(spec) <= required | set(SETTINGS), "comparison recipe fields are incomplete/unsupported")
    require(type(spec["schema"]) is int and spec["schema"] == 1, "comparison requires schema1")
    require(type(spec["temperature"]) in (int, float) and math.isfinite(spec["temperature"])
            and 0 < spec["temperature"] <= 3.4028234e38, "invalid sampling temperature")
    require(all(type(spec.get(key, default)) is int and spec.get(key, default) == default
                for key, default in SETTINGS.items()), "matched V1 requires replay32/burn8/batch1")
    paths = {name: explicit_path(spec[name], name, exists=name != "output")
             for name in ("previous", "seed_history", "output", "care_binary", "round_binary")}
    output = paths["output"]
    require(not output.exists(), "fresh comparison output required; interrupted outputs are retained, never resumed")
    require(isinstance(spec["rounds"], list) and len(spec["rounds"]) == 2, "exactly two explicit rounds required")
    seen = set()
    for round_rows in spec["rounds"]:
        require(isinstance(round_rows, list) and 1 <= len(round_rows) <= 16, "rounds require1..16 collectors")
        names = set()
        for row in round_rows:
            check_lesson(row, training=True)
            require(row["seed"] not in seen, "training/heldout seeds must be globally unique in the recipe")
            require(row["collector_run_id"].lower() not in names, "duplicate/case-aliased collector ID")
            seen.add(row["seed"])
            names.add(row["collector_run_id"].lower())
        round_rows.sort(key=lambda row: row["collector_run_id"])
    assessments = spec["assessments"]
    require(isinstance(assessments, list) and 1 <= len(assessments) <= 16, "assessments require1..16 heldout cases")
    for row in assessments:
        check_lesson(row, training=False)
        require(row["seed"] not in seen, "training/heldout seeds must be globally unique in the recipe")
        seen.add(row["seed"])
    documents, imported_pins = history_inputs(paths["seed_history"])
    history = documents[paths["seed_history"]]
    require(file_pin(paths["seed_history"])["bytes"] <= 4 * MIB, "comparison root history exceeds4 MiB")
    require(history.get("kind") == "mixed-care-seed-history" and history.get("complete") is True
            and history.get("seed_admission", STRICT) == STRICT,
            "comparison V1 supports only existing strict complete operator-attested history; unknown history is unsupported")
    source_pins = native_pins(paths["previous"])
    require(history.get("source_sha256") == source_pins, "initial history does not match exact native source")
    for key in ("reserved_seed_ranges", "consumed_seed_ranges"):
        seeds._ranges(history.get(key))
    seeds._list(history.get("assessment_seeds"))
    source_evidence = source_seed_evidence(paths["previous"])
    receipt = source_evidence[0]
    source_seed = seeds._seed(receipt.get("seed"))
    require(seeds._contains(seeds._ranges(history["consumed_seed_ranges"]), source_seed),
            "original complete history must disclose its source seed")
    training_ranges, declared_assessments = [], set()
    for document in [*documents.values(), *source_evidence]:
        ranges, declared = exclusions(document)
        training_ranges.extend(ranges)
        declared_assessments.update(declared)
    training_ranges = seeds._ranges(training_ranges)
    for row in [row for round_rows in spec["rounds"] for row in round_rows]:
        require(not seeds._contains(training_ranges, row["seed"]) and row["seed"] not in declared_assessments,
                "training seed overlaps initial source/history exclusions or assessments")
    for row in assessments:
        require(not seeds._contains(training_ranges, row["seed"]), "heldout assessment overlaps known training/source seeds")
    files = {**imported_pins, str(spec_path): file_pin(spec_path)}
    for path in (paths["care_binary"], paths["round_binary"], Path(__file__).resolve(), Path(seeds.__file__).resolve()):
        files[str(path)] = file_pin(path)
    for path in [paths["previous"], *map(Path, files)]:
        require(not overlaps(output, path), "comparison output must not overlap any preserved input/evidence")
    normalized = {**spec, **{key: str(path) for key, path in paths.items()}, **SETTINGS}
    plan = {"schema": 1, "kind": "cpu-care-matched-comparison-plan", "spec": normalized,
            "recipe_path": str(spec_path), "immutable_files": files,
            "initial_source_tree": raw_tree(paths["previous"]), "initial_source_sha256": source_pins,
            "initial_fixture": receipt.get("fixture") is True,
            "history_claim": "existing operator complete attestation plus each arm's disclosed runs; no new attestation",
            "seed_reuse_scope": "intentional paired replication in independent source-bound arms",
            "comparison_scope": "two bounded rounds; descriptive heldout outcomes; no promotion or synergy claim"}
    plan["plan_sha256"] = hashlib.sha256(json.dumps(plan, sort_keys=True, separators=(",", ":"),
                                                     allow_nan=False).encode()).hexdigest()
    return plan


def write_new(path, value):
    encoded = json.dumps(value, sort_keys=True, indent=2, allow_nan=False).encode() + b"\n"
    require(len(encoded) <= MAX_JSON_BYTES, "generated JSON exceeds bound")
    with path.open("xb") as stream:
        stream.write(encoded)
        stream.flush()
        os.fsync(stream.fileno())


def verify_inputs(plan):
    require(all(file_pin(Path(path)) == pin for path, pin in plan["immutable_files"].items()),
            "pinned recipe/binary/history/helper input changed")
    require(raw_tree(Path(plan["spec"]["previous"])) == plan["initial_source_tree"], "immutable initial source tree changed")


def episode_history(plan, previous, trained, *, ledger=None, snapshot=None):
    original = Path(plan["spec"]["seed_history"])
    document = copy.deepcopy(read_json(original, 4 * MIB))
    document["source_sha256"] = native_pins(previous)
    document["consumed_seed_ranges"] = seeds._ranges(document["consumed_seed_ranges"] + [[seed, seed] for seed in trained])
    document["assessment_seeds"] = sorted(set(document["assessment_seeds"]) |
                                           {row["seed"] for row in plan["spec"]["assessments"]})
    document["imports"] = [{"path": str(original), "sha256": plan["immutable_files"][str(original)]["sha256"]}]
    if ledger is not None and ledger.exists():
        raw = ledger.read_bytes()
        require(len(raw) <= 64 * MIB, "serial lane ledger exceeds native bound")
        with snapshot.open("xb") as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        digest = hashlib.sha256(raw).hexdigest()
        require(file_pin(ledger)["sha256"] == digest, "serial ledger changed before dispatch")
        document["imports"].append({"path": str(ledger), "sha256": digest})
    return document


class Runner:
    def __init__(self, plan):
        self.plan = plan
        self.root = Path(plan["spec"]["output"])
        self.frozen = {}
        self.logs = self.root / "commands"
        self.logs.mkdir()
        self.calls = 0

    def guard(self):
        verify_inputs(self.plan)
        for path, expected in self.frozen.items():
            require(raw_tree(path) == expected, f"retained completed source/private artifact changed: {path}")

    def freeze(self, path):
        self.frozen[path] = raw_tree(path)

    def command(self, binary, args, cwd, *, prior=True):
        self.guard()
        number = f"{self.calls:03d}"
        self.calls += 1
        environment = {key: value for key, value in os.environ.items() if not key.startswith("ALIFE_SLM_")}
        environment.update(ALIFE_SLM_PRIOR="on" if prior else "off", ALIFE_SLM_PRIOR_BACKEND="deterministic")
        argv = [str(binary), *map(str, args)]
        write_new(self.logs / f"{number}.argv.json", {"argv": argv, "cwd": str(cwd),
                  "prior_environment": {key: value for key, value in environment.items() if key.startswith("ALIFE_SLM_")}})
        print(f"comparison command {number}: {Path(binary).name} {args[0]}", file=sys.stderr, flush=True)
        with (self.logs / f"{number}.stdout.json").open("xb") as out, (self.logs / f"{number}.stderr.log").open("xb") as err:
            completed = subprocess.run(argv, cwd=cwd, env=environment, stdin=subprocess.DEVNULL,
                                       stdout=out, stderr=err, timeout=3600, check=False)
        self.guard()
        require(completed.returncode == 0,
                f"native command failed({completed.returncode}); preserve outputs and inspect commands/{number}.stderr.log; no automatic retry")
        return read_json(self.logs / f"{number}.stdout.json", 64 * MIB)


def integer(value, label):
    require(type(value) is int and value >= 0, f"invalid actual {label}")
    return value


def training_step(receipt, *, cooperative, path):
    if cooperative:
        before, after, update = receipt["source_handoff"], receipt["output_handoff"], receipt["update"]
        actor_delta = after["actor"]["optimizer_step"] - before["actor"]["optimizer_step"]
        value_delta = after["value"]["optimizer_step"] - before["value"]["optimizer_step"]
        settings = {"window_ticks": receipt["plan"]["window_ticks"], "burn_in": receipt["plan"]["burn_in"],
                    "effective_batch_size": receipt["plan"]["effective_batch_size"]}
        config = receipt["plan"]["config"]
        windows = receipt["windows"]
    else:
        actor_delta = receipt["actor_optimizer_step"] - receipt["initial_actor_optimizer_step"]
        value_delta = receipt["value_optimizer_step"] - receipt["initial_value_optimizer_step"]
        update = receipt
        settings = {"window_ticks": receipt["replay_window_ticks"], "burn_in": receipt["replay_burn_in"],
                    "effective_batch_size": receipt["effective_batch_size"]}
        config = receipt["ppo_config"]
        windows = receipt["training_windows"]
    require(isinstance(update["stopped_for_kl"], bool), "actual KL stop flag missing")
    return {"path": str(path), "training_rows": integer(receipt["training_rows"], "training rows"),
            "windows": integer(windows, "training windows"), "settings": settings, "ppo_config": config,
            "actor_optimizer_updates": integer(actor_delta, "actor updates"),
            "value_optimizer_updates": integer(value_delta, "value updates"),
            "completed_epochs": integer(update["completed_epochs"], "completed epochs"),
            "stopped_for_kl": update["stopped_for_kl"],
            "mean_kl_before_updates": update["mean_kl_before_updates"]}


def arm_training(runner, arm):
    spec = runner.plan["spec"]
    root = runner.root / arm
    root.mkdir()
    cwd = root / "process"
    cwd.mkdir()
    histories = root / "histories"
    histories.mkdir()
    lane = root / "episodes"
    lane.mkdir()
    previous, trained, rounds = Path(spec["previous"]), [], []
    for index, rows in enumerate(spec["rounds"]):
        steps = []
        if arm == "cooperative":
            history = histories / f"round-{index}.json"
            write_new(history, episode_history(runner.plan, previous, trained))
            output = root / f"round-{index}"
            care_spec = {"schema_version": 1, "previous": str(previous), "seed_history": str(history),
                         "output": str(output), "known_exclusions": False, "temperature": spec["temperature"],
                         "prior": "deterministic_v1", **SETTINGS, "record_diagnostic_replay": False, "collectors": rows}
            recipe, plan = root / f"round-{index}-spec.json", root / f"round-{index}-plan.json"
            write_new(recipe, care_spec)
            runner.command(spec["round_binary"], ["plan", "--spec", recipe, "--output", plan], cwd)
            runner.command(spec["round_binary"], ["run", "--plan", plan], cwd)
            runner.command(spec["round_binary"], ["inspect", "--result", output / "learner"], cwd)
            result = read_json(output / "learner/cooperative-round.json")
            steps.append(training_step(result, cooperative=True, path=output / "learner"))
            provider = read_json(plan, 16 * MIB)["prior_provider_identity"]
            for row in rows:
                prior = read_json(output / "collectors" / row["collector_run_id"] / "care-episode.json")["semantic_prior"]
                require(prior["backend"] == "deterministic" and prior["failures"] == 0
                        and prior["provider_identity"] == provider, "cooperative prior differs from explicit deterministic plan")
            steps[-1]["prior_provider_identity"] = provider
            runner.freeze(output)
            exported = root / f"founder-{index}"
            runner.command(spec["round_binary"], ["export-source", "--source", previous,
                           "--result", output / "learner", "--output", exported], cwd)
            runner.freeze(exported)
            previous = exported
            trained.extend(row["seed"] for row in rows)
        else:
            for row in rows:
                number = len(trained)
                history = histories / f"episode-{number:03d}.json"
                snapshot = histories / f"episode-{number:03d}.ledger.json"
                write_new(history, episode_history(runner.plan, previous, trained, ledger=lane / LEDGER, snapshot=snapshot))
                output = lane / f"episode-{number:03d}"
                runner.command(spec["care_binary"], ["--previous", previous, "--output", output,
                               "--seed-history", history, "--seed", row["seed"], "--lesson", row["lesson"],
                               "--decisions", row["decisions"], "--temperature", spec["temperature"]], cwd)
                receipt = runner.command(spec["care_binary"], ["--validate-output", output], cwd)
                require(receipt["seed"] == row["seed"] and receipt["training_rows"] == row["decisions"],
                        "serial native receipt dose/seed differs from recipe")
                attempts = read_json(lane / LEDGER, 64 * MIB)["cpu_care_attempts"]
                require(attempts[-1]["status"] == "accepted" and attempts[-1]["native_output_sha256"] == native_pins(output),
                        "ordinary serial native reservation did not accept this exact output")
                steps.append(training_step(receipt, cooperative=False, path=output))
                prior = receipt["episode"]["semantic_prior"]
                require(prior["backend"] == "deterministic" and prior["failures"] == 0,
                        "serial episode lacked its explicit deterministic prior")
                steps[-1]["prior_provider_identity"] = prior["provider_identity"]
                runner.freeze(output)
                previous = output
                trained.append(row["seed"])
        rounds.append({"round": index, "requested_training_rows": sum(row["decisions"] for row in rows), "steps": steps})
    initial = Path(spec["previous"])
    ages = lambda source: {head: integer(read_json(source / f"{head}-checkpoint.json", 64 * MIB)["optimizer_step"], f"{head} age")
                           for head in ("actor", "value")}
    before, after = ages(initial), ages(previous)
    updates = {head: integer(after[head] - before[head], f"total {head} updates") for head in before}
    require(all(updates[head] == sum(step[f"{head}_optimizer_updates"] for row in rounds for step in row["steps"])
                for head in before), "final source ages differ from actual per-step update totals")
    return {"final_source": str(previous), "final_source_sha256": native_pins(previous), "rounds": rounds,
            "initial_optimizer_steps": before, "final_optimizer_steps": after, "optimizer_updates": updates,
            "training_rows": sum(step["training_rows"] for row in rounds for step in row["steps"])}


def budget_comparison(arms):
    rows = []
    for cooperative, serial in zip(arms["cooperative"]["rounds"], arms["serial"]["rounds"]):
        groups = [cooperative["steps"], serial["steps"]]
        totals = [{key: sum(step[key] for step in steps) for key in
                   ("training_rows", "windows", "actor_optimizer_updates", "value_optimizer_updates")} for steps in groups]
        configs = [step["ppo_config"] for steps in groups for step in steps]
        providers = [step["prior_provider_identity"] for steps in groups for step in steps]
        matched = (totals[0] == totals[1]
                   and totals[0]["training_rows"] == cooperative["requested_training_rows"] == serial["requested_training_rows"]
                   and totals[0]["actor_optimizer_updates"] == totals[0]["value_optimizer_updates"]
                   and all(f32_config_identity(config) == f32_config_identity(configs[0]) for config in configs)
                   and all(provider == providers[0] for provider in providers)
                   and all(step["settings"] == SETTINGS and not step["stopped_for_kl"] for steps in groups for step in steps)
                   and all(step["windows"] == 1 for step in serial["steps"]))
        rows.append({"round": cooperative["round"], "budget_matched": matched,
                     "cooperative_actual": totals[0], "serial_actual": totals[1]})
    return {"budget_matched": all(row["budget_matched"] for row in rows), "rounds": rows}


def f32_config_identity(value):
    """Compare Rust f32 values exactly despite compact/expanded JSON decimals."""
    if type(value) is float:
        require(math.isfinite(value), "nonfinite actual PPO configuration")
        return ("f32", struct.pack("!f", value))
    if isinstance(value, dict):
        return ("object", tuple((key, f32_config_identity(child)) for key, child in sorted(value.items())))
    if isinstance(value, list):
        return ("array", tuple(f32_config_identity(child) for child in value))
    require(value is None or type(value) in (int, bool, str), "unsupported actual PPO configuration value")
    return (type(value).__name__, value)


def assessments(runner, arms):
    spec = runner.plan["spec"]
    trained = [row["seed"] for rows in spec["rounds"] for row in rows]
    paired = []
    for index, assessment in enumerate(spec["assessments"]):
        pair = dict(assessment)
        for arm, training in arms.items():
            root = runner.root / arm
            history = root / "histories" / f"assessment-{index:02d}.json"
            source = Path(training["final_source"])
            write_new(history, episode_history(runner.plan, source, trained))
            output = root / f"assessment-{index:02d}"
            request = {"previous": str(source), "output": str(output),
                       "seed_history": str(history), "temperature": spec["temperature"],
                       "known_exclusions": False, **assessment}
            recipe = root / f"assessment-{index:02d}-spec.json"
            write_new(recipe, request)
            result = runner.command(spec["round_binary"], ["evaluate", "--spec", recipe], root / "process", prior=False)
            receipt = read_json(output / "care-evaluation.json")
            require(receipt["result"] == result and all(result[key] == assessment[key] for key in assessment),
                    "heldout receipt/CLI result differs from its requested lesson/seed/dose")
            require(result["evaluated_decisions"] == assessment["decisions"] and result["bootstrap_decisions"] == 1
                    and result["teacher_present"] is False and result["teacher_cue_frames"] == 0
                    and result["semantic_prior_enabled"] is False and result["offline_learning_disabled"] is True
                    and result["actor_checkpoint_unchanged"] is True and result["value_checkpoint_unchanged"] is True
                    and result["offline_actor_optimizer_updates"] == result["offline_value_optimizer_updates"] == 0
                    and result["promotion"] == "none", "evaluation did not retain its heldout/frozen-founder contract")
            runner.freeze(output)
            pair[arm] = {"output": str(output), "receipt_sha256": file_pin(output / "care-evaluation.json")["sha256"],
                         "result": result}
        paired.append(pair)
    return paired


def execute(plan):
    verify_inputs(plan)
    root = Path(plan["spec"]["output"])
    require(root.resolve() == root, "comparison output parent changed since planning")
    require(not root.exists(), "fresh output required; interrupted outputs are retained and never restarted")
    root.mkdir(parents=True, exist_ok=False)
    write_new(root / "comparison-plan.json", plan)
    runner = Runner(plan)
    arms = {arm: arm_training(runner, arm) for arm in ("cooperative", "serial")}
    paired = assessments(runner, arms)
    runner.guard()
    report = {"schema": 1, "kind": "cpu-care-matched-comparison", "complete": True,
              "plan_sha256": plan["plan_sha256"], "initial_source": plan["spec"]["previous"],
              "initial_source_sha256": plan["initial_source_sha256"], "initial_fixture": plan["initial_fixture"],
              "training": arms, **budget_comparison(arms), "heldout_pairs": paired,
              "retained_artifact_trees": {str(path): files for path, files in runner.frozen.items()},
              "serial_history_imports": "live lane ledger required at dispatch; raw pre-dispatch snapshots retained in histories; old live imports are not asserted immutable",
              "seed_reuse_scope": plan["seed_reuse_scope"], "history_claim": plan["history_claim"],
              "inference": "paired descriptive outcomes only; fixture or single-case gains do not establish synergy",
              "promotion": "none"}
    write_new(root / "comparison-report.json", report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spec", type=Path, required=True)
    parser.add_argument("--run", action="store_true")
    args = parser.parse_args()
    plan = build_plan(args.spec)
    result = execute(plan) if args.run else plan
    print(json.dumps(result, sort_keys=True, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()

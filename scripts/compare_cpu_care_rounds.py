#!/usr/bin/env python3
"""Compare two explicit frozen-collector rounds with matched serial learning.

--spec RECIPE.json validates and prints a plan without running executables or
creating files. --run requires a fresh output tree. Interrupted outputs, native
private lives, logs, and reservations are retained; this tool never resumes,
restarts a life, selects an accepted source, or promotes a result.

V1 defaults to an operator's existing strict complete history. Explicit
--known-exclusions accepts incomplete history with its existing recovery proof;
historical freshness stays unknown and outcomes are conditional comparisons.
Training episodes have 1..32 decisions: ordinary serial PPO then has one 32-tick
replay window and a batch of one, matching the cooperative learner's 32/8/1.
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
KNOWN = {"policy": "known-exclusions", "historical_freshness": "unknown"}
UNKNOWN_CLAIM_SCOPE = ("historical freshness unknown; disclosed exclusions only; "
                       "no claim of unseen-seed generalization or synergy")
ADMISSION_HELPERS = ("run_mixed_care_serial.py", "grab_food_seed_reservations.py",
                     "mixed_care_ledger.py", "run_eat_held_food_night.py",
                     "summarize_ppo_diagnostics.py")
SETTINGS = {"window_ticks": 32, "burn_in": 8, "effective_batch_size": 1}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_json(path, limit=MAX_JSON_BYTES):
    with Path(path).open("rb") as stream:
        raw = stream.read(limit + 1)
    require(len(raw) <= limit, f"JSON exceeds byte bound: {path}")
    return decode_json(raw, path)


def decode_json(raw, path, *, object_only=True):
    def finite_float(text):
        value = float(text)
        require(math.isfinite(value), "nonfinite JSON number")
        return value
    value = json.loads(raw, object_pairs_hook=seeds._unique_object, parse_float=finite_float,
                       parse_constant=lambda value: require(False, f"nonfinite JSON: {value}"))
    require(not object_only or isinstance(value, dict), f"JSON object required: {path}")
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


def qualified_checkout():
    """Resolve only admission's checkout; private process/cache cwd stays separate."""
    root = Path(__file__).resolve().parent.parent
    environment = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    result = subprocess.run(["git", "-C", str(root), "rev-parse", "--show-toplevel", "--absolute-git-dir"],
                            env=environment, stdin=subprocess.DEVNULL, capture_output=True,
                            timeout=30, check=False, text=True)
    require(result.returncode == 0, "known-exclusions comparison requires its qualified source checkout")
    values = result.stdout.splitlines()
    require(len(values) == 2 and Path(values[0]).samefile(root),
            "known-exclusions admission checkout does not match this runner")
    # Git emits the ordinary Windows spelling even when Rust launched this
    # script through a verbatim \\?\ path. Bind filesystem identity, then use
    # Git's spelling for its child environment rather than comparing aliases.
    return {"root": str(Path(values[0]).resolve(strict=True)),
            "git_dir": str(Path(values[1]).resolve(strict=True))}


def known_history_inputs(history, paths, documents, pins):
    """Reuse recovery validation; never create or retarget a recovery attestation."""
    import run_mixed_care_serial as mixed

    attestation = history.get("recovery_reconciliation")
    require(isinstance(attestation, dict), "known-exclusions comparison requires existing recovery reconciliation")
    references = attestation.get("evidence")
    require(isinstance(references, list) and 1 <= len(references) <= 128,
            "known-exclusions comparison requires existing cited recovery evidence")
    extra_pins, extra_documents = {}, []
    total = sum(pin["bytes"] for pin in pins.values())

    def check_dependencies(value, path):
        if isinstance(value, dict):
            require(value.get("kind") not in {mixed.ledger_store.MANIFEST, mixed.ledger_store.SEGMENT},
                    "ledger-bound manifest/segment recovery evidence is unsupported across fresh comparison lanes")
            if "imports" in value:
                require(isinstance(value["imports"], list), "invalid recovery proof imports")
                for child in value["imports"]:
                    require(isinstance(child, dict) and set(child) == {"path", "sha256"}
                            and isinstance(child["path"], str), "invalid recovery proof import")
                    dependency = Path(child["path"])
                    dependency = (dependency if dependency.is_absolute() else path.parent / dependency).resolve(strict=True)
                    require(dependency in documents and pins[str(dependency)]["sha256"] == child["sha256"],
                            "recovery proof dependencies must already be exact-byte history imports")
            for child in value.values():
                check_dependencies(child, path)
        elif isinstance(value, list):
            for child in value:
                check_dependencies(child, path)

    for reference in references:
        require(isinstance(reference, dict) and "format" not in reference,
                "comparison requires plain immutable recovery evidence; ledger-bound manifests cannot span its fresh lanes")
        path = explicit_path(reference.get("path"), "recovery evidence")
        require(path.stat().st_size <= 16 * MIB, "recovery evidence exceeds16 MiB")
        with path.open("rb") as stream:
            raw = stream.read(16 * MIB + 1)
        require(len(raw) <= 16 * MIB, "recovery evidence exceeds16 MiB")
        pin = {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}
        require(pin["sha256"] == reference.get("sha256"), "recovery evidence raw bytes changed")
        if str(path) not in pins and str(path) not in extra_pins:
            total += pin["bytes"]
            require(len(pins) + len(extra_pins) < 124 and total <= 16 * MIB,
                    "comparison history/recovery graph exceeds file/byte bounds")
        try:
            document = decode_json(raw, path, object_only=False)
        except json.JSONDecodeError as error:
            require(path.suffix.lower() != ".json" and not error.doc.lstrip().startswith(("{", "[")),
                    "malformed structured recovery evidence")
        except UnicodeDecodeError as error:
            prefix = raw.decode(error.encoding, errors="replace").lstrip("\ufeff \t\r\n")
            require(path.suffix.lower() != ".json" and not prefix.startswith(("{", "[")),
                    "invalid structured recovery evidence encoding")
        else:
            check_dependencies(document, path)
            if path not in documents:
                extra_documents.append(document)
        extra_pins[str(path)] = pin
    # All comparison lanes are new CPU-only lanes with no prior mixed runs.
    # A prior PC-lane digest is not silently changed into the empty-lane digest.
    mixed.recovery_attestation(history, paths["output"] / mixed.LEDGER)
    mixed.reconcile_prior_runs(history, [])
    return extra_pins, extra_documents


def comparison_context(plan):
    if plan["known_exclusions"]:
        return {"known_exclusions": True, "seed_admission": dict(KNOWN),
                "claim_scope": UNKNOWN_CLAIM_SCOPE}
    return {"known_exclusions": False, "seed_admission": dict(STRICT)}


def verify_unknown_context(value, plan):
    if plan["known_exclusions"]:
        require(all(value.get(key) == expected for key, expected in comparison_context(plan).items()),
                "native comparison result lost its explicit unknown-freshness limitation")


def build_plan(spec_path, known_exclusions=False):
    require(type(known_exclusions) is bool, "known-exclusions requires explicit boolean opt-in")
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
    require(history.get("kind") == "mixed-care-seed-history", "mixed-care seed history required")
    if known_exclusions:
        require(history.get("complete") is False and history.get("seed_admission") == KNOWN,
                "explicit known-exclusions requires incomplete history and truthful unknown freshness")
    else:
        require(history.get("complete") is True and history.get("seed_admission", STRICT) == STRICT,
                "strict comparison requires complete operator-attested history; unknown history is unsupported without --known-exclusions")
    checkout, recovery_pins, recovery_documents = None, {}, []
    if known_exclusions:
        checkout = qualified_checkout()
        recovery_pins, recovery_documents = known_history_inputs(history, paths, documents, imported_pins)
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
    for document in [*documents.values(), *source_evidence, *recovery_documents]:
        ranges, declared = exclusions(document)
        training_ranges.extend(ranges)
        declared_assessments.update(declared)
    training_ranges = seeds._ranges(training_ranges)
    for row in [row for round_rows in spec["rounds"] for row in round_rows]:
        require(not seeds._contains(training_ranges, row["seed"]) and row["seed"] not in declared_assessments,
                "training seed overlaps initial source/history exclusions or assessments")
    for row in assessments:
        require(not seeds._contains(training_ranges, row["seed"]), "heldout assessment overlaps known training/source seeds")
    files = {**imported_pins, **recovery_pins, str(spec_path): file_pin(spec_path)}
    for path in (paths["care_binary"], paths["round_binary"], Path(__file__).resolve(), Path(seeds.__file__).resolve()):
        files[str(path)] = file_pin(path)
    if checkout is not None:
        for name in ADMISSION_HELPERS:
            path = Path(checkout["root"]) / "scripts" / name
            files[str(path)] = file_pin(path)
    for path in [paths["previous"], *map(Path, files)]:
        require(not overlaps(output, path), "comparison output must not overlap any preserved input/evidence")
    normalized = {**spec, **{key: str(path) for key, path in paths.items()}, **SETTINGS}
    plan = {"schema": 1, "kind": "cpu-care-matched-comparison-plan", "spec": normalized,
            "recipe_path": str(spec_path), "immutable_files": files,
            "initial_source_tree": raw_tree(paths["previous"]), "initial_source_sha256": source_pins,
            "initial_fixture": receipt.get("fixture") is True,
            "known_exclusions": known_exclusions, "seed_admission": dict(KNOWN if known_exclusions else STRICT),
            "admission_checkout": checkout,
            "history_claim": ("existing incomplete history and unchanged recovery attestation plus disclosed runs; "
                              "historical freshness remains unknown" if known_exclusions else
                              "existing operator complete attestation plus each arm's disclosed runs; no new attestation"),
            "seed_reuse_scope": "intentional paired replication in independent source-bound arms",
            "comparison_scope": "two bounded rounds; descriptive heldout outcomes; no promotion or synergy claim"}
    if known_exclusions:
        plan["claim_scope"] = UNKNOWN_CLAIM_SCOPE
        plan["missing_history"] = history["missing_history"]
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
    if plan["known_exclusions"]:
        require(qualified_checkout() == plan["admission_checkout"], "qualified admission checkout changed")


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
        if self.plan["known_exclusions"]:
            environment = {key: value for key, value in environment.items() if not key.startswith("GIT_")}
            environment.update(GIT_DIR=self.plan["admission_checkout"]["git_dir"],
                               GIT_WORK_TREE=self.plan["admission_checkout"]["root"])
        argv = [str(binary), *map(str, args)]
        write_new(self.logs / f"{number}.argv.json", {"argv": argv, "cwd": str(cwd),
                  "prior_environment": {key: value for key, value in environment.items() if key.startswith("ALIFE_SLM_")},
                  **comparison_context(self.plan)})
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
                         "output": str(output), "known_exclusions": runner.plan["known_exclusions"], "temperature": spec["temperature"],
                         "prior": "deterministic_v1", **SETTINGS, "record_diagnostic_replay": False, "collectors": rows}
            recipe, plan = root / f"round-{index}-spec.json", root / f"round-{index}-plan.json"
            write_new(recipe, care_spec)
            verify_unknown_context(runner.command(spec["round_binary"], ["plan", "--spec", recipe, "--output", plan], cwd), runner.plan)
            verify_unknown_context(runner.command(spec["round_binary"], ["run", "--plan", plan], cwd), runner.plan)
            runner.command(spec["round_binary"], ["inspect", "--result", output / "learner"], cwd)
            result = read_json(output / "learner/cooperative-round.json")
            steps.append(training_step(result, cooperative=True, path=output / "learner"))
            steps[-1].update(comparison_context(runner.plan))
            steps[-1]["generic_member_historical_freshness"] = "unasserted"
            provider = read_json(plan, 16 * MIB)["prior_provider_identity"]
            for row in rows:
                prior = read_json(output / "collectors" / row["collector_run_id"] / "care-episode.json")["semantic_prior"]
                require(prior["backend"] == "deterministic" and prior["failures"] == 0
                        and prior["provider_identity"] == provider, "cooperative prior differs from explicit deterministic plan")
                verify_unknown_context(read_json(output / "collectors" / row["collector_run_id"] / "care-episode.json"), runner.plan)
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
                argv = ["--previous", previous, "--output", output,
                               "--seed-history", history, "--seed", row["seed"], "--lesson", row["lesson"],
                               "--decisions", row["decisions"], "--temperature", spec["temperature"]]
                if runner.plan["known_exclusions"]:
                    argv.append("--known-exclusions")
                runner.command(spec["care_binary"], argv, cwd)
                receipt = runner.command(spec["care_binary"], ["--validate-output", output], cwd)
                require(receipt["seed_admission"] == runner.plan["seed_admission"],
                        "serial native receipt changed its seed-admission policy")
                verify_unknown_context(receipt["episode"], runner.plan)
                require(receipt["seed"] == row["seed"] and receipt["training_rows"] == row["decisions"],
                        "serial native receipt dose/seed differs from recipe")
                attempts = read_json(lane / LEDGER, 64 * MIB)["cpu_care_attempts"]
                require(attempts[-1]["status"] == "accepted" and attempts[-1]["native_output_sha256"] == native_pins(output),
                        "ordinary serial native reservation did not accept this exact output")
                steps.append(training_step(receipt, cooperative=False, path=output))
                steps[-1].update(comparison_context(runner.plan))
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
            **comparison_context(runner.plan),
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
                       "known_exclusions": runner.plan["known_exclusions"], **assessment}
            recipe = root / f"assessment-{index:02d}-spec.json"
            write_new(recipe, request)
            result = runner.command(spec["round_binary"], ["evaluate", "--spec", recipe], root / "process", prior=False)
            verify_unknown_context(result, runner.plan)
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
              **comparison_context(plan),
              "retained_artifact_trees": {str(path): files for path, files in runner.frozen.items()},
              "serial_history_imports": "live lane ledger required at dispatch; raw pre-dispatch snapshots retained in histories; old live imports are not asserted immutable",
              "seed_reuse_scope": plan["seed_reuse_scope"], "history_claim": plan["history_claim"],
              "inference": (UNKNOWN_CLAIM_SCOPE if plan["known_exclusions"] else
                            "paired descriptive outcomes only; fixture or single-case gains do not establish synergy"),
              "promotion": "none"}
    write_new(root / "comparison-report.json", report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spec", type=Path, required=True)
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--known-exclusions", action="store_true",
                        help="explicit incomplete-history mode; retain recovery checks and unknown historical freshness")
    args = parser.parse_args()
    plan = build_plan(args.spec, known_exclusions=args.known_exclusions)
    result = execute(plan) if args.run else plan
    print(json.dumps(result, sort_keys=True, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()

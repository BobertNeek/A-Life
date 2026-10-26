#!/usr/bin/env python3
"""Legacy history-based scheduler and read-only migration/reconciliation helpers.

New native serial runs use run_current_care.py's single-current checkpoint policy.
This module retains old input/recovery behavior for explicit legacy admission.

The shared ledger is authoritative. Summary receipts are derived, never added to
on restart. A dispatched attempt is never retried with the same seed or output.
Recent-checkpoint recovery retains the full receipt/seed chain but only needs
recent full payloads. This scheduler does not delete historical files.
"""
import argparse
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import signal
import struct
import subprocess
import sys

import grab_food_seed_reservations as seeds
import mixed_care_ledger as ledger_store
import run_eat_held_food_night as host
import summarize_ppo_diagnostics as inspector_io

LESSONS = ("eat_held_food", "eat_reachable_food", "feeding", "relocated_food")
LEDGER = host.MIXED_LEDGER_NAME
FILES = seeds.SOURCE_FILES
require = inspector_io.require
FRESHNESS = {"strict": "operator-attested-complete", "known-exclusions": "unknown"}


def seed_admission(plan):
    """Missing metadata always means strict; never infer opt-in from history."""
    value = plan.get("seed_admission", {"policy": "strict", "historical_freshness": FRESHNESS["strict"]})
    require(isinstance(value, dict) and set(value) == {"policy", "historical_freshness"} and
            isinstance(value["policy"], str) and value["policy"] in FRESHNESS and
            value["historical_freshness"] == FRESHNESS[value["policy"]], "invalid seed admission metadata")
    return value


def explanation(value):
    require(isinstance(value, str) and value.strip() and len(value) <= 4096,
            "nonempty bounded missing-history/recovery explanation required")


def recovery_attestation(doc, ledger_path):
    """Validate an operator's cited attestation, never a substitute for ledger checks."""
    explanation(doc.get("missing_history"))
    attestation = doc.get("recovery_reconciliation")
    assertions = {"accepted_checkpoint_authoritative", "known_inflight_attempts_reconciled_or_excluded",
                  "no_known_detached_later_accepted_chain"}
    require(isinstance(attestation, dict) and set(attestation) == assertions | {
        "explanation", "evidence", "prior_ledger_runs_sha256"} and
        all(attestation[key] is True for key in assertions), "explicit recovery reconciliation attestation required")
    explanation(attestation["explanation"])
    digest = attestation["prior_ledger_runs_sha256"]
    require(isinstance(digest, str) and len(digest) == 64 and all(c in "0123456789abcdef" for c in digest),
            "recovery attestation requires prior ledger runs digest")
    evidence = seeds._list(attestation["evidence"])
    require(evidence, "recovery attestation requires cited exact-byte evidence")
    seen = set()
    verified_segments = {}
    for item in evidence:
        require(isinstance(item, dict) and
                (set(item) == {"path", "sha256"} or
                 (set(item) == {"path", "sha256", "format"} and item["format"] == ledger_store.MANIFEST)) and
                isinstance(item["path"], str) and Path(item["path"]).is_absolute(), "invalid recovery evidence reference")
        path = Path(item["path"]).resolve(strict=True)
        require(path != ledger_path.resolve(), "recovery evidence must be an immutable snapshot, not the live mixed ledger")
        require(path not in seen and host.sha256(path) == item["sha256"], "recovery evidence changed or duplicated")
        seen.add(path)
        if "format" in item:
            ledger_store.verify_snapshot(path, ledger_path, read_document=load, verified=verified_segments)


def load(path):
    return seeds._read(path)


def integer(value, low=0, high=seeds.MAX_SEED):
    require(type(value) is int and low <= value <= high, "invalid integer count or seed")
    return value


def file_hashes(directory):
    names = list(FILES) + [name for name in ("terrain-continuation.json", "critic-correction.json")
                          if (directory / name).exists()]
    require(not any((directory / name).exists() for name in ("warmup.json", "adaptation.json")),
            "only sealed cycles are supported; no adaptation or reset route")
    return {name: host.sha256(directory / name) for name in names}


def unchanged(directory, expected):
    require(file_hashes(directory) == expected, f"sealed source changed: {directory}")


def inspect(directory, binary, *, on_started=None, guard=None, cutoff=None):
    """Use the existing full native CPU admission; host checks cannot replace it."""
    before = file_hashes(directory)
    receipt, _ = inspector_io.load_json(directory / "cycle.json", seeds.MAX_DOCUMENT_BYTES)
    require(receipt.get("biological_objective_version") == 2 and
            receipt.get("objective_state_reset") is False, "requires objective2 without reset")
    policy = integer(receipt.get("policy_version"), 0, seeds.MAX_SEED - 1)
    if guard is not None:
        guard()
    timeout = 90 if cutoff is None else min(90, (cutoff - dt.datetime.now(host.UTC)).total_seconds())
    require(timeout > 0, "schedule cutoff reached before checkpoint inspection")
    result = inspector_io.run_inspector([str(binary), "--validate-checkpoint", str(directory),
                                        "--expected-policy", str(policy)], timeout=timeout, on_started=on_started)
    result.check_returncode()
    report = inspector_io.decode_json(result.stdout, "checkpoint inspection")
    require(report.get("neural_execution") is False and report.get("weights_modified") is False
            and report.get("expected_policy_version") == policy and report.get("policy_version") == policy
            and report.get("founder_seed_base") == receipt.get("founder_seed_base"),
            "native checkpoint inspection disagrees with source")
    digests = {item["file_name"]: item["sha256"] for item in report.get("file_digests", [])}
    require(digests == {name: before[name] for name in FILES}, "native inspection file binding mismatch")
    for head in ("actor", "value"):
        age = integer(report.get(f"{head}_optimizer_step"))
        checkpoint, _ = inspector_io.load_json(directory / f"{head}-checkpoint.json", seeds.MAX_DOCUMENT_BYTES)
        require(age == checkpoint.get("optimizer_step"), "native inspection age mismatch")
    unchanged(directory, before)
    return receipt, report, before


def schedule_rows(document, *, supported_lessons=LESSONS):
    require(set(document) == {"schema", "steps"}, "schedule requires only schema and steps")
    rows = []
    for step in seeds._list(document["steps"]):
        require(isinstance(step, dict) and set(step) == {"lesson", "episodes", "decisions"},
                "each schedule step requires lesson, episodes and decisions")
        require(step["lesson"] in supported_lessons, "unsupported care lesson")
        count = integer(step["episodes"], 1, seeds.MAX_ENTRIES)
        integer(step["decisions"], 1, 128)
        require(len(rows) + count <= seeds.MAX_ENTRIES, "schedule exceeds episode bound")
        rows.extend({"lesson": step["lesson"], "decisions": step["decisions"]} for _ in range(count))
    require(rows, "empty schedule")
    return rows


def plan_rows(plan):
    """Old omitted conditions retain the exact schedule/default shape."""
    rows = schedule_rows(plan["schedule"])
    condition = plan.get("food_need_condition", "newborn_v1")
    require(condition in ("newborn_v1", "ordinary_healthy_aging_v1"), "unsupported food need condition")
    if "food_need_condition" in plan:
        for row in rows:
            if row["lesson"] in ("feeding", "relocated_food"):
                row["food_need_condition"] = condition
    return rows


def validate_food_need_condition(receipt, row):
    initialization = receipt.get("food_need_initialization")
    if initialization is None:
        require("food_need_condition" not in row, "missing prospective food need initialization")
        return
    require(isinstance(initialization, dict) and
            initialization.get("condition") == row.get("food_need_condition", "newborn_v1"),
            "food need initialization differs from the saved condition")


def grab_import(path, expected):
    """Read-only import of Grab exclusions, deliberately preserving their scope."""
    ledger, digest = load(path)
    require(digest == expected and ledger.get("kind") == "grab-food-seed-ledger",
            "Grab import bytes or kind mismatch")
    ranges = seeds._ranges(ledger.get("training_seed_ranges"))
    assessments = {seeds._seed(seed) for seed in seeds._list(ledger.get("assessment_seeds"))}
    for allocation in seeds._list(ledger.get("allocations")):
        require(isinstance(allocation, dict), "invalid Grab allocation")
        ranges += seeds._ranges([allocation.get("training_seed_range")])
        assessments.add(seeds._seed(allocation.get("assessment_seed")))
        seeds._hashes(allocation.get("source_sha256"))
        for key in ("output", "history_path", "history_sha256", "reserved_utc"):
            require(isinstance(allocation.get(key), str) and allocation[key], "Grab allocation lacks provenance")
    return ranges, assessments


def history(path, source_hashes, source_seed, lane, *, admission=None):
    doc, digest = load(path)
    selected = seed_admission({} if admission is None else {"seed_admission": admission})
    require(doc.get("kind") == "mixed-care-seed-history",
            "explicit mixed-care seed history required")
    if selected["policy"] == "strict":
        require(doc.get("complete") is True, "explicit complete mixed-care seed history required in strict mode")
    else:
        require(doc.get("complete") is False and "seed_admission" in doc,
                "known-exclusions requires explicit incomplete history and unknown historical freshness")
        recovery_attestation(doc, lane / LEDGER)
    require(seed_admission(doc) == selected, "history seed admission policy/freshness mismatch")
    require(seeds._hashes(doc.get("source_sha256")) == {name: source_hashes[name] for name in FILES},
            "history is not bound to this sealed source")
    reserved = seeds._ranges(doc.get("reserved_seed_ranges"))
    consumed = seeds._ranges(doc.get("consumed_seed_ranges"))
    assessments = {seeds._seed(seed) for seed in seeds._list(doc.get("assessment_seeds"))}
    imported = set()
    for item in seeds._list(doc.get("imports")):
        require(isinstance(item, dict) and set(item) == {"path", "sha256"}, "invalid explicit import")
        target = Path(item["path"]).resolve(strict=True)
        require(target not in imported, "duplicate history import")
        imported.add(target)
        ranges, assessment = grab_import(target, item["sha256"])
        reserved += ranges
        assessments |= assessment
    grab = lane / seeds.LEDGER_NAME
    require(not grab.is_symlink(), "Grab seed ledger must be a regular lane-local file, not a symlink")
    require(not grab.exists() or grab.resolve() in imported,
            "existing Grab ledger requires an explicit exact-byte import")
    require(seeds._contains(seeds._ranges(consumed), seeds._seed(source_seed)),
            "consumed history must include the source seed")
    return doc, digest, seeds._ranges(reserved + consumed), assessments


def read_ledger(path):
    require(not path.is_symlink(), "mixed ledger must not be a symlink")
    ledger = ledger_store.read(path, read_document=load)
    require(ledger.get("kind") == "mixed-care-seed-ledger", "invalid mixed ledger kind")
    seeds._ranges(ledger.get("excluded_seed_ranges"))
    for seed in seeds._list(ledger.get("assessment_seeds")):
        seeds._seed(seed)
    seen = set()
    ranges = []
    for run in seeds._list(ledger.get("runs")):
        require(isinstance(run, dict) and isinstance(run.get("output"), str) and
                run["output"] not in seen, "duplicate or invalid ledger run")
        seen.add(run["output"])
        plan = run.get("plan")
        require(isinstance(plan, dict) and run.get("plan_sha256") == object_hash(plan), "ledger plan changed")
        imported = run.get("history_import")
        require(isinstance(imported, dict) and seed_admission(plan) == seed_admission(imported) and
                imported.get("complete") is (seed_admission(plan)["policy"] == "strict"),
                "ledger history seed admission mismatch")
        allocation = seeds._ranges([plan.get("seed_range")])[0]
        require(not any(allocation[0] <= last and first <= allocation[1] for first, last in ranges),
                "ledger allocations overlap")
        ranges.append(allocation)
        attempts = seeds._list(run.get("attempts"))
        require(len(attempts) <= allocation[1] - allocation[0] + 1, "attempts exceed allocation")
        slot = 0
        for index, attempt in enumerate(attempts):
            require(isinstance(attempt, dict) and attempt.get("seed") == allocation[0] + index
                    and type(attempt.get("seed")) is int and attempt.get("slot") == slot,
                    "ledger attempt sequence changed")
            status = attempt.get("status")
            require(status in ("pending", "accepted", "abandoned") and
                    (status != "pending" or index == len(attempts) - 1), "invalid attempt status")
            if status == "accepted":
                require(isinstance(attempt.get("sha256"), dict), "accepted seal lacks hashes")
                slot += 1
    return ledger


def write_ledger(path, ledger):
    return ledger_store.write(path, ledger)


def ledger_snapshot_reference(snapshot, source):
    return ledger_store.snapshot_reference(snapshot, source, read_document=load)


def object_hash(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, allow_nan=False).encode()).hexdigest()


def retained_count(plan):
    policy = plan.get("retention", {"mode": "all"})
    require(policy.get("mode") in ("all", "recent-checkpoints"), "unknown retention policy")
    return None if policy["mode"] == "all" else integer(policy.get("keep"), 2)


def source_receipt(plan):
    if "source_receipt" in plan:
        require(isinstance(plan["source_receipt"], dict), "invalid saved source receipt")
        return plan["source_receipt"]
    path = Path(plan["source"]) / "cycle.json"
    require(host.sha256(path) == plan["source_sha256"]["cycle.json"], "source receipt changed")
    return inspector_io.load_json(path, seeds.MAX_DOCUMENT_BYTES)[0]


def accepted_hashes(attempt):
    hashes = attempt["sha256"]
    required = set(FILES) | {"initial.alife-foundation"}
    require(required <= set(hashes) <= required | {"terrain-continuation.json", "critic-correction.json"}
            and all(isinstance(v, str) and len(v) == 64 and all(c in "0123456789abcdef" for c in v)
                    for v in hashes.values()), "invalid accepted checkpoint hashes")
    return {k: v for k, v in hashes.items() if k != "initial.alife-foundation"}


def reconcile_prior_runs(doc, runs, *, check_payloads=True):
    """Bind the attestation to surviving prior state; grant no acceptance or retries."""
    require(doc["recovery_reconciliation"]["prior_ledger_runs_sha256"] == object_hash(runs),
            "recovery attestation prior ledger runs changed")
    latest = hashes = None
    for run in runs:
        plan = run["plan"]
        require(latest is None or (plan["source"] == latest and plan["source_sha256"] == hashes),
                "prior run source lineage collision")
        if check_payloads:
            unchanged(Path(plan["source"]), plan["source_sha256"])
        else:
            old = source_receipt(plan)
            rows = plan_rows(plan)
        previous = run["plan"]["source"]
        hashes = plan["source_sha256"]
        for index, attempt in enumerate(run["attempts"]):
            directory = Path(run["output"]) / f"attempt-{index:05d}"
            require(attempt.get("directory") == str(directory) and attempt.get("previous") == previous,
                    "prior attempt lineage collision")
            require(attempt["status"] != "pending", "prior pending attempt must be reconciled by its saved run")
            if attempt["status"] == "abandoned":
                require(not (directory / "cycle.json").exists(), "abandoned attempt later acquired a seal; reconcile manually")
            else:
                if check_payloads:
                    hashes = file_hashes(directory)
                    require({**hashes, "initial.alife-foundation": host.sha256(directory / "initial.alife-foundation")} ==
                            attempt["sha256"], "prior accepted receipt/checkpoint changed")
                    actual, _ = inspector_io.load_json(directory / "cycle.json", seeds.MAX_DOCUMENT_BYTES)
                    require(actual == attempt["receipt"], "prior accepted dose changed")
                else:
                    expected = accepted_hashes(attempt)
                    require(attempt["sha256"]["initial.alife-foundation"] == hashes["trained.alife-foundation"],
                            "prior accepted source link changed")
                    receipt = attempt["receipt"]
                    validate_receipt(receipt, old, rows[attempt["slot"]], attempt["seed"])
                    hashes, old = expected, receipt
                previous = str(directory)
        latest = previous


def reserve(plan, ledger_path, *, commit=True):
    """Reserve the entire proposed allocation, before any native trainer child."""
    doc, digest, ranges, assessments = history(Path(plan["history"]), plan["source_sha256"],
        plan["source_seed"], ledger_path.parent, admission=seed_admission(plan))
    require(digest == plan["history_sha256"], "history changed since planning")
    require(not ledger_path.is_symlink(), "mixed ledger must not be a symlink")
    if ledger_path.exists():
        ledger = read_ledger(ledger_path)
    else:
        ledger = ledger_store.State()
    require(all(run["output"] != plan["output"] for run in ledger["runs"]), "output already reserved; use --resume")
    if seed_admission(plan)["policy"] == "known-exclusions":
        reconcile_prior_runs(doc, ledger["runs"], check_payloads=retained_count(plan) is None)
    if ledger["runs"]:
        prior = ledger["runs"][-1]
        require(not any(a["status"] == "pending" for a in prior["attempts"]),
                "previous run has an uncertain attempt; resume it before allocating")
        accepted = [a for a in prior["attempts"] if a["status"] == "accepted"]
        latest = accepted[-1]["directory"] if accepted else prior["plan"]["source"]
        expected = accepted[-1]["sha256"] if accepted else prior["plan"]["source_sha256"]
        expected = {key: value for key, value in expected.items() if key != "initial.alife-foundation"}
        require(plan["source"] == latest and plan["source_sha256"] == expected,
                "new schedule must chain from the latest accepted sealed predecessor")
    ranges += ledger["excluded_seed_ranges"]
    ranges += [run["plan"]["seed_range"] for run in ledger["runs"]]
    assessments |= set(ledger["assessment_seeds"])
    first, last = plan["seed_range"]
    require(not any(first <= end and start <= last for start, end in ranges) and
            not any(first <= seed <= last for seed in assessments), "allocation overlaps excluded seeds")
    ledger["excluded_seed_ranges"] = seeds._ranges(ranges)
    ledger["assessment_seeds"] = sorted(assessments)
    ledger["runs"].append({"output": plan["output"], "plan": plan, "plan_sha256": object_hash(plan),
                           "history_import": doc, "attempts": []})
    if commit:
        write_ledger(ledger_path, ledger)
    return ledger


COUNT_FIELDS = ("requested_training_ticks", "captured_replay_rows", "trained_replay_rows",
                "bootstrap_replay_rows", "completed_epochs", "actor_optimizer_updates", "value_optimizer_updates")


def world_position_f32(value):
    """Compare native Vec3f values across typed serde and widened JSON traces."""
    coordinates = host.coordinates(value)
    require(all(type(number) in (int, float) for number in coordinates), "invalid native f32 pose")
    try:
        result = struct.unpack("<3f", struct.pack("<3f", *coordinates))
    except (OverflowError, struct.error) as error:
        raise ValueError("pose is outside native f32 range") from error
    require(all(math.isfinite(number) for number in result), "pose is outside native f32 range")
    return result


def validate_food_relocation(receipt):
    """Bind passive exact-target evidence to loss rows; a valid miss is still learning."""
    evidence = receipt.get("food_relocation")
    require(isinstance(evidence, dict) and evidence.get("schema") == 1 and evidence.get("assessment") is None,
            "relocated-food evidence required")
    organism = integer(evidence.get("organism"), 1)
    target = integer(evidence.get("target"), 1)
    captured, trained = receipt["captured_replay_rows"], receipt["trained_replay_rows"]
    rows = evidence.get("rows")
    require(evidence.get("captured_rows") == captured and evidence.get("trained_rows") == trained and
            isinstance(rows, list) and len(rows) == captured, "relocation row counts disagree")
    setup_tick = integer(evidence.get("setup_tick"))
    for name in ("initial_position", "relocation_target"):
        host.coordinates(evidence.get(name))
    require(evidence["initial_position"] != evidence["relocation_target"], "relocation must move food")
    event = evidence.get("relocation")
    first = captured
    if event is not None:
        require(isinstance(event, dict), "invalid physical relocation")
        first = integer(event.get("row"), 0, captured)
        tick = integer(event.get("decision_tick"), setup_tick + 2)
        for name in ("from_position", "to_position"):
            host.coordinates(event.get(name))
        require(event["from_position"] != event["to_position"] and
                event["to_position"] == evidence["relocation_target"] and
                event.get("ownership_before") == event.get("ownership_after") ==
                {"owner": None, "consumed": False}, "relocation is not a loose-food world move")
        require((first == 0 or rows[first - 1].get("outcome_tick", tick + 1) <= tick) and
                (first == captured or rows[first].get("decision_tick", tick - 1) >= tick),
                "relocation tick is outside its captured boundary")
        if first < captured and rows[first].get("decision_tick") == tick:
            require(world_position_f32(rows[first].get("food_position_before_decision")) ==
                    world_position_f32(event["to_position"]),
                    "captured post-move position contradicts physical relocation")
        if first > 0 and rows[first - 1].get("outcome_tick") == tick:
            require(world_position_f32(rows[first - 1].get("food_position_after_outcome")) ==
                    world_position_f32(event["from_position"]),
                    "captured pre-move position contradicts physical relocation")
    acquisitions, meals = [], []
    visible = [0, 0]
    previous_tick = setup_tick
    for index, trace in enumerate(rows):
        require(isinstance(trace, dict) and trace.get("row") == index and
                trace.get("organism_id") == organism and trace.get("lesson_target") == target and
                trace.get("behavior") == "OnPolicy" and trace.get("loss_row") is (index < trained) and
                trace.get("food_relocated") is (event is not None and index >= first) and
                type(trace.get("food_visible")) is bool, "relocation row identity or loss binding mismatch")
        tick = integer(trace.get("decision_tick"), previous_tick)
        previous_tick = integer(trace.get("outcome_tick"), tick + 1, tick + 1)
        integer(trace.get("sequence_id"), 1)
        integer(trace.get("dispatch_generation"))
        digest = trace.get("frame_digest")
        require(isinstance(digest, list) and len(digest) == 4, "relocation frame digest missing")
        for word in digest:
            integer(word, 0, 2**64 - 1)
        host.coordinates(trace.get("food_position_before_decision"))
        if trace.get("food_position_after_outcome") is not None:
            host.coordinates(trace["food_position_after_outcome"])
        before = trace.get("lesson_target_ownership_before_decision")
        after = trace.get("lesson_target_ownership_after_outcome")
        for ownership in (before, after):
            require(ownership is None or isinstance(ownership, dict) and
                    type(ownership.get("consumed")) is bool and
                    (ownership.get("owner") is None or type(ownership["owner"]) is int and ownership["owner"] > 0),
                    "invalid food ownership observation")
        action = trace.get("selected_action") or {}
        require(action.get("organism_id") == organism, "foreign relocation action")
        commands = trace.get("selected_commands")
        outcomes = trace.get("channel_outcomes")

        def selected(primitives, contact):
            def physical(value):
                return isinstance(value, dict) and value.get("target_entity") == target and value.get("contact") == contact
            if commands is None:
                return action.get("action_id") in primitives and action.get("target_entity") == target and physical(trace.get("physical"))
            require(isinstance(commands, list) and (outcomes is None or isinstance(outcomes, list)),
                    "invalid manipulation evidence")
            return any(command.get("channel") == "Manipulation" and command.get("primitive") in primitives and
                       (command.get("target") or {}).get("entity") == target and
                       any(outcome.get("channel") == "Manipulation" and physical(outcome.get("physical"))
                           for outcome in outcomes or []) for command in commands)

        for name in ("ordinary_acquisition", "nourished_meal"):
            require(type(trace.get(name)) is bool, "missing exact food outcome flag")
        if trace["ordinary_acquisition"]:
            require(before == {"owner": None, "consumed": False} and
                    after == {"owner": organism, "consumed": False} and selected((210, 211), "Touch"),
                    "acquisition requires the learner's actual food pickup")
            acquisitions.append(index)
        if trace["nourished_meal"]:
            physiology = trace.get("physiology") or {}
            keys = [f"{phase}_{field}" for field in ("energy", "hunger", "health", "injury", "pain")
                    for phase in ("before", "after")]
            require(all(type(physiology.get(key)) in (int, float) and math.isfinite(physiology[key]) for key in keys),
                    "nourishment requires measured physiology")
            require(before == {"owner": organism, "consumed": False} and after is not None and after["consumed"] and
                    selected((210,), "Consumed") and
                    (physiology["after_energy"] > physiology["before_energy"] or
                     physiology["after_hunger"] < physiology["before_hunger"]) and
                    physiology["after_health"] >= physiology["before_health"] and
                    physiology["after_injury"] <= physiology["before_injury"] and
                    physiology["after_pain"] <= physiology["before_pain"], "meal lacks exact-target nourishing consequence")
            meals.append(index)
        if index < trained and trace["food_visible"]:
            visible[int(trace["food_relocated"])] += 1
    require(evidence.get("acquisition_rows") == acquisitions and evidence.get("nourished_meal_rows") == meals,
            "relocation outcome row indexes disagree")
    outcome = evidence.get("food_outcome") or {}
    for key in ("ordinary_acquisition_events", "nourished_meal_events"):
        integer(outcome.get(key), 0, captured)
    for key in ("trained_post_relocation_acquisitions", "trained_post_relocation_nourished_meals",
                "trained_pre_relocation_visible_rows", "trained_post_relocation_visible_rows"):
        integer(evidence.get(key), 0, trained)
    require(outcome.get("organism") == organism and outcome.get("target") == target and
            outcome.get("ordinary_acquisition_events") == len(acquisitions) and
            outcome.get("nourished_meal_events") == len(meals), "relocation food outcome binding mismatch")
    integer(outcome.get("consumption_events"), len(meals), captured)
    integer(outcome.get("explicit_grab_acquisition_events"), 0, len(acquisitions))
    post_acquisitions = sum(first <= row < trained for row in acquisitions) if event is not None else 0
    post_meals = sum(first <= row < trained and any(first <= acquisition < row for acquisition in acquisitions)
                     for row in meals) if event is not None else 0
    require(evidence.get("trained_post_relocation_acquisitions") == post_acquisitions and
            evidence.get("trained_post_relocation_nourished_meals") == post_meals and
            evidence.get("trained_pre_relocation_visible_rows") == visible[0] and
            evidence.get("trained_post_relocation_visible_rows") == visible[1] and
            evidence.get("lesson_completed") is bool(post_meals and all(visible)),
            "relocation completion includes untrained, unseen or unacquired food")


def validate_feeding_food(receipt):
    """Admit compact native evidence; unavailable historical evidence stays unknown.

    The native observer proves target/learner ownership, actual Eat consumption
    and measured physiology. These compact counts do not re-prove those facts.
    """
    evidence = receipt.get("feeding_food")
    if evidence is None:
        return
    require(receipt.get("lesson") == "feeding" and isinstance(evidence, dict) and
            set(evidence) == {"food_outcome", "nourished_meal_rows", "captured_rows", "trained_rows"},
            "invalid Feeding food evidence")
    captured = integer(evidence.get("captured_rows"), 1)
    trained = integer(evidence.get("trained_rows"), 1, captured)
    require(captured == receipt["captured_replay_rows"] and trained == receipt["trained_replay_rows"],
            "Feeding food evidence replay binding mismatch")
    outcome = evidence.get("food_outcome")
    require(isinstance(outcome, dict), "missing Feeding exact food outcome")
    integer(outcome.get("organism"), 1)
    integer(outcome.get("target"), 1)
    acquisitions = integer(outcome.get("ordinary_acquisition_events"), 0, captured)
    integer(outcome.get("explicit_grab_acquisition_events"), 0, acquisitions)
    consumed = integer(outcome.get("consumption_events"), 0, receipt["consumed_events"])
    meals = integer(outcome.get("nourished_meal_events"), 0, consumed)
    rows = evidence.get("nourished_meal_rows")
    require(isinstance(rows, list) and len(rows) == meals, "Feeding meal rows disagree with exact outcome")
    previous = -1
    for meal in rows:
        previous = integer(meal, previous + 1, captured - 1)


def validate_receipt(receipt, old, row, seed):
    validate_food_need_condition(receipt, row)
    require(receipt.get("biological_objective_version") == 2 and
            receipt.get("objective_state_reset") is False, "requires objective2 without reset")
    require(receipt.get("seed") == seed and receipt.get("lesson") == row["lesson"] and
            receipt.get("founder_seed_base") == old.get("founder_seed_base") and
            receipt.get("policy_version") == old["policy_version"] + 1 and
            receipt.get("terminal_death_tick") is None and receipt.get("speech_target_rows") == 0,
            "cycle identity, lesson or healthy handoff mismatch")
    require(receipt.get("old_asset_digest") == old.get("new_asset_digest"), "wrong sealed predecessor")
    for field in COUNT_FIELDS:
        integer(receipt.get(field))
    require(receipt["requested_training_ticks"] == row["decisions"] and
            receipt["trained_replay_rows"] == row["decisions"] == receipt.get("training_ticks") and
            receipt["bootstrap_replay_rows"] == 1 and
            receipt["captured_replay_rows"] == row["decisions"] + 1, "decision/bootstrap counts disagree")
    for head in ("actor", "value"):
        integer(receipt[f"{head}_optimizer_updates"], 1)
    require(receipt["completed_epochs"] > 0, "cycle has no completed optimizer epochs")
    held = integer(receipt.get("prepared_held_food_opportunities"), 0, 1)
    reachable = integer(receipt.get("prepared_reachable_food_opportunities"), 0, 1)
    require((held, reachable) == {"eat_held_food": (1, 0), "eat_reachable_food": (0, 1),
                                "feeding": (0, 0), "relocated_food": (0, 0)}[row["lesson"]], "lesson opportunities disagree")
    if row["lesson"] == "eat_held_food":
        host.assert_setup(receipt)
    elif row["lesson"] == "eat_reachable_food":
        setup = receipt.get("reachable_food_setup") or {}
        require(setup.get("initial_owner") is None and setup.get("initial_consumed") is False and
                setup.get("initial_sleeping") is False and setup.get("sampling_seed") and
                1 <= setup.get("sampling_attempts", 0) <= 32, "invalid reachable setup")
        for key in ("realized_body_position", "food_position"):
            host.coordinates(setup.get(key))
        require(math.isfinite(setup.get("realized_body_yaw", float("nan"))), "invalid reachable yaw")
    captured = integer(receipt.get("held_food_consumption_events"))
    trained = integer(receipt.get("trained_held_food_consumption_events"))
    require(trained <= captured <= held + reachable, "exact nourishing meal counts disagree")
    integer(receipt.get("consumed_events"), 0, receipt["captured_replay_rows"])
    validate_feeding_food(receipt)
    if row["lesson"] == "relocated_food":
        validate_food_relocation(receipt)


def validate_cycle(directory, previous, row, seed, inspector, *, on_started=None, guard=None, cutoff=None):
    options = {"on_started": on_started, "guard": guard, "cutoff": cutoff}
    receipt, report, hashes = inspect(directory, inspector, **options)
    old, ages, previous_hashes = inspect(previous, inspector, **options)
    validate_receipt(receipt, old, row, seed)
    hashes["initial.alife-foundation"] = host.sha256(directory / "initial.alife-foundation")
    require(previous_hashes["trained.alife-foundation"] == hashes["initial.alife-foundation"],
            "wrong sealed predecessor")
    for head in ("actor", "value"):
        updates = integer(receipt[f"{head}_optimizer_updates"], 1)
        require(report[f"{head}_optimizer_step"] - ages[f"{head}_optimizer_step"] == updates,
                "optimizer age lost across lesson boundary")
    return receipt, hashes


def summary(run):
    totals = {lesson: {"episodes": 0, **dict.fromkeys(COUNT_FIELDS, 0),
                      "exact_nourishing_consumption_events": 0, "trained_exact_nourishing_consumption_events": 0,
                      "generic_consumed_events": 0} for lesson in LESSONS}
    feeding = totals["feeding"]
    feeding.update(exact_nourishment_measured_episodes=0, exact_nourishment_unknown_episodes=0,
                   known_exact_nourishing_consumption_events=0,
                   known_trained_exact_nourishing_consumption_events=0)
    latest = run["plan"]["source"]
    for attempt in run["attempts"]:
        if attempt["status"] != "accepted":
            continue
        receipt = attempt["receipt"]
        total = totals[receipt["lesson"]]
        total["episodes"] += 1
        for key in COUNT_FIELDS:
            total[key] += receipt[key]
        if receipt["lesson"] == "relocated_food":
            evidence = receipt["food_relocation"]
            total["exact_nourishing_consumption_events"] += evidence["food_outcome"]["nourished_meal_events"]
            total["trained_exact_nourishing_consumption_events"] += sum(
                row < evidence["trained_rows"] for row in evidence["nourished_meal_rows"])
            for key in ("trained_post_relocation_acquisitions", "trained_post_relocation_nourished_meals"):
                total[key] = total.get(key, 0) + evidence[key]
            total["completed_relocation_episodes"] = total.get("completed_relocation_episodes", 0) + int(evidence["lesson_completed"])
            total["generic_consumed_events"] += receipt["consumed_events"]
        elif receipt["lesson"] == "feeding":
            total["generic_consumed_events"] += receipt["consumed_events"]
            evidence = receipt.get("feeding_food")
            if evidence is None:
                total["exact_nourishment_unknown_episodes"] += 1
            else:
                total["exact_nourishment_measured_episodes"] += 1
                total["known_exact_nourishing_consumption_events"] += evidence["food_outcome"]["nourished_meal_events"]
                total["known_trained_exact_nourishing_consumption_events"] += sum(
                    row < evidence["trained_rows"] for row in evidence["nourished_meal_rows"])
        else:
            total["exact_nourishing_consumption_events"] += receipt["held_food_consumption_events"]
            total["trained_exact_nourishing_consumption_events"] += receipt["trained_held_food_consumption_events"]
        latest = attempt["directory"]
    feeding["exact_nourishing_consumption_events"] = (
        None if feeding["exact_nourishment_unknown_episodes"] else feeding["known_exact_nourishing_consumption_events"])
    feeding["trained_exact_nourishing_consumption_events"] = (
        None if feeding["exact_nourishment_unknown_episodes"] else feeding["known_trained_exact_nourishing_consumption_events"])
    return {"schema": 1, "kind": "mixed-care-summary", "seed_admission": seed_admission(run["plan"]),
            "per_lesson": totals, "latest_checkpoint": latest,
            "accepted_episodes": sum(total["episodes"] for total in totals.values()), "promoted": False,
            "personal_lifetime_continues": False, "trained_parameters_and_optimizer_continue": True}


def admit_run_context(plan):
    require(Path.cwd().resolve() == Path(__file__).resolve().parent.parent and
            Path(plan["output"]).parent == Path.cwd() / "target" / "founder-training",
            "use the shared target/founder-training lane")
    require(not subprocess.check_output(["git", "status", "--porcelain"], text=True).strip(),
            "run requires a clean source checkout for reproducible continuation")
    require(subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip() == plan["git_head"],
            "source commit changed since plan")


def execute(plan, *, resume=False, cutoff=None):
    output = Path(plan["output"])
    lane = output.parent
    ledger_path = lane / LEDGER
    lock = lane / ".eat-held-food-owner.lock"
    cutoff = cutoff or host.cutoff_time(plan["cutoff"])
    descriptor = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    active = []
    try:
        os.write(descriptor, json.dumps({"pid": os.getpid(), "output": str(output)}).encode())
        os.fsync(descriptor)
        admit_run_context(plan)
        host.assert_lane_idle()
        if resume:
            ledger = read_ledger(ledger_path)  # Missing ledger never means an unused allocation.
        else:
            ledger = reserve(plan, ledger_path)
        matches = [run for run in ledger["runs"] if run["output"] == str(output)]
        require(len(matches) == 1 and matches[0]["plan"] == plan, "resume plan collision")
        run = matches[0]
        require(run is ledger["runs"][-1], "a newer allocation owns this lineage; cannot resume an older run")
        rows = plan_rows(plan)
        doc, digest, _, _ = history(Path(plan["history"]), plan["source_sha256"], plan["source_seed"], lane,
                                   admission=seed_admission(plan))
        require(digest == plan["history_sha256"], "history changed on resume")
        if seed_admission(plan)["policy"] == "known-exclusions":
            reconcile_prior_runs(doc, ledger["runs"][:-1], check_payloads=retained_count(plan) is None)

        def guard():
            require(not (output / "STOP").exists(), "owner STOP requested interruption")
            for path, digest in plan["binary_sha256"].items():
                require(host.sha256(path) == digest, "pinned executable changed")
            require(dt.datetime.now(host.UTC) < cutoff, "schedule cutoff reached")
            resources = host.resource_snapshot(output)
            require(resources["disk_free_bytes"] >= plan["min_disk_gib"] * 2**30 and
                    resources["available_memory_bytes"] is not None and
                    resources["available_memory_bytes"] >= plan["min_memory_gib"] * 2**30, "resource reserve reached")
            require(os.name != "nt" or resources["idle_seconds"] >= plan["idle_seconds"], "interactive PC use resumed")

        # Rebuild the chain and validate surviving accepted seals before dispatch.
        inspection_options = {"on_started": active.append, "guard": guard, "cutoff": cutoff}
        previous = Path(plan["source"])
        keep = retained_count(plan)
        accepted = [a for a in run["attempts"] if a["status"] == "accepted"]
        retained = {a["directory"] for a in accepted[-keep:]} if keep is not None else set()
        previous_hashes = plan["source_sha256"]
        previous_report = None
        old = source_receipt(plan)
        if keep is None or not accepted:
            unchanged(previous, previous_hashes)
            guard()
            actual, previous_report, _ = inspect(previous, Path(plan["inspector"]), **inspection_options)
            require(actual == old, "saved source receipt changed")
        slot = 0
        for index, attempt in enumerate(run["attempts"]):
            directory = output / f"attempt-{index:05d}"
            require(attempt.get("directory") == str(directory) and attempt.get("previous") == str(previous)
                    and slot < len(rows), "attempt lineage collision")
            require(attempt.get("food_need_condition", "newborn_v1") == rows[slot].get("food_need_condition", "newborn_v1"),
                    "attempt food need condition differs from saved plan")
            if attempt["status"] == "abandoned":
                require(not (directory / "cycle.json").exists(), "abandoned attempt later acquired a seal; reconcile manually")
                continue
            if attempt["status"] == "accepted" and keep is not None:
                expected = accepted_hashes(attempt)
                receipt = attempt["receipt"]
                validate_receipt(receipt, old, rows[slot], attempt["seed"])
                require(attempt["sha256"]["initial.alife-foundation"] == previous_hashes["trained.alife-foundation"],
                        "accepted predecessor hash changed")
                report = None
                if str(directory) in retained:
                    actual, report, hashes = inspect(directory, Path(plan["inspector"]), **inspection_options)
                    hashes["initial.alife-foundation"] = host.sha256(directory / "initial.alife-foundation")
                    require(actual == receipt and hashes == attempt["sha256"], "accepted dose or checkpoint changed")
                    if previous_report is not None:
                        for head in ("actor", "value"):
                            require(report[f"{head}_optimizer_step"] - previous_report[f"{head}_optimizer_step"] ==
                                    receipt[f"{head}_optimizer_updates"], "optimizer age lost across retained checkpoints")
                previous_hashes, old, previous_report = expected, receipt, report
                previous = directory
                slot += 1
                continue
            if attempt["status"] == "accepted":
                for name, digest in attempt["sha256"].items():
                    require(host.sha256(directory / name) == digest, "accepted receipt/checkpoint changed")
            if (directory / "cycle.json").exists():
                receipt, hashes = validate_cycle(directory, previous, rows[slot], attempt["seed"],
                                                 Path(plan["inspector"]), **inspection_options)
                if attempt["status"] == "accepted":
                    require(receipt == attempt["receipt"] and hashes == attempt["sha256"], "accepted dose changed")
                else:
                    attempt.update(status="accepted", receipt=receipt, sha256=hashes)
                    write_ledger(ledger_path, ledger)
                previous = directory
                slot += 1
            else:
                require(attempt["status"] == "pending", "accepted seal missing")
                attempt["status"] = "abandoned"
                write_ledger(ledger_path, ledger)
        seeds.write_document(output / "schedule.json", summary(run))
        while slot < len(rows):
            index = len(run["attempts"])
            seed = plan["seed_range"][0] + index
            require(seed <= plan["seed_range"][1], "reserved seeds exhausted; reconcile into a new explicit allocation")
            guard()
            host.assert_lane_idle()
            expected = plan["source_sha256"]
            # The most recent attempt can be abandoned; use the actual accepted predecessor.
            if previous != Path(plan["source"]):
                expected = next(a["sha256"] for a in reversed(run["attempts"]) if a["status"] == "accepted")
                expected = {key: value for key, value in expected.items() if key != "initial.alife-foundation"}
            unchanged(previous, expected)
            directory = output / f"attempt-{index:05d}"
            require(not directory.exists(), "attempt output collision")
            row = rows[slot]
            attempt = {"slot": slot, "seed": seed, "directory": str(directory), "previous": str(previous), "status": "pending"}
            if "food_need_condition" in row:
                attempt["food_need_condition"] = row["food_need_condition"]
            run["attempts"].append(attempt)
            write_ledger(ledger_path, ledger)
            argv = [plan["binary"], "--resume-cycle", str(previous), str(directory), str(row["decisions"]),
                    "--seed", str(seed), "--lesson", row["lesson"]]
            if "food_need_condition" in row:
                argv += ["--food-need-condition", row["food_need_condition"]]
            deadline = min(cutoff, dt.datetime.now(host.UTC) + dt.timedelta(seconds=plan["command_seconds"]))
            active.clear()
            host.run_child(argv, output / f"attempt-{index:05d}", deadline, guard,
                           host.prior_environment(), on_started=active.append)
            guard()
            unchanged(previous, expected)
            receipt, hashes = validate_cycle(directory, previous, row, seed, Path(plan["inspector"]),
                                             **inspection_options)
            attempt.update(status="accepted", receipt=receipt, sha256=hashes)
            write_ledger(ledger_path, ledger)
            seeds.write_document(output / "schedule.json", summary(run))
            previous = directory
            slot += 1
        return summary(run)
    finally:
        os.close(descriptor)
        # A cleanup failure may leave a child alive: retain ownership for manual reconciliation.
        if all(child.poll() is not None for child in active):
            lock.unlink()


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--checkpoint-inspector", type=Path)
    parser.add_argument("--source", type=Path)
    parser.add_argument("--schedule", type=Path)
    parser.add_argument("--seed-history", type=Path)
    parser.add_argument("--seed-admission", choices=tuple(FRESHNESS),
                        help="new-plan policy; default strict, known-exclusions requires incomplete attested history")
    parser.add_argument("--food-need-condition", choices=("newborn_v1", "ordinary_healthy_aging_v1"),
                        help="prospective Feeding/RelocatedFood initialization; other lessons keep their setup")
    parser.add_argument("--seed", type=int)
    parser.add_argument("--seed-count", type=int, help="whole reservation, including optional interruption spares")
    parser.add_argument("--cutoff", type=host.cutoff_time)
    parser.add_argument("--command-seconds", type=int, default=1200)
    parser.add_argument("--min-disk-gib", type=float, default=8)
    parser.add_argument("--min-memory-gib", type=float, default=2)
    parser.add_argument("--idle-seconds", type=int, default=30)
    parser.add_argument("--keep-checkpoints", type=int, help="new plans validate this many recent full recovery checkpoints")
    parser.add_argument("--run", action="store_true")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--start", action="store_true", help="reserve and execute a saved host-only plan; requires --run")
    mode.add_argument("--resume", action="store_true", help="reconcile the saved plan/ledger; requires --run")
    return parser


def main(argv=None):
    parser = arguments()
    args = parser.parse_args(argv)
    try:
        output = args.output.resolve()
        if args.resume or args.start:
            require(args.run, "--start/--resume requires --run")
            require(all(getattr(args, name) is None for name in
                        ("binary", "checkpoint_inspector", "source", "schedule", "seed_history", "seed", "seed_count", "seed_admission", "keep_checkpoints", "food_need_condition")),
                    "saved execution uses the immutable plan; do not supply a new configuration")
            plan, _ = load(output / "plan.json")
            require(plan.get("kind") == "mixed-care-plan" and plan.get("output") == str(output), "saved plan/output collision")
            if args.start:
                require({path.name for path in output.iterdir()} == {"plan.json"},
                        "start requires a host-only plan directory; interrupted work requires --resume")
        else:
            require(all(getattr(args, name) is not None for name in
                        ("binary", "checkpoint_inspector", "source", "schedule", "seed_history", "seed", "cutoff")),
                    "new plans require binary, checkpoint-inspector, source, schedule, seed-history, seed and cutoff")
            source = args.source.resolve(strict=True)
            binary = args.binary.resolve(strict=True)
            inspector = args.checkpoint_inspector.resolve(strict=True)
            schedule, _ = load(args.schedule)
            rows = schedule_rows(schedule)
            count = integer(args.seed_count if args.seed_count is not None else len(rows), len(rows), seeds.MAX_ENTRIES)
            first = integer(args.seed, 1)
            require(first + count - 1 <= seeds.MAX_SEED, "seed allocation overflow")
            integer(args.command_seconds, 1, 3600)
            integer(args.idle_seconds, 0, 3600)
            require(all(math.isfinite(value) and value > 0 for value in (args.min_disk_gib, args.min_memory_gib)), "invalid resource limits")
            require(dt.datetime.now(host.UTC) < args.cutoff <= dt.datetime.now(host.UTC) + dt.timedelta(hours=24), "cutoff must be within 24 hours")
            keep = integer(3 if args.keep_checkpoints is None else args.keep_checkpoints, 2)
            hashes = file_hashes(source)
            receipt, _ = inspector_io.load_json(source / "cycle.json", seeds.MAX_DOCUMENT_BYTES)
            history_path = args.seed_history.resolve(strict=True)
            policy = args.seed_admission or "strict"
            admission = {"policy": policy, "historical_freshness": FRESHNESS[policy]}
            # Bind the bytes here; reserve() verifies all history/evidence once
            # and rejects a changed digest before creating any plan or reservation.
            _, digest = load(history_path)
            source_seed = seeds._seed(receipt.get("seed"))
            plan = {"schema": 1, "kind": "mixed-care-plan", "output": str(output), "source": str(source),
                    "source_sha256": hashes, "source_receipt": receipt,
                    "retention": {"mode": "recent-checkpoints", "keep": keep}, "source_seed": source_seed, "binary": str(binary), "inspector": str(inspector),
                    "binary_sha256": {str(path): host.sha256(path) for path in (binary, inspector)},
                    "history": str(history_path), "history_sha256": digest, "seed_admission": admission, "schedule": schedule,
                    **({"food_need_condition": args.food_need_condition} if args.food_need_condition else {}),
                    "seed_range": [first, first + count - 1], "cutoff": args.cutoff.isoformat(),
                    "command_seconds": args.command_seconds, "min_disk_gib": args.min_disk_gib,
                    "min_memory_gib": args.min_memory_gib, "idle_seconds": args.idle_seconds,
                    "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()}
            reserve(plan, output.parent / LEDGER, commit=False)
            # Plan is host-only: no executable, lock, reservation or source mutation.
            output.mkdir(parents=True, exist_ok=False)
            seeds.write_document(output / "plan.json", plan)
        if args.cutoff is not None:
            require(dt.datetime.now(host.UTC) < args.cutoff <= dt.datetime.now(host.UTC) + dt.timedelta(hours=24),
                    "cutoff must be within 24 hours")
        result = execute(plan, resume=args.resume, cutoff=args.cutoff) if args.run else plan
        print(json.dumps(result, indent=2))
        return 0
    except (ValueError, OSError, RuntimeError, subprocess.SubprocessError, KeyboardInterrupt) as error:
        print(f"mixed-care scheduler stopped: {error}; retain ledger and outputs for reconciliation", file=sys.stderr)
        return 1


def terminate(*_):
    raise KeyboardInterrupt("termination requested")


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, terminate)
    raise SystemExit(main())

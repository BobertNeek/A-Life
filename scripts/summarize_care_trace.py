"""Read-only behavior summary of retained evaluation traces; never run a brain.

Usage: python -B scripts/summarize_care_trace.py EVALUATION_DIRECTORY
       --decision-budget 128 --execution-status completed

Accepts lesson-trace.json or lesson-diagnostic.json (or either file directly).
The optional pilot.json supplies provenance, not success evidence. Status and
budget are caller declarations: pilot.ticks is an observed length, not a budget.
Whole-trace counts are null for missing instrumentation, gaps or uncertain runs;
observed_count/events always describe only the available evidence. Reaching the
budget is independent of nourishment and is never itself a success or failure.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import sys

sys.dont_write_bytecode = True
from foundation_probe_schema import candidates, number, object_slots, position, uint

MAX_BYTES = 64 * 1024 * 1024
CHANNELS = ("Locomotion", "Manipulation")
MISSING = object()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def identifier(value):
    return uint(value, 64, positive=True)


def ownership(value):
    if value is MISSING or value is None:
        return None
    require(isinstance(value, dict) and set(value) == {"owner", "consumed"}, "malformed ownership")
    require(value["owner"] is None or identifier(value["owner"]), "malformed owner")
    require(type(value["consumed"]) is bool, "malformed consumed flag")
    require(not value["consumed"] or value["owner"] is None, "consumed target still has an owner")
    return value


def commands(row):
    value = row.get("selected_commands")
    if value is None:
        return None  # Never fall back to the representative chosen_action.
    require(isinstance(value, list) and len(value) <= 6, "malformed motor bundle")
    seen = set()
    for c in value:
        require(isinstance(c, dict) and {"channel", "primitive", "target"} <= c.keys(), "malformed command")
        require(isinstance(c["channel"], str) and c["channel"] not in seen, "duplicate/invalid channel")
        seen.add(c["channel"])
        require(uint(c["primitive"], 32, positive=True), "invalid primitive")
        if c["target"] is not None:
            require(isinstance(c["target"], dict) and "entity" in c["target"], "malformed command target")
            target = c["target"]["entity"]
            require(target is None or identifier(target), "invalid command target")
    return value


def entity(command):
    return (command["target"] or {}).get("entity")


def outcomes(row):
    value = row.get("channel_outcomes")
    if value is None:
        return None
    require(isinstance(value, list) and len(value) <= 6, "malformed channel outcomes")
    seen = set()
    for c in value:
        require(isinstance(c, dict) and isinstance(c.get("channel"), str), "malformed outcome")
        require(c["channel"] not in seen, "duplicate outcome channel")
        seen.add(c["channel"])
        p = c.get("physical")
        require(isinstance(p, dict) and {"contact", "target_entity"} <= p.keys(), "malformed physical outcome")
        require(isinstance(p["contact"], str), "malformed contact")
        require(p["target_entity"] is None or identifier(p["target_entity"]), "invalid outcome target")
    return value


def benefit(row):
    p = row.get("physiology")
    if p is None:
        return None
    require(isinstance(p, dict), "malformed physiology")
    fields = [f"{side}_{name}" for side in ("before", "after")
              for name in ("energy", "hunger", "health", "injury", "pain")]
    if not set(fields) <= p.keys():
        return None
    p = {key: number(p[key], key, 0, 1) for key in fields}
    # Same measured-benefit guard as held_food_body_benefit in foundation_training.rs.
    return ((p["after_energy"] > p["before_energy"] or p["after_hunger"] < p["before_hunger"])
            and p["after_health"] >= p["before_health"]
            and p["after_injury"] <= p["before_injury"]
            and p["after_pain"] <= p["before_pain"])


def summarize(document, *, pilot=None, decision_budget=None, execution_status="unknown"):
    require(execution_status in {"unknown", "completed", "incomplete", "timeout"}, "invalid execution status")
    require(decision_budget is None or uint(decision_budget, 32, positive=True), "invalid decision budget")
    rows = document if isinstance(document, list) else document.get("steps") if isinstance(document, dict) else None
    require(isinstance(rows, list), "trace must contain steps")
    pilot = {} if pilot is None else pilot
    require(isinstance(pilot, dict), "pilot must be an object")
    ids = {name: set() for name in ("lesson_target", "organism_id")}
    ticks = []; parsed = []; tracked = set()
    for row in rows:
        require(isinstance(row, dict) and uint(row.get("tick"), 64), "missing/invalid decision tick")
        tick = row["tick"]
        require(not ticks or tick > ticks[-1], "duplicate or reversed decision tick")
        ticks.append(tick)
        for key in ids:
            if row.get(key) is not None:
                require(identifier(row[key]), f"invalid {key}")
                ids[key].add(row[key])
        for key, expected in (("decision_tick", tick), ("outcome_tick", tick + 1)):
            if key in row:
                require(type(row[key]) is int and row[key] == expected, f"inconsistent {key}")
        slots = row.get("object_slots")
        cs = row.get("candidates")
        if slots is not None:
            object_slots(slots)  # Reuse the existing native sensory contract.
            if cs is not None:
                candidates(cs, slots)
                for c in cs:
                    if identifier(row.get("lesson_target")) and c["target"] == row["lesson_target"] and isinstance(c["observation"], dict):
                        tracked.add(slots[c["observation"]["ObjectSlot"]]["tracked_object_id"])
        parsed.append((row, commands(row), outcomes(row),
                       ownership(row.get("lesson_target_ownership_before_decision", MISSING)),
                       ownership(row.get("lesson_target_ownership_after_outcome", MISSING))))
    require(all(len(value) <= 1 for value in ids.values()), "multiple subjects or lesson targets")
    require(len(tracked) <= 1, "ambiguous food tracking identity")
    target = next(iter(ids["lesson_target"]), None)
    subject = next(iter(ids["organism_id"]), None)
    tracked_id = next(iter(tracked), None)
    contiguous = bool(ticks) and ticks == list(range(len(ticks)))
    pilot_ticks = pilot.get("ticks")
    require(pilot_ticks is None or uint(pilot_ticks, 32), "invalid pilot tick count")
    within_budget = decision_budget is None or len(rows) <= decision_budget
    complete = (execution_status == "completed" and contiguous and within_budget
                and (pilot_ticks is None or pilot_ticks == len(rows)))
    metrics = {name: {"events": [], "unknown_ticks": []} for name in (
        "food_visible", "visibility_loss", "visual_reacquisition", "food_approach", "wrong_entity_eat",
        "own_pickup", "exact_target_consumption", "nourishing_consumption",
        "locomotion_switch_away", "locomotion_switch_back", "manipulation_switch_away", "manipulation_switch_back")}

    def event(name, known, happened=False, **extra):
        metric = metrics[name]
        if not known:
            metric["unknown_ticks"].append(tick)
        elif happened:
            metric["events"].append({"decision_tick": tick, **extra})

    previous = None
    for row, selected, physical, before, after in parsed:
        tick = row["tick"]
        visible = None if tracked_id is None or row.get("object_slots") is None else any(
            slot["tracked_object_id"] == tracked_id for slot in row["object_slots"])
        event("food_visible", visible is not None, visible is True)
        adjacent = previous is not None and previous["tick"] + 1 == tick
        transition_known = adjacent and visible is not None and previous["visible"] is not None
        for name, start, end in (("visibility_loss", True, False), ("visual_reacquisition", False, True)):
            # No transition is observable at the start, or across missing rows.
            event(name, (tick == 0 and visible is not None) or transition_known,
                  transition_known and previous["visible"] == start and visible == end)
        selected_known = selected is not None and target is not None
        targets = {channel: None for channel in CHANNELS}
        if selected_known:
            for c in selected:
                if c["channel"] in targets:
                    targets[c["channel"]] = entity(c)
        for channel in CHANNELS:
            known = adjacent and selected_known and previous["selected_known"]
            old = previous["targets"][channel] if known else None
            current = targets[channel]
            for suffix, happened in (("away", old == target and current not in (None, target)),
                                     ("back", old not in (None, target) and current == target)):
                event(f"{channel.lower()}_switch_{suffix}", (tick == 0 and selected_known) or known,
                      known and happened, from_entity=old, to_entity=current, food_visible=visible)
        event("food_approach", selected_known, selected_known and any(
            c["channel"] == "Locomotion" and c["primitive"] == 101 and entity(c) == target for c in selected or []))
        event("wrong_entity_eat", selected_known, selected_known and any(
            c["channel"] == "Manipulation" and c["primitive"] == 210 and entity(c) not in (None, target)
            for c in selected or []))
        identity = (target is not None and subject is not None and row.get("lesson_target") == target
                    and row.get("organism_id") == subject and "decision_tick" in row and "outcome_tick" in row)
        ownership_known = identity and before is not None and after is not None
        selected_physical_known = selected_known and physical is not None

        def matched(primitives, contact):
            return any(c["channel"] == "Manipulation" and c["primitive"] in primitives and entity(c) == target
                       for c in selected or []) and any(
                p["channel"] == "Manipulation" and p["physical"]["target_entity"] == target
                and p["physical"]["contact"] == contact for p in physical or [])

        pickup = ownership_known and before == {"owner": None, "consumed": False} and after == {"owner": subject, "consumed": False}
        consumption = ownership_known and not before["consumed"] and after["consumed"]
        pickup_proved = pickup and selected_physical_known and matched({210, 211}, "Touch")
        consumed_proved = consumption and selected_physical_known and matched({210}, "Consumed")
        # Unattributed ownership/consumption transitions remain unknown, not exact events.
        event("own_pickup", ownership_known and (not pickup or pickup_proved), pickup_proved)
        event("exact_target_consumption", ownership_known and (not consumption or consumed_proved), consumed_proved)
        body_benefit = benefit(row) if consumed_proved else None
        nourished_known = ownership_known and (not consumption or
            (consumed_proved and (before["owner"] != subject or body_benefit is not None)))
        event("nourishing_consumption", nourished_known,
              consumed_proved and before["owner"] == subject and body_benefit is True)
        previous = dict(tick=tick, visible=visible, selected_known=selected_known, targets=targets)
    for value in metrics.values():
        value["observed_count"] = len(value["events"])
        value["count"] = value["observed_count"] if complete and not value["unknown_ticks"] else None
    return {
        "schema_version": 1, "neural_execution": False, "qualification": False,
        "subject": subject, "food_target": target, "food_tracked_object_id": tracked_id,
        "execution": {"caller_declared_status": execution_status, "decision_budget": decision_budget,
                      "observed_decisions": len(rows), "contiguous_from_zero": contiguous,
                      "within_declared_budget": within_budget,
                      "pilot_ticks": pilot_ticks, "whole_trace_counts_available": complete,
                      "budget_exhausted": len(rows) >= decision_budget if complete and decision_budget is not None else None},
        "metrics": metrics,
        "provenance": {key: pilot.get(key) for key in ("seed", "lesson", "source_asset_digest", "teacher_mode", "initial_sampling", "silent_request", "teacher_cue_frames")},
        "definitions": {
            "switch": "Adjacent recorded decisions selecting food then a different entity, or the reverse, per motor channel. Untargeted/absent commands are not entity switches; gaps are unknown.",
            "visibility": "Captured object-slot tracking identity bound through a food candidate. Candidate absence alone is not invisibility. Reacquisition here is visual, not physical possession.",
            "nourishment": "Own held target, selected Eat, matching Manipulation Consumed outcome and ownership transition, with energy gain or hunger reduction and no health/injury/pain harm.",
            "coverage": "Null count is unknown; observed_count is only a lower bound for supplied rows. Completed status is caller-declared, not authenticated. No overall success classification is emitted.",
            "generic_consumption": "Pilot generic consumed counts and representative chosen_action are never exact nourishment evidence."}}


def load_json(path):
    with path.open('rb') as f:
        data = f.read(MAX_BYTES + 1)
    require(len(data) <= MAX_BYTES, "JSON exceeds 64 MiB")

    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result

    def nonfinite(value):
        raise ValueError(f"nonfinite JSON constant {value}")

    def finite_float(value):
        parsed = float(value)
        require(math.isfinite(parsed), "nonfinite JSON number")
        return parsed

    return json.loads(data.decode('utf-8-sig'), object_pairs_hook=pairs,
                      parse_constant=nonfinite, parse_float=finite_float), hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("--decision-budget", type=int)
    parser.add_argument("--execution-status", choices=("unknown", "completed", "incomplete", "timeout"), default="unknown")
    args = parser.parse_args()
    try:
        path = args.input
        if path.is_dir():
            paths = [path / name for name in ("lesson-trace.json", "lesson-diagnostic.json") if (path / name).exists()]
            require(len(paths) == 1, "directory must contain exactly one trace/diagnostic file")
            path = paths[0]
        document, digest = load_json(path)
        pilot_path = path.parent / "pilot.json"
        pilot, pilot_digest = load_json(pilot_path) if pilot_path.exists() else (None, None)
        result = summarize(document, pilot=pilot, decision_budget=args.decision_budget, execution_status=args.execution_status)
        result["input"] = {"trace": str(path), "sha256": digest, "pilot_sha256": pilot_digest}
    except (ValueError, OSError, RecursionError) as error:
        parser.error(str(error))
    print(json.dumps(result, sort_keys=True, allow_nan=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

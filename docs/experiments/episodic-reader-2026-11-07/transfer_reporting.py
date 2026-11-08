#!/usr/bin/env python3
"""Sanitize the eight predeclared fresh CPU lives; never open diagnostic banks.

Usage: python3 transfer_reporting.py PRIVATE PUBLIC [--require-complete] [--allow-not-started]
Only original JSON journals, summaries, and manifests are read. Incomplete lives
are reported as missing, not rerun or counted. This is a reporting check, not an
independent execution of Rust checkpoint restoration or a gameplay qualification.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import struct
import sys

CONSTRUCTION_SEED = 2026110901
EPISODES = tuple((2026111201 + i, bool(i % 2)) for i in range(4))
ASSETS = {
    "family-value-off": "ba006d31a7ceb1f75d22704893494871652b6bc0200ef1a0f31039bab11b1e6a",
    "family-value-on": "6edcd6dd1dc5bfedf8f7b73bf605bb480e36c0ba022a316840a6f0bd87629632",
}
ASSIGNMENTS = tuple((seed, arm, mirror) for seed, mirror in EPISODES for arm in ASSETS)
SLEEP_POLICY = ("Request RecoveryProtocol once immediately after the first actual A consumption, "
                "regardless of its measured value. Never request without exposure.")
SPEC = dict(seed=CONSTRUCTION_SEED, preparation_ticks=600, initial_energy=0.4,
            baseline_ticks=24, life_ticks=256, max_a_consumptions=3, nutrition=0.05,
            a_harm=0.35, regrow_ticks=16, minimum_health=0.70, maximum_injury=0.25,
            sleep_policy=SLEEP_POLICY)
PUBLIC_LIMITS = {"summary.json": 131072, "causal-rows.json": 2097152, "report.md": 131072}
PRIVATE_JSON_LIMIT = 16 * 1024 * 1024
JOURNAL_LIMIT = 64 * 1024 * 1024
LINE_LIMIT = 1024 * 1024
FAMILIES = ("Idle", "Rest", "Inspect", "Approach", "Avoid", "Contact", "Ingest", "Other")
KINDS = ("Idle", "Hold", "Rest", "Inspect", "Move", "Interact", "Vocalize", "Write", "Gesture", "Look")
SLOTS = {"Move": 0, "Look": 1, "Interact": 2, "Write": 2, "Vocalize": 3,
         "Hold": 4, "Rest": 4, "Inspect": 4}
MARGIN_KEYS = ("ingest_a", "ingest_b", "ingest_a_minus_b", "approach_a", "avoid_a")
STOP_REASONS = ("tick_budget", "harmful_consumption_cap", "vitality_safeguard",
                "organism_not_alive", "host_error")


class ValidationError(ValueError):
    """Input or publication contract failed. Error messages never include payloads."""


def need(condition, message):
    if not condition:
        raise ValidationError(message)


def integer(value, low=0, high=2**64 - 1):
    need(type(value) is int and low <= value <= high, "Invalid integer")
    return value


def number(value):
    need(type(value) in (int, float) and math.isfinite(value), "Invalid finite number")
    return value


def truth(value):
    need(value is True, "Preservation or provenance check is not true")


def equal_number(actual, expected):
    need(math.isclose(number(actual), expected, rel_tol=1e-6, abs_tol=2e-6), "Numeric mismatch")


def no_duplicates(pairs):
    result = {}
    for key, value in pairs:
        need(key not in result, "Duplicate JSON key")
        result[key] = value
    return result


def decode(data):
    try:
        return json.loads(data, object_pairs_hook=no_duplicates,
                          parse_constant=lambda _: (_ for _ in ()).throw(ValidationError("Nonfinite JSON")))
    except (UnicodeError, json.JSONDecodeError, RecursionError) as error:
        raise ValidationError("Invalid JSON") from error


def read_bytes(path, limit):
    need(path.is_file() and not path.is_symlink(), "Missing file or symlink input")
    need(path.stat().st_size <= limit, "Input size limit exceeded")
    with path.open("rb") as file:
        data = file.read(limit + 1)
    need(len(data) <= limit, "Input size limit exceeded")
    return data


def read_json(path):
    return decode(read_bytes(path, PRIVATE_JSON_LIMIT))


def check_spec(spec):
    need(type(spec) is dict and set(spec) == set(SPEC), "Spec keys differ from predeclaration")
    for key, expected in SPEC.items():
        if type(expected) is float:
            equal_number(spec[key], expected)
        else:
            need(type(spec[key]) is type(expected) and spec[key] == expected, "Wrong budget or construction seed")


def check_arm(arm, name):
    expected = dict(name=name, action_gain=0.25, memory_reader=True, memory_gain=12.0,
                    family_valence_reader=name == "family-value-on",
                    filename=name + ".alife-foundation", sha256=ASSETS[name])
    need(type(arm) is dict and set(arm) == set(expected), "Wrong arm structure")
    for key, value in expected.items():
        if type(value) is float:
            equal_number(arm[key], value)
        else:
            need(type(arm[key]) is type(value) and arm[key] == value, "Wrong asset or arm")


def check_episode(value, seed, mirror):
    need(value == {"seed": seed, "mirror_z": mirror} and type(value.get("mirror_z")) is bool,
         "Wrong episode assignment")


def sha(value):
    need(type(value) is str and re.fullmatch("[0-9a-f]{64}", value) is not None, "Invalid SHA256")
    return value


def validate_manifest(manifest):
    need(manifest["schema"] == "fresh-native-transfer-v1", "Wrong transfer schema")
    plan = manifest["plan"]
    need(plan["construction_seed"] == CONSTRUCTION_SEED and type(plan["max_lives"]) is int
         and plan["max_lives"] == 8, "Wrong construction seed or life budget")
    need(len(plan["episodes"]) == 4, "Wrong episode count")
    for actual, (seed, mirror) in zip(plan["episodes"], EPISODES):
        check_episode(actual, seed, mirror)
    need(plan["asset_sha256"] == list(ASSETS.values()), "Wrong frozen asset digests")
    check_spec(manifest["spec"])
    sha(manifest["executable_sha256"])
    sha(manifest["retrieval_source_sha256"])
    need(type(manifest["source_commit"]) is str and re.fullmatch("[0-9a-f]{40}", manifest["source_commit"]),
         "Invalid source commit")


def targets_by_label(objects, expected_ids=None):
    need(type(objects) is list and len(objects) == 2, "Expected exactly two resource targets")
    result = {}
    for obj in objects:
        need(obj["label"] in ("A", "B") and obj["label"] not in result, "Ambiguous target label")
        integer(obj["id"], 1)
        need(obj["kind"] == "Food", "Wrong resource kind")
        need(type(obj["consumed"]) is bool, "Invalid resource state")
        if obj["carried_by"] is not None:
            integer(obj["carried_by"], 1)
        result[obj["label"]] = obj
    need(result["A"]["id"] != result["B"]["id"], "Aliased target identities")
    if expected_ids is not None:
        need({key: obj["id"] for key, obj in result.items()} == expected_ids, "Target identity changed")
    return result


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def draw(seed, counter, factor):
    mask = 0xffffffff
    value = (seed ^ ((counter * 0x9e3779b9) & mask) ^ ((factor * 0x85ebca6b) & mask)) & mask
    value = ((value ^ (value >> 16)) * 0x7feb352d) & mask
    value = ((value ^ (value >> 15)) * 0x846ca68b) & mask
    value ^= value >> 16
    return f32(f32((value >> 9) + 0.5) / 8388608.0)


def sample(logits, mask, random):
    indices = [i for i in range(len(logits)) if mask & (1 << i)]
    need(bool(indices), "Empty sampling mask")
    maximum = max(logits[i] for i in indices)
    weights = [f32(math.exp(f32(logits[i] - maximum))) for i in indices]
    total = 0.0
    for weight in weights:
        total = f32(total + weight)
    threshold, cumulative = f32(random * total), 0.0
    for i, weight in zip(indices, weights):
        cumulative = f32(cumulative + weight)
        if threshold < cumulative:
            return i
    return indices[-1]


def enabled_motor_slots(channels):
    # phenotype/decoder.rs FACTORIZED_MOTOR_CHANNELS, not the six-slot sampling
    # wire order. Orientation and SpeciesSpecific are absent from this founder.
    prefix = ["Locomotion", "Manipulation", "Vocal", "Posture"]
    need(type(channels) is list and 1 <= len(channels) <= 4, "Invalid native motor channels")
    need(channels == prefix[:len(channels)], "Native motor channels are not a canonical prefix")
    return {dict(Locomotion=0, Manipulation=2, Vocal=3, Posture=4)[channel] for channel in channels}


def sampled_indices(candidates, logits, seed, counter, channels):
    count = len(candidates)
    enabled = enabled_motor_slots(channels)
    masks = [0] * 6
    forced = []
    for i, candidate in enumerate(candidates):
        slot = SLOTS.get(candidate["kind"])
        # Bind enabled channels to this fresh body's native phenotype receipt.
        # A representative may force a slot even outside these enabled masks.
        if slot in enabled:
            masks[slot] |= 1 << i
        forced.append(4 if candidate["kind"] in ("Idle", "Gesture") else slot)
    sampling_seed = (seed ^ (seed >> 32)) & 0xffffffff
    representative = sample(logits, (1 << count) - 1, draw(sampling_seed, counter, 0))
    motor = [representative if forced[representative] == slot else
             sample(logits, mask, draw(sampling_seed, counter, slot + 1)) if mask else None
             for slot, mask in enumerate(masks)]
    return representative, motor, masks, forced


def check_action(action, candidates, logits, seed, counter, channels):
    representative, motor, masks, forced = sampled_indices(candidates, logits, seed, counter, channels)
    need(action["candidate_count"] == len(candidates) and action["representative_mask"] == (1 << len(candidates)) - 1,
         "Sampling receipt has wrong candidate count or mask")
    need(action["motor_masks"] == masks and action["forced_slots"] == forced,
         "Sampling receipt has wrong channel metadata")
    need(action["representative"] == representative and action["motor_candidates"] == motor,
         "Recorded sampled action does not match actual frame")
    equal_number(action["temperature"], 1.0)
    need(number(action["old_joint_log_probability"]) <= 1e-5, "Invalid sampling probability")
    return representative, motor


def margins(candidates, logits, ids):
    def best(label, family):
        scores = [score for candidate, score in zip(candidates, logits)
                  if candidate["family"] == family and candidate["target"]["entity"] == ids[label]]
        return max(scores) if scores else None
    a, b = best("A", "Ingest"), best("B", "Ingest")
    return dict(ingest_a=a, ingest_b=b, ingest_a_minus_b=f32(a - b) if a is not None and b is not None else None,
                approach_a=best("A", "Approach"), avoid_a=best("A", "Avoid"))


def check_margins(actual, expected):
    need(type(actual) is dict and set(actual) == set(MARGIN_KEYS), "Wrong margin fields")
    for key, value in expected.items():
        if value is None:
            need(actual[key] is None, "Margin claims an absent candidate")
        else:
            equal_number(actual[key], value)


def action_label(candidate, ids):
    if candidate is None:
        return "none"
    target = next((label for label, entity in ids.items() if candidate["target"]["entity"] == entity), "other")
    return candidate["family"] + ":" + target


def consumed_ids(outcome):
    physical = [outcome["physical"]]
    joint = outcome.get("joint")
    if joint is not None:
        physical += [channel["physical"] for channel in joint.get("channel_outcomes", [])]
    return {part["target_entity"] for part in physical if part["contact"] == "Consumed"}


def empty_counts():
    return dict(a_meals=0, b_meals=0, negative_a_biological_credit=0, negative_a_experienced_valence=0,
                a_ingest_opportunities=0, paired_ingest_opportunities=0,
                a_contact_ingest_opportunities=0, paired_contact_ingest_opportunities=0,
                post_harm_a_contact_opportunities=0, post_harm_paired_contact_opportunities=0,
                held_a_decisions=0, held_b_decisions=0, held_a_b_ingest_attempts=0,
                held_a_b_ingest_failures=0, held_a_release_opportunities=0,
                held_a_release_selections=0, held_a_release_failures=0,
                held_a_avoid_selections=0, detached_rows=0, family_ablation_choice_changes=0,
                family_ablation_to_ingest_a=0)


def analyze_life(journal, summary, manifest, seed, arm, mirror):
    """Validate an original life journal, returning only bounded scalar reductions."""
    need(summary["schema"] == "fresh-native-life-result-v1" and summary["complete"] is True,
         "Incomplete or wrong life summary")
    check_episode(summary["episode"], seed, mirror)
    need(summary["seed"] == seed and summary["construction_seed"] == CONSTRUCTION_SEED, "Wrong life seed")
    check_arm(summary["arm"], arm)
    for key in ("source_commit", "retrieval_source_sha256"):
        need(summary[key] == manifest[key], "Life provenance differs from manifest")
    need(summary["backend"] == "native-cpu" and summary["scene"] == "learning"
         and summary["experiment"] == "family-value", "Wrong life mode")
    truth(summary["optimizer_exact"])
    cold = summary["cold_restore"]
    truth(cold["checkpoint_roundtrip_exact"])
    truth(cold["next_checkpoint_exact"])
    need(type(cold["next_step"]["completed"]) is bool, "Missing cold next-step result")
    if not cold["next_step"]["completed"]:
        need(type(cold["next_step"].get("identical_atomic_error")) is str, "Missing equal cold error")
    need(summary["reversal_tested"] is False and summary["unrelated_acquired_retention_tested"] is False,
         "Unexpected preservation/generalization claim")
    raw = read_bytes(journal, JOURNAL_LIMIT)
    lines = raw.splitlines()
    need(1 <= len(lines) <= 1 + 256 * 2 + 1, "Wrong journal length")
    need(all(0 < len(line) <= LINE_LIMIT for line in lines), "Wrong journal line size")
    events = [decode(line) for line in lines]
    start = events[0]
    need(start["event"] == "start" and start["scene"] == "learning" and start["backend"] == "native-cpu"
         and start["experiment"] == "family-value", "Wrong journal start")
    check_arm(start["arm"], arm)
    channels = start["motor_channels"]
    enabled_motor_slots(channels)
    check_spec(start["spec"])
    check_episode(start["episode"], seed, mirror)
    need(start["construction_seed"] == CONSTRUCTION_SEED and start["retrieval_source_sha256"] == manifest["retrieval_source_sha256"],
         "Wrong journal provenance")
    for key in ("fresh_disposable", "no_adam_updates", "no_learning_off_comparator"):
        truth(start[key])
    previous_targets = targets_by_label(start["target_objects"])
    ids = {label: obj["id"] for label, obj in previous_targets.items()}
    target_order = [obj["id"] for obj in start["target_objects"]]
    for label, expected_z in (("A", 0.35 if mirror else -0.35), ("B", -0.35 if mirror else 0.35)):
        obj = previous_targets[label]
        need(obj["carried_by"] is None and obj["consumed"] is False, "Life did not start with fresh resources")
        equal_number(obj["position"]["z"], expected_z)
        equal_number(obj["position"]["x"], 0.35)
        equal_number(obj["position"]["y"], 0.0)
        equal_number(obj["nutrition"], 0.05)
        equal_number(obj["hazard_pain"], 0.35 if label == "A" else 0.0)
    counts, causal = empty_counts(), []
    steps = decisions = consolidations = post_sleep = 0
    requested_tick = first_a_tick = None
    last_boundary = None
    pending_row = None
    last_counter = None
    for event in events[1:]:
        kind = event["event"]
        need(kind in ("decision", "boundary", "requested_recovery_sleep"), "Unknown or duplicate journal event")
        if kind == "requested_recovery_sleep":
            need(last_boundary is not None and requested_tick is None and first_a_tick is not None
                 and event["tick"] == first_a_tick + 1 == last_boundary["tick"], "Invalid recovery-sleep timing")
            requested_tick = integer(event["tick"])
            continue
        need(event["index"] == steps and event["tick"] == 600 + steps + (kind == "boundary"),
             "Noncontiguous index or tick")
        if kind == "boundary":
            steps += 1
            need(steps <= 256, "Tick budget exceeded")
            consolidations += integer(event["consolidations"], 0, 256)
            if pending_row is not None and requested_tick is not None and consolidations > 0:
                post_sleep += 1
            new_targets = targets_by_label(event["targets"], ids)
            if pending_row is not None:
                held_a, release, own = pending_row
                if held_a and release and new_targets["A"]["carried_by"] == own:
                    counts["held_a_release_failures"] += 1
            previous_targets, last_boundary, pending_row = new_targets, event, None
            continue
        need(pending_row is None, "Multiple decisions for one learner tick")
        decisions += 1
        counter = integer(event["sampling_counter"], 0, 2**32 - 1)
        need(last_counter is None or counter == last_counter + 1, "Sampling counter discontinuity")
        last_counter = counter
        candidates, logits = event["candidates"], event["logits"]
        need(type(candidates) is list and 1 <= len(candidates) <= 32 and len(candidates) == len(logits), "Invalid actual frame")
        candidate_indices = set()
        for candidate, logit in zip(candidates, logits):
            integer(candidate["candidate_index"], 0, 65535)
            need(candidate["candidate_index"] not in candidate_indices, "Duplicate candidate identity")
            candidate_indices.add(candidate["candidate_index"])
            need(candidate["family"] in FAMILIES and candidate["kind"] in KINDS, "Unknown candidate semantics")
            need(type(candidate["features"]) is list and len(candidate["features"]) == 24, "Invalid feature frame")
            for feature in candidate["features"]:
                number(feature)
            number(logit)
        representative, motor = check_action(event["action"], candidates, logits, seed, counter, channels)
        command = event["selected_command"]
        chosen = candidates[representative]
        need(command["action_id"] == chosen["action_id"] and command["target_entity"] == chosen["target"]["entity"],
             "Selected command differs from sampled representative")
        actual_margins = margins(candidates, logits, ids)
        check_margins(event["target_margins"], actual_margins)
        outcome = event["outcome"]
        own = integer(outcome["organism_id"], 1)
        need(command["organism_id"] == own and type(outcome["success"]) is bool, "Wrong outcome owner")
        meals = consumed_ids(outcome)
        need(meals <= set(ids.values()), "Consumed an unassigned resource")
        indices = event["consumed_target_indices"]
        need(type(indices) is list and all(type(i) is int and 0 <= i < 2 for i in indices)
             and len(set(indices)) == len(indices) and {target_order[i] for i in indices} == meals,
             "Consumed-target identity mapping mismatch")
        had_harm = counts["negative_a_biological_credit"] > 0
        has = {label: any(c["family"] == "Ingest" and c["target"]["entity"] == entity for c in candidates)
               for label, entity in ids.items()}
        contact = {label: any(c["family"] == "Ingest" and c["target"]["entity"] == entity
                             and c["features"][18] > 0 for c in candidates) for label, entity in ids.items()}
        counts["a_ingest_opportunities"] += has["A"]
        counts["paired_ingest_opportunities"] += has["A"] and has["B"]
        counts["a_contact_ingest_opportunities"] += contact["A"]
        counts["paired_contact_ingest_opportunities"] += contact["A"] and contact["B"]
        counts["post_harm_a_contact_opportunities"] += had_harm and contact["A"]
        counts["post_harm_paired_contact_opportunities"] += had_harm and contact["A"] and contact["B"]
        reward, harm, valence = (number(event[key]) for key in ("biological_reward", "aversive_harm", "experienced_valence"))
        need(harm >= 0, "Negative aversive harm")
        for label, entity in ids.items():
            counts[label.lower() + "_meals"] += entity in meals
        if ids["A"] in meals:
            first_a_tick = event["tick"] if first_a_tick is None else first_a_tick
            counts["negative_a_biological_credit"] += reward < 0 and harm > 0
            counts["negative_a_experienced_valence"] += valence < 0 and harm > 0
        need(counts["a_meals"] <= 3, "Harmful-dose budget exceeded")
        held_a = previous_targets["A"]["carried_by"] == own
        held_b = previous_targets["B"]["carried_by"] == own
        manipulation = candidates[motor[2]] if motor[2] is not None else None
        locomotion = candidates[motor[0]] if motor[0] is not None else None
        ingest_b = manipulation is not None and manipulation["family"] == "Ingest" and manipulation["target"]["entity"] == ids["B"]
        # GRAB is the frozen world's public action semantic, not a target entity ID.
        is_release = lambda c: c is not None and c["action_id"] == 211 and c["target"]["entity"] == ids["A"]
        release = is_release(manipulation)
        counts["held_a_decisions"] += held_a
        counts["held_b_decisions"] += held_b
        counts["held_a_b_ingest_attempts"] += held_a and ingest_b
        counts["held_a_b_ingest_failures"] += held_a and ingest_b and ids["B"] not in meals
        counts["held_a_release_opportunities"] += held_a and any(is_release(c) for c in candidates)
        counts["held_a_release_selections"] += held_a and release
        counts["held_a_avoid_selections"] += held_a and locomotion is not None and locomotion["family"] == "Avoid"
        pending_row = held_a, release, own
        detached = event["detached_assay"]
        if detached is None:
            continue
        sha(detached["uncompressed_provenance_sha256"])
        check_margins(detached["actual_pre_update"], actual_margins)
        ablated = detached["without_ingest_family_valence_context"]
        need(set(ablated) == set(MARGIN_KEYS), "Wrong detached margin fields")
        counter_logits = list(logits)
        for i, candidate in enumerate(candidates):
            if candidate["family"] == "Ingest":
                label = next((key for key, entity in ids.items() if candidate["target"]["entity"] == entity), None)
                need(label is not None, "Unassigned Ingest target in causal assay")
                need(sum(c["family"] == "Ingest" and c["target"]["entity"] == ids[label] for c in candidates) == 1,
                     "Ambiguous Ingest margin reconstruction")
                counter_logits[i] = number(ablated["ingest_" + label.lower()])
        check_margins(ablated, margins(candidates, counter_logits, ids))
        # Reader-off fixes the genetic coefficient, not later somatic offsets.
        # Its acquired lane-32 contribution must not be assumed to remain zero.
        counter_rep, counter_motor = check_action(detached["without_ingest_family_valence_choice_same_counter"],
                                                 candidates, counter_logits, seed, counter, channels)
        actual_action = action_label(manipulation, ids)
        counter_action = action_label(candidates[counter_motor[2]] if counter_motor[2] is not None else None, ids)
        changed = (representative, motor) != (counter_rep, counter_motor)
        to_a = counter_action == "Ingest:A" and actual_action != "Ingest:A"
        counts["detached_rows"] += 1
        counts["family_ablation_choice_changes"] += changed
        counts["family_ablation_to_ingest_a"] += to_a
        causal.append(dict(seed=seed, arm=arm, mirror_z=mirror, index=event["index"], tick=event["tick"],
                           after_harm=had_harm, held_a=held_a, actual_manipulation=actual_action,
                           ablated_manipulation=counter_action, actual_representative=action_label(chosen, ids),
                           ablated_representative=action_label(candidates[counter_rep], ids),
                           sampled_choice_changed=changed, ablation_to_ingest_a=to_a,
                           actual_scores=actual_margins, ablated_scores={key: ablated[key] for key in MARGIN_KEYS}))
    need(pending_row is None, "Unfinished journal boundary")
    need(summary["steps"] == steps and summary["decisions"] == decisions, "Summary event counts mismatch")
    summary_counts = {"negative_a_meals": "negative_a_biological_credit",
                      "negative_a_experienced_valence_meals": "negative_a_experienced_valence",
                      **{key: key for key in ("a_ingest_opportunities", "paired_ingest_opportunities",
                                             "a_contact_ingest_opportunities", "paired_contact_ingest_opportunities")}}
    for key, counter_key in summary_counts.items():
        need(type(summary[key]) is int and summary[key] == counts[counter_key], "Summary counter mismatch")
    need(summary["consumptions"] == [counts[next(label.lower() for label, entity in ids.items() if entity == target) + "_meals"]
                                      for target in target_order], "Summary meal mapping mismatch")
    need((first_a_tick is None and requested_tick is None) or requested_tick == first_a_tick + 1,
         "Missing exposure-triggered sleep request")
    need(summary["requested_sleep_tick"] == requested_tick and summary["sleep_consolidations"] == consolidations
         and summary["post_sleep_decisions"] == post_sleep, "Summary sleep counts mismatch")
    phase = summary["phase_provenance"]
    truth(phase["all_consumed_known"])
    need(type(phase["meals_checked"]) is int and phase["meals_checked"] == counts["a_meals"] + counts["b_meals"],
         "Wrong consumed-phase check count")
    stop = summary["stop_reason"]
    need(type(stop) is str, "Invalid stop reason")
    stop = "host_error" if stop.startswith("host_error: ") else stop
    need(stop in STOP_REASONS, "Unknown stop reason")
    body = last_boundary["body"] if last_boundary is not None else start["initial_biology"]["body"]
    for key in ("health", "injury", "energy"):
        equal_number(summary["final_body"][key], number(body[key]))
    if stop == "tick_budget":
        need(steps == 256, "Early tick-budget stop")
    need(steps > 0 or stop == "host_error", "Zero-step life without host error")
    if stop == "harmful_consumption_cap":
        need(counts["a_meals"] == 3, "Dose-cap stop without dose cap")
    if stop == "vitality_safeguard":
        need(body["health"] < 0.70 or body["injury"] > 0.25, "Vitality stop without safeguard")
    final_held = next((label for label, obj in previous_targets.items() if obj["carried_by"] is not None), "none")
    return dict(seed=seed, arm=arm, mirror_z=mirror, asset_sha256=ASSETS[arm], steps=steps,
                decisions=decisions, stop_reason=stop, counts=counts, requested_sleep=requested_tick is not None,
                sleep_consolidations=consolidations, post_sleep_decisions=post_sleep,
                founder_optimizer_exact=True, cold_checkpoint_exact=True, cold_next_checkpoint_exact=True,
                cold_next_step_completed=cold["next_step"]["completed"], phase_meals_checked=phase["meals_checked"],
                all_consumed_known=True, final_held=final_held,
                final_health=body["health"], final_injury=body["injury"], final_energy=body["energy"],
                journal_sha256=hashlib.sha256(raw).hexdigest()), causal


# Public schemas are recursive exact allowlists. There is no arbitrary string,
# object, or array escape hatch and no path/diagnostic payload field.
def enum(*values):
    return ("enum", values)


def array(schema, maximum):
    return ("array", schema, maximum)


def nullable(schema):
    return ("nullable", schema)


COUNT = ("int", 0, 4096)
SEED = enum(*(seed for seed, _ in EPISODES))
ARM = enum(*ASSETS)
CASE_ID = {"seed": SEED, "arm": ARM, "mirror_z": bool}
SCORES = {key: nullable(float) for key in MARGIN_KEYS}
ACTION = enum("none", *(family + ":" + target for family in FAMILIES for target in ("A", "B", "other")))
LIFE_SCHEMA = dict(CASE_ID, asset_sha256="sha256", steps=("int", 0, 256), decisions=("int", 0, 256),
                   stop_reason=enum(*STOP_REASONS), counts={key: COUNT for key in empty_counts()},
                   requested_sleep=bool, sleep_consolidations=COUNT, post_sleep_decisions=COUNT,
                   founder_optimizer_exact=enum(True), cold_checkpoint_exact=enum(True),
                   cold_next_checkpoint_exact=enum(True), cold_next_step_completed=bool,
                   phase_meals_checked=COUNT, all_consumed_known=enum(True), final_held=enum("A", "B", "none"),
                   final_health=float, final_injury=float, final_energy=float, journal_sha256="sha256")
SUMMARY_SCHEMA = dict(schema=enum("fresh-native-transfer-public-v1"), backend=enum("native-cpu"),
                      status=enum("complete", "partial", "not_started"), expected_lives=enum(8), completed_lives=("int", 0, 8),
                      construction_seed=enum(CONSTRUCTION_SEED), source_commit=nullable("commit"),
                      executable_sha256=nullable("sha256"), retrieval_source_sha256=nullable("sha256"),
                      lives=array(LIFE_SCHEMA, 8), missing_assignments=array(CASE_ID, 8),
                      completion_marker_present=bool, automatic_improvement_claim=enum(False),
                      reversal_tested=enum(False), unrelated_acquired_retention_tested=enum(False))
CAUSAL_ROW_SCHEMA = dict(CASE_ID, index=("int", 0, 255), tick=("int", 600, 855), after_harm=bool, held_a=bool,
                         actual_manipulation=ACTION, ablated_manipulation=ACTION,
                         actual_representative=ACTION, ablated_representative=ACTION,
                         sampled_choice_changed=bool, ablation_to_ingest_a=bool,
                         actual_scores=SCORES, ablated_scores=SCORES)
CAUSAL_SCHEMA = dict(schema=enum("fresh-native-transfer-causal-v1"), backend=enum("native-cpu"),
                     comparison=enum("fixed-state-ingest-family-valence-ablation"), rows=array(CAUSAL_ROW_SCHEMA, 2048))


def validate_schema(value, schema):
    if type(schema) is dict:
        need(type(value) is dict and set(value) == set(schema), "Public object has missing or extra fields")
        for key, child in schema.items():
            validate_schema(value[key], child)
    elif schema is bool:
        need(type(value) is bool, "Public boolean invalid")
    elif schema is float:
        need(abs(number(value)) <= 1e6, "Public numeric bound exceeded")
    elif schema == "sha256":
        sha(value)
    elif schema == "commit":
        need(type(value) is str and re.fullmatch("[0-9a-f]{40}", value), "Public source commit invalid")
    elif schema[0] == "enum":
        need(any(type(value) is type(option) and value == option for option in schema[1]), "Public enum invalid")
    elif schema[0] == "int":
        integer(value, schema[1], schema[2])
    elif schema[0] == "array":
        need(type(value) is list and len(value) <= schema[2], "Public array bound exceeded")
        for child in value:
            validate_schema(child, schema[1])
    elif schema[0] == "nullable":
        if value is not None:
            validate_schema(value, schema[1])
    else:
        raise ValidationError("Unknown publication schema")


def validate_public(summary, causal):
    validate_schema(summary, SUMMARY_SCHEMA)
    validate_schema(causal, CAUSAL_SCHEMA)
    cases = [(life["seed"], life["arm"], life["mirror_z"]) for life in summary["lives"]]
    missing = [(life["seed"], life["arm"], life["mirror_z"]) for life in summary["missing_assignments"]]
    need(len(set(cases + missing)) == 8 and set(cases + missing) == set(ASSIGNMENTS), "Public assignment coverage mismatch")
    need(summary["completed_lives"] == len(cases), "Public life count mismatch")
    complete = not missing and summary["completion_marker_present"]
    provenance = (summary["source_commit"], summary["executable_sha256"], summary["retrieval_source_sha256"])
    if summary["status"] == "not_started":
        need(not cases and not summary["completion_marker_present"] and all(value is None for value in provenance),
             "False not-started claim")
    else:
        need(all(value is not None for value in provenance), "Missing run provenance")
        need(summary["status"] == ("complete" if complete else "partial"), "False public completion claim")
    for life in summary["lives"]:
        need(life["asset_sha256"] == ASSETS[life["arm"]] and life["counts"]["a_meals"] <= 3, "Public asset or dose mismatch")
    seen = set()
    for row in causal["rows"]:
        case = row["seed"], row["arm"], row["mirror_z"]
        key = (*case, row["index"])
        need(case in cases and key not in seen, "Duplicate or unassigned causal row")
        seen.add(key)


def collect(private, allow_not_started=False):
    private = Path(private)
    if allow_not_started and not (private / "manifest.json").exists():
        need(not private.is_symlink(), "Invalid private input directory")
        need(not private.exists() or not any(private.rglob("*")), "Missing manifest with existing private files")
        summary = dict(schema="fresh-native-transfer-public-v1", backend="native-cpu", status="not_started",
                       expected_lives=8, completed_lives=0, construction_seed=CONSTRUCTION_SEED,
                       source_commit=None, executable_sha256=None, retrieval_source_sha256=None, lives=[],
                       missing_assignments=[dict(seed=seed, arm=arm, mirror_z=mirror) for seed, arm, mirror in ASSIGNMENTS],
                       completion_marker_present=False, automatic_improvement_claim=False, reversal_tested=False,
                       unrelated_acquired_retention_tested=False)
        causal = dict(schema="fresh-native-transfer-causal-v1", backend="native-cpu",
                      comparison="fixed-state-ingest-family-valence-ablation", rows=[])
        validate_public(summary, causal)
        return summary, causal
    need(private.is_dir() and not private.is_symlink(), "Invalid private input directory")
    manifest = read_json(private / "manifest.json")
    validate_manifest(manifest)
    expected = {f"episode-{seed}/{arm}.learning.summary.json" for seed, arm, _ in ASSIGNMENTS}
    actual = {str(path.relative_to(private)) for path in private.rglob("*.summary.json")}
    need(actual <= expected, "Unexpected or duplicate life summary")
    lives, rows, originals, missing = [], [], [], []
    for seed, arm, mirror in ASSIGNMENTS:
        folder = private / f"episode-{seed}"
        need(not folder.is_symlink(), "Symlink episode directory")
        summary_path = folder / f"{arm}.learning.summary.json"
        if not summary_path.exists():
            missing.append(dict(seed=seed, arm=arm, mirror_z=mirror))
            continue
        summary = read_json(summary_path)
        life, case_rows = analyze_life(folder / f"{arm}.learning.jsonl", summary, manifest, seed, arm, mirror)
        lives.append(life)
        rows.extend(case_rows)
        originals.append(summary)
    marker = private / "transfer.complete.json"
    if marker.exists():
        completion = read_json(marker)
        need(completion["complete"] is True and not missing and len(completion["results"]) == 8,
             "Completion marker has non-eight coverage")
        by_case = {}
        for result in completion["results"]:
            key = result["seed"], result["arm"]["name"]
            need(key not in by_case, "Duplicate completion case")
            by_case[key] = result
        need(by_case == {(result["seed"], result["arm"]["name"]): result for result in originals},
             "Completion results differ from original life summaries")
    summary = dict(schema="fresh-native-transfer-public-v1", backend="native-cpu",
                   status="complete" if not missing and marker.exists() else "partial",
                   expected_lives=8, completed_lives=len(lives), construction_seed=CONSTRUCTION_SEED,
                   source_commit=manifest["source_commit"], executable_sha256=manifest["executable_sha256"],
                   retrieval_source_sha256=manifest["retrieval_source_sha256"], lives=lives,
                   missing_assignments=missing, completion_marker_present=marker.exists(),
                   automatic_improvement_claim=False, reversal_tested=False, unrelated_acquired_retention_tested=False)
    causal = dict(schema="fresh-native-transfer-causal-v1", backend="native-cpu",
                  comparison="fixed-state-ingest-family-valence-ablation", rows=rows)
    validate_public(summary, causal)
    return summary, causal


def render_report(summary):
    text = ["# Frozen episodic-reader transfer", "", f"Status: {summary['status']}. Validated completed lives: {summary['completed_lives']}/8.",
            "Backend: native CPU. Four fresh paired seeds, alternating mirrored Z positions. Each life is capped at 256 ticks and three A meals.", "",
            "## Results", ""]
    for life in summary["lives"]:
        c = life["counts"]
        text.append(f"- {life['seed']} {life['arm']}, mirror={str(life['mirror_z']).lower()}: "
                    f"{life['steps']} ticks; A/B meals {c['a_meals']}/{c['b_meals']}; stop {life['stop_reason']}. "
                    f"Negative biological-credit A meals {c['negative_a_biological_credit']}; "
                    f"post-harm A/paired contact opportunities {c['post_harm_a_contact_opportunities']}/{c['post_harm_paired_contact_opportunities']}. "
                    f"Held-A B-ingest failures {c['held_a_b_ingest_failures']}/{c['held_a_b_ingest_attempts']}; "
                    f"release selections/opportunities {c['held_a_release_selections']}/{c['held_a_release_opportunities']}; "
                    f"final held {life['final_held']}. Family ablation changes {c['family_ablation_choice_changes']}/{c['detached_rows']}, "
                    f"including {c['family_ablation_to_ingest_a']} switches to Ingest(A).")
    if summary["missing_assignments"]:
        text += ["", "Missing/uncompleted assignments: " + ", ".join(f"{case['seed']} {case['arm']}" for case in summary["missing_assignments"]) + "."]
    text += ["", "## Interpretation and verification limits", "",
             "Counters were recomputed from original fresh journals. A/B identities come from labeled start targets, not numeric-ID assumptions. Contact uses grounded feature lane 18. A post-harm opportunity is a later actual decision after an A meal with negative native biological credit and positive measured harm.",
             "Held counts use the previous boundary's ownership. Failed B ingestion while holding A means the manipulation channel selected Ingest(B) without B consumption. This does not by itself prove possession was the sole cause. Release is the frozen world's GRAB toggle (action 211) on its own held A; no new grip behavior is implemented.",
             "Actual and family-ablated sampled actions are checked against each actual candidate frame and its sampling counter. Ablation reconstruction requires one Ingest candidate per target. Detached comparisons hold the existing state fixed; they are not alternate future rollouts or proof of learned synaptic avoidance.",
             "Founder/optimizer preservation, exact checkpoint roundtrip and next checkpoint equality are required recorded checks. Consumed-phase coverage requires a recorded knownConsumed check for every meal. This reporter verifies their flags and coverage, but does not reopen private checkpoints or independently rerun those Rust checks.",
             "No automatic improvement, generalization, reliable feeding/survival, reversal, or unrelated acquired-memory retention claim is made. A partial report preserves completed evidence and must not be represented as a completed eight-life study.", ""]
    return "\n".join(text)


def validate_public_outputs(public):
    """Reopen only the three public files and enforce schemas, sizes and prose.

    Other files in PUBLIC belong to the orchestrator and are not read here.
    Returns (summary, causal_rows) after validation; raises on any failure.
    """
    public = Path(public)
    need(public.is_dir() and not public.is_symlink(), "Invalid publication directory")
    payloads = {name: read_bytes(public / name, limit) for name, limit in PUBLIC_LIMITS.items()}
    summary, causal = decode(payloads["summary.json"]), decode(payloads["causal-rows.json"])
    validate_public(summary, causal)
    need(payloads["report.md"] == render_report(summary).encode("utf-8"), "Public report does not match sanitized summary")
    return summary, causal


def publish(private, public, require_complete=False, allow_not_started=False):
    summary, causal = collect(private, allow_not_started=allow_not_started)
    public = Path(public)
    need(not public.is_symlink() and public.resolve() != Path(private).resolve(), "Unsafe public directory")
    payloads = {"summary.json": json.dumps(summary, indent=2, allow_nan=False) + "\n",
                "causal-rows.json": json.dumps(causal, indent=2, allow_nan=False) + "\n",
                "report.md": render_report(summary)}
    for name, content in payloads.items():
        need(len(content.encode("utf-8")) <= PUBLIC_LIMITS[name], "Publication size limit exceeded")
        need(not (public / name).is_symlink(), "Symlink public destination")
    public.mkdir(parents=True, exist_ok=True)
    # Each file is replaced atomically; a process interruption cannot truncate it.
    # The orchestrator validates all three together before uploading its bundle.
    for name, content in payloads.items():
        temp = public / ("." + name + ".tmp")
        need(not temp.is_symlink(), "Symlink temporary destination")
        with temp.open("w", encoding="utf-8") as file:
            file.write(content)
        temp.replace(public / name)
    return 2 if summary["status"] == "not_started" or (require_complete and summary["status"] != "complete") else 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("private", type=Path)
    parser.add_argument("public", type=Path)
    parser.add_argument("--require-complete", action="store_true")
    parser.add_argument("--allow-not-started", action="store_true")
    args = parser.parse_args(argv)
    try:
        return publish(args.private, args.public, args.require_complete, args.allow_not_started)
    except (ValidationError, KeyError, TypeError, IndexError, OSError, OverflowError) as error:
        # Do not echo source paths, malformed payloads, host errors or diagnostics.
        print("Transfer reporting validation failed (" + type(error).__name__ + "). No upload is authorized.", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Read-only, offline marginal-policy audit. No native replay parser or neural execution."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import struct
import sys


INPUT_SCHEMA = "alife-manipulation-probability-input-v1"
OUTPUT_SCHEMA = "alife-manipulation-probability-report-v1"
SOURCE_COMMIT = "1bf0ac6b4d877fdb5cb632b7440e359ba8f0d26a"
SOURCE_PATH = "crates/alife_gpu_backend/src/training_rollout.rs"
SOURCE_BLOB = "cbeb25761c092b22767f31ef4df0c381a6919535"
CHANNEL_NAMES = ["Locomotion", "Gaze", "Manipulation", "Vocalization", "Posture", "Reserved"]
KIND_SLOT = {"Move": 0, "Look": 1, "Interact": 2, "Write": 2,
             "Vocalize": 3, "Hold": 4, "Rest": 4, "Inspect": 4,
             "Idle": None, "Gesture": None}


class AuditError(ValueError):
    """A contract violation: produce no report rather than silently repair input."""


def integer(value, where, minimum=0, maximum=0xffffffff):
    if type(value) is not int or not minimum <= value <= maximum:
        raise AuditError(f"{where}: expected integer in [{minimum}, {maximum}]")
    return value


def text_label(value, where):
    if not isinstance(value, str) or not value.strip():
        raise AuditError(f"{where}: expected nonempty string")
    return value


def as_f32(value, where):
    if type(value) not in (int, float):
        raise AuditError(f"{where}: expected finite f32-representable number")
    try:
        rounded = struct.unpack("<f", struct.pack("<f", value))[0]
    except (OverflowError, struct.error):
        raise AuditError(f"{where}: outside finite f32 range") from None
    if not math.isfinite(rounded):
        raise AuditError(f"{where}: expected finite number; encode nonfinite logits using f32_bits")
    return rounded


def f32_bits(value):
    return struct.unpack("<I", struct.pack("<f", value))[0]


def parse_logit(raw, where):
    """Numbers are narrowed once; bit objects preserve NaN payloads and signed zero."""
    if isinstance(raw, dict):
        if set(raw) != {"f32_bits"}:
            raise AuditError(f"{where}: bit object must contain only f32_bits")
        bits = raw["f32_bits"]
        if isinstance(bits, str):
            if (len(bits) != 10 or not bits.startswith("0x")
                    or any(c not in "0123456789abcdefABCDEF" for c in bits[2:])):
                raise AuditError(f"{where}.f32_bits: expected 0x plus eight hex digits")
            try:
                bits = int(bits[2:], 16)
            except ValueError:
                raise AuditError(f"{where}.f32_bits: invalid hex digits") from None
        bits = integer(bits, f"{where}.f32_bits")
        return struct.unpack("<f", struct.pack("<I", bits))[0], bits
    value = as_f32(raw, where)
    return value, f32_bits(value)


def temperature(value, where):
    value = as_f32(value, where)
    # Rust compares two f32 values, including the 1.0e-4 literal.
    if value < as_f32(1.0e-4, "minimum temperature"):
        raise AuditError(f"{where}: temperature must be at least f32(1.0e-4)")
    return value


def stable_distribution(logits, mask, tau):
    """The receipt's f64 max/subtract/exp/sum/log math before final f32 cast.

    Keep sequential addition, as in Rust Iterator::sum, rather than math.fsum.
    An empty motor factor is a valid all-zero distribution, not a uniform one.
    """
    selected = [i for i in range(len(logits)) if mask & (1 << i)]
    probabilities = [0.0] * len(logits)
    log_probabilities = [None] * len(logits)
    if not selected:
        return probabilities, log_probabilities
    maximum = max(logits[i] for i in selected)
    scaled = [(logits[i] - maximum) / tau for i in selected]
    weights = [math.exp(value) for value in scaled]
    total = 0.0
    for weight in weights:
        total += weight
    normalizer = math.log(total)
    for i, z, weight in zip(selected, scaled, weights):
        probabilities[i] = weight / total
        log_probabilities[i] = z - normalizer
    return probabilities, log_probabilities


def receipt_f32_log_probability(value, where):
    # Source rejects log_probability results that become nonfinite after f32 cast.
    return as_f32(value, where)


def validate_row(row, row_number):
    where = f"rows[{row_number}]"
    if not isinstance(row, dict):
        raise AuditError(f"{where}: expected object")
    text_label(row.get("row_id"), f"{where}.row_id")
    if row.get("evidence_kind") not in ("synthetic", "exported_replay"):
        raise AuditError(f"{where}.evidence_kind: expected synthetic or exported_replay")
    if row.get("behavior") not in ("OnPolicy", "Demonstrator"):
        raise AuditError(f"{where}.behavior: expected OnPolicy or Demonstrator")
    sample_tau = temperature(row.get("sampling_temperature"), f"{where}.sampling_temperature")
    enabled = integer(row.get("enabled_channels"), f"{where}.enabled_channels", 1)
    raw_logits = row.get("logits")
    if not isinstance(raw_logits, list) or not 1 <= len(raw_logits) <= 32:
        raise AuditError(f"{where}.logits: expected 1 to 32 entries")
    size = len(raw_logits)
    parsed = [parse_logit(value, f"{where}.logits[{i}]") for i, value in enumerate(raw_logits)]
    logits = [value for value, _ in parsed]
    bits = [bits for _, bits in parsed]
    kinds = row.get("candidate_kinds")
    labels = row.get("candidate_labels")
    if not isinstance(kinds, list) or len(kinds) != size:
        raise AuditError(f"{where}.candidate_kinds: length must equal logits length")
    if not isinstance(labels, list) or len(labels) != size:
        raise AuditError(f"{where}.candidate_labels: length must equal logits length")
    for i, label in enumerate(labels):
        text_label(label, f"{where}.candidate_labels[{i}]")
    representative_mask = 0
    motor_masks = [0] * 6
    forced = []
    for i, (kind, value) in enumerate(zip(kinds, logits)):
        if not isinstance(kind, str) or kind not in KIND_SLOT:
            raise AuditError(f"{where}.candidate_kinds[{i}]: unknown ActionKind")
        slot = KIND_SLOT[kind]
        forced.append(4 if slot is None else slot)
        if not math.isfinite(value):
            continue
        representative_mask |= 1 << i
        if slot is not None and enabled & (1 << slot):
            motor_masks[slot] |= 1 << i
    if representative_mask == 0:
        raise AuditError(f"{where}: no finite representative candidate")
    supplied_rep = integer(row.get("representative_mask"), f"{where}.representative_mask")
    supplied_motors = row.get("motor_masks")
    if not isinstance(supplied_motors, list) or len(supplied_motors) != 6:
        raise AuditError(f"{where}.motor_masks: expected six u32 masks")
    for i, value in enumerate(supplied_motors):
        integer(value, f"{where}.motor_masks[{i}]")
    supplied_forced = row.get("forced_motor_slots")
    if not isinstance(supplied_forced, list) or len(supplied_forced) != size:
        raise AuditError(f"{where}.forced_motor_slots: length must equal logits length")
    for i, value in enumerate(supplied_forced):
        integer(value, f"{where}.forced_motor_slots[{i}]", maximum=5)
    if (supplied_rep, supplied_motors, supplied_forced) != (representative_mask, motor_masks, forced):
        raise AuditError(f"{where}: supplied masks/forcing disagree with current source semantics")
    queries = row.get("queries")
    if not isinstance(queries, list) or not queries:
        raise AuditError(f"{where}.queries: expected a nonempty list")
    query_labels = set()
    for q, query in enumerate(queries):
        qwhere = f"{where}.queries[{q}]"
        if not isinstance(query, dict):
            raise AuditError(f"{qwhere}: expected object")
        label = text_label(query.get("label"), f"{qwhere}.label")
        if label in query_labels:
            raise AuditError(f"{qwhere}: duplicate query label")
        query_labels.add(label)
        indices = query.get("candidate_indices")
        if not isinstance(indices, list):
            raise AuditError(f"{qwhere}.candidate_indices: expected list (empty means unavailable)")
        seen = set()
        for i in indices:
            integer(i, f"{qwhere}.candidate_indices", maximum=size - 1)
            if i in seen:
                raise AuditError(f"{qwhere}: duplicate candidate index")
            seen.add(i)
            if forced[i] != 2:
                raise AuditError(f"{qwhere}: candidate {i} is not a Manipulation action")
    selection = row.get("selection")
    if selection is not None:
        if not isinstance(selection, dict):
            raise AuditError(f"{where}.selection: expected object")
        rep = integer(selection.get("representative_index"), f"{where}.selection.representative_index",
                      maximum=size - 1)
        if not representative_mask & (1 << rep):
            raise AuditError(f"{where}.selection: representative is masked")
        motors = selection.get("motor_indices")
        if not isinstance(motors, list) or len(motors) != 6:
            raise AuditError(f"{where}.selection.motor_indices: expected six zero-based indices or null")
        for slot, index in enumerate(motors):
            if slot == forced[rep]:
                if type(index) is not int or index != rep:
                    raise AuditError(f"{where}.selection: forced slot must contain representative")
            elif motor_masks[slot] == 0:
                if index is not None:
                    raise AuditError(f"{where}.selection: empty independent channel must be null")
            else:
                integer(index, f"{where}.selection.motor_indices[{slot}]", maximum=size - 1)
                if not motor_masks[slot] & (1 << index):
                    raise AuditError(f"{where}.selection: motor candidate is masked")
    return logits, bits, sample_tau


def selected_likelihood(row, log_distributions, tau, sample_tau):
    selection = row.get("selection")
    if selection is None:
        return None
    rep = selection["representative_index"]
    forced = row["forced_motor_slots"][rep]
    factors = [receipt_f32_log_probability(log_distributions[0][rep], "representative log probability")]
    for slot, chosen in enumerate(selection["motor_indices"]):
        if slot == forced or row["motor_masks"][slot] == 0:
            factors.append(0.0)
        else:
            factors.append(receipt_f32_log_probability(log_distributions[slot + 1][chosen],
                                                      f"motor {slot} log probability"))
    joint = 0.0
    for factor in factors:
        joint = as_f32(joint + factor, "policy joint log probability")
    return {"factor_log_probabilities_f32": factors,
            "policy_joint_log_probability_f32": joint,
            "behavior_joint_log_probability_f32": joint if row["behavior"] == "OnPolicy" and tau == sample_tau else None}


def audit(document):
    if not isinstance(document, dict) or document.get("schema") != INPUT_SCHEMA:
        raise AuditError(f"schema must be {INPUT_SCHEMA}")
    if document.get("policy_source_commit") != SOURCE_COMMIT:
        raise AuditError(f"policy_source_commit must be the audited contract {SOURCE_COMMIT}")
    temperature_specs = document.get("temperatures")
    if not isinstance(temperature_specs, list) or not temperature_specs:
        raise AuditError("temperatures: expected nonempty list")
    temperatures = []
    labels = set()
    for i, spec in enumerate(temperature_specs):
        if not isinstance(spec, dict):
            raise AuditError(f"temperatures[{i}]: expected object")
        label = text_label(spec.get("label"), f"temperatures[{i}].label")
        if label in labels:
            raise AuditError("temperatures: duplicate label")
        labels.add(label)
        temperatures.append((spec, temperature(spec.get("value"), f"temperatures[{i}].value")))
    rows = document.get("rows")
    if not isinstance(rows, list) or not rows:
        raise AuditError("rows: expected nonempty list")
    reports = []
    row_ids = set()
    for number, row in enumerate(rows):
        logits, bits, sample_tau = validate_row(row, number)
        if row["row_id"] in row_ids:
            raise AuditError("rows: duplicate row_id")
        row_ids.add(row["row_id"])
        row_report = {"row_id": row["row_id"], "evidence_status":
                      "synthetic_fixture" if row["evidence_kind"] == "synthetic" else "caller_supplied_export_not_independently_verified",
                      "input": copy.deepcopy(row), "logit_bits": [f"0x{value:08x}" for value in bits],
                      "effective_sampling_temperature_f32": sample_tau,
                      "masked_nonfinite_candidate_indices": [i for i, v in enumerate(logits) if not math.isfinite(v)],
                      "at_temperatures": []}
        for spec, tau in temperatures:
            factors = [stable_distribution(logits, mask, tau) for mask in
                       [row["representative_mask"], *row["motor_masks"]]]
            distributions = [p for p, _ in factors]
            log_distributions = [p for _, p in factors]
            representative = distributions[0]
            manipulation = distributions[3]
            force_mass = [sum(representative[i] for i, slot in enumerate(row["forced_motor_slots"]) if slot == c)
                          for c in range(6)]
            # Sum the complement directly to avoid cancellation near probability one.
            other_mass = sum(representative[i] for i, slot in enumerate(row["forced_motor_slots"]) if slot != 2)
            candidate_marginals = [(representative[i] if row["forced_motor_slots"][i] == 2 else 0.0)
                                   + other_mass * manipulation[i] for i in range(len(logits))]
            queries = []
            for query in row["queries"]:
                indices = query["candidate_indices"]
                direct = sum(representative[i] for i in indices)
                independent = sum(manipulation[i] for i in indices)
                queries.append({"label": query["label"], "candidate_indices": list(indices),
                                "representative_forced_probability": direct,
                                "motor2_conditional_probability": independent,
                                "independent_contribution_probability": other_mass * independent,
                                "manipulation_policy_probability": direct + other_mass * independent})
            empty_probability = other_mass if row["motor_masks"][2] == 0 else 0.0
            row_report["at_temperatures"].append({
                "temperature": copy.deepcopy(spec), "effective_temperature_f32": tau,
                "interpretation": "collected_sampling_temperature" if tau == sample_tau else "counterfactual_fixed_logit_temperature",
                "representative_probabilities": representative,
                "independent_motor_probabilities": distributions[1:],
                "representative_force_slot_probabilities": force_mass,
                "representative_forces_other_than_manipulation_probability": other_mass,
                "manipulation_candidate_probabilities": candidate_marginals,
                "manipulation_empty_probability": empty_probability,
                "manipulation_probability_sum_including_empty": sum(candidate_marginals) + empty_probability,
                "queries": queries,
                "selected_likelihood": selected_likelihood(row, log_distributions, tau, sample_tau)})
        reports.append(row_report)
    return {"schema": OUTPUT_SCHEMA,
            "policy_contract": {"repository": "BobertNeek/A-Life", "commit": SOURCE_COMMIT,
                                "path": SOURCE_PATH, "git_blob_sha1": SOURCE_BLOB,
                                "url": f"https://github.com/BobertNeek/A-Life/blob/{SOURCE_COMMIT}/{SOURCE_PATH}"},
            "scope": "offline policy marginals from caller-supplied logits; no native replay parsing, neural execution, objective changes, or GPU/RNG bit-parity claim",
            "channel_names": CHANNEL_NAMES, "input_metadata": copy.deepcopy(document.get("metadata")),
            "rows": reports}


def reject_constant(value):
    raise AuditError(f"nonstandard JSON constant {value}; encode nonfinite logits with f32_bits")


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise AuditError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="explicit neutral JSON export; native replays are not supported")
    parser.add_argument("--output", type=Path, help="new report file; existing files are never overwritten")
    args = parser.parse_args(argv)
    try:
        raw = args.input.read_bytes()
        document = json.loads(raw.decode("utf-8"), parse_constant=reject_constant, object_pairs_hook=unique_object)
        report = audit(document)
        report["input_sha256"] = hashlib.sha256(raw).hexdigest()
        result = json.dumps(report, indent=2, allow_nan=False) + "\n"
        if args.output is None:
            sys.stdout.write(result)
        else:
            with args.output.open("x", encoding="utf-8", newline="\n") as handle:
                handle.write(result)
    except (AuditError, OSError, UnicodeError, ValueError) as error:
        print(f"audit error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

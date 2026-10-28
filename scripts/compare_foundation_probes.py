"""Compare native probe inputs under an explicitly selected assessment scope.

This read-only tool cannot qualify historical artifacts that lack the versioned
native identity. It never reinterprets old qualification receipts or runs a brain.
The fresh checkpoint scope also supports newly captured Feeding; historical
Feeding without that identity is not certified.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import re

from foundation_probe_schema import InvalidComparisonEvidence, number, uint, validate_sensory_context

POLICY_WEIGHTS_SCOPE = "subject_n2048_policy_weights_v1"
CHECKPOINT_SCOPE = "fresh_n2048_foundation_checkpoint_v1"
SCOPES = (POLICY_WEIGHTS_SCOPE, CHECKPOINT_SCOPE)


def required(record, key):
    if not isinstance(record, dict) or key not in record:
        raise InvalidComparisonEvidence(f"missing {key}")
    return record[key]


def sha256(value):
    return hashlib.sha256(value).hexdigest()


def policy_asset_identity(path):
    """Hash every asset byte and retain its digest trailer in bounded memory."""
    digest, trailer = hashlib.sha256(), b""
    with path.open("rb") as stream:
        while block := stream.read(65536):
            digest.update(block)
            # A short final/read fragment can complete a trailer begun earlier.
            trailer = block[-32:] if len(block) >= 32 else (trailer + block)[-32:]
    return digest.hexdigest(), trailer


def checked_sha(value, label):
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{64}", value):
        raise InvalidComparisonEvidence(f"invalid {label} SHA-256")
    return value


def canonical(value):
    try:
        return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()
    except (TypeError, ValueError) as error:
        raise InvalidComparisonEvidence("noncanonical/nonfinite comparison input") from error


def food_need_initialization(pilot, subject):
    """Fresh-assessment condition evidence; never infer a missing historical setup."""
    value = required(pilot, "food_need_initialization")
    if not isinstance(value, dict) or set(value) != {"condition", "organism", "before", "prepared"}:
        raise InvalidComparisonEvidence("missing/malformed food need initialization")
    if value["condition"] not in ("newborn_v1", "ordinary_healthy_aging_v1") or value["organism"] != subject or not uint(value["organism"], 64, positive=True):
        raise InvalidComparisonEvidence("unsupported food need initialization condition/subject")
    fields = {"tick", "energy", "hunger", "health", "injury", "pain", "sleeping"}
    for state in (value["before"], value["prepared"]):
        if not isinstance(state, dict) or set(state) != fields or not uint(state["tick"], 64) or type(state["sleeping"]) is not bool:
            raise InvalidComparisonEvidence("malformed food need initialization state")
        for key in fields - {"tick", "sleeping"}:
            number(state[key], "food need initialization " + key, 0, 1)
    before, prepared = value["before"], value["prepared"]
    if not 0 <= prepared["tick"] - before["tick"] <= 2400:
        raise InvalidComparisonEvidence("invalid food need setup duration")
    if value["condition"] == "newborn_v1":
        if before != prepared:
            raise InvalidComparisonEvidence("default food need initialization changed state")
    elif (prepared["sleeping"] or prepared["health"] < 0.95 or prepared["injury"] > 0.01
          or prepared["pain"] > 0.01 or not 0.2 < prepared["energy"] <= 0.9 or prepared["hunger"] < 0.12):
        raise InvalidComparisonEvidence("unhealthy or unmeasurable aged food need initialization")
    return value


def load_identity(directory, *, expected_policy_sha256, expected_executable_sha256,
                  assessment_scope=POLICY_WEIGHTS_SCOPE):
    directory = Path(directory)
    expected_policy_sha256 = checked_sha(expected_policy_sha256, "expected policy")
    expected_executable_sha256 = checked_sha(expected_executable_sha256, "expected executable")
    pilot = json.loads((directory / "pilot.json").read_text(encoding="utf-8"))
    if assessment_scope not in SCOPES:
        raise InvalidComparisonEvidence("unsupported assessment scope")
    longitudinal = assessment_scope == CHECKPOINT_SCOPE
    identity = required(pilot, "initial_checkpoint_assessment_identity" if longitudinal else "initial_comparison_identity")
    scope_field = "scope" if longitudinal else "exclusion"
    fields = {"schema_version", scope_field, "world", "policy_asset_digest", "policy_sha256", "executable_sha256"}
    if not isinstance(identity, dict) or set(identity) != fields:
        raise InvalidComparisonEvidence("missing/malformed native comparison identity")
    if type(identity["schema_version"]) is not int or identity["schema_version"] != 1 or identity[scope_field] != assessment_scope:
        raise InvalidComparisonEvidence("unsupported comparison identity/exclusion")
    world = identity["world"]
    if not isinstance(world, dict) or set(world) != {"schema_version", "subject", "words"}:
        raise InvalidComparisonEvidence("malformed comparison world signature")
    if type(world["schema_version"]) is not int or world["schema_version"] != 1 or type(world["subject"]) is not int or not 0 < world["subject"] < 2**64:
        raise InvalidComparisonEvidence("unsupported world signature/subject")
    if not isinstance(world["words"], list) or len(world["words"]) != 4 or any(type(w) is not int or not 0 <= w < 2**64 for w in world["words"]):
        raise InvalidComparisonEvidence("malformed world signature words")
    asset_sha256, asset_trailer = policy_asset_identity(directory / "initial.alife-foundation")
    if asset_sha256 != expected_policy_sha256 or checked_sha(identity["policy_sha256"], "recorded policy") != expected_policy_sha256:
        raise InvalidComparisonEvidence("policy asset does not match the predeclared treatment")
    digest = identity["policy_asset_digest"]
    if not isinstance(digest, list) or len(digest) != 32 or any(type(x) is not int or not 0 <= x <= 255 for x in digest) or len(asset_trailer) < 32 or bytes(digest) != asset_trailer:
        raise InvalidComparisonEvidence("policy digest is not bound to the canonical asset")
    if required(pilot, "source_asset_digest") != bytes(digest).hex():
        raise InvalidComparisonEvidence("pilot policy provenance differs from native identity")
    if checked_sha(identity["executable_sha256"], "recorded executable") != expected_executable_sha256:
        raise InvalidComparisonEvidence("executable provenance differs from the predeclared binary")
    if not isinstance(required(pilot, "initial_world_signature"), str) or not pilot["initial_world_signature"]:
        raise InvalidComparisonEvidence("missing canonical world identity for audit")
    trace_path = directory / "lesson-trace.json"
    if not trace_path.exists():
        trace_path = directory / "lesson-diagnostic.json"
    steps = required(json.loads(trace_path.read_text(encoding="utf-8")), "steps")
    if not isinstance(steps, list) or not steps:
        raise InvalidComparisonEvidence("missing first decision")
    first = steps[0]
    if required(first, "organism_id") != world["subject"]:
        raise InvalidComparisonEvidence("decision belongs to a different subject")
    sampling = required(pilot, "initial_sampling")
    if not isinstance(sampling, dict) or set(sampling) != {"seed", "counter", "temperature", "demonstrator"}:
        raise InvalidComparisonEvidence("missing/malformed sampler provenance")
    if required(first, "sampling") != sampling:
        raise InvalidComparisonEvidence("first decision sampler differs from admission")
    if any(type(sampling[k]) is not int or sampling[k] < 0 for k in ("seed", "counter")) or type(sampling["temperature"]) not in (int, float) or not math.isfinite(sampling["temperature"]) or sampling["temperature"] <= 0:
        raise InvalidComparisonEvidence("invalid sampler provenance")
    provenance_fields = ("seed", "founder_seed_base", "lesson", "teacher_mode", "initial_sampling", "silent_request", "request_target", "vocabulary_token", "teacher_cue_tokens")
    provenance = {key: required(pilot, key) for key in provenance_fields}
    if type(provenance["teacher_mode"]) is not bool:
        raise InvalidComparisonEvidence("malformed teacher provenance")
    if any(type(provenance[k]) is not int or provenance[k] <= 0 for k in ("seed", "founder_seed_base")):
        raise InvalidComparisonEvidence("malformed seed provenance")
    if longitudinal and provenance["lesson"] == "feeding":
        # Bind the explicit scene condition as well as actual native geometry.
        # Newborn and aged scenes are intentionally different inputs.
        setup = {key: required(pilot, key) for key in ("food_position", "initial_food_distance")}
        if not isinstance(setup["food_position"], list) or len(setup["food_position"]) != 2:
            raise InvalidComparisonEvidence("missing Feeding food position")
        for coordinate in setup["food_position"]:
            number(coordinate, "Feeding food position")
        number(setup["initial_food_distance"], "Feeding initial food distance", 0)
        setup["food_need_initialization"] = food_need_initialization(pilot, world["subject"])
    else:
        setup = {key: required(pilot, key) for key in ("grab_food_setup", "held_food_setup")}
        lesson_setup = {"grab_food": "grab_food_setup", "eat_held_food": "held_food_setup"}
        if provenance["lesson"] not in lesson_setup or not isinstance(setup[lesson_setup[provenance["lesson"]]], dict):
            raise InvalidComparisonEvidence("unsupported lesson or missing physical setup")
    input_fields = ("organism_id", "tick", "decision_tick", "sequence_id", "dispatch_generation", "frame_digest", "position", "gaze_rotation", "heard_tokens", "semantic_context", "object_slots", "terrain_ranges", "global_grip_before_decision", "lesson_target", "lesson_target_held_before_decision", "lesson_target_ownership_before_decision", "representative_mask", "motor_masks")
    output_fields = {"behavior", "channel_outcomes", "chosen_action", "chosen_family", "chosen_target", "experienced_valence", "frustration_delta", "lesson_target_ownership_after_outcome", "motor_indices", "outcome_tick", "physical", "prediction_error", "selected_commands", "success", "vocalized_token"}
    if not isinstance(first, dict) or set(first) - set(input_fields) - output_fields - {"sampling", "candidates", "physiology"}:
        raise InvalidComparisonEvidence("unsupported decision schema; unknown fields need an explicit contract")
    context = {key: required(first, key) for key in input_fields}
    for key in ("organism_id", "tick", "decision_tick", "sequence_id", "dispatch_generation"):
        if not uint(context[key], 64, positive=key in {"organism_id", "sequence_id", "dispatch_generation"}):
            raise InvalidComparisonEvidence(f"malformed decision identity {key}")
    frame_digest = context["frame_digest"]
    if not isinstance(frame_digest, list) or len(frame_digest) != 4 or not all(uint(word, 64) for word in frame_digest):
        raise InvalidComparisonEvidence("malformed sensory frame digest")
    candidates = required(first, "candidates")
    validate_sensory_context(context, candidates)
    if not uint(context["representative_mask"], 32, positive=True) or not isinstance(context["motor_masks"], list) or len(context["motor_masks"]) != 6 or not all(uint(m, 32) for m in context["motor_masks"]):
        raise InvalidComparisonEvidence("malformed candidate masks")
    # Scores are treatment outputs; all candidate identity/observation fields bind.
    context["candidates"] = [{k: v for k, v in c.items() if k != "logit"} for c in candidates]
    physiology = required(first, "physiology")
    physiology_fields = {"before_brain_atp", "before_energy", "before_fatigue", "before_health", "before_hunger", "before_injury", "before_pain"}
    if not isinstance(physiology, dict) or not physiology_fields <= physiology.keys():
        raise InvalidComparisonEvidence("missing pre-decision physiology")
    context["physiology"] = {k: v for k, v in physiology.items() if k.startswith("before_")}
    if set(context["physiology"]) != physiology_fields:
        raise InvalidComparisonEvidence("unsupported pre-decision physiology schema")
    for key, value in context["physiology"].items():
        number(value, "pre-decision physiology " + key, 0, 1)
    components = dict(world=world, provenance=provenance, setup=setup, context=context)
    if longitudinal:
        # Bind the available first prior input and provider configuration, not
        # later aggregate counters or treatment-dependent subsequent requests.
        prior = required(pilot, "semantic_prior")
        if prior is not None:
            provider_fields = ("backend", "context_contract", "source_provider_identity", "recorded_bank_digest",
                               "model", "provider_identity", "model_sha256")
            provider = {key: required(prior, key) for key in provider_fields}
            inputs = required(prior, "decision_inputs")
            if not isinstance(inputs, list) or not inputs:
                raise InvalidComparisonEvidence("missing first prior input")
            prior_input = inputs[0]
            if any(required(prior_input, prior_key) != context[context_key] for prior_key, context_key in
                   (("organism", "organism_id"), ("tick", "tick"), ("sequence", "sequence_id"),
                    ("dispatch_generation", "dispatch_generation"), ("frame_digest", "frame_digest"))):
                raise InvalidComparisonEvidence("first prior input differs from first decision")
            components["prior"] = dict(provider=provider, first_input=prior_input)
        elif context["semantic_context"] is not None:
            raise InvalidComparisonEvidence("semantic context lacks prior provenance")
        else:
            components["prior"] = None
    return dict(schema_version=1, components=components, comparison_sha256=sha256(canonical(components)),
                assessment_scope=assessment_scope, policy_asset_digest=digest,
                policy_sha256=expected_policy_sha256, executable_sha256=expected_executable_sha256,
                canonical_world_signature=pilot["initial_world_signature"])


def compare(control, candidate, *, control_policy_sha256, candidate_policy_sha256, executable_sha256,
            assessment_scope=POLICY_WEIGHTS_SCOPE):
    left = load_identity(control, expected_policy_sha256=control_policy_sha256, expected_executable_sha256=executable_sha256,
                         assessment_scope=assessment_scope)
    right = load_identity(candidate, expected_policy_sha256=candidate_policy_sha256, expected_executable_sha256=executable_sha256,
                          assessment_scope=assessment_scope)
    mismatches = [key for key in left["components"] if canonical(left["components"][key]) != canonical(right["components"][key])]
    return dict(schema_version=1, assessment_scope=assessment_scope, matched=not mismatches, mismatched_components=mismatches,
                control=left, candidate=right, neural_execution=False, historical_receipts_modified=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("control", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--control-policy-sha256", required=True)
    parser.add_argument("--candidate-policy-sha256", required=True)
    parser.add_argument("--executable-sha256", required=True)
    parser.add_argument("--assessment-scope", choices=SCOPES, default=POLICY_WEIGHTS_SCOPE,
                        help="fresh checkpoint scope requires its separate native identity; never inferred from older probes")
    args = parser.parse_args()
    try:
        result = compare(args.control, args.candidate, control_policy_sha256=args.control_policy_sha256,
                         candidate_policy_sha256=args.candidate_policy_sha256, executable_sha256=args.executable_sha256,
                         assessment_scope=args.assessment_scope)
    except (ValueError, OSError) as error:
        parser.error(str(error))
    print(json.dumps(result, sort_keys=True, allow_nan=False))
    return 0 if result["matched"] else 1


if __name__ == "__main__":
    raise SystemExit(main())

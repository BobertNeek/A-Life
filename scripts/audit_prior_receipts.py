#!/usr/bin/env python3
"""Read existing pilot receipts; report delivery/exposure without running a learner."""
import argparse
from collections import Counter
import json
import math
from pathlib import Path

FAILURE_POLICY = "log_and_continue_without_prior"
COUNT_KEYS = ("decision_frames", "decision_frames_with_prior", "decision_frames_without_prior")
PROVIDER_KEYS = ("backend", "provider_identity", "source_provider_identity", "model",
                 "model_sha256", "context_contract", "recorded_bank_digest")
STATUSES = {"delivered", "unavailable", "unsupported_profile", "deliberate_dropout", "pending",
            "provider_failure", "empty_hints", "developmental_gain_zero", "no_semantic_encoder"}
MAX_U64 = 2**64 - 1


def uint(value, maximum=MAX_U64):
    return type(value) is int and 0 <= value <= maximum


def count(record, key, warnings, source):
    value = record.get(key)
    if value is None or not uint(value):
        warnings.append(f"{source}.{key}: unknown or invalid")
        return None
    return value


def reject_constant(value):
    raise ValueError(f"non-JSON constant {value}")


def finite_float(text):
    value = float(text)
    if not math.isfinite(value):
        raise ValueError("nonfinite JSON number")
    return value


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key {key}")
        result[key] = value
    return result


def read_receipt(path, files, warnings):
    try:
        value = json.loads(path.read_text(encoding="utf-8"), parse_constant=reject_constant,
                           parse_float=finite_float, object_pairs_hook=unique_object)
        if not isinstance(value, dict):
            raise ValueError("expected JSON object")
    except FileNotFoundError:
        files[path.name] = "missing"
        warnings.append(f"{path.name}: missing; evidence unknown")
        return None
    except (OSError, UnicodeError, ValueError, RecursionError) as error:
        files[path.name] = "unreadable"
        warnings.append(f"{path.name}: unreadable ({error}); evidence unknown")
        return None
    files[path.name] = "present"
    return value


def audit_decisions(prior, totals, warnings):
    rows = prior.get("decision_inputs") if prior is not None else None
    result = {"observed_rows": None, "confirmed_delivered_encoded": None,
              "missing_or_failed": None, "unknown_rows": None, "status_counts": None,
              "coverage_complete": None}
    if not isinstance(rows, list):
        warnings.append("decision_inputs: missing or invalid; observed delivery unknown")
        return result
    identities = [tuple(row.get(key) for key in ("organism", "sequence"))
                  if isinstance(row, dict) and all(uint(row.get(key)) for key in
                                                   ("organism", "sequence")) and
                  row["organism"] > 0 and row["sequence"] > 0
                  else None for row in rows]
    occurrences = Counter(identity for identity in identities if identity is not None)
    delivered = missing = unknown = 0
    statuses = Counter()
    for index, (row, identity) in enumerate(zip(rows, identities)):
        if not isinstance(row, dict):
            unknown += 1
            warnings.append(f"decision_inputs[{index}]: invalid row")
            continue
        status = row.get("status")
        status_valid = isinstance(status, str) and status in STATUSES
        statuses[status if status_valid else "unknown"] += 1
        if identity is None or occurrences[identity] != 1 or not uint(row.get("tick")):
            unknown += 1
            warnings.append(f"decision_inputs[{index}]: invalid or duplicate decision identity")
        elif not status_valid:
            unknown += 1
            warnings.append(f"decision_inputs[{index}]: unknown status")
        elif status == "delivered":
            lanes = [row.get(key) for key in ("nonzero_prior_lanes", "nonzero_encoded_prior_lanes")]
            if all(uint(value) and value > 0 for value in lanes):
                delivered += 1
            else:
                unknown += 1
                warnings.append(f"decision_inputs[{index}]: delivered status lacks nonzero encoded lanes")
        else:
            lanes = [row.get(key) for key in ("nonzero_prior_lanes", "nonzero_encoded_prior_lanes")]
            if all(uint(value) and value > 0 for value in lanes):
                unknown += 1
                warnings.append(f"decision_inputs[{index}]: non-delivered status contradicts nonzero encoded lanes")
            else:
                missing += 1
    frames, with_prior, without_prior = (totals[key] for key in COUNT_KEYS)
    complete = (None not in (frames, with_prior, without_prior) and unknown == 0
                and frames == len(rows) and with_prior == delivered and without_prior == missing
                and with_prior + without_prior == frames)
    if frames is not None and frames != len(rows):
        warnings.append("decision_inputs: observed rows differ from total; native receipt retains at most 256 rows")
    if None not in (frames, with_prior, without_prior) and with_prior + without_prior != frames:
        warnings.append("semantic_prior: declared decision totals disagree")
    if frames == len(rows) and unknown == 0 and (with_prior != delivered or without_prior != missing):
        warnings.append("decision_inputs: observed delivery disagrees with declared totals")
    if missing:
        warnings.append("decision_inputs: missing or failed delivery is diagnostic; continue without prior")
    result.update(observed_rows=len(rows), confirmed_delivered_encoded=delivered,
                  missing_or_failed=missing, unknown_rows=unknown, status_counts=dict(sorted(statuses.items())),
                  coverage_complete=complete)
    return result


def audit_provider_events(prior, warnings):
    prior = prior or {}
    events = {}
    for key in ("failures", "dropout_frames", "prime_timeouts", "empty_hint_frames"):
        value = prior.get(key)
        events[key] = value if uint(value) else None
        if key in prior and not uint(value):
            warnings.append(f"semantic_prior.{key}: invalid; event count unknown")
        elif uint(value) and value > 0:
            warnings.append(f"semantic_prior.{key}: {value}; diagnostic, continue without prior")
    failures = prior.get("provider_failures")
    events["provider_failure_rows"] = len(failures) if isinstance(failures, list) else None
    error = prior.get("last_error")
    events["last_error"] = error if isinstance(error, str) else None
    if isinstance(failures, list) and failures:
        warnings.append("semantic_prior: provider failure receipts present")
    if isinstance(error, str) and error:
        warnings.append("semantic_prior: last provider error recorded")
    return events


def audit_cues(pilot, trace, warnings):
    cue_source = pilot if pilot is not None and "teacher_cue_tokens" in pilot else trace
    cue = cue_source.get("teacher_cue_tokens") if cue_source is not None else None
    valid_cue = (isinstance(cue, list) and 0 < len(cue) <= 16
                 and all(uint(token, 255) and token > 0 for token in cue))
    learners = []
    for receipt in (pilot, trace):
        for key in ("grab_food_setup", "held_food_setup"):
            setup = (receipt or {}).get(key)
            if isinstance(setup, dict):
                learner = setup.get("organism")
                if uint(learner) and learner > 0:
                    learners.append(learner)
                else:
                    warnings.append(f"{key}.organism: invalid; expected learner unknown")
    expected = learners[0] if learners and len(set(learners)) == 1 else None
    if expected is None:
        warnings.append("lesson setup: expected learner unknown or conflicting")
    result = {"configured_tokens": cue if valid_cue else None,
              "expected_learner": expected,
              "declared_any_token_frames": count(pilot or {}, "teacher_cue_frames", warnings, "pilot"),
              "trace_steps": None, "observed_full_ordered_cue_frames": None,
              "full_ordered_cue_frames": None, "coverage_complete": None, "evidence": [],
              "observed_generic_teacher_cue_frames": None, "generic_teacher_cue_frames": None,
              "generic_teacher_evidence": []}
    if not valid_cue:
        warnings.append("teacher_cue_tokens: missing or invalid; full ordered cue evidence unknown")
    if pilot is not None and trace is not None and "teacher_cue_tokens" in trace:
        if "teacher_cue_tokens" in pilot and pilot["teacher_cue_tokens"] != trace["teacher_cue_tokens"]:
            warnings.append("teacher_cue_tokens: pilot and lesson trace disagree")
            valid_cue = False
    steps = trace.get("steps") if trace is not None else None
    if not isinstance(steps, list):
        warnings.append("lesson trace steps: missing or invalid; heard cue evidence unknown")
        return result
    episode_frames = (pilot or {}).get("ticks")
    complete = valid_cue and uint(episode_frames) and episode_frames == len(steps)
    if not uint(episode_frames) or episode_frames != len(steps):
        warnings.append("lesson trace: episode decision coverage unknown or incomplete")
    ticks = Counter(step.get("tick") for step in steps
                    if isinstance(step, dict) and uint(step.get("tick")))
    matched_frames, generic_frames = set(), set()
    hearing_known = True
    for index, step in enumerate(steps):
        heard = step.get("heard_tokens") if isinstance(step, dict) else None
        if not isinstance(heard, list) or len(heard) != 16:
            complete = False
            warnings.append(f"steps[{index}].heard_tokens: missing or invalid native 16-slot array")
            continue
        if not uint(step.get("tick")) or ticks[step["tick"]] != 1:
            complete = False
            warnings.append(f"steps[{index}]: missing, invalid or duplicate decision tick")
        groups = {}
        bad_utterances = set()
        for token in heard:
            if token is None:
                continue
            if not isinstance(token, dict):
                complete = False
                warnings.append(f"steps[{index}].heard_tokens: invalid token")
                continue
            source = token.get("source_kind")
            utterance = token.get("utterance_id")
            if source not in ("Teacher", "Player", "Creature"):
                complete = False
                warnings.append(f"steps[{index}].heard_tokens: unknown source kind")
                if uint(utterance):
                    bad_utterances.add(utterance)
                continue
            if source != "Teacher":
                if uint(utterance):
                    bad_utterances.add(utterance)
                continue
            if (not uint(utterance) or utterance == 0 or not uint(token.get("sequence_position"), 15)
                    or not uint(token.get("token_id"), 255) or token["token_id"] == 0
                    or any(key not in token or (token[key] is not None and
                           (not uint(token[key]) or token[key] == 0))
                           for key in ("source_entity", "speaker_id", "addressee"))):
                complete = False
                warnings.append(f"steps[{index}].heard_tokens: invalid Teacher identity or token position")
                if uint(utterance):
                    bad_utterances.add(utterance)
                continue
            groups.setdefault(utterance, []).append(token)
        for utterance, tokens in groups.items():
            tokens.sort(key=lambda token: token["sequence_position"])
            positions = [token["sequence_position"] for token in tokens]
            identities = {tuple(token[key] for key in ("source_entity", "speaker_id", "addressee"))
                          for token in tokens}
            if utterance in bad_utterances:
                complete = False
                warnings.append(f"steps[{index}]: ambiguous Teacher utterance identity")
                continue
            if len(set(positions)) != len(positions) or len(identities) != 1:
                complete = False
                warnings.append(f"steps[{index}]: duplicate positions or mixed Teacher identity")
                continue
            if (valid_cue and utterance not in bad_utterances and
                    positions == list(range(len(cue))) and [token["token_id"] for token in tokens] == cue):
                generic_frames.add(index)
                channels = [token.get("teacher_channel") for token in tokens]
                known_channels = all(isinstance(channel, str) and channel in
                                     ("Hearing", "Vision", "Writing", "Gesture", "Object") for channel in channels)
                hearing = all(channel == "Hearing" for channel in channels) if known_channels else None
                addressee = tokens[0]["addressee"]
                directed = expected == addressee if expected is not None else None
                recipient = "broadcast" if addressee is None else (
                    "unknown_learner" if expected is None else "learner" if directed else "other_learner")
                if hearing is None:
                    hearing_known = False
                    warnings.append(f"steps[{index}]: Teacher channel missing or invalid; hearing unknown")
                if recipient != "learner":
                    warnings.append(f"steps[{index}]: Teacher cue recipient is {recipient}; learner-directed cue not established")
                if recipient == "unknown_learner":
                    hearing_known = False
                evidence = {"step": index, "tick": step.get("tick") if uint(step.get("tick")) else None,
                            "utterance_id": utterance, "tokens": cue, "sequence_positions": positions,
                            "teacher_channels": channels, "hearing_channel_confirmed": hearing,
                            "recipient": recipient, "learner_directed": directed if addressee is not None else False,
                            **{key: tokens[0][key] for key in ("source_entity", "speaker_id", "addressee")}}
                if hearing and recipient != "other_learner":
                    matched_frames.add(index)
                    result["evidence"].append(evidence)
                else:
                    result["generic_teacher_evidence"].append(evidence)
    result.update(trace_steps=len(steps), observed_full_ordered_cue_frames=len(matched_frames) if valid_cue else None,
                  full_ordered_cue_frames=len(matched_frames) if complete and hearing_known else None,
                  observed_generic_teacher_cue_frames=len(generic_frames) if valid_cue else None,
                  generic_teacher_cue_frames=len(generic_frames) if complete else None,
                  coverage_complete=complete and hearing_known)
    if valid_cue and not matched_frames:
        warnings.append("no full ordered same-utterance Teacher cue confirmed in observed trace")
    return result


def audit(path):
    directory = Path(path)
    if directory.name == "pilot.json":
        directory = directory.parent
    warnings, files = [], {}
    pilot = read_receipt(directory / "pilot.json", files, warnings)
    readiness = read_receipt(directory / "semantic-prior-readiness.json", files, warnings)
    trace_name = "lesson-trace.json"
    if not (directory / trace_name).exists() and (directory / "lesson-diagnostic.json").exists():
        trace_name = "lesson-diagnostic.json"
    trace = read_receipt(directory / trace_name, files, warnings)
    sources = [("pilot.json", pilot), ("semantic-prior-readiness.json", readiness), (trace_name, trace)]
    priors = [(name, value["semantic_prior"]) for name, value in sources
              if value is not None and isinstance(value.get("semantic_prior"), dict)]
    source, prior = priors[0] if priors else (None, None)
    if prior is None:
        warnings.append("semantic_prior: absent or invalid; provider and coverage unknown")
    for name, other in priors[1:]:
        if prior != other:
            warnings.append(f"semantic_prior: {source} and {name} disagree; reporting {source} without merging")
    totals = {key: count(prior or {}, key, warnings, "semantic_prior") for key in COUNT_KEYS}
    provider = {key: value if isinstance(value := (prior or {}).get(key), str) and value.strip() else None
                for key in PROVIDER_KEYS}
    if provider["backend"] is None or provider["provider_identity"] is None:
        warnings.append("semantic_prior: declared provider backend or identity unknown")
    if pilot is not None and uint(pilot.get("ticks")) and totals["decision_frames"] is not None:
        if pilot["ticks"] != totals["decision_frames"]:
            warnings.append("semantic_prior: decision_frames differs from pilot ticks")
    decisions = audit_decisions(prior, totals, warnings)
    events = audit_provider_events(prior, warnings)
    cues = audit_cues(pilot, trace, warnings)
    return {"directory": str(directory), "files": files, "semantic_prior_source": source,
            "declared_provider": provider, "provider_events": events,
            "decision_totals": totals, "decision_inputs": decisions,
            "teacher_cues": cues, "warnings": list(dict.fromkeys(warnings))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", type=Path, nargs="+", help="Existing probe directories or pilot.json paths")
    args = parser.parse_args()
    print(json.dumps({"scope": "read-only delivery and exposure diagnostics; no comprehension inference",
                      "failure_policy": FAILURE_POLICY, "audits": [audit(path) for path in args.paths]},
                     indent=2, allow_nan=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

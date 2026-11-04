#!/usr/bin/env python3
"""Read actual native ownership, independently of requested/selected primitives.

Usage: python scripts/audit_retained_language_physical_outcomes.py RUN_DIRECTORY
No simulation, relabelling of the training corpus or invented transition occurs.
"""
import collections
import io
import json
import pathlib
import sys

import zstandard


def audit(directory, arm):
    groups = {}
    source_path = directory / f"recovery-{arm}" / "reloaded-evaluation.jsonl.zst"
    with source_path.open("rb") as source:
        for line in io.TextIOWrapper(zstandard.ZstdDecompressor().stream_reader(source)):
            record = json.loads(line)
            if record.get("phase") != "factorial_evaluation":
                continue
            key = (record["layout"], record["recovery_sleep"], record["requested"])
            group = groups.setdefault(key, {"first_physical_events": None, "physical_counts": collections.Counter(), "acquisitions_without_explicit_grab": collections.Counter(), "last_owners": None, "consumption_ticks": [], "native_rows": 0})
            row = record["row"]
            organism = row["organism_id"]
            tick = row["patch"]["pre_action"]["tick"]
            channels = row["patch"]["decision"]["selected_bundle"]["channels"]
            explicit = {c["target"]["entity"] for c in channels if c["primitive"] == 211 and c.get("target") and c["target"].get("entity") is not None}
            events = []
            for i, activity in enumerate(["GetFood", "GetToy"]):
                if record["owned_before"][i] != organism and record["owned_after"][i] == organism:
                    events.append(activity)
                    entity = record["scene"]["food" if i == 0 else "toy"]
                    if entity not in explicit:
                        group["acquisitions_without_explicit_grab"][activity] += 1
            # Existing consumed event is sealed by the native ingestion receipt;
            # unlike ownership it cannot be inferred from a requested primitive.
            if record["completed"] and record["completed"]["lesson"] == "EatFood":
                events.append("EatFood")
                group["consumption_ticks"].append(tick)
            group["physical_counts"].update(events)
            if events and group["first_physical_events"] is None:
                group["first_physical_events"] = {"tick": tick, "events": events}
            group["last_owners"] = record["owned_after"]
            group["native_rows"] += 1
    trials = []
    for (layout, sleep, requested), g in groups.items():
        trials.append({"layout": layout, "recovery_sleep": sleep, "requested": requested, **g, "requested_physical_event_observed": None if requested is None else g["physical_counts"][requested] > 0})
    return {"trials": trials, "requested_trials": sum(t["requested"] is not None for t in trials), "eventual_requested_physical_event_trials": sum(t["requested_physical_event_observed"] is True for t in trials), "implicit_food_acquisitions": sum(t["acquisitions_without_explicit_grab"]["GetFood"] for t in trials), "implicit_toy_acquisitions": sum(t["acquisitions_without_explicit_grab"]["GetToy"] for t in trials), "requested_food_consumption_trials": sum(t["requested"] == "EatFood" and bool(t["consumption_ticks"]) for t in trials)}


def main():
    directory = pathlib.Path(sys.argv[1])
    print(json.dumps({"kind": "actual-native-physical-ownership-audit", "note": "Ownership changes count regardless of primitive. Same-tick acquisition and ingestion remain a set, without invented sub-tick ordering. Primary explicit-Grab scores and training labels remain preserved.", "arms": {a: audit(directory, a) for a in ["Grounded", "Shuffled", "NoSpeech"]}}, indent=2))


if __name__ == "__main__":
    main()

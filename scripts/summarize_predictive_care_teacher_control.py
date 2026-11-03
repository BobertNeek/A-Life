#!/usr/bin/env python3
"""Validate retained teacher-control histories and export all aggregate rows."""
import argparse
import csv
import json
import math
import struct
from pathlib import Path


def load(path):
    return json.loads(path.read_text())


def summarize(root):
    result = load(root / "result.json")
    assert result["experiment"] == "disposable-native-care-teacher-control-v1"
    assert not result["accepted_state_written"] and not result["trained_artifacts_published"]
    behaviors, updates = [], []
    for arm in result["arms"]:
        directory = root / arm["arm"]
        initial = load(directory / "initial-actor-only-recovery.json")
        assert initial["unchanged_source_critic"] == initial["source"]["value"]
        before = initial["actor"]
        assert before["optimizer_step"] == arm["initial_actor_step"]
        for case in range(2):
            for epoch in range(16):
                after = load(directory / f"lesson-{case}-epoch-{epoch}-actor.json")
                assert after["optimizer_step"] == before["optimizer_step"] + 1
                assert after["stage_mask"] == before["stage_mask"] and after["config"] == before["config"]
                for i, word in enumerate(before["stage_mask"]["words"]):
                    assert after["update_ages"][i] == before["update_ages"][i] + int(word == 1)
                    if word != 1:
                        for field in ["weights", "first_moment", "second_moment"]:
                            assert struct.pack("f", before[field][i]) == struct.pack("f", after[field][i])
                before = after
            complete = load(directory / f"after-lesson-{case}-actor-only-recovery.json")
            assert complete["unchanged_source_critic"] == initial["unchanged_source_critic"]
            for key in ["weights", "first_moment", "second_moment", "update_ages",
                        "stage_mask", "config", "optimizer_step"]:
                assert complete["actor"][key] == before[key]
        assert before["optimizer_step"] == arm["final_actor_step"]
        phases = [("train", arm["training"])] + [
            ("eval-" + p, [e for e in arm["evaluation"] if e["phase"] == p])
            for p in ["before", "after"]
        ]
        for phase, entries in phases:
            assert len(entries) == 2
            for case, entry in enumerate(entries):
                metric = entry["behavior"]
                traces = load(directory / f"{phase}-{case}" / "raw-care-trace.json")
                assert len(traces) == 65 and traces[-1]["bootstrap_row"]
                scored = traces[:64]
                assert all(not row["bootstrap_row"] for row in scored)
                if phase != "train":
                    assert metric["teacher_cue_frames_all_including_bootstrap"] == 0
                for key, field in [("nourished_meals", "nourished_meal"),
                                   ("acquisitions", "acquisition_event"),
                                   ("consumptions", "consumption_event")]:
                    assert metric[key] == sum(row[field] for row in scored)
                assert metric["failures"] == sum(not row["outcome"]["success"] for row in scored)
                selection = entry["collection_policy"] if phase == "train" else entry["selection"]
                contacts = {}
                bundled = 0
                for row in scored:
                    bundled += any(c["channel"] == "Manipulation" and c["primitive"] == 210
                                   and c["target"]["entity"] == row["lesson_target"]
                                   for c in row["bundle"]["channels"])
                    for channel in row["outcome"]["joint"]["channel_outcomes"]:
                        if channel["channel"] == "Manipulation":
                            contact = channel["physical"]["contact"]
                            contacts[contact] = contacts.get(contact, 0) + 1
                assert bundled == selection["ingest_bundled"]
                distance = sum(math.sqrt(sum(row["outcome"]["physical"]["displacement"][c] ** 2
                                            for c in ["x", "y", "z"])) for row in scored)
                meals = [i+1 for i, row in enumerate(scored) if row["nourished_meal"]]
                behaviors.append({
                    "arm": arm["arm"], "phase": phase, "seed": metric["seed"],
                    "lesson": metric["lesson"], "decisions": 64,
                    "nourished_meals": metric["nourished_meals"], "acquisitions": metric["acquisitions"],
                    "consumptions": metric["consumptions"], "successes": metric["successes"],
                    "failures": metric["failures"], "path_length_world_units": distance,
                    "first_nourished_decision_1_based": meals[0] if meals else "",
                    "ingest_candidate_rows": selection["ingest_candidate_rows"],
                    "ingest_probability_mean_when_candidate_present": selection["ingest_probability_mean"],
                    "ingest_probability_max": selection["ingest_probability_max"],
                    "ingest_representative_selections": selection["ingest_representative_selections"],
                    "ingest_manipulation_selections": selection["ingest_manipulation_selections"],
                    "ingest_food_target_commands_bundled": bundled,
                    "manipulation_contacts": json.dumps(contacts, sort_keys=True),
                    "energy_before": scored[0]["outcome"]["measured_physiology"]["before"]["body"]["energy"],
                    "energy_after": scored[-1]["outcome"]["measured_physiology"]["after"]["body"]["energy"],
                    "health_after": scored[-1]["outcome"]["measured_physiology"]["after"]["body"]["health"],
                    "teacher_cue_frames_including_bootstrap": metric["teacher_cue_frames_all_including_bootstrap"],
                })
        for lesson in arm["training"]:
            for point in lesson["teacher_updates"]:
                updates.append({"arm": arm["arm"], "lesson": lesson["behavior"]["lesson"],
                                "epoch": point["epoch"], **point["update"]})
    assert len(behaviors) == 12 and len(updates) == 64
    return behaviors, updates


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output-prefix", required=True, type=Path)
    args = parser.parse_args()
    for suffix, rows in zip(["-behaviors.csv", "-updates.csv"], summarize(args.input)):
        with Path(str(args.output_prefix) + suffix).open("x", newline="") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator="\n")
            writer.writeheader()
            writer.writerows(rows)
    print("Validated all 12 episodes and 64 native updates; exported complete tables.")


if __name__ == "__main__":
    main()

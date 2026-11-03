#!/usr/bin/env python3
"""Export every episode's raw behavior from a local disposable care trial."""
import argparse
import csv
import json
import math
import struct
from pathlib import Path


def verify_learner_history(root, arm):
    initial = json.loads((root / arm["arm"] / "initial-experimental-handoff.json").read_text())
    before = initial["actor"]
    assert before["optimizer_step"] == arm["initial_actor_step"]
    for i in range(4):
        handoff = json.loads((root / arm["arm"] / f"after-lesson-{i}-experimental-handoff.json").read_text())
        after = handoff["actor"]
        assert "head" not in handoff and "head" not in after
        assert after["config"] == before["config"]
        assert after["stage_mask"] == before["stage_mask"]
        assert after["optimizer_step"] == before["optimizer_step"] + 1
        assert after["weights"] != before["weights"]
        for j, word in enumerate(before["stage_mask"]["words"]):
            assert after["update_ages"][j] == before["update_ages"][j] + int(word == 1)
            if word != 1:
                for field in ["weights", "first_moment", "second_moment"]:
                    assert struct.pack("f", after[field][j]) == struct.pack("f", before[field][j])
        assert handoff["value"]["optimizer_step"] == i + 1
        before = after
    assert before["optimizer_step"] == arm["final_actor_step"]
    return initial["value"]


def summarize(root):
    result = json.loads((root / "result.json").read_text())
    plan = json.loads((root / "fixed-plan.json").read_text())
    assert (result["experiment"], plan["decisions"]) in [
        ("fixed-disposable-native-care-smoke-v1", 16),
        ("fixed-disposable-native-care-long-smoke-v1", 64),
    ]
    assert not result["accepted_state_written"]
    assert not result["trained_artifacts_published"]
    assert set(plan["train_seeds"]).isdisjoint(plan["eval_seeds"])
    assert {a["arm"] for a in result["arms"]} == {"untrained", "pretrained"}
    initial_critics = [verify_learner_history(root, arm) for arm in result["arms"]]
    assert initial_critics[0] == initial_critics[1]
    rows = []
    for arm in result["arms"]:
        phases = [("train", arm["training"])] + [
            ("eval-" + phase, [x for x in arm["evaluation"] if x["phase"] == phase])
            for phase in ["before", "after"]
        ]
        for phase, records in phases:
            assert len(records) == 4
            for i, entry in enumerate(records):
                metric = entry["behavior"]
                raw = json.loads(
                    (root / arm["arm"] / f"{phase}-{i}" / "raw-care-trace.json").read_text()
                )
                dose = plan["decisions"]
                assert len(raw) == dose + 1 and raw[-1]["bootstrap_row"]
                scored = raw[:dose]
                assert all(not row["bootstrap_row"] for row in scored)
                assert all(row["loss_row"] == (phase == "train") for row in scored)
                assert all(row["evaluation_row"] == (phase != "train") for row in scored)
                seeds = plan["train_seeds"] if phase == "train" else plan["eval_seeds"]
                assert metric["seed"] == seeds[i]
                assert metric["lesson"] == plan["schedule"][i]
                if phase != "train":
                    assert metric["teacher_cue_frames_all_including_bootstrap"] == 0
                for key, field in [("nourished_meals", "nourished_meal"),
                                   ("acquisitions", "acquisition_event"),
                                   ("consumptions", "consumption_event"),
                                   ("moved", "moved"), ("blocked", "blocked")]:
                    assert metric[key] == sum(row[field] for row in scored)
                successes = sum(row["outcome"]["success"] for row in scored)
                assert metric["successes"] == successes
                assert metric["failures"] == dose - successes
                actions = {}
                for row in scored:
                    key = str(row["action"]["action_id"])
                    actions[key] = actions.get(key, 0) + 1
                first = scored[0]["outcome"]["measured_physiology"]["before"]
                last = scored[-1]["outcome"]["measured_physiology"]["after"]
                distance = sum(math.sqrt(sum(
                    row["outcome"]["physical"]["displacement"][c] ** 2
                    for c in ["x", "y", "z"])) for row in scored)
                rows.append({
                    "arm": arm["arm"], "phase": phase, "seed": metric["seed"],
                    "lesson": metric["lesson"], "decisions": dose,
                    "meals": metric["nourished_meals"], "acquisitions": metric["acquisitions"],
                    "consumptions": metric["consumptions"], "moved": metric["moved"],
                    "blocked": metric["blocked"], "successes": successes, "failures": dose-successes,
                    "selected_lesson_target": sum(row["action"]["target_entity"] == row["lesson_target"] for row in scored),
                    "path_length_world_units": distance,
                    "energy_before": first["body"]["energy"], "energy_after": last["body"]["energy"],
                    "health_before": first["body"]["health"], "health_after": last["body"]["health"],
                    "actions": json.dumps(actions, sort_keys=True),
                    "teacher_cue_frames_all_including_bootstrap": metric["teacher_cue_frames_all_including_bootstrap"],
                })
                if plan["decisions"] == 64:
                    def held(row, boundary):
                        ownership = row["lesson_target_ownership_" + boundary]
                        return (ownership is not None and not ownership["consumed"]
                                and ownership["owner"] == row["organism"])
                    rows[-1].update({
                        "nonzero_displacement_outcomes": sum(
                            sum(row["outcome"]["physical"]["displacement"][c] ** 2
                                for c in ["x", "y", "z"]) > 1e-12 for row in scored),
                        "lesson_food_held_initially": held(scored[0], "before_decision"),
                        "lesson_food_held_at_scored_end": held(scored[-1], "after_outcome"),
                        "held_food_outcomes": sum(held(row, "after_outcome") for row in scored),
                    })
    assert len(rows) == 24
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    rows = summarize(args.input)
    with args.output.open("x", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    print(f"Validated and exported all {len(rows)} episodes; bootstrap rows excluded.")


if __name__ == "__main__":
    main()

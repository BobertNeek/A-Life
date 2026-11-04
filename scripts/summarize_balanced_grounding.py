#!/usr/bin/env python3
"""Audit real V15/V16 rows; report six seed units separately before/after sleep."""
import collections
import io
import json
import pathlib
import sys

import zstandard

WORDS = {"EatFood": [8, 1], "GetFood": [9, 1], "GetToy": [9, 2]}


def audit(path):
    trials = {}
    for line in io.TextIOWrapper(zstandard.ZstdDecompressor().stream_reader(path.open("rb"))):
        event = json.loads(line)
        if event.get("phase") != "balanced_readout_evaluation":
            continue
        row = event["row"]
        seed = event["world_seed"]
        key = (seed, event["post_sleep"], event["requested"])
        tick = row["patch"]["pre_action"]["tick"]
        t = trials.setdefault(key, {"case": event["case"], "first_tick": tick, "rows": 0, "food_acquisition_steps": [], "toy_acquisition_steps": [], "food_consumption_steps": [], "first_acquisition": None, "last_owners": None, "utterances": 0, "declarations": 0, "timely_factual_declarations": 0, "eligible_rows": 0, "token_sequences": collections.Counter()})
        step = tick - t["first_tick"]
        t["rows"] += 1
        organism = row["organism_id"]
        for i, kind in enumerate(["food", "toy"]):
            if event["owned_before"][i] != organism and event["owned_after"][i] == organism:
                t[kind + "_acquisition_steps"].append(step)
                if t["first_acquisition"] is None:
                    t["first_acquisition"] = kind
        joint = row["patch"]["outcome"].get("joint") or {}
        consumed = any(o["physical"]["target_entity"] == event["scene"]["food"] and o["physical"]["contact"] == "Consumed" for o in joint.get("channel_outcomes", []))
        assert consumed == event["actual_food_consumption"]
        if consumed:
            t["food_consumption_steps"].append(step)
        t["last_owners"] = event["owned_after"]
        target = event["grounded_speech_target"]
        if target is not None:
            assert step >= 8
            assert event["narration_evidence"] is not None
            assert WORDS[event["narration_evidence"]["lesson"]] == [target["token"], target["continuation"][0]]
            t["eligible_rows"] += 1
        bundle = row["patch"]["decision"]["selected_bundle"]
        payloads = [c["payload"]["values"] for c in bundle["channels"] if c["channel"] == "Vocal"] if bundle else []
        for u in event["emitted"]:
            assert u["source_kind"] == "Creature" and u["speaker_id"] == organism
            assert payloads and payloads[0][0] == 0x53485031 and u["tokens"] == payloads[0][4:]
            declare = payloads[0][1] == 0
            t["utterances"] += 1
            t["declarations"] += declare
            t["token_sequences"][" ".join(map(str, u["tokens"]))] += 1
            t["timely_factual_declarations"] += bool(declare and target and u["tokens"] == [target["token"], target["continuation"][0]])
    rows = [{"world_seed": k[0], "post_sleep": k[1], "requested": k[2], **v} for k, v in trials.items()]
    summaries = {}
    for post_sleep in [False, True]:
        phase = [t for t in rows if t["post_sleep"] == post_sleep]
        by_seed = collections.defaultdict(dict)
        for t in phase:
            by_seed[t["world_seed"]][t["requested"]] = t
        pairs = []
        for seed, g in by_seed.items():
            assert len(g) == 4
            eat, food, toy = g["EatFood"], g["GetFood"], g["GetToy"]
            full = bool(eat["food_consumption_steps"]) and bool(food["food_acquisition_steps"]) and not food["food_consumption_steps"]
            cue = any(x < 8 for x in eat["food_consumption_steps"]) and any(x < 8 for x in food["food_acquisition_steps"]) and not any(x < 8 for x in food["food_consumption_steps"])
            noun = food["first_acquisition"] == "food" and toy["first_acquisition"] == "toy"
            pairs.append({"world_seed": seed, "physical_verb_full_window": full, "physical_verb_cue_window": cue, "physical_noun_first_acquisition": noun, "physical_noun_cue_window": noun and bool(food["food_acquisition_steps"]) and food["food_acquisition_steps"][0] < 8 and bool(toy["toy_acquisition_steps"]) and toy["toy_acquisition_steps"][0] < 8, "eat_food_consumption_steps": eat["food_consumption_steps"], "get_food_consumption_steps": food["food_consumption_steps"], "get_food_acquisition_steps": food["food_acquisition_steps"], "get_toy_acquisition_steps": toy["toy_acquisition_steps"]})
        summaries["post_sleep" if post_sleep else "pre_sleep"] = {"independent_seed_units": len(by_seed), "requested_trials": len(by_seed) * 3, "verb_pairs_full_window": sum(p["physical_verb_full_window"] for p in pairs), "verb_pairs_cue_window": sum(p["physical_verb_cue_window"] for p in pairs), "noun_pairs_first_acquisition": sum(p["physical_noun_first_acquisition"] for p in pairs), "noun_pairs_cue_window": sum(p["physical_noun_cue_window"] for p in pairs), "actual_utterances": sum(t["utterances"] for t in phase), "timely_factual_declarations": sum(t["timely_factual_declarations"] for t in phase), "eligible_speech_rows": sum(t["eligible_rows"] for t in phase), "native_rows": sum(t["rows"] for t in phase), "by_seed": pairs}
    return {"kind": "independent-native-ownership-ingestion-speech-audit", "independent_N_counts_seeds_once": len({t["world_seed"] for t in rows}), "pre_post_not_independent_replicates": True, "summaries": summaries, "trials": rows}


if __name__ == "__main__":
    directory = pathlib.Path(sys.argv[1])
    names = sys.argv[2:] or ["Untreated", "Grounded", "Shuffled", "NoSpeech"]
    print(json.dumps({"arms": {name: audit(directory / f"recovery-{name}" / "reloaded-evaluation.jsonl.zst") for name in names}}, indent=2))

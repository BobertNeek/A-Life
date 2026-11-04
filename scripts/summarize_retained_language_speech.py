#!/usr/bin/env python3
"""Summarize actual world utterances; late-event matches are exploratory only.

Usage: python scripts/summarize_retained_language_speech.py /absolute/v8-run
Requires zstandard, as used by the native trace inspection workflow.
"""
import collections
import io
import json
import pathlib
import sys

import zstandard

WORDS = {"EatFood": [8, 1], "GetToy": [9, 2]}
MAGIC = 0x53485031


def summarize(directory, arm):
    path = directory / f"recovery-{arm}" / "reloaded-evaluation.jsonl.zst"
    counts = collections.Counter()
    sequences = collections.Counter()
    previous = {}
    latencies = []
    receipts = []
    with path.open("rb") as source:
        for line in io.TextIOWrapper(zstandard.ZstdDecompressor().stream_reader(source)):
            event = json.loads(line)
            if event.get("phase") != "curriculum_evaluation":
                continue
            key = (event["layout"], event["recovery_sleep"], event["requested"])
            row = event["row"]
            tick = row["patch"]["pre_action"]["tick"]
            target = event["grounded_speech_target"]
            eligible = target["token"] is not None
            counts["scored_rows"] += 1
            counts["eligible_rows"] += eligible
            counts["selected_vocalize"] += row["action"]["motor_candidates"][3] is not None
            vocal = [c["payload"]["values"] for c in row["patch"]["decision"]["selected_bundle"]["channels"] if c["channel"] == "Vocal"]
            payload = vocal[0] if vocal and vocal[0] else None
            for utterance in event["emitted"]:
                assert payload and payload[0] == MAGIC
                tokens = utterance["tokens"]
                assert tokens == payload[4:]
                declare = payload[1] == 0
                counts["actual_utterances"] += 1
                counts["declarations"] += declare
                counts["spoken_while_player_heard"] += bool(payload[2])
                counts["outside_primary_window"] += not eligible
                counts["strict_expected_token_match"] += eligible and tokens == [target["token"], target["continuation"][0]]
                counts["strict_expected_declaration"] += declare and eligible and tokens == [target["token"], target["continuation"][0]]
                sequences[" ".join(map(str, tokens))] += 1
                prior = previous.get(key)
                latest_match = prior is not None and tokens == WORDS[prior["goal"]]
                counts["tokens_match_preceding_latest_activity"] += latest_match
                counts["declaration_matches_preceding_latest_activity"] += declare and latest_match
                counts["utterances_without_preceding_completion"] += prior is None
                if latest_match:
                    latencies.append(tick - prior["tick"])
                receipts.append({"layout": key[0], "recovery_sleep": key[1], "requested": key[2], "tick": tick, "tokens": tokens, "speech_act_raw": payload[1], "player_heard": bool(payload[2]), "inside_primary_window": eligible, "preceding_latest_activity": prior, "matches_preceding_activity": latest_match})
            if event["completed"] is not None:
                previous[key] = {"goal": event["completed"]["goal"], "tick": tick}
    return {"counts": dict(counts), "emitted_token_sequences": dict(sequences), "preceding_activity_match_latency_ticks": latencies, "actual_utterance_receipts": receipts}


def main():
    directory = pathlib.Path(sys.argv[1])
    result = {"kind": "actual-world-speech-timing-summary", "primary_criterion_unchanged": True, "late_activity_matches_are_exploratory_not_request_comprehension": True, "method": "Count actual emitted world receipts and native Vocal payload act-kind; compare with the predefined three eligible narration rows and, separately, the latest earlier real completed activity in the same cue fork.", "arms": {}}
    for arm in ["Grounded", "Shuffled", "NoSpeech"]:
        result["arms"][arm] = summarize(directory, arm)
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()

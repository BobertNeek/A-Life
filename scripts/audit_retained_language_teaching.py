#!/usr/bin/env python3
"""Audit actual narration lag and repeated-Get labels in retained native traces."""
import collections
import hashlib
import io
import json
import pathlib
import sys

import zstandard


def main():
    path = pathlib.Path(sys.argv[1])
    arms = collections.defaultdict(lambda: {"rows": 0, "activities": collections.Counter(), "positive_lags": collections.defaultdict(collections.Counter), "get_labels_reversing_existing_ownership": []})
    with path.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    with path.open("rb") as source:
        for line in io.TextIOWrapper(zstandard.ZstdDecompressor().stream_reader(source)):
            row = json.loads(line)
            if row.get("phase") not in {"curriculum_training", "factorial_training"}:
                continue
            out = arms[row["arm"]]
            out["rows"] += 1
            event = row.get("completed")
            if event:
                out["activities"][event.get("goal", event.get("lesson"))] += 1
            target = row.get("speech_target")
            evidence = row.get("narration_evidence")
            if target and target["token"] is not None and evidence:
                activity = evidence.get("goal", evidence.get("lesson"))
                relation = "requested" if activity == row["requested"] else "nonrequested"
                out["positive_lags"][relation][row["row"]["patch"]["pre_action"]["tick"] - evidence["tick"]] += 1
            if row["requested"] not in {"GetFood", "GetToy"} or not isinstance(row.get("owned_before"), list):
                continue
            toy = row["requested"] == "GetToy"
            if row["owned_before"][int(toy)] != row["row"]["organism_id"]:
                continue
            mask = row["label"]["representative"]
            if mask is None or mask.bit_count() != 1:
                continue
            candidate = row["row"]["patch"]["pre_action"]["perception"]["base"]["candidates"][mask.bit_length() - 1]
            target = row["scene"]["toy" if toy else "food"]
            if candidate["action_id"] == 211 and candidate["target"]["entity"] == target:
                out["get_labels_reversing_existing_ownership"].append({"round": row["round"], "layout": row["layout"], "requested": row["requested"], "tick": row["row"]["patch"]["pre_action"]["tick"]})
    print(json.dumps({"kind": "captured-native-teaching-audit", "compressed_corpus_sha256": digest, "arms": dict(arms)}, indent=2))


if __name__ == "__main__":
    main()

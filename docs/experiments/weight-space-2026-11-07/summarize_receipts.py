#!/usr/bin/env python3
"""Summarize completed native CPU probe receipts. Standard library only."""
import collections
import hashlib
import json
import pathlib
import sys


def summarize(path):
    receipt = json.loads((path / "receipt.json").read_text())
    groups = collections.defaultdict(list)
    for trial in receipt["results"]:
        rows = trial["rows"]
        assert len(rows) == receipt["ticks"], "incomplete endpoint"
        assert trial["optimizer_unchanged"] and trial["cold_next_step_exact"]
        selected_ingest = sum(
            any(i is not None and r["candidates"][i]["family"] == "Ingest"
                for i in r["action"]["motor_candidates"])
            for r in rows
        )
        completed_ingest = sum(
            any(c["physical"]["contact"] == "Consumed"
                for c in r["outcome"]["joint"]["channel_outcomes"])
            for r in rows
        )
        first = rows[0]
        first_ingest = [first["logits"][i] for i, c in enumerate(first["candidates"])
                        if c["family"] == "Ingest"]
        first_tastes = [c["features"][15] for c in first["candidates"]
                        if c["target"]["entity"] is not None]
        groups[(trial["scene"], trial["arm"])].append({
            "seed": trial["seed"],
            "encoded_hunger_plus_0_1_logit_delta": first.get("encoded_hunger_plus_0_1_logit_delta"),
            "active_tiles": trial.get("active_tiles"),
            "candidate_digest": trial.get("candidate_digest"),
            "consumed": not trial["target_active"],
            "final_energy": trial["final_body"]["energy"],
            "final_injury": trial["final_body"]["injury"],
            "ingest_selected_ticks": selected_ingest,
            "ingest_executed_ticks": completed_ingest,
            "first_ingest_logits": first_ingest,
            "first_tastes": sorted(set(first_tastes)),
            "initial_ingest_motor_activation_sum": sum(
                m["activation"] for m in first["ingest_taste_motors"]),
        })
    result = []
    for (scene, arm), trials in sorted(groups.items()):
        result.append({
            "scene": scene, "arm": arm, "n": len(trials),
            "consumed": sum(t["consumed"] for t in trials),
            "mean_final_energy": sum(t["final_energy"] for t in trials) / len(trials),
            "mean_ingest_selected_ticks": sum(t["ingest_selected_ticks"] for t in trials) / len(trials),
            "executed_ingestions": sum(t["ingest_executed_ticks"] for t in trials),
            "max_final_injury": max(t["final_injury"] for t in trials),
            "trials": trials,
        })
    causal_residuals=[]
    if receipt["mode"]=="surgery":
        indexed={(t["seed"],t["scene"],t["arm"]):t for t in receipt["results"]}
        for (seed,scene,arm),trial in indexed.items():
            if arm=="untouched": continue
            baseline=indexed[(seed,scene,"untouched")]["rows"][0]
            candidate=trial["rows"][0]
            assert baseline["candidates"]==candidate["candidates"], "unmatched first frame"
            assert [m["activation"] for m in baseline["ingest_taste_motors"]]==[m["activation"] for m in candidate["ingest_taste_motors"]]
            delta=sum(b["activation"]*(c["weight"]-b["weight"]) for b,c in zip(baseline["ingest_taste_motors"],candidate["ingest_taste_motors"]))
            for i,c in enumerate(candidate["candidates"]):
                expected=delta*c["features"][15] if c["family"]=="Ingest" else 0.
                causal_residuals.append(abs(candidate["logits"][i]-baseline["logits"][i]-expected))
        assert max(causal_residuals,default=0.)<1e-3, "unexpected initial logit effect"
    return {
        "first_frame_weight_equation_max_residual":max(causal_residuals,default=None),
        "backend": receipt["backend"], "experimental": True, "promoted": False,
        "ticks": receipt["ticks"], "mode": receipt["mode"], "gain": receipt["gain"],
        "receipt_sha256": hashlib.sha256((path / "receipt.json").read_bytes()).hexdigest(),
        "groups": result,
    }


if __name__ == "__main__":
    # stdout only: callers choose a new artifact, never change original receipts.
    print(json.dumps(summarize(pathlib.Path(sys.argv[1])), indent=2, allow_nan=False))

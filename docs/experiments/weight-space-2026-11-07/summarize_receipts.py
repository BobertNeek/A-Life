#!/usr/bin/env python3
"""Summarize completed native CPU probe receipts. Standard library only."""
import collections
import hashlib
import json
import pathlib
import sys


def validate_coverage(receipt, seen_keys):
    assert seen_keys, "no completed trials"
    if receipt.get("receipt_format") == "journal-v2" and not receipt.get("aggregation_recovery"):
        assert "seed_count" in receipt and "arms" in receipt, "journal lacks declared coverage"
    # Old embedded receipts did not declare counts/arms. Check their balanced
    # observed matrix; journal-v2 uses the predeclared CLI counts and arms.
    arms = receipt.get("arms", sorted({k[2] for k in seen_keys}))
    scenes = receipt.get("scenes", sorted({k[1] for k in seen_keys}))
    seed_start = receipt.get("seed_start", min(k[0] for k in seen_keys))
    seed_count = receipt.get("seed_count", len({k[0] for k in seen_keys}))
    assert len(arms) == len(set(arms)), "duplicate arm"
    assert len(scenes) == len(set(scenes)), "duplicate scene"
    expected = {(seed, scene, arm) for seed in range(seed_start, seed_start + seed_count)
                for scene in scenes for arm in arms}
    assert seen_keys == expected, "incomplete or unexpected seed/scene/arm coverage"


def summarize(path):
    receipt_path = path if path.is_file() else path / "receipt.json"
    receipt = json.loads(receipt_path.read_text())
    first_frames = {}
    source_hashes = {}
    seen_keys = set()
    if "trial_files" in receipt:
        assert receipt["complete"]
        assert len(receipt["trial_files"]) == len(set(receipt["trial_files"])), "duplicate trial filename"
        def read_trials():
            for name in receipt["trial_files"]:
                assert pathlib.Path(name).name == name, "journal path must be local"
                raw = (receipt_path.parent / name).read_bytes()
                source_hashes[name] = hashlib.sha256(raw).hexdigest()
                yield json.loads(raw)
        trials_source = read_trials()
    else:
        trials_source = iter(receipt["results"])
    groups = collections.defaultdict(list)
    for trial in trials_source:
        key=(trial["seed"],trial["scene"],trial["arm"])
        assert key not in seen_keys, "duplicate trial identity"
        seen_keys.add(key)
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
        role = "need" if trial["arm"].endswith("no-need") else "tonic" if trial["arm"].endswith("no-tonic") else None
        lesion_indices={i for c in trial["edits"] if c.get("role")==role and role for i in c.get("bridge_synapses",[])}
        lesion_peak=max((abs(e["effective"]) for r in rows for e in r.get("effective_bridges",[]) if e["index"] in lesion_indices),default=None)
        first = rows[0]
        if receipt["mode"] == "surgery":
            first_frames[(trial["seed"],trial["scene"],trial["arm"])]=first
        first_ingest = [first["logits"][i] for i, c in enumerate(first["candidates"])
                        if c["family"] == "Ingest"]
        first_tastes = [c["features"][15] for c in first["candidates"]
                        if c["target"]["entity"] is not None]
        groups[(trial["scene"], trial["arm"])].append({
            "seed": trial["seed"],
            "initial_biological_development":trial.get("initial_biology",{}).get("development"),
            "initial_homeostasis":trial.get("initial_biology",{}).get("homeostasis"),
            "initial_body_energy":trial.get("initial_biology",{}).get("body",{}).get("energy"),
            "preparation_ticks":trial.get("preparation_ticks",0),
            "encoded_hunger_plus_0_1_logit_delta": first.get("encoded_hunger_plus_0_1_logit_delta"),
            "encoded_hunger_3_frozen_frames_delta":first.get("encoded_hunger_3_frozen_frames_delta"),
            "peak_effective_genetic_lesion_weight":lesion_peak,
            "final_position":trial.get("final_position"),
            "final_target_carrier":trial.get("final_target_carrier"),
            "mean_measured_weighted_work":sum(r["outcome"]["cognitive_work"]["weighted_total"] for r in rows)/len(rows),
            "active_tiles": trial.get("active_tiles"),
            "candidate_digest": trial.get("candidate_digest"),
            "consumed": not trial["target_active"],
            "final_energy": trial["final_body"]["energy"],
            "final_health":trial["final_body"]["health"],
            "final_hunger":trial.get("final_homeostasis",{}).get("drives",{}).get("hunger"),
            "final_pain":trial.get("final_homeostasis",{}).get("drives",{}).get("pain"),
            "peak_observed_pain_delta":max(r["outcome"]["pain_delta"] for r in rows),
            "final_injury": trial["final_body"]["injury"],
            "ingest_selected_ticks": selected_ingest,
            "ingest_executed_ticks": completed_ingest,
            "first_ingest_logits": first_ingest,
            "first_tastes": sorted(set(first_tastes)),
            "initial_ingest_motor_activation_sum": sum(
                m["activation"] for m in first["ingest_taste_motors"]),
        })
    validate_coverage(receipt, seen_keys)
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
        for (seed,scene,arm),candidate in first_frames.items():
            if arm=="untouched": continue
            baseline=first_frames[(seed,scene,"untouched")]
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
        "target_hazard_pain":receipt.get("target_hazard_pain",0),
        "preparation_ticks":receipt.get("preparation_ticks",0),
        "receipt_sha256": hashlib.sha256(receipt_path.read_bytes()).hexdigest(),
        "journal_trial_sha256":source_hashes,
        "aggregation_recovery":receipt.get("aggregation_recovery"),
        "groups": result,
    }


if __name__ == "__main__":
    # stdout only: callers choose a new artifact, never change original receipts.
    print(json.dumps(summarize(pathlib.Path(sys.argv[1])), indent=2, allow_nan=False))

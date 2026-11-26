"""Bind a ledger-registered R41 weight patch to stock saved-life A/B manifests.

This emits data, not a process launch or an official historical comparison report.
The one original sealed case is not part of the disclosed registry.
"""
from argparse import ArgumentParser
from hashlib import sha256
import json
import math
from pathlib import Path
import re
import struct
from time import time

DISCLOSED_CATALOG = "e842779cd7c573a6e8c45adcd100463d225601805a630230da681d6187fcfd9e"
ACTIVE_CARD = "8544f30571929b83198c206dd6a94ac97feac7db055784647f1014fae59e3129"
ACTIVE_CARD_ID = "qualia-disclosed-29-v1"


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False,
                      allow_nan=False).encode()


def need(condition, message):
    if not condition:
        raise ValueError(message)


def read(path):
    return json.loads(path.read_bytes())


def f32_word(value):
    need(type(value) in (int, float) and math.isfinite(value), "nonfinite weight")
    return struct.unpack("<I", struct.pack("<f", value))[0]


def frozen_request(state, card, case, deadline):
    need(sha256(card).hexdigest() == ACTIVE_CARD, "frozen active card bytes changed")
    cfg = json.loads(card)
    controls = state["saved_life_controls"]
    pinned = controls["active_card"]
    need(pinned["id"] == cfg["scorecard_id"] == ACTIVE_CARD_ID and pinned["sha256"] == ACTIVE_CARD,
         "frozen card is not authoritative in the live state")
    need(state["known_good_eval_bin_sha256"] == cfg["executable_sha256"], "runner mismatch")
    entry = cfg["cases"][case]
    scene = entry["scenario"]
    need(sha256(canonical(scene)).hexdigest() == entry["scenario_sha256"] and scene["case_id"] == case,
         "frozen scene authority changed")
    need(entry["decisions"] == scene["decision_limit"] and scene["semantic_prior_mode"] == "off",
         "frozen horizon or prior changed")
    source = controls["full_saved_life_current_sha256"]
    need(re.fullmatch("[0-9a-f]{64}", source) is not None, "missing mature Current digest")
    return entry, {"schema": 1, "owner": scene["starting_organism_id"], "source_current_sha256": source,
                   "scenario": "published_card_case", "card_case": case,
                   "evaluation_nonce": scene["evaluation_nonce"],
                   "max_decisions": scene["decision_limit"], "max_world_ticks": scene["world_tick_limit"],
                   "max_output_bytes": (384 if scene["decision_limit"] == 1024 else 256) * 1024**2,
                   "deadline_unix_seconds": deadline,
                   "finalization_deadline_unix_seconds": deadline + 90}


def prepare_baseline(state, card, case, deadline):
    controls = state["saved_life_controls"]
    need(controls["round_paused"] is False, "round paused")
    entry, common = frozen_request(state, card, case, deadline)
    return {"schema":"qualia.frozen_manifest_authority.v1", "scope": "fresh baseline control",
            "scorecard_id": ACTIVE_CARD_ID, "rubric_sha256": ACTIVE_CARD,
            "scenario_sha256": entry["scenario_sha256"],
            "evaluator_sha256": state["known_good_eval_bin_sha256"],
            "A": {**common, "run_id":f"active29-{state['active_base_id'].lower()}-control-{case}",
                  "intervention": {"kind":"control"}}}


def prepare(state, patch_path, catalog, case, nonce, deadline, card=None):
    controls = state["saved_life_controls"]
    need(controls["round_paused"] is False, "round paused")
    patch = read(patch_path)
    item = next((p for p in state["in_flight_patches"]
                 if p["status"] == "unscored"
                 and p["patch_id"] == patch["patch_id"]
                 and Path(p["patch_path"]).resolve() == patch_path.resolve()), None)
    need(item is not None and item["target_base_id"] == state["active_base_id"], "unregistered or stale patch")
    need(item.get("patch_sha256") == sha256(patch_path.read_bytes()).hexdigest(), "registered patch bytes drifted")
    need(patch["schema"] == "alife.brain_surgery.patch.v1" and patch["schema_version"] == 1, "wrong patch schema")
    need(patch["target_base_id"] == state["active_base_id"]
         and patch["target_foundation_digest"] == state["active_base_foundation_digest"]
         and patch["target_dendritic_branches_sha256"] == state["active_base_dendritic_sha256"], "wrong mature predecessor")
    need(patch.get("dendritic_edits") == {"op": "keep"}, "requires a separately qualified native V11 route")
    need(sha256(catalog).hexdigest() == DISCLOSED_CATALOG, "disclosed starting conditions changed")
    spec = json.loads(catalog)["read_only_29_disclosed_recipe_envelopes"][case]["original_case_native_request"]
    edits = []
    for word in patch["weight_edits"]:
        index = word["index"]
        need(type(index) is int and 0 <= index < 32768 and word["mode"] == "set", "not an exact typed weight edit")
        edits.append((index, f32_word(word["value"])))
    need(edits and len({i for i, _ in edits}) == len(edits), "empty or duplicate coordinates")
    edits.sort()
    source = controls["full_saved_life_current_sha256"]
    need(re.fullmatch("[0-9a-f]{64}", source) is not None, "missing mature Current digest")
    native_schema = patch.get("native_request_schema", 1)
    need(type(native_schema) is int and native_schema in (1, 2, 3), "unrecognized typed native subset")
    request = {"schema": native_schema, "source_current_sha256": source, "generator_id": patch["patch_id"], "edits": edits}
    common = {"schema": 1, "owner": 3, "source_current_sha256": source,
              "scenario": "published_card_case", "card_case": case,
              "evaluation_nonce": nonce, "max_decisions": spec["max_decisions"],
              "max_world_ticks": spec["max_world_ticks"],
              "max_output_bytes": (384 if spec["max_decisions"] == 1024 else 256) * 1024**2,
              "deadline_unix_seconds": deadline, "finalization_deadline_unix_seconds": deadline + 90}
    auth = {"scope": "disclosed diagnostic only; not a promotion score"}
    if card is not None:
        entry, common = frozen_request(state, card, case, deadline)
        need(nonce == common["evaluation_nonce"], "cannot override the shared frozen evaluation nonce")
        auth = {"schema": "qualia.frozen_manifest_authority.v1", "scope": "matched active-disclosed native case",
                "scorecard_id": ACTIVE_CARD_ID, "rubric_sha256": ACTIVE_CARD,
                "scenario_sha256": entry["scenario_sha256"]}
    label = sha256(f"{patch['patch_id']}:{case}".encode()).hexdigest()[:16]
    return {**auth,
            "evaluator_sha256": state["known_good_eval_bin_sha256"],
            "patch_sha256": sha256(patch_path.read_bytes()).hexdigest(),
            "native_request_canonical_sha256": sha256(canonical(request)).hexdigest(),
            "A": {**common, "run_id": "r41-" + label + "-A", "intervention": {"kind": "control"}},
            "B": {**common, "run_id": "r41-" + label + "-B",
                  "intervention": {"kind": "direct_weights", "edit": request}}}


def main():
    p = ArgumentParser(description=__doc__)
    p.add_argument("--state", type=Path, required=True)
    variant = p.add_mutually_exclusive_group(required=True)
    variant.add_argument("--patch", type=Path)
    variant.add_argument("--baseline", action="store_true")
    p.add_argument("--catalog", type=Path, required=True)
    p.add_argument("--card", type=Path)
    p.add_argument("--case", required=True)
    p.add_argument("--nonce", type=int, default=1794935016)
    args = p.parse_args()
    need(args.nonce > 0, "invalid nonce")
    card = args.card.read_bytes() if args.card else None
    state = read(args.state)
    deadline = int(time()) + 420
    if args.baseline:
        need(card is not None, "a frozen card is required for a baseline")
        payload = prepare_baseline(state, card, args.case, deadline)
    else:
        payload = prepare(state, args.patch, args.catalog.read_bytes(), args.case, args.nonce, deadline, card)
    print(json.dumps(payload, indent=2))


if __name__ == "__main__":
    main()

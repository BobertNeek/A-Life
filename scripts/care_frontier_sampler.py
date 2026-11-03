#!/usr/bin/env python3
"""Select bounded native care tasks from frozen, teacher/prior-free assessments.

This is an empirical curriculum sampler, not Dr. Zero/HRPO. It never runs or
updates a solver, changes rewards, reserves seeds, selects Current or promotes.
Only existing native lesson placements are admitted. Zero observed successes
mean unresolved/too hard, not a proof of physical impossibility.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import random
import os
import subprocess
import tempfile

from compare_cpu_care_rounds import read_json, raw_tree, file_pin, require

LESSONS = {"eat_held_food", "eat_reachable_food", "feeding", "relocated_food"}
REQUEST_FIELDS = ("previous", "output", "seed_history", "seed", "decisions", "temperature", "lesson",
                  "food_need_condition", "known_exclusions")


def native_receipt_digest(receipt):
    # Native save writes a serde_json::Value (sorted keys), while its digest
    # serializes the typed request in declaration order. Keep that exact order.
    # This adapter intentionally admits only temperature1 V1 care assessments.
    request = receipt["request"]
    require(set(request) in (set(REQUEST_FIELDS), set(REQUEST_FIELDS)-{"food_need_condition"}), "unknown/missing native request fields")
    require(request["temperature"] == 1.0, "V1 sampler requires native temperature1")
    ordered = {key: request[key] for key in REQUEST_FIELDS if key in request}
    ordered["temperature"] = 1.0
    def sorted_maps(value):
        if isinstance(value, dict):
            return {key: sorted_maps(value[key]) for key in sorted(value)}
        if isinstance(value, list):
            return [sorted_maps(child) for child in value]
        return value
    packet = [receipt["schema_version"], ordered, sorted_maps(receipt["input_sha256"]),
              sorted_maps(receipt["artifacts"]), sorted_maps(receipt["result"])]
    encoded = json.dumps(packet, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode()
    return list(hashlib.sha256(b"ALIFE-CPU-CARE-EVALUATION-V1" + encoded).digest())


def integer(value, lo, hi):
    return type(value) is int and lo <= value <= hi


def family_key(lesson, condition):
    return lesson, condition


def validate_bank(bank):
    require(set(bank) == {"schema", "decision_budget", "slots", "sampler_seed", "current_difficulty", "families", "pool", "holdouts"},
            "bank requires explicit bounded sampler fields only; reward overrides are unsupported")
    require(type(bank["schema"]) is int and bank["schema"] == 1 and integer(bank["decision_budget"], 1, 128)
            and integer(bank["slots"], 1, 32) and integer(bank["sampler_seed"], 0, 2**64 - 1), "invalid sampler budget")
    require(type(bank["families"]) is list and 1 <= len(bank["families"]) <= 16, "bounded families required")
    families, keys = {}, set()
    for family in bank["families"]:
        require(set(family) == {"id", "lesson", "food_need_condition", "placement", "difficulty"}, "invalid family fields")
        name = family["id"]
        require(isinstance(name, str) and name.isascii() and name.replace("_", "").isalnum() and len(name) <= 48
                and name not in families, "unique portable family ID required")
        lesson, condition = family["lesson"], family["food_need_condition"]
        require(lesson in LESSONS and condition in {"newborn_v1", "ordinary_healthy_aging_v1"}, "unsupported lesson/need condition")
        require(condition == "newborn_v1" or lesson in {"feeding", "relocated_food"}, "need preparation unsupported for lesson")
        require(family["placement"] == "native_seeded_v1", "unvalidated/custom/impossible placements rejected; native placements only")
        require(integer(family["difficulty"], 0, 7), "bounded prewritten difficulty required")
        key = family_key(lesson, condition)
        require(key not in keys, "ambiguous family recipe")
        keys.add(key)
        families[name] = family
    levels = {family["difficulty"] for family in families.values()}
    require(levels == set(range(max(levels)+1)) and bank["current_difficulty"] in levels
            and type(bank["current_difficulty"]) is int, "contiguous host difficulty ladder required")
    seeds = set()
    for kind in ("pool", "holdouts"):
        rows = bank[kind]
        require(type(rows) is list and 1 <= len(rows) <= 1024, "bounded nonempty task/holdout pools required")
        for task in rows:
            require(set(task) == {"family", "seed"} and task["family"] in families
                    and integer(task["seed"], 1, 2**64 - 1), "invalid native task")
            require(task["seed"] not in seeds, "task/holdout seeds must be globally disjoint")
            seeds.add(task["seed"])
    require(len(bank["pool"]) >= bank["slots"], "insufficient fixed-budget task pool")
    return families


def score_assessment(root):
    """Checksum/score a native-admitted archive; not a private recovery loader."""
    root = Path(root)
    receipt = read_json(root / "care-evaluation.json", 16 * 1024**2)
    require(receipt.get("schema_version") == 1, "unsupported native assessment")
    require(set(receipt) == {"schema_version", "request", "input_sha256", "artifacts", "result", "payload_sha256"}
            and receipt["payload_sha256"] == native_receipt_digest(receipt), "native assessment payload seal mismatch")
    # Match native evaluation.rs inventory keys, including on Windows. Keep
    # raw_tree's OS-local keys unchanged for its other before/after callers.
    inventory = {}
    for name, pin in raw_tree(root).items():
        native_name = name.replace("\\", "/")
        if native_name != "care-evaluation.json":
            require(native_name not in inventory, "ambiguous archive path after separator normalization")
            inventory[native_name] = pin["sha256"]
    require(receipt["artifacts"] == inventory, "native assessment archive changed/unsealed")
    request, result = receipt["request"], receipt["result"]
    require(result["input_sha256"] == receipt["input_sha256"], "native input binding mismatch")
    n = request["decisions"]
    require(integer(n, 1, 128) and integer(request["seed"], 1, 2**64 - 1), "invalid assessment dose/seed")
    require(result["mode"] == "native-cpu-care-assessment" and result["backend"] == "native-cpu"
            and result["lesson"] == request["lesson"] and result["seed"] == request["seed"]
            and result["evaluated_decisions"] == n and result["bootstrap_decisions"] == 1, "native request/result mismatch")
    require(result["teacher_present"] is False and result["teacher_cue_frames"] == 0
            and result["semantic_prior_enabled"] is False and result["offline_learning_disabled"] is True
            and result["offline_actor_optimizer_updates"] == result["offline_value_optimizer_updates"] == 0
            and result["actor_checkpoint_unchanged"] is True and result["value_checkpoint_unchanged"] is True
            and result["promotion"] == "none", "assessment must be silent/prior-free with frozen full offline heads")
    require(result["actor_checkpoint_before_sha256"] == result["actor_checkpoint_after_sha256"]
            and result["value_checkpoint_before_sha256"] == result["value_checkpoint_after_sha256"], "offline heads changed")
    trace = read_json(root / "care-trace.json", 64 * 1024**2)
    require(trace.get("complete") is not False and not trace.get("recovery_prefix", False), "incomplete recovery trace cannot score")
    rows = trace["rows"]
    require(trace["backend"] == "native-cpu" and trace["lesson"] == request["lesson"]
            and len(rows) == n + 1, "incomplete assessment; retain as censored, not a failure score")
    organisms, last_tick, nourished, survived = set(), None, 0, True
    for index, row in enumerate(rows):
        require(row["row"] == index and row["loss_row"] is False and row["teacher_heard"] is False
                and row["evaluation_row"] == (index < n) and row["bootstrap_row"] == (index >= n), "invalid loss/bootstrap/cue boundary")
        organisms.add(row["organism"])
        start, end = row["decision_tick"], row["outcome_tick"]
        require(integer(start, 0, 2**64-1) and end == start + 1 and (last_tick is None or start > last_tick), "invalid native outcome ticks")
        last_tick = start
        physiology = row["outcome"]["measured_physiology"]
        require(isinstance(physiology, dict), "measured native physiology required")
        before, after = physiology["before"]["body"], physiology["after"]["body"]
        for body in (before, after):
            require(all(type(body[key]) in (float, int) and math.isfinite(body[key]) and 0 <= body[key] <= 1
                        for key in ("energy", "health", "injury")), "malformed native body outcome")
        require(type(row["nourished_meal"]) is bool, "native nourishment event required")
        if index < n:
            survived &= after["health"] > 0
            nourished += int(row["nourished_meal"])
            if row["nourished_meal"]:
                require(after["energy"] > before["energy"] or physiology["after"]["homeostasis"]["drives"]["hunger"]
                        < physiology["before"]["homeostasis"]["drives"]["hunger"], "nourishment lacks measured benefit")
    require(len(organisms) == 1 and nourished == result["evaluated_nourished_meals"], "native subject/meal tally mismatch")
    return {"lesson": request["lesson"], "food_need_condition": request.get("food_need_condition", "newborn_v1"),
            "seed": request["seed"], "decisions": n, "temperature": request["temperature"],
            "actor_sha256": result["actor_checkpoint_before_sha256"],
            "value_sha256": result["value_checkpoint_before_sha256"], "source_sha256": result["source_sha256"],
            "policy_version": result["policy_version"], "nourished_meals": nourished,
            "survived_evaluated_window": bool(survived), "success": bool(nourished and survived),
            "receipt_sha256": hashlib.sha256((root / "care-evaluation.json").read_bytes()).hexdigest()}


def revalidate_completed(root, binary):
    """Use the existing native completed-output loader; no new live dispatch."""
    root = root.resolve(strict=True)
    before = raw_tree(root)
    receipt = read_json(root / "care-evaluation.json", 16 * 1024**2)
    require(Path(receipt["request"]["output"]).resolve(strict=True) == root, "native output request differs from selected archive")
    # Check the seal/shape first, and bind output to this existing completed root.
    score_assessment(root)
    pin = file_pin(binary)
    environment = {k: v for k, v in os.environ.items() if not k.startswith("ALIFE_SLM_")}
    environment.update(ALIFE_SLM_PRIOR="off", ALIFE_SLM_PRIOR_BACKEND="off")
    with tempfile.TemporaryDirectory(prefix="alife-frontier-native-verify-") as tmp:
        spec = Path(tmp) / "request.json"
        spec.write_text(json.dumps(receipt["request"], ensure_ascii=False), encoding="utf-8")
        completed = subprocess.run([str(binary), "evaluate", "--spec", str(spec)], stdin=subprocess.DEVNULL,
                                   capture_output=True, env=environment, timeout=60, check=False)
    require(completed.returncode == 0 and json.loads(completed.stdout) == receipt["result"], "native completed assessment validation failed")
    require(raw_tree(root) == before and file_pin(binary) == pin, "native read-only verification changed archive/executable")
    return score_assessment(root)


def adjust_difficulty(current, successes, trials, maximum):
    require(integer(current, 0, maximum) and integer(successes, 0, trials), "invalid batch difficulty evidence")
    if trials < 3:
        return current, "insufficient_batch_evidence"
    rate = successes / trials
    if rate >= .75:
        return min(maximum, current+1), "batch_success_raise_at_most_one"
    if rate <= .25:
        return max(0, current-1), "batch_failure_lower_at_most_one"
    return current, "batch_mixed_hold"


def select(bank, observations):
    families = validate_bank(bank)
    require(type(observations) is list and 1 <= len(observations) <= 1024, "bounded discovery evidence required")
    recipes = {family_key(f["lesson"], f["food_need_condition"]): name for name, f in families.items()}
    forbidden = {t["seed"] for kind in ("pool", "holdouts") for t in bank[kind]}
    seen, identity, grouped = set(), None, {name: [] for name in families}
    for row in observations:
        key = family_key(row["lesson"], row["food_need_condition"])
        require(key in recipes and row["seed"] not in seen and row["seed"] not in forbidden, "discovery/task/holdout overlap or unknown recipe")
        require(row["decisions"] == bank["decision_budget"] and row["temperature"] == 1.0, "discovery dose/sampling mismatch")
        source = (row["actor_sha256"], row["value_sha256"], row["policy_version"], row["source_sha256"])
        require(identity is None or identity == source, "discovery must bind one frozen source/full offline heads")
        identity = source
        seen.add(row["seed"])
        grouped[recipes[key]].append(row)
    stats, eligible = {}, []
    for name, rows in grouped.items():
        successes = sum(row["success"] for row in rows)
        rate = successes / len(rows) if rows else None
        status = ("insufficient_evidence" if len(rows) < 3 else "no_observed_success" if successes == 0
                  else "observed_trivial" if successes == len(rows) else "frontier")
        survival = sum(row["survived_evaluated_window"] for row in rows) / len(rows) if rows else None
        weight = 4 * rate * (1-rate) * survival if status == "frontier" else 0.0
        stats[name] = {"trials": len(rows), "successes": successes, "success_rate": rate,
                       "survival_rate": survival, "status": status, "sampling_weight": weight}
        if weight > 0:
            eligible.append(name)
    batch = [row for name, rows in grouped.items() if families[name]["difficulty"] == bank["current_difficulty"] for row in rows]
    target, reason = adjust_difficulty(bank["current_difficulty"], sum(row["success"] for row in batch),
                                       len(batch), max(f["difficulty"] for f in families.values()))
    # Baseline is predeclared balanced round-robin, independent of outcomes.
    queues = {name: [dict(t) for t in bank["pool"] if t["family"] == name] for name in families}
    baseline = []
    while len(baseline) < bank["slots"]:
        for name in sorted(queues):
            if queues[name] and len(baseline) < bank["slots"]:
                baseline.append(queues[name].pop(0))
    rng = random.Random(bank["sampler_seed"])
    candidates = [dict(t) for t in bank["pool"] if t["family"] in eligible and families[t["family"]]["difficulty"] == target]
    adaptive = []
    # Keep a declared easy/retention dose (25% rounded down), even if that
    # template is observed trivial. It never replaces solver reward arithmetic.
    retention = [dict(t) for t in bank["pool"] if families[t["family"]]["difficulty"] < target
                 and stats[t["family"]]["successes"] > 0]
    retention.sort(key=lambda t: (families[t["family"]]["difficulty"], t["seed"]))
    adaptive.extend(retention[:bank["slots"]//4])
    candidates = [t for t in candidates if t not in adaptive]
    while candidates and len(adaptive) < bank["slots"]:
        names = sorted({t["family"] for t in candidates})
        name = rng.choices(names, weights=[stats[name]["sampling_weight"] for name in names], k=1)[0]
        chosen = rng.choice([t for t in candidates if t["family"] == name])
        candidates.remove(chosen)
        adaptive.append(chosen)
    complete = len(adaptive) == bank["slots"]
    return {"schema": 1, "method": "empirical-native-care-frontier-v1", "solver_rewards_changed": False,
            "accepted_state_selected": False, "statistics": stats, "discovery": observations,
            "decision_budget": bank["decision_budget"], "slots": bank["slots"],
            "difficulty_before": bank["current_difficulty"], "difficulty_after": target,
            "difficulty_reason": reason, "retention_slots_requested": bank["slots"]//4,
            "retention_slots_filled": min(len(retention), bank["slots"]//4) if complete else 0,
            "requested_training_decisions_per_arm": bank["slots"] * bank["decision_budget"],
            "baseline": baseline, "adaptive": adaptive if complete else [], "holdouts": bank["holdouts"],
            "status": "ready" if complete else "no_sufficient_empirical_frontier",
            "scope": "discovery-only empirical selector; zero successes do not prove impossibility; no HRPO or capability claim"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bank", type=Path, required=True)
    parser.add_argument("--assessment", type=Path, action="append", required=True)
    parser.add_argument("--native-evaluator", type=Path, required=True,
                        help="explicit train_cpu_care_round binary; verifies existing completed outputs without dispatch")
    args = parser.parse_args()
    try:
        require(1 <= len(args.assessment) <= 1024, "bounded discovery evidence required before opening archives")
        bank = read_json(args.bank, 1024**2)
        validate_bank(bank)
        result = select(bank, [revalidate_completed(p, args.native_evaluator.resolve(strict=True)) for p in args.assessment])
    except (ValueError, KeyError, TypeError, OSError, subprocess.TimeoutExpired) as error:
        parser.error(str(error))
    print(json.dumps(result, sort_keys=True, allow_nan=False))
    return 0 if result["status"] == "ready" else 2


if __name__ == "__main__":
    raise SystemExit(main())

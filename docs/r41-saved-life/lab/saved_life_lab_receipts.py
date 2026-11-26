#!/usr/bin/env python3
"""Translate pinned independently accepted saved-life evidence; never run or score a simulation."""
import argparse
from datetime import datetime
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import re


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def digest(value):
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value), "expected full SHA-256")
    return value


def load(path, sha):
    raw = Path(path).read_bytes()
    require(hashlib.sha256(raw).hexdigest() == digest(sha), f"evidence digest mismatch: {path}")
    return json.loads(raw)


def validate_identity(identity):
    require(isinstance(identity["base_id"], str) and identity["base_id"], "missing base_id")
    for key in ("foundation_digest", "dendritic_branches_sha256", "source_current_sha256"):
        digest(identity[key])


def integer(n, name):
    require(type(n) is int and n > 0, f"{name} must be a positive measured integer")
    return n


def reopen(e, i):
    validate_identity(i)
    require(e["schema"] == "qualia.accepted_saved_life_reopen.v1", "wrong trusted evidence schema")
    for key in ("base_id", "foundation_digest", "dendritic_branches_sha256", "source_current_sha256"):
        require(e[key] == i[key], f"wrong reopened {key}")
    require(digest(e["executable_sha256"]) == digest(i["executable_sha256"]), "evaluator digest mismatch")
    digest(e["receipt_sha256"])
    for key in ("gpu_restored", "same_individual", "private_world_memory_neural_and_sampling_preserved",
                "source_unchanged", "independently_accepted", "zero_cut"):
        require(e[key] is True, f"missing accepted saved-life {key}")
    start = integer(e["tick_start"], "tick_start")
    require(type(e["tick_end"]) is int and e["tick_end"] == start, "zero-cut reopen must be stationary")
    require(type(i["dendritic_branch_count"]) is int and i["dendritic_branch_count"] >= 0
            and e["dendritic_branch_count"] == i["dendritic_branch_count"], "branch count mismatch")
    parent = integer(e["parent_pid"], "recorded parent_pid")
    child = integer(e["child_pid"], "recorded child_pid")
    require(parent != child and type(e["child_exit"]) is int and e["child_exit"] == 0,
            "missing successful separate-process closure")
    when = datetime.fromisoformat(e["finished_at"])
    require(when.utcoffset() is not None, "finished_at requires timezone")
    timestamp = integer(int(when.timestamp()), "finished_at")
    mode = e["semantic_prior_mode"]
    require(isinstance(mode, str) and mode and mode == i["semantic_prior_mode"], "semantic prior mode mismatch")
    return dict(schema="alife.brain_surgery.reopen_receipt.v1", schema_version=1,
                base_id=i["base_id"], foundation_digest=i["foundation_digest"],
                dendritic_branches_sha256=i["dendritic_branches_sha256"],
                dendritic_branch_count=i["dendritic_branch_count"], tick=start,
                parent_pid=parent, child_pid=child, verified=True, semantic_prior_mode=mode,
                verified_at_unix=timestamp)


def numeric(x, name):
    require(type(x) in (int, float) and math.isfinite(x), f"{name} must be a finite number")
    return x


def comparison(e, cfg):
    require(e["schema"] == "qualia.accepted_saved_life_scorecard.v1", "wrong trusted evidence schema")
    require(e["full_score"] is True and e["independently_accepted"] is True, "incomplete or unaccepted card")
    require(e["scorecard_id"] == cfg["scorecard_id"], "scorecard ID mismatch")
    require(digest(e["rubric_sha256"]) == digest(cfg["rubric_sha256"]), "rubric digest mismatch")
    require(digest(e["executable_sha256"]) == digest(cfg["executable_sha256"]), "evaluator digest mismatch")
    require(isinstance(cfg["candidate_patch_id"], str) and cfg["candidate_patch_id"]
            and e["candidate_patch_id"] == cfg["candidate_patch_id"], "patch ID mismatch")
    for arm in ("base", "candidate"):
        identity = cfg[arm]
        validate_identity(identity)
        for key in ("base_id", "foundation_digest", "dendritic_branches_sha256", "source_current_sha256"):
            require(e[arm][key] == identity[key], f"wrong {arm} {key}")
    expected = cfg["cases"]
    actual = e["cases"]
    require(isinstance(expected, dict) and expected and isinstance(actual, dict)
            and actual.keys() == expected.keys(), "missing, extra or unpinned scorecard cases")
    results = []
    for case, spec in expected.items():
        row = actual[case]
        decisions = integer(spec["decisions"], "decisions")
        require(digest(row["scenario_sha256"]) == digest(spec["scenario_sha256"]), f"scenario mismatch: {case}")
        for flag in ("matched", "real_game_inputs", "same_disposable_start"):
            require(row[flag] is True, f"no {flag}: {case}")
        scores = []
        completed = []
        for arm in ("base", "candidate"):
            native = row[arm]
            digest(native["native_receipt_sha256"])
            digest(native["rows_sha256"])
            require(native["native_valid"] is True and native["gpu_restored"] is True
                    and integer(native["decisions"], "native decisions") == decisions
                    and native["source_current_sha256"] == cfg[arm]["source_current_sha256"],
                    f"invalid or unmatched native {arm} evidence: {case}")
            score = numeric(native["score"], "score")
            require(0 <= score <= 100, "score must be on the documented 0–100 scale")
            require(type(native["completed"]) is bool, "missing measured completion")
            scores.append(score)
            completed.append(native["completed"])
        weight = numeric(spec["weight"], "case weight")
        require(weight > 0, "case weights must be positive")
        results.append(dict(case_id=case, lesson=spec["lesson"], weight=weight, base_score=scores[0],
                            candidate_score=scores[1], delta=scores[1] - scores[0],
                            base_completed=completed[0], candidate_completed=completed[1], error=None))
    weight = math.fsum(r["weight"] for r in results)
    base, candidate = (math.fsum(r[key] * r["weight"] for r in results) / weight
                       for key in ("base_score", "candidate_score"))
    epsilon = 1e-9
    improved = sum(r["delta"] > epsilon for r in results)
    regressed = [r["case_id"] for r in results if r["delta"] < -epsilon]
    threshold = numeric(cfg["min_improvement"], "min improvement")
    require(threshold >= 0, "min improvement must be nonnegative")
    return dict(schema="alife.brain_surgery.comparison_report.v1", schema_version=1,
                scorecard_id=cfg["scorecard_id"], base_id=cfg["base"]["base_id"],
                candidate_id=cfg["candidate"]["base_id"], candidate_patch_id=cfg["candidate_patch_id"],
                base_foundation_digest=cfg["base"]["foundation_digest"],
                candidate_foundation_digest=cfg["candidate"]["foundation_digest"],
                base_dendritic_sha256=cfg["base"]["dendritic_branches_sha256"],
                candidate_dendritic_sha256=cfg["candidate"]["dendritic_branches_sha256"],
                base_average=base, candidate_average=candidate, delta=candidate-base,
                min_improvement=threshold, improved_cases=improved, regressed_cases=regressed,
                unchanged_cases=len(results)-improved-len(regressed), failed_cases=0,
                promoted=candidate-base > threshold+epsilon, results=results,
                saved_life_evidence={"full_score": True,
                    "rubric_sha256": cfg["rubric_sha256"],
                    "evaluator_sha256": cfg["executable_sha256"],
                    "source_current_sha256": cfg["base"]["source_current_sha256"],
                    "case_count": len(results)})


def active_comparison(e, card, card_sha):
    """Convert independently checked native measurements on the active frozen card."""
    require(e.get("schema") == "qualia.accepted_saved_life_active_card.v1",
            "wrong active native evidence schema")
    require(card.get("schema") == "qualia.saved_life_card.v1" and card["scorecard_id"] == e["scorecard_id"],
            "wrong active card ID")
    require(e["full_score"] is True and e["independently_accepted"] is True,
            "active card has not been completely and independently measured")
    require(digest(e["rubric_sha256"]) == digest(card_sha) and
            digest(e["executable_sha256"]) == digest(card["executable_sha256"]), "wrong active rubric/runner")
    patch_id = e["candidate_patch_id"]
    require(isinstance(patch_id, str) and patch_id, "missing bound patch ID")
    patch_sha = digest(e["patch_sha256"])
    ids = {arm: e[arm] for arm in ("base", "candidate")}
    for identity in ids.values():
        validate_identity(identity)
    source = ids["base"]["source_current_sha256"]
    calibrated = ids["candidate"]["source_current_sha256"]
    require(source != calibrated, "no actual mature candidate Current")
    require(e["cases"].keys() == card["cases"].keys(), "not all exact active cases")
    total_possible = Fraction(0)
    results = []
    earned_total = {"base": Fraction(0), "candidate": Fraction(0)}
    totals_per_domain = {}
    for cid, spec in card["cases"].items():
        pair = e["cases"][cid]
        require(all(pair.get(k) is True for k in ("matched", "real_game_inputs", "same_disposable_start")),
                f"unmatched/invalid exact native case {cid}")
        recorded = {}
        size = integer(spec["decisions"], "decisions")
        width = Fraction(*spec["weight_rational"])
        require(width >= 0 and numeric(spec["weight"], "case weight") == float(width),
                f"invalid card weight {cid}")
        require(width == sum((Fraction(*weight) for weight in spec["atoms"].values()), Fraction(0)),
                f"atom and case weights differ {cid}")
        total_possible += width
        domain = spec["lesson"]
        totals_per_domain[domain] = totals_per_domain.get(domain, Fraction(0)) + width
        scenario = digest(spec["scenario_sha256"])
        for arm in ("base", "candidate"):
            row = pair[arm]
            require(row["schema"] == "qualia.disclosed_native_case_evidence.v1" and row["case_id"] == cid
                    and row["independently_accepted"] is True and row["real_game_inputs"] is True, f"off-protocol {cid}/{arm}")
            require(row["scorecard_id"] == card["scorecard_id"] and row["rubric_sha256"] == card_sha and
                    row["evaluator_sha256"] == e["executable_sha256"] and row["scenario_sha256"] == scenario and
                    row["decisions"] == size, f"wrong frozen contract {cid}/{arm}")
            require(row["source_current_sha256"] == source and
                    row["effective_current_sha256"] == ids[arm]["source_current_sha256"] and
                    row["intervention"] == ("control" if arm == "base" else "direct_weights"),
                    f"wrong physical starting identity {cid}/{arm}")
            words = row["score_atoms"]
            require(words.keys() == spec["atoms"].keys() and all(type(v) is bool for v in words.values()),
                    f"missing exact observed atoms {cid}/{arm}")
            earned = sum((Fraction(*spec["atoms"][key]) for key, value in words.items() if value), Fraction(0))
            score = float(100 * earned / width) if width else 0.0
            require(Fraction(*row["earned_rational"]) == earned and numeric(row["score"], "case score") == score,
                    f"native atoms and asserted score disagree {cid}/{arm}")
            earned_total[arm] += earned
            recorded[arm] = (row, score)
        a, a_score = recorded["base"]
        b, b_score = recorded["candidate"]
        require(digest(a["opening_cut_sha256"]) == digest(b["opening_cut_sha256"]),
                f"actual initial cut mismatch {cid}")
        results.append(dict(case_id=cid, lesson=domain, weight=float(width), scenario_sha256=scenario,
                            decisions=size, base_score=a_score, candidate_score=b_score, delta=b_score-a_score,
                            base_completed=True, candidate_completed=True, error=None,
                            base_native_receipt_sha256=digest(a["native_receipt_sha256"]),
                            candidate_native_receipt_sha256=digest(b["native_receipt_sha256"]),
                            base_rows_sha256=digest(a["rows_sha256"]), candidate_rows_sha256=digest(b["rows_sha256"])))
    require(total_possible == 100 and totals_per_domain == card["domain_weights"] and
            len(totals_per_domain) == 5 and set(totals_per_domain.values()) == {Fraction(20)},
            "not the frozen equal-domain card")
    base, candidate = (float(earned_total[arm]) for arm in ("base", "candidate"))
    epsilon = 1e-9
    threshold = numeric(card["min_improvement"], "minimum improvement")
    require(threshold >= 0, "negative promotion threshold")
    improved = sum(row["delta"] > epsilon for row in results)
    regressed = [row["case_id"] for row in results if row["delta"] < -epsilon]
    return dict(schema="alife.brain_surgery.comparison_report.v1", schema_version=1,
                scorecard_id=card["scorecard_id"], base_id=ids["base"]["base_id"],
                candidate_id=ids["candidate"]["base_id"], candidate_patch_id=patch_id,
                base_foundation_digest=ids["base"]["foundation_digest"],
                candidate_foundation_digest=ids["candidate"]["foundation_digest"],
                base_dendritic_sha256=ids["base"]["dendritic_branches_sha256"],
                candidate_dendritic_sha256=ids["candidate"]["dendritic_branches_sha256"],
                base_average=base, candidate_average=candidate,
                delta=float(earned_total["candidate"]-earned_total["base"]), min_improvement=threshold,
                improved_cases=improved, regressed_cases=regressed,
                unchanged_cases=len(results)-improved-len(regressed), failed_cases=0,
                promoted=float(earned_total["candidate"]-earned_total["base"]) > threshold + epsilon,
                results=results, saved_life_evidence={
                    "full_score": True, "independently_accepted": True, "rubric_sha256": card_sha,
                    "evaluator_sha256": e["executable_sha256"], "source_current_sha256": source,
                    "candidate_current_sha256": calibrated, "patch_sha256": patch_sha, "case_count": len(results)})


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("kind", choices=("reopen", "comparison"))
    p.add_argument("--evidence", required=True, type=Path)
    p.add_argument("--evidence-sha256", required=True, help="trusted verifier-provided SHA-256 pin")
    p.add_argument("--contract", required=True, type=Path)
    p.add_argument("--contract-sha256", required=True, help="trusted lab owner-provided SHA-256 pin")
    a = p.parse_args()
    try:
        proof = load(a.evidence, a.evidence_sha256)
        contract = load(a.contract, a.contract_sha256)
        if a.kind == "reopen":
            result = reopen(proof, contract)
        elif contract.get("schema") == "qualia.saved_life_card.v1":
            result = active_comparison(proof, contract, a.contract_sha256)
        else:
            result = comparison(proof, contract)
    except (OSError, ValueError, TypeError, KeyError, AttributeError) as exc:
        p.exit(2, f"saved-life evidence not accepted: {exc}\n")
    print(json.dumps(result, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()

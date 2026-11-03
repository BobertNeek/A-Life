"""Read-only, per-domain retention analysis; never selects or installs a brain.

Inputs are normalized observations from separately validated native assessments.
This does not authenticate a checkpoint, run an organism, or certify input/world
matching. Missing coverage or evidence blocks eligibility for owner review.
"""
import argparse
import json
import math
import re
from collections import defaultdict
from pathlib import Path


def require(value, message):
    if not value:
        raise ValueError(message)


def summarize(spec, observations):
    domains = spec["domains"]
    require(domains and len(domains) == len(set(domains)), "declare distinct required domains")
    require(set(spec["domain_owners"]) == set(domains), "missing domain ownership/coverage")
    require(all(set(owners) <= {"pc", "cloud"} and owners
                for owners in spec["domain_owners"].values()), "invalid domain owners")
    minimum = spec.get("minimum_heldout_pairs", 32)
    require(type(minimum) is int and minimum > 0, "invalid minimum pair count")
    alpha = spec.get("alpha", 0.05)
    risk = spec.get("maximum_loss_probability_bound", 0.10)
    require(type(alpha) in (int, float) and math.isfinite(alpha) and 0 < alpha < 1,
            "invalid alpha")
    require(type(risk) in (int, float) and math.isfinite(risk) and 0 < risk < 1,
            "invalid risk limit")
    development = set(spec["development_seeds"])
    heldout = set(spec["heldout_seeds"])
    require(all(type(s) is int and 0 <= s < 2**64 for s in development | heldout), "invalid seed")
    require(heldout and not development & heldout, "development/heldout overlap or empty holdout")
    declared_cases = [(r["domain"], r["seed"], r["condition"]) for r in spec["cases"]]
    require(len(declared_cases) == len(set(declared_cases)), "duplicate predeclared case")
    require(all(d in domains and s in heldout for d, s, _ in declared_cases), "invalid declared case")
    require(spec["historical_freshness"] in ("unknown", "attested"), "declare historical freshness")
    roles = spec["roles"]
    require(len(roles) == len(set(roles)) and {"pc", "cloud"} <= set(roles), "missing/duplicate parent roles")
    cases = defaultdict(dict)
    for row in observations:
        require(row["domain"] in domains and row["role"] in roles, "undeclared domain/role")
        require(type(row["success"]) is bool, "success must be an observed boolean")
        require(type(row["safe"]) is bool, "safe must be a verified boolean")
        require(row["censored"] is False, "censored observations leave required coverage unresolved")
        require(row["partition"] == "heldout" and row["seed"] in heldout,
                "unregistered/nonheldout result")
        require(row["native_evidence_verified"] is True and row["matched_inputs_verified"] is True,
                "native evidence and matched physiology/world controls are required")
        require(row["offline_optimizer_updates"] == 0 and row["teacher_removed"] is True
                and row["prior_removed"] is True, "assessment learning/teacher/prior boundary")
        require(all(isinstance(row[k], str) and re.fullmatch(r"[0-9a-f]{64}", row[k])
                    for k in ("evidence_sha256", "input_match_sha256")), "invalid/missing evidence pins")
        key = (row["domain"], row["seed"], row["condition"])
        require(row["role"] not in cases[key], "duplicate case/role")
        cases[key][row["role"]] = row
    # Every declared heldout seed must contribute a paired case. A successful
    # subset cannot silently replace a larger predeclared failed/missing suite.
    require({key[1] for key in cases} == heldout, "missing predeclared heldout seed")
    require(set(cases) == set(declared_cases), "missing or unregistered predeclared condition")
    for key, group in cases.items():
        require(set(group) == set(roles), f"missing paired role for {key}")
        require(len({row["input_match_sha256"] for row in group.values()}) == 1,
                "case input match identities differ")
    result = {"domains": {}, "candidate_review": {}, "parent_comparison": {},
              "accepted_state_changed": False, "automatic_adoption": False,
              "native_authentication_performed": False,
              "historical_freshness": spec["historical_freshness"],
              "scope": "Analysis of supplied, separately verified observations; not native evidence authentication or a guarantee against catastrophic forgetting."}
    candidates = [role for role in roles if role not in ("pc", "cloud")]
    # Bonferroni across every candidate/domain/parent loss comparison.
    comparisons = max(1, len(domains) * max(1, len(candidates)) * 2)
    corrected_alpha = alpha / comparisons
    for domain in domains:
        pairs = [group for key, group in cases.items() if key[0] == domain]
        require(pairs, f"no observations for {domain}")
        independent_seeds = len({key[1] for key in cases if key[0] == domain})
        counts = {role: sum(pair[role]["success"] for pair in pairs) for role in roles}
        safe = {role: all(pair[role]["safe"] for pair in pairs) for role in roles}
        result["domains"][domain] = {"pairs": len(pairs), "distinct_seeds": independent_seeds,
                                     "successes": counts, "safe": safe}
        result["parent_comparison"][domain] = {
            "pc_only_successes": sum(p["pc"]["success"] and not p["cloud"]["success"] for p in pairs),
            "cloud_only_successes": sum(p["cloud"]["success"] and not p["pc"]["success"] for p in pairs)}
    for role in candidates:
        blockers = []
        bounds = {}
        for domain, stats in result["domains"].items():
            pairs = [group for key, group in cases.items() if key[0] == domain]
            if stats["distinct_seeds"] < minimum:
                blockers.append(f"{domain}: fewer than {minimum} heldout pairs")
            if not stats["safe"][role]:
                blockers.append(f"{domain}: physiology/safety regression")
            if not any(stats["successes"][p] for p in spec["domain_owners"][domain]):
                blockers.append(f"{domain}: original skill not demonstrated by its parent")
            for parent in ("pc", "cloud"):
                losses = sum(p[parent]["success"] and not p[role]["success"] for p in pairs)
                # Exact one-sided binomial bound ONLY in the zero-loss case;
                # observed losses are rejected, not hidden by gains elsewhere.
                bound = 1 - corrected_alpha ** (1 / stats["distinct_seeds"]) if losses == 0 else None
                bounds[f"{domain}:{parent}"] = {"losses": losses, "zero_loss_upper_bound": bound}
                if losses:
                    blockers.append(f"{domain}: {losses} observed losses against {parent}")
                elif bound > risk:
                    blockers.append(f"{domain}: insufficient precision against {parent}")
        result["candidate_review"][role] = {"eligible_for_owner_review": not blockers,
                                             "blockers": blockers, "paired_losses": bounds}
    # Pareto dominance is reported per domain. No scalar chooses a winner when
    # each parent retains distinct competence, or coverage/precision is limited.
    for parent, other in (("pc", "cloud"), ("cloud", "pc")):
        no_losses = all(not p[other]["success"] or p[parent]["success"] for p in cases.values())
        some_gain = any(p[parent]["success"] and not p[other]["success"] for p in cases.values())
        safe = all(p[parent]["safe"] for p in cases.values())
        result.setdefault("parent_dominance", {})[parent] = no_losses and some_gain and safe
    result["limits"] = ["Supplied verifier flags and hashes need an independent native evidence audit.",
                        "Repeated seeds/conditions are not independent draws; report the sampling design.",
                        "Uncovered domains, lifetime memories, and long-horizon continued-learning retention remain unproved.",
                        "A parent dominance observation is not automatic adoption or statistical superiority."]
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("spec", type=Path)
    parser.add_argument("observations", type=Path)
    args = parser.parse_args()
    print(json.dumps(summarize(json.loads(args.spec.read_text()),
                               json.loads(args.observations.read_text())), indent=2, allow_nan=False))


if __name__ == "__main__":
    main()

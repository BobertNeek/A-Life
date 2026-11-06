#!/usr/bin/env python3
"""Independent scalar reading of native eligibility/fast-update WGSL contracts.

Uses actual sealed diagnostic inputs and canonical-coordinate mutable snapshots.
No GPU executes and added-edge private learning/sleep is not qualified here.
"""
import json
import sys
from pathlib import Path

import numpy as np

from audit_retained_bridge import F, FAMILIES, compose_map, distance, load, packed_forward

DRIVES = ["hunger", "fatigue", "fear", "pain", "loneliness", "curiosity", "brain_atp", "temperature_stress", "reproductive_drive"]
HORMONES = ["adrenaline", "cortisol", "dopamine", "oxytocin", "serotonin", "acetylcholine", "learning_modulator", "developmental_hormone", "sleep_pressure"]


def total(values):
    result = F(0)
    for value in values:
        result = F(result + F(value))
    return result


def decay(reference, elapsed):
    if elapsed == 0 or reference == 1: return F(1)
    if reference == 0: return F(0)
    if elapsed == 1: return F(reference)
    return F(np.power(F(reference), F(elapsed)))


def lanes(plan, record):
    p = plan["phenotype"]
    out = record["row"]["patch"]["outcome"]
    phys = out["measured_physiology"]
    value = phys["before"]["value_profile"]
    delta = phys["homeostatic_delta"]
    drives = [F(delta["drives"][key]) for key in DRIVES] + list(map(F, delta["drives"]["extension"]))
    drives[3] = min(F(0), drives[3])
    hormones = [F(delta["hormones"][key]) for key in HORMONES] + list(map(F, delta["hormones"]["extension"]))
    hormones[9] = max(F(0), hormones[9])
    improvement = np.clip(F(F(total(F(w) * d for w, d in zip(value["drives"], drives))
                              + total(F(w) * d for w, d in zip(value["hormones"], hormones)))
                            + F(F(value["energy"]) * F(phys["energy_delta"]))), F(-1), F(1))
    harm = max(F(0), F(phys["pain_delta"]),
               F(F(phys["after"]["body"]["injury"]) - F(phys["before"]["body"]["injury"])),
               F(F(phys["before"]["body"]["health"]) - F(phys["after"]["body"]["health"])))
    pain = np.clip(F(F(value["injury"]) * harm), F(0), F(1))
    frustration = np.clip(F(F(out["frustration_delta"]) * F(value["disappointment"])), F(0), F(1))
    receptors = p["plasticity_receptors"]
    count = F(len(receptors))
    expression = [np.clip(F(F(.25) + F(F(total(abs(F(r["receptor_profile"][lane])) for r in receptors) / count)) * F(.375)), F(.25), F(1)) for lane in [6, 7]]
    # The exported effects contain activation * expression. Inverting that
    # product can differ by an f32 ULP; declared scalar tolerances cover it.
    appetitive = F(F(record["native_receptor_effects"]["plasticity_appetitive"]) / expression[0])
    aversive = F(F(record["native_receptor_effects"]["plasticity_aversive"]) / expression[1])
    negative = np.clip(F(F(pain + max(F(0), F(-improvement))) + frustration), F(0), F(1))
    return [F(out["prediction_error"]), pain, improvement, frustration, F(0), F(0),
            F(max(F(0), improvement) * appetitive), F(negative * aversive)]


def check(plan, source, record):
    p = plan["phenotype"]
    mapping, packed_live, acquired = compose_map(plan, source)
    row = record["row"]
    q = record["evaluations"][0]["sequence"]
    before, after = record["before_neural"], record["after_neural"]
    pre = np.empty(p["neuron_count"], dtype=F)
    assert q["ticks"][0]["microstep_count"] > 0, "zero-step previous-bank qualification is separate"
    expected, offsets = packed_forward(plan, source, mapping, packed_live, acquired, q, before, pre)
    final = np.asarray(expected["final_activations"][0], dtype=F)
    tick = row["patch"]["pre_action"]["tick"]
    elapsed = 0 if before["last_decision_tick"] is None else tick - before["last_decision_tick"]
    outcome_elapsed = row["patch"]["outcome"]["outcome_tick"] - tick
    modulator = lanes(plan, record)
    factors = []
    for receptor in p["plasticity_receptors"]:
        weights = list(map(F, receptor["receptor_profile"]))
        scale = total(abs(w) for w in weights)
        factors.append(F(0) if scale == 0 else np.clip(F(total(F(w * v) for w, v in zip(weights, modulator)) / scale), F(-1), F(1)))
    state = record["before_world"]["organism_records"][0]["phenotype"]["brain_genome"]["plasticity_parameters"]
    periods = row["patch"]["pre_action"]["development_state"]["open_critical_periods"]
    selected = []
    for index in row["action"]["motor_candidates"]:
        if index is not None and index not in selected: selected.append(index)
    assert row["action"]["representative"] in selected
    metadata = np.asarray(source["topology"]["synapse_learning_metadata_words"], dtype=np.uint32).reshape(-1, 8)
    decoders = np.asarray(source["topology"]["decoder_eligibility_metadata_words"], dtype=np.uint32).reshape(-1, 8)
    staged = np.empty(len(mapping), dtype=F)
    fast = np.empty(len(mapping), dtype=F)
    for canonical, live in enumerate(mapping):
        md = metadata[live]
        receptor = p["plasticity_receptors"][md[4]]
        recurrent = md[1] == 1
        route = source["topology"]["route_indices"][live] if recurrent else None
        enabled = q["ticks"][0]["enabled_routes"][route] if recurrent else True
        local = F(0)
        if recurrent and enabled:
            local = F(pre[md[2]] * final[md[3]])
        elif not recurrent:
            decoder = decoders[live - source["topology"]["recurrent_synapse_count"]]
            if decoder[1] in [1, 2, 4]:
                for index in selected:
                    candidate = q["ticks"][0]["candidates"][index]
                    if FAMILIES[candidate["family"]] == decoder[2]:
                        contribution = F(candidate["decoder_inputs"][decoder[3]])
                        if decoder[1] == 1: contribution = F(contribution * final[decoder[4]])
                        local = F(local + contribution)
            elif decoder[1] == 3:
                assert row["action"]["motor_candidates"][3] is None, "speech receipt qualification is separate"
            else: raise AssertionError("unknown native decoder head")
        next_e = F(F(decay(receptor["eligibility_decay"], elapsed) * F(before["eligibility"][canonical])) + local)
        staged[canonical] = next_e if recurrent and not enabled else np.clip(next_e, F(-1), F(1))
        credited = F(staged[canonical] * decay(receptor["eligibility_decay"], outcome_elapsed))
        def rate(name):
            base = F(receptor[name])
            if not recurrent: return base
            projection = p["projections"][route]
            bias = max([F(0)] + [F(period["plasticity_bias"]) for period in periods if period["lobe"] in (projection["source_lobe"], projection["target_lobe"])])
            ceiling = state["base_learning_rate" if name == "learning_rate" else "normalization_rate"]
            return min(F(base * F(F(1) + bias)), F(ceiling))
        synapse = p["synapses"][canonical]
        genetic = F(synapse["genetic_weight"])
        alpha = F(synapse["alpha"])
        old_fast = F(before["fast"][canonical])
        effective = F(F(genetic + F(before["lifetime"][canonical])) + F(alpha * old_fast))
        signed = F(factors[md[4]] * credited)
        post = final[md[3]]
        positive = F(F(rate("learning_rate") * alpha) * signed)
        normalization = F(F(F(F(rate("normalization_rate") * abs(signed)) * post) * post) * effective)
        delta = F(positive - normalization)
        fast[canonical] = np.clip(F(old_fast + delta), F(receptor["fast_weight_min"]), F(receptor["fast_weight_max"]))
    checks = {
        "pre_bank": distance(pre, after["inactive_activations"], 2e-6, 2e-5),
        "staged_eligibility": distance(staged, after["eligibility"], 2e-6, 2e-5),
        "fast_next_update": distance(fast, after["fast"], 2e-6, 2e-5),
        "lifetime_unchanged": distance(before["lifetime"], after["lifetime"], 0, 0),
        "offsets": offsets,
    }
    return {"checks": checks, "canonical_coordinates": len(mapping), "reconstructed_credit_lanes": list(map(float, modulator)),
            "fast_coordinates_changed": int(np.count_nonzero(np.asarray(before["fast"], dtype=F) != np.asarray(after["fast"], dtype=F))),
            "generation_advanced_once": after["weight_generation"] == before["weight_generation"] + 1}


def main(root):
    plan = load(root / "native-plan.json")
    rows = []
    for condition in ["held", "feeding_aged"]:
        source = load(root / f"{condition}-mapping.json")["source"]
        for index, record in enumerate(load(root / f"{condition}-rows.json")):
            result = check(plan, source, record)
            rows.append({"condition": condition, "decision": index, **result})
    passed = all(row["generation_advanced_once"] and not any(c["failed_values"] for c in row["checks"].values()) for row in rows)
    result = {"passed": passed, "scope": "canonical-coordinate staged eligibility and one waking fast update against independently read native scalar contracts; no GPU executes",
              "limits": "Does not qualify hardware arithmetic, acquired-edge private eligibility/updates, sleep, or full live continuation. Receptor activations are reconstructed from the exported effects with declared f32 tolerances.", "rows": rows}
    output = root / "independent-waking-audit.json"
    if output.exists(): raise RuntimeError("preserve existing waking audit before another run")
    output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"passed": passed, "decisions": len(rows), "maxima": {k: max(r["checks"][k]["max_absolute_error"] for r in rows) for k in rows[0]["checks"]}}))
    return not passed


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))

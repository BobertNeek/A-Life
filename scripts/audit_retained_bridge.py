#!/usr/bin/env python3
"""Independent scalar packed-layout audit of audit_retained_bridge exports.

Requires NumPy. Inputs and detailed output stay local; this script contains no
checkpoint data. It follows the native packed ABI, not CPU semantic coordinates.
Numerical tolerances account for f32 summation and WGSL transcendental functions.
"""
import json
import math
import sys
from pathlib import Path

import numpy as np

F = np.float32
FAMILIES = {s: i for i, s in enumerate(
    ["Idle", "Rest", "Inspect", "Approach", "Avoid", "Contact", "Ingest", "Other"])}
CADENCE = {s: i for i, s in enumerate(
    ["Hot60Hz", "Hot15To60Hz", "Hot10To30Hz", "Hot5To15Hz", "Hot1To5Hz", "SleepOrOffline", "Disabled"])}


def load(p):
    return json.loads(p.read_text())


def floats(bits):
    return np.asarray(bits, dtype=np.uint32).view(np.float32)


def distance(a, b, atol=2e-5, rtol=3e-4):
    a, b = np.asarray(a, dtype=np.float64), np.asarray(b, dtype=np.float64)
    delta = np.abs(a - b)
    bound = atol + rtol * np.maximum(np.abs(a), np.abs(b))
    return {"max_absolute_error": float(delta.max(initial=0)),
            "failed_values": int(np.count_nonzero(delta > bound)),
            "values": int(a.size), "absolute_tolerance": atol, "relative_tolerance": rtol}


def typed_same(a, b):
    """Exact integers/structure and f32 bits, despite incidental JSON f64 text."""
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(typed_same(a[k], b[k]) for k in a)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(typed_same(x, y) for x, y in zip(a, b))
    if isinstance(a, float) and isinstance(b, float):
        return F(a).tobytes() == F(b).tobytes()
    return a == b


def fires(cadence, step):
    return cadence == 0 or cadence in (1, 2) and step % 2 == 0 or cadence in (3, 4) and step == 0


def compose_map(plan, source):
    """Canonical -> native bare packed -> live, skipping actual acquired slots."""
    t = source["topology"]
    acquired = set()
    for span in t["v11_checkpoint"]["sparse_spans"]:
        for e in span["edges"]:
            target = e["target"]
            matches = [i for i in range(t["target_offsets"][target], t["target_offsets"][target + 1])
                       if t["source_indices"][i] == e["source"] and t["route_indices"][i] == e["route"]]
            assert len(matches) == 1, "missing or duplicate acquired edge"
            acquired.add(matches[0])
    packed_live = [i for i in range(t["total_synapse_count"]) if i not in acquired]
    assert len(packed_live) == len(plan["genetic"])
    mapping = [packed_live[i] for i in plan["canonical_to_packed"]]
    genetic, alpha = floats(t["genetic_weight_bits"]), floats(t["alpha_bits"])
    assert np.array_equal(genetic[packed_live].view(np.uint32), np.asarray(plan["genetic"], dtype=F).view(np.uint32))
    assert np.array_equal(alpha[packed_live].view(np.uint32), np.asarray(plan["alpha"], dtype=F).view(np.uint32))
    # Independently verify source/target/route after the interior insertion.
    for target in range(t["neuron_count"]):
        expected = [(plan["source_indices"][i], plan["route_indices"][i])
                    for i in range(plan["target_offsets"][target], plan["target_offsets"][target + 1])]
        actual = [(t["source_indices"][i], t["route_indices"][i])
                  for i in range(t["target_offsets"][target], t["target_offsets"][target + 1]) if i not in acquired]
        assert expected == actual
    metadata = np.asarray(plan["decoder_metadata_words"], dtype=np.uint32).reshape(-1, 8).copy()
    metadata[:, 0] = np.asarray(packed_live)[metadata[:, 0]]
    assert np.array_equal(metadata.ravel(), t["decoder_eligibility_metadata_words"])
    return mapping, packed_live, acquired


def packed_forward(plan, source, mapping, packed_live, acquired, q, before, pre_output=None):
    p, t = plan["phenotype"], source["topology"]
    tick = q["ticks"][0]
    g, alpha = floats(t["genetic_weight_bits"]), floats(t["alpha_bits"])
    lifetime = floats(source["lifetime"][f'bank_{source["lifetime"]["active_bank"]}_bits']).copy()
    fast = floats(source["fast"][f'bank_{source["fast"]["active_bank"]}_bits']).copy()
    if before is not None:
        lifetime[mapping], fast[mapping] = np.asarray(before["lifetime"], dtype=F), np.asarray(before["fast"], dtype=F)
    effective = (g + lifetime) + alpha * fast
    base_offset = lifetime[mapping] + alpha[mapping] * fast[mapping]
    offset_check = distance(base_offset, tick["effective_weight_offsets"], atol=0, rtol=0)
    # Replay ablations remove acquired edges explicitly, never renumber Adam.
    retained_edges = {(e["source"], e["target"], e["route"]): F(e["effective_weight"])
                      for e in tick["structural_synapses"]}
    active_acquired = set()
    for target in range(t["neuron_count"]):
        for i in range(t["target_offsets"][target], t["target_offsets"][target + 1]):
            if i in acquired:
                key = (t["source_indices"][i], target, t["route_indices"][i])
                if key in retained_edges:
                    assert effective[i].view(np.uint32) == retained_edges[key].view(np.uint32)
                    active_acquired.add(i)
    assert len(active_acquired) == len(retained_edges), "unexpected live structural changes in bounded probe"
    a = np.asarray(q["initial"]["activations"], dtype=F)
    m = np.asarray(q["initial"]["metabolic_load"], dtype=F)
    ema = np.asarray(q["initial"]["activity_ema"], dtype=F)
    encoded = np.asarray(tick["encoded_inputs"], dtype=F)
    n = t["neuron_count"]
    branches = q["initial"]["dendrites"]["branches"]
    for step in range(tick["microstep_count"]):
        if pre_output is not None:
            pre_output[:] = a
        sums = np.zeros(n, dtype=F)
        for target in range(n):
            for i in range(t["target_offsets"][target], t["target_offsets"][target + 1]):
                if i in acquired and i not in active_acquired:
                    continue
                route = t["route_indices"][i]
                if tick["enabled_routes"][route] and fires(CADENCE[p["projections"][route]["update_cadence"]], step):
                    sums[target] = F(sums[target] + F(a[t["source_indices"][i]] * effective[i]))
        dendritic = np.zeros(n, dtype=F)
        for b in branches:
            s = F(0)
            for i in b["inputs"]:
                s = F(s + F(a[i["source"]] * F(i["weight"])))
            contribution = F(F(np.tanh(F(max(F(0), F(s - F(b["threshold"])))))) * F(b["output_gain"]))
            dendritic[b["target"]] = F(dendritic[b["target"]] + contribution)
        sums = sums + dendritic
        next_a, next_m = np.empty(n, dtype=F), np.empty(n, dtype=F)
        for i, d in enumerate(p["neuron_dynamics"]):
            x = F(F(F(F(F(d["bias"]) + encoded[i]) + F(F(tick["projection_gain"]) * sums[i]))
                    - F(F(d["homeostatic_gain"]) * m[i])) - F(tick["local_threshold_shift"]))
            kind = d["activation"]
            act = x if kind == "Identity" else max(F(0), x) if kind == "Relu" else F(np.tanh(x)) if kind == "Tanh" else F(F(1) / F(F(1) + F(np.exp(F(-x)))))
            next_a[i] = F(F(F(F(1) - F(d["leak"])) * a[i]) + F(F(d["leak"]) * act))
            next_m[i] = np.clip(F(F(F(d["metabolic_decay"]) * m[i]) + F(F(F(F(1) - F(d["metabolic_decay"])) * next_a[i]) * next_a[i])), 0, 1)
            ema[i] = np.clip(F(F(F(d["activity_ema_decay"]) * ema[i]) + F(F(F(1) - F(d["activity_ema_decay"])) * abs(next_a[i]))), 0, 1)
        a, m = next_a, next_m
    families = np.asarray(plan["decoder_family_words"], dtype=np.uint32).reshape(-1, 8)
    metadata = np.asarray(t["decoder_eligibility_metadata_words"], dtype=np.uint32).reshape(-1, 8)
    indices = np.asarray(plan["decoder_weight_index_words"], dtype=np.uint32).reshape(-1, 4)
    # Native ABI separates action maps, memory map, and cognitive metadata.
    motor_start = p["decoder"]["motor_start"]
    logits = []
    for c in tick["candidates"]:
        family = FAMILIES[c["family"]]
        fr = next(r for r in families if r[0] == family)
        logit = F(floats([fr[1]])[0] + F(c["innate_bias"]))
        inputs = np.asarray(c["decoder_inputs"], dtype=F)
        for packed, lane, motor, reserved in indices:
            live = packed_live[packed]
            md = metadata[live - t["recurrent_synapse_count"]]
            if md[2] == family:
                # Native shader is motor * feature * effective, in that order.
                logit = F(logit + F(F(a[motor_start + motor] * inputs[lane]) * effective[live]))
        mem, cog = F(0), F(0)
        for packed in plan["memory_weight_indices"]:
            live = packed_live[packed]
            md = metadata[live - t["recurrent_synapse_count"]]
            if md[2] == family:
                mem = F(mem + F(inputs[md[3]] * effective[live]))
        for md in metadata:
            if md[1] == 4 and md[2] == family:
                cog = F(cog + F(inputs[md[3]] * effective[md[0]]))
        gain = F(q["memory_candidate_gain"])
        logits.append(F(F(logit + np.clip(mem, -gain, gain)) + np.clip(cog, -gain, gain)))
    return {"candidate_logits": [logits], "final_activations": [a], "final_activity_ema": [ema], "final_metabolic_load": [m]}, offset_check


def native_sample(logits, frame_candidates, seed, counter, enabled_slots):
    def slot(kind):
        return {"Move": 0, "Look": 1, "Interact": 2, "Write": 2, "Vocalize": 3,
                "Hold": 4, "Rest": 4, "Inspect": 4}.get(kind)
    def draw(factor):
        v = (seed ^ ((counter * 0x9e3779b9) & 0xffffffff) ^ ((factor * 0x85ebca6b) & 0xffffffff))
        v = ((v ^ (v >> 16)) * 0x7feb352d) & 0xffffffff
        v = ((v ^ (v >> 15)) * 0x846ca68b) & 0xffffffff
        v ^= v >> 16
        return F(F(F(v >> 9) + F(.5)) / F(8388608))
    def pick(allowed, factor):
        maximum = max(logits[i] for i in allowed)
        exps = [(i, F(np.exp(F(logits[i] - maximum)))) for i in allowed]
        total = F(0)
        for _, e in exps: total = F(total + e)
        target, cumulative = F(draw(factor) * total), F(0)
        for i, e in exps:
            cumulative = F(cumulative + e)
            if target < cumulative: return i
        return exps[-1][0]
    representative = pick(list(range(len(logits))), 0)
    kind = frame_candidates[representative]["kind"]
    forced = 4 if kind in ("Idle", "Gesture") else slot(kind)
    selected = [None] * 6
    for s in range(6):
        allowed = [i for i, c in enumerate(frame_candidates) if s in enabled_slots and slot(c["kind"]) == s]
        if forced == s: selected[s] = representative
        elif allowed: selected[s] = pick(allowed, s + 1)
    return {"representative": representative, "motor_candidates": selected}


def input_checks(plan, record):
    """Rebuild encoder, candidate features, memory lanes and native innate bias."""
    p = plan["phenotype"]
    frame = record["row"]["patch"]["pre_action"]["perception"]
    base = frame["base"]
    effects = record["native_receptor_effects"]
    tick = record["row"]["replay_tick"]
    sensory = base["sensory"]
    channels = sensory["channels"]
    flat = sum([channels[k] for k in ["visual_affordance", "auditory_acoustic", "smell_chemistry", "tactile_contact"]], [])
    flat += [channels["pain_signal"], channels["novelty_signal"]]
    body = base["body"]
    position = [body["pose"]["translation"][k] for k in "xyz"]
    if base["sensor_profile"] == "GroundedTerrainVisionV1": position = [0., 0., 0.]
    body_flat = position + [body["pose"]["rotation"][k] for k in "xyzw"] + [body["velocity"][v][k] for v in ["linear", "angular"] for k in "xyz"]
    drives = base["homeostasis"]["drives"]
    home = [drives[k] for k in ["hunger", "fatigue", "fear", "pain", "loneliness", "curiosity", "brain_atp", "temperature_stress", "reproductive_drive"]] + drives["extension"]
    hormone = base["homeostasis"]["hormones"]
    home += [hormone[k] for k in ["adrenaline", "cortisol", "dopamine", "oxytocin", "serotonin", "acetylcholine", "learning_modulator", "developmental_hormone", "sleep_pressure"]] + hormone["extension"]
    language = [F(0)] * 256
    for slot, heard in enumerate(sensory["language_context"]["heard_tokens"][:16]):
        if heard is not None:
            for bit in range(8): language[slot * 8 + bit] = F(heard["confidence"] if heard["token_id"] & (1 << bit) else -heard["confidence"])
    assert sensory["semantic_context"] is None, "bounded assessment expected no semantic prior"
    groups = {"SensoryChannel": [F(F(x) * F(effects["attention_gain"])) for x in flat], "Body": body_flat,
              "Homeostasis": [F(F(x) * F(effects["interoceptive_gain"])) for x in home], "HeardLanguage": language[:128], "SemanticPrior": language[128:]}
    encoded = np.zeros(p["neuron_count"], dtype=F)
    for assignment in p["sensor_encoder"]["assignments"]:
        x = F(F(F(groups[assignment["source_group"]][assignment["source_index"]]) * F(assignment["scale"])) + F(assignment["bias"]))
        v = np.clip(x, F(assignment["clamp_min"]), F(assignment["clamp_max"]))
        target = assignment["target_neuron"]
        encoded[target] = F(encoded[target] + v)
    gate = F(F(F(F(effects["structural_growth_gate"]) + F(effects["sleep_gate"])) + F(effects["consolidation_gate"])) / F(3))
    encoded = np.clip(encoded * F(effects["regional_excitability"]) + F(F(gate - F(.5)) * F(.05)), F(-1), F(1))
    checks = {"encoded_inputs": distance(encoded, tick["encoded_inputs"], atol=2e-7, rtol=2e-6),
              "projection_gain": distance([effects["projection_gain"]], [tick["projection_gain"]], 0, 0),
              "threshold_shift": distance([effects["local_threshold_shift"]], [tick["local_threshold_shift"]], 0, 0)}
    features, memory, innate = [], [], []
    contexts = np.asarray(frame["context"]["values"], dtype=F).reshape(-1, 16)
    families = np.asarray(plan["decoder_family_words"], dtype=np.uint32).reshape(-1, 8)
    for index, (c, encoded_candidate) in enumerate(zip(base["candidates"], tick["candidates"])):
        assert c["candidate_index"] == index and c["family"] == encoded_candidate["family"]
        features.extend(c["features"])
        context = contexts[index]
        memory.extend([F(v * np.clip(context[12], F(0), F(1))) for v in context[:8]])
        memory.extend([F(v * np.clip(context[13], F(0), F(1))) for v in context[8:12]])
        fr = next(r for r in families if r[0] == FAMILIES[c["family"]])
        flags, gain = int(fr[6]), floats([fr[7]])[0]
        mask, cuecode = flags & 0x1ff, (flags >> 16) & 0xff
        urgency = max([F(0)] + [F(home[i]) for i in range(9) if mask & (1 << i)])
        f = np.asarray(c["features"], dtype=F)
        cue, reach = F(1), F(1)
        if cuecode:
            bearing = F(F(F(1) - f[18]) * np.clip(F(abs(f[0]) + abs(f[1])), F(0), F(1)))
            if cuecode == 25:
                valence = F(0) if f[15] < F(-.2) else F(1) if f[15] > F(.2) else F(.75)
                signal = F(bearing * valence)
            else: signal = f[cuecode - 1]
            if fr[0] == 4 and cuecode == 16 and flags & 0x02000000 and not flags & 0x01000000 and f[18] == 0:
                signal = bearing
            elif flags & 0x02000000: signal = F(-signal)
            cue = np.clip(F(F(signal - F(.4)) / F(.6)), F(0), F(1))
        if flags & 0x01000000: reach = np.clip(F(F(F(.25) - f[2]) / F(.15)), F(0), F(1))
        innate.append(F(F(F(max(F(0), F(urgency - F(.55))) * gain) * cue) * reach) if mask and gain != 0 else F(0))
    checks["candidate_features"] = distance(features, [v for c in tick["candidates"] for v in c["decoder_inputs"][:24]], 0, 0)
    checks["memory_input_lanes"] = distance(memory, [v for c in tick["candidates"] for v in c["decoder_inputs"][24:36]], 0, 0)
    checks["native_innate_bias"] = distance(innate, [c["innate_bias"] for c in tick["candidates"]], 2e-7, 2e-6)
    for key in ["activations", "activity_ema", "metabolic_load"]:
        checks["initial_" + key] = distance(record["before_neural"]["neural"][key], record["row"]["initial"][key], 0, 0)
    return checks


def main(root):
    plan = load(root / "native-plan.json")
    report = {"scope": "independent packed scalar plus offline WGSL; no hardware live parity", "checks": [], "cases": []}
    failures = []
    for condition in ["held", "feeding_aged"]:
        source_record = load(root / f"{condition}-mapping.json")
        source = source_record["source"]
        mapping, packed_live, acquired = compose_map(plan, source)
        assert mapping == source_record["adapter_map"]
        report["checks"].append({"condition": condition, "all_coordinate_mapping_exact": True, "coordinates": len(mapping), "acquired_slots": sorted(acquired), "native_decoder_metadata_exact": True})
        for number, record in enumerate(load(root / f"{condition}-rows.json")):
            inputs = input_checks(plan, record)
            if any(c["failed_values"] for c in inputs.values()): failures.append((condition, number, "input"))
            row, old = record["row"], record["original_trace"]
            trace = {
                "frame_exact": typed_same(row["patch"]["pre_action"]["perception"]["frame_digest"], old["frame_digest"]),
                "command_exact": typed_same(row["patch"]["decision"]["selected_action"], old["action"]),
                "bundle_exact": typed_same(row["patch"]["decision"]["selected_bundle"], old["bundle"]),
                "outcome_exact": typed_same(row["patch"]["outcome"], old["outcome"]),
                "sampling_record_exact": typed_same(row["action"], old["ppo_action"]),
                "recomputed_cpu_sample_exact": typed_same(row["action"], record["evaluations"][0]["cpu_sample"]),
            }
            if not all(trace.values()): failures.append((condition, number, "typed-trace-reproduction"))
            results = []
            for ev in record["evaluations"]:
                before = record["before_neural"]
                if ev["variant"] in ("zero_lifetime", "zero_fast", "zero_somatic"):
                    before = dict(before)
                    for layer in ("lifetime", "fast"):
                        if ev["variant"] == "zero_somatic" or ev["variant"] == "zero_" + layer:
                            before[layer] = [0.] * len(before[layer])
                expected, offsets = packed_forward(plan, source, mapping, packed_live, acquired, ev["sequence"], before)
                item = {"variant": ev["variant"], "effective_offset_check": offsets,
                        "scalar_vs_cpu": {key: distance(expected[key], ev["cpu"][key]) for key in expected}}
                if "offline_wgsl" in ev:
                    item["wgsl_vs_cpu"] = {key: distance(ev["offline_wgsl"][key], ev["cpu"][key]) for key in expected}
                    item["wgsl_sample_matches_cpu"] = all(ev["offline_wgsl_sample"][k] == ev["cpu_sample"][k] for k in ("representative", "motor_candidates"))
                # Candidate identities come from actual canonical finalized frames.
                frame = record["row"]["patch"]["pre_action"]["perception"]
                candidates = frame["base"]["candidates"]
                sampled = native_sample(expected["candidate_logits"][0], candidates, record["sampling_seed"], record["row"]["sampling_counter"], {0})
                item["independent_sample"] = sampled
                item["independent_sample_matches_cpu"] = all(sampled[k] == ev["cpu_sample"][k] for k in sampled)
                item["logits"] = [float(x) for x in expected["candidate_logits"][0]]
                if any(c["failed_values"] for c in item["scalar_vs_cpu"].values()) or offsets["failed_values"] or not item["independent_sample_matches_cpu"]:
                    failures.append((condition, number, ev["variant"], "scalar"))
                if "wgsl_vs_cpu" in item and (any(c["failed_values"] for c in item["wgsl_vs_cpu"].values()) or not item["wgsl_sample_matches_cpu"]):
                    failures.append((condition, number, ev["variant"], "wgsl"))
                results.append(item)
            report["cases"].append({"condition": condition, "decision": number, "results": results, "input_checks": inputs, "typed_trace_reproduction": trace,
                                    "candidates": candidates, "original_trace": record["original_trace"]})
    synthetic = load(root / "nonzero-all-families.json")
    source = load(root / "held-mapping.json")["source"]
    mapping, packed_live, acquired = compose_map(plan, source)
    expected, offsets = packed_forward(plan, source, mapping, packed_live, acquired, synthetic["sequence"], None)
    report["nonzero_all_families"] = {"scalar_vs_cpu": {key: distance(expected[key], synthetic["cpu"][key]) for key in expected}}
    if "offline_wgsl" in synthetic:
        report["nonzero_all_families"]["wgsl_vs_cpu"] = {key: distance(synthetic["offline_wgsl"][key], synthetic["cpu"][key]) for key in expected}
    if any(c["failed_values"] for group in report["nonzero_all_families"].values() for c in group.values()):
        failures.append(("synthetic", "nonzero", "all-families"))
    report["failures"] = failures
    (root / "independent-packed-audit.json").write_text(json.dumps(report, indent=2))
    print(json.dumps({"cases": len(report["cases"]), "failures": failures, "nonzero_all_families": report["nonzero_all_families"]}))
    return bool(failures)


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))

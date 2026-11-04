#!/usr/bin/env python3
"""Summarize native public-founder captures; no simulation or learning is invented.

Generate each capture with the ignored capture_native_language_causal_source test.
This report is causal-path evidence, never a retained-behavior qualification.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path


def flatten(value):
    if isinstance(value, list):
        for item in value:
            yield from flatten(item)
    else:
        yield value


def difference(left, right):
    a, b = list(flatten(left)), list(flatten(right))
    if len(a) != len(b):
        raise ValueError("native capture dimensions differ")
    return {
        "changed_values": sum(x != y for x, y in zip(a, b)),
        "max_absolute_difference": max((abs(x - y) for x, y in zip(a, b)), default=0),
    }


def audit(directory):
    def read(name):
        return json.loads((directory / name).read_bytes())

    p = read("phenotype.json")
    a, b = read("eat-rows.json"), read("get-rows.json")
    if any(row["replay_tick"]["structural_synapses"] for row in a + b):
        raise ValueError("this compiled-graph assay requires no additional structural edges")
    ae, be = read("eat-evaluation.json"), read("get-evaluation.json")
    interventions = read("diagnostic-interventions.json")
    regions = p["lobe_layout"]["regions"]

    def lobe(neuron):
        return next(r["kind"] for r in regions if r["start"] <= neuron < r["start"] + r["len"])

    readouts = collections.defaultdict(set)
    adjacency = collections.defaultdict(list)
    for synapse in p["synapses"]:
        if synapse["kind"] == "Recurrent":
            if a[0]["replay_tick"]["enabled_routes"][synapse["route_index"]]:
                adjacency[synapse["source"]].append(synapse["target"])
        else:
            head = synapse["kind"]["Decoder"]["head"]
            if head in ("ActionCandidate", "SpeechPayload"):
                readouts[head].add(synapse["source"])

    changed = [i for i, (x, y) in enumerate(zip(
        a[0]["replay_tick"]["encoded_inputs"], b[0]["replay_tick"]["encoded_inputs"]
    )) if x != y]
    reachability = collections.defaultdict(list)
    for neuron in changed:
        seen, queue = {neuron}, collections.deque([neuron])
        while queue:
            for target in adjacency[queue.popleft()]:
                if target not in seen:
                    seen.add(target)
                    queue.append(target)
        signature = json.dumps({
            "outdegree": len(adjacency[neuron]),
            "reachable_neurons_by_lobe": dict(sorted(collections.Counter(lobe(n) for n in seen).items())),
            "reachable_action_sources": len(seen & readouts["ActionCandidate"]),
            "reachable_speech_sources": len(seen & readouts["SpeechPayload"]),
        }, sort_keys=True)
        reachability[signature].append(neuron)

    activations = []
    for tick, (x, y) in enumerate(zip(ae["activations"], be["activations"])):
        changed_state = {i for i, (j, k) in enumerate(zip(x, y)) if j != k}
        activations.append({
            "tick": tick,
            "changed_by_lobe": dict(sorted(collections.Counter(lobe(n) for n in changed_state).items())),
            "changed_action_sources": len(changed_state & readouts["ActionCandidate"]),
            "changed_speech_sources": len(changed_state & readouts["SpeechPayload"]),
        })
    return {
        "phenotype_sha256": hashlib.sha256((directory / "phenotype.json").read_bytes()).hexdigest(),
        "experimental_language_input": p.get("experimental_language_input"),
        "changed_input_neurons": changed,
        "all_compiled_routes_enabled": all(a[0]["replay_tick"]["enabled_routes"]),
        "additional_structural_edges": 0,
        "first_tick_non_input_replay_context_exact": {
            key: a[0]["replay_tick"][key] == b[0]["replay_tick"][key]
            for key in a[0]["replay_tick"] if key != "encoded_inputs"
        },
        "first_tick_motor_masks": a[0]["action"]["motor_masks"],
        "first_tick_native_masks_exact": {
            key: a[0]["action"][key] == b[0]["action"][key]
            for key in ("candidate_count", "representative_mask", "motor_masks", "forced_slots")
        },
        "reachability_groups": [dict(json.loads(k), input_neurons=v) for k, v in reachability.items()],
        "actual_word_comparison": {
            "first_tick_logits": difference(ae["logits"][0], be["logits"][0]),
            **{key: difference(ae[key], be[key]) for key in ("logits", "gradient")},
        },
        "activation_changes": activations,
        "diagnostic_input_pulses": {
            name: {key: difference(interventions["baseline"][key], interventions[name][key])
                   for key in ("logits", "activations", "gradient")}
            for name in ("word_neuron_pulse", "ingest_readout_pulse")
        },
        "native_baseline_nonzero_gradient_coordinates": sum(x != 0 for x in ae["gradient"]),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("legacy_capture", type=Path)
    parser.add_argument("experimental_capture", type=Path)
    args = parser.parse_args()
    print(json.dumps({
        "kind": "experimental-native-language-causal-audit-v1",
        "demonstrates_retained_behavior": False,
        "legacy": audit(args.legacy_capture),
        "experimental": audit(args.experimental_capture),
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()

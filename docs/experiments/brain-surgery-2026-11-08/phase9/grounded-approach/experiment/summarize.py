#!/usr/bin/env python3
"""Summarize public scalar rows only; never load a brain or run a world."""
import json
import sys
from pathlib import Path

rows = [json.loads(line) for line in Path(sys.argv[1]).read_text().splitlines() if line.strip()]
assert len(rows) == 6
assert len({(row["initial_loneliness"], row["case"]) for row in rows}) == 6
lanes = ["hunger", "fatigue", "fear", "pain", "loneliness", "curiosity", "brain_atp", "temperature_stress", "reproductive_drive", "extension0", "extension1"]
for row in rows:
    assert row["biology_updates"] == 1
    assert row["ticks"] == [600, 601]
    assert row["store_records"] == 0
    assert row["exact_same_source_lookup"]["evidence"] == "Unknown"
    assert row["source_sequence"] is None
    assert len(row["raw_eleven_deltas"]) == 11
contrasts = []
for pressure in sorted({row["initial_loneliness"] for row in rows}):
    arms = {row["case"]: row for row in rows if row["initial_loneliness"] == pressure}
    peer = arms["peer_approach"]
    for control in ["toy_approach", "peer_passive"]:
        other = arms[control]
        differences = [a - b for a, b in zip(peer["raw_eleven_deltas"], other["raw_eleven_deltas"])]
        contrasts.append({"initial_loneliness": pressure, "comparison": f"peer_approach minus {control}", "raw_delta_difference": dict(zip(lanes, differences)), "interpretation": "between-fork contrast, not isolated motor credit or a stored effect"})
summary = {
    "actual_world_intervals": len(rows), "budget_remaining": 0,
    "original_maximum_budget": 8, "unspent_closed_slots": 2,
    "unavailable_controls": [{"case": "peer_blocked", "initial_loneliness": p, "actual_world_intervals": 0, "reason": "four cage objects occupy all object-candidate slots; no prescribed peer command fabricated"} for p in [0, 0.15]],
    "all_intended_physical_conditions_met": all(row["intended_condition_met"] for row in rows),
    "neural_forward_passes": 0, "selected_policy_actions": 0,
    "admitted_neural_sources": 0,
    "exact_same_source_unknown_count": sum(row["exact_same_source_lookup"]["evidence"] == "Unknown" for row in rows),
    "actual_next_source_unknown_count": sum(row["actual_next_source_lookup"] is not None and row["actual_next_source_lookup"]["evidence"] == "Unknown" for row in rows),
    "actual_next_source_key_equality_count": sum(row["actual_next_source_lookup"] is not None and row["actual_next_source_lookup"]["same_exact_context_key"] for row in rows),
    "rows": [{key: row[key] for key in ["case", "initial_loneliness", "intended_condition_met", "travel", "initial_peer_contact", "final_peer_contact", "body_event", "raw_eleven_deltas", "body_energy_before", "body_energy_after", "sensor_confidence"]} for row in rows],
    "contrasts": contrasts,
    "store_result": "Unknown because authentic neural evidence was not produced; not a retained-source recall failure",
}
print(json.dumps(summary, indent=2, sort_keys=True))

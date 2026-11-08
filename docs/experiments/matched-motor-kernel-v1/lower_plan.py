"""Verify/reproduce frozen branch/readout lowering; does not run the circuit."""
import hashlib
import json
from pathlib import Path
import sys

RECIPE_SHA256 = "3f084d2ca401acd992629a19fa4db763a7ecc7315c62f592b0e0fb2f0e9eac1a"
FAMILIES = ("Idle", "Rest", "Inspect", "Approach", "Avoid", "Contact", "Ingest", "Other")


def lower(path):
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != RECIPE_SHA256:
        raise ValueError("frozen_recipe_mismatch")
    plan = json.loads(raw)
    if plan["variant"] != "M0" or len(plan["new_branches"]) != 70 or len(plan["new_arbitration_readout_head"]) != 50:
        raise ValueError("invalid_recipe_shape")
    lines = [
        "// Exact lowering of the archived M0 recipe; logical addresses subtract2048.",
        "// Generated once, no runtime hidden-world inputs or action selection.",
        "use super::{Branch, Readout};",
        "#[rustfmt::skip]",
        "pub(super) const BRANCHES: &[Branch] = &[",
    ]
    for branch in plan["new_branches"]:
        inputs = ", ".join(f"({source - 2048}, {weight:.8f}_f32)" for source, weight in branch["inputs"])
        lines.append(f'    Branch {{ target: {branch["target"] - 2048}, inputs: &[{inputs}], threshold: {branch["threshold"]:.8f}, gain: {branch["output_gain"]:.8f} }},')
    lines.extend(["];", "#[rustfmt::skip]", "pub(super) const READOUTS: &[Readout] = &["])
    for row in plan["new_arbitration_readout_head"]:
        lines.append(f'    Readout {{ family: {FAMILIES.index(row["family"])}, source: {row["source"] - 2048}, lane: {row["lane"]}, weight: {row["weight"]:.8f} }},')
    lines.extend(["];", "#[rustfmt::skip]", f'pub const RECIPE_SHA256: &str = "{RECIPE_SHA256}";'])
    return "\n".join(lines) + "\n"


if __name__ == "__main__":
    directory = Path(__file__).resolve().parent
    expected = lower(directory / "mediation-m0-manifest.json")
    if sys.argv[1:] == ["--write"]:
        (directory / "plan.rs").write_text(expected)
    elif sys.argv[1:]:
        raise SystemExit("usage: lower_plan.py [--write]")
    elif (directory / "plan.rs").read_text() != expected:
        raise SystemExit("lowered_source_mismatch")
    print(json.dumps({"frozen_recipe_sha256": RECIPE_SHA256, "branches": 70, "branch_inputs": 184, "readouts": 50, "exact_lowering": True}, sort_keys=True))

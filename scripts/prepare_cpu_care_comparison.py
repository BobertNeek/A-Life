#!/usr/bin/env python3
"""Prepare private requests for the public cases; never run or reserve anything.

A native CPU source is mandatory. This script does not manufacture admission for
Current/GPU checkpoints. The evaluator remains the authority for native source,
seed history, graph compatibility, protected outputs and artifact admission.
"""
import argparse
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--foundation", required=True, type=Path)
    parser.add_argument("--anchor", required=True, type=Path)
    parser.add_argument("--history", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    catalogue = json.loads((Path(__file__).resolve().parents[1] /
                            "docs/fixtures/cpu-care-eight-cases-v1.json").read_text())
    foundation = args.foundation.resolve(strict=True)
    anchor = args.anchor.resolve(strict=True)
    history = args.history.resolve(strict=True)
    if not foundation.is_file() or not history.is_file() or not anchor.is_dir():
        parser.error("foundation/history must be files; anchor must be a native CPU directory")
    cycle = anchor / "cpu-cycle.json"
    if not cycle.is_file():
        parser.error("anchor lacks cpu-cycle.json; Current/GPU inputs require supported adaptation")
    receipt = json.loads(cycle.read_text())
    body_seed = receipt.get("founder_seed_base") or receipt.get("seed")
    if body_seed != catalogue["founder_seed_base"]:
        parser.error("anchor founder seed differs from the declared comparison body; do not edit receipts")
    root = args.output.resolve()
    # Outputs must be outside sources; the native evaluator checks all remaining
    # native dependency roots and exact pinned history before world dispatch.
    for protected in (foundation, anchor, history):
        if root == protected or root in protected.parents or protected in root.parents:
            parser.error("output overlaps a source; use a separate absent private directory")
    if root.exists():
        parser.error("output exists; preserve requests and lives instead of restarting")
    digest = hashlib.sha256(foundation.read_bytes()).hexdigest()
    requests = {}
    for case in catalogue["cases"]:
        request = {"foundation": str(foundation), "expected_foundation_sha256": digest,
                   "seed_anchor": str(anchor), "seed_history": str(history),
                   "output": str(root / case["id"]), "seed": case["seed"],
                   "decisions": catalogue["decisions"], "temperature": catalogue["temperature"],
                   "lesson": case["lesson"], "food_need_condition": case["food_need_condition"],
                   "known_exclusions": catalogue["known_exclusions"]}
        requests[case["id"]] = json.dumps(request, indent=2) + "\n"
    root.mkdir(parents=True, exist_ok=False)
    for ident, request in requests.items():
        with (root / (ident + "-spec.json")).open("x") as stream:
            stream.write(request)
    print("Prepared 8 private requests; no native admission, reservation or execution performed.")


if __name__ == "__main__":
    main()

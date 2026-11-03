#!/usr/bin/env python3
"""Bounded CPU-only sampler pilot from a newly generated disposable founder.

No accepted/previous source argument exists. Never resumes or overwrites an
output. Discovery and holdouts disable teacher/prior via native evaluate; solver
training uses unchanged native PPO/biological rewards with identical requested
decision budgets. If discovery finds no empirical frontier, stop honestly.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

from compare_cpu_care_rounds import file_pin, native_pins, raw_tree, read_json, require
from care_frontier_sampler import score_assessment, select, validate_bank

FAMILIES = [
    {"id": "held", "lesson": "eat_held_food", "food_need_condition": "newborn_v1", "placement": "native_seeded_v1", "difficulty": 0},
    {"id": "reachable", "lesson": "eat_reachable_food", "food_need_condition": "newborn_v1", "placement": "native_seeded_v1", "difficulty": 1},
    {"id": "feeding", "lesson": "feeding", "food_need_condition": "newborn_v1", "placement": "native_seeded_v1", "difficulty": 3},
    {"id": "feeding_aged", "lesson": "feeding", "food_need_condition": "ordinary_healthy_aging_v1", "placement": "native_seeded_v1", "difficulty": 2},
]


def write_new(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, sort_keys=True, allow_nan=False)
        stream.write("\n")


class Pilot:
    def __init__(self, root, care_binary, round_binary, decisions, discovery_trials, slots, bank=None, discovery=None):
        require(not root.exists() and not root.is_symlink(), "fresh disposable output required; never resume")
        require(1 <= decisions <= 128 and 3 <= discovery_trials <= 8 and 1 <= slots <= 8, "bounded pilot settings required")
        self.root, self.care, self.round = root, care_binary.resolve(strict=True), round_binary.resolve(strict=True)
        self.pins = {str(p): file_pin(p) for p in (self.care, self.round)}
        self.decisions, self.trials, self.slots = decisions, discovery_trials, slots
        self.bank, self.discovery = bank, discovery
        self.calls = []
        root.mkdir(parents=True, exist_ok=False)
        self.frozen = {}

    def guard(self):
        require(all(file_pin(Path(p)) == pin for p, pin in self.pins.items()), "native executable changed")
        require(all(raw_tree(Path(p)) == pin for p, pin in self.frozen.items()), "completed disposable source/archive changed")

    def command(self, binary, arguments, cwd):
        self.guard()
        number = len(self.calls)
        stem = self.root / "commands" / f"{number:03d}"
        stem.parent.mkdir(exist_ok=True)
        environment = {k: v for k, v in os.environ.items() if not k.startswith("ALIFE_SLM_")}
        environment.update(ALIFE_SLM_PRIOR="off", ALIFE_SLM_PRIOR_BACKEND="off")
        argv = [str(binary), *map(str, arguments)]
        write_new(stem.with_suffix(".argv.json"), {"argv": argv, "cwd": str(cwd), "prior": "off"})
        print(f"native fixture command {number}: {Path(binary).name} {arguments[0]}", flush=True)
        started = time.monotonic()
        with stem.with_suffix(".stdout.json").open("xb") as out, stem.with_suffix(".stderr.log").open("xb") as err:
            completed = subprocess.run(argv, cwd=cwd, env=environment, stdin=subprocess.DEVNULL,
                                       stdout=out, stderr=err, timeout=60, check=False)
        self.calls.append({"index": number, "exit_code": completed.returncode, "seconds": time.monotonic()-started})
        self.guard()
        require(completed.returncode == 0, f"native fixture failed; preserve command {number} and never retry it")
        return read_json(stem.with_suffix(".stdout.json"), 64 * 1024**2)

    def history(self, source, path, trained, assessments):
        write_new(path, {"schema": 1, "kind": "mixed-care-seed-history", "complete": True,
                        "evidence": "Only this newly generated disposable fixture; no retained lineage or historical/generalization claim",
                        "source_sha256": native_pins(source), "consumed_seed_ranges": [[s, s] for s in sorted(set(trained))],
                        "reserved_seed_ranges": [], "assessment_seeds": sorted(assessments), "imports": []})
        return path

    def assessment(self, source, prefix, task, families, trained, assessment_seeds):
        family = families[task["family"]]
        history = self.history(source, prefix.with_suffix(".history.json"), trained, assessment_seeds)
        request = {"previous": str(source), "output": str(prefix), "seed_history": str(history),
                   "seed": task["seed"], "decisions": self.decisions, "temperature": 1.0,
                   "lesson": family["lesson"], "food_need_condition": family["food_need_condition"], "known_exclusions": False}
        if "food_offset" in family:
            request["food_offset"] = family["food_offset"]
        spec = prefix.with_suffix(".request.json")
        write_new(spec, request)
        native_result = self.command(self.round, ["evaluate", "--spec", spec], self.root)
        require(read_json(prefix / "care-evaluation.json")["result"] == native_result
                and all(native_result[key] == request[key] for key in ("lesson", "seed", "decisions")),
                "native CLI/receipt/request binding mismatch")
        scored = score_assessment(prefix, self.round)
        self.frozen[str(prefix)] = raw_tree(prefix)
        return scored

    def run(self):
        source = self.root / "initial-source"
        self.command(self.care, ["--fixture", "--output", source, "--seed", 881001,
                                 "--lesson", "eat_held_food", "--decisions", 2], self.root)
        self.frozen[str(source)] = raw_tree(source)
        discovery = [{"family": f["id"], "seed": 882001 + 100*i+j}
                     for i, f in enumerate(FAMILIES) for j in range(self.trials)]
        bank = {"schema": 1, "decision_budget": self.decisions, "slots": self.slots, "sampler_seed": 20261103,
                "current_difficulty": 1,
                "families": FAMILIES, "pool": [{"family": f["id"], "seed": 884001+100*i+j}
                                                for i, f in enumerate(FAMILIES) for j in range(8)],
                "holdouts": [{"family": f["id"], "seed": 886001+100*i+j}
                             for i, f in enumerate(FAMILIES) for j in range(2)]}
        if self.bank is not None:
            bank, discovery = self.bank, self.discovery
            require(bank["decision_budget"] == self.decisions and bank["slots"] == self.slots,
                    "fixture settings differ from generated bank")
            require(isinstance(discovery, list) and 1 <= len(discovery) <= 128,
                    "bounded discovery seeds required")
            known = {f["id"] for f in bank["families"]}
            seeds = [t["seed"] for t in discovery]
            used = {t["seed"] for kind in ("pool", "holdouts") for t in bank[kind]}
            require(all(set(t) == {"family", "seed"} and t["family"] in known
                        and type(t["seed"]) is int and 1 <= t["seed"] < 2**64 for t in discovery)
                    and len(set(seeds)) == len(seeds) and not (set(seeds) & (used | {881001}))
                    and 881001 not in used, "discovery, training, holdout and founder seeds must be disjoint")
        families = validate_bank(bank)
        write_new(self.root / "task-bank.json", bank)
        assessments = {t["seed"] for t in discovery + bank["holdouts"]}
        observations = [self.assessment(source, self.root / "discovery" / f"{i:02d}", task,
                                       families, [881001], assessments) for i, task in enumerate(discovery)]
        plan = select(bank, observations)
        write_new(self.root / "selection.json", plan)
        result = {"schema": 1, "scope": "disposable native CPU fixture only; no accepted individual, HRPO, speedup or efficacy claim",
                  "status": plan["status"], "executable_pins": self.pins, "decision_budget": self.decisions,
                  "discovery": observations, "statistics": plan["statistics"], "arms": {},
                  "solver_rewards_changed": False, "promotion": "none"}
        if plan["status"] == "ready":
            for arm in ("baseline", "adaptive"):
                previous, trained, records = source, [881001], []
                lane = self.root / arm
                lane.mkdir()
                for i, task in enumerate(plan[arm]):
                    family = families[task["family"]]
                    history = self.history(previous, lane / f"train-{i:02d}.history.json", trained, assessments)
                    output = lane / f"train-{i:02d}"
                    arguments = ["--previous", previous, "--output", output,
                        "--seed-history", history, "--seed", task["seed"], "--lesson", family["lesson"],
                        "--food-need-condition", family["food_need_condition"], "--decisions", self.decisions]
                    if "food_offset" in family:
                        arguments.extend(["--food-offset-x", family["food_offset"][0],
                                          "--food-offset-z", family["food_offset"][1]])
                    receipt = self.command(self.care, arguments, lane)
                    records.append({"task": task, "receipt": receipt})
                    self.frozen[str(output)] = raw_tree(output)
                    previous = output
                    trained.append(task["seed"])
                heldout = [self.assessment(previous, lane / "holdout" / f"{i:02d}", task, families, trained, assessments)
                           for i, task in enumerate(bank["holdouts"])]
                result["arms"][arm] = {"requested_training_decisions": self.slots*self.decisions,
                    "training": records, "heldouts": heldout,
                    "heldout_successes": sum(row["success"] for row in heldout)}
        else:
            # A measured fixed-budget frozen reference is still useful when no
            # adaptive training arm is justified. These are assessment-only
            # seeds now; they are never silently reused for training.
            reference_seeds = assessments | {task["seed"] for task in plan["baseline"]}
            result["fixed_frozen_reference"] = [self.assessment(source,
                self.root / "fixed-reference" / f"{i:02d}", task, families, [881001], reference_seeds)
                for i, task in enumerate(plan["baseline"])]
        # Always record genuinely held-out frozen-source observations, including
        # the negative result when discovery did not justify adaptive dispatch.
        result["initial_holdouts"] = [self.assessment(source, self.root / "initial-holdout" / f"{i:02d}", task,
            families, [881001], assessments) for i, task in enumerate(bank["holdouts"])]
        result["commands"] = self.calls
        self.guard()
        write_new(self.root / "pilot-result.json", result)
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--care-binary", type=Path, required=True)
    parser.add_argument("--round-binary", type=Path, required=True)
    parser.add_argument("--decisions", type=int, default=64)
    parser.add_argument("--discovery-per-family", type=int, default=4)
    parser.add_argument("--slots", type=int, default=4)
    parser.add_argument("--bank", type=Path, help="validated generated bank")
    parser.add_argument("--discovery", type=Path, help="separate disjoint family/seed list")
    args = parser.parse_args()
    if bool(args.bank) != bool(args.discovery):
        parser.error("--bank and --discovery must be supplied together")
    pilot = Pilot(args.output.absolute(), args.care_binary, args.round_binary,
                  args.decisions, args.discovery_per_family, args.slots,
                  read_json(args.bank, 1024**2) if args.bank else None,
                  read_json(args.discovery, 1024**2) if args.discovery else None)
    try:
        result = pilot.run()
    except (ValueError, KeyError, TypeError, OSError, subprocess.TimeoutExpired) as error:
        parser.exit(1, f"{error}; preserve all existing output, no automatic retry\n")
    print(json.dumps({"output": str(pilot.root), "status": result["status"], "statistics": result["statistics"]}, sort_keys=True))


if __name__ == "__main__":
    main()

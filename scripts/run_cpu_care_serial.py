#!/usr/bin/env python3
"""Explicit CPU care schedule; native CLI owns reservation, learning and acceptance.

Planning is read-only. --run creates a dedicated CPU lane; --resume continues its
saved plan. Accepted episodes advance the explicit schedule, never a curriculum
promotion. Failed/interrupted seeds remain excluded and are never retried.
"""
import argparse
import json
import math
import os
from pathlib import Path
import subprocess

import grab_food_seed_reservations as seeds
import run_mixed_care_serial as mixed

LEDGER = ".grab-food-seed-reservations.json"
PLAN = ".cpu-care-plan.json"
HELPERS = ("run_mixed_care_serial.py", "grab_food_seed_reservations.py",
           "run_eat_held_food_night.py", "summarize_ppo_diagnostics.py")
require = mixed.require


def pins(source):
    receipts = [name for name in ("cycle.json", "cpu-cycle.json", "group50.json")
                if (source / name).is_file()]
    require(len(receipts) == 1, "source requires exactly one native receipt")
    return {name: mixed.host.sha256(source / name) for name in
            [receipts[0], "trained.alife-foundation", "actor-checkpoint.json", "value-checkpoint.json"]}


def read(path):
    return mixed.inspector_io.load_json(path, seeds.MAX_DOCUMENT_BYTES)[0]


def command(binary, *args):
    result = subprocess.run([binary, *map(str, args)], check=True, text=True, capture_output=True)
    return json.loads(result.stdout)


def build_plan(args):
    source = args.previous.resolve(strict=True)
    lane = args.lane.resolve()
    require(source != lane and source not in lane.parents and lane not in source.parents,
            "CPU lane and immutable source must be separate trees")
    require(not lane.exists() or not any(lane.iterdir()), "new CPU lane must be empty")
    schedule = read(args.schedule)
    rows = mixed.schedule_rows(schedule)  # Reuse the native explicit three-lesson/dose grammar.
    first, last = map(int, args.seeds.split(":"))
    require(0 < first <= last <= seeds.MAX_SEED, "invalid explicit seed range")
    require(last - first + 1 >= len(rows), "seed range cannot cover the explicit episode schedule")
    require(math.isfinite(args.temperature) and args.temperature > 0, "invalid temperature")
    history = args.seed_history.resolve(strict=True)
    document = read(history)
    admission = {"policy": "known-exclusions" if args.known_exclusions else "strict",
                 "historical_freshness": "unknown" if args.known_exclusions else "operator-attested-complete"}
    require(mixed.seed_admission(document) == admission and
            document.get("complete") is (not args.known_exclusions), "history admission mismatch")
    source_pins = pins(source)
    require(document.get("source_sha256") == source_pins, "history is not bound to initial source")
    binary = args.binary.resolve(strict=True)
    root = Path(__file__).resolve().parent.parent
    git = lambda *a: subprocess.check_output(["git", *a], cwd=root, text=True).strip()
    require(not git("status", "--porcelain"), "run planning requires a clean source checkout")
    return {"schema": 1, "kind": "native-cpu-care-plan", "source": str(source), "source_sha256": source_pins,
            "lane": str(lane), "history": str(history), "history_sha256": mixed.host.sha256(history),
            "binary": str(binary), "binary_sha256": mixed.host.sha256(binary), "schedule": schedule,
            "seed_range": [first, last], "temperature": args.temperature, "seed_admission": admission,
            "git_head": git("rev-parse", "HEAD"), "git_tree": git("rev-parse", "HEAD^{tree}"),
            "scheduler_sha256": mixed.host.sha256(Path(__file__)),
            "helper_sha256": {name: mixed.host.sha256(root / "scripts" / name) for name in HELPERS},
            "cargo_lock_sha256": mixed.host.sha256(root / "Cargo.lock"),
            "prior_environment": {k: v for k, v in os.environ.items() if k.startswith("ALIFE_SLM_")}}


def episode_history(plan, previous, attempts, lane):
    history = Path(plan["history"])
    require(mixed.host.sha256(history) == plan["history_sha256"], "initial history changed")
    doc = read(history)
    doc["source_sha256"] = pins(previous)
    receipt_name = next(name for name in doc["source_sha256"] if name.endswith(".json") and
                        name not in ("actor-checkpoint.json", "value-checkpoint.json"))
    source_seed = read(previous / receipt_name)["seed"]
    doc["consumed_seed_ranges"] = seeds._ranges(doc["consumed_seed_ranges"] +
                                               [[a["seed"], a["seed"]] for a in attempts] +
                                               [[source_seed, source_seed]])
    ledger_path = lane / LEDGER
    doc["imports"] = [item for item in doc["imports"] if Path(item["path"]).resolve() != ledger_path]
    if ledger_path.exists():
        doc["imports"].append({"path": str(ledger_path), "sha256": mixed.host.sha256(ledger_path)})
    return doc


def verify_source(plan):
    root = Path(__file__).resolve().parent.parent
    git = lambda *a: subprocess.check_output(["git", *a], cwd=root, text=True).strip()
    require(not git("status", "--porcelain") and git("rev-parse", "HEAD") == plan["git_head"]
            and git("rev-parse", "HEAD^{tree}") == plan["git_tree"], "pinned clean source checkout changed")
    require(mixed.host.sha256(root / "Cargo.lock") == plan["cargo_lock_sha256"], "pinned Cargo.lock changed")
    require(plan["helper_sha256"] == {name: mixed.host.sha256(root / "scripts" / name) for name in HELPERS},
            "pinned scheduling/admission helpers changed")


def execute(plan):
    lane = Path(plan["lane"])
    binary = plan["binary"]
    require(plan["schema"] == 1 and plan["kind"] == "native-cpu-care-plan", "invalid saved CPU plan")
    verify_source(plan)
    require(mixed.host.sha256(binary) == plan["binary_sha256"], "pinned CPU binary changed")
    require(mixed.host.sha256(Path(__file__)) == plan["scheduler_sha256"], "pinned scheduler changed")
    require({k: v for k, v in os.environ.items() if k.startswith("ALIFE_SLM_")} == plan["prior_environment"],
            "semantic-prior environment changed")
    require(pins(Path(plan["source"])) == plan["source_sha256"], "immutable starting source changed")
    rows = mixed.schedule_rows(plan["schedule"])
    previous = Path(plan["source"])
    ledger_path = lane / LEDGER
    attempts = read(ledger_path)["cpu_care_attempts"] if ledger_path.exists() else []
    if attempts and attempts[-1]["status"] == "pending":
        command(binary, "--reconcile-pending", lane)
        attempts = read(ledger_path)["cpu_care_attempts"]
    slot = 0
    for index, attempt in enumerate(attempts):
        require(slot < len(rows) and attempt["seed"] == plan["seed_range"][0] + index and
                attempt["output"] == str(lane / f"episode-{index:05d}") and
                attempt["source_sha256"] == pins(previous) and
                attempt["lesson"] == rows[slot]["lesson"] and attempt["decisions"] == rows[slot]["decisions"],
                "saved CPU attempt does not match explicit schedule/lineage")
        if attempt["status"] == "accepted":
            output = Path(attempt["output"])
            require(pins(output) == attempt["native_output_sha256"], "accepted CPU output changed")
            command(binary, "--validate-output", output)
            previous = output
            slot += 1
        else:
            require(attempt["status"] == "failed" and not (Path(attempt["output"]) / "cpu-cycle.json").exists(),
                    "uncertain CPU attempt requires reconciliation")
    while slot < len(rows):
        require(not (lane / "STOP").exists(), "CPU owner STOP requested")
        require(mixed.host.sha256(binary) == plan["binary_sha256"], "pinned CPU binary changed")
        index = len(attempts)
        seed = plan["seed_range"][0] + index
        require(seed <= plan["seed_range"][1], "explicit seed range exhausted; no seed reuse")
        output = lane / f"episode-{index:05d}"
        require(not output.exists(), "CPU output already exists")
        history = lane / f"history-{index:05d}.json"
        document = episode_history(plan, previous, attempts, lane)
        if history.exists():
            require(read(history) == document, "existing dispatch history differs; preserve for reconciliation")
        else:
            seeds.write_document(history, document)
        row = rows[slot]
        argv = ["--previous", previous, "--output", output, "--seed-history", history,
                "--seed", seed, "--lesson", row["lesson"], "--decisions", row["decisions"],
                "--temperature", plan["temperature"]]
        if plan["seed_admission"]["policy"] == "known-exclusions":
            argv.append("--known-exclusions")
        command(binary, *argv)
        receipt = command(binary, "--validate-output", output)
        attempts = read(ledger_path)["cpu_care_attempts"]
        require(attempts[-1]["status"] == "accepted" and
                attempts[-1]["native_output_sha256"] == pins(output), "native acceptance missing")
        print(json.dumps({"episode": slot, "output": str(output), "policy_version": receipt["policy_version"],
                          "trained_nourished_meals": receipt["episode"]["trained_nourished_meals"],
                          "lesson_completed": receipt["episode"]["lesson_completed"]}), flush=True)
        previous = output
        slot += 1
    return {"complete": True, "latest_checkpoint": str(previous), "accepted_episodes": slot,
            "promoted": False, "terminal_gpu_individual_restore_proven": False,
            "seed_admission": plan["seed_admission"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--resume", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--previous", type=Path)
    parser.add_argument("--lane", type=Path)
    parser.add_argument("--seed-history", type=Path)
    parser.add_argument("--schedule", type=Path)
    parser.add_argument("--seeds", help="explicit nonzero inclusive FIRST:LAST range")
    parser.add_argument("--temperature", type=float, default=1.)
    parser.add_argument("--known-exclusions", action="store_true")
    parser.add_argument("--run", action="store_true")
    args = parser.parse_args()
    if args.resume:
        require(args.run and not any((args.binary, args.previous, args.lane, args.seed_history, args.schedule, args.seeds,
                                      args.known_exclusions)), "resume uses only --resume LANE --run")
        lane = args.resume.resolve(strict=True)
        plan = read(lane / PLAN)
        require(plan["lane"] == str(lane), "saved lane identity changed")
    else:
        require(all((args.binary, args.previous, args.lane, args.seed_history, args.schedule, args.seeds)),
                "new plan requires binary, source, lane, history, schedule and explicit seeds")
        plan = build_plan(args)
        lane = Path(plan["lane"])
        if not args.run:
            print(json.dumps(plan, indent=2))
            return
        lane.mkdir(parents=True, exist_ok=True)
    lock = lane / ".cpu-care-scheduler.lock"
    descriptor = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    try:
        os.write(descriptor, str(os.getpid()).encode())
        os.fsync(descriptor)
        if not args.resume:
            require(not (lane / PLAN).exists() and not (lane / LEDGER).exists(), "CPU lane acquired another plan")
            seeds.write_document(lane / PLAN, plan)
        print(json.dumps(execute(plan), indent=2))
    finally:
        os.close(descriptor)
        lock.unlink()


if __name__ == "__main__":
    main()

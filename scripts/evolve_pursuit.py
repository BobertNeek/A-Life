#!/usr/bin/env python3
"""PyTorch-driven genetic weight search, scored only by the native CPU world.

No surrogate simulation, gradients, or teacher policy controls the creatures.
The six search coordinates are coherent groups of real decoder weight edits.
"""
import argparse
import json
import pathlib
import subprocess
import time

import torch


def summary(result):
    groups = {}
    score = 0.0
    for trial in result["trials"]:
        group = groups.setdefault(trial["scene"], {"n": 0, "consumed": 0, "caught": 0, "failed_ingests": 0})
        group["n"] += 1
        group["consumed"] += int(trial["consumed"])
        group["caught"] += int(trial["caught_tick"] is not None)
        group["failed_ingests"] += trial["failed_ingests"]
        if trial["scene"] == "toy":
            score -= 0.2 * trial["failed_ingests"]
        else:
            score += 100 * trial["consumed"] + 5 * (trial["caught_tick"] is not None)
            score += max(0, 2.5 - trial["min_distance"])
            if trial["consumed_tick"] is not None:
                score += 2 * (1 - trial["consumed_tick"] / result["ticks"])
        score -= 20 * trial["injury"]
    return {"score": score / len(result["trials"]), "scenes": groups, "seconds": result["seconds"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=pathlib.Path)
    parser.add_argument("foundation", type=pathlib.Path)
    parser.add_argument("output", type=pathlib.Path)
    parser.add_argument("--generations", type=int, default=3)
    parser.add_argument("--population", type=int, default=8)
    parser.add_argument("--seeds", type=int, default=3)
    parser.add_argument("--ticks", type=int, default=80)
    parser.add_argument("--seed-start", type=int, default=2026111300)
    parser.add_argument("--search-seed", type=int, default=13)
    parser.add_argument("--scenes", default="contact,stationary,receding,crossing,toy")
    parser.add_argument("--start-params", default="0,0,0,0,0,0")
    parser.add_argument("--stop-file", type=pathlib.Path)
    parser.add_argument("--allowance-file", type=pathlib.Path,
                        help='Parent-maintained JSON {"remaining_percent": number}; stop at <=40')
    args = parser.parse_args()
    if args.generations < 0 or args.population < 2:
        parser.error("generations >=0 and population >=2 required")
    args.output.mkdir(parents=True, exist_ok=False)
    torch.set_num_threads(1)
    generator = torch.Generator().manual_seed(args.search_seed)
    center = torch.tensor([float(x) for x in args.start_params.split(",")])
    if center.shape != (6,) or not torch.isfinite(center).all():
        parser.error("six finite starting parameters required")
    scale = torch.tensor([1., 3., 3., 6., 6., .5])
    stop_reason = None

    def stopping():
        nonlocal stop_reason
        if args.stop_file and args.stop_file.exists():
            stop_reason = "stop file"
        if args.allowance_file and args.allowance_file.exists():
            allowance = json.loads(args.allowance_file.read_text())
            if float(allowance["remaining_percent"]) <= 40:
                stop_reason = "allowance <=40% remaining (parent observation)"
        return stop_reason is not None

    manifest = {**vars(args), "torch_version": torch.__version__, "time_started": time.time(),
                "usage_directly_visible": False, "method": "elitist antithetic Gaussian search"}
    (args.output / "manifest.json").write_text(json.dumps(manifest, default=str, indent=2))
    proc = subprocess.Popen([str(args.binary.resolve()), str(args.foundation.resolve())],
                            stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    counter = 0
    best = None

    def evaluate(params, label):
        nonlocal counter
        request = {"params": params.tolist(), "seed_start": args.seed_start, "seeds": args.seeds,
                   "ticks": args.ticks, "scenes": args.scenes.split(","), "trace": False}
        proc.stdin.write(json.dumps(request) + "\n")
        proc.stdin.flush()
        raw = proc.stdout.readline()
        if not raw:
            raise RuntimeError(f"native worker exited: {proc.poll()}")
        result = json.loads(raw)
        if "error" in result:
            raise RuntimeError(result["error"])
        filename = f"{counter:04d}-{label}.json"
        (args.output / filename).write_text(json.dumps(result, separators=(",", ":")))
        item = {"params": params.tolist(), "file": filename, **summary(result)}
        with (args.output / "search.jsonl").open("a") as log:
            log.write(json.dumps(item) + "\n")
        print(json.dumps(item), flush=True)
        counter += 1
        return item

    try:
        if not stopping():
            best = evaluate(center, "baseline")
            (args.output / "best.json").write_text(json.dumps(best, indent=2))
        for generation in range(args.generations):
            if stopping():
                break
            noise = torch.randn(((args.population + 1) // 2, 6), generator=generator)
            population = torch.cat((noise, -noise))[:args.population]
            # Selection sees the same scenes and RNG seeds for every challenger.
            proposals = (center + population * scale).clamp(-64, 64)
            for index, proposal in enumerate(proposals):
                if stopping():
                    break
                item = evaluate(proposal, f"g{generation}-m{index}")
                if item["score"] > best["score"]:
                    best = item
                    (args.output / "best.json").write_text(json.dumps(best, indent=2))
            center = torch.tensor(best["params"])
            scale *= 0.8
    finally:
        proc.stdin.close()
        proc.wait()
        (args.output / "status.json").write_text(json.dumps({"evaluations": counter,
            "stop_reason": stop_reason or "requested generations complete", "best": best}, indent=2))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Plan a frozen 2-vs-8 epoch diagnostic; execution requires --run."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

LESSONS = (
    "feeding", "hazard_avoidance", "obstacle_navigation", "recovery",
    "vision_search", "maze_navigation", "vocabulary_reception", "vocabulary_production",
)


def snapshot(paths, deadline):
    result = {}
    for path in sorted(paths):
        digest = hashlib.sha256()
        with path.open("rb") as source:
            while chunk := source.read(1024 * 1024):
                if time.monotonic() >= deadline:
                    raise TimeoutError("40-minute diagnostic cap reached during input verification")
                digest.update(chunk)
        result[str(path)] = digest.hexdigest()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--source-receipt", type=Path, required=True,
                        help="Reviewed build receipt identifying the binary's source and build command")
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="New, absent directory")
    parser.add_argument("--world-seed", type=int, required=True)
    parser.add_argument("--founder-seed", type=int, required=True)
    parser.add_argument("--eval-ticks", type=int, required=True)
    parser.add_argument("--request-token", type=int, required=True)
    parser.add_argument("--run", action="store_true", help="Execute the planned GPU commands locally")
    args = parser.parse_args()
    if not (0 < args.world_seed < 2**64 and 0 < args.founder_seed < 2**64
            and args.eval_ticks > 0 and 0 < args.request_token < 256):
        parser.error("seeds, tick count and request token must be valid positive values")
    deadline = time.monotonic() + 40 * 60
    binary, manifest_path, receipt = (p.resolve(strict=True) for p in
                                     (args.binary, args.manifest, args.source_receipt))
    output = args.output.resolve()
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest["founder_seed_base"] != args.founder_seed:
        parser.error("evaluation founder seed must match the frozen corpus founder seed")
    roots = [(manifest_path.parent / p).resolve(strict=True) for p in manifest["pilots"]]
    if any(root == output or root in output.parents for root in roots):
        parser.error("output must be outside every frozen pilot directory")
    paths = {binary, manifest_path, receipt}
    if manifest.get("source_asset"):
        paths.add((manifest_path.parent / manifest["source_asset"]).resolve(strict=True))
    for root in roots:
        paths.update(p.resolve(strict=True) for p in root.rglob("*") if p.is_file())
    before = snapshot(paths, deadline)
    commands = []
    for epochs in (2, 8):
        arm = output / f"epochs-{epochs}"
        commands.append({"name": f"warmup-{epochs}", "prior_off": False,
                         "argv": [str(binary), "--warmup", str(arm), str(manifest_path), str(epochs)]})
        for lesson in LESSONS:
            argv = [str(binary), "--evaluate", str(output / f"eval-{epochs}-{lesson}"),
                    str(arm / "trained.alife-foundation"), lesson, str(args.eval_ticks),
                    str(args.world_seed), str(args.founder_seed)]
            if lesson.startswith("vocabulary_"):
                argv += ["--request-token", str(args.request_token)]
            commands.append({"name": f"eval-{epochs}-{lesson}", "prior_off": True, "argv": argv})
    plan = {"status": "planned", "cap_seconds": 2400, "inputs_sha256": before,
            "warmup_prior_mode": os.environ.get("ALIFE_SLM_PRIOR"),
            "warmup_prior_model_sha256": os.environ.get("ALIFE_SLM_PRIOR_MODEL_SHA256"),
            "commands": commands, "completed_commands": [],
            "comparison_complete": False, "promoted": False}
    output.mkdir(parents=True, exist_ok=False)
    plan_path = output / "dose-plan.json"

    def save():
        plan_path.write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")

    save()
    if not args.run:
        print(f"Plan only: {plan_path}")
        return
    plan["status"] = "running"
    save()
    env = os.environ.copy()
    try:
        for command in commands:
            if snapshot(paths, deadline) != before:
                raise RuntimeError("frozen binary/corpus/source receipt changed")
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("40-minute diagnostic cap reached")
            child_env = env.copy()
            if command["prior_off"]:
                child_env["ALIFE_SLM_PRIOR"] = "off"
            with (output / f"{command['name']}.log").open("wb") as log:
                # subprocess.run kills and waits for its own child on timeout.
                subprocess.run(command["argv"], env=child_env, stdout=log,
                               stderr=subprocess.STDOUT, timeout=remaining, check=True)
            if snapshot(paths, deadline) != before:
                raise RuntimeError("frozen input changed during the command")
            plan["completed_commands"].append(command["name"])
            save()
        plan["comparison_complete"] = True
        plan["status"] = "completed"
    except Exception as error:
        plan["status"] = "incomplete"
        plan["error"] = str(error)
        raise
    finally:
        save()


if __name__ == "__main__":
    main()

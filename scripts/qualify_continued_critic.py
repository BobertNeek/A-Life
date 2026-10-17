#!/usr/bin/env python3
"""Bounded disposable continuation; never starts or adopts a production owner."""
import argparse
import csv
import ctypes
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import time

from summarize_ppo_diagnostics import run_inspector, decode_json


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def file_map(root):
    return {str(p.relative_to(root)): sha(p) for p in root.rglob("*") if p.is_file()}


def write(path, value):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, allow_nan=False), encoding="utf-8")
    temporary.replace(path)


def check_slices(before, after):
    """Separate gates prevent a short transient disappearing in a long mean."""
    for label in ["early16", "whole_trajectory", "last16"]:
        initial = before[label]["residual_mse"]
        observed = after[label]["residual_mse"]
        limit = max(4 * initial, 1.0)
        if not math.isfinite(observed) or observed > limit:
            raise ValueError(f"{label} residual MSE {observed} exceeds predeclared {limit}")
        if after[label]["out_of_support_predictions"]:
            raise ValueError(f"{label} has out-of-support predictions")


def validate_admission(admitted, cycle):
    if admitted.get("neural_execution") is not False or admitted.get("weights_modified") is not False or (
        admitted["policy_version"], admitted["actor_optimizer_step"], admitted["value_optimizer_step"]
    ) != (cycle["policy_version"], cycle["actor_optimizer_step"], cycle["value_optimizer_step"]):
        raise ValueError("Native checkpoint admission differs from the sealed cycle")


def workers():
    names = ["train_n2048_care.exe", "qualify_critic_residual.exe", "critic_gpu_regression.exe"]
    if os.name != "nt":
        raise RuntimeError("Native resource/ownership runner currently requires Windows")
    found = []
    for name in names:
        result = run_inspector(["tasklist", "/FI", "IMAGENAME eq " + name, "/FO", "CSV", "/NH"], timeout=5)
        result.check_returncode()
        for row in csv.reader(result.stdout.decode("utf-8", errors="replace").splitlines()):
            if row and row[0].lower() == name:
                found.append(row[:2])
    return found


def resources(disk):
    class Memory(ctypes.Structure):
        _fields_ = [("length", ctypes.c_ulong), ("load", ctypes.c_ulong)] + [
            (name, ctypes.c_ulonglong) for name in ["total", "available", "page_total", "page_available",
                                                 "virtual_total", "virtual_available", "extended"]]
    memory = Memory(); memory.length = ctypes.sizeof(memory)
    if not ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(memory)):
        raise OSError("GlobalMemoryStatusEx failed")
    result = run_inspector(["nvidia-smi", "--query-gpu=memory.free,temperature.gpu",
                            "--format=csv,noheader,nounits"], timeout=5)
    result.check_returncode()
    gpu = [tuple(map(int, row.split(","))) for row in result.stdout.decode().strip().splitlines()]
    record = dict(RAM_free_GiB=memory.available/2**30, disk_free_GiB=shutil.disk_usage(disk).free/2**30,
                  VRAM_free_MiB=min(x[0] for x in gpu), GPU_temperature_C=max(x[1] for x in gpu))
    if record["RAM_free_GiB"] < 12 or record["disk_free_GiB"] < 15 or record["VRAM_free_MiB"] < 2048 or record["GPU_temperature_C"] >= 80:
        raise RuntimeError(f"Resource reserve not met: {record}")
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["source", "output", "trainer", "evaluator", "inspector"]:
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--deadline-utc", required=True)
    parser.add_argument("--cycles", type=int, default=128)
    parser.add_argument("--first-seed", type=int, required=True)
    parser.add_argument("--fresh-context-seeds", type=int, nargs=2, required=True)
    parser.add_argument("--acceptance-seeds", type=int, nargs=2, required=True)
    parser.add_argument("--reserve-disk", type=Path, default=Path("D:/"))
    parser.add_argument("--old-trainer", type=Path)
    parser.add_argument("--old-source-repository", type=Path)
    parser.add_argument("--protected-checkpoint", type=Path, action="append", default=[])
    args = parser.parse_args()
    if args.old_trainer and not args.old_source_repository:
        parser.error("The legacy control needs its original source checkout for truthful revision receipts")
    deadline = dt.datetime.fromisoformat(args.deadline_utc)
    if deadline.tzinfo is None or not 1 <= args.cycles <= 128:
        parser.error("Require an aware deadline and 1..128 disposable cycles")
    reserved_contexts = [*args.fresh_context_seeds, *args.acceptance_seeds]
    if args.first_seed <= 0 or len(set(reserved_contexts)) != 4 or any(seed <= 0 or
        args.first_seed <= seed < args.first_seed + args.cycles for seed in reserved_contexts):
        parser.error("Require positive, distinct nonoverlapping training/context seeds")
    if args.output.exists():
        parser.error("Output must be new; preserve previous attempts")
    if workers():
        raise RuntimeError("A native training/evaluation worker already exists")
    protected = {str(p): file_map(p) for p in [args.source, *args.protected_checkpoint]}
    binaries = {str(p): sha(p) for p in [args.trainer, args.evaluator, args.inspector]}
    if args.old_trainer: binaries[str(args.old_trainer)]=sha(args.old_trainer)
    args.output.mkdir(parents=True)
    (args.output / "episodes").mkdir()
    source = args.output / "source-clone"
    shutil.copytree(args.source, source)
    if file_map(source) != protected[str(args.source)]:
        raise ValueError("Source clone differs")
    source_cycle = read(source / "cycle.json")
    source_actor = read(source / "actor-checkpoint.json")
    source_value = read(source / "value-checkpoint.json")
    stages = [n for n in [1, 4, 16, 64, 128] if n <= args.cycles]
    request = dict(schema=1, purpose="disposable_continued_residual_critic_qualification", cycles=args.cycles,
        first_seed=args.first_seed, fresh_context_seeds=args.fresh_context_seeds,
        untouched_final_acceptance_seeds=args.acceptance_seeds, checkpoints=stages,
        deadline_utc=deadline.isoformat(), early16_whole128_last16_gate="each MSE <= max(4*baseline,1.0)",
        Eating_seed=202610169901, Grab_seed=202610169902, founder=539363617, requested_probe_decisions=16,
        source=str(args.source), source_policy=source_cycle["policy_version"], source_actor_age=source_actor["optimizer_step"],
        source_critic_age=source_value["optimizer_step"], protected=protected, binaries=binaries,
        source_sha256=file_map(source), production_adoption_permitted=False)
    write(args.output / "PLAN.json", request)
    os.environ.update(ALIFE_SLM_PRIOR_BACKEND="deterministic", ALIFE_SLM_PRIOR="on", ALIFE_TRAINING_STAGE="0", ALIFE_PPO_DIAGNOSTICS="1")
    commands = []
    def remaining():
        return (deadline-dt.datetime.now(dt.timezone.utc)).total_seconds()
    def call(label, argv, gpu=True):
        if remaining() < 100:
            raise TimeoutError("Preserve cleanup reserve before native deadline")
        if gpu:
            if workers(): raise RuntimeError("GPU lane occupied")
            headroom = resources(args.reserve_disk)
        else: headroom = None
        started = time.monotonic()
        result = run_inspector([str(x) for x in argv], timeout=min(90, remaining()-10))
        (args.output/(label+".stdout.log")).write_bytes(result.stdout)
        (args.output/(label+".stderr.log")).write_bytes(result.stderr)
        record = dict(label=label, argv=[str(x) for x in argv], seconds=time.monotonic()-started,
                      exit_code=result.returncode, resources=headroom)
        commands.append(record); write(args.output/"COMMANDS.json", commands)
        result.check_returncode()
        if workers(): raise RuntimeError("Native child did not leave the GPU lane")
        if any(sha(p)!=digest for p,digest in binaries.items()): raise ValueError("Pinned binary changed")
        return result
    def probe(checkpoint, number):
        results = {}
        for lesson, seed in [("eat_held_food",202610169901),("grab_food",202610169902)]:
            target = args.output/(f"{lesson}-{number:03d}")
            call(target.name, [args.trainer,"--evaluate",target,checkpoint/"trained.alife-foundation",lesson,"16",str(seed),"539363617"])
            pilot = read(target/"pilot.json")
            if pilot["seed"] != seed: raise ValueError("Probe seed differs")
            # Actual decision count may be8 after early consumption, despite a cap16.
            results[lesson] = dict(pilot=pilot, directory=str(target))
        return results
    outcome = dict(status="started", accepted_cycles=0, milestones=[], source=str(source))
    try:
        call("gradient-regression",[args.evaluator,"--gradient-regression"])
        baseline_probes = probe(source,0)
        baseline_eating = baseline_probes["eat_held_food"]["pilot"].get("consumed_events",0)
        if baseline_eating < 1: raise ValueError("Matched source Eating baseline failed")
        contexts = []
        for seed in args.fresh_context_seeds:
            capture = args.output/f"fresh-context-{seed}"
            call(capture.name,[args.evaluator,"--capture-summary",source,capture,str(seed),"128","32"])
            baseline = args.output/f"fresh-baseline-{seed}.json"
            call(baseline.stem,[args.evaluator,"--evaluate-capture",capture,source/"value-checkpoint.json",source/"value-checkpoint.json",baseline])
            contexts.append((seed,capture,read(baseline)))
        legacy_actor_sha=None
        if args.old_trainer:
            legacy=args.output/"legacy-control-cycle1"
            current_directory=Path.cwd()
            try:
                os.chdir(args.old_source_repository)
                call(legacy.name,[args.old_trainer,"--resume-cycle",source,legacy,"16","--seed",str(args.first_seed),"--lesson","grab_food","--sampling-temperature","32"])
            finally:
                os.chdir(current_directory)
            legacy_actor_sha=sha(legacy/"actor-checkpoint.json")
        def milestone(checkpoint,number):
            observations=[]
            for context_seed,capture,baseline in contexts:
                report=args.output/f"context-{context_seed}-cycle-{number:03d}.json"
                call(report.stem,[args.evaluator,"--evaluate-capture",capture,source/"value-checkpoint.json",checkpoint/"value-checkpoint.json",report])
                observed=read(report)
                if (observed["feature_sha256"],observed["capture_sha256"]) != (baseline["feature_sha256"],baseline["capture_sha256"]):
                    raise ValueError("Fixed context differs")
                check_slices(baseline["after"],observed["after"])
                observations.append(dict(seed=context_seed,metrics=observed["after"]))
            probes=probe(checkpoint,number)
            if probes["eat_held_food"]["pilot"].get("consumed_events",0)<baseline_eating:
                raise ValueError("Matched Eating retention regressed")
            outcome["milestones"].append(dict(cycle=number,contexts=observations,probes=probes))
            write(args.output/"PROGRESS.json",outcome)
            print(json.dumps(dict(cycle=number,status="slice_and_retention_gates_passed")),flush=True)
        latest = source
        for number in range(1,args.cycles+1):
            if remaining()<300:
                outcome["collection_stopped_for_final_validation_reserve"]=True
                break
            target = args.output/"episodes"/f"cycle-{number:03d}"
            seed = args.first_seed+number-1
            call(target.name,[args.trainer,"--resume-cycle",latest,target,"16","--seed",str(seed),"--lesson","grab_food","--sampling-temperature","32"])
            c = read(target/"cycle.json"); value = read(target/"value-checkpoint.json")
            if number==1 and legacy_actor_sha is not None:
                if sha(target/"actor-checkpoint.json")!=legacy_actor_sha:
                    raise ValueError("Matched old/new actor weights or full Adam differ")
                outcome["matched_legacy_actor_checkpoint_exact"]=True
            if (c["policy_version"],c["actor_optimizer_step"],c["value_optimizer_step"]) != (
                source_cycle["policy_version"]+number,source_actor["optimizer_step"]+2*number,source_value["optimizer_step"]+2*number):
                raise ValueError("Policy/Adam continuity differs")
            if (c["trained_replay_rows"],c["captured_replay_rows"],c["bootstrap_replay_rows"],c["critic_objective_version"])!=(16,17,1,1):
                raise ValueError("Wrong row counts or continued critic objective")
            if c["critic_residual_update_sha256"]!=sha(target/"critic-residual-update.json"):
                raise ValueError("Residual update evidence differs")
            admission=call("admit-"+str(number),[args.inspector,"--validate-checkpoint",target,"--expected-policy",str(c["policy_version"])],gpu=False)
            admitted=decode_json(admission.stdout,"checkpoint admission")
            validate_admission(admitted,c)
            latest=target
            outcome.update(accepted_cycles=number,latest_checkpoint=str(latest),policy=c["policy_version"],actor_age=c["actor_optimizer_step"],critic_age=value["optimizer_step"])
            write(args.output/"PROGRESS.json",outcome)
            if number in stages:
                milestone(latest,number)
        if not outcome["accepted_cycles"]:raise TimeoutError("No disposable cycle fit before the final validation reserve")
        if not outcome["milestones"] or outcome["milestones"][-1]["cycle"]!=outcome["accepted_cycles"]:
            milestone(latest,outcome["accepted_cycles"])
        acceptance=[]
        for seed in args.acceptance_seeds:
            capture=args.output/f"acceptance-context-{seed}"
            call(capture.name,[args.evaluator,"--capture-summary",source,capture,str(seed),"128","32"])
            baseline=args.output/f"acceptance-baseline-{seed}.json"
            call(baseline.stem,[args.evaluator,"--evaluate-capture",capture,source/"value-checkpoint.json",source/"value-checkpoint.json",baseline])
            report=args.output/f"acceptance-final-{seed}.json"
            call(report.stem,[args.evaluator,"--evaluate-capture",capture,source/"value-checkpoint.json",latest/"value-checkpoint.json",report])
            initial=read(baseline);observed=read(report)
            if observed["feature_sha256"]!=initial["feature_sha256"]:raise ValueError("Acceptance context differs")
            check_slices(initial["after"],observed["after"])
            current_capture=args.output/f"current-actor-context-{seed}"
            call(current_capture.name,[args.evaluator,"--capture-summary",latest,current_capture,str(seed),"128","32"])
            current_report=args.output/f"current-actor-{seed}.json"
            call(current_report.stem,[args.evaluator,"--evaluate-capture",current_capture,latest/"value-checkpoint.json",latest/"value-checkpoint.json",current_report])
            current=read(current_report)
            check_slices(initial["after"],current["after"])
            acceptance.append(dict(seed=seed,baseline=initial["after"],final_on_fixed_source_features=observed["after"],
                current_actor_rollout=current["after"],current_actor_policy=current["actor_policy"],
                context_limit="Current actor/world trajectories may differ; this is generalization evidence, not a causal actor-representation partition."))
        outcome["untouched_acceptance"]=acceptance
        outcome["status"]=("bounded_clone_qualification_complete_not_production_adopted" if outcome["accepted_cycles"]==128
                           else "short_clone_evidence_complete_extended_qualification_pending")
    except BaseException as error:
        outcome.update(status="qualification_held",error=str(error))
        raise
    finally:
        unchanged=all(file_map(Path(p))==mapping for p,mapping in protected.items())
        outcome.update(ended_utc=dt.datetime.now(dt.timezone.utc).isoformat(),protected_checkpoints_unchanged=unchanged,
                       native_workers_remaining=workers(),production_adopted=False)
        write(args.output/"RESULT.json",outcome)
        if not unchanged or outcome["native_workers_remaining"]: raise RuntimeError("Preservation/cleanup not verified")
    print(json.dumps(outcome,indent=2))


if __name__=="__main__":
    main()

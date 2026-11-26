#!/usr/bin/env python3
"""Fail-closed state machine for Qualia brain surgery rounds.

Enforces:
- Single shared active base bundle (no blind stacking R(n+1) onto unscored R(n)).
- At most one unscored patch in flight per Astra owner, bounded global backlog.
- Data-only patches (rejects unlanded changes under crates/ unless explicit override).
- Real progress = matched base-vs-candidate scorecard comparisons (never synthetic proxies).
- Reopen-verified backups on every promotion + 2-hour stall-fuse rollback directive.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
from typing import Any

STATE_SCHEMA = "alife.brain_surgery.state.v1"
BUNDLE_MANIFEST_SCHEMA = "alife.brain_surgery.base_bundle.v1"
PATCH_SCHEMA = "alife.brain_surgery.patch.v1"
REPORT_SCHEMA = "alife.brain_surgery.comparison_report.v1"
REOPEN_SCHEMA = "alife.brain_surgery.reopen_receipt.v1"
SCHEMA_VERSION = 1


class SurgeryLoopError(RuntimeError):
    """Fail-closed state machine invariant violation."""


def utc_now_iso(now_utc: str | None = None) -> str:
    if now_utc:
        parsed = parse_utc(now_utc)
        return parsed.isoformat().replace("+00:00", "Z")
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def parse_utc(value: str) -> dt.datetime:
    normalized = value.strip()
    if normalized.endswith("Z"):
        normalized = normalized[:-1] + "+00:00"
    parsed = dt.datetime.fromisoformat(normalized)
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=dt.timezone.utc)
    return parsed.astimezone(dt.timezone.utc)


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(65536)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


def read_json(path: pathlib.Path, label: str) -> dict[str, Any]:
    if not path.is_file():
        raise SurgeryLoopError(f"missing {label}: {path}")
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        raise SurgeryLoopError(f"invalid JSON in {label} ({path}): {exc}") from exc
    if not isinstance(data, dict):
        raise SurgeryLoopError(f"{label} must be a JSON object: {path}")
    return data


def write_json_atomic(path: pathlib.Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    raw = json.dumps(payload, indent=2, sort_keys=False) + "\n"
    fd, tmp_name = tempfile.mkstemp(prefix=".surgery-tmp-", dir=str(path.parent))
    tmp_path = pathlib.Path(tmp_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(raw)
        os.replace(tmp_path, path)
    finally:
        if tmp_path.exists():
            tmp_path.unlink(missing_ok=True)


def load_bundle_manifest(bundle_dir: pathlib.Path) -> tuple[dict[str, Any], str]:
    manifest_path = bundle_dir / "manifest.json"
    manifest = read_json(manifest_path, "bundle manifest")
    if manifest.get("schema") != BUNDLE_MANIFEST_SCHEMA or manifest.get("schema_version") != SCHEMA_VERSION:
        raise SurgeryLoopError(f"unsupported bundle manifest schema in {manifest_path}")
    for key in (
        "base_id",
        "foundation_file",
        "foundation_sha256",
        "foundation_digest",
        "dendritic_branches_file",
        "dendritic_branches_sha256",
        "scorecard_file",
        "scorecard_sha256",
    ):
        if key not in manifest:
            raise SurgeryLoopError(f"bundle manifest missing {key}: {manifest_path}")

    for file_key, hash_key in (
        ("foundation_file", "foundation_sha256"),
        ("dendritic_branches_file", "dendritic_branches_sha256"),
        ("scorecard_file", "scorecard_sha256"),
    ):
        target = bundle_dir / str(manifest[file_key])
        if not target.is_file():
            raise SurgeryLoopError(f"bundle file missing: {target}")
        if sha256_file(target) != manifest[hash_key]:
            raise SurgeryLoopError(f"bundle SHA-256 mismatch for {target}")
    return manifest, sha256_file(manifest_path)


def load_state(state_path: pathlib.Path) -> dict[str, Any]:
    state = read_json(state_path, "surgery state")
    if state.get("schema") != STATE_SCHEMA or state.get("schema_version") != SCHEMA_VERSION:
        raise SurgeryLoopError(f"unsupported surgery state schema in {state_path}")
    return state


def detect_git_sha(repo_root: pathlib.Path) -> str | None:
    try:
        proc = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=repo_root,
            capture_output=True,
            text=True,
            timeout=15,
            check=False,
        )
        if proc.returncode == 0:
            return proc.stdout.strip()
    except OSError:
        return None
    return None


def dirty_crates_files(repo_root: pathlib.Path) -> list[str]:
    proc = subprocess.run(
        ["git", "status", "--porcelain", "--", "crates/"],
        cwd=repo_root,
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    if proc.returncode != 0:
        raise SurgeryLoopError(f"git status failed in {repo_root}: {proc.stderr.strip()}")
    return [line for line in proc.stdout.splitlines() if line.strip()]


def compute_watchdog(state: dict[str, Any], now_utc: str | None = None) -> dict[str, Any]:
    now = parse_utc(now_utc) if now_utc else dt.datetime.now(dt.timezone.utc)
    last_score = parse_utc(str(state["last_real_score_utc"]))
    hours_since = max(0.0, (now - last_score).total_seconds() / 3600.0)
    fuse_hours = float(state.get("stall_fuse_hours", 2.0))
    in_flight = list(state.get("in_flight_patches", []))
    unscored = [p for p in in_flight if p.get("status") == "unscored"]
    scored = [p for p in in_flight if p.get("status") == "scored"]

    if hours_since >= fuse_hours:
        status = "ROLLBACK_REQUIRED"
        action = (
            f"No matched base-vs-candidate score recorded in {hours_since:.2f}h (>= {fuse_hours:.2f}h). "
            f"STOP editing tooling or running stock-creature proxy checks. Reset to known_good_git_sha="
            f"{state.get('known_good_git_sha')} and restore/verify active_base_dir={state.get('active_base_dir')} "
            f"with scripts/bootstrap_cloud_surgery.sh before proposing new edits."
        )
    elif unscored:
        status = "SCORE_QUEUE_BACKLOG"
        ids = ", ".join(str(p["patch_id"]) for p in unscored)
        action = (
            f"Score existing candidate(s) [{ids}] with brain_surgery_eval score before stacking new edits. "
            f"Proxy stock-creature checks do not count as progress."
        )
    else:
        status = "HEALTHY"
        action = (
            f"Ready for data-only patches targeting active_base_id={state.get('active_base_id')}. "
            f"Never chain edits without scoring the previous round."
        )

    return {
        "status": status,
        "hours_since_last_real_score": round(hours_since, 4),
        "stall_fuse_hours": fuse_hours,
        "last_real_score_utc": state["last_real_score_utc"],
        "active_base_id": state["active_base_id"],
        "active_base_dir": state["active_base_dir"],
        "active_base_score": state["active_base_score"],
        "active_base_drive_id": state.get("active_base_drive_id"),
        "known_good_git_sha": state.get("known_good_git_sha"),
        "unscored_patches": [p["patch_id"] for p in unscored],
        "scored_patches": [p["patch_id"] for p in scored],
        "required_action": action,
    }


def cmd_init(args: argparse.Namespace) -> dict[str, Any]:
    state_path = pathlib.Path(args.state).resolve()
    base_dir = pathlib.Path(args.base_dir).resolve()
    manifest, manifest_sha = load_bundle_manifest(base_dir)
    if args.require_reopen_verified and not manifest.get("reopen_verified", False):
        raise SurgeryLoopError(
            f"base bundle {base_dir} is not reopen_verified=true; run brain_surgery_eval verify-reopen first"
        )
    repo_root = pathlib.Path(args.repo_root).resolve()
    git_sha = args.git_sha or detect_git_sha(repo_root)
    eval_sha = sha256_file(pathlib.Path(args.eval_bin).resolve()) if args.eval_bin else None

    state = {
        "schema": STATE_SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "active_base_id": manifest["base_id"],
        "active_base_dir": str(base_dir),
        "active_base_manifest_sha256": manifest_sha,
        "active_base_foundation_digest": manifest["foundation_digest"],
        "active_base_dendritic_sha256": manifest["dendritic_branches_sha256"],
        "active_base_score": float(args.base_score),
        "active_base_drive_id": args.drive_id,
        "known_good_git_sha": git_sha,
        "known_good_eval_bin_sha256": eval_sha,
        "last_real_score_utc": utc_now_iso(args.now_utc),
        "stall_fuse_hours": float(args.stall_fuse_hours),
        "max_unscored_in_flight": int(args.max_unscored),
        "in_flight_patches": [],
        "promoted_history": [],
    }
    write_json_atomic(state_path, state)
    return state


def cmd_propose(args: argparse.Namespace) -> dict[str, Any]:
    state_path = pathlib.Path(args.state).resolve()
    state = load_state(state_path)
    base_dir = pathlib.Path(state["active_base_dir"])
    manifest, manifest_sha = load_bundle_manifest(base_dir)
    if manifest_sha != state["active_base_manifest_sha256"]:
        raise SurgeryLoopError("active_base_dir manifest SHA-256 drifted from SURGERY_STATE.json")

    watchdog = compute_watchdog(state, args.now_utc)
    if watchdog["status"] == "ROLLBACK_REQUIRED" and watchdog["unscored_patches"]:
        raise SurgeryLoopError(
            "stall fuse tripped with unscored patches still pending; resolve/rollback before proposing new edits"
        )

    if not args.allow_engine_changes:
        dirty = dirty_crates_files(pathlib.Path(args.repo_root).resolve())
        if dirty:
            raise SurgeryLoopError(
                "refusing new surgery patch while crates/ has uncommitted engine changes "
                f"({len(dirty)} entries, e.g. {dirty[0]}). Separate engine work from weight surgery "
                "or pass --allow-engine-changes explicitly."
            )

    patch_path = pathlib.Path(args.patch).resolve()
    patch = read_json(patch_path, "surgery patch")
    if patch.get("schema") != PATCH_SCHEMA or patch.get("schema_version") != SCHEMA_VERSION:
        raise SurgeryLoopError(f"unsupported patch schema in {patch_path}")

    patch_id = str(patch.get("patch_id", "")).strip()
    if not patch_id:
        raise SurgeryLoopError("patch_id must be non-empty")
    if patch.get("target_base_id") != state["active_base_id"]:
        raise SurgeryLoopError(
            f"patch {patch_id} targets {patch.get('target_base_id')!r}, expected active base "
            f"{state['active_base_id']!r} (blind stacking onto unscored rounds is blocked)"
        )
    if patch.get("target_foundation_digest") != manifest["foundation_digest"]:
        raise SurgeryLoopError(f"patch {patch_id} target_foundation_digest mismatch")
    if patch.get("target_dendritic_branches_sha256") != manifest["dendritic_branches_sha256"]:
        raise SurgeryLoopError(f"patch {patch_id} target_dendritic_branches_sha256 mismatch")

    in_flight = state["in_flight_patches"]
    if any(p["patch_id"] == patch_id and p["status"] != "superseded" for p in in_flight):
        raise SurgeryLoopError(f"patch_id {patch_id!r} is already registered")
    if any(p["base_id"] == patch_id for p in state.get("promoted_history", [])):
        raise SurgeryLoopError(f"patch_id {patch_id!r} conflicts with a previously promoted base")

    unscored = [p for p in in_flight if p["status"] == "unscored"]
    owner = args.owner.strip()
    if not owner:
        raise SurgeryLoopError("--owner must be non-empty")
    if any(p["owner"] == owner for p in unscored):
        raise SurgeryLoopError(
            f"owner {owner!r} already has an unscored patch in flight; score it first before proposing another"
        )
    if len(unscored) >= int(state["max_unscored_in_flight"]):
        raise SurgeryLoopError(
            f"unscored patch backlog reached limit ({state['max_unscored_in_flight']}); score existing patches first"
        )

    entry = {
        "patch_id": patch_id,
        "owner": owner,
        "patch_path": str(patch_path),
        "target_base_id": state["active_base_id"],
        "status": "unscored",
        "proposed_at_utc": utc_now_iso(args.now_utc),
        "report_path": None,
        "base_score": None,
        "candidate_score": None,
        "delta": None,
        "promoted_decision": None,
        "scored_at_utc": None,
    }
    in_flight.append(entry)
    write_json_atomic(state_path, state)
    return entry


def cmd_record_score(args: argparse.Namespace) -> dict[str, Any]:
    state_path = pathlib.Path(args.state).resolve()
    state = load_state(state_path)
    report_path = pathlib.Path(args.report).resolve()
    report = read_json(report_path, "comparison report")

    if report.get("schema") != REPORT_SCHEMA or report.get("schema_version") != SCHEMA_VERSION:
        raise SurgeryLoopError("report is not a matched brain_surgery comparison_report.v1 (proxy checks refused)")
    if report.get("base_id") != state["active_base_id"]:
        raise SurgeryLoopError(
            f"report base_id {report.get('base_id')!r} != active_base_id {state['active_base_id']!r}"
        )
    if report.get("base_foundation_digest") != state["active_base_foundation_digest"]:
        raise SurgeryLoopError("report base_foundation_digest mismatch")
    if report.get("base_dendritic_sha256") != state["active_base_dendritic_sha256"]:
        raise SurgeryLoopError("report base_dendritic_sha256 mismatch")
    if int(report.get("failed_cases", 1)) != 0:
        raise SurgeryLoopError(
            f"comparison report has failed_cases={report.get('failed_cases')}; cannot count as a valid score"
        )
    if not report.get("results"):
        raise SurgeryLoopError("comparison report contains zero case results")
    if report.get("candidate_patch_id") != args.patch_id:
        raise SurgeryLoopError(
            f"report candidate_patch_id {report.get('candidate_patch_id')!r} != --patch-id {args.patch_id!r}"
        )

    target = next(
        (
            p
            for p in state["in_flight_patches"]
            if p["patch_id"] == args.patch_id and p["target_base_id"] == state["active_base_id"]
        ),
        None,
    )
    if target is None:
        raise SurgeryLoopError(f"patch {args.patch_id!r} not found in active in_flight_patches")
    if target["status"] != "unscored":
        raise SurgeryLoopError(f"patch {args.patch_id!r} already has status={target['status']!r}")

    scored_at = utc_now_iso(args.now_utc)
    target["status"] = "scored"
    target["report_path"] = str(report_path)
    target["base_score"] = float(report["base_average"])
    target["candidate_score"] = float(report["candidate_average"])
    target["delta"] = float(report["delta"])
    target["promoted_decision"] = bool(report["promoted"])
    target["scored_at_utc"] = scored_at
    state["last_real_score_utc"] = scored_at
    write_json_atomic(state_path, state)
    return target


def cmd_promote(args: argparse.Namespace) -> dict[str, Any]:
    state_path = pathlib.Path(args.state).resolve()
    state = load_state(state_path)
    patch = next(
        (
            p
            for p in state["in_flight_patches"]
            if p["patch_id"] == args.patch_id and p["target_base_id"] == state["active_base_id"]
        ),
        None,
    )
    if patch is None or patch["status"] != "scored":
        raise SurgeryLoopError(f"patch {args.patch_id!r} must be scored against active base before promotion")
    if not patch["promoted_decision"]:
        raise SurgeryLoopError(f"patch {args.patch_id!r} did not beat active base on overall average; cannot promote")

    candidate_dir = pathlib.Path(args.candidate_bundle).resolve()
    cand_manifest, _ = load_bundle_manifest(candidate_dir)

    receipt_path = pathlib.Path(args.reopen_receipt).resolve()
    reopen = read_json(receipt_path, "reopen receipt")
    if reopen.get("schema") != REOPEN_SCHEMA or reopen.get("schema_version") != SCHEMA_VERSION:
        raise SurgeryLoopError("invalid reopen_receipt schema")
    if reopen.get("verified") is not True or int(reopen.get("tick", 0)) <= 0:
        raise SurgeryLoopError("reopen verification must pass with verified=true and tick > 0 before promotion")
    if (
        reopen.get("base_id") != cand_manifest["base_id"]
        or reopen.get("foundation_digest") != cand_manifest["foundation_digest"]
        or reopen.get("dendritic_branches_sha256") != cand_manifest["dendritic_branches_sha256"]
    ):
        raise SurgeryLoopError("reopen receipt does not match candidate bundle weights + V11 branches")

    new_base_id = args.new_base_id.strip()
    if not new_base_id:
        raise SurgeryLoopError("--new-base-id must be non-empty")

    backup_root = pathlib.Path(args.backup_dir).resolve()
    backup_dest = backup_root / new_base_id
    if backup_dest.exists():
        raise SurgeryLoopError(f"promotion backup destination already exists: {backup_dest}")
    backup_root.mkdir(parents=True, exist_ok=True)
    shutil.copytree(candidate_dir, backup_dest)
    shutil.copy2(receipt_path, backup_dest / "reopen_receipt.json")

    # Normalize manifest base_id and embedded reopen receipt SHA in the promoted backup copy.
    cand_manifest["base_id"] = new_base_id
    cand_manifest["parent_base_id"] = state["active_base_id"]
    cand_manifest["reopen_verified"] = True
    cand_manifest["reopen_receipt_file"] = "reopen_receipt.json"
    cand_manifest["reopen_receipt_sha256"] = sha256_file(backup_dest / "reopen_receipt.json")
    write_json_atomic(backup_dest / "manifest.json", cand_manifest)
    dest_manifest_sha = sha256_file(backup_dest / "manifest.json")

    for other in state["in_flight_patches"]:
        if other["patch_id"] == args.patch_id:
            other["status"] = "promoted"
        elif other["status"] in ("unscored", "scored"):
            other["status"] = "superseded"

    promoted_record = {
        "base_id": new_base_id,
        "parent_base_id": state["active_base_id"],
        "patch_id": args.patch_id,
        "score": float(patch["candidate_score"]),
        "previous_score": float(patch["base_score"]),
        "delta": float(patch["delta"]),
        "backup_dir": str(backup_dest),
        "manifest_sha256": dest_manifest_sha,
        "foundation_digest": cand_manifest["foundation_digest"],
        "dendritic_branches_sha256": cand_manifest["dendritic_branches_sha256"],
        "drive_id": args.drive_id,
        "promoted_at_utc": utc_now_iso(args.now_utc),
    }
    state.setdefault("promoted_history", []).append(promoted_record)
    state["active_base_id"] = new_base_id
    state["active_base_dir"] = str(backup_dest)
    state["active_base_manifest_sha256"] = dest_manifest_sha
    state["active_base_foundation_digest"] = cand_manifest["foundation_digest"]
    state["active_base_dendritic_sha256"] = cand_manifest["dendritic_branches_sha256"]
    state["active_base_score"] = float(patch["candidate_score"])
    if args.drive_id:
        state["active_base_drive_id"] = args.drive_id

    write_json_atomic(state_path, state)
    return {
        "promoted": promoted_record,
        "gdrive_backup_hint": (
            f"tar -C {backup_root} -cf {new_base_id}.tar {new_base_id} && "
            f"gdrive mutate upload {backup_root / (new_base_id + '.tar')}"
        ),
    }


def cmd_watchdog(args: argparse.Namespace) -> dict[str, Any]:
    state = load_state(pathlib.Path(args.state).resolve())
    return compute_watchdog(state, args.now_utc)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Fail-closed brain surgery round controller")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_init = sub.add_parser("init")
    p_init.add_argument("--state", required=True)
    p_init.add_argument("--base-dir", required=True)
    p_init.add_argument("--base-score", required=True, type=float)
    p_init.add_argument("--repo-root", default=".")
    p_init.add_argument("--git-sha", default=None)
    p_init.add_argument("--eval-bin", default=None)
    p_init.add_argument("--drive-id", default=None)
    p_init.add_argument("--stall-fuse-hours", type=float, default=2.0)
    p_init.add_argument("--max-unscored", type=int, default=5)
    p_init.add_argument("--require-reopen-verified", action="store_true")
    p_init.add_argument("--now-utc", default=None)

    p_prop = sub.add_parser("propose")
    p_prop.add_argument("--state", required=True)
    p_prop.add_argument("--owner", required=True)
    p_prop.add_argument("--patch", required=True)
    p_prop.add_argument("--repo-root", default=".")
    p_prop.add_argument("--allow-engine-changes", action="store_true")
    p_prop.add_argument("--now-utc", default=None)

    p_score = sub.add_parser("record-score")
    p_score.add_argument("--state", required=True)
    p_score.add_argument("--patch-id", required=True)
    p_score.add_argument("--report", required=True)
    p_score.add_argument("--now-utc", default=None)

    p_prom = sub.add_parser("promote")
    p_prom.add_argument("--state", required=True)
    p_prom.add_argument("--patch-id", required=True)
    p_prom.add_argument("--new-base-id", required=True)
    p_prom.add_argument("--candidate-bundle", required=True)
    p_prom.add_argument("--reopen-receipt", required=True)
    p_prom.add_argument("--backup-dir", required=True)
    p_prom.add_argument("--drive-id", default=None)
    p_prom.add_argument("--now-utc", default=None)

    p_watch = sub.add_parser("watchdog")
    p_watch.add_argument("--state", required=True)
    p_watch.add_argument("--now-utc", default=None)
    p_watch.add_argument("--fail-on-stall", action="store_true")

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        if args.cmd == "init":
            result = cmd_init(args)
        elif args.cmd == "propose":
            result = cmd_propose(args)
        elif args.cmd == "record-score":
            result = cmd_record_score(args)
        elif args.cmd == "promote":
            result = cmd_promote(args)
        elif args.cmd == "watchdog":
            result = cmd_watchdog(args)
            print(json.dumps(result, indent=2))
            if args.fail_on_stall and result["status"] == "ROLLBACK_REQUIRED":
                return 2
            return 0
        else:
            raise SurgeryLoopError(f"unknown subcommand: {args.cmd}")
        print(json.dumps(result, indent=2))
        return 0
    except SurgeryLoopError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Fail-closed state machine for Qualia brain surgery rounds.

Enforces:
- Cross-process exclusive file locking around read -> validate -> atomic write.
- Single shared active base bundle (no blind stacking R(n+1) onto unscored R(n)).
- At most one unscored patch in flight per Astra owner, bounded global backlog.
- Data-only patches (rejects unlanded changes under crates/ unless explicit override).
- Candidate digest binding from score -> reopen verification -> byte-identical backup promotion.
- A stall is observable; rollback is reserved for verified active state failure.
"""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import datetime as dt
import hashlib
import json
import math
import re
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time
from typing import Any, Iterator

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


@contextmanager
def state_lock(state_path: pathlib.Path, timeout_secs: float = 30.0) -> Iterator[None]:
    """Exclusive cross-process lock serializing read -> validate -> write on state_path."""
    state_path.parent.mkdir(parents=True, exist_ok=True)
    lock_path = pathlib.Path(f"{state_path}.lock")
    deadline = time.monotonic() + timeout_secs
    with lock_path.open("a+b") as handle:
        if os.name == "nt":
            import msvcrt

            while True:
                try:
                    handle.seek(0)
                    msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
                    break
                except OSError as exc:
                    if time.monotonic() >= deadline:
                        raise SurgeryLoopError(
                            f"timed out acquiring state lock {lock_path}: {exc}"
                        ) from exc
                    time.sleep(0.02)
            try:
                yield
            finally:
                handle.seek(0)
                msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)
        else:
            import fcntl

            while True:
                try:
                    fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except BlockingIOError as exc:
                    if time.monotonic() >= deadline:
                        raise SurgeryLoopError(
                            f"timed out acquiring state lock {lock_path}: {exc}"
                        ) from exc
                    time.sleep(0.02)
            try:
                yield
            finally:
                fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def load_bundle_manifest(bundle_dir: pathlib.Path) -> tuple[dict[str, Any], str]:
    manifest_path = bundle_dir / "manifest.json"
    manifest = read_json(manifest_path, "bundle manifest")
    if (
        manifest.get("schema") != BUNDLE_MANIFEST_SCHEMA
        or manifest.get("schema_version") != SCHEMA_VERSION
    ):
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

    if "full_saved_life_current_file" in manifest:
        current = bundle_dir / str(manifest["full_saved_life_current_file"])
        if (current.parent != bundle_dir or not current.is_file() or current.is_symlink()
                or sha256_file(current) != manifest.get("full_saved_life_current_sha256")):
            raise SurgeryLoopError("mature Current missing or digest drifted")
    if manifest.get("reopen_verified"):
        receipt_file = manifest.get("reopen_receipt_file")
        receipt_hash = manifest.get("reopen_receipt_sha256")
        if not receipt_file or not receipt_hash:
            raise SurgeryLoopError(
                f"bundle manifest has reopen_verified=true without receipt metadata: {manifest_path}"
            )
        receipt_path = bundle_dir / str(receipt_file)
        if not receipt_path.is_file():
            raise SurgeryLoopError(f"bundle reopen receipt missing: {receipt_path}")
        if sha256_file(receipt_path) != receipt_hash:
            raise SurgeryLoopError(f"bundle reopen_receipt SHA-256 mismatch: {receipt_path}")
        receipt = read_json(receipt_path, "bundle reopen receipt")
        validate_reopen_receipt_matches_manifest(receipt, manifest)

    return manifest, sha256_file(manifest_path)


def validate_reopen_receipt_matches_manifest(
    reopen: dict[str, Any], manifest: dict[str, Any]
) -> None:
    if reopen.get("schema") != REOPEN_SCHEMA or reopen.get("schema_version") != SCHEMA_VERSION:
        raise SurgeryLoopError("invalid reopen_receipt schema")
    if reopen.get("verified") is not True or int(reopen.get("tick", 0)) <= 0:
        raise SurgeryLoopError(
            "reopen verification must pass with verified=true and tick > 0"
        )
    if (
        reopen.get("base_id") != manifest["base_id"]
        or reopen.get("foundation_digest") != manifest["foundation_digest"]
        or reopen.get("dendritic_branches_sha256") != manifest["dendritic_branches_sha256"]
    ):
        raise SurgeryLoopError(
            "reopen receipt does not match bundle base_id, weights digest, or V11 branch hash"
        )


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


def verified_state_failure(state: dict[str, Any]) -> str | None:
    """Validate immutable active artifacts, not merely the time since progress."""
    try:
        manifest, digest = load_bundle_manifest(pathlib.Path(state["active_base_dir"]))
        for actual, key in (
            (digest, "active_base_manifest_sha256"),
            (manifest["base_id"], "active_base_id"),
            (manifest["foundation_digest"], "active_base_foundation_digest"),
            (manifest["dendritic_branches_sha256"], "active_base_dendritic_sha256"),
        ):
            if actual != state.get(key):
                raise SurgeryLoopError(f"active bundle differs from {key}")
        gate = state.get("saved_life_controls", {})
        if gate and (not manifest.get("reopen_verified") or
                     manifest.get("full_saved_life_current_sha256") != gate.get("full_saved_life_current_sha256")):
            raise SurgeryLoopError("active mature Current/reopen identity differs from state")
        runtime = gate.get("evaluator_local_path")
        if runtime and sha256_file(pathlib.Path(runtime)) != state["known_good_eval_bin_sha256"]:
            raise SurgeryLoopError("installed saved-life evaluator differs from state")
        card = gate.get("active_card")
        if card:
            load_active_card(state)
        return None
    except (SurgeryLoopError, OSError, KeyError, ValueError, TypeError) as exc:
        return str(exc)


def load_active_card(state: dict[str, Any]) -> dict[str, Any]:
    gate = state["saved_life_controls"]
    pin = gate["active_card"]
    path = pathlib.Path(pin["path"])
    if sha256_file(path) != pin["sha256"]:
        raise SurgeryLoopError("active card SHA-256 differs from state")
    card = read_json(path, "frozen active card")
    if (card["scorecard_id"] != pin["id"] or len(card["cases"]) != 29
            or card["executable_sha256"] != state["known_good_eval_bin_sha256"]
            or card["historical_rubric_sha256"] != gate["historical_rubric_sha256"]):
        raise SurgeryLoopError("active card identity differs from state")
    baseline = pin.get("baseline")
    if baseline and (baseline["base_id"] != state["active_base_id"]
                     or baseline["score"] != state["active_base_score"]
                     or sha256_file(pathlib.Path(baseline["path"])) != baseline["sha256"]):
        raise SurgeryLoopError("active baseline differs from state")
    return card


def compute_watchdog(state: dict[str, Any], now_utc: str | None = None,
                     *, failure: str | None = None) -> dict[str, Any]:
    now = parse_utc(now_utc) if now_utc else dt.datetime.now(dt.timezone.utc)
    stamps = [parse_utc(str(state["last_real_score_utc"]))]
    for key in ("last_recovery_utc", "last_completed_comparison_utc", "last_candidate_closed_utc",
                "last_baseline_utc"):
        if state.get(key):
            stamps.append(parse_utc(state[key]))
    last_score = stamps[0]
    hours_since_real_score = max(0.0, (now - last_score).total_seconds() / 3600.0)
    hours_since_progress = max(0.0, (now - max(stamps)).total_seconds() / 3600.0)
    fuse_hours = float(state.get("stall_fuse_hours", 2.0))
    in_flight = list(state.get("in_flight_patches", []))
    unscored = [p for p in in_flight if p.get("status") == "unscored"]
    scored = [p for p in in_flight if p.get("status") == "scored"]
    gate = state.get("saved_life_controls", {})
    card = gate.get("active_card")
    stalled = hours_since_progress >= fuse_hours

    if failure:
        status = "ROLLBACK_REQUIRED"
        action = "Do not dispatch. Restore the exact verified active bundle or card and independently reopen it."
    elif gate.get("round_paused"):
        status = "PAUSED"
        action = "Finish the authorized handoff or verification; do not dispatch new patches."
    elif card and not card.get("baseline"):
        status = "BASELINE_REQUIRED"
        action = "Run the frozen full card against the exact active saved-life base before formal scoring."
    elif unscored:
        status = "SCORE_QUEUE_BACKLOG"
        action = "Run matched saved-life comparisons for the existing candidates; no proxy tests."
    elif stalled:
        status = "STALLED"
        action = "No measured progress within the time window. Resume the next real comparison; do not roll back for time alone."
    else:
        status = "HEALTHY"
        action = f"Ready for data-only patches targeting active_base_id={state.get('active_base_id')}."

    return {
        "status": status,
        "stalled": stalled,
        "verified_state_failure": failure,
        "hours_since_last_real_score": round(hours_since_real_score, 4),
        "hours_since_last_progress": round(hours_since_progress, 4),
        "stall_fuse_hours": fuse_hours,
        "last_real_score_utc": state["last_real_score_utc"],
        "last_recovery_utc": state.get("last_recovery_utc"),
        "last_completed_comparison_utc": state.get("last_completed_comparison_utc"),
        "last_candidate_closed_utc": state.get("last_candidate_closed_utc"),
        "last_baseline_utc": state.get("last_baseline_utc"),
        "active_card_id": card and card["id"],
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
    with state_lock(state_path):
        manifest, manifest_sha = load_bundle_manifest(base_dir)
        if args.require_reopen_verified and not manifest.get("reopen_verified", False):
            raise SurgeryLoopError(
                f"base bundle {base_dir} is not reopen_verified=true; run brain_surgery_eval verify-reopen --stamp-bundle first"
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
            "last_recovery_utc": None,
            "stall_fuse_hours": float(args.stall_fuse_hours),
            "max_unscored_in_flight": int(args.max_unscored),
            "in_flight_patches": [],
            "promoted_history": [],
        }
        write_json_atomic(state_path, state)
        return state


def cmd_propose(args: argparse.Namespace) -> dict[str, Any]:
    state_path = pathlib.Path(args.state).resolve()
    with state_lock(state_path):
        state = load_state(state_path)
        if "saved_life_controls" in state:
            if state["saved_life_controls"].get("round_paused", True):
                raise SurgeryLoopError("next round is paused pending saved-life lab handoff")
            if args.owner not in {a["owner"] for a in state.get("astra_assignments", [])
                                  if a.get("can_propose") is True}:
                raise SurgeryLoopError("owner is not registered to propose a candidate")
        base_dir = pathlib.Path(state["active_base_dir"])
        manifest, manifest_sha = load_bundle_manifest(base_dir)
        if manifest_sha != state["active_base_manifest_sha256"]:
            raise SurgeryLoopError("active_base_dir manifest SHA-256 drifted from SURGERY_STATE.json")

        watchdog = compute_watchdog(state, args.now_utc, failure=verified_state_failure(state))
        if watchdog["status"] == "ROLLBACK_REQUIRED":
            raise SurgeryLoopError(
                f"verified state failure: {watchdog['verified_state_failure']}"
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
            "patch_sha256": sha256_file(patch_path),
            "target_base_id": state["active_base_id"],
            "status": "unscored",
            "proposed_at_utc": utc_now_iso(args.now_utc),
            "report_path": None,
            "candidate_id": None,
            "candidate_foundation_digest": None,
            "candidate_dendritic_sha256": None,
            "base_score": None,
            "candidate_score": None,
            "delta": None,
            "promoted_decision": None,
            "scored_at_utc": None,
        }
        in_flight.append(entry)
        write_json_atomic(state_path, state)
        return entry


def valid_digest(value: Any) -> bool:
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def validate_card_cases(card: dict[str, Any]) -> dict[str, Any]:
    cases = card["cases"]
    if not isinstance(cases, dict) or len(cases) != 29 or any(
        not valid_digest(v.get("scenario_sha256")) or type(v.get("decisions")) is not int
        or v["decisions"] <= 0 or type(v.get("weight")) not in (int, float)
        or not math.isfinite(v["weight"]) or v["weight"] < 0 for v in cases.values()
    ) or not math.isclose(math.fsum(v["weight"] for v in cases.values()), 100.0, abs_tol=1e-6):
        raise SurgeryLoopError("invalid frozen 29-case card scenarios, decisions, or weights")
    return cases


def weighted_score(cases: dict[str, Any], scores: dict[str, Any]) -> float:
    if scores.keys() != cases.keys() or any(type(v) not in (int, float) or not 0 <= v <= 100 for v in scores.values()):
        raise SurgeryLoopError("missing, duplicated, or invalid frozen-card score rows")
    return math.fsum(cases[k]["weight"] * v for k, v in scores.items()) / math.fsum(
        v["weight"] for v in cases.values())


def record_baseline(state: dict[str, Any], path: pathlib.Path) -> dict[str, Any]:
    card = load_active_card(state)
    specs = validate_card_cases(card)
    gate = state["saved_life_controls"]
    pin = gate["active_card"]
    if pin.get("baseline"):
        raise SurgeryLoopError("baseline is already frozen")
    proof = read_json(path, "fresh native baseline")
    expected = dict(schema="qualia.saved_life_baseline.v1", base_id=state["active_base_id"],
                    scorecard_id=pin["id"], rubric_sha256=pin["sha256"],
                    evaluator_sha256=state["known_good_eval_bin_sha256"],
                    source_current_sha256=gate["full_saved_life_current_sha256"])
    if any(proof.get(k) != v for k, v in expected.items()) or proof["cases"].keys() != specs.keys():
        raise SurgeryLoopError("baseline is not tied to the complete active card and saved-life base")
    for key, v in proof["cases"].items():
        spec = specs[key]
        if (v.get("scenario_sha256") != spec["scenario_sha256"] or v.get("decisions") != spec["decisions"]
                or not valid_digest(v.get("native_receipt_sha256")) or not valid_digest(v.get("rows_sha256"))):
            raise SurgeryLoopError(f"baseline native evidence invalid for {key}")
    score = weighted_score(specs, {k: v["score"] for k,v in proof["cases"].items()})
    if not math.isclose(score, proof["score"], abs_tol=1e-8):
        raise SurgeryLoopError("baseline score differs from frozen-card weights")
    when = utc_now_iso(proof["measured_at_utc"])
    if not parse_utc(gate["round_transition_utc"]) <= parse_utc(when) <= parse_utc(utc_now_iso()):
        raise SurgeryLoopError("baseline was not measured after the authorized transition")
    pin["baseline"] = dict(base_id=state["active_base_id"], score=score, path=str(path),
                           sha256=sha256_file(path), measured_at_utc=when)
    state["active_base_score"] = score
    state["last_baseline_utc"] = when
    return pin["baseline"]


def cmd_record_baseline(args: argparse.Namespace) -> dict[str, Any]:
    path = pathlib.Path(args.state).resolve()
    with state_lock(path):
        state = load_state(path)
        failure = verified_state_failure(state)
        if failure:
            raise SurgeryLoopError(failure)
        baseline = record_baseline(state, pathlib.Path(args.evidence).resolve())
        write_json_atomic(path,state)
        return baseline


def check_active_comparison(state: dict[str, Any], report: dict[str, Any], target: dict[str, Any]) -> str:
    gate = state["saved_life_controls"]
    pin = gate["active_card"]
    if not pin.get("baseline"):
        raise SurgeryLoopError("fresh baseline must be established before recording a full-card score")
    card = load_active_card(state)
    specs = validate_card_cases(card)
    evidence = report["saved_life_evidence"]
    expected = dict(full_score=True, rubric_sha256=pin["sha256"], case_count=len(specs),
                    source_current_sha256=gate["full_saved_life_current_sha256"],
                    evaluator_sha256=state["known_good_eval_bin_sha256"], patch_sha256=target["patch_sha256"])
    candidate_current = evidence.get("candidate_current_sha256")
    if (report.get("scorecard_id") != pin["id"] or any(evidence.get(k)!=v for k,v in expected.items())
            or not valid_digest(candidate_current) or candidate_current == expected["source_current_sha256"]
            or sha256_file(pathlib.Path(target["patch_path"])) != target["patch_sha256"]):
        raise SurgeryLoopError("report does not match the active card, exact patch and mature arms")
    results = report["results"]
    rows = {r["case_id"]:r for r in results}
    if len(results) != len(specs) or rows.keys() != specs.keys():
        raise SurgeryLoopError("full-card report does not contain every frozen case exactly once")
    for name, spec in specs.items():
        r = rows[name]
        if (r["decisions"]!=spec["decisions"] or r["scenario_sha256"]!=spec["scenario_sha256"]
                or r["weight"]!=spec["weight"] or r.get("error")
                or not all(valid_digest(r.get(k)) for k in ("base_native_receipt_sha256",
                    "candidate_native_receipt_sha256", "base_rows_sha256", "candidate_rows_sha256"))):
            raise SurgeryLoopError(f"native matched evidence missing or inconsistent for {name}")
    base, candidate = [weighted_score(specs, {k:r[arm+"_score"] for k,r in rows.items()})
                       for arm in ("base","candidate")]
    epsilon = 1e-8
    threshold = card["min_improvement"]
    delta = candidate-base
    if (not all(math.isclose(x,y,abs_tol=epsilon) for x,y in (
        (base,state["active_base_score"]),(base,report["base_average"]),
        (candidate,report["candidate_average"]),(delta,report["delta"]),
        (threshold,report["min_improvement"])))
            or report["promoted"] is not (delta > threshold+1e-9)):
        raise SurgeryLoopError("scores or promotion decision disagree with frozen card and fresh base")
    return candidate_current


def cmd_record_comparison(args: argparse.Namespace) -> dict[str, Any]:
    path = pathlib.Path(args.state).resolve()
    proof_path = pathlib.Path(args.evidence).resolve()
    with state_lock(path):
        state = load_state(path)
        failure = verified_state_failure(state)
        if failure:
            raise SurgeryLoopError(failure)
        gate = state["saved_life_controls"]
        card = load_active_card(state)
        pin = gate["active_card"]
        proof = read_json(proof_path, "independent native comparison")
        key = proof["case_id"]
        spec = card["cases"].get(key)
        target = next((p for p in state["in_flight_patches"]
                       if p["patch_id"] == proof["patch_id"] and p["target_base_id"] == state["active_base_id"]
                       and p["status"] == "unscored"),None)
        if not spec or not target:
            raise SurgeryLoopError("comparison lacks a frozen case or registered candidate")
        expected = dict(schema="qualia.saved_life_completed_comparison.v1", scorecard_id=pin["id"],
                        rubric_sha256=pin["sha256"], evaluator_sha256=state["known_good_eval_bin_sha256"],
                        base_id=state["active_base_id"], source_current_sha256=gate["full_saved_life_current_sha256"],
                        patch_sha256=target["patch_sha256"], scenario_sha256=spec["scenario_sha256"],
                        decisions=spec["decisions"])
        if (any(proof.get(k)!=v for k,v in expected.items())
                or sha256_file(pathlib.Path(target["patch_path"])) != target["patch_sha256"]
                or any(proof.get(k) is not True for k in ("independently_accepted","matched","real_game_inputs"))
                or not all(valid_digest(proof.get(k)) for k in ("candidate_current_sha256",
                    "base_native_receipt_sha256","candidate_native_receipt_sha256","base_rows_sha256","candidate_rows_sha256"))
                or proof["candidate_current_sha256"]==expected["source_current_sha256"]):
            raise SurgeryLoopError("comparison did not match the exact mature card, patch and arms")
        when = utc_now_iso(proof["completed_at_utc"])
        if not parse_utc(target["proposed_at_utc"]) <= parse_utc(when) <= parse_utc(utc_now_iso()):
            raise SurgeryLoopError("native completion time outside registered experiment")
        items = target.setdefault("completed_comparisons", [])
        identity = (proof["base_native_receipt_sha256"], proof["candidate_native_receipt_sha256"])
        if any(tuple(item["native_receipt_sha256"])==identity for item in items):
            raise SurgeryLoopError("native comparison already recorded")
        item = dict(case_id=key, completed_at_utc=when, path=str(proof_path), sha256=sha256_file(proof_path),
                    native_receipt_sha256=list(identity))
        items.append(item)
        prev = state.get("last_completed_comparison_utc")
        if not prev or parse_utc(prev) < parse_utc(when):
            state["last_completed_comparison_utc"] = when
        write_json_atomic(path,state)
        return item


def cmd_close_candidate(args: argparse.Namespace) -> dict[str, Any]:
    path = pathlib.Path(args.state).resolve()
    with state_lock(path):
        state = load_state(path)
        target = next((p for p in state["in_flight_patches"]
                       if p["patch_id"] == args.patch_id and p["target_base_id"] == state["active_base_id"]
                       and p["status"] in ("unscored","scored")),None)
        if not target or not (target.get("completed_comparisons") or target.get("report_path")):
            raise SurgeryLoopError("closing requires recorded matched evidence")
        if target.get("promoted_decision"):
            raise SurgeryLoopError("a winning scored candidate requires a promotion decision, not routine close")
        if not args.reason.strip():
            raise SurgeryLoopError("closure must state the measured outcome")
        now=utc_now_iso()
        target.update(status="closed",closed_at_utc=now,closure_reason=args.reason)
        state["last_candidate_closed_utc"]=now
        write_json_atomic(path,state)
        return target


def cmd_record_score(args: argparse.Namespace) -> dict[str, Any]:
    state_path = pathlib.Path(args.state).resolve()
    with state_lock(state_path):
        state = load_state(state_path)
        report_path = pathlib.Path(args.report).resolve()
        report = read_json(report_path, "comparison report")
        if "saved_life_controls" in state and not state["saved_life_controls"].get("active_card"):
            gate = state["saved_life_controls"]
            evidence = report.get("saved_life_evidence", {})
            if (evidence.get("full_score") is not True
                    or evidence.get("rubric_sha256") != gate["historical_rubric_sha256"]
                    or evidence.get("evaluator_sha256") != state["known_good_eval_bin_sha256"]
                    or evidence.get("source_current_sha256") != gate["full_saved_life_current_sha256"]
                    or evidence.get("case_count") != 30):
                raise SurgeryLoopError("report is not an exact complete mature saved-life card")

        if report.get("schema") != REPORT_SCHEMA or report.get("schema_version") != SCHEMA_VERSION:
            raise SurgeryLoopError(
                "report is not a matched brain_surgery comparison_report.v1 (proxy checks refused)"
            )
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

        candidate_id = str(report.get("candidate_id") or "").strip()
        cand_foundation_digest = str(report.get("candidate_foundation_digest") or "").strip()
        cand_dendritic_sha = str(report.get("candidate_dendritic_sha256") or "").strip()
        if not candidate_id or not cand_foundation_digest or not cand_dendritic_sha:
            raise SurgeryLoopError(
                "comparison report must include non-empty candidate_id, candidate_foundation_digest, "
                "and candidate_dendritic_sha256"
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

        if state.get("saved_life_controls", {}).get("active_card"):
            target["candidate_current_sha256"] = check_active_comparison(state, report, target)
            target["active_card_sha256"] = state["saved_life_controls"]["active_card"]["sha256"]
        scored_at = utc_now_iso(args.now_utc)
        target["status"] = "scored"
        target["report_path"] = str(report_path)
        target["candidate_id"] = candidate_id
        target["candidate_foundation_digest"] = cand_foundation_digest
        target["candidate_dendritic_sha256"] = cand_dendritic_sha
        target["base_score"] = float(report["base_average"])
        target["candidate_score"] = float(report["candidate_average"])
        target["delta"] = float(report["delta"])
        target["promoted_decision"] = bool(report["promoted"])
        target["scored_at_utc"] = scored_at
        state["last_real_score_utc"] = scored_at
        state["last_completed_comparison_utc"] = scored_at
        write_json_atomic(state_path, state)
        return target


def cmd_record_recovery(args: argparse.Namespace) -> dict[str, Any]:
    state_path = pathlib.Path(args.state).resolve()
    with state_lock(state_path):
        state = load_state(state_path)
        base_dir = pathlib.Path(state["active_base_dir"])
        manifest, manifest_sha = load_bundle_manifest(base_dir)
        if manifest_sha != state["active_base_manifest_sha256"]:
            raise SurgeryLoopError("active_base_dir manifest SHA-256 drifted from SURGERY_STATE.json")
        reopen = read_json(pathlib.Path(args.reopen_receipt).resolve(), "recovery reopen receipt")
        validate_reopen_receipt_matches_manifest(reopen, manifest)
        recovered_at = utc_now_iso(args.now_utc)
        state["last_recovery_utc"] = recovered_at
        write_json_atomic(state_path, state)
        return compute_watchdog(state, recovered_at)


def cmd_promote(args: argparse.Namespace) -> dict[str, Any]:
    state_path = pathlib.Path(args.state).resolve()
    with state_lock(state_path):
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
            raise SurgeryLoopError(
                f"patch {args.patch_id!r} must be scored against active base before promotion"
            )
        if not patch["promoted_decision"]:
            raise SurgeryLoopError(
                f"patch {args.patch_id!r} did not beat active base on overall average; cannot promote"
            )

        candidate_dir = pathlib.Path(args.candidate_bundle).resolve()
        cand_manifest, cand_manifest_sha = load_bundle_manifest(candidate_dir)

        # Fix #1: Bind candidate bundle strictly to the exact digests recorded at score time.
        if (
            cand_manifest["base_id"] != patch.get("candidate_id")
            or cand_manifest["foundation_digest"] != patch.get("candidate_foundation_digest")
            or cand_manifest["dendritic_branches_sha256"] != patch.get("candidate_dendritic_sha256")
        ):
            raise SurgeryLoopError(
                "candidate bundle does not match the scored candidate_id, foundation_digest, "
                "or dendritic_branches_sha256"
            )

        # Fix #2: Never rename/rewrite manifest.json or reopen_receipt.json during promote.
        new_base_id = (args.new_base_id or cand_manifest["base_id"]).strip()
        if not new_base_id:
            raise SurgeryLoopError("--new-base-id must be non-empty")
        if cand_manifest["base_id"] != new_base_id:
            raise SurgeryLoopError(
                f"candidate bundle base_id is {cand_manifest['base_id']!r}, cannot rename to {new_base_id!r} "
                f"after scoring/reopen; materialize with --candidate-id {new_base_id!r} first"
            )
        if not cand_manifest.get("reopen_verified"):
            raise SurgeryLoopError(
                f"candidate bundle {candidate_dir} must be stamped with verify-reopen --stamp-bundle before promote"
            )
        embedded_receipt_path = candidate_dir / str(cand_manifest["reopen_receipt_file"])
        embedded_receipt_sha = sha256_file(embedded_receipt_path)
        if args.reopen_receipt:
            supplied_receipt_path = pathlib.Path(args.reopen_receipt).resolve()
            if sha256_file(supplied_receipt_path) != embedded_receipt_sha:
                raise SurgeryLoopError(
                    "supplied --reopen-receipt does not match candidate bundle's stamped reopen_receipt.json"
                )

        gate = state.get("saved_life_controls", {})
        card_pin = gate.get("active_card")
        if card_pin:
            if (patch.get("active_card_sha256") != card_pin["sha256"]
                    or not valid_digest(patch.get("candidate_current_sha256"))
                    or cand_manifest.get("full_saved_life_current_sha256") != patch["candidate_current_sha256"]
                    or not args.drive_id or args.drive_id == state.get("active_base_drive_id")):
                raise SurgeryLoopError("promotion must bind the exact scored mature Current, card and new off-host backup")
            reopen = read_json(embedded_receipt_path, "candidate separate-process reopen")
            if (reopen.get("source_current_sha256") != patch["candidate_current_sha256"]
                    or reopen.get("evaluator_sha256") != state["known_good_eval_bin_sha256"]
                    or reopen.get("parent_pid") == reopen.get("child_pid")
                    or any(type(reopen.get(k)) is not int or reopen[k] <= 0 for k in ("parent_pid","child_pid"))):
                raise SurgeryLoopError("candidate reopen must verify the exact scored mature Current in a separate process")

        backup_root = pathlib.Path(args.backup_dir).resolve()
        backup_dest = backup_root / new_base_id
        if backup_dest.exists():
            raise SurgeryLoopError(f"promotion backup destination already exists: {backup_dest}")
        backup_root.mkdir(parents=True, exist_ok=True)
        stage_dest = backup_root / f".promote-stage-{os.getpid()}"
        if stage_dest.exists():
            shutil.rmtree(stage_dest)
        try:
            shutil.copytree(candidate_dir, stage_dest)
            os.replace(stage_dest, backup_dest)
        finally:
            if stage_dest.exists():
                shutil.rmtree(stage_dest, ignore_errors=True)

        copied_manifest, copied_manifest_sha = load_bundle_manifest(backup_dest)
        if copied_manifest_sha != cand_manifest_sha:
            raise SurgeryLoopError("copied promotion backup manifest SHA-256 mismatch")

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
            "manifest_sha256": copied_manifest_sha,
            "foundation_digest": copied_manifest["foundation_digest"],
            "dendritic_branches_sha256": copied_manifest["dendritic_branches_sha256"],
            "drive_id": args.drive_id,
            "promoted_at_utc": utc_now_iso(args.now_utc),
        }
        state.setdefault("promoted_history", []).append(promoted_record)
        state["active_base_id"] = new_base_id
        state["active_base_dir"] = str(backup_dest)
        state["active_base_manifest_sha256"] = copied_manifest_sha
        state["active_base_foundation_digest"] = copied_manifest["foundation_digest"]
        state["active_base_dendritic_sha256"] = copied_manifest["dendritic_branches_sha256"]
        state["active_base_score"] = float(patch["candidate_score"])
        if card_pin:
            measured = pathlib.Path(patch["report_path"])
            gate["full_saved_life_current_sha256"] = copied_manifest["full_saved_life_current_sha256"]
            card_pin["baseline"] = dict(base_id=new_base_id, score=state["active_base_score"], path=str(measured),
                                         sha256=sha256_file(measured), measured_at_utc=patch["scored_at_utc"])
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
    return compute_watchdog(state, args.now_utc, failure=verified_state_failure(state))


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

    p_baseline = sub.add_parser("record-baseline")
    p_baseline.add_argument("--state", required=True)
    p_baseline.add_argument("--evidence", required=True)

    p_comparison = sub.add_parser("record-comparison")
    p_comparison.add_argument("--state", required=True)
    p_comparison.add_argument("--evidence", required=True)

    p_close = sub.add_parser("close-candidate")
    p_close.add_argument("--state", required=True)
    p_close.add_argument("--patch-id", required=True)
    p_close.add_argument("--reason", required=True)

    p_rec = sub.add_parser("record-recovery")
    p_rec.add_argument("--state", required=True)
    p_rec.add_argument("--reopen-receipt", required=True)
    p_rec.add_argument("--now-utc", default=None)

    p_prom = sub.add_parser("promote")
    p_prom.add_argument("--state", required=True)
    p_prom.add_argument("--patch-id", required=True)
    p_prom.add_argument("--new-base-id", default=None)
    p_prom.add_argument("--candidate-bundle", required=True)
    p_prom.add_argument("--reopen-receipt", default=None)
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
        elif args.cmd == "record-baseline":
            result = cmd_record_baseline(args)
        elif args.cmd == "record-comparison":
            result = cmd_record_comparison(args)
        elif args.cmd == "close-candidate":
            result = cmd_close_candidate(args)
        elif args.cmd == "record-recovery":
            result = cmd_record_recovery(args)
        elif args.cmd == "promote":
            result = cmd_promote(args)
        elif args.cmd == "watchdog":
            result = cmd_watchdog(args)
            print(json.dumps(result, indent=2))
            if args.fail_on_stall:
                if result["status"] == "ROLLBACK_REQUIRED":
                    return 2
                if result["stalled"]:
                    return 1
            return 0
        else:
            raise SurgeryLoopError(f"unknown subcommand: {args.cmd}")
        print(json.dumps(result, indent=2))
        return 0
    except (SurgeryLoopError, OSError, KeyError, ValueError, TypeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

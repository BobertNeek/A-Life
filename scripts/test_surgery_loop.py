from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPTS_DIR.parent
LOOP = SCRIPTS_DIR / "surgery_loop.py"
BOOTSTRAP = SCRIPTS_DIR / "bootstrap_cloud_surgery.sh"


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def make_bundle(
    root: Path,
    base_id: str,
    *,
    foundation_bytes: bytes | None = None,
    branches: list[dict] | None = None,
    reopen_verified: bool = True,
) -> Path:
    root.mkdir(parents=True, exist_ok=True)
    f_bytes = foundation_bytes if foundation_bytes is not None else f"foundation-{base_id}".encode()
    d_obj = {"branches": branches or []}
    d_bytes = json.dumps(d_obj, indent=2).encode()
    s_bytes = json.dumps(
        {
            "schema": "alife.brain_surgery.scorecard.v1",
            "schema_version": 1,
            "scorecard_id": "test-scorecard",
            "cases": [],
        },
        indent=2,
    ).encode()

    (root / "foundation.alife-foundation").write_bytes(f_bytes)
    (root / "dendritic_branches.json").write_bytes(d_bytes)
    (root / "scorecard.json").write_bytes(s_bytes)

    f_digest = sha256_bytes(f_bytes)
    d_sha = sha256_bytes(d_bytes)
    receipt_file = None
    receipt_sha = None
    if reopen_verified:
        receipt = {
            "schema": "alife.brain_surgery.reopen_receipt.v1",
            "schema_version": 1,
            "base_id": base_id,
            "foundation_digest": f_digest,
            "dendritic_branches_sha256": d_sha,
            "dendritic_branch_count": len(d_obj["branches"]),
            "tick": 1,
            "parent_pid": 100,
            "child_pid": 101,
            "verified": True,
            "semantic_prior_mode": "deterministic_seed_700001_backend_deterministic",
            "verified_at_unix": 1_760_000_000,
        }
        r_bytes = json.dumps(receipt, indent=2).encode()
        (root / "reopen_receipt.json").write_bytes(r_bytes)
        receipt_file = "reopen_receipt.json"
        receipt_sha = sha256_bytes(r_bytes)

    manifest = {
        "schema": "alife.brain_surgery.base_bundle.v1",
        "schema_version": 1,
        "base_id": base_id,
        "parent_base_id": None,
        "foundation_file": "foundation.alife-foundation",
        "foundation_sha256": sha256_bytes(f_bytes),
        "foundation_digest": f_digest,
        "sensor_profile": "grounded_terrain_vision_v1",
        "weight_count": 2048,
        "dendritic_branches_file": "dendritic_branches.json",
        "dendritic_branches_sha256": d_sha,
        "dendritic_branch_count": len(d_obj["branches"]),
        "semantic_prior_mode": "deterministic_for_training",
        "semantic_prior_context_contract": "test",
        "scorecard_file": "scorecard.json",
        "scorecard_sha256": sha256_bytes(s_bytes),
        "reopen_verified": reopen_verified,
        "reopen_receipt_file": receipt_file,
        "reopen_receipt_sha256": receipt_sha,
    }
    (root / "manifest.json").write_bytes(json.dumps(manifest, indent=2).encode())
    return root


def write_patch(
    path: Path,
    patch_id: str,
    base_bundle: Path,
) -> Path:
    manifest = json.loads((base_bundle / "manifest.json").read_text())
    patch = {
        "schema": "alife.brain_surgery.patch.v1",
        "schema_version": 1,
        "patch_id": patch_id,
        "target_base_id": manifest["base_id"],
        "target_foundation_digest": manifest["foundation_digest"],
        "target_dendritic_branches_sha256": manifest["dendritic_branches_sha256"],
        "weight_edits": [],
        "dendritic_edits": {"op": "keep"},
    }
    path.write_text(json.dumps(patch, indent=2))
    return path


def write_report(
    path: Path,
    patch_id: str,
    base_bundle: Path,
    cand_bundle: Path,
    *,
    base_avg: float = 50.0,
    cand_avg: float = 62.0,
    failed_cases: int = 0,
    promoted: bool = True,
) -> Path:
    bm = json.loads((base_bundle / "manifest.json").read_text())
    cm = json.loads((cand_bundle / "manifest.json").read_text())
    report = {
        "schema": "alife.brain_surgery.comparison_report.v1",
        "schema_version": 1,
        "scorecard_id": "test-scorecard",
        "base_id": bm["base_id"],
        "candidate_id": cm["base_id"],
        "candidate_patch_id": patch_id,
        "base_foundation_digest": bm["foundation_digest"],
        "candidate_foundation_digest": cm["foundation_digest"],
        "base_dendritic_sha256": bm["dendritic_branches_sha256"],
        "candidate_dendritic_sha256": cm["dendritic_branches_sha256"],
        "base_average": base_avg,
        "candidate_average": cand_avg,
        "delta": cand_avg - base_avg,
        "min_improvement": 0.0,
        "improved_cases": 1,
        "regressed_cases": [],
        "unchanged_cases": 0,
        "failed_cases": failed_cases,
        "promoted": promoted,
        "results": [
            {
                "case_id": "case-a",
                "lesson": "feeding",
                "weight": 1.0,
                "base_score": base_avg,
                "candidate_score": cand_avg,
                "delta": cand_avg - base_avg,
                "base_completed": False,
                "candidate_completed": True,
                "error": None,
            }
        ],
    }
    path.write_text(json.dumps(report, indent=2))
    return path


def run_cli(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(
        [sys.executable, str(LOOP), *args],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
    )
    if check and proc.returncode != 0:
        raise AssertionError(
            f"command {args} failed ({proc.returncode}):\nstdout={proc.stdout}\nstderr={proc.stderr}"
        )
    return proc


def find_bash() -> str | None:
    git_bash = Path(r"C:\Program Files\Git\bin\bash.exe")
    if git_bash.exists():
        return str(git_bash)
    for candidate in ("bash", "sh"):
        resolved = shutil.which(candidate)
        if resolved and "system32" not in resolved.lower():
            return resolved
    return None


class SurgeryLoopTests(unittest.TestCase):
    def test_bootstrap_script_syntax_and_safety_flags(self) -> None:
        self.assertTrue(BOOTSTRAP.exists())
        text = BOOTSTRAP.read_text(encoding="utf-8")
        verify_lines = [
            line.strip()
            for line in text.splitlines()
            if "verify-reopen" in line
            and not line.lstrip().startswith("#")
            and "echo" not in line
        ]
        self.assertTrue(verify_lines)
        for line in verify_lines:
            self.assertIn("--out", line)
            self.assertNotIn("--stamp-bundle", line)
        self.assertIn("--fail-on-stall", text)
        self.assertIn("record-recovery", text)
        if os.name != "nt":
            self.assertTrue(os.access(BOOTSTRAP, os.X_OK))
        bash = find_bash()
        if bash is not None:
            res = subprocess.run([bash, "-n", str(BOOTSTRAP)], text=True, capture_output=True)
            self.assertEqual(res.returncode, 0, res.stderr)

    def test_propose_score_promote_and_byte_identical_backup(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            state = root / "SURGERY_STATE.json"
            base = make_bundle(root / "bases" / "r41", "r41")
            cand = make_bundle(
                root / "cands" / "r42",
                "r42",
                foundation_bytes=b"foundation-r42-improved",
            )
            patch = write_patch(root / "p1.json", "p1", base)
            report = write_report(root / "r1.json", "p1", base, cand)
            backups = root / "backups"

            t0 = "2026-11-26T00:00:00Z"
            run_cli(
                "init",
                "--state",
                str(state),
                "--base-dir",
                str(base),
                "--base-score",
                "50.0",
                "--require-reopen-verified",
                "--now-utc",
                t0,
            )
            run_cli(
                "propose",
                "--state",
                str(state),
                "--owner",
                "astra-1",
                "--patch",
                str(patch),
                "--allow-engine-changes",
                "--now-utc",
                "2026-11-26T00:10:00Z",
            )
            run_cli(
                "record-score",
                "--state",
                str(state),
                "--patch-id",
                "p1",
                "--report",
                str(report),
                "--now-utc",
                "2026-11-26T00:20:00Z",
            )
            cand_manifest_before = (cand / "manifest.json").read_bytes()
            cand_receipt_before = (cand / "reopen_receipt.json").read_bytes()

            run_cli(
                "promote",
                "--state",
                str(state),
                "--patch-id",
                "p1",
                "--new-base-id",
                "r42",
                "--candidate-bundle",
                str(cand),
                "--backup-dir",
                str(backups),
                "--now-utc",
                "2026-11-26T00:30:00Z",
            )

            ledger = json.loads(state.read_text())
            self.assertEqual(ledger["active_base_id"], "r42")
            self.assertEqual(ledger["active_base_score"], 62.0)
            self.assertEqual(
                (backups / "r42" / "manifest.json").read_bytes(),
                cand_manifest_before,
            )
            self.assertEqual(
                (backups / "r42" / "reopen_receipt.json").read_bytes(),
                cand_receipt_before,
            )

    def test_promote_rejects_unscored_or_mismatched_candidate_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            state = root / "SURGERY_STATE.json"
            base = make_bundle(root / "bases" / "r41", "r41")
            scored_cand = make_bundle(
                root / "cands" / "r42",
                "r42",
                foundation_bytes=b"foundation-r42-scored",
            )
            unscored_other = make_bundle(
                root / "cands" / "r42-spoof",
                "r42",
                foundation_bytes=b"foundation-r42-UNSCORED-SPOOF",
            )
            patch = write_patch(root / "p1.json", "p1", base)
            report = write_report(root / "r1.json", "p1", base, scored_cand)

            run_cli(
                "init",
                "--state",
                str(state),
                "--base-dir",
                str(base),
                "--base-score",
                "50.0",
                "--require-reopen-verified",
                "--now-utc",
                "2026-11-26T00:00:00Z",
            )
            run_cli(
                "propose",
                "--state",
                str(state),
                "--owner",
                "astra-1",
                "--patch",
                str(patch),
                "--allow-engine-changes",
                "--now-utc",
                "2026-11-26T00:10:00Z",
            )
            run_cli(
                "record-score",
                "--state",
                str(state),
                "--patch-id",
                "p1",
                "--report",
                str(report),
                "--now-utc",
                "2026-11-26T00:20:00Z",
            )

            bad = run_cli(
                "promote",
                "--state",
                str(state),
                "--patch-id",
                "p1",
                "--new-base-id",
                "r42",
                "--candidate-bundle",
                str(unscored_other),
                "--backup-dir",
                str(root / "backups"),
                check=False,
            )
            self.assertNotEqual(bad.returncode, 0)
            self.assertIn("foundation_digest", bad.stderr)
            ledger = json.loads(state.read_text())
            self.assertEqual(ledger["active_base_id"], "r41")

    def test_promote_rejects_post_score_rename(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            state = root / "SURGERY_STATE.json"
            base = make_bundle(root / "bases" / "r41", "r41")
            cand = make_bundle(
                root / "cands" / "p1",
                "p1",
                foundation_bytes=b"foundation-p1",
            )
            patch = write_patch(root / "p1.json", "p1", base)
            report = write_report(root / "r1.json", "p1", base, cand)

            run_cli(
                "init",
                "--state",
                str(state),
                "--base-dir",
                str(base),
                "--base-score",
                "50.0",
                "--require-reopen-verified",
                "--now-utc",
                "2026-11-26T00:00:00Z",
            )
            run_cli(
                "propose",
                "--state",
                str(state),
                "--owner",
                "astra-1",
                "--patch",
                str(patch),
                "--allow-engine-changes",
                "--now-utc",
                "2026-11-26T00:10:00Z",
            )
            run_cli(
                "record-score",
                "--state",
                str(state),
                "--patch-id",
                "p1",
                "--report",
                str(report),
                "--now-utc",
                "2026-11-26T00:20:00Z",
            )

            bad = run_cli(
                "promote",
                "--state",
                str(state),
                "--patch-id",
                "p1",
                "--new-base-id",
                "r42",
                "--candidate-bundle",
                str(cand),
                "--backup-dir",
                str(root / "backups"),
                check=False,
            )
            self.assertNotEqual(bad.returncode, 0)
            self.assertIn("base_id", bad.stderr)
            ledger = json.loads(state.read_text())
            self.assertEqual(ledger["active_base_id"], "r41")

    def test_propose_after_readonly_recovery_keeps_manifest_hash(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            state = root / "SURGERY_STATE.json"
            base = make_bundle(root / "bases" / "r41", "r41")
            manifest_before = (base / "manifest.json").read_bytes()

            run_cli(
                "init",
                "--state",
                str(state),
                "--base-dir",
                str(base),
                "--base-score",
                "50.0",
                "--require-reopen-verified",
                "--now-utc",
                "2026-11-26T00:00:00Z",
            )
            run_cli(
                "record-recovery",
                "--state",
                str(state),
                "--reopen-receipt",
                str(base / "reopen_receipt.json"),
                "--now-utc",
                "2026-11-26T02:30:00Z",
            )
            self.assertEqual((base / "manifest.json").read_bytes(), manifest_before)

            patch = write_patch(root / "p1.json", "p1", base)
            run_cli(
                "propose",
                "--state",
                str(state),
                "--owner",
                "astra-1",
                "--patch",
                str(patch),
                "--allow-engine-changes",
                "--now-utc",
                "2026-11-26T02:35:00Z",
            )
            ledger = json.loads(state.read_text())
            self.assertEqual(ledger["last_real_score_utc"], "2026-11-26T00:00:00Z")
            self.assertEqual(ledger["last_recovery_utc"], "2026-11-26T02:30:00Z")
            self.assertEqual(len(ledger["in_flight_patches"]), 1)

    def test_concurrent_propose_serializes_same_and_distinct_owners(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            state = root / "SURGERY_STATE.json"
            base = make_bundle(root / "bases" / "r41", "r41")
            run_cli(
                "init",
                "--state",
                str(state),
                "--base-dir",
                str(base),
                "--base-score",
                "50.0",
                "--require-reopen-verified",
                "--now-utc",
                "2026-11-26T00:00:00Z",
            )

            # Two simultaneous proposals from the same owner: exactly one wins, one gets 2.
            p1a = write_patch(root / "p1a.json", "p1a", base)
            p1b = write_patch(root / "p1b.json", "p1b", base)
            procs_same = [
                subprocess.Popen(
                    [
                        sys.executable,
                        str(LOOP),
                        "propose",
                        "--state",
                        str(state),
                        "--owner",
                        "astra-1",
                        "--patch",
                        str(p),
                        "--allow-engine-changes",
                        "--now-utc",
                        "2026-11-26T00:05:00Z",
                    ],
                    cwd=REPO_ROOT,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                )
                for p in (p1a, p1b)
            ]
            codes_same = []
            for proc in procs_same:
                proc.communicate()
                codes_same.append(proc.returncode)
            self.assertEqual(sorted(codes_same), [0, 2])
            ledger_after_same = json.loads(state.read_text())
            self.assertEqual(len(ledger_after_same["in_flight_patches"]), 1)

            # Two simultaneous proposals from distinct owners: both persist without losing updates.
            p2 = write_patch(root / "p2.json", "p2", base)
            p3 = write_patch(root / "p3.json", "p3", base)
            procs_diff = [
                subprocess.Popen(
                    [
                        sys.executable,
                        str(LOOP),
                        "propose",
                        "--state",
                        str(state),
                        "--owner",
                        owner,
                        "--patch",
                        str(p),
                        "--allow-engine-changes",
                        "--now-utc",
                        "2026-11-26T00:06:00Z",
                    ],
                    cwd=REPO_ROOT,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                )
                for owner, p in (("astra-2", p2), ("astra-3", p3))
            ]
            codes_diff = []
            for proc in procs_diff:
                proc.communicate()
                codes_diff.append(proc.returncode)
            self.assertEqual(codes_diff, [0, 0])
            ledger_after_diff = json.loads(state.read_text())
            patch_ids = {row["patch_id"] for row in ledger_after_diff["in_flight_patches"]}
            self.assertEqual(len(patch_ids), 3)
            self.assertTrue({"p2", "p3"}.issubset(patch_ids))

    def test_stall_fuse_blocks_empty_queue_until_recovery(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            state = root / "SURGERY_STATE.json"
            base = make_bundle(root / "bases" / "r41", "r41")
            patch = write_patch(root / "p1.json", "p1", base)

            run_cli(
                "init",
                "--state",
                str(state),
                "--base-dir",
                str(base),
                "--base-score",
                "50.0",
                "--require-reopen-verified",
                "--now-utc",
                "2026-11-26T00:00:00Z",
            )

            stalled_time = "2026-11-26T02:30:00Z"
            wd = run_cli(
                "watchdog",
                "--state",
                str(state),
                "--now-utc",
                stalled_time,
                "--fail-on-stall",
                check=False,
            )
            self.assertEqual(wd.returncode, 2)
            self.assertIn("ROLLBACK_REQUIRED", wd.stdout)

            blocked_propose = run_cli(
                "propose",
                "--state",
                str(state),
                "--owner",
                "astra-1",
                "--patch",
                str(patch),
                "--allow-engine-changes",
                "--now-utc",
                stalled_time,
                check=False,
            )
            self.assertEqual(blocked_propose.returncode, 2)

            run_cli(
                "record-recovery",
                "--state",
                str(state),
                "--reopen-receipt",
                str(base / "reopen_receipt.json"),
                "--now-utc",
                "2026-11-26T02:31:00Z",
            )
            wd_ok = run_cli(
                "watchdog",
                "--state",
                str(state),
                "--now-utc",
                "2026-11-26T02:32:00Z",
                "--fail-on-stall",
            )
            self.assertIn('"status": "HEALTHY"', wd_ok.stdout)
            run_cli(
                "propose",
                "--state",
                str(state),
                "--owner",
                "astra-1",
                "--patch",
                str(patch),
                "--allow-engine-changes",
                "--now-utc",
                "2026-11-26T02:33:00Z",
            )


if __name__ == "__main__":
    unittest.main()

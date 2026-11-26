#!/usr/bin/env python3
"""Unit tests for scripts/surgery_loop.py and cloud bootstrap script portability."""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import surgery_loop as sl


def sha256_bytes(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def make_bundle(root: pathlib.Path, base_id: str, foundation_digest: str = "blake3-base") -> dict:
    root.mkdir(parents=True, exist_ok=True)
    f_bytes = f"foundation-{base_id}-{foundation_digest}".encode("utf-8")
    b_bytes = json.dumps({"branches": []}).encode("utf-8")
    s_bytes = json.dumps({"schema": "alife.brain_surgery.scorecard.v1", "schema_version": 1, "scorecard_id": "sc", "cases": []}).encode("utf-8")
    (root / "foundation.alife-foundation").write_bytes(f_bytes)
    (root / "dendritic_branches.json").write_bytes(b_bytes)
    (root / "scorecard.json").write_bytes(s_bytes)
    manifest = {
        "schema": sl.BUNDLE_MANIFEST_SCHEMA,
        "schema_version": 1,
        "base_id": base_id,
        "parent_base_id": None,
        "foundation_file": "foundation.alife-foundation",
        "foundation_sha256": sha256_bytes(f_bytes),
        "foundation_digest": foundation_digest,
        "sensor_profile": "GroundedTerrainVisionV1",
        "weight_count": 2048,
        "dendritic_branches_file": "dendritic_branches.json",
        "dendritic_branches_sha256": sha256_bytes(b_bytes),
        "dendritic_branch_count": 0,
        "semantic_prior_mode": "deterministic_for_training",
        "semantic_prior_context_contract": "v1",
        "scorecard_file": "scorecard.json",
        "scorecard_sha256": sha256_bytes(s_bytes),
        "reopen_verified": True,
        "reopen_receipt_file": None,
        "reopen_receipt_sha256": None,
    }
    (root / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    return manifest


def make_patch(
    path: pathlib.Path,
    patch_id: str,
    target_base_id: str,
    manifest: dict,
) -> pathlib.Path:
    payload = {
        "schema": sl.PATCH_SCHEMA,
        "schema_version": 1,
        "patch_id": patch_id,
        "target_base_id": target_base_id,
        "target_foundation_digest": manifest["foundation_digest"],
        "target_dendritic_branches_sha256": manifest["dendritic_branches_sha256"],
        "weight_edits": [],
        "dendritic_edits": {"op": "keep"},
    }
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    return path


def make_git_repo(repo_dir: pathlib.Path) -> None:
    (repo_dir / "crates").mkdir(parents=True, exist_ok=True)
    (repo_dir / "crates" / "lib.rs").write_text("// ok\n", encoding="utf-8")
    subprocess.run(["git", "init"], cwd=repo_dir, capture_output=True, check=True)
    subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=repo_dir, check=True)
    subprocess.run(["git", "config", "user.name", "Test"], cwd=repo_dir, check=True)
    subprocess.run(["git", "add", "."], cwd=repo_dir, check=True)
    subprocess.run(["git", "commit", "-m", "init"], cwd=repo_dir, capture_output=True, check=True)


def find_bash() -> str | None:
    if os.name == "nt":
        for candidate in (
            r"C:\Program Files\Git\usr\bin\bash.exe",
            r"C:\Program Files\Git\bin\bash.exe",
        ):
            if pathlib.Path(candidate).is_file():
                return candidate
        return None
    return "bash"


class SurgeryLoopTests(unittest.TestCase):
    def test_propose_score_promote_and_rollback_fuse(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            repo = root / "repo"
            make_git_repo(repo)
            base_dir = root / "bases" / "r41"
            cand_dir = root / "cands" / "p1"
            backups = root / "backups"
            state_path = root / "SURGERY_STATE.json"

            base_manifest = make_bundle(base_dir, "r41", foundation_digest="f_r41")
            sl.main(
                [
                    "init",
                    "--state",
                    str(state_path),
                    "--base-dir",
                    str(base_dir),
                    "--base-score",
                    "92.5",
                    "--repo-root",
                    str(repo),
                    "--max-unscored",
                    "2",
                    "--stall-fuse-hours",
                    "2.0",
                    "--now-utc",
                    "2026-11-25T10:00:00Z",
                ]
            )

            # Reject dirty crates/ mixing engine edits with weight surgery.
            (repo / "crates" / "lib.rs").write_text("// dirty\n", encoding="utf-8")
            p1_path = make_patch(root / "p1.json", "p1", "r41", base_manifest)
            with self.assertRaises(sl.SurgeryLoopError):
                sl.cmd_propose(
                    sl.build_parser().parse_args(
                        [
                            "propose",
                            "--state",
                            str(state_path),
                            "--owner",
                            "astra-1",
                            "--patch",
                            str(p1_path),
                            "--repo-root",
                            str(repo),
                            "--now-utc",
                            "2026-11-25T10:05:00Z",
                        ]
                    )
                )
            (repo / "crates" / "lib.rs").write_text("// ok\n", encoding="utf-8")

            # Reject blind stacking onto a wrong/unscored parent base.
            bad_parent_patch = make_patch(root / "bad.json", "p-bad", "r008", base_manifest)
            with self.assertRaises(sl.SurgeryLoopError):
                sl.cmd_propose(
                    sl.build_parser().parse_args(
                        [
                            "propose",
                            "--state",
                            str(state_path),
                            "--owner",
                            "astra-1",
                            "--patch",
                            str(bad_parent_patch),
                            "--repo-root",
                            str(repo),
                            "--now-utc",
                            "2026-11-25T10:08:00Z",
                        ]
                    )
                )

            # Valid proposal for astra-1 succeeds; second unscored proposal for same owner rejects.
            rc = sl.main(
                [
                    "propose",
                    "--state",
                    str(state_path),
                    "--owner",
                    "astra-1",
                    "--patch",
                    str(p1_path),
                    "--repo-root",
                    str(repo),
                    "--now-utc",
                    "2026-11-25T10:10:00Z",
                ]
            )
            self.assertEqual(rc, 0)

            p1_dup = make_patch(root / "p1_dup.json", "p1-dup", "r41", base_manifest)
            with self.assertRaisesRegex(sl.SurgeryLoopError, "already has an unscored patch"):
                sl.cmd_propose(
                    sl.build_parser().parse_args(
                        [
                            "propose",
                            "--state",
                            str(state_path),
                            "--owner",
                            "astra-1",
                            "--patch",
                            str(p1_dup),
                            "--repo-root",
                            str(repo),
                            "--now-utc",
                            "2026-11-25T10:15:00Z",
                        ]
                    )
                )

            # Record matched report; failed or proxy report rejects.
            bad_report = root / "bad_report.json"
            bad_report.write_text(
                json.dumps(
                    {
                        "schema": sl.REPORT_SCHEMA,
                        "schema_version": 1,
                        "base_id": "r41",
                        "candidate_patch_id": "p1",
                        "base_foundation_digest": "f_r41",
                        "base_dendritic_sha256": base_manifest["dendritic_branches_sha256"],
                        "failed_cases": 1,
                        "base_average": 92.5,
                        "candidate_average": 95.0,
                        "delta": 2.5,
                        "promoted": False,
                        "results": [{"case_id": "c1"}],
                    }
                ),
                encoding="utf-8",
            )
            self.assertEqual(
                sl.main(
                    [
                        "record-score",
                        "--state",
                        str(state_path),
                        "--patch-id",
                        "p1",
                        "--report",
                        str(bad_report),
                    ]
                ),
                2,
            )

            good_report = root / "good_report.json"
            good_report.write_text(
                json.dumps(
                    {
                        "schema": sl.REPORT_SCHEMA,
                        "schema_version": 1,
                        "base_id": "r41",
                        "candidate_patch_id": "p1",
                        "base_foundation_digest": "f_r41",
                        "base_dendritic_sha256": base_manifest["dendritic_branches_sha256"],
                        "failed_cases": 0,
                        "base_average": 92.5,
                        "candidate_average": 94.0,
                        "delta": 1.5,
                        "promoted": True,
                        "results": [{"case_id": "c1"}],
                    }
                ),
                encoding="utf-8",
            )
            self.assertEqual(
                sl.main(
                    [
                        "record-score",
                        "--state",
                        str(state_path),
                        "--patch-id",
                        "p1",
                        "--report",
                        str(good_report),
                        "--now-utc",
                        "2026-11-25T11:00:00Z",
                    ]
                ),
                0,
            )

            # Promote rejects unverified reopen receipt, succeeds when verified=true.
            cand_manifest = make_bundle(cand_dir, "p1", foundation_digest="f_p1")
            unverified_receipt = root / "reopen_bad.json"
            unverified_receipt.write_text(
                json.dumps(
                    {
                        "schema": sl.REOPEN_SCHEMA,
                        "schema_version": 1,
                        "base_id": "p1",
                        "foundation_digest": "f_p1",
                        "dendritic_branches_sha256": cand_manifest["dendritic_branches_sha256"],
                        "dendritic_branch_count": 0,
                        "tick": 1,
                        "verified": False,
                    }
                ),
                encoding="utf-8",
            )
            self.assertEqual(
                sl.main(
                    [
                        "promote",
                        "--state",
                        str(state_path),
                        "--patch-id",
                        "p1",
                        "--new-base-id",
                        "r42",
                        "--candidate-bundle",
                        str(cand_dir),
                        "--reopen-receipt",
                        str(unverified_receipt),
                        "--backup-dir",
                        str(backups),
                    ]
                ),
                2,
            )

            verified_receipt = root / "reopen_ok.json"
            verified_receipt.write_text(
                json.dumps(
                    {
                        "schema": sl.REOPEN_SCHEMA,
                        "schema_version": 1,
                        "base_id": "p1",
                        "foundation_digest": "f_p1",
                        "dendritic_branches_sha256": cand_manifest["dendritic_branches_sha256"],
                        "dendritic_branch_count": 0,
                        "tick": 1,
                        "verified": True,
                    }
                ),
                encoding="utf-8",
            )
            self.assertEqual(
                sl.main(
                    [
                        "promote",
                        "--state",
                        str(state_path),
                        "--patch-id",
                        "p1",
                        "--new-base-id",
                        "r42",
                        "--candidate-bundle",
                        str(cand_dir),
                        "--reopen-receipt",
                        str(verified_receipt),
                        "--backup-dir",
                        str(backups),
                        "--now-utc",
                        "2026-11-25T11:05:00Z",
                    ]
                ),
                0,
            )

            updated = sl.load_state(state_path)
            self.assertEqual(updated["active_base_id"], "r42")
            self.assertEqual(updated["active_base_score"], 94.0)
            self.assertTrue((backups / "r42" / "reopen_receipt.json").is_file())

            # Watchdog reports HEALTHY within fuse window, ROLLBACK_REQUIRED after 2h.
            healthy = sl.compute_watchdog(updated, "2026-11-25T11:30:00Z")
            self.assertEqual(healthy["status"], "HEALTHY")
            stalled = sl.compute_watchdog(updated, "2026-11-25T13:30:00Z")
            self.assertEqual(stalled["status"], "ROLLBACK_REQUIRED")
            self.assertEqual(
                sl.main(
                    [
                        "watchdog",
                        "--state",
                        str(state_path),
                        "--now-utc",
                        "2026-11-25T13:30:00Z",
                        "--fail-on-stall",
                    ]
                ),
                2,
            )

    def test_bootstrap_script_syntax(self) -> None:
        bash = find_bash()
        if bash is None:
            self.skipTest("Git Bash not present on this host")
        script = SCRIPTS_DIR / "bootstrap_cloud_surgery.sh"
        self.assertTrue(script.is_file())
        proc = subprocess.run(
            [bash, "-n", script.as_posix()],
            cwd=SCRIPTS_DIR.parent,
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        self.assertEqual(proc.returncode, 0, msg=proc.stderr)


if __name__ == "__main__":
    unittest.main()

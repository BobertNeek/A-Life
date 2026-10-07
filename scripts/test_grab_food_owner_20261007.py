#!/usr/bin/env python3
"""Harmless CPU fixtures: no provider, GPU, trainer or Cargo process is executed."""
import datetime as dt
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("grab_food_owner", Path(__file__).with_name("run_grab_food_owner_20261007.py"))
owner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(owner)

class OwnerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.config = {
            "source": str(self.root / "source"), "binary": str(self.root / "new-binary"),
            "qualification": str(self.root / "qualification.json"), "source_sha256": {"cycle.json": "fixture"},
            "source_policy_version": 602, "source_actor_age": 1844, "source_value_age": 1204,
            "seed": 202610061277, "assessment_seed": 202610061201, "target_distinct_episodes": 3000,
            "production_budget_mode": "until_work_cutoff", "work_cutoff_utc": "2026-10-07T14:55:00Z",
            "absolute_cutoff_utc": "2026-10-07T15:00:00Z", "owner_locks": [],
            "model_path": "fixture-model", "model_sha256": "fixture", "expected_alias": "fixture-prior",
            "server": "fixture-server", "server_sha256": "fixture", "provider_port": 18081,
            "gpu_layers": 999, "provider_threads": 4, "context_size": 2048, "parallel_slots": 1,
            "cpu_checks": [],
            "full_resource_after_utc": "2026-10-07T07:00:00Z",
            "resource_policy": "explicit revised resource permission fixture",
            "full_admission_limits": {"max_cpu_percent": 90, "max_gpu_percent": 90},
            "admission_limits": {"idle_seconds": 120, "max_cpu_percent": 35, "max_gpu_percent": 20,
                "min_ram_gib": 8, "min_disk_gib": 15, "max_vram_mib": 7168, "max_temperature_c": 80},
        }

    def test_missing_qualification_blocks_before_host_ownership_or_provider(self):
        with (
            patch.object(owner, "assert_source", return_value={}),
            patch.object(owner, "qualification", side_effect=ValueError("CPU qualification blocked")),
            patch.object(owner, "snapshot") as snapshot,
            patch.object(owner, "claim_once") as claim,
            patch.object(subprocess, "Popen") as spawn,
        ):
            with self.assertRaisesRegex(ValueError, "qualification"):
                owner.execute(self.config, self.root / "config.json", {})
            snapshot.assert_not_called(); claim.assert_not_called(); spawn.assert_not_called()

    def test_input_and_open_roblox_allow_limited_work_but_distress_and_competition_block(self):
        snap = {"resources": {"idle_seconds": 150., "foreground_name": "explorer",
            "free_ram_gib": 20., "free_c_gib": 100., "gpu_memory_mib": 1000,
            "gpu_temperature_c": 40., "cpu_load_percent": 10., "gpu_util_percent": 5.},
            "processes": [{"Name": "RobloxPlayerBeta.exe", "ProcessId": 14056, "ParentProcessId": 0}], "service": None}
        now = dt.datetime(2026, 10, 7, 2, tzinfo=dt.timezone.utc)
        with patch.object(owner, "utc", return_value=now):
            owner.check_resources(snap, self.config, admission=True)
            owner.check_resources({**snap, "resources": {**snap["resources"],
                "idle_seconds": 0., "foreground_name": "RobloxPlayerBeta"}}, self.config, admission=True)
        for change in ({"cpu_load_percent": 36.}, {"gpu_util_percent": 21.},
                       {"free_ram_gib": 7.}, {"gpu_temperature_c": 80.}):
            altered = {**snap, "resources": {**snap["resources"], **change}}
            with patch.object(owner, "utc", return_value=now), self.assertRaises(RuntimeError):
                owner.check_resources(altered, self.config, admission=True)
        snap["processes"] = [{"Name": "train_n2048_care.exe", "ProcessId": 999999,
                              "ParentProcessId": 0}]
        with (
            self.assertRaisesRegex(RuntimeError, "competing"),
        ):
            owner.check_resources(snap, self.config)

    def test_full_resources_require_roblox_closed_or_0700_utc_and_keep_safe_reserves(self):
        snap = {"processes": [{"Name": "RobloxPlayerBeta.exe"}], "resources": {
            "idle_seconds": 0., "foreground_name": "RobloxPlayerBeta", "free_ram_gib": 20.,
            "free_c_gib": 100., "gpu_memory_mib": 2000, "gpu_temperature_c": 40.,
            "cpu_load_percent": 60., "gpu_util_percent": 60.}, "service": None}
        early = dt.datetime(2026, 10, 7, 2, tzinfo=dt.timezone.utc)
        with patch.object(owner, "utc", return_value=early):
            self.assertFalse(owner.full_resource_allowed(snap, self.config))
            self.assertTrue(owner.full_resource_allowed({**snap, "processes": []}, self.config))
            with self.assertRaisesRegex(RuntimeError, "busy"):
                owner.check_resources(snap, self.config, admission=True)
        late = dt.datetime(2026, 10, 7, 7, tzinfo=dt.timezone.utc)
        with patch.object(owner, "utc", return_value=late), patch.object(owner, "owned_pids", return_value=set()):
            snap["processes"][0].update(ProcessId=14056, ParentProcessId=0)
            self.assertTrue(owner.full_resource_allowed(snap, self.config))
            owner.check_resources(snap, self.config, admission=True)
            with self.assertRaisesRegex(RuntimeError, "distress"):
                owner.check_resources({**snap, "resources": {**snap["resources"], "free_ram_gib": 7.}}, self.config)

    def test_native_plan_and_run_paths_are_distinct_and_keep_objective2(self):
        with (
            patch.object(owner, "REPO", self.root),
            patch.object(owner, "assert_source"),
            patch.object(owner, "read", return_value={"ready": False, "blocker": "unbuilt"}),
            patch.object(owner.night, "sha256", return_value="fixture"),
        ):
            plan = owner.prepare_plan(self.config, self.root / "config.json", "fixture")
        self.assertNotEqual(plan["plan_output"], plan["run_output"])
        self.assertNotIn("--run", plan["native_plan_argv"])
        self.assertIn("--run", plan["native_run_argv"])
        argv = plan["native_run_argv"]
        self.assertEqual(argv[argv.index("--seed") + 1], "202610061277")
        self.assertEqual(argv[argv.index("--assessment-seed") + 1], "202610061201")
        self.assertNotIn("--preserve-objective-state-from", argv)
        self.assertFalse(plan["training_started"])
        self.assertFalse(plan["native_plan_executed"])
        self.assertEqual(plan["production_budget_mode"], "until_work_cutoff")
        self.assertEqual(argv[argv.index("--cutoff") + 1], "2026-10-07T14:55:00Z")
        self.assertEqual(argv[argv.index("--idle-seconds") + 1], "0")

    def test_duplicate_attempt_stops_before_resource_work(self):
        (self.root / "RUN-ATTEMPT.json").write_text("existing owner marker")
        plan = {"config_sha256": "fixture", "source_sha256": self.config["source_sha256"],
                "prepared_utc": "2026-10-07T01:00:00Z",
                "run_output": str(self.root / "target/founder-training/grab-run"),
                "plan_output": str(self.root / "target/founder-training/grab-plan")}
        now = dt.datetime(2026, 10, 7, 1, tzinfo=dt.timezone.utc)
        with (
            patch.object(owner, "HERE", self.root),
            patch.object(owner, "REPO", self.root),
            patch.object(owner, "utc", return_value=now),
            patch.object(owner, "assert_source", return_value={}),
            patch.object(owner, "qualification", return_value={}),
            patch.object(owner.night, "sha256", return_value="fixture"),
            patch.object(owner, "snapshot") as snap,
        ):
            with self.assertRaisesRegex(RuntimeError, "already claimed"):
                owner.execute(self.config, self.root / "config.json", plan)
            snap.assert_not_called()
        self.assertEqual((self.root / "RUN-ATTEMPT.json").read_text(), "existing owner marker")

    def test_atomic_claim_preserves_existing_marker(self):
        path = self.root / "owner.lock"
        owner.claim_once(path, {"owner": "one"})
        with (
            self.assertRaises(FileExistsError),
        ):
            owner.claim_once(path, {"owner": "two"})
        self.assertEqual(json.loads(path.read_text()), {"owner": "one"})

    def closed_startup(self):
        receipt = self.root / "20261007T000000000000Z-owner-run.json"
        output = self.root / "target/founder-training/previous-startup"
        prior = {"owner_thread": owner.OWNER, "owner_pid": 31777,
                 "status": "blocked_or_stopped", "ended_utc": "2026-10-07T00:10:00Z",
                 "training_process_started": False, "learner_capture_observed": False,
                 "commands": [], "prepared_plan": {
                     "source_sha256": self.config["source_sha256"], "run_output": str(output)}}
        receipt.write_text(json.dumps(prior))
        marker = self.root / "RUN-ATTEMPT.json"
        marker.write_text(json.dumps({"owner_thread": owner.OWNER, "pid": 31777,
            "receipt": str(receipt), "source_sha256": self.config["source_sha256"]}))
        return receipt, marker, prior

    def test_explicit_zero_dose_startup_recovery_archives_marker_without_deletion(self):
        receipt, marker, _ = self.closed_startup()
        original = marker.read_bytes()
        with patch.object(owner, "HERE", self.root), patch.object(owner, "REPO", self.root):
            with self.assertRaisesRegex(RuntimeError, "already claimed"):
                owner.startup_recovery_reference(self.config, None)
            reference = owner.startup_recovery_reference(self.config, receipt)
            owner.check_recovery_processes(reference, {"processes": []})
            result = owner.archive_startup_recovery(reference, "recovery-fixture", self.root / "new-owner-run.json")
        self.assertEqual(Path(result["archived_marker"]).read_bytes(), original)
        self.assertFalse(marker.exists())
        self.assertTrue(receipt.is_file())
        self.assertFalse(owner.read(result["receipt"])["training_state_changed"])

    def test_started_training_and_partial_checkpoint_are_not_startup_recovery(self):
        receipt, _, prior = self.closed_startup()
        with patch.object(owner, "HERE", self.root), patch.object(owner, "REPO", self.root):
            receipt.write_text(json.dumps({**prior, "training_process_started": True}))
            with self.assertRaisesRegex(ValueError, "zero-training-dose"):
                owner.startup_recovery_reference(self.config, receipt)
            receipt.write_text(json.dumps(prior))
            partial = Path(prior["prepared_plan"]["run_output"]) / "episode-00000"
            partial.mkdir(parents=True)
            (partial / "trained.alife-foundation").write_bytes(b"possible trained state; do not discard")
            with self.assertRaisesRegex(ValueError, "checkpoint artifacts"):
                owner.startup_recovery_reference(self.config, receipt)

    def test_active_previous_pid_blocks_recovery(self):
        receipt, _, _ = self.closed_startup()
        with patch.object(owner, "HERE", self.root), patch.object(owner, "REPO", self.root):
            reference = owner.startup_recovery_reference(self.config, receipt)
        with self.assertRaisesRegex(RuntimeError, "remains active"):
            owner.check_recovery_processes(reference, {"processes": [{"ProcessId": 31777}]})

    def test_external_prior_identity_must_remain_the_same(self):
        snap = {"resources": {"idle_seconds": 150., "foreground_name": "explorer",
            "free_ram_gib": 20., "free_c_gib": 100., "gpu_memory_mib": 1000,
            "gpu_temperature_c": 40., "cpu_load_percent": 10., "gpu_util_percent": 5.},
            "processes": [], "service": {"pid": 50, "created": "new"}}
        with (
            self.assertRaisesRegex(RuntimeError, "identity changed"),
        ):
            owner.check_resources(snap, self.config, {"pid": 50, "created": "old"})

if __name__ == "__main__":
    unittest.main()

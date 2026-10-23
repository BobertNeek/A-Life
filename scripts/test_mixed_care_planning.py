"""Planning verifies complete history once and execution still checks it afresh."""
import contextlib
import datetime as dt
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import run_mixed_care_serial as mixed


class MixedPlanningTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.source = self.root / "source"
        self.source.mkdir()
        self.output = self.root / "planned"
        self.binary = self.root / "fixture-binary"
        self.binary.write_bytes(b"host-only fixture")
        self.receipt = {"seed": 90, "policy_version": 10, "biological_objective_version": 2,
                        "objective_state_reset": False}
        for name in mixed.FILES:
            data = self.receipt if name == "cycle.json" else {"fixture": name}
            (self.source / name).write_text(json.dumps(data))
        self.evidence = self.root / "immutable-evidence.json"
        self.evidence.write_text('{"operator":"synthetic recovery evidence"}')
        self.history = self.root / "history.json"
        self.document = {
            "schema": 1, "kind": "mixed-care-seed-history", "complete": False,
            "seed_admission": {"policy": "known-exclusions", "historical_freshness": "unknown"},
            "source_sha256": mixed.file_hashes(self.source), "reserved_seed_ranges": [],
            "consumed_seed_ranges": [[90, 90]], "assessment_seeds": [], "imports": [],
            "missing_history": "Synthetic host test with explicitly incomplete earlier history.",
            "recovery_reconciliation": {
                "accepted_checkpoint_authoritative": True,
                "known_inflight_attempts_reconciled_or_excluded": True,
                "no_known_detached_later_accepted_chain": True,
                "explanation": "Synthetic exact-byte evidence, never a claim of historical freshness.",
                "prior_ledger_runs_sha256": mixed.object_hash([]),
                "evidence": [{"path": str(self.evidence), "sha256": mixed.host.sha256(self.evidence)}],
            },
        }
        self.save_history()
        self.schedule = self.root / "schedule.json"
        mixed.seeds.write_document(self.schedule, {"schema": 1, "steps": [
            {"lesson": "feeding", "episodes": 1, "decisions": 2}]})
        self.cutoff = (dt.datetime.now(mixed.host.UTC) + dt.timedelta(hours=1)).isoformat()
        self.argv = ["--binary", str(self.binary), "--checkpoint-inspector", str(self.binary),
                     "--source", str(self.source), "--output", str(self.output),
                     "--schedule", str(self.schedule), "--seed-history", str(self.history),
                     "--seed-admission", "known-exclusions", "--seed", "100",
                     "--cutoff", self.cutoff, "--keep-checkpoints", "3"]
        self.git = patch.object(mixed.subprocess, "check_output", return_value="fixture-head\n")
        self.git.start()
        self.addCleanup(self.git.stop)

    def save_history(self):
        mixed.seeds.write_document(self.history, self.document)

    def plan(self, argv=None):
        stderr = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(stderr):
            result = mixed.main(self.argv if argv is None else argv)
        return result, stderr.getvalue()

    def assert_no_plan_or_reservation(self):
        self.assertFalse(self.output.exists())
        self.assertFalse((self.root / mixed.LEDGER).exists())

    def test_planning_hashes_complete_recovery_evidence_once(self):
        original = mixed.host.sha256
        reads = []
        def counted(path):
            if Path(path).resolve() == self.evidence.resolve():
                reads.append(Path(path))
            return original(path)
        with patch.object(mixed.host, "sha256", side_effect=counted), \
                patch.object(mixed, "execute", side_effect=AssertionError("planning dispatched")) as execute:
            self.assertEqual(self.plan()[0], 0)
        self.assertEqual([path.resolve() for path in reads], [self.evidence.resolve()])
        execute.assert_not_called()
        plan = mixed.load(self.output / "plan.json")[0]
        self.assertEqual(plan["retention"], {"mode": "recent-checkpoints", "keep": 3})
        self.assertEqual(plan["source_receipt"], self.receipt)
        self.assertEqual(plan["history_sha256"], original(self.history))
        self.assertFalse((self.root / mixed.LEDGER).exists())

    def test_corrupted_evidence_rejects_before_plan_or_reservation(self):
        self.evidence.write_text("changed evidence")
        result, error = self.plan()
        self.assertEqual(result, 1)
        self.assertIn("recovery evidence changed", error)
        self.assert_no_plan_or_reservation()

    def test_history_change_between_binding_and_validation_is_rejected(self):
        original = mixed.reserve
        def changed(plan, path, **options):
            self.document["missing_history"] += " Changed after the initial digest."
            self.save_history()
            return original(plan, path, **options)
        with patch.object(mixed, "reserve", side_effect=changed):
            result, error = self.plan()
        self.assertEqual(result, 1)
        self.assertIn("history changed since planning", error)
        self.assert_no_plan_or_reservation()

    def test_consumed_seed_exclusion_remains_enforced(self):
        argv = list(self.argv)
        argv[argv.index("--seed") + 1] = "90"
        result, error = self.plan(argv)
        self.assertEqual(result, 1)
        self.assertIn("allocation overlaps excluded seeds", error)
        self.assert_no_plan_or_reservation()

    def test_wrong_source_binding_remains_rejected(self):
        self.document["source_sha256"]["trained.alife-foundation"] = "0" * 64
        self.save_history()
        result, error = self.plan()
        self.assertEqual(result, 1)
        self.assertIn("history is not bound to this sealed source", error)
        self.assert_no_plan_or_reservation()

    def test_admission_policy_mismatch_remains_rejected(self):
        self.document["complete"] = True
        self.save_history()
        result, error = self.plan()
        self.assertEqual(result, 1)
        self.assertIn("known-exclusions requires", error)
        self.assert_no_plan_or_reservation()

    def test_direct_reservation_independently_revalidates_evidence(self):
        self.assertEqual(self.plan()[0], 0)
        plan = mixed.load(self.output / "plan.json")[0]
        self.evidence.write_text("changed after planning")
        with self.assertRaisesRegex(ValueError, "recovery evidence changed"):
            mixed.reserve(plan, self.root / mixed.LEDGER, commit=False)
        self.assertFalse((self.root / mixed.LEDGER).exists())

    def test_execution_independently_revalidates_evidence(self):
        self.assertEqual(self.plan()[0], 0)
        self.evidence.write_text("changed after planning")
        with patch.object(mixed, "admit_run_context"), patch.object(mixed.host, "assert_lane_idle"), \
                patch.object(mixed, "inspect", side_effect=AssertionError("native inspection started")) as inspect, \
                patch.object(mixed.host, "run_child", side_effect=AssertionError("native training started")) as trainer:
            result, error = self.plan(["--output", str(self.output), "--start", "--run",
                                       "--cutoff", self.cutoff])
        self.assertEqual(result, 1)
        self.assertIn("recovery evidence changed", error)
        inspect.assert_not_called()
        trainer.assert_not_called()
        self.assertFalse((self.root / mixed.LEDGER).exists())


if __name__ == "__main__":
    unittest.main()

"""Pure comparison preflight/accounting tests; these do not execute neural training."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import compare_cpu_care_rounds as compare


class ComparisonPreflightTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.source = self.root / "synthetic-source"
        self.source.mkdir()
        self.write(self.source / "cpu-cycle.json", {"seed": 20, "fixture": True})
        self.write(self.source / "actor-checkpoint.json", {"optimizer_step": 7})
        self.write(self.source / "value-checkpoint.json", {"optimizer_step": 8})
        (self.source / "trained.alife-foundation").write_bytes(b"synthetic codec stand-in; never executed")
        self.binary = self.root / "unexecuted-binary"
        self.binary.write_bytes(b"pure preflight fixture; not executable")
        self.history = self.root / "history.json"
        self.document = {"schema": 1, "kind": "mixed-care-seed-history", "complete": True,
                         "evidence": "Synthetic preflight attestation; no real lineage or competence claim",
                         "source_sha256": compare.native_pins(self.source), "imports": [],
                         "reserved_seed_ranges": [], "consumed_seed_ranges": [[20, 20]],
                         "assessment_seeds": [90]}
        self.write(self.history, self.document)
        self.output = self.root / "fresh-comparison"
        self.spec_path = self.root / "recipe.json"
        self.spec = {"schema": 1, "previous": str(self.source), "seed_history": str(self.history),
                     "output": str(self.output), "care_binary": str(self.binary), "round_binary": str(self.binary),
                     "temperature": 1.0,
                     "rounds": [[{"collector_run_id": "b", "seed": 30, "lesson": "eat_held_food", "decisions": 2},
                                 {"collector_run_id": "a", "seed": 31, "lesson": "eat_reachable_food", "decisions": 3}],
                                [{"collector_run_id": "a", "seed": 32, "lesson": "feeding", "decisions": 2}]],
                     "assessments": [{"lesson": "eat_reachable_food", "seed": 90, "decisions": 4}]}

    def write(self, path, value):
        path.write_text(json.dumps(value), encoding="utf-8")

    def plan(self, spec=None):
        self.write(self.spec_path, self.spec if spec is None else spec)
        return compare.build_plan(self.spec_path)

    def test_preflight_is_read_only_canonical_and_preserves_existing_attestation(self):
        self.write(self.spec_path, self.spec)
        before = compare.raw_tree(self.root)
        plan = compare.build_plan(self.spec_path)
        self.assertEqual(compare.raw_tree(self.root), before)
        self.assertEqual([row["collector_run_id"] for row in plan["spec"]["rounds"][0]], ["a", "b"])
        self.assertEqual(plan["spec"]["window_ticks"], 32)
        self.assertTrue(plan["initial_fixture"])
        self.assertFalse(self.output.exists())
        generated = compare.episode_history(plan, self.source, [30, 31])
        self.assertEqual(generated["complete"], self.document["complete"])
        self.assertEqual(generated["evidence"], self.document["evidence"])
        self.assertEqual(generated["consumed_seed_ranges"], [[20, 20], [30, 31]])
        self.assertEqual(generated["assessment_seeds"], [90])
        self.assertEqual(generated["imports"][0]["path"], str(self.history))

    def test_duplicate_training_or_heldout_seeds_and_case_aliases_fail(self):
        for change in (lambda spec: spec["rounds"][1][0].update(seed=30),
                       lambda spec: spec["assessments"][0].update(seed=31),
                       lambda spec: spec["rounds"][0][1].update(collector_run_id="B")):
            with self.subTest(change=change):
                spec = copy.deepcopy(self.spec)
                change(spec)
                with self.assertRaises(ValueError):
                    self.plan(spec)
                self.assertFalse(self.output.exists())

    def test_unsupported_budget_and_unknown_history_are_rejected(self):
        for change in (lambda spec: spec.update(effective_batch_size=2),
                       lambda spec: spec.update(burn_in=1),
                       lambda spec: spec["rounds"][0][0].update(decisions=33)):
            spec = copy.deepcopy(self.spec)
            change(spec)
            with self.assertRaises(ValueError):
                self.plan(spec)
        self.document.update(complete=False, seed_admission={"policy": "known-exclusions", "historical_freshness": "unknown"})
        self.write(self.history, self.document)
        with self.assertRaisesRegex(ValueError, "unknown history is unsupported"):
            self.plan()

    def test_source_binding_exclusions_and_raw_import_changes_fail(self):
        self.document["source_sha256"]["cpu-cycle.json"] = "0" * 64
        self.write(self.history, self.document)
        with self.assertRaisesRegex(ValueError, "exact native source"):
            self.plan()
        self.document["source_sha256"] = compare.native_pins(self.source)
        imported = self.root / "import.json"
        self.write(imported, {"schema": 1, "training_seed_ranges": [[30, 30]], "assessment_seeds": []})
        self.document["imports"] = [{"path": str(imported), "sha256": compare.file_pin(imported)["sha256"]}]
        self.write(self.history, self.document)
        with self.assertRaisesRegex(ValueError, "training seed overlaps"):
            self.plan()
        imported.write_text(imported.read_text() + "\n")
        with self.assertRaisesRegex(ValueError, "raw bytes changed"):
            self.plan()

    def test_path_overlap_and_existing_interrupted_outputs_fail(self):
        for output in (self.source / "nested", self.root):
            spec = {**self.spec, "output": str(output)}
            with self.assertRaises(ValueError):
                self.plan(spec)
        self.output.mkdir()
        (self.output / "private-checkpoint.json").write_text("retained interrupted fixture")
        with self.assertRaisesRegex(ValueError, "interrupted outputs are retained"):
            self.plan()
        self.assertEqual((self.output / "private-checkpoint.json").read_text(), "retained interrupted fixture")

    def test_binary_or_original_source_mutation_is_detected_before_execute_creates_output(self):
        plan = self.plan()
        self.binary.write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "pinned recipe/binary/history/helper"):
            compare.execute(plan)
        self.assertFalse(self.output.exists())

    def test_serial_audit_snapshot_keeps_old_bytes_while_admission_names_live_ledger(self):
        plan = self.plan()
        ledger, snapshot = self.root / "live-ledger.json", self.root / "dispatch.ledger.json"
        self.write(ledger, {"schema": 1, "training_seed_ranges": [[30, 30]]})
        original = ledger.read_bytes()
        generated = compare.episode_history(plan, self.source, [30], ledger=ledger, snapshot=snapshot)
        self.assertEqual(generated["imports"][-1]["path"], str(ledger))
        self.assertEqual(snapshot.read_bytes(), original)
        self.write(ledger, {"schema": 1, "training_seed_ranges": [[30, 31]]})
        self.assertEqual(snapshot.read_bytes(), original)
        self.assertNotEqual(compare.file_pin(ledger)["sha256"], generated["imports"][-1]["sha256"])

    def test_source_ancestry_limits_are_checked_before_parsing_receipts(self):
        lineage = self.source / "source-lineage"
        lineage.mkdir()
        oversized = lineage / "large.json"
        with oversized.open("wb") as stream:
            stream.truncate(16 * compare.MIB)
        with patch.object(compare, "read_json", side_effect=AssertionError("must preflight before parsing")):
            with self.assertRaisesRegex(ValueError, "exceeds16 MiB"):
                compare.source_seed_evidence(self.source)
        oversized.unlink()
        for index in range(124):
            (lineage / f"{index}.json").write_text("{}")
        with patch.object(compare, "read_json", side_effect=AssertionError("must preflight before parsing")):
            with self.assertRaisesRegex(ValueError, "exceeds124"):
                compare.source_seed_evidence(self.source)


class BudgetAccountingTests(unittest.TestCase):
    def arms(self):
        step = {"training_rows": 2, "windows": 1, "actor_optimizer_updates": 2,
                "value_optimizer_updates": 2, "ppo_config": {"epochs": 2, "temperature": 1.0},
                "settings": dict(compare.SETTINGS), "stopped_for_kl": False,
                "prior_provider_identity": "same-explicit-provider"}
        row = {"round": 0, "requested_training_rows": 2, "steps": [step]}
        return {"cooperative": {"rounds": [copy.deepcopy(row)]}, "serial": {"rounds": [copy.deepcopy(row)]}}

    def test_actual_budget_config_and_kl_differences_cannot_be_labelled_matched(self):
        arms = self.arms()
        self.assertTrue(compare.budget_comparison(arms)["budget_matched"])
        for change in (lambda step: step.update(actor_optimizer_updates=1),
                       lambda step: step.update(stopped_for_kl=True),
                       lambda step: step["ppo_config"].update(epochs=3),
                       lambda step: step.update(windows=2),
                       lambda step: step.update(prior_provider_identity="different")):
            arms = self.arms()
            change(arms["serial"]["rounds"][0]["steps"][0])
            self.assertFalse(compare.budget_comparison(arms)["budget_matched"])

    def test_f32_equivalent_json_presentations_match_but_adjacent_values_do_not(self):
        arms = self.arms()
        compact = {"epochs": 2, "gamma": 0.99, "optimizer": {"epsilon": 1e-8}, "flags": [True, "fixed"]}
        expanded = {"epochs": 2, "gamma": 0.9900000095367432,
                    "optimizer": {"epsilon": 9.99999993922529e-9}, "flags": [True, "fixed"]}
        arms["cooperative"]["rounds"][0]["steps"][0]["ppo_config"] = compact
        arms["serial"]["rounds"][0]["steps"][0]["ppo_config"] = expanded
        self.assertNotEqual(compact, expanded)
        self.assertTrue(compare.budget_comparison(arms)["budget_matched"])
        self.assertEqual(arms["serial"]["rounds"][0]["steps"][0]["ppo_config"], expanded)
        expanded["gamma"] = 0.9900000691413879  # Next representable f32 above compact gamma.
        self.assertFalse(compare.budget_comparison(arms)["budget_matched"])
        self.assertNotEqual(compare.f32_config_identity(True), compare.f32_config_identity(1))
        self.assertNotEqual(compare.f32_config_identity(1), compare.f32_config_identity(1.0))
        self.assertNotEqual(compare.f32_config_identity(-0.0), compare.f32_config_identity(0.0))


if __name__ == "__main__":
    unittest.main()

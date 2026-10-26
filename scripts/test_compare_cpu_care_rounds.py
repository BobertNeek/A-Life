"""Pure comparison preflight/accounting tests; these do not execute neural training."""
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

import compare_cpu_care_rounds as compare
import run_mixed_care_serial as mixed


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

    def plan(self, spec=None, *, known_exclusions=False):
        self.write(self.spec_path, self.spec if spec is None else spec)
        return compare.build_plan(self.spec_path, known_exclusions=known_exclusions)

    def unknown_history(self):
        evidence = self.root / "retained-recovery.json"
        self.write(evidence, {"schema": 1, "kind": "synthetic-recovery-evidence",
                              "explanation": "Development fixture only; source and old history are synthetic"})
        self.document.update(complete=False, seed_admission=dict(compare.KNOWN),
                             missing_history="Synthetic fixture deliberately has incomplete historical records",
                             recovery_reconciliation={
                                 "accepted_checkpoint_authoritative": True,
                                 "known_inflight_attempts_reconciled_or_excluded": True,
                                 "no_known_detached_later_accepted_chain": True,
                                 "explanation": "Explicit isolated synthetic source and fresh CPU-only lanes",
                                 "evidence": [{"path": str(evidence), "sha256": compare.file_pin(evidence)["sha256"]}],
                                 "prior_ledger_runs_sha256": mixed.object_hash([])})
        self.write(self.history, self.document)
        return evidence

    def test_unknown_requires_explicit_opt_in_and_preserves_incomplete_recovery_facts(self):
        self.unknown_history()
        with self.assertRaisesRegex(ValueError, "unknown history is unsupported"):
            self.plan()
        self.write(self.spec_path, self.spec)
        before = compare.raw_tree(self.root)
        plan = compare.build_plan(self.spec_path, known_exclusions=True)
        self.assertEqual(compare.raw_tree(self.root), before)
        self.assertEqual(plan["seed_admission"], compare.KNOWN)
        self.assertEqual(plan["claim_scope"], compare.UNKNOWN_CLAIM_SCOPE)
        generated = compare.episode_history(plan, self.source, [30, 31])
        for key in ("complete", "seed_admission", "missing_history", "recovery_reconciliation"):
            self.assertEqual(generated[key], self.document[key])
        self.assertEqual(plan["missing_history"], self.document["missing_history"])
        self.assertTrue(all(str(Path(plan["admission_checkout"]["root"]) / "scripts" / name)
                            in plan["immutable_files"] for name in compare.ADMISSION_HELPERS))
        self.assertFalse(self.output.exists())

    def test_unknown_does_not_relabel_complete_or_falsely_certified_history(self):
        with self.assertRaisesRegex(ValueError, "incomplete history"):
            self.plan(known_exclusions=True)
        self.unknown_history()
        for change in (lambda: self.document.update(complete=True),
                       lambda: self.document.update(seed_admission=dict(compare.STRICT))):
            original = copy.deepcopy(self.document)
            change()
            self.write(self.history, self.document)
            with self.assertRaisesRegex(ValueError, "truthful unknown freshness"):
                self.plan(known_exclusions=True)
            self.document = original
        with self.assertRaisesRegex(ValueError, "explicit boolean"):
            compare.build_plan(self.spec_path, known_exclusions="true")

    def test_unknown_cannot_bypass_missing_recovery_or_retarget_prior_lane_proof(self):
        self.unknown_history()
        original = copy.deepcopy(self.document)
        for change in (lambda: self.document.pop("missing_history"),
                       lambda: self.document["recovery_reconciliation"].update(accepted_checkpoint_authoritative=False),
                       lambda: self.document["recovery_reconciliation"].update(prior_ledger_runs_sha256="a" * 64),
                       lambda: self.document["recovery_reconciliation"]["evidence"][0].update(format="manifest")):
            self.document = copy.deepcopy(original)
            change()
            self.write(self.history, self.document)
            with self.assertRaises(ValueError):
                self.plan(known_exclusions=True)
            self.assertFalse(self.output.exists())

    def test_unknown_recovery_ranges_and_imported_reservations_remain_exclusionary(self):
        evidence = self.unknown_history()
        self.write(evidence, {"schema": 1, "interrupted_seed_ranges": [[30, 30]]})
        self.document["recovery_reconciliation"]["evidence"][0]["sha256"] = compare.file_pin(evidence)["sha256"]
        self.write(self.history, self.document)
        with self.assertRaisesRegex(ValueError, "training seed overlaps"):
            self.plan(known_exclusions=True)
        self.write(evidence, {"schema": 1, "training_seed_ranges": [[90, 90]]})
        self.document["recovery_reconciliation"]["evidence"][0]["sha256"] = compare.file_pin(evidence)["sha256"]
        self.write(self.history, self.document)
        with self.assertRaisesRegex(ValueError, "assessment overlaps known"):
            self.plan(known_exclusions=True)
        imported = self.root / "old-attempts.json"
        self.write(imported, {"schema": 1, "reserved_seed_ranges": [[30, 30]]})
        self.write(evidence, {"schema": 1})
        self.document["recovery_reconciliation"]["evidence"][0]["sha256"] = compare.file_pin(evidence)["sha256"]
        self.document["imports"] = [{"path": str(imported), "sha256": compare.file_pin(imported)["sha256"]}]
        self.write(self.history, self.document)
        with self.assertRaisesRegex(ValueError, "training seed overlaps"):
            self.plan(known_exclusions=True)

    def test_unknown_scans_all_bounded_json_proof_presentations_and_rejects_untagged_archives(self):
        original = self.unknown_history()
        alternate = self.root / "plain-recovery.txt"
        document = {"schema": 1, "training_seed_ranges": [[30, 30]]}
        presentations = [b" " * 8192 + json.dumps(document).encode(),
                         b"\xef\xbb\xbf" + json.dumps(document).encode(),
                         json.dumps([document]).encode(),
                         json.dumps(document).encode("utf-16")]
        for raw in presentations:
            with self.subTest(raw=raw[:12]):
                alternate.write_bytes(raw)
                self.document["recovery_reconciliation"]["evidence"] = [
                    {"path": str(alternate), "sha256": compare.file_pin(alternate)["sha256"]}]
                self.write(self.history, self.document)
                with self.assertRaisesRegex(ValueError, "training seed overlaps"):
                    self.plan(known_exclusions=True)
        alternate.write_bytes(json.dumps(document).encode() + b"\xff")
        self.document["recovery_reconciliation"]["evidence"][0]["sha256"] = compare.file_pin(alternate)["sha256"]
        self.write(self.history, self.document)
        with self.assertRaisesRegex(ValueError, "invalid structured recovery evidence encoding"):
            self.plan(known_exclusions=True)
        for kind in (mixed.ledger_store.MANIFEST, mixed.ledger_store.SEGMENT):
            self.write(alternate, {"schema": 1, "kind": kind})
            self.document["recovery_reconciliation"]["evidence"][0]["sha256"] = compare.file_pin(alternate)["sha256"]
            self.write(self.history, self.document)
            with self.assertRaisesRegex(ValueError, "manifest/segment"):
                self.plan(known_exclusions=True)
            self.document["imports"] = [{"path": str(alternate), "sha256": compare.file_pin(alternate)["sha256"]}]
            self.write(self.history, self.document)
            with self.assertRaisesRegex(ValueError, "manifest/segment"):
                self.plan(known_exclusions=True)
            self.document["imports"] = []
        self.write(alternate, {"schema": 1, "imports": [{"path": str(original),
                                                           "sha256": compare.file_pin(original)["sha256"]}]})
        self.document["recovery_reconciliation"]["evidence"][0]["sha256"] = compare.file_pin(alternate)["sha256"]
        self.write(self.history, self.document)
        with self.assertRaisesRegex(ValueError, "must already be exact-byte history imports"):
            self.plan(known_exclusions=True)

    def test_unknown_recovery_proof_is_raw_pinned_before_output_or_native_mutation(self):
        evidence = self.unknown_history()
        plan = self.plan(known_exclusions=True)
        evidence.write_bytes(evidence.read_bytes() + b"\n")
        with self.assertRaisesRegex(ValueError, "pinned recipe/binary/history/helper"):
            compare.execute(plan)
        self.assertFalse(self.output.exists())
        with self.assertRaisesRegex(ValueError, "recovery evidence raw bytes changed"):
            self.plan(known_exclusions=True)

    def test_unknown_native_git_context_keeps_process_and_prior_cache_isolated(self):
        self.unknown_history()
        plan = self.plan(known_exclusions=True)
        self.output.mkdir()
        runner = compare.Runner(plan)
        cwd = self.output / "private-process"
        cwd.mkdir()
        calls = []

        def invoke(argv, **kwargs):
            calls.append((argv, kwargs))
            kwargs["stdout"].write(b"{}")
            return SimpleNamespace(returncode=0)

        with patch.object(compare, "qualified_checkout", return_value=plan["admission_checkout"]), \
                patch.object(compare.subprocess, "run", side_effect=invoke), \
                patch.dict(compare.os.environ, {"GIT_DIR": "hostile", "GIT_WORK_TREE": "hostile",
                                                "ALIFE_SLM_PRIOR_BACKEND": "local"}):
            runner.command(self.binary, ["plan"], cwd)
        environment = calls[0][1]["env"]
        self.assertEqual(calls[0][1]["cwd"], cwd)
        self.assertNotEqual(str(cwd), plan["admission_checkout"]["root"])
        self.assertEqual(environment["GIT_DIR"], plan["admission_checkout"]["git_dir"])
        self.assertEqual(environment["GIT_WORK_TREE"], plan["admission_checkout"]["root"])
        self.assertEqual(environment["ALIFE_SLM_PRIOR_BACKEND"], "deterministic")
        receipt = compare.read_json(runner.logs / "000.argv.json")
        self.assertEqual(receipt["seed_admission"], compare.KNOWN)
        self.assertEqual(receipt["claim_scope"], compare.UNKNOWN_CLAIM_SCOPE)

    @unittest.skipUnless(os.name == "nt", "Windows verbatim path identity")
    def test_qualified_checkout_accepts_windows_verbatim_script_path(self):
        ordinary = compare.qualified_checkout()
        script = str(Path(compare.__file__).resolve())
        verbatim = script if script.startswith("\\\\?\\") else "\\\\?\\" + script
        with patch.object(compare, "__file__", verbatim):
            qualified = compare.qualified_checkout()
        self.assertEqual(qualified, ordinary)
        self.assertTrue(Path(qualified["root"]).samefile(Path(script).parent.parent))

    def test_qualified_checkout_rejects_a_different_existing_directory(self):
        actual = compare.qualified_checkout()
        result = SimpleNamespace(returncode=0, stdout=str(self.root) + "\n" + actual["git_dir"] + "\n")
        with patch.object(compare.subprocess, "run", return_value=result):
            with self.assertRaisesRegex(ValueError, "does not match this runner"):
                compare.qualified_checkout()

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

    def pack_receipts(self, documents):
        import hashlib
        import struct
        lineage = self.source / "source-lineage"
        lineage.mkdir(exist_ok=True)
        raw = compare.PACK_MAGIC + struct.pack("<Q", len(documents))
        for document in documents:
            receipt = json.dumps(document).encode()
            raw += hashlib.sha256(receipt).hexdigest().encode() + struct.pack("<Q", len(receipt)) + receipt
        raw += compare.PACK_END
        digest = hashlib.sha256(raw).hexdigest()
        path = lineage / (digest + ".pack")
        path.write_bytes(raw)
        self.write(lineage / "manifest.json", {"schema_version": 1, "chunks": [{"sha256": digest, "receipts": len(documents)}]})
        return path

    def test_packed_receipts_preserve_all_seed_exclusions_and_reject_corruption(self):
        documents = [{"seed": 567}, {"assessment_seeds": [778899], "arbitrary_history": {"training_seeds": [998877]}}]
        path = self.pack_receipts(documents)
        evidence = compare.source_seed_evidence(self.source)
        self.assertEqual(evidence[1:], documents)
        original = path.read_bytes()
        path.unlink()
        with self.assertRaises(ValueError):
            compare.source_seed_evidence(self.source)
        path.write_bytes(original[:-1] + b"!")
        with self.assertRaisesRegex(ValueError, "object hash"):
            compare.source_seed_evidence(self.source)
        path.write_bytes(original)
        (path.parent / "manifest.json").unlink()
        with self.assertRaisesRegex(ValueError, "no manifest"):
            compare.source_seed_evidence(self.source)

    def test_packed_historical_seeds_still_exclude_training_and_assessment(self):
        path = self.pack_receipts([{"seed": 30}])
        with self.assertRaisesRegex(ValueError, "training seed overlaps"):
            self.plan()
        path.unlink()
        self.pack_receipts([{"seed": 90}])
        with self.assertRaisesRegex(ValueError, "assessment overlaps known"):
            self.plan()

    def test_packed_preflight_rejects_count_and_bytes_before_receipt_decode(self):
        path = self.pack_receipts([{"seed": 1}])
        with path.open("ab") as stream:
            stream.truncate(17 * compare.MIB)
        with patch.object(compare, "decode_json", wraps=compare.decode_json) as decode:
            with self.assertRaisesRegex(ValueError, "exceeds16 MiB"):
                compare.source_seed_evidence(self.source)
            self.assertEqual(decode.call_count, 1)  # bounded manifest only
        manifest = path.parent / "manifest.json"
        document = compare.read_json(manifest)
        document["chunks"][0]["receipts"] = 128
        self.write(manifest, document)
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

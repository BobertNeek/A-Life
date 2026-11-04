"""Synthetic evidence tests; the separate pilot uses actual native executables."""
import copy
import hashlib
import json
import os
from pathlib import Path, PureWindowsPath
import tempfile
import unittest
import subprocess
import sys
from unittest.mock import patch

import care_frontier_sampler as sampler
import run_care_frontier_fixture as fixture_runner
from run_care_frontier_fixture import Pilot


class FixtureDiscoveryCliTests(unittest.TestCase):
    def test_cli_loads_discovery_array_and_passes_it_to_pilot(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bank_path = root / "bank.json"
            discovery_path = root / "discovery.json"
            bank_path.write_text(json.dumps({"test": "bank"}), encoding="utf-8")
            discovery = [{"family": "a", "seed": 301}]
            discovery_path.write_text(json.dumps(discovery), encoding="utf-8")
            argv = ["fixture", "--output", str(root / "out"), "--care-binary", "care",
                    "--round-binary", "round", "--bank", str(bank_path),
                    "--discovery", str(discovery_path)]
            with patch.object(sys, "argv", argv), patch.object(fixture_runner, "Pilot") as cls:
                cls.return_value.root = root / "out"
                cls.return_value.run.return_value = {"status": "test", "statistics": {}}
                fixture_runner.main()
                self.assertEqual(cls.call_args.args[-1], discovery)

    def test_discovery_rejects_duplicate_keys_and_non_array(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "discovery.json"
            for raw in ('[{"family":"a","family":"b","seed":1}]', '{}', '[NaN]'):
                path.write_text(raw, encoding="utf-8")
                with self.assertRaises(ValueError):
                    fixture_runner.read_discovery(path)
            path.write_bytes(b" " * (1024**2 + 1))
            with self.assertRaisesRegex(ValueError, "exceeds byte bound"):
                fixture_runner.read_discovery(path)


def bank():
    return {"schema": 1, "decision_budget": 2, "slots": 3, "sampler_seed": 7, "current_difficulty": 1,
            "families": [{"id": name, "lesson": lesson, "food_need_condition": "newborn_v1", "placement": "native_seeded_v1", "difficulty": i}
                         for i, (name, lesson) in enumerate((("easy", "eat_held_food"), ("edge", "eat_reachable_food"), ("hard", "feeding")))],
            "pool": [{"family": name, "seed": 1000+100*i+j}
                     for i, name in enumerate(("easy", "edge", "hard")) for j in range(5)],
            "holdouts": [{"family": "edge", "seed": 9000}]}


def observations():
    rows = []
    for i, (lesson, outcomes) in enumerate((("eat_held_food", [1, 1, 1, 1]),
                                           ("eat_reachable_food", [1, 0, 1, 0]), ("feeding", [0, 0, 0, 0]))):
        for j, success in enumerate(outcomes):
            rows.append({"lesson": lesson, "food_need_condition": "newborn_v1", "seed": 100+10*i+j,
                         "decisions": 2, "temperature": 1.0, "actor_sha256": "a"*64, "value_sha256": "b"*64,
                         "source_sha256": {"actor-checkpoint.json": "c"*64}, "policy_version": 1,
                         "nourished_meals": success, "survived_evaluated_window": True,
                         "success": bool(success), "receipt_sha256": "d"*64})
    return rows


def fixture(root, mutate=None):
    rows = []
    for i in range(3):
        before = {"body": {"health": 1.0, "energy": .5, "injury": 0.0}, "homeostasis": {"drives": {"hunger": .2}}}
        after = copy.deepcopy(before)
        if i == 0:
            after["body"]["energy"] = .6
        rows.append({"row": i, "loss_row": False, "teacher_heard": False, "evaluation_row": i < 2,
                     "bootstrap_row": i >= 2, "organism": 1, "decision_tick": i, "outcome_tick": i+1,
                     "outcome": {"measured_physiology": {"before": before, "after": after}}, "nourished_meal": i == 0})
    trace = {"backend": "native-cpu", "lesson": "feeding", "rows": rows}
    request = {"previous": "synthetic-source", "output": str(root), "seed_history": "synthetic-history.json",
               "decisions": 2, "seed": 1234, "lesson": "feeding", "temperature": 1.0, "known_exclusions": False}
    result = {"mode": "native-cpu-care-assessment", "backend": "native-cpu", "lesson": "feeding", "seed": 1234,
              "evaluated_decisions": 2, "bootstrap_decisions": 1, "teacher_present": False, "teacher_cue_frames": 0,
              "semantic_prior_enabled": False, "offline_learning_disabled": True,
              "offline_actor_optimizer_updates": 0, "offline_value_optimizer_updates": 0,
              "actor_checkpoint_unchanged": True, "value_checkpoint_unchanged": True, "promotion": "none",
              "actor_checkpoint_before_sha256": "a"*64, "actor_checkpoint_after_sha256": "a"*64,
              "value_checkpoint_before_sha256": "b"*64, "value_checkpoint_after_sha256": "b"*64,
              "evaluated_nourished_meals": 1, "source_sha256": {}, "input_sha256": {}, "policy_version": 1}
    if mutate:
        mutate(trace, request, result)
    path = root / "care-trace.json"
    path.write_text(json.dumps(trace))
    receipt = {"schema_version": 1, "request": request, "result": result, "input_sha256": {},
               "artifacts": {"care-trace.json": hashlib.sha256(path.read_bytes()).hexdigest()}}
    receipt["payload_sha256"] = [0] * 32  # Synthetic; only mocked native admission may accept it.
    (root / "care-evaluation.json").write_text(json.dumps(receipt))


def nested_fixture(root):
    fixture(root)
    nested = root / "source-native" / "evidence.json"
    nested.parent.mkdir()
    nested.write_bytes(b'{"scope":"synthetic native nested artifact"}')
    path = root / "care-evaluation.json"
    receipt = json.loads(path.read_text())
    receipt["artifacts"]["source-native/evidence.json"] = hashlib.sha256(nested.read_bytes()).hexdigest()
    receipt["payload_sha256"] = [0] * 32  # Synthetic; only mocked native admission may accept it.
    path.write_text(json.dumps(receipt))
    return nested


def admitted_score(root):
    """Mock only native admission while exercising Python structural guards."""
    result = sampler.read_json(root / "care-evaluation.json")["result"]
    admitted = subprocess.CompletedProcess([], 0, json.dumps(result).encode(), b"")
    with patch.object(sampler.subprocess, "run", return_value=admitted):
        return sampler.score_assessment(root, Path(__file__))


class FrontierTests(unittest.TestCase):
    def test_selects_empirically_feasible_frontier_and_keeps_fixed_budget_and_holdouts(self):
        b, rows = bank(), observations()
        original = copy.deepcopy((b, rows))
        chosen = sampler.select(b, rows)
        self.assertEqual(chosen, sampler.select(b, rows))
        self.assertEqual(chosen["status"], "ready")
        self.assertEqual([t["family"] for t in chosen["baseline"]], ["easy", "edge", "hard"])
        self.assertTrue(all(t["family"] == "edge" for t in chosen["adaptive"]))
        self.assertEqual(len({t["seed"] for t in chosen["adaptive"]}), 3)
        self.assertEqual(chosen["requested_training_decisions_per_arm"], 6)
        self.assertFalse(chosen["solver_rewards_changed"])
        self.assertEqual(chosen["statistics"]["hard"]["status"], "no_observed_success")
        self.assertEqual(chosen["statistics"]["easy"]["status"], "observed_trivial")
        self.assertEqual((b, rows), original)

    def test_no_empirical_frontier_is_negative_and_never_fabricates_dispatch(self):
        rows = observations()
        for row in rows:
            row["success"] = False
        result = sampler.select(bank(), rows)
        self.assertEqual(result["status"], "no_sufficient_empirical_frontier")
        self.assertEqual(result["adaptive"], [])
        self.assertEqual(len(result["baseline"]), 3)

    def test_bounded_batch_difficulty_up_down_and_retention_mix(self):
        self.assertEqual(sampler.adjust_difficulty(1, 4, 4, 2)[0], 2)
        self.assertEqual(sampler.adjust_difficulty(1, 0, 4, 2)[0], 0)
        self.assertEqual(sampler.adjust_difficulty(1, 2, 4, 2)[0], 1)
        self.assertEqual(sampler.adjust_difficulty(0, 0, 4, 2)[0], 0)
        self.assertEqual(sampler.adjust_difficulty(2, 4, 4, 2)[0], 2)
        self.assertEqual(sampler.adjust_difficulty(1, 0, 2, 2)[0], 1)
        b = bank(); b["slots"] = 4
        result = sampler.select(b, observations())
        self.assertEqual(result["retention_slots_filled"], 1)
        self.assertEqual(result["adaptive"][0]["family"], "easy")
        self.assertEqual([t["family"] for t in result["adaptive"]][1:], ["edge"]*3)

    def test_rejects_custom_placements_reward_changes_seed_leakage_and_identity_or_dose_drift(self):
        mutations = [lambda b: b["families"][0].update(placement=[999, 999]),
                     lambda b: b.update(reward_override=1), lambda b: b["pool"][0].update(seed=True),
                     lambda b: b.update(schema=True), lambda b: b.update(current_difficulty=True),
                     lambda b: b["holdouts"][0].update(seed=1000),
                     lambda b: b["pool"][0].update(seed=100),
                     lambda b: b["families"][0].update(food_need_condition="ordinary_healthy_aging_v1")]
        for mutation in mutations:
            b = bank(); mutation(b)
            with self.assertRaises(ValueError):
                sampler.select(b, observations())
        for field, value in (("actor_sha256", "wrong"), ("value_sha256", "wrong"),
                             ("decisions", 3), ("temperature", .9), ("policy_version", 2), ("seed", 9000)):
            rows = observations(); rows[1][field] = value
            with self.assertRaises(ValueError):
                sampler.select(bank(), rows)

    def test_native_score_excludes_bootstrap_and_binds_the_archive(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); fixture(root)
            score = admitted_score(root)
            self.assertEqual(score["nourished_meals"], 1)
            self.assertTrue(score["success"])
            self.assertTrue(score["survived_evaluated_window"])
            (root / "care-trace.json").write_text("changed")
            with self.assertRaisesRegex(ValueError, "archive changed"):
                admitted_score(root)

    def test_native_refusal_blocks_receipt_only_edits_without_fallback(self):
        for change in (lambda r: r["request"].update(food_need_condition="ordinary_healthy_aging_v1"),
                       lambda r: r["result"].update(actor_checkpoint_before_sha256="forged"),
                       lambda r: r["result"].update(semantic_prior_enabled=True)):
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); fixture(root)
                path = root / "care-evaluation.json"
                receipt = json.loads(path.read_text()); change(receipt); path.write_text(json.dumps(receipt))
                before = sampler.raw_tree(root)
                failed = subprocess.CompletedProcess([], 1, b"", b"invalid native seal")
                with patch.object(sampler.subprocess, "run", return_value=failed) as native:
                    with self.assertRaisesRegex(ValueError, "native completed assessment validation failed"):
                        sampler.score_assessment(root, Path(__file__))
                native.assert_called_once()
                self.assertEqual(sampler.raw_tree(root), before)

    def test_public_scoring_requires_native_evaluator_and_exact_native_result(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); fixture(root)
            with self.assertRaises(TypeError):
                sampler.score_assessment(root)
            def native(argv, **kwargs):
                self.assertEqual(argv[:3], [str(Path(__file__).resolve()), "evaluate", "--spec"])
                self.assertEqual(json.loads(Path(argv[3]).read_text()), sampler.read_json(root / "care-evaluation.json")["request"])
                self.assertEqual(kwargs["env"]["ALIFE_SLM_PRIOR"], "off")
                self.assertEqual(kwargs["env"]["ALIFE_SLM_PRIOR_BACKEND"], "off")
                return subprocess.CompletedProcess(argv, 0, b'{"different":"result"}', b"")
            with patch.object(sampler.subprocess, "run", side_effect=native):
                with self.assertRaisesRegex(ValueError, "native completed assessment validation failed"):
                    sampler.score_assessment(root, Path(__file__))
            self.assertTrue(admitted_score(root)["success"])

    def test_native_read_only_validation_cannot_mutate_the_archive(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); fixture(root)
            result = sampler.read_json(root / "care-evaluation.json")["result"]
            def native(argv, **kwargs):
                (root / "unexpected-write").write_bytes(b"changed")
                return subprocess.CompletedProcess(argv, 0, json.dumps(result).encode(), b"")
            with patch.object(sampler.subprocess, "run", side_effect=native):
                with self.assertRaisesRegex(ValueError, "changed archive/executable"):
                    sampler.score_assessment(root, Path(__file__))

    def test_nested_native_archive_uses_forward_slashes_and_still_rejects_changed_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); nested = nested_fixture(root)
            original_receipt = (root / "care-evaluation.json").read_bytes()
            self.assertTrue(admitted_score(root)["success"])
            self.assertEqual((root / "care-evaluation.json").read_bytes(), original_receipt)
            nested.write_bytes(b"changed nested artifact")
            with self.assertRaisesRegex(ValueError, "archive changed/unsealed"):
                admitted_score(root)

    def test_windows_relative_inventory_matches_the_unchanged_native_receipt(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); nested_fixture(root)
            raw = sampler.raw_tree(root)
            windows = {str(PureWindowsPath(name)): pin for name, pin in raw.items()}
            self.assertIn("source-native\\evidence.json", windows)
            original_receipt = (root / "care-evaluation.json").read_bytes()
            with patch.object(sampler, "raw_tree", return_value=windows):
                self.assertTrue(admitted_score(root)["success"])
            self.assertEqual((root / "care-evaluation.json").read_bytes(), original_receipt)

    def test_separator_normalization_rejects_ambiguous_names(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); nested_fixture(root)
            raw = sampler.raw_tree(root)
            raw["source-native/evidence.json"] = {"sha256": "a"*64}
            raw["source-native\\evidence.json"] = {"sha256": "b"*64}
            with patch.object(sampler, "raw_tree", return_value=raw):
                with self.assertRaisesRegex(ValueError, "ambiguous archive path"):
                    admitted_score(root)

    def test_native_score_refuses_cues_prior_updates_censoring_and_unmeasured_success(self):
        mutations = [lambda t, q, r: r.update(teacher_present=True),
                     lambda t, q, r: r.update(semantic_prior_enabled=True),
                     lambda t, q, r: r.update(offline_actor_optimizer_updates=1),
                     lambda t, q, r: r.update(actor_checkpoint_after_sha256="changed"),
                     lambda t, q, r: t["rows"].pop(),
                     lambda t, q, r: t["rows"][0].update(teacher_heard=True),
                     lambda t, q, r: t.update(recovery_prefix=True),
                     lambda t, q, r: t.update(complete=False),
                     lambda t, q, r: t["rows"][2].update(evaluation_row=True),
                     lambda t, q, r: t["rows"][0]["outcome"]["measured_physiology"]["after"]["body"].update(energy=.5)]
        for mutation in mutations:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); fixture(root, mutation)
                with self.assertRaises((ValueError, KeyError)):
                    admitted_score(root)

    def test_fixture_runner_cannot_resume_or_accept_a_retained_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(ValueError, "fresh disposable"):
                Pilot(Path(tmp), Path(__file__), Path(__file__), 2, 3, 1)

    def test_standalone_bounds_before_io_and_never_verifies_another_output(self):
        script = Path(__file__).with_name("care_frontier_sampler.py")
        args = [sys.executable, "-B", str(script), "--bank", "missing-bank", "--native-evaluator", "missing-binary"]
        args.extend([item for _ in range(1025) for item in ("--assessment", "never-open")])
        result = subprocess.run(args, capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"bounded discovery evidence required before opening", result.stderr)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); fixture(root)
            path = root / "care-evaluation.json"
            receipt = json.loads(path.read_text())
            receipt["request"]["output"] = str(root.parent)
            receipt["payload_sha256"] = [0] * 32  # Synthetic; only mocked native admission may accept it.
            path.write_text(json.dumps(receipt))
            with self.assertRaisesRegex(ValueError, "differs from selected archive"):
                sampler.score_assessment(root, Path("missing-binary"))


@unittest.skipUnless(os.environ.get("ALIFE_FRONTIER_TEST_CARE") and os.environ.get("ALIFE_FRONTIER_TEST_ROUND"),
                     "explicit native CPU fixture binaries required")
class NativeFrontierTests(unittest.TestCase):
    def test_genuine_sibling_path_order_and_negative_tamper_controls(self):
        """Fresh public founder only; native owns its PathBuf/serde checksum."""
        care = Path(os.environ["ALIFE_FRONTIER_TEST_CARE"]).resolve(strict=True)
        evaluator = Path(os.environ["ALIFE_FRONTIER_TEST_ROUND"]).resolve(strict=True)
        environment = {key: value for key, value in os.environ.items() if not key.startswith("ALIFE_SLM_")}
        environment.update(ALIFE_SLM_PRIOR="off", ALIFE_SLM_PRIOR_BACKEND="off")
        def run(binary, args):
            completed = subprocess.run([str(binary), *map(str, args)], env=environment,
                                       stdin=subprocess.DEVNULL, capture_output=True, timeout=60, check=True)
            return json.loads(completed.stdout)
        with tempfile.TemporaryDirectory(prefix="alife-frontier-sibling-regression-") as tmp:
            root = Path(tmp).resolve()
            source, history, output = root / "source", root / "source-history.json", root / "assessment"
            run(care, ["--fixture", "--output", source, "--seed", 881001,
                       "--lesson", "eat_held_food", "--decisions", 2])
            names = ("cpu-cycle.json", "trained.alife-foundation", "actor-checkpoint.json", "value-checkpoint.json")
            history.write_text(json.dumps({"schema": 1, "kind": "mixed-care-seed-history", "complete": True,
                "evidence": "New disposable public fixture only; no retained lineage or capability claim",
                "source_sha256": {name: sampler.file_pin(source / name)["sha256"] for name in names},
                "consumed_seed_ranges": [[881001, 881001]], "reserved_seed_ranges": [],
                "assessment_seeds": [889901], "imports": []}), encoding="utf-8")
            request = {"previous": str(source), "output": str(output), "seed_history": str(history),
                       "seed": 889901, "decisions": 2, "temperature": 1.0, "lesson": "eat_held_food",
                       "food_need_condition": "newborn_v1", "known_exclusions": False}
            spec = root / "request.json"
            spec.write_text(json.dumps(request), encoding="utf-8")
            native_result = run(evaluator, ["evaluate", "--spec", spec])
            receipt_path = output / "care-evaluation.json"
            receipt_bytes = receipt_path.read_bytes()
            receipt = json.loads(receipt_bytes)
            keys = receipt["input_sha256"]
            self.assertNotEqual(sorted(keys), sorted(keys, key=lambda name: Path(name).parts))
            before = sampler.raw_tree(root)
            score = sampler.score_assessment(output, evaluator)
            self.assertEqual(score["seed"], 889901)
            self.assertEqual(score["nourished_meals"], native_result["evaluated_nourished_meals"])
            self.assertEqual(sampler.raw_tree(root), before)
            # Receipt-only tamper leaves artifact hashes unchanged; native must
            # reject the typed seal, with no Python-only scoring fallback.
            receipt["result"]["policy_version"] += 1
            try:
                receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
                tampered = sampler.raw_tree(output)
                with self.assertRaisesRegex(ValueError, "native completed assessment validation failed"):
                    sampler.score_assessment(output, evaluator)
                self.assertEqual(sampler.raw_tree(output), tampered)
            finally:
                receipt_path.write_bytes(receipt_bytes)
            artifact = output / "care-trace.json"
            artifact_bytes = artifact.read_bytes()
            try:
                artifact.write_bytes(artifact_bytes + b" ")
                with self.assertRaisesRegex(ValueError, "native completed assessment validation failed"):
                    sampler.score_assessment(output, evaluator)
            finally:
                artifact.write_bytes(artifact_bytes)
            self.assertEqual(sampler.raw_tree(root), before)
            self.assertEqual(sampler.score_assessment(output, evaluator), score)


if __name__ == "__main__":
    unittest.main()

import copy
import unittest
import contextlib
import datetime as dt
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from unittest.mock import patch

import qualify_continued_critic as qualification
from qualify_continued_critic import check_slices, validate_admission


class SeparateSliceGates(unittest.TestCase):
    def fixture(self):
        return {key:dict(rows=rows,residual_mse=.02,out_of_support_predictions=0) for key,rows in
                [("early16",16),("whole_trajectory",128),("last16",16)]}

    def test_short_real_trajectories_cannot_stand_in_for_whole128(self):
        for side in [0,1]:
            for rows in [16,64,127]:
                contexts=[self.fixture(),self.fixture()]
                contexts[side]["whole_trajectory"]["rows"]=rows
                with self.assertRaisesRegex(ValueError,"whole_trajectory.*real transitions"):
                    check_slices(*contexts)

    def test_each_edge_slice_requires_its_own16_real_rows(self):
        for label in ["early16","last16"]:
            baseline=self.fixture(); observed=copy.deepcopy(baseline)
            observed[label]["rows"]=15
            with self.assertRaisesRegex(ValueError,label):check_slices(baseline,observed)

    def test_initial_transient_fails_even_when_whole_average_passes(self):
        baseline=self.fixture(); observed=copy.deepcopy(baseline)
        observed["early16"]["residual_mse"]=1.687
        observed["whole_trajectory"]["residual_mse"]=.59
        with self.assertRaisesRegex(ValueError,"early16"):
            check_slices(baseline,observed)

    def test_nonfinite_or_support_escape_cannot_pass(self):
        for field,value in [("residual_mse",float("nan")),("out_of_support_predictions",1)]:
            baseline=self.fixture(); observed=copy.deepcopy(baseline)
            observed["last16"][field]=value
            with self.assertRaises(ValueError):check_slices(baseline,observed)

    def test_all_slices_inside_predeclared_limits_pass(self):
        check_slices(self.fixture(),self.fixture())

    def test_admission_must_be_read_only_and_bound_to_all_checkpoint_counters(self):
        cycle=dict(policy_version=7,actor_optimizer_step=12,value_optimizer_step=5012)
        valid=dict(cycle,neural_execution=False,weights_modified=False)
        validate_admission(valid,cycle)
        for key in ["policy_version","actor_optimizer_step","value_optimizer_step"]:
            changed=dict(valid);changed[key]+=1
            with self.assertRaises(ValueError):validate_admission(changed,cycle)
        for key in ["neural_execution","weights_modified"]:
            for bad in [True,None]:
                changed=dict(valid);changed[key]=bad
                with self.assertRaises(ValueError):validate_admission(changed,cycle)


# Host-only orchestration checks; native commands are deterministic doubles.
class QualificationComparability(unittest.TestCase):
    def run_qualification(self, current_actor_rows=128):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        source = root / "preserved-source"
        source.mkdir()
        output = root / "disposable-output"
        def write(path, value):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(value), encoding="utf-8")
        write(source / "cycle.json", dict(policy_version=5502))
        write(source / "actor-checkpoint.json", dict(optimizer_step=12888))
        write(source / "value-checkpoint.json", dict(optimizer_step=16004))
        (source / "trained.alife-foundation").write_bytes(b"preserved-asset")
        protected = qualification.file_map(source)
        binaries = {}
        for role in ["trainer", "evaluator", "inspector"]:
            binary = root / role
            binary.write_bytes(role.encode())
            binaries[role] = binary
        commands = []
        def native(argv, timeout):
            commands.append(argv)
            role = Path(argv[0]).name
            mode = argv[1]
            stdout = b"{}"
            if role == "trainer" and mode == "--evaluate":
                target, asset, lesson, cap, seed, founder = argv[2:]
                # The real baseline consumed at decision 8, within the original cap16.
                write(Path(target) / "pilot.json", dict(seed=int(seed), ticks=min(8, int(cap)),
                      consumed_events=int(int(cap) >= 8), founder_seed_base=int(founder),
                      lesson=lesson, teacher_mode=False))
            elif role == "trainer" and mode == "--resume-cycle":
                target = Path(argv[3])
                sidecar = target / "critic-residual-update.json"
                write(sidecar, dict(optimizer_updates=2))
                write(target / "cycle.json", dict(policy_version=5503, actor_optimizer_step=12890,
                      value_optimizer_step=16006, trained_replay_rows=16, captured_replay_rows=17,
                      bootstrap_replay_rows=1, critic_objective_version=1,
                      critic_residual_update_sha256=qualification.sha(sidecar)))
                write(target / "actor-checkpoint.json", dict(optimizer_step=12890))
                write(target / "value-checkpoint.json", dict(optimizer_step=16006))
                (target / "trained.alife-foundation").write_bytes(b"candidate-asset")
            elif role == "evaluator" and mode == "--capture-summary":
                checkpoint, capture, seed, requested, temperature = argv[2:]
                capture = Path(capture)
                rows = current_actor_rows if capture.name.startswith("current-actor") else 128
                write(capture / "critic-capture.json", dict(seed=int(seed), requested_decisions=int(requested),
                      transition_count=rows, actor_policy=5503 if Path(checkpoint).name == "cycle-001" else 5502,
                      terminal_death_tick=900 if rows < int(requested) else None))
                (capture / "critic-features.f32le").write_bytes(b"real-capture-double:" + str(rows).encode())
            elif role == "evaluator" and mode == "--evaluate-capture":
                capture, baseline, candidate, report = argv[2:]
                capture = Path(capture)
                receipt = qualification.read(capture / "critic-capture.json")
                rows = receipt["transition_count"]
                metrics = {label:dict(rows=n, residual_mse=.02, out_of_support_predictions=0)
                           for label,n in [("early16",16),("whole_trajectory",rows),("last16",16)]}
                write(Path(report), dict(seed=receipt["seed"], rows=rows, actor_policy=receipt["actor_policy"],
                      feature_sha256=qualification.sha(capture / "critic-features.f32le"),
                      capture_sha256=qualification.sha(capture / "critic-capture.json"),
                      before=metrics, after=metrics))
            elif role == "inspector" and mode == "--validate-checkpoint":
                cycle = qualification.read(Path(argv[2]) / "cycle.json")
                stdout = json.dumps(dict(cycle, neural_execution=False, weights_modified=False)).encode()
            elif (role, mode) != ("evaluator", "--gradient-regression"):
                raise AssertionError((role, mode))
            return subprocess.CompletedProcess(argv, 0, stdout, b"")
        deadline = (dt.datetime.now(dt.timezone.utc) + dt.timedelta(hours=1)).isoformat()
        argv = ["qualify_continued_critic.py", "--source", str(source), "--output", str(output),
                "--cycles", "1", "--first-seed", "100", "--fresh-context-seeds", "200", "201",
                "--acceptance-seeds", "300", "301", "--deadline-utc", deadline]
        for role, binary in binaries.items():
            argv += ["--" + role, str(binary)]
        error = None
        with patch.dict(qualification.os.environ), patch.object(sys, "argv", argv), \
                patch.object(qualification, "workers", return_value=[]), \
                patch.object(qualification, "resources", return_value={}), \
                patch.object(qualification, "run_inspector", side_effect=native), \
                contextlib.redirect_stdout(io.StringIO()):
            try:
                qualification.main()
            except ValueError as caught:
                error = caught
        self.assertEqual(qualification.file_map(source), protected)
        return qualification.read(output / "RESULT.json"), commands, error

    def test_matched_caps_fixed_contexts_and_disposable_counters(self):
        result, commands, error = self.run_qualification()
        self.assertIsNone(error)
        probes = [command for command in commands if command[1] == "--evaluate"]
        self.assertEqual(len(probes), 4)
        for baseline, candidate in zip(probes[:2], probes[2:]):
            self.assertEqual(baseline[4:], candidate[4:])
            self.assertEqual(baseline[5], "16")
        evaluations = [command for command in commands if command[1] == "--evaluate-capture"]
        self.assertEqual(evaluations[0][2:4], evaluations[2][2:4])
        self.assertEqual(evaluations[1][2:4], evaluations[3][2:4])
        self.assertNotEqual(evaluations[0][4], evaluations[2][4])
        self.assertNotEqual(evaluations[1][4], evaluations[3][4])
        self.assertEqual(result["accepted_cycles"], 1)
        self.assertEqual((result["policy"], result["actor_age"], result["critic_age"]), (5503,12890,16006))
        self.assertFalse(result["production_adopted"])
        self.assertEqual(result["status"], "short_clone_evidence_complete_extended_qualification_pending")
        self.assertTrue(result["protected_checkpoints_unchanged"])

    def test_real_death_short_context_cannot_be_complete_whole128_evidence(self):
        result, commands, error = self.run_qualification(current_actor_rows=16)
        self.assertIsNotNone(error, "16 terminal rows were accepted as whole128 qualification")
        self.assertEqual(result["status"], "qualification_held")
        self.assertFalse(result["production_adopted"])
        self.assertEqual(result["accepted_cycles"], 1)


if __name__ == "__main__":
    unittest.main()

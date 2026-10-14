#!/usr/bin/env python3
"""Exercise diagnostic plans without launching a trainer or requiring a GPU."""

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("training_dose_diagnostic.py")


class DiagnosticPlanTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.binary = self.root / "nonexecutable-trainer"
        self.binary.write_bytes(b"This file cannot execute a trainer.\n")
        self.receipt = self.root / "source-receipt.json"
        self.receipt.write_text('{"source": "test-only"}\n', encoding="utf-8")
        pilot = self.root / "pilot"
        pilot.mkdir()
        (pilot / "receipt.json").write_text('{}\n', encoding="utf-8")
        self.manifest = self.root / "manifest.json"
        self.manifest.write_text(json.dumps({"founder_seed_base": 42, "pilots": ["pilot"]}),
                                 encoding="utf-8")
        self.output = self.root / "new-plan"

    def plan(self, ticks=2048, token=16, world_seed=1, founder_seed=42):
        # Deliberately omit --run; a nonexecutable binary makes accidental launch fail.
        return subprocess.run([
            sys.executable, str(SCRIPT), "--binary", str(self.binary),
            "--source-receipt", str(self.receipt), "--manifest", str(self.manifest),
            "--output", str(self.output), "--world-seed", str(world_seed),
            "--founder-seed", str(founder_seed), "--eval-ticks", str(ticks),
            "--request-token", str(token),
        ], capture_output=True, text=True, check=False)

    def test_plan_matches_evaluator_cli_and_preserves_frozen_inputs(self):
        result = self.plan()
        self.assertEqual(result.returncode, 0, result.stderr)
        plan = json.loads((self.output / "dose-plan.json").read_text(encoding="utf-8"))
        self.assertEqual(plan["status"], "planned")
        self.assertEqual(plan["completed_commands"], [])
        self.assertFalse(plan["comparison_complete"])
        self.assertFalse(plan["promoted"])
        self.assertEqual(plan["cap_seconds"], 2400)
        self.assertEqual(len(plan["commands"]), 18)
        for command in plan["commands"]:
            argv = command["argv"]
            if argv[1] == "--warmup":
                self.assertFalse(command["prior_off"])
                self.assertEqual(len(argv), 5)
                self.assertIn(argv[-1], ("2", "8"))
                continue
            self.assertTrue(command["prior_off"])
            self.assertEqual(argv[5:8], ["2048", "1", "42"])
            if argv[4] == "vocabulary_reception":
                self.assertEqual(argv[8:], ["--request-token", "16"])
            else:
                self.assertEqual(argv[8:], [], command["name"])
        for path, digest in plan["inputs_sha256"].items():
            self.assertEqual(hashlib.sha256(Path(path).read_bytes()).hexdigest(), digest)

    def test_supported_request_pairs_and_tick_endpoints(self):
        for ticks in (1, 2048):
            for token in (1, 2, 9, 13, 15, 16):
                with self.subTest(ticks=ticks, token=token):
                    self.output = self.root / f"plan-{ticks}-{token}"
                    result = self.plan(ticks=ticks, token=token, world_seed=2**64 - 1)
                    self.assertEqual(result.returncode, 0, result.stderr)

    def test_unsupported_inputs_fail_before_creating_a_plan(self):
        invalid = ([{"ticks": n} for n in (-1, 0, 2049)]
                   + [{"token": n} for n in (0, 3, 8, 10, 14, 17, 255, 65536)]
                   + [{"world_seed": n} for n in (0, -1, 2**64)]
                   + [{"founder_seed": n} for n in (0, -1, 2**64)])
        for index, arguments in enumerate(invalid):
            with self.subTest(arguments=arguments):
                self.output = self.root / f"invalid-plan-{index}"
                result = self.plan(**arguments)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()

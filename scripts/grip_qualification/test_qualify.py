"""Source-only workflow/sanitizer tests. No Cargo, native fixtures or network."""

import contextlib
import copy
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import MagicMock, patch

import qualify as q


ROOT = Path(__file__).resolve().parents[2]
PRIVATE = "PRIVATE_CHECKPOINT_neural_world_journal_927"


class FormatSourceTests(unittest.TestCase):
    def test_archived_rust_does_not_block_or_enter_compiled_source_formatting(self):
        source = "crates/alife_training/examples/probe_grip_reader.rs"
        archive = "docs/experiments/brain-surgery-2026-11-08/reader-v1/source/" + source
        with patch.object(q, "git", return_value=source + "\n" + archive) as git:
            self.assertEqual(q.changed_format_sources(Path("."), {source: 100}), [source])
        git.assert_called_once_with(Path("."), "diff", "--name-only", "--diff-filter=ACMR",
                                    q.BASE, "HEAD", "--", "*.rs")

    def test_archive_only_change_does_not_schedule_native_source_formatting(self):
        with patch.object(q, "git", return_value="docs/experiments/frozen/source.rs"):
            self.assertEqual(q.changed_format_sources(Path("."), {}), [])

    def test_unknown_compiled_or_nonarchive_rust_is_still_rejected(self):
        for name in ("crates/alife_training/src/unbound.rs", "build.rs", "docs/source.rs"):
            with self.subTest(name=name), patch.object(q, "git", return_value=name):
                with self.assertRaisesRegex(q.GateError, "source_binding_failed"):
                    q.changed_format_sources(Path("."), {})


class ProjectionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="grip-source-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.path = self.root / "synthetic.log"
        self.source = "crates/alife_core/src/memory_query.rs"
        self.sources = {self.source: 1200}

    def log(self, lines):
        self.path.write_text("\n".join(json.dumps(v) if isinstance(v, dict) else v for v in lines))
        return self.path

    def diagnostic(self):
        return {"reason": "compiler-message", "target": {
            "kind": ["lib"], "src_path": str(self.root / self.source)}, "message": {
            "code": {"code": "E0308", "explanation": PRIVATE}, "level": "error",
            "message": PRIVATE, "rendered": PRIVATE, "children": [{"message": PRIVATE}],
            "spans": [{"file_name": self.source, "line_start": 11, "is_primary": True,
                       "label": PRIVATE, "text": [{"text": PRIVATE}],
                       "suggested_replacement": PRIVATE}]}}

    def project(self, lines):
        return list(q.compiler_diagnostics(self.log(lines), self.root, self.sources))

    def test_compiler_projection_uses_only_fixed_message_and_verified_source_span(self):
        result = self.project([self.diagnostic()])
        self.assertEqual(result, [{"code": "E0308", "level": "error", "file": self.source,
                                   "line": 11, "message": "mismatched types"}])
        self.assertNotIn(PRIVATE, json.dumps(result))

    def test_unknown_json_build_scripts_and_raw_output_are_never_reported(self):
        record = self.diagnostic()
        output = [PRIVATE, "::error::" + PRIVATE, "thread panicked: " + PRIVATE,
                  {"reason": "build-script-executed", "message": PRIVATE},
                  {"reason": "compiler-artifact", "executable": PRIVATE},
                  {"checkpoint": PRIVATE}, "null", "[]", '"' + PRIVATE + '"']
        for kind in (["custom-build"], ["bin"], ["proc-macro"], PRIVATE, None):
            value = copy.deepcopy(record)
            value["target"]["kind"] = kind
            output.append(value)
        self.assertEqual(self.project(output), [])

    def test_unknown_codes_and_compile_error_payloads_get_no_free_text(self):
        record = self.diagnostic()
        record["message"]["code"] = {"code": "clippy::" + PRIVATE}
        result = self.project([record])
        self.assertEqual(result[0]["code"], "unclassified")
        self.assertNotIn(PRIVATE, json.dumps(result))
        record["message"]["code"] = None
        self.assertEqual(self.project([record])[0]["code"], "unclassified")

    def test_unknown_source_and_dependency_paths_are_rejected(self):
        for name in ("/tmp/" + PRIVATE, "../" + self.source, "crates/../" + self.source,
                     self.source.replace("/", "\\"), "crates/alife_core/generated.rs"):
            record = self.diagnostic()
            record["message"]["spans"][0]["file_name"] = name
            self.assertEqual(self.project([record]), [])
        record = self.diagnostic()
        record["target"]["src_path"] = "/tmp/registry/private.rs"
        self.assertEqual(self.project([record]), [])

    def test_primary_line_must_be_an_in_range_integer(self):
        for line in (True, False, 0, -1, 1201, "11", 1.25, None):
            record = self.diagnostic()
            record["message"]["spans"][0]["line_start"] = line
            self.assertEqual(self.project([record]), [])
        record = self.diagnostic()
        record["message"]["spans"][0]["is_primary"] = False
        self.assertEqual(self.project([record]), [])

    def test_malformed_nested_values_and_levels_cannot_escape(self):
        for key, values in (("target", (None, [], PRIVATE)), ("message", (None, [], PRIVATE))):
            for value in values:
                record = self.diagnostic()
                record[key] = value
                self.assertEqual(self.project([record]), [])
        for value in (None, {}, PRIVATE, "note", "\x1b[31merror"):
            record = self.diagnostic()
            record["message"]["level"] = value
            self.assertEqual(self.project([record]), [])

    def test_diagnostics_are_deduplicated_and_bounded(self):
        records = []
        for line in range(1, 60):
            record = self.diagnostic()
            record["message"]["spans"][0]["line_start"] = line
            records += [record, copy.deepcopy(record)]
        result = self.project(records)
        self.assertEqual(len(result), q.MAX_DIAGNOSTICS)
        self.assertEqual(len({v["line"] for v in result}), q.MAX_DIAGNOSTICS)

    def test_oversized_binary_and_recursive_lines_are_ignored_without_tail_leaks(self):
        self.path.write_bytes(b"X" * (q.MAX_LINE_BYTES + 20) + b"\n\xff\n" +
                              b"[" * 5000 + b"\n" + json.dumps(self.diagnostic()).encode())
        result = list(q.compiler_diagnostics(self.path, self.root, self.sources))
        self.assertEqual(len(result), 1)
        self.assertNotIn(PRIVATE, json.dumps(result))

    def test_only_bounded_known_failed_test_names_and_summary_are_projected(self):
        names = {"tests::one", "tests::two"}
        result = q.test_result(self.log([
            "test tests::one ... ok", "test tests::two ... FAILED",
            "test " + PRIVATE + " ... FAILED", "assertion left == right: " + PRIVATE,
            "thread 'tests::two' panicked: " + PRIVATE,
            "test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.02s",
        ]), names)
        self.assertEqual(result, {"failed_tests": ["tests::two"], "ignored_tests": [],
                                  "status": "FAILED", "passed": 1, "failed": 1,
                                  "ignored": 0, "filtered": 7, "ignored_inventory_valid": True})
        self.assertNotIn(PRIVATE, json.dumps(result))

    def test_inventory_accepts_only_test_identifier_lines(self):
        path = self.log(["tests::one: test", "assertion failed: " + PRIVATE,
                         "tests::one: benchmark", "bad/name: test", "::error::test: test",
                         "\x1b[31mtests::two: test", "X" * 241 + ": test"])
        self.assertEqual(q.listed_tests(path), {"tests::one"})

    def test_empty_inventory_is_a_failure_not_a_zero_test_success(self):
        with self.assertRaises(q.GateError):
            q.listed_tests(self.log(["0 tests, 0 benchmarks"]))

    def test_missing_duplicate_mismatched_and_zero_summaries_cannot_pass(self):
        good = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s"
        for lines in ([], [good, good], [good.replace("1 passed", "2 passed")],
                      [good.replace("1 passed", "0 passed")], [good + PRIVATE]):
            self.assertNotIn("status", q.test_result(self.log(lines), {"tests::one"}))

    def test_failed_identifier_count_is_bounded(self):
        names = {f"tests::failure_{i}" for i in range(50)}
        result = q.test_result(self.log([f"test {name} ... FAILED" for name in names]), names)
        self.assertEqual(len(result["failed_tests"]), q.MAX_FAILED_NAMES)

    def test_ignored_identifier_count_is_bounded_and_unknown_names_are_private(self):
        names = {f"tests::skip_{i}" for i in range(50)}
        result = q.test_result(self.log([f"test {name} ... ignored, {PRIVATE}" for name in names]
            + [f"test {PRIVATE} ... ignored, {PRIVATE}",
               "test result: ok. 0 passed; 0 failed; 50 ignored; 0 measured; 0 filtered out; finished in 0.01s"]),
            names, frozenset(names))
        self.assertEqual(len(result["ignored_tests"]), q.MAX_IGNORED_NAMES)
        self.assertFalse(result["ignored_inventory_valid"])
        self.assertNotIn(PRIVATE, json.dumps(result))

    def test_terse_failure_format_still_reports_only_known_names(self):
        result = q.test_result(self.log(["tests::one --- FAILED", PRIVATE + " --- FAILED"]), {"tests::one"})
        self.assertEqual(result["failed_tests"], ["tests::one"])


class SourceAndCommandTests(unittest.TestCase):
    def event(self):
        head = "a" * 40
        event = {"action": "synchronize", "repository": {"full_name": q.REPOSITORY},
                 "pull_request": {"head": {"sha": head, "ref": q.BRANCH,
                                           "repo": {"full_name": q.REPOSITORY}}}}
        env = {"GITHUB_ACTIONS": "true", "GITHUB_EVENT_NAME": "pull_request",
               "GITHUB_REPOSITORY": q.REPOSITORY, "EXPECTED_SOURCE_REVISION": head}
        return event, env, head

    def test_exact_pr_head_identity_is_required(self):
        event, env, head = self.event()
        q.validate_event(event, env, head)
        for bad in ("b" * 40, "refs/pull/1/merge", "a" * 39, PRIVATE):
            with self.assertRaises(q.GateError):
                q.validate_event(event, env, bad)

    def test_wrong_repo_branch_event_or_environment_is_rejected(self):
        event, env, head = self.event()
        changes = [("GITHUB_ACTIONS", "false"), ("GITHUB_EVENT_NAME", "push"),
                   ("GITHUB_REPOSITORY", "fork/A-Life"), ("EXPECTED_SOURCE_REVISION", "b" * 40)]
        for key, value in changes:
            with self.assertRaises(q.GateError):
                q.validate_event(event, dict(env, **{key: value}), head)
        for key, value in (("ref", "main"), ("repo", {"full_name": "fork/A-Life"})):
            bad = copy.deepcopy(event)
            bad["pull_request"]["head"][key] = value
            with self.assertRaises(q.GateError):
                q.validate_event(bad, env, head)
        event["action"] = "workflow_dispatch"
        with self.assertRaises(q.GateError):
            q.validate_event(event, env, head)

    def test_malformed_event_fails_closed(self):
        _, env, head = self.event()
        for event in ({}, [], None, {"pull_request": []}):
            with self.assertRaises(q.GateError):
                q.validate_event(event, env, head)

    def test_all_cargo_commands_are_fixed_locked_serial_libtest_or_compile(self):
        for suite in q.SUITES:
            for phase in ("build", "list", "test", "lint"):
                command = q.cargo_command(suite, phase, "/runner/temp/target")
                self.assertEqual(command[:2], ["cargo", "clippy" if phase == "lint" else "test"])
                self.assertIn("--locked", command)
                self.assertEqual(command[command.index("--profile") + 1], "dev")
                self.assertEqual(command[command.index("--target-dir") + 1], "/runner/temp/target")
                for forbidden in ("run", "--nocapture", "--show-output", "--ignored", "--all-targets", "--workspace"):
                    self.assertNotIn(forbidden, command)
                if phase == "build":
                    self.assertIn("--no-run", command)
                    self.assertIn("--message-format=json", command)
                elif phase == "lint":
                    self.assertEqual(command[-3:], ["--", "-D", "warnings"])
                else:
                    self.assertIn("--test-threads=1", command)
                    if suite.kind == "example":
                        self.assertEqual(suite.target, "probe_grip_reader")
                        self.assertIn("tests::", command)

    def test_unknown_suite_or_phase_cannot_run(self):
        for suite, phase in ((q.SUITES[0], "run"), (q.Suite("alife_training", "probe_native_learning", "example"), "test"),
                             (q.Suite("alife_training", "probe_grip_reader", "example"), "test")):
            with self.assertRaises(q.GateError):
                q.cargo_command(suite, phase, "/tmp/target")

    def test_only_the_existing_recall_microbenchmark_is_declared_ignored(self):
        self.assertEqual(len(q.SUITES), 18)
        ignored = {suite.label: suite.expected_ignored for suite in q.SUITES if suite.expected_ignored}
        self.assertEqual(ignored, {"alife_core/candidate_memory_retrieval":
                                  frozenset({"memory_cpu_recall_microbenchmark"})})

    def test_required_contract_targets_and_source_files_exist(self):
        selected = {(s.package, s.target) for s in q.SUITES}
        for target in ("candidate_memory_queries", "candidate_memory_retrieval", "memory_query",
                       "n2048_grip_reader_candidate", "n2048_foundation_abi", "n512_foundation_abi",
                       "n512_legacy_compatibility_abi"):
            self.assertIn(("alife_core", target), selected)
        for suite in q.SUITES:
            path = ROOT / "crates" / suite.package
            path /= "src/lib.rs" if suite.kind == "lib" else (
                ("examples/" if suite.kind == "example" else "tests/") + suite.target + ".rs")
            self.assertTrue(path.is_file(), str(path))

    def test_workflow_is_one_pr_only_same_repository_exact_head_job(self):
        workflow = (ROOT / ".github/workflows/grip-aware-candidate.yml").read_text()
        self.assertIn("types: [opened, synchronize, reopened]", workflow)
        self.assertIn("github.repository == 'BobertNeek/A-Life'", workflow)
        self.assertIn("github.event.pull_request.head.repo.full_name == 'BobertNeek/A-Life'", workflow)
        self.assertIn("github.event.pull_request.head.ref == '" + q.BRANCH + "'", workflow)
        self.assertIn("ref: ${{ github.event.pull_request.head.sha }}", workflow)
        self.assertIn("persist-credentials: false", workflow)
        self.assertIn("timeout-minutes: 30", workflow)
        self.assertIn("timeout-minutes: 26", workflow)
        self.assertIn("toolchain: '1.99.1'", workflow)
        self.assertEqual(workflow.count("    runs-on:"), 1)
        self.assertEqual(workflow.count("        run:"), 1)
        for forbidden in ("workflow_dispatch:", "\n  push:", "pull_request_target:", "matrix:",
                          "actions/cache", "upload-artifact", "download-artifact", "secrets.",
                          "probe_native_learning", "hosted_transfer.py execute", "cargo run"):
            self.assertNotIn(forbidden, workflow)

    def test_disk_time_and_log_budgets_are_bounded(self):
        self.assertEqual(q.PREFLIGHT_FREE, 12 * (1 << 30))
        self.assertEqual(q.FLOOR_FREE, 2 * (1 << 30))
        self.assertGreater(q.STOP_FREE, q.FLOOR_FREE)
        self.assertLess(q.TOTAL_SECONDS, 26 * 60)
        self.assertLessEqual(q.MAX_LOG_BYTES, 64 << 20)

    def test_unexpected_exceptions_never_print_their_contents(self):
        for error in (RuntimeError(PRIVATE), q.GateError(PRIVATE), q.GateError()):
            output = io.StringIO()
            with patch.object(q, "qualify", side_effect=error), contextlib.redirect_stdout(output):
                self.assertEqual(q.main(), 1)
            self.assertNotIn(PRIVATE, output.getvalue())
            self.assertIn("infrastructure_failure", output.getvalue())


class RunnerGuardTests(unittest.TestCase):
    """Fake subprocesses only: these tests cannot launch Cargo or a fixture."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="grip-runner-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.runner = q.Runner(self.root, self.root, {})
        self.space = patch.object(q.shutil, "disk_usage", return_value=SimpleNamespace(free=20 * q.GIB))
        self.space_mock = self.space.start()
        self.addCleanup(self.space.stop)

    def test_private_paths_are_cold_and_resource_environment_is_serial(self):
        self.assertTrue(self.runner.target.is_relative_to(self.root))
        self.assertTrue(self.runner.logs.is_relative_to(self.root))
        self.assertFalse(self.runner.target.exists())
        for key, value in (("CARGO_BUILD_JOBS", "1"), ("CARGO_INCREMENTAL", "0"),
                           ("CARGO_PROFILE_DEV_DEBUG", "0"), ("CARGO_PROFILE_TEST_DEBUG", "0"),
                           ("RUST_TEST_THREADS", "1"), ("RUST_MIN_STACK", "33554432"),
                           ("VK_ICD_FILENAMES", "/no-vulkan-icd.json")):
            self.assertEqual(self.runner.environment[key], value)
        for key in ("TMPDIR", "TMP", "TEMP", "CARGO_TARGET_DIR"):
            self.assertTrue(Path(self.runner.environment[key]).is_relative_to(self.root))
        with self.assertRaises(FileExistsError):
            q.Runner(self.root, self.root, {})

    def test_low_disk_and_expired_budget_prevent_process_start(self):
        with patch.object(q.subprocess, "Popen") as spawn:
            self.space_mock.return_value = SimpleNamespace(free=q.STOP_FREE - 1)
            with self.assertRaisesRegex(q.GateError, "disk_reserve_stop"):
                self.runner.run("synthetic", ["never-executed"])
            self.space_mock.return_value = SimpleNamespace(free=20 * q.GIB)
            self.runner.deadline = 0
            with self.assertRaisesRegex(q.GateError, "time_budget_stop"):
                self.runner.run("synthetic", ["never-executed"])
            spawn.assert_not_called()

    def fake_process(self, command, **kwargs):
        kwargs["stdout"].write(PRIVATE.encode())
        kwargs["stdout"].flush()
        process = MagicMock(pid=4242, returncode=1)
        process.poll.return_value = 1
        return process

    def test_process_failure_never_echoes_raw_log_and_kills_descendants(self):
        output = io.StringIO()
        with patch.object(q.subprocess, "Popen", side_effect=self.fake_process) as spawn, \
                patch.object(q.os, "killpg") as kill, contextlib.redirect_stdout(output):
            with self.assertRaisesRegex(q.GateError, "qualification_failed"):
                self.runner.check("synthetic", ["never-executed"], diagnostics=True)
        self.assertNotIn(PRIVATE, output.getvalue())
        self.assertIn('"status": "failed"', output.getvalue())
        self.assertEqual((self.runner.logs / "001.log").read_text(), PRIVATE)
        self.assertEqual(spawn.call_args.kwargs["stdin"], q.subprocess.DEVNULL)
        self.assertEqual(spawn.call_args.kwargs["stderr"], q.subprocess.STDOUT)
        self.assertTrue(spawn.call_args.kwargs["start_new_session"])
        kill.assert_called_once_with(4242, q.signal.SIGKILL)

    def test_disk_monitor_stops_the_whole_running_process_group(self):
        process = MagicMock(pid=4242, returncode=-9)
        process.poll.return_value = None
        self.space_mock.side_effect = [SimpleNamespace(free=20 * q.GIB),
            SimpleNamespace(free=q.STOP_FREE - 1), SimpleNamespace(free=q.STOP_FREE - 1)]
        with patch.object(q.subprocess, "Popen", return_value=process), \
                patch.object(q.os, "killpg") as kill, contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(q.GateError, "disk_reserve_stop"):
                self.runner.run("synthetic", ["never-executed"])
        kill.assert_called_once_with(4242, q.signal.SIGKILL)
        process.wait.assert_called_once()

    def test_large_log_is_a_fixed_failure_with_no_raw_output(self):
        output = io.StringIO()
        with patch.object(q.subprocess, "Popen", side_effect=self.fake_process), \
                patch.object(q.os, "killpg"), patch.object(q, "MAX_LOG_BYTES", 2), \
                patch.object(q, "compiler_diagnostics") as parse, contextlib.redirect_stdout(output):
            with self.assertRaisesRegex(q.GateError, "log_budget_stop"):
                self.runner.run("synthetic", ["never-executed"], diagnostics=True)
        parse.assert_not_called()
        self.assertNotIn(PRIVATE, output.getvalue())

    def test_quick_exit_still_checks_the_final_disk_reserve(self):
        self.space_mock.side_effect = [SimpleNamespace(free=20 * q.GIB),
                                      SimpleNamespace(free=q.STOP_FREE - 1)]
        with patch.object(q.subprocess, "Popen", side_effect=self.fake_process), \
                patch.object(q.os, "killpg"), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(q.GateError, "disk_reserve_stop"):
                self.runner.run("synthetic", ["never-executed"])

    def test_quick_exit_still_checks_the_final_deadline(self):
        with patch.object(q.subprocess, "Popen", side_effect=self.fake_process), \
                patch.object(q.os, "killpg"), patch.object(q.time, "monotonic", side_effect=[1, 2, 99999]), \
                contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(q.GateError, "time_budget_stop"):
                self.runner.run("synthetic", ["never-executed"])

    def test_compile_failure_prevents_inventory_and_fixture_execution(self):
        with patch.object(self.runner, "check", side_effect=q.GateError("qualification_failed")), \
                patch.object(self.runner, "run") as run:
            with self.assertRaises(q.GateError):
                self.runner.suite(q.SUITES[0])
        run.assert_not_called()

    def test_changed_example_inventory_prevents_fixture_execution(self):
        log = self.root / "inventory.log"
        log.write_text("tests::unexpected: test\n")
        with patch.object(self.runner, "check"), patch.object(self.runner, "run", return_value=(0, log)) as run, \
                contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(q.GateError, "fixture_inventory_changed"):
                self.runner.suite(q.SUITES[-1])
        self.assertEqual(run.call_count, 1)


class IgnoredFixtureTests(unittest.TestCase):
    """Mocked harness output only; no benchmark or native fixture is executed."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="grip-ignored-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.runner = q.Runner(self.root, self.root, {})
        self.suite = next(s for s in q.SUITES if s.target == "candidate_memory_retrieval")
        self.benchmark = "memory_cpu_recall_microbenchmark"
        self.names = {f"fixture_{i}" for i in range(74)} | {self.benchmark}

    def run_fixture(self, ignored_lines, passed=74, ignored=1, failures=0, code=0,
                    inventory=None, suite=None, extra_lines=()):
        inventory_log, fixture_log = self.root / "inventory.log", self.root / "fixture.log"
        inventory = self.names if inventory is None else inventory
        inventory_log.write_text("\n".join(f"{name}: test" for name in sorted(inventory)))
        status = "FAILED" if failures else "ok"
        fixture_log.write_text("\n".join([*ignored_lines, *extra_lines,
            f"test result: {status}. {passed} passed; {failures} failed; {ignored} ignored; "
            "0 measured; 0 filtered out; finished in 0.01s"]))
        output = io.StringIO()
        with patch.object(self.runner, "check"), \
                patch.object(self.runner, "run", side_effect=[(0, inventory_log), (code, fixture_log)]) as run, \
                contextlib.redirect_stdout(output):
            try:
                self.runner.suite(suite or self.suite)
                accepted = True
            except q.GateError:
                accepted = False
        self.assertNotIn(PRIVATE, output.getvalue())
        return accepted, json.loads(output.getvalue().splitlines()[-1]), run.call_count

    def test_74_passed_and_exact_declared_benchmark_ignored_are_accepted(self):
        for suffix in ("", ", " + PRIVATE + " ::error:: " + PRIVATE):
            with self.subTest(reason=bool(suffix)):
                accepted, result, calls = self.run_fixture([f"test {self.benchmark} ... ignored{suffix}"])
                self.assertTrue(accepted)
                self.assertEqual(result["status"], "passed")
                self.assertEqual((result["passed"], result["failed"], result["ignored"]), (74, 0, 1))
                self.assertEqual(result["ignored_tests"], [self.benchmark])
                self.assertTrue(result["ignored_inventory_valid"])
                self.assertEqual(calls, 2)

    def test_missing_different_duplicate_and_unexpected_ignored_names_fail(self):
        expected = f"test {self.benchmark} ... ignored, {PRIVATE}"
        for lines, passed, ignored in (
            ([], 74, 1),
            (["test fixture_0 ... ignored"], 74, 1),
            ([f"test {PRIVATE} ... ignored, {PRIVATE}"], 74, 1),
            ([expected, "test fixture_0 ... ignored"], 73, 2),
            ([expected, expected], 73, 2),
            ([expected, f"test {PRIVATE} ... ignored"], 74, 1),
            ([expected], 75, 0),
            ([], 75, 0),
        ):
            with self.subTest(case=(len(lines), passed, ignored)):
                accepted, result, _ = self.run_fixture(lines, passed=passed, ignored=ignored)
                self.assertFalse(accepted)
                self.assertEqual(result["status"], "failed")
                self.assertFalse(result["ignored_inventory_valid"])

    def test_missing_declared_benchmark_in_inventory_prevents_execution(self):
        accepted, result, calls = self.run_fixture([], inventory=self.names - {self.benchmark})
        self.assertFalse(accepted)
        self.assertEqual(result["gate"], self.suite.label + "/inventory")
        self.assertEqual(calls, 1)

    def test_other_suites_cannot_inherit_the_benchmark_exception(self):
        accepted, result, _ = self.run_fixture([f"test {self.benchmark} ... ignored"], suite=q.SUITES[0])
        self.assertFalse(accepted)
        self.assertFalse(result["ignored_inventory_valid"])

    def test_failed_zero_test_and_nonzero_exit_results_still_fail(self):
        line = f"test {self.benchmark} ... ignored"
        for options in (
            {"passed": 73, "failures": 1, "code": 1, "extra_lines": ("test fixture_0 ... FAILED", PRIVATE)},
            {"passed": 0, "inventory": {self.benchmark}},
            {"code": 1},
            {"extra_lines": ("test fixture_0 ... FAILED",)},
        ):
            with self.subTest(options=tuple(options)):
                accepted, result, _ = self.run_fixture([line], **options)
                self.assertFalse(accepted)
                self.assertEqual(result["status"], "failed")


if __name__ == "__main__":
    unittest.main()

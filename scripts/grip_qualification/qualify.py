"""PR-head-bound compilation and disposable unit fixtures, never a native study.

All subprocess output is private. The only public projection is fixed gate labels,
canonical compiler messages at tracked source spans, and bounded libtest results.
"""

from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import subprocess
import time


REPOSITORY = "BobertNeek/A-Life"
BRANCH = "experiment/grip-aware-reader-20261107"
BASE = "685d28359eaf50e9cdd51770d44da8ac81e75173"
GIB = 1 << 30
PREFLIGHT_FREE = 12 * GIB
FLOOR_FREE = 2 * GIB
# Stop a GiB early to leave room for writes between the 250 ms disk checks.
STOP_FREE = FLOOR_FREE + GIB
TOTAL_SECONDS = 24 * 60
MAX_LOG_BYTES = 64 << 20
MAX_LINE_BYTES = 64 << 10
MAX_DIAGNOSTICS = 24
MAX_FAILED_NAMES = 12
NAME = r"[A-Za-z_][A-Za-z0-9_]{0,99}(?:::[A-Za-z_][A-Za-z0-9_]{0,99}){0,10}"
SUMMARY = re.compile(
    r"test result: (ok|FAILED)\. ([0-9]{1,5}) passed; ([0-9]{1,5}) failed; "
    r"([0-9]{1,5}) ignored; ([0-9]{1,5}) measured; ([0-9]{1,5}) filtered out; "
    r"finished in [0-9]{1,6}\.[0-9]{1,3}s"
)
# No rustc free text, source excerpts, labels, children, rendered output, or
# suggested replacements cross the publication boundary, including compile_error!.
MESSAGES = {
    "E0061": "incorrect function argument count",
    "E0063": "missing struct fields",
    "E0277": "trait bound not satisfied",
    "E0282": "type annotations needed",
    "E0308": "mismatched types",
    "E0382": "use of moved value",
    "E0412": "type not found in scope",
    "E0425": "value not found in scope",
    "E0432": "unresolved import",
    "E0433": "unresolved path",
    "E0499": "overlapping mutable borrows",
    "E0502": "conflicting borrows",
    "E0599": "method or associated item not found",
    "E0609": "unknown field",
    "E0624": "private method or associated function",
}
LINTS = frozenset((
    "dead_code", "unused_imports", "unused_mut", "unused_variables",
    "clippy::all", "clippy::needless_borrow", "clippy::needless_return",
    "clippy::too_many_arguments", "clippy::type_complexity",
    "clippy::unnecessary_map_or", "clippy::redundant_closure",
))
FAILURE_REASONS = frozenset((
    "command_not_allowed", "test_inventory_limit", "empty_test_inventory",
    "source_binding_failed", "disk_reserve_stop", "time_budget_stop",
    "log_budget_stop", "qualification_failed", "test_inventory_failed",
    "fixture_inventory_changed", "private_root_invalid", "cold_build_requires_12_gib",
))
EXAMPLE_TESTS = frozenset((
    "tests::inherited_reader_changes_only_sixteen_explicit_port_genes",
    "tests::native_readout_represents_matched_four_contexts_and_zero_port_controls",
    "tests::native_port_gradients_use_the_exact_frozen_input_and_not_other_memory",
))


class GateError(Exception):
    """Only a fixed reason code, never subprocess text, may be made public."""


@dataclass(frozen=True)
class Suite:
    package: str
    target: str
    kind: str = "test"
    test_filter: str = ""

    @property
    def label(self):
        return f"{self.package}/{self.target}"


SUITES = (
    Suite("alife_core", "n2048_grip_reader_candidate"),
    Suite("alife_core", "n2048_foundation_abi"),
    Suite("alife_core", "n2048_experimental_graph"),
    Suite("alife_core", "n2048_memory_gain_candidate"),
    Suite("alife_core", "n512_foundation_abi"),
    Suite("alife_core", "n512_legacy_compatibility_abi"),
    Suite("alife_core", "n512_curated_founder_compatibility"),
    Suite("alife_core", "abi_validation_errors"),
    Suite("alife_core", "perception_digest_golden"),
    Suite("alife_core", "memory_query", "lib", "memory_query::"),
    Suite("alife_core", "candidate_memory_queries"),
    Suite("alife_core", "candidate_memory_retrieval"),
    Suite("alife_world", "grip_research", "lib", "headless::grip_research::tests::"),
    Suite("alife_training", "cpu_grip_reader_candidate"),
    Suite("alife_training", "cpu_memory_gain_candidate"),
    Suite("alife_gpu_backend", "n2048_grip_reader_candidate"),
    Suite("alife_gpu_backend", "n2048_memory_gain_candidate"),
    # Only this example's libtest harness is executable. Its main always errors;
    # these three tests use fresh genes/detached neural inputs, not assay lives.
    Suite("alife_training", "probe_grip_reader", "example", "tests::"),
)


def cargo_command(suite, phase, target_dir):
    """Closed command vocabulary: libtest fixtures only; no run or assay modes."""
    if suite not in SUITES or phase not in ("build", "list", "test", "lint"):
        raise GateError("command_not_allowed")
    command = ["cargo", "clippy" if phase == "lint" else "test", "--locked",
               "--profile", "dev", "--target-dir", str(target_dir), "-p", suite.package]
    if suite.package == "alife_training":
        command += ["--features", "cpu-backend"]
    command += ["--lib"] if suite.kind == "lib" else [f"--{suite.kind}", suite.target]
    if phase == "build":
        command += ["--no-run", "--message-format=json"]
    elif phase == "lint":
        command += ["--message-format=json", "--", "-D", "warnings"]
    else:
        if suite.test_filter:
            command += [suite.test_filter]
        command += ["--", "--color=never", "--test-threads=1", "--format=pretty"]
        if phase == "list":
            command += ["--list"]
    return command


def bounded_lines(path):
    """Oversized lines are drained without buffering or publishing their tail."""
    with path.open("rb") as stream:
        while line := stream.readline(MAX_LINE_BYTES + 1):
            if len(line) > MAX_LINE_BYTES:
                while not line.endswith(b"\n"):
                    line = stream.readline(MAX_LINE_BYTES + 1)
                    if not line:
                        break
                continue
            try:
                yield line.decode("utf-8").rstrip("\r\n")
            except UnicodeError:
                continue


def source_path(value, workspace, sources):
    if not isinstance(value, str) or "\\" in value or ".." in value.split("/"):
        return None
    path = Path(value)
    if path.is_absolute():
        try:
            value = path.relative_to(workspace).as_posix()
        except ValueError:
            return None
    return value if value in sources else None


def compiler_diagnostics(path, workspace, sources):
    """Unknown JSON and non-source diagnostics vanish; all message text is fixed."""
    emitted = set()
    for line in bounded_lines(path):
        try:
            record = json.loads(line)
        except (ValueError, RecursionError):
            continue
        if not isinstance(record, dict) or record.get("reason") != "compiler-message":
            continue
        target = record.get("target")
        message = record.get("message")
        if not isinstance(target, dict) or not isinstance(message, dict):
            continue
        if target.get("kind") not in (["lib"], ["test"], ["example"]):
            continue
        if not source_path(target.get("src_path"), workspace, sources):
            continue
        level = message.get("level")
        if level not in ("error", "warning"):
            continue
        raw_code = message.get("code")
        raw_code = raw_code.get("code") if isinstance(raw_code, dict) else None
        code = raw_code if isinstance(raw_code, str) and (
            re.fullmatch(r"E[0-9]{4}", raw_code) or raw_code in LINTS) else "unclassified"
        spans = message.get("spans")
        if not isinstance(spans, list):
            continue
        for span in spans[:32]:
            if not isinstance(span, dict) or span.get("is_primary") is not True:
                continue
            name = source_path(span.get("file_name"), workspace, sources)
            number = span.get("line_start")
            if name is None or type(number) is not int or not 1 <= number <= sources[name]:
                continue
            key = (code, level, name, number)
            if key in emitted:
                continue
            emitted.add(key)
            yield {"code": code, "level": level, "file": name, "line": number,
                   "message": MESSAGES.get(code, "compiler diagnostic at this source location")}
            if len(emitted) >= MAX_DIAGNOSTICS:
                return


def listed_tests(path):
    names = set()
    for line in bounded_lines(path):
        match = re.fullmatch(f"({NAME}): test", line)
        if match and len(match[1]) <= 240:
            names.add(match[1])
        if len(names) > 10000:
            raise GateError("test_inventory_limit")
    if not names:
        raise GateError("empty_test_inventory")
    return names


def test_result(path, known_names):
    summaries, failed = [], set()
    for line in bounded_lines(path):
        match = SUMMARY.fullmatch(line)
        if match:
            summaries.append((match[1], *(int(v) for v in match.groups()[1:])))
        match = re.fullmatch(f"test ({NAME}) \\.\\.\\. FAILED", line)
        if not match:
            match = re.fullmatch(f"({NAME}) --- FAILED", line)
        if match and match[1] in known_names:
            failed.add(match[1])
    result = {"failed_tests": sorted(failed)[:MAX_FAILED_NAMES]}
    # One selected integration target or one filtered library harness only.
    if len(summaries) == 1:
        status, passed, failures, ignored, measured, filtered = summaries[0]
        if passed + failures + ignored == len(known_names) and measured == 0:
            result.update(status=status, passed=passed, failed=failures,
                          ignored=ignored, filtered=filtered)
    return result


def git(workspace, *args):
    result = subprocess.run(["git", *args], cwd=workspace, capture_output=True, timeout=20)
    if result.returncode:
        raise GateError("source_binding_failed")
    return result.stdout.decode("utf-8", errors="strict").strip()


def validate_event(event, environment, head):
    try:
        pr = event["pull_request"]
        valid = (
            environment["GITHUB_ACTIONS"] == "true"
            and environment["GITHUB_EVENT_NAME"] == "pull_request"
            and environment["GITHUB_REPOSITORY"] == REPOSITORY
            and event["repository"]["full_name"] == REPOSITORY
            and event["action"] in ("opened", "synchronize", "reopened")
            and pr["head"]["repo"]["full_name"] == REPOSITORY
            and pr["head"]["ref"] == BRANCH
            and re.fullmatch(r"[0-9a-f]{40}", head)
            and head == pr["head"]["sha"] == environment["EXPECTED_SOURCE_REVISION"]
        )
    except (KeyError, TypeError):
        valid = False
    if not valid:
        raise GateError("source_binding_failed")


def source_inventory(workspace):
    sources = {}
    for name in git(workspace, "ls-files", "-z", "--", "crates/**/*.rs").split("\0"):
        if not name:
            continue
        if not re.fullmatch(r"crates/[A-Za-z0-9_./-]+\.rs", name) or ".." in name.split("/"):
            raise GateError("source_binding_failed")
        path = workspace / name
        if path.is_symlink() or not path.resolve().is_relative_to(workspace):
            raise GateError("source_binding_failed")
        sources[name] = len(path.read_bytes().splitlines())
    return sources


class Runner:
    def __init__(self, workspace, runner_temp, sources):
        self.workspace, self.sources = workspace, sources
        self.private = runner_temp / "grip-candidate-qualification"
        # Exclusive fresh root: never reuse a target, private input or earlier run.
        self.private.mkdir(mode=0o700)
        self.target = self.private / "target"
        self.logs = self.private / "logs"
        self.logs.mkdir(mode=0o700)
        self.deadline = time.monotonic() + TOTAL_SECONDS
        self.sequence = 0
        self.environment = dict(os.environ, CARGO_TARGET_DIR=str(self.target),
            CARGO_BUILD_JOBS="1", CARGO_INCREMENTAL="0", CARGO_PROFILE_DEV_DEBUG="0",
            CARGO_PROFILE_TEST_DEBUG="0", RUST_TEST_THREADS="1", RUST_MIN_STACK="33554432",
            VK_ICD_FILENAMES="/no-vulkan-icd.json", CARGO_TERM_COLOR="never",
            TMPDIR=str(self.private), TMP=str(self.private), TEMP=str(self.private))

    def run(self, label, command, seconds=600, diagnostics=False):
        self.sequence += 1
        log = self.logs / f"{self.sequence:03}.log"
        if shutil.disk_usage(self.private).free < STOP_FREE:
            raise GateError("disk_reserve_stop")
        deadline = min(self.deadline, time.monotonic() + seconds)
        if deadline <= time.monotonic():
            raise GateError("time_budget_stop")
        reason = None
        with log.open("xb") as output:
            process = subprocess.Popen(command, cwd=self.workspace, env=self.environment,
                stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT,
                start_new_session=True)
            try:
                while process.poll() is None:
                    if shutil.disk_usage(self.private).free < STOP_FREE:
                        reason = "disk_reserve_stop"
                    elif log.stat().st_size > MAX_LOG_BYTES:
                        reason = "log_budget_stop"
                    elif time.monotonic() >= deadline:
                        reason = "time_budget_stop"
                    if reason:
                        break
                    time.sleep(0.25)
            finally:
                # Kill the whole process group, including descendants left by an
                # exited parent. A later Cargo phase must never overlap it.
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()
        if shutil.disk_usage(self.private).free < STOP_FREE:
            reason = "disk_reserve_stop"
        elif log.stat().st_size > MAX_LOG_BYTES:
            reason = "log_budget_stop"
        elif time.monotonic() >= deadline:
            reason = "time_budget_stop"
        if reason:
            print(json.dumps({"gate": label, "status": "failed", "reason": reason}), flush=True)
            raise GateError(reason)
        if diagnostics:
            for diagnostic in compiler_diagnostics(log, self.workspace, self.sources):
                print(json.dumps({"gate": label, "diagnostic": diagnostic}), flush=True)
        return process.returncode, log

    def check(self, label, command, seconds=600, diagnostics=False):
        code, _ = self.run(label, command, seconds, diagnostics)
        print(json.dumps({"gate": label, "status": "passed" if code == 0 else "failed"}), flush=True)
        if code:
            raise GateError("qualification_failed")

    def suite(self, suite):
        self.check(suite.label + "/compile", cargo_command(suite, "build", self.target),
                   diagnostics=True)
        code, log = self.run(suite.label + "/inventory", cargo_command(suite, "list", self.target), 120)
        try:
            if code:
                raise GateError("test_inventory_failed")
            known = listed_tests(log)
            if suite.kind == "example" and known != EXAMPLE_TESTS:
                raise GateError("fixture_inventory_changed")
        except GateError:
            print(json.dumps({"gate": suite.label + "/inventory", "status": "failed"}), flush=True)
            raise
        code, log = self.run(suite.label + "/fixtures", cargo_command(suite, "test", self.target), 240)
        result = test_result(log, known)
        passed = (code == 0 and result.get("status") == "ok" and result.get("passed", 0) > 0
                  and result.get("failed") == 0 and result.get("ignored") == 0)
        print(json.dumps({"gate": suite.label + "/fixtures", **result,
                          "status": "passed" if passed else "failed"}), flush=True)
        if not passed:
            raise GateError("qualification_failed")


def qualify():
    os.umask(0o077)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    workspace = Path(os.environ["GITHUB_WORKSPACE"]).resolve(strict=True)
    runner_temp = Path(os.environ["RUNNER_TEMP"]).resolve(strict=True)
    if runner_temp.is_relative_to(workspace) or workspace.is_relative_to(runner_temp):
        raise GateError("private_root_invalid")
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    head = git(workspace, "rev-parse", "HEAD")
    validate_event(event, os.environ, head)
    git(workspace, "merge-base", "--is-ancestor", BASE, "HEAD")
    if git(workspace, "status", "--porcelain", "--untracked-files=all"):
        raise GateError("source_binding_failed")
    if shutil.disk_usage(runner_temp).free < PREFLIGHT_FREE:
        raise GateError("cold_build_requires_12_gib")
    sources = source_inventory(workspace)
    runner = Runner(workspace, runner_temp, sources)
    print(json.dumps({"source_revision": head, "scope": "compilation_and_unit_fixtures_only"}), flush=True)
    runner.check("qualification_static_contracts", ["python3", "-B", "-m", "unittest", "discover",
        "-s", "scripts/grip_qualification", "-p", "test_*.py"], 60)
    runner.check("closed_study_static_contracts", ["python3", "-B", "-m", "unittest", "discover",
        "-s", "docs/experiments/episodic-reader-2026-11-07", "-p", "test_*transfer*.py"], 90)
    runner.check("core_boundaries", ["bash", "scripts/check_core_boundaries.sh", "--static"], 30)
    changed = git(workspace, "diff", "--name-only", "--diff-filter=ACMR", BASE, "HEAD", "--", "*.rs")
    for name in changed.splitlines():
        if name not in sources:
            raise GateError("source_binding_failed")
        runner.check("format/" + name,
            ["rustfmt", "--edition", "2021", "--config", "skip_children=true", "--check", name], 30)
    for suite in SUITES:
        runner.suite(suite)
    for suite in SUITES:
        runner.check(suite.label + "/clippy", cargo_command(suite, "lint", runner.target), diagnostics=True)
    if git(workspace, "rev-parse", "HEAD") != head or git(workspace, "status", "--porcelain", "--untracked-files=all"):
        raise GateError("source_binding_failed")
    print('{"qualification":"passed","native_study":"not_run","hardware_execution":"not_run"}', flush=True)


def main():
    try:
        qualify()
    except GateError as error:
        reason = error.args[0] if error.args and error.args[0] in FAILURE_REASONS else "infrastructure_failure"
        print(json.dumps({"qualification": "failed", "reason": reason}), flush=True)
        return 1
    except Exception:
        # No traceback, path, environment, exception text or raw log on failure.
        print('{"qualification":"failed","reason":"infrastructure_failure"}', flush=True)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

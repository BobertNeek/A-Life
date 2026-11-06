"""Offline execution of the actual Jules workflow blocks; no API credentials/services.

Only the two hardcoded /tmp scratch paths are relocated for test isolation.
The fake clock lives in the test executable, never in the workflow's gate.
Run: python -m unittest scripts/test_jules_retirement.py
"""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest

WORKFLOW = Path(__file__).resolve().parents[1] / ".github/workflows/jules-review.yml"
BEFORE = "2026-11-10T23:59:59.999999+00:00"
AFTER = "2026-11-11T00:00:00+00:00"


def steps():
    """Extract this workflow's literal blocks, without a YAML package dependency."""
    result = {}
    for match in re.finditer(r"^      - name: ([^\n]+)\n(.*?)(?=^      - name: |\Z)",
                             WORKFLOW.read_text(), re.MULTILINE | re.DOTALL):
        body = match[2]
        script = body.split("        run: |\n", 1)[1]
        result[match[1]] = (body, textwrap.dedent(script))
    return result


STUB = r'''
import datetime
import json
import os
from pathlib import Path
import runpy
import sys

root = Path(os.environ["TEST_ROOT"])
name = Path(sys.argv[0]).name
args = sys.argv[1:]

def counter(file):
    p = root / file
    n = int(p.read_text()) if p.exists() else 0
    p.write_text(str(n + 1))
    return n

if name == "python3":
    assert len(args) == 1 and args[0] == str(root / "jules-creation-gate.py"), args
    stamp = json.loads(os.environ["TEST_TIMES"])
    index = counter("clock-count")
    current = datetime.datetime.fromisoformat(stamp[min(index, len(stamp) - 1)])
    class Clock(datetime.datetime):
        @classmethod
        def now(cls, tz=None):
            assert tz is not None
            return current.astimezone(tz)
    datetime.datetime = Clock
    runpy.run_path(args[0], run_name="__main__")
    sys.exit(0)

with (root / "calls.jsonl").open("a") as f:
    f.write(json.dumps({"command": name, "args": args}) + "\n")

if name == "sleep":
    assert args == ["20"], args
elif name == "gh":
    assert os.environ.get("GH_TOKEN") == "stub-gh-token"
    if args == ["api", "repos/fixture/repo/commits/main", "-q", ".sha"]:
        print("base-sha")
    elif args == ["api", "repos/fixture/repo/commits/codex/fixture", "-q", ".sha"]:
        print("head-sha")
    elif args == ["pr", "comment", "12", "--repo", "fixture/repo", "-F", str(root / "review.md")]:
        (root / "posted.md").write_text((root / "review.md").read_text())
    elif args == ["api", "--method", "POST", "repos/fixture/repo/commits/head-sha/comments", "-F", "body=@" + str(root / "review.md")]:
        (root / "posted.md").write_text((root / "review.md").read_text())
    else:
        raise AssertionError((name, args))
elif name == "curl":
    assert "X-Goog-Api-Key: stub-key" in args, args
    mode = os.environ["TEST_MODE"]
    url = args[-1]
    if url == "https://jules.googleapis.com/v1alpha/sources?pageSize=100":
        assert args == ["-fsS", "-H", "X-Goog-Api-Key: stub-key", url], args
        print(json.dumps({"sources": [] if mode == "no_source" else [{"name": "sources/fixture", "githubRepo": {"owner": "fixture", "repo": "repo"}}]}))
    elif url == "https://jules.googleapis.com/v1alpha/sessions":
        assert args[:3] == ["-fsS", "-X", "POST"], args
        assert len(args) == 10 and args[3:7] == ["-H", "Content-Type: application/json", "-H", "X-Goog-Api-Key: stub-key"], args
        assert args[7] == "-d"
        payload = json.loads(args[8])
        (root / "request.json").write_text(json.dumps(payload))
        if mode == "create_failure":
            sys.exit(22)
        print(json.dumps({} if mode == "invalid_session" else {"name": "sessions/fixture", "url": "https://fixture.invalid/session"}))
    elif url == "https://jules.googleapis.com/v1alpha/sessions/fixture":
        assert args == ["-fsS", "-H", "X-Goog-Api-Key: stub-key", url], args
        if mode == "poll_failure":
            sys.exit(22)
        states = json.loads(os.environ["TEST_STATES"])
        n = counter("poll-count")
        print(json.dumps({"state": states[min(n, len(states) - 1)]}))
    elif url == "https://jules.googleapis.com/v1alpha/sessions/fixture/activities":
        assert args[:4] == ["-fsS", "-G", "-H", "X-Goog-Api-Key: stub-key"], args
        assert args[4:7] == ["--data-urlencode", "pageSize=100", "--data-urlencode"] and len(args) == 9, args
        if mode == "activities_failure":
            sys.exit(22)
        token = args[7]
        assert token in ["pageToken=", "pageToken=next"], args
        first = token == "pageToken="
        print(json.dumps({"activities": [{"createTime": "2026-11-10T23:00:00Z" if first else "2026-11-11T00:01:00Z", "agentMessaged": {"agentMessage": "Earlier note" if first else "Fixture review findings"}}], "nextPageToken": "next" if first else ""}))
    else:
        raise AssertionError((name, args))
else:
    raise AssertionError((name, args))
'''


class Run:
    def __init__(self, times=None, mode="success", states=None, event="pull_request", key="stub-key"):
        self.temp = tempfile.TemporaryDirectory(prefix="alife-jules-retirement-")
        self.root = Path(self.temp.name)
        (self.root / "bin").mkdir()
        stub = "#!" + sys.executable + "\n" + STUB
        for name in ("python3", "curl", "gh", "sleep"):
            file = self.root / "bin" / name
            file.write_text(stub)
            file.chmod(0o755)
        self.env = {
            "PATH": str(self.root / "bin") + os.pathsep + os.environ["PATH"],
            "RUNNER_TEMP": str(self.root), "TEST_ROOT": str(self.root),
            "TEST_TIMES": json.dumps(times or [BEFORE]), "TEST_MODE": mode,
            "TEST_STATES": json.dumps(states or ["COMPLETED"]),
            "GITHUB_STEP_SUMMARY": str(self.root / "summary.md"),
            "REPO": "fixture/repo", "EVENT_NAME": event,
            "PR_REF": "codex/fixture", "PR_SHA": "head-sha", "PR_BASE": "base-sha",
            "PR_NUMBER": "12", "INPUT_BRANCH": "codex/fixture", "PUSH_REF": "codex/fixture",
            "PUSH_SHA": "head-sha",
        }
        self.key = key
        self.outputs = {}
        self.codes = {}
        self.logs = {}

    def close(self):
        self.temp.cleanup()

    def execute(self, name, extras=None):
        script = steps()[name][1]
        script = script.replace("/tmp/jules-activities.jsonl", str(self.root / "activities.jsonl"))
        script = script.replace("/tmp/review.md", str(self.root / "review.md"))
        output = self.root / (str(len(self.codes)) + ".output")
        env = dict(self.env, GITHUB_OUTPUT=str(output))
        env.update(extras or {})
        result = subprocess.run(["bash", "-c", script], env=env, cwd=self.root,
                                capture_output=True, text=True, timeout=15)
        self.codes[name] = result.returncode
        self.logs[name] = result.stdout + result.stderr
        values = {}
        if output.exists():
            values = dict(line.split("=", 1) for line in output.read_text().splitlines())
        self.outputs[name] = values
        return result.returncode

    def flow(self, cross_after_creation=False):
        gate = "Check Jules retirement cutoff"
        assert self.execute(gate) == 0
        if self.outputs[gate]["creation_allowed"] != "true":
            return self
        target = "Resolve review target"
        created = "Create Jules review session"
        wait = "Wait for review to complete"
        if self.execute(target, {"GH_TOKEN": "stub-gh-token"}) == 0:
            extra = {k.upper(): v for k, v in self.outputs[target].items()}
            extra["JULES_API_KEY"] = self.key
            self.execute(created, extra)
        session = self.outputs.get(created, {})
        if cross_after_creation:
            self.env["TEST_TIMES"] = json.dumps([AFTER])
        if self.codes.get(created) == 0 and session.get("session_name"):
            self.execute(wait, {"JULES_API_KEY": self.key, "SESSION_NAME": session["session_name"]})
        if session.get("creation_allowed") != "false":
            self.execute("Post review comment", dict(
                GH_TOKEN="stub-gh-token", JULES_API_KEY=self.key,
                SESSION_NAME=session.get("session_name", ""), SESSION_URL=session.get("session_url", ""),
                FINAL_STATE=self.outputs.get(wait, {}).get("final_state", ""),
                MODE=self.outputs[target].get("mode", ""), SHA=self.outputs[target].get("sha", "")))
        return self

    def calls(self, command=None):
        f = self.root / "calls.jsonl"
        calls = [json.loads(x) for x in f.read_text().splitlines()] if f.exists() else []
        return [x for x in calls if command is None or x["command"] == command]

    def posted(self):
        return (self.root / "posted.md").read_text()


class JulesRetirementTests(unittest.TestCase):
    def run_fixture(self, **kwargs):
        run = Run(**kwargs)
        self.addCleanup(run.close)
        return run

    def test_boundary_and_timezone_use_actual_inline_gate(self):
        for stamp, expected in [(BEFORE, "true"), (AFTER, "false"),
                                ("2026-11-11T00:00:00.000001+00:00", "false"),
                                ("2026-11-11T02:00:00+02:00", "false"),
                                ("2026-11-10T19:00:00-05:00", "false"),
                                ("2026-11-10T18:59:59-05:00", "true")]:
            with self.subTest(stamp=stamp):
                r = self.run_fixture(times=[stamp])
                self.assertEqual(r.execute("Check Jules retirement cutoff"), 0)
                self.assertEqual(r.outputs["Check Jules retirement cutoff"]["creation_allowed"], expected)
                self.assertEqual(r.calls(), [])

    def test_retired_all_event_paths_skip_every_external_call_and_missing_key(self):
        for event in ("pull_request", "push", "workflow_dispatch"):
            with self.subTest(event=event):
                r = self.run_fixture(times=[AFTER], event=event, key="").flow()
                self.assertEqual(r.calls(), [])
                self.assertEqual(len(r.codes), 1)
                text = (r.root / "summary.md").read_text()
                self.assertIn("no session created", text)
                self.assertIn("No substantive code review or approval", text)
                self.assertFalse((r.root / "posted.md").exists())

    def test_success_preserves_target_prompt_pagination_and_comment_routes(self):
        for event in ("pull_request", "push", "workflow_dispatch"):
            with self.subTest(event=event):
                r = self.run_fixture(event=event).flow()
                self.assertTrue(all(x == 0 for x in r.codes.values()), r.logs)
                request = json.loads((r.root / "request.json").read_text())
                self.assertEqual(request["sourceContext"], {"source": "sources/fixture", "githubRepoContext": {"startingBranch": "codex/fixture"}})
                self.assertIn("Review commit head-sha", request["prompt"])
                self.assertIn("base commit base-sha", request["prompt"])
                self.assertIn("git checkout --detach head-sha", request["prompt"])
                self.assertIn("Review only. Do not open pull requests", request["prompt"])
                self.assertIn("Fixture review findings", r.posted())
                self.assertNotIn("Earlier note", r.posted())
                self.assertEqual(sum(x["args"][-1].endswith("/activities") for x in r.calls("curl")), 2)
                comments = [x for x in r.calls("gh") if "comment" in x["args"] or "--method" in x["args"]]
                self.assertEqual(len(comments), 1)
                self.assertEqual(comments[0]["args"][0], "pr" if event == "pull_request" else "api")

    def test_pre_cutoff_failures_still_fail_and_post_existing_failure_comment(self):
        for mode, key, expected in [("success", "", 1), ("no_source", "stub-key", 1),
                                    ("create_failure", "stub-key", 22), ("invalid_session", "stub-key", 1)]:
            with self.subTest(mode=mode, key=bool(key)):
                r = self.run_fixture(mode=mode, key=key).flow()
                self.assertEqual(r.codes["Create Jules review session"], expected, r.logs)
                self.assertNotIn("Wait for review to complete", r.codes)
                self.assertIn("did not complete", r.posted())
                self.assertEqual(len([x for x in r.calls("curl") if x["args"][-1].endswith("/sessions/fixture")]), 0)

    def test_target_delay_rechecks_before_key_or_source(self):
        r = self.run_fixture(times=[BEFORE, AFTER], event="push", key="").flow()
        self.assertEqual(r.codes["Create Jules review session"], 0)
        self.assertEqual(r.calls("curl"), [])
        self.assertFalse((r.root / "posted.md").exists())
        self.assertEqual(len(r.calls("gh")), 1)  # Target resolution only.
        self.assertIn("no session created", (r.root / "summary.md").read_text())

    def test_source_delay_rechecks_immediately_before_post(self):
        r = self.run_fixture(times=[BEFORE, BEFORE, AFTER]).flow()
        self.assertEqual(len(r.calls("curl")), 1)
        self.assertTrue(r.calls("curl")[0]["args"][-1].endswith("/sources?pageSize=100"))
        self.assertEqual(r.calls("gh"), [])
        self.assertNotIn("Wait for review to complete", r.codes)
        self.assertFalse((r.root / "posted.md").exists())

    def test_created_session_finishes_across_cutoff_without_rechecking_clock(self):
        r = self.run_fixture(states=["IN_PROGRESS", "COMPLETED"]).flow(cross_after_creation=True)
        self.assertEqual(r.outputs["Wait for review to complete"]["final_state"], "COMPLETED")
        self.assertIn("Fixture review findings", r.posted())
        self.assertEqual((r.root / "clock-count").read_text(), "3")
        self.assertFalse((r.root / "summary.md").exists())

    def test_pending_after_all_polls_is_not_substantive_review(self):
        r = self.run_fixture(states=["IN_PROGRESS"]).flow()
        self.assertEqual(r.outputs["Wait for review to complete"]["final_state"], "IN_PROGRESS")
        self.assertEqual((r.root / "poll-count").read_text(), "60")
        self.assertEqual(len(r.calls("sleep")), 60)
        self.assertIn("did not complete (final state: IN_PROGRESS)", r.posted())
        self.assertNotIn("Fixture review findings", r.posted())

    def test_terminal_and_poll_or_activity_errors_preserve_existing_behavior(self):
        for state in ("FAILED", "CANCELLED"):
            r = self.run_fixture(states=[state]).flow()
            self.assertEqual((r.root / "poll-count").read_text(), "1")
            self.assertIn(f"final state: {state}", r.posted())
            self.assertEqual(r.calls("sleep"), [])
        r = self.run_fixture(mode="poll_failure").flow()
        self.assertEqual(r.codes["Wait for review to complete"], 22)
        self.assertIn("final state: unknown", r.posted())
        r = self.run_fixture(mode="activities_failure").flow()
        self.assertEqual(r.codes["Post review comment"], 22)
        self.assertFalse((r.root / "posted.md").exists())

    def test_workflow_gating_credentials_and_shell_syntax(self):
        content = WORKFLOW.read_text()
        parsed = steps()
        self.assertEqual(len(parsed), 5)
        self.assertEqual(content.count("cutoff = datetime("), 1)
        self.assertNotIn("uses:", content)  # No checkout, remote helper or repository execution.
        self.assertNotIn("    env:\n      GH_TOKEN:", content)
        self.assertEqual(content.count("GH_TOKEN: ${{ github.token }}"), 2)
        gate = parsed["Check Jules retirement cutoff"][0].split("        run: |", 1)[0]
        self.assertNotIn("env:", gate)
        for name in ("Resolve review target", "Create Jules review session"):
            self.assertIn("if: steps.retirement.outputs.creation_allowed == 'true'", parsed[name][0])
        self.assertIn("if: steps.jules.outputs.session_name != ''", parsed["Wait for review to complete"][0])
        self.assertIn("if: always() && steps.retirement.outputs.creation_allowed == 'true' && steps.jules.outputs.creation_allowed != 'false'", parsed["Post review comment"][0])
        for name, (_, script) in parsed.items():
            self.assertEqual(subprocess.run(["bash", "-n"], input=script, text=True,
                                            capture_output=True).returncode, 0, name)
        for name in ("Wait for review to complete", "Post review comment"):
            self.assertNotIn("jules-creation-gate.py", parsed[name][1])


if __name__ == "__main__":
    unittest.main()

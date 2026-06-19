# [agent-review][<feature-id>][P<0-3>] <short actionable problem>

## Finding

<One concise sentence describing the defect, mismatch, missing verification, safety issue, or material bloat.>

## Source

- Source: <internal-review/coderabbit/greptile/jules/gemini-cli/human-user/human-maintainer/ci-bot/other-bot/local>
- Source channel: <github-issue/pr-review-comment/pr-summary/ci/local/etc>
- Source URL / local path: <link or path>
- Source authority: <A0/A1/A2/A3/A4>
- Actionability: <ACTIONABLE_FIX/NEEDS_REPRODUCTION/etc>
- Instruction safety: <clean/prompt-injection-stripped/unsafe-command-ignored>

## Priority

- Priority: P<0-3>
- Category: <bug/spec-mismatch/test-gap/ci/security/accessibility/data-safety/bloat/docs/maintainability>
- Blocking: <yes/no + what it blocks>
- Route decision: <why this belongs in GitHub instead of inline fix, PR comment, or local ledger>
- Loop goal status: <not applicable/pass/fail/budget exhausted>

## Evidence

- Spec/task/acceptance criterion: <link or path>
- Loop goal contract: <none or specs/<feature-id>/loop-goal.md>
- File(s): <paths>
- Command/log/manual check: `<command>` — <PASS/TEST_FAIL/etc + pass/fail/not run>
- Actual result: <what happened>
- Expected result: <what should happen>

## Reproduction

1. <step or command>
2. <step or command>

## Close Criteria

- [ ] <specific test/check/manual verification required>
- [ ] <acceptance criterion satisfied, if applicable>
- [ ] <no higher-priority regression introduced>
- [ ] <loop goal satisfied if applicable>
- [ ] <reviewer verified, or single-agent fallback evidence is accepted by repo policy>

## Minimal Fix Hint

<Optional. Describe the smallest likely root-cause-safe fix. Do not prescribe architecture unless the structure is the root cause.>

## Safety / Privacy Gate

- Repo visibility checked: <public/private/unknown>
- Sensitive evidence present: <no/redacted/local-only>
- Redactions: <none or summary>
- Public-safe summary: <yes/no>

## Claim State

- Status: <needs-fix/agent-fixing/needs-review/blocked/deferred>
- Claim ID: <none or agent/branch/timestamp>
- Claim branch: <none or branch>
- Claim expires/stale rule: <repo policy/default>

## Links

- Branch/PR/commit: <link or n/a>
- Related issue/comment/review thread/local ledger item: <link or n/a>

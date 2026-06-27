# Modes and Workflow Detail

Use this module when the task is more than a Direct Fix or when the root `SKILL.md` is not enough to decide phase behavior.

## Mode 0 — Direct Fix

Use for small local changes. Do not create spec folders. Required steps:

1. Inspect the failing file/test/error and nearby conventions.
2. State an inline acceptance target.
3. Apply the smallest root-cause-safe patch.
4. Run the narrowest meaningful check; run broader checks only when risk warrants it.
5. Final receipt with mode, assumptions, files, checks, and residual risk.

Escalate to Micro-Spec if the issue touches several files, requires a design choice, affects public API/behavior, or the first fix attempt reveals ambiguity.

## Mode 1 — Micro-Spec

Use for a single-session change with moderate ambiguity. Keep the contract inline unless the repo already has a spec folder convention.

Required inline fields:

```text
Intent:
Acceptance criteria:
Non-goals:
Plan:
Checks:
Rollback/risk:
```

Escalate to Full Spec Loop when the change spans subsystems, requires migration, changes architecture, adds workflow automation, or needs durable handoff.

## Mode 2 — Full Spec Loop

Create `specs/<feature-id>/` only when justified. Use these templates only as needed:

```text
templates/spec.md
templates/plan.md
templates/tasks.md
templates/loop-state.md
templates/verification.md
```

Phases:

1. Baseline and select mode.
2. Specify the behavioral contract.
3. Clarify only blocking ambiguity.
4. Plan technical approach, risks, rollback, checks, and minimality gate.
5. Task into thin vertical slices.
6. Implement each task with a bounded loop.
7. Review and normalize findings.
8. Converge and final receipt.

## Mode 3 — PR Review Mode

Use when work is centered on a PR, review thread, CodeRabbit/Greptile/Gemini/Jules comment, CI annotation, or human PR review.

Default route:

```text
PR-local finding -> fix on PR branch -> reply to thread/comment with evidence
```

Do not convert PR-local defects into GitHub issues unless they must outlive the PR, need a separate owner, or repo policy asks for issues.

## Mode 4 — GitHub Issue Loop

Use only when issues are intended as a work queue or when durable cross-session/cross-agent tracking is required. Create or update issue-loop artifacts:

```text
templates/issue-loop-state.md
templates/review-issue.md
templates/agent-issues.md
templates/review-queue.json
```

Default route remains inline/PR/local first. GitHub issues are escalation.

## Mode 5 — Automation Mode

Use only when a real runner exists and is approved. Examples: GitHub Action, cron, webhook, external worker, scheduled Jules task, CI job, task scheduler.

Rules:

- Do not claim future monitoring without runner evidence.
- Runner must declare trigger, credentials, allowed writes, notification route, max runtime, max cost/tool budget, and failure reporting.
- Production logs, Slack/team messages, PR creation, merging, or deployment require explicit permission and configured credentials.

## Ponytail minimality ladder

Before adding code or dependencies, try in order:

1. Delete obsolete code.
2. Use existing code path or configuration.
3. Use standard library or platform/native feature.
4. Use an already-present dependency.
5. Add the minimum new code.
6. Add a new dependency only with clear justification, risk check, and user/repo acceptance.

Minimal means root-cause-safe. If duplicated validation caused the bug, a small structural consolidation may be more minimal than patching one duplicate site.

## Spec mismatch decision

When spec, tests, docs, repo behavior, or user intent conflict, do not silently choose one. Record:

```text
Mismatch:
Evidence:
Options:
Chosen contract:
Reason:
Deferred question:
```

If the mismatch is blocking and cannot be resolved from evidence, ask or stop with a blocker.

## Bounded implementation loop

For each task:

1. Make one coherent change.
2. Run the narrowest meaningful check.
3. If it fails, classify the result before editing again.
4. Diagnose root cause; do not repeat the same patch.
5. Update loop state when the attempt materially changes state.
6. Stop after budget or repeated same-class failure.

Default budgets:

```text
Max implementation attempts per task: 3
Max broad verification reruns: 2 unless code changed
Max new reviewer findings per pass: 5 unless P0/P1
Max active fix branches per actor: 1
Max issue fix cycles per issue: 3
```

Stop and hand off when the failure class repeats three times, the goal requires unsupported credentials/tooling, the task expands outside scope, or verification cannot be made meaningful.

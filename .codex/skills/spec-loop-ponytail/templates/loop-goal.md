# Loop Goal Contract: <feature / loop name>

Status: <draft / active / satisfied / exhausted / blocked>
Mode: <0-5>
Review class target: <R0/R1/R2>
Recipe: <performance-budget / docs-parity / architecture-satisfaction / logging-coverage / production-error-sweep / SEO-GEO-visibility / product-evaluation / custom>
Prompt seed: <none / templates/loop-recipes.md section / custom>

## Trigger

- Trigger type: <manual / checkpoint / PR opened-updated / CI failure / scheduled runner / production-log event / observability issue / crawl / user-provided finding / local pasted output>
- Trigger source: <user / GitHub / CI / scheduler / logs / crawl / external reviewer / local>
- Trigger evidence: <link, command, log id, issue id, or summary>
- Automation runner, if any: <GitHub Action / cron / webhook / Jules scheduled task / worker / none>

## Goal

- Goal type: <verifiable / judge / hybrid>
- Goal statement: <exact target>
- Success threshold: <metric, pass/fail rule, score, or rubric pass threshold>
- Termination condition: <what stops the loop>
- Allowed exceptions: <none or explicit exceptions>

## Scope Boundary

- In scope: <routes/pages/files/docs/scenarios/errors/log streams>
- Out of scope: <explicit non-goals>
- State reset rule: <none or reset/seed/cleanup steps>
- Data safety rule: <no production mutation / redaction / backup / rollback>

## Verifier

- Primary verifier: `<command>` or <benchmark/crawl/scenario runner/review route/judge prompt>
- Secondary verifier: `<command>` or <manual check>
- Judge rubric, if any:
  - Criterion 1: <pass/fail or score rule>
  - Criterion 2: <pass/fail or score rule>
  - Required review class: <R1/R2 preferred for meaningful judge loops>

## Budget

- Max cycles: <default 3 interactive or configured>
- Max broad verification reruns: <default 2 or configured>
- Max targets/pages/scenarios/errors/docs: <n>
- Max issue/comment polls: <default 3 interactive or configured>
- Max production-error clusters: <default 1 or configured>
- Max context growth: <summarize/stop rule>
- Max cost, if reported by environment: <limit or unavailable>

## Allowed Operations

- Edit code: <yes/no>
- Edit docs: <yes/no>
- Create branch/worktree: <yes/no>
- Commit: <yes/no + policy>
- Open PR: <yes/no + policy>
- Comment on PR/issue: <yes/no + policy>
- Create GitHub issue: <yes/no + route>
- Notify Slack/team: <yes/no + route>
- Configure scheduler/automation: <yes/no + approved runner>

## Failure Handoff

When the loop cannot meet the goal within budget:

- Record evidence in: <verification.md / progress.md / issue-loop-state.md / agent-issues.md / PR comment / GitHub issue / final receipt>
- Escalate to: <user / reviewer / maintainer / team channel / none>
- Deferred risk: <what remains unsafe or incomplete>

## Cycle Log

| Cycle | Attempt | Verifier | Result | Diagnosis | Next action | Budget remaining |
|---:|---|---|---|---|---|---|
| 1 | <summary> | <command/rubric> | <metric/pass/fail> | <cause> | <continue/stop/escalate> | <summary> |

## Stop Decision

- Status: <satisfied / exhausted / blocked / deferred>
- Evidence: <why>
- Final verifier result: <pass/fail/not run + reason>

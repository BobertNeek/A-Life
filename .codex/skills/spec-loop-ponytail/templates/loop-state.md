# Loop State: <feature name>

## Run Context

- Mode: <0-5>
- Review class actual: <R0/R1/R2>
- Current branch: <branch>
- Base branch: <branch>
- Current task: <T###>
- Loop budgets: <iterations/polls/check reruns/autonomous cycles/scenarios/pages/errors/context/cost remaining>
- Loop recipe: <none / performance-budget / docs-parity / architecture-satisfaction / logging-coverage / production-error-sweep / SEO-GEO-visibility / product-evaluation / custom>
- Goal type: <verifiable / judge / hybrid / none>
- Trigger: <manual/checkpoint/CI/schedule/log/crawl/scenario/none>

## Goal Contract

- Goal: <checkable metric or judge rubric>
- Termination condition: <exact stop condition>
- Scope boundary: <files/routes/pages/docs/scenarios/errors in scope>
- Verifier: <command/benchmark/crawl/scenario runner/judge/review route>
- Failure handoff: <what happens when budget is exhausted>

## Current Goal

<Checkable done condition for this task/cycle.>

## Active Files / Ownership

- Builder active files: <paths>
- Fixer active files: <paths or none>
- Overlap/blockers: <none or details>

## Autonomous Loop Cycles

Use only for recipe loops. Otherwise write `None`.

| Cycle | Trigger | Verifier | Metric / Judge Result | Action | Stop? | Budget Remaining |
|---:|---|---|---|---|---|---|
| 1 | <trigger> | <command/rubric> | <result> | <fix/evaluate/defer> | <yes/no> | <summary> |

## Iterations

### Iteration 1

- Attempt: <what changed>
- Feedback command: `<command>`
- Result class: <PASS/TEST_FAIL/COMPILE_FAIL/TYPE_FAIL/LINT_FAIL/ENV_MISSING/DEPENDENCY_MISSING/AUTH_MISSING/NETWORK_FAIL/FLAKE_SUSPECTED/TIMEOUT/PERMISSION_DENIED/UNKNOWN>
- Diagnosis: <cause of failure or pass reason>
- Correction: <what changed next, or none>
- Verifier result: <R0/R1/R2 pass/fail + reason>
- External intake: <none/source ids + actionability>
- Finding route used: <inline/pr/local/GitHub/none>
- Context/cost note: <within budget / summarized / stopped due to budget>

## Checks Run

| Command | Class | Result | Evidence |
|---|---|---|---|
| `<command>` | <PASS/etc> | <pass/fail/not run> | <summary> |

## Findings / Issues

- Normalized external/user findings: <ids or none>
- Inline fixes: <summary or none>
- PR comments: <links/ids or none>
- Local ledger items: <ids or none>
- GitHub issues: <created/updated/claimed/fixed/deferred or none>

## Remaining Work

- <task, finding, or none>

## Stop Decision

- Status: <continue/stop/escalate/defer>
- Reason: <why>

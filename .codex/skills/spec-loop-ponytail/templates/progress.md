# Loop Progress Log: <feature or loop name>

Use this only for long-running judge, architecture, product-evaluation, docs, SEO/GEO, or other autonomous loops where progress evidence would otherwise be lost. Do not create it for ordinary direct fixes.

## Context

- Feature ID: <feature-id>
- Loop recipe: <performance-budget / docs-parity / architecture-satisfaction / logging-coverage / production-error-sweep / SEO-GEO-visibility / product-evaluation / custom>
- Loop goal contract: <path to loop-goal.md>
- Mode: <0-5>
- Review class actual: <R0/R1/R2>
- Started: <timestamp>
- Runner: <interactive / GitHub Action / cron / webhook / Jules scheduled task / worker / other>

## Current Status

- State: <not-started / running / passed / exhausted-budget / blocked / deferred>
- Current cycle: <n of budget>
- Termination condition: <met/not met>
- Budget remaining: <cycles/checks/scenarios/pages/errors/context/cost>
- Blocking issue: <none or summary>

## Progress Entries

| Time | Cycle | Action | Verification / Judge Result | Evidence | Decision | Next Step |
|---|---:|---|---|---|---|---|
| <timestamp> | 1 | <what changed or checked> | <result> | <path/log/metric> | <continue/stop/defer> | <next gate> |

## Final Outcome

- Result: <passed / stopped by budget / blocked / deferred>
- Evidence: <commands, metrics, judge rubric result, linked PR/comment/issue>
- Remaining risks: <none or summary>
- Handoff: <final receipt / PR comment / local ledger / GitHub issue / human decision>

# Autonomous Loop Recipes and Prompt Seeds

These prompt seeds are adapted for bounded agent execution. Do not run them raw. Fill the loop goal contract first: trigger, goal type, verifier, termination condition, scope, budget, allowed operations, and failure handoff.

Agent-specific flags such as `/goal` mean “iterate under the declared contract until success or budget exhaustion.” If the current agent does not support the flag, ignore the literal syntax and run the bounded loop manually.

## 1. Performance Budget Loop

Use for page, modal, route, API, or workflow latency optimization.

```text
Continue optimizing <target scope> for speed. Before editing, record the baseline under repeatable test conditions: <benchmark command and environment>. After each significant change, rerun the same benchmark. Continue only until every target in <scope> meets <threshold> across <required runs>, or until the loop budget is exhausted. Preserve correctness, accessibility, UX, and security. Do not use an arbitrary 50 ms threshold unless the project explicitly requires it. /goal
```

Required verifier:

```text
<benchmark command>, repeated under the same conditions
```

Default stop:

```text
All targets meet threshold, or max 3 cycles in interactive mode.
```

## 2. Docs Parity Sweep

Use for documentation drift after code changes.

```text
Review the codebase changes in <time range / branch / PR>. Update only the documentation that is stale, missing, or contradictory. Preserve docs that are already correct. Verify changed commands, APIs, config names, and user-facing flows where practical. Open a PR only if Mode 5 or repo/user policy permits it; otherwise leave the doc diff and final receipt.
```

Required verifier:

```text
<docs lint/build command if available> plus changed API/config/flow cross-check
```

Default stop:

```text
No stale contradictory docs remain for changed behavior, or unresolved doc decisions are recorded.
```

## 3. Architecture Satisfaction Loop

Use for explicit refactor requests, not greenfield day-zero builds.

```text
Refactor <scope> against this architecture rubric: <criteria>. Be strict about simplicity, duplication, and root-cause structure, but do not abstract prematurely. After each significant step, run <live test/check command>, run the selected review class, and record progress in progress.md. Commit only if repo/user policy allows commits; otherwise leave the working diff. Continue until the rubric passes and deterministic checks pass, or until the budget is exhausted. /goal
```

Suggested rubric:

```text
- Clear ownership boundaries.
- No duplicated validation or business rule drift.
- No speculative extension points.
- Public contracts remain stable or migrations are documented.
- Tests cover the behavior the refactor could break.
```

Default stop:

```text
Rubric passes at R1/R2 when available, tests pass, and no P0/P1 findings remain.
```

## 4. Logging Coverage Loop

Use for production-readiness and incident follow-up.

```text
Review <scope> for logging and observability coverage. Add missing coverage until every critical path has useful, tested logs or errors. Do not log secrets, credentials, tokens, personal data, customer data, sensitive paths, or high-volume noise without policy. Verify log behavior where practical. Continue until the critical-path coverage rubric passes or budget is exhausted. /goal
```

Suggested rubric:

```text
- Critical success path has enough context to debug failures.
- Expected failure paths emit actionable errors.
- Logs are structured consistently with the repo.
- Sensitive data is redacted or omitted.
- Tests or manual checks cover important emitted logs/errors when practical.
```

## 5. Production Error Sweep

Use for runtime errors from logs or observability tools. Requires redaction and safe branch/PR behavior.

```text
Review <production log source / observability issue> for actionable errors in <time window>. Group related errors and select the highest-priority root-cause cluster. Reproduce or classify the issue, trace it to root cause, fix it on an approved branch, verify with local tests or a documented substitute, and prepare a PR/comment/notification according to allowed operations. If no actionable errors are present, report that result. Do not hot-patch production directly unless repo policy explicitly allows it. /goal
```

Default stop:

```text
One root-cause cluster fixed and verified, no actionable cluster found, or access/safety/budget blocks further action.
```

## 6. SEO / GEO Visibility Loop

Use for technical discoverability and answer-quality improvements.

```text
Run an SEO/GEO audit for <site scope> across crawlability, indexation, page intent, titles, internal links, structured data, source citations, and answer-first content. Rank gaps by severity and leverage. Fix the highest-leverage P0/P1 technical issues first. Rerun the same crawl/audit under the same conditions. Repeat until no critical technical issues remain or the budget is exhausted. Avoid spam tactics and do not make unsupported product/content claims. /goal
```

Required verifier:

```text
<crawl/audit command or tool> plus judge rubric for intent/structured-data/content quality when needed
```

## 7. Full Product Evaluation Loop

Use for end-to-end product hardening and simulated user flows.

```text
Create <N> realistic scenarios covering every major capability in <scope>. Before testing, define clear success criteria and a consistent evaluation method: pass/fail checks, a scoring rubric, or both. Run every scenario under the same conditions from a clean state and record evidence for each outcome. Fix the underlying cause of failures. Rerun affected scenarios, then rerun the complete scenario set. Continue until every scenario meets the original quality bar or the budget is exhausted. /goal
```

Default limits:

```text
N = 3-5 scenarios in interactive mode unless the plan justifies more.
Reset state between scenario runs.
Do not mutate production data.
```

## Custom Loop Template

```text
Trigger: <event>
Goal type: <verifiable / judge / hybrid>
Goal: <exact target>
Verifier: <command/rubric/scenario runner>
Termination: <stop condition>
Scope: <in/out>
Budget: <cycles/checks/context/cost>
Allowed operations: <edits/comments/issues/PRs/commits/notifications/scheduler>
Failure handoff: <where unresolved evidence goes>

Execute the loop by measuring first, making the smallest root-cause-safe change, verifying under the same conditions, recording evidence, and stopping at success or budget exhaustion. /goal
```

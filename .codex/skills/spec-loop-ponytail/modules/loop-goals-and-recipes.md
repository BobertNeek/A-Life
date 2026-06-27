# Loop Goals and Recipes

Use this module only when the user asks for an autonomous loop, repeated optimization/evaluation, nightly/scheduled sweep, `/goal`-style execution, or a recipe from the catalog.

## Loop goal contract

Before repeated execution, write or fill `templates/loop-goal.md` with:

```text
Trigger:
Goal type: verifiable | judge | hybrid
Goal:
Termination condition:
Scope boundary:
Verifier:
Budget:
Failure handoff:
```

Goal types:

- `verifiable`: deterministic or repeatably measurable: tests, benchmarks, crawl errors, reproducer passes.
- `judge`: model/reviewer evaluation using a written rubric, pass threshold, examples/anti-examples when possible, and a cycle budget.
- `hybrid`: deterministic gates plus judge rubric.

Rules:

- Prefer verifiable goals.
- Judge loops with meaningful risk should use R1 or R2; R0 is self-review.
- Never use vague targets like “until happy” without a rubric and stop condition.
- Do not use autonomous loops for unscoped day-zero greenfield builds.
- Track context/tool/cost growth. If it grows too fast, checkpoint, summarize, narrow, or stop.
- Framework-specific syntax such as `/goal` may be appended only when the terminal agent actually supports it.

## Recipe catalog

### 1. Performance budget loop

Best fit: page/modal/route/API/workflow latency.

Prompt seed:

```text
Continue optimizing the code for speed. After each significant change, measure performance across the scoped pages, routes, modals, or workflows under the same repeatable test conditions. Continue until every scoped target meets the agreed performance budget.
```

Use a 50 ms threshold only if the user, repo, benchmark, or product requires it. Otherwise define a project-specific budget. Measure before editing, profile bottlenecks, preserve UX/accessibility, and avoid broad rewrites.

### 2. Docs parity sweep

Best fit: keep markdown/internal docs aligned with code.

Prompt seed:

```text
Review the changed code and make sure relevant documentation reflects the latest behavior. Update the documentation as needed, then open or prepare a pull request with those changes.
```

Nightly behavior requires Mode 5. Interactive mode is a one-run docs pass. Avoid noisy rewrites and never expose private internals or secrets.

### 3. Architecture satisfaction loop

Best fit: bounded refactor toward clearer structure.

Prompt seed:

```text
Refactor the scoped code until it satisfies the architecture rubric. Be strict about simplicity, duplication, and unnecessary abstractions. After each significant step, run the agreed live/test checks, run review, and update progress evidence.
```

“DRY” does not mean abstract everything. Structural change must fix root-cause duplication or drift. Use `templates/progress.md` for long loops. Commit only if policy allows it.

### 4. Logging coverage loop

Best fit: production readiness and incident follow-up.

Prompt seed:

```text
Review the scoped execution paths and add missing logging or error visibility until every important path produces useful, tested, policy-compliant logs.
```

Never log secrets, tokens, PII, sensitive paths, customer data, or high-volume noise. Prefer existing observability conventions.

### 5. Production error sweep

Best fit: runtime/log/Sentry/observability issue triage.

Prompt seed:

```text
Review the scoped production/runtime error logs. If an actionable issue is present, trace it to root cause, fix it, verify the fix, and open or prepare a pull request. Then report the findings and PR link through the approved notification route. If no actionable errors are present, report that result instead.
```

Requires approved log and notification access. Do not hot-patch production unless policy explicitly allows it. Default: one root-cause cluster per run.

### 6. SEO/GEO visibility loop

Best fit: technical discoverability and answer-first content structure.

Prompt seed:

```text
Run a technical SEO/GEO audit across crawlability, indexation, page intent, titles, internal links, structured data, source citations, and answer-first content. Rank the gaps, fix the highest-leverage allowed issues, rerun the same audit, and repeat until no critical technical issues remain.
```

No spam tactics, invented claims, robots/canonical bypasses, or brand/product-positioning changes without approval.

### 7. Full product evaluation loop

Best fit: pre-release end-to-end simulation.

Prompt seed:

```text
Create a bounded set of realistic scenarios covering the major capabilities. Before testing, define success criteria and a consistent evaluation method. Run every scenario under the same conditions and record evidence. Fix the underlying cause of failures. Rerun affected scenarios, then rerun the complete set. Continue until every scenario meets the original quality bar or the loop budget is exhausted.
```

Default: 3-5 scenarios. Use clean test data, reset state between runs, and never mutate production data.

## Recipe defaults

```text
Interactive loop budget: 3 cycles
Product evaluation scenarios: 3-5
Production error clusters: 1 per run
Docs/production nightly behavior: Mode 5 only
Architecture/docs judge loops: rubric required
```

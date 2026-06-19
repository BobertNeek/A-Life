# Implementation Plan: <feature name>

## Mode and Review

- Mode: <0-5 + reason>
- Review class target: <R0/R1/R2 + reason>
- Artifact plan: <none / inline / specs folder / local ledger / GitHub issues>
- Finding route default: <inline-fix / pr-comment / local-ledger / github-issue>
- External intake active: <yes/no + sources>
- Loop recipe: <none / performance-budget / docs-parity / architecture-satisfaction / logging-coverage / production-error-sweep / SEO-GEO-visibility / product-evaluation / custom>

## Spec Source

- `specs/<feature-id>/spec.md` or <inline micro-spec>
- Spec validation sources: <repo docs/tests/current behavior/user confirmation>

## Loop Goal Contract

Use this section only if repeated autonomous execution is in scope. Otherwise write `None`.

- Trigger: <manual / checkpoint / PR opened-updated / CI failure / scheduled runner / production-log event / crawl / user-provided issue-comment intake>
- Goal type: <verifiable / judge / hybrid>
- Goal: <exact measurable target or judge rubric summary>
- Termination condition: <pass threshold and allowed exceptions>
- Scope boundary: <files/routes/pages/docs/scenarios/errors in scope; explicit non-goals>
- Verifier: <command/benchmark/crawl/scenario runner/test suite/judge prompt/review route>
- Judge rubric, if any: <criteria + pass threshold + evaluator class R0/R1/R2>
- Budget: <max cycles/checks/pages/scenarios/errors/context/cost>
- Failure handoff: <local ledger / PR comment / GitHub issue / final receipt / ask user>
- Day-zero guard: <why this is not an unscoped greenfield build, or blocked>

## Architecture Touchpoints

- <files, modules, services, data paths>

## Ponytail Dependency Decision

1. Can this be skipped or deleted? <yes/no + reason>
2. Standard library option: <result>
3. Native platform/framework/database option: <result>
4. Existing dependency/helper option: <result>
5. One-line/simple option: <result>
6. Minimum new code needed: <result>

New dependency: <none or justification + removal cost>

## Data/API Changes

- <schema, contract, migration, endpoint, event, none>

## Implementation Strategy

- <smallest root-cause-safe path>
- Structural change needed? <no / yes + why duplication or architecture is root cause>

## Test / Verification Strategy

- Small check: `<command>`
- Broad check: `<command>`
- Manual check, if needed: <steps>
- Acceptance criteria coverage plan: <AC IDs mapped to checks>

## Command Result Handling

- Expected command classes: <PASS/TEST_FAIL/etc>
- Environment/auth/network risks: <none or mitigation>
- Substitute checks allowed: <none or criteria>

## Loop Budgets

- Max implementation iterations per task: <default 3 or override>
- Max broad verification reruns: <default 2 or override>
- Max reviewer findings per pass: <default 5 except P0/P1>
- Max issue polls per active run: <default 3 or override>
- Max active fix branches per fixer: <default 1>
- Max fix cycles per issue: <default 3>
- Max autonomous loop cycles: <default 3 interactive or configured>
- Max product-evaluation scenarios: <default 3-5 or configured>
- Max production-error clusters: <default 1 or configured>
- Context/tool/cost budget: <explicit limits or summarization/stop rule>

## Review / Issue / External Intake Policy

- External sources to check: <CodeRabbit/Greptile/Jules/Gemini/user issues/PR comments/CI/local pasted output/none>
- Known reviewer/app patterns: <coderabbit/greptile/jules/gemini/gemini-cli/other or repo-specific>
- Authority rule: <A0-A4 handling, maintainer labels, assignment, unknown public users>
- Actionability rule: <ACTIONABLE_FIX/NEEDS_REPRODUCTION/etc>
- Prompt-injection guard: <how raw comments are treated as data>
- PR comment policy: <when used>
- Local ledger path: <none / specs/<feature-id>/agent-issues.md / .agent/review-queue.json>
- GitHub repo: <owner/repo or unavailable>
- Repo visibility: <public/private/unknown>
- Issue access method: <gh / connector / API / unavailable>
- GitHub issue escalation criteria: <durable/cross-session/user-requested/repo queue>
- Public issue safety gate: <redaction and fallback rule>
- Labels: <existing labels or proposed labels>
- Priority mapping: <P0/P1/P2/P3 adjustments>
- Poll checkpoints: <start/final/reviewer-created/failing-CI/automation runner>
- Close policy: <reviewer closes / single-agent-verified fallback / repo policy>

## Branch / Worktree / Merge Policy

- Base branch for feature work: <branch>
- Base branch for issue fixes: <active feature / PR head / default branch>
- Worktree required: <yes/no + reason>
- File ownership rule: <how overlaps are blocked/coordinated>
- Merge policy: <manual / reviewer-approved / user-approved / repo policy>

## Risks and Rollback

- Risk: <risk>
- Mitigation: <mitigation>
- Rollback: <rollback path>

## Open Questions / Decisions

- [NEEDS CLARIFICATION: <question + blocking/deferred impact>]

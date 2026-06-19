# Tasks: <feature name>

Mode: <0-5>
Review class target: <R0/R1/R2>
Loop recipe: <none / performance-budget / docs-parity / architecture-satisfaction / logging-coverage / production-error-sweep / SEO-GEO-visibility / product-evaluation / custom>

## Task Format

- [ ] T### <concrete objective>
  - Area: <files/modules>
  - Minimal rule: <delete/reuse/stdlib/native/existing/simple/minimum new code/structural root-cause fix>
  - Done when: <checkable condition>
  - Acceptance criteria: <AC IDs>
  - Verify: `<command>` or <manual check>
  - Finding route: <inline-fix/pr-comment/local-ledger/github-issue>
  - Loop trigger: <none/manual/CI/schedule/log/crawl/scenario>
  - Loop goal: <none / loop-goal.md section / measurable target / judge rubric>
  - Reviewer trigger: <none/after task/before convergence/after CI failure>
  - Rollback: <specific note if risky>
  - Parallel safe: <yes/no + file ownership reason>

## Loop Task Notes

Use only when tasks are part of an autonomous loop. Otherwise write `None`.

- Active loop contract: <none or specs/<feature-id>/loop-goal.md>
- Loop stop rule: <exact termination condition>
- Budget remaining at task start: <cycles/checks/scenarios/pages/errors/context/cost>
- State reset rule, if scenarios or benchmarks run: <none or steps>

## Tasks

- [ ] T001 <first concrete vertical slice>
- [ ] T002 <second concrete slice, or remove>
- [ ] T003 <verification/convergence slice, or remove>

## Blocked / Deferred

- <task id or issue id> — <reason + needed decision>

## Artifact Lint Notes

Before convergence, replace generic placeholder tasks with concrete tasks. Do not leave `T00x`, `<...>`, or `n/a` in completed artifacts.

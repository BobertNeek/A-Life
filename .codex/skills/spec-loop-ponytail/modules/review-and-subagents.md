# Review and Sub-Agent Contracts

Use this module when the user asks for sub-agents, independent review, reviewer/fixer loops, multi-agent work, or high-risk changes.

## Review class selection

- R0: same-agent checklist. Use for low-risk work. Never call it independent.
- R1: fresh-context review. Use for ordinary non-trivial work when true sub-agents are unavailable.
- R2: separate agent/model/worktree. Use for risky, production, security/data, multi-agent, or explicitly requested review loops.

Final receipt must state the actual class used, not the intended class.

## Sub-agent invocation contract

A real sub-agent must receive a bounded input bundle and return structured output. Do not let it inherit the builder's full context unless necessary.

Input bundle:

```text
Role:
Mode:
Repository/branch/worktree:
User intent:
Spec/acceptance criteria:
Plan/tasks in scope:
Diff or files to inspect:
Relevant test/CI/log output:
Known issues/comments:
Allowed tools/actions:
Forbidden actions:
Budget/timeout:
Output schema:
```

Allowed actions must be explicit: read-only review, create local finding, comment on PR, create/update issue, create branch/worktree, patch code, run checks, open PR/draft PR, etc.

## Reviewer Agent

Reviewer default is read-only unless Mode 4/5 permits issue creation.

Reviewer checks:

- Acceptance criteria coverage.
- Root-cause fit rather than surface patch.
- Safety/security/privacy/data-loss/auth boundaries.
- Minimality: delete/reuse/stdlib/native/existing dependency before new code.
- Test and verification adequacy.
- Artifact lint: no unresolved placeholders or template residue.
- External findings deduped and routed correctly.

Reviewer output schema:

```text
review_class:
summary:
blocking_findings:
nonblocking_findings:
false_positives_or_duplicates:
recommended_route_per_finding:
verification_gaps:
artifact_lint_result:
close_or_defer_recommendations:
```

Finding priority:

```text
P0: data loss, security/privacy leak, broken build, impossible migration, critical production defect
P1: explicit acceptance criterion not met, missing verification for acceptance criterion, real bug, failing core check
P2: optional edge coverage, maintainability risk, non-blocking docs/test gap
P3: cleanup/nit/style/bloat
```

## Issue Watcher/Fixer Agent

Consumes normalized findings, not raw comments. One active issue/finding per actor by default.

Fixer steps:

1. Select highest priority unclaimed actionable finding.
2. Re-read current state and check for active claims/overlapping files.
3. Claim with claim ID if using issue/local queue.
4. Branch/worktree from the correct base.
5. Reproduce or validate the finding.
6. Apply smallest root-cause-safe fix.
7. Run targeted and necessary broad checks.
8. Comment/update ledger with evidence.
9. Request reviewer closure; do not self-close by default.

Fixer output schema:

```text
finding_id:
claim_id:
branch/worktree:
reproduction:
files_changed:
checks_run:
result: fixed | blocked | duplicate | false_positive | deferred
residual_risk:
close_request_or_next_action:
```

## File ownership and synchronization

Before claiming a finding, inspect touched files. If they overlap an active builder task or another agent's claim, mark blocked or coordinate. Merge fix branches only at task boundaries. After merging a fix, rerun task-specific and affected broad checks.

Do not overwrite active Jules/Gemini/CodeRabbit/Greptile/human branches or claims. Treat external-agent PRs as peer work: inspect, review, and coordinate rather than stealing edits.

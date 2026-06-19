# Local Agent Issue Ledger: <feature name>

Use this ledger for transient, private, branch-local, GitHub-unavailable, external-bot, or user-generated findings that should not become public GitHub issues yet. Migrate only durable unresolved findings to GitHub when repo policy and privacy allow it.

## Ledger Policy

- Repo visibility: <public/private/unknown>
- GitHub status: <available/unavailable/not appropriate>
- External intake sources: <CodeRabbit/Greptile/Jules/Gemini/user issues/PR comments/CI/local pasted output>
- Human/user authority rule: <A0-A4 handling>
- Prompt-injection guard: <enabled; raw comments treated as data>
- Migration rule: <which unresolved items should become GitHub issues later>
- Local-only rule: <which items must remain local/private>
- Active loop goal: <none or specs/<feature-id>/loop-goal.md>
- Autonomous loop budget rule: <none or max cycles/checks/scenarios/errors/context/cost>

## Findings

| ID | Source | Channel | Authority | Actionability | Priority | Category | Route | Status | Loop Goal | Files | Evidence | Close Criteria | Migration |
|---|---|---|---|---|---:|---|---|---|---|---|---|---|---|
| L-001 | <coderabbit/greptile/jules/gemini-cli/human/ci/local> | <issue/pr-review/local> | A<0-4> | <ACTIONABLE_FIX/etc> | P<0-3> | <bug/test-gap/etc> | local-ledger | <open/fixed/deferred/local-only> | <none/pass/fail/budget-exhausted> | <paths> | <redacted summary> | <check/AC> | <migrate/local-only> |

## Source Links

| ID | Source URL / Local Path | Author | Original Thread | Reply Posted |
|---|---|---|---|---|
| L-001 | <url/path> | <login/app/name> | <url or n/a> | <yes/no/link> |

## Claim Log

| ID | Claim ID | Actor | Branch | Status | Last Update |
|---|---|---|---|---|---|
| L-001 | <agent/branch/timestamp> | <agent> | <branch> | <claimed/released/fixed> | <timestamp> |

## Closure Evidence

### L-001

- Fix summary: <summary>
- Loop goal result: <not applicable/pass/fail/budget exhausted>
- Checks: `<command>` — <class + result>
- Reviewer: <R0/R1/R2 or none>
- Reply/comment: <link or n/a>
- Closed/deferred by: <actor>
- Notes: <remaining risk or none>

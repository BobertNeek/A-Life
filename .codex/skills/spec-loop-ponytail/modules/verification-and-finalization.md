# Verification and Finalization

Use this module before final response or convergence.

## Verification rubric

A change is complete only when one of these is true:

1. Required checks pass.
2. A narrower substitute check passes and the reason broad checks were unavailable is documented.
3. The task is honestly blocked with evidence and no claim of completion.

Acceptance criteria:

- Every explicit acceptance criterion must be verified or marked blocked.
- Missing verification for an explicit acceptance criterion is P1, not P2.
- Optional edge coverage may be P2 if not tied to acceptance criteria.

## Artifact lint

Fail convergence if generated artifacts contain unresolved template residue:

```text
<...>
[NEEDS CLARIFICATION: ...] unless explicitly deferred with owner/reason
T00x generic placeholder tasks
TODO placeholders with no owner/reason
"n/a" where real command/risk/rollback/evidence is required
copy-pasted prompt text that was not filled for this repo
```

## Ponytail review gate

Before finalizing, ask:

```text
Can this code be deleted instead?
Can an existing path/config do this?
Can stdlib/native platform do this?
Can an existing dependency do this?
Is new code the minimum root-cause-safe change?
Did we preserve safety, validation, auth, accessibility, migrations, rollback, auditability, and required behavior?
```

## Review finding closure

Blocking findings: P0/P1 must be fixed, downgraded with evidence, deferred by explicit user/repo decision, or reported as blockers.

Non-blocking findings: P2/P3 may be deferred if recorded with reason and next action.

Fixer does not close its own durable GitHub issue by default; request reviewer close with evidence unless policy allows self-close.

## Final receipt format

Use this shape:

```text
Mode: <0-5 and reason>
Review class: <R0/R1/R2 actually used>
Assumptions: <validated assumptions and unresolved decisions>
Files changed: <list>
Checks run: <commands/results or why unavailable>
Issues/comments handled: <external/GitHub/local findings, routes, results>
Deferred/blockers: <none or explicit>
Risk/rollback: <main residual risk and rollback path>
Next recommended action: <one concrete next action, if any>
```

Do not claim background monitoring, scheduled future work, PR creation, merge, deployment, or notifications unless those actions actually occurred through approved tooling.

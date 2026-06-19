# Feature Spec: <feature name>

Status: <draft / validated / deferred>
Mode: <0-5>
Review target: <R0/R1/R2>
Loop recipe: <none / performance-budget / docs-parity / architecture-satisfaction / logging-coverage / production-error-sweep / SEO-GEO-visibility / product-evaluation / custom>

## Problem

<What user or system problem are we solving?>

## User Value

<Why this matters now.>

## Evidence and Validation Sources

- User request: <summary>
- Repo behavior/docs/tests checked: <paths or commands>
- Spec mismatches found: <none or decision id>

## Scope

### In scope

- <behavior>

### Non-goals

- <explicitly not doing>

## Functional Requirements

- FR-001: <testable requirement>
- FR-002: [NEEDS CLARIFICATION: <specific question, blocking/deferred impact>]

## Non-Functional Requirements

- NFR-001: <performance, security, accessibility, compatibility, data integrity, etc.>

## Autonomous Loop Objective

Use this section only when repeated autonomous execution is in scope. Otherwise write `None`.

- Trigger: <manual / PR event / CI failure / scheduled runner / log event / crawl / user-provided findings / none>
- Goal type: <verifiable / judge / hybrid / none>
- Goal: <measurable target or rubric summary>
- Termination condition: <exact stop condition>
- Scope boundary: <routes/pages/files/docs/scenarios/errors in scope and non-goals>
- Budget: <max cycles/checks/scenarios/pages/errors/context/cost>
- Failure handoff: <what to record if budget is exhausted>

## Edge Cases and Failure States

- <case>

## Acceptance Criteria

- AC-001: Given <state>, when <action>, then <observable result>.
- AC-002: <checkable condition>

## Assumptions

- A-001: <small reversible assumption, why safe, how to revisit>

## Decision Log

| ID | Decision | Evidence | Impact | Status |
|---|---|---|---|---|
| D-001 | <decision or spec-mismatch> | <source> | <what it changes> | <open/resolved/deferred> |

## Artifact Lint Notes

Before convergence, remove unresolved placeholders such as `<...>` and resolve or explicitly defer `[NEEDS CLARIFICATION: ...]` markers with impact.

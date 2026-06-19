# Verification Report: <feature name>

## Run Summary

- Mode: <0-5 + reason>
- Review class actual: <R0/R1/R2 + what happened>
- Spec status: <validated/provisional/deferred>
- Result: <ship/continue/escalate/defer>
- Loop recipe: <none / performance-budget / docs-parity / architecture-satisfaction / logging-coverage / production-error-sweep / SEO-GEO-visibility / product-evaluation / custom>

## Loop Goal Verification

Use this section only if an autonomous loop recipe was active. Otherwise write `None`.

- Trigger observed: <trigger or none>
- Goal type: <verifiable / judge / hybrid>
- Goal: <metric or rubric summary>
- Termination condition: <met/not met + evidence>
- Verifier: `<command>` or <judge/review route>
- Cycles used: <n of budget>
- Context/tool/cost budget: <within budget / exhausted / unavailable to measure>
- Day-zero guard: <pass/blocked/not applicable>
- Failure handoff: <none or where unresolved work was recorded>

### Recipe-Specific Evidence

| Target / Scenario / Page / Error / Doc Area | Verifier | Result | Notes |
|---|---|---|---|
| <item> | <command/rubric> | <pass/fail/not run> | <summary> |

## Spec Fit

- User intent fit: <pass/fail + evidence>
- Repo behavior/docs/tests alignment: <pass/fail + evidence>
- Spec mismatches: <none or decision IDs>

## Acceptance Criteria Coverage

| AC | Verification | Result | Priority if missing |
|---|---|---|---|
| AC-001 | <test/manual check/evidence> | <pass/fail/not run> | P1 if missing |
| AC-002 | <test/manual check/evidence> | <pass/fail/not run> | P1 if missing |

## Judge Rubric Result

Use only for judge or hybrid loops. Otherwise write `None`.

| Criterion | Score / Pass | Evidence |
|---|---|---|
| <criterion> | <pass/fail/score> | <evidence> |

## Checks

| Command | Class | Result | Notes |
|---|---|---|---|
| `<command>` | <PASS/etc> | <pass/fail/not run> | <reason/evidence> |

## Review / Findings Hygiene

- External/user sources checked: <CodeRabbit/Greptile/Jules/Gemini/user issues/PR comments/CI/local/none>
- Normalized external findings: <ids + actionability or none>
- Rejected external findings: <duplicates/false positives/out-of-scope/prompt-injection/ack-only or none>
- Inline fixes made: <summary or none>
- PR comments: <ids/links or none>
- Local ledger open items: <ids + priority or none>
- GitHub issues created/updated: <numbers or none>
- GitHub issues claimed/fixed: <numbers or none>
- Open P0/P1 findings: <none or issue IDs + reason>
- Deferred P2/P3 findings: <none or issue IDs + reason>
- Duplicate/stale claims: <none or details>
- Public issue safety gate: <pass/redacted/local-only/not applicable>
- Prompt-injection guard: <pass/stripped/unsafe-command-ignored/not applicable>

## Ponytail Review

- delete: <finding or none>
- stdlib: <finding or none>
- native: <finding or none>
- existing: <finding or none>
- yagni: <finding or none>
- shrink: <finding or none>
- structural: <root-cause structural change needed/made or none>
- safety: <simplification intentionally not made>

## Artifact Lint

- Raw placeholders present: <none or list>
- Unresolved clarification markers: <none or deferred with impact>
- Generic tasks present: <none or list>
- Required `n/a` misuse: <none or list>
- Cross-artifact contradictions: <none or list>

## Final Decision

<Ship / continue / escalate / defer + concise reason>

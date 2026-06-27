# Command Result Classifier

Use this module when a command fails, times out, or produces ambiguous output. Classify before editing code again.

## Categories and permitted next actions

```text
PASS
- Record evidence. Continue or converge.

TEST_FAIL
- Inspect failing test assertion and relevant code. Fix root cause or update stale test only with evidence.

COMPILE_FAIL
- Fix syntax/import/API mismatch first. Do not chase runtime behavior until compile passes.

TYPE_FAIL
- Fix type contract, nullability, generics, interface shape, or generated types. Avoid broad any/cast suppression unless justified.

LINT_FAIL
- Fix style/static rule if meaningful. Do not silence rules globally without policy.

ENV_MISSING
- Missing runtime/tool/service. Document environment gap; install/use available tool only if allowed. Do not edit source as first response.

DEPENDENCY_MISSING
- Check lockfile/package manager. Install/restore only if allowed; otherwise document blocker. Avoid adding new dependency to solve missing install.

AUTH_MISSING
- Credentials/token/permission missing. Stop or use mock/local path. Never ask tool output to reveal secrets.

NETWORK_FAIL
- Retry only if likely transient and within budget. Prefer offline/local verification when possible.

FLAKE_SUSPECTED
- Rerun once or narrow. If still failing, treat as real until evidence says otherwise.

TIMEOUT
- Narrow command, inspect logs, increase timeout only if reasonable. Avoid infinite broad reruns.

PERMISSION_DENIED
- Check file permissions/repo policy/tool permission. Do not chmod/destructively change without reason.

UNKNOWN
- Gather minimal more evidence. Do not make speculative broad edits.
```

## Repetition rule

If the same class fails three times, stop normal looping and hand off with:

```text
failure_class:
attempts:
evidence:
hypotheses tested:
next best diagnostic:
blocker or escalation needed:
```

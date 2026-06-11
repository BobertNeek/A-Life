# P30 - Offline log tools, clustering, replay, visualization

Group: Group 4 - Tools parallel

Branch: `codex/P30-offline-log-tools`

Prerequisites: P11

Concurrency: Yes. Can run after P11 independently of adapters/GPU.

Next plan(s): P31, P32, P33, P34

## Purpose

Make exported experience data useful. Offline tools should read logs, summarize behavior, and support research without becoming runtime dependencies.

## Owned scope

- `alife_tools` and/or `tools/` Python/Rust utilities, log readers, docs.

## Required implementation steps

1. Implement a reader for `PackedExperienceFrame` and side buffers. It must enforce schema version and reject incompatible logs with clear errors.
2. Implement export/import utilities for scenario traces and benchmark logs. Keep runtime dependencies optional; tools should not be required for gameplay.
3. Implement basic behavior clustering over packed logs: K-Means or DBSCAN if dependencies are acceptable, or a simple deterministic clustering baseline if not.
4. Implement replay/summary tool that prints creature trajectory, drive/hormone trends, action distribution, reward/pain, memory/topology summary IDs, and scenario markers.
5. Add optional visualization outputs: CSV/JSON/Markdown first; plots only if dependency policy allows.
6. Add docs and sample commands using P18/P19 fixture logs.
7. Ensure tools can run headless in CI on small sample logs.
8. Update traceability for offline tools.

## Required tests and validation

- Tests for schema rejection, reading sample logs, side-buffer linkage, deterministic clustering on small fixture, replay summary output, and no runtime dependency on tools.
- Tool command smoke tests in CI if feasible.

## Acceptance criteria

- Packed logs are inspectable outside the runtime.
- Offline analysis does not leak back into real-time dependencies.
- Future ETF/D2NWG/evolution tools have stable data input.

## Failure handling

- If Python environment is uncertain, implement Rust CLI first and leave Python notebooks optional.
- If clustering dependencies are heavy, provide a simple baseline and extension hook.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P30 - Offline log tools, clustering, replay, visualization
Branch: codex/P30-offline-log-tools
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P31, P32, P33, P34
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.

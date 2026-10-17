# Attention switch-cost repair — 2026-10-17

Base: main `74e4ec59d0a5aaaa33619a1777994666041ca0e2`.

## Finding and repair

`select_focal_targets` built raw top-k and required its previous identity to
already be in that set before applying the existing switch cost. At capacity
one, even a tiny salience lead therefore displaced the incumbent. The same
problem occurred at larger top-k boundaries.

The repair removes that membership condition. An incumbent still present in
current peripheral summaries retains the first slot until a challenger exceeds
its salience plus the existing switch cost. Remaining slots retain ordinary
salience order and the original capacity bound. Missing identities are not
reintroduced; a decisive challenger switches immediately. No scores, thresholds,
schema, acquired state, or action-selection authority change.

This repairs the existing mechanism toward **AOA-ATT-005**. It does not establish
all attention capabilities or learned gameplay competence.

## Action arbitration boundary

Current world candidates have one-tick durations. GPU WGSL scores and selects
current candidates and motor channels each waking decision. Recurrent neural
state, attention, and physical possession carry context across decisions; the
last selected motor bundle is retained for inspection/checkpoint continuity,
not as a host action override.

`MotorCommitment` and `ExecutiveSustain` emissions use current decision
confidence; those names do not implement a retained motor command.

Selected Eat on safe reachable loose food acquires it in that decision; a later
selected Eat consumes held food. Repeating Grab on the same held object releases
it. This repair adds no motor latch, automatic continuation, action chooser,
urgent-action special case, or new behavior gate. World reach, ownership,
effector, consumed-state and liveness checks remain authoritative.

## Verification

- The seven new attention regressions on untouched base source: **4 passed,
  3 failed**, reproducing single-slot, larger top-k boundary, and exact-cost
  retention failures
- The same seven regressions with the repair: **7 passed**
- Full `alife_core` CPU suite and final lint/check results are recorded in the PR
- Read-only independent review found no blocking issue

Tests run with isolated official Rust 1.99.1 on the cloud CPU. No personal
computer, GPU dispatch, training, checkpoint rewrite, or actor/value change was
performed. Behavioral improvement remains **Unknown**; these are deterministic
attention contract regressions.

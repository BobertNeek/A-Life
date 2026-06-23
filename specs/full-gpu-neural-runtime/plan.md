# Full GPU Neural Runtime Plan

1. Add backend compact action-summary support on top of the existing P25 static forward passes.
2. Add backend full-runtime report types and a tiny deterministic live fixture mapper.
3. Add an optional `alife_game_app` bridge that converts live sensory salience into GPU input, dispatches static scoring when available, CPU-shadow checks the compact summary, and feeds proposals through existing live tick arbitration.
4. Keep GPU plasticity post-seal and diagnostic/shadow-only, reporting the current architectural gap instead of touching `alife_core`.
5. Add CLI command `full-gpu-runtime-smoke`.
6. Add tests for fallback, compact readback, routing counters, H_shadow-only plasticity, no bulk readback, and command/report wording.
7. Add productization report and gap report describing the honest current state.
8. Run focused tests, full validation, same-agent review if no independent reviewer is available, then commit/push.


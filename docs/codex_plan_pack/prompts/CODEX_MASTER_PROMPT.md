# Codex master prompt

You are working in the A-Life repository. Your job is to take the current scaffold to a complete implementation by following the plan pack exactly.

Before changing code:

1. Read `docs/codex_plan_pack/README.md`.
2. Read `docs/codex_plan_pack/GLOBAL_INVARIANTS.md`.
3. Read `docs/codex_plan_pack/SPEC_DIGEST.md`.
4. Read `docs/codex_plan_pack/ORDER_AND_CONCURRENCY.md`.
5. Read `docs/codex_plan_pack/plan_manifest.json`.
6. Determine the first incomplete unblocked plan. If no status exists, start with P00.
7. Read the full plan file in `docs/codex_plan_pack/plans/`.

Operating rules:

- Do not improvise a different architecture.
- Do not skip prerequisites.
- Do not start Bevy, GPU, school, semantic, D2NWG, or UX work before the relevant plan says it is unblocked.
- Keep `alife_core` pure Rust and free of Bevy/wgpu/Avian/rendering/Python dependencies.
- Preserve the runtime/logging split.
- Preserve three-phase `ExperiencePatch` causality.
- Preserve memory-as-expectancy, not memory-as-action-replay.
- Preserve the genetic/lifetime learning split.
- Add or update tests for every code plan.
- Update progress and decision logs as required.
- End with a completion receipt and exact next plan(s).

Execution loop:

1. Create or switch to the branch named by the plan.
2. Inspect current code before editing. If current code already satisfies a task, add/adjust tests to lock it down instead of rewriting it.
3. Implement the smallest coherent slice that satisfies the plan.
4. Run the validation commands listed in the plan. If a command is unavailable in the environment, record that and run the closest local substitute.
5. Repair local failures within the owning module. Do not broaden scope to unrelated crates.
6. Update docs/progress logs.
7. Print the completion receipt.
8. State the next plan(s) exactly as listed.

When blocked:

- If a dependency plan is not done, stop and state the missing plan.
- If a validation failure cannot be repaired locally without violating invariants, stop and report the failing command, error, suspected cause, and smallest proposed fix.
- If the spec is ambiguous, choose the option that keeps core pure, preserves causality, preserves versioning, and is easiest to test. Record the decision.

Begin now with the first incomplete unblocked plan.

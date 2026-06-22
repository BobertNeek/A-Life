# S-Phase Execution Order

## Sequential order

S01 -> S02 -> S03 -> S04 -> S05 -> S06 -> S07 -> S08 -> S09 -> S10 -> S11 -> stop.

No plan may run before its predecessor is merged to `main` and validated.

## Review policy

Each S-plan includes its own review. Goal Mode may continue to the next S-plan only if:

1. implementation is complete,
2. plan self-review passes,
3. standard validation passes on branch,
4. branch is merged to `main`,
5. standard validation passes on `main`,
6. the plan receipt says next plan may proceed.

Stop immediately on:

- validation failure that cannot be fixed locally,
- `FIX_REQUIRED` or `BLOCKER`,
- user/product decision needed,
- graphics/GPU hardware evidence ambiguity that affects release claims,
- architecture change that could affect `alife_core`,
- temptation to create G25/P37/new chain.

## Segment checkpoints

The goal prompt may proceed through S01-S11 in one run, but it must stop after S11.

Recommended human review after S03, S06, S08, and S11 if the user wants tighter control. These are not manifest plans unless the user asks to add them.

## Concurrency

Do not run S-plans concurrently. Product evidence is stateful and depends on previous UX/graphics results.

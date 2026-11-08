# Awake shared translation interval: source-bound proposal

Base: sleeping-thermal-combined-v1 commit f83be325d47b435c9e8889d70ea64496f2b382b4, tree da127f8d53fd615f3df18e01a5cbff8c2232738c. PR309 is now merged as accepted main c775c8c92e8f5e625f8a3e861ef79200199b54df with exactly that tree; its thermal parent is dd6501f801d95a0a86789d762f15c6930ded27fd/tree449decc0. No production edits or runtime allocation in this document.

## Actual seams
- alife_world/headless.rs: MOVE_STEP is .1 at20Hz (2units/s). apply_registered_motor_bundle_inner adapts capacity/embodiment, executes Locomotion then Orientation/Manipulation/Vocal/Posture, folds events, samples initial/final contacts and thermal max, advances biology once, aggregates per-channel outcomes and commits interval velocity.
- execute_move computes a bounded straight/locally steered endpoint, applies terrain and obstacle/carry legality, records swept hazard contacts, changes position/yaw/carry, and finishes an outcome. It does not expose a staged trajectory/time budget.
- contact_withdrawal.rs independently permits another MOVE_STEP*capacity, preserves yaw, uses credible external surface normals or acute backward fallback, respects carry/peer/hazard/terrain legality, and charges .02*capacity. Enabling it after ordinary movement would allow two full translation allocations in one interval.
- Ordinary movement's PhysicalActionOutcome cost is .08 while its OutcomeProfile movement energy is -.04; blocked and hazard profiles differ; with_body_event explicitly preserves the profile energy while adding damage. Those are separate existing semantics, not a single path-energy quantity. Do not sum opaque receipt and biological cost fields or silently rewrite all physiology.
- staged_tick selects only sleep/body contacts. Awake selected intent currently has no ordinary separate reflex evidence in its sealed credit. The research V13 path is evidence for a later minimal ordinary extraction, not already installed here.

## Proposed distinct physical factor
Stage a translation trajectory before mutating authoritative world state. One per-organism interval budget is derived once from the actual locomotor capacity and effector gain, with explicit requested/allocated/executed path lengths. No second biology advance, policy selection, optimizer action or hidden target planner.

At painful initial external contact, give the body reflex translation priority; voluntary locomotion gets zero remaining translation if the reflex consumes the budget. Preserve the selected command and its actual zero/curtailed outcome, plus separate reflex receipt. Other effectors still use their normal physical legality; do not force cross-channel target equality.

For newly entered painful external contact, evaluate the actual resolved voluntary segment's first contact time. Allocate its physically traveled prefix, then only the remaining interval distance/time to withdrawal from that contact. Total traveled path, not net displacement, must fit one interval. This requires explicit trajectory staging; a tentative full move plus a full backward step is not a valid implementation.

Unknown-direction acute damage can use the user-approved backward heading basis when translation time remains. If it arises after the interval's translation budget is spent, report budget exhaustion distinctly from collision, not a free extra step or invented source. Any next-interval queued reflex would be a separate bounded serialized state design, not silently added here. Exact-center contacts, conflicting normals and self-carried sources retain established semantics.

Keep quiet/no-contact controls bit-exact. Do not change default energy constants until a separate explicit candidate accounting rule is source-reviewed: actual path work and attempted/blocked effort must be separately represented, and costs applied once. Existing sleeping .02 and voluntary .04/.08 fields cannot be treated as equivalent without a deliberate physical-contract decision.

## Required first fixed fixtures (not allocated)
- Initial contact with voluntary toward/away/tangent translation: reflex priority, total path<=budget, correct selected intent versus actual channel outcome.
- New swept contact at early/mid/late fractions: traveled prefix+withdrawal<=budget, dose measured once, no crossing/teleport through terrain, peer or carried object.
- Rest/Idle/no-op and quiet full movement controls; blocked/zero-capacity/held body; exact-center fallback; opposed multi-source normals; self-carried painful source plus external source.
- Internal acute damage with available versus exhausted translation budget; no chronic-injury scalar trigger, no runaway latch.
- Movement+Rest recovery suppression from any actual path, including zero-net; thermal max across actual contact trajectory/endpoints with final applied event receipt; one biology interval.
- Exact rollback and save/restore around boundary; no double command/credit; real CPU/GPU packer/producer checks only after world evidence shape is frozen.

## Explicit unresolved decisions
1. How to model shared actuator energy/effort without conflating the existing action-outcome cost and body dose or silently changing quiet/sleeping baseline.
2. Whether the first factor covers only external contact with in-interval time, leaving budget-exhausted late internal damage honestly stationary, or introduces a separately versioned one-interval pending response.
3. Minimal ordinary sealed-evidence ABI extraction from tested research V13, without grip genome or interrupted cognition dependencies.

This plan is not proof of execution, general escape, cognitive wake or a completed motor reconciliation lobe. The production-frame motor operating-point experiment and mixed/Completed checkpoint startup work remain separate factors.

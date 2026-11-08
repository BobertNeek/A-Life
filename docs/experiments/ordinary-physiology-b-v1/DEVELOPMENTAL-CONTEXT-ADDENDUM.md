# Developmental context for the eight-tick result

Post-run source and scalar-evidence audit, 8 November 2026. This is an additive interpretation note; the frozen runtime, original report, raw result, seed, horizon and receipts are unchanged. No new native execution occurred.

## What the fatigue numbers actually mean

All 640 recorded transitions have biochemical expression and maturation equal to zero. Development's last-update tick stays zero. Every fatigue transition, including all eleven stationary Rest intervals, exactly equals:

F_next = f32(F_previous × f32(0.99995)).

There are zero mismatches. The stationary Rest intervals occur at ages 6 or 8. Their body.sleeping flag is true and their sleep-state chemical concentration is 1, so the Rest/body pathway works. Their fatigue changes are only -0.00000499933958 or -0.0000324845314, with no additional chemical fatigue relief.

Rest supplies sleep_recovery = 1. Real movement cancels that recovery before authoritative biology advances once. The canonical SleepRecovery emitter has a fatigue coefficient of -0.70, but its expression floor is zero. Chemical release multiplies by the larger of biochemical expression and the emitter's floor. With expression zero, this recovery contribution is zero.

For canonical midpoint founder genes, the approximate one-tick fatigue equation is:

F_next = clamp(0.2e + (F - 0.2e) × 0.99995 - 0.70Re + 0.00007Ae),

where e is biochemical expression, R is sleep recovery, and A indicates the awake body. At e = 0 this reduces to the exact observed decay. The recovery dose is not divided by the founder's 1/400 metabolic turnover.

## Why this is not a demonstrated ordinary newborn defect

Canonical new-game fatigue starts at zero. The closed assay deliberately replaces that concentration with 0.1 or 0.65 while retaining newborn development and inherited hormones.

Canonical waking fatigue production (+0.00007) also has expression floor zero. Thus ordinary waking production and Rest clearance are disabled together during this initial zero-expression interval. At the 60-tick development refresh they activate together.

For midpoint genes, source-derived values are maturation duration 1,824 ticks and biochemical activation gate 0.11. At age 60, expression is approximately 0.299, giving approximately 0.209 fatigue clearance from a full stationary Rest event before clamping. Full expression begins at the age-240 refresh. These are source calculations, not new measured lives.

This audit found no ordinary canonical path in which newborn fatigue naturally accumulates while recovery remains disabled. It does not certify every possible mutated genome. The seeded newborn matrix remains useful for neural response and sampling analysis, but is not a valid demonstration of ordinary fatigue relief or mature self-care.

## Distinctions retained

- The 32 moving-Rest selections still expose an independent motor-coordination problem. The correct world rule cancels their sleep recovery.
- All eleven stationary rests had no remaining object-directed locomotion competitor. They do not establish object-present stationary-rest coordination.
- The 32 moving-Rest outcomes share sampling counter 3 across matched arms; they are correlated common-random-number outcomes, not 32 independent trials.
- Body.sleeping, scheduled host sleep and consolidation are separate. The sleep_maturation_ready flag is not consumed by CPU sleep entry. Here fatigue is below its 0.9 threshold, sleep pressure is zero and ATP remains affordable; this explains the lack of scheduled sleep without treating that readiness flag as a blocking gate.
- The legacy Rest diagnostic advertises fatigue -0.35 and ATP +0.12. These are not authoritative measured physiology. The CPU outcome uses actual before/after biochemistry.
- Sleep-state expression can reduce subsequent sleeping upkeep; repair still spends reserve. Rest supplies no nutrition and no canonical direct ATP replenishment.
- Critical-period plasticity metadata is separate from the biochemical expression multiplier.
- Bare-founder recovery tests are not interchangeable with canonical new-game chemistry.

## Source closure

The relevant chemistry, development and canonical-initialization files are byte-identical at frozen runtime 6dee4ea0, publication-stage cdd33942 and accepted main dd5f6697.

The raw diagnostic SHA-256 was rechecked as 68ffe6229a770a573fc74075afa04dd7d9d267fc680facafa6044fbff1673a20. Only fresh closed-experiment evidence was read.

Relevant repository paths at 6dee4ea0:
- crates/alife_world/src/headless.rs: Rest event near 7966–7989; movement normalization near 4898–4918 and 7766–7774.
- crates/alife_core/src/biochemistry.rs: sleeping assignment near 164–166 and 273; developmental refresh near 936–948.
- crates/alife_core/src/biochemical_graph.rs: SleepRecovery emitter near 1135–1138; expression gating near 1377–1420; receptor-derived homeostasis near 1470–1517 and 1961–1970.
- crates/alife_world/src/new_game.rs: canonical initialization near 225–238 and 385–430.
- crates/alife_training/examples/support/physiology_host.rs: deliberately seeded initial concentrations near 120–193.
- crates/alife_training/src/cpu/host.rs: sleep entry near 742–760 and measured outcome construction near 1173–1187.
- crates/alife_core/src/sleep.rs: controller thresholds near 249–281.

## Next valid controls

Future ordinary-physiology work should include canonical chemistry across ages 59 to 60, paired stationary Rest/Idle at ages 0, 60 and 240 with identical initial fatigue, and moving-Rest controls. Measure actual chemical outcomes, recovery doses, movement costs and exact continuation. A prepared juvenile must not be called an adult.

Changing both waking/recovery expression floors could deliberately make fatigue operate from birth. That is a possible biological design change, not a repair established by this closed matrix. The present response is to fix the experiment's interpretation and developmental controls before changing physiology.

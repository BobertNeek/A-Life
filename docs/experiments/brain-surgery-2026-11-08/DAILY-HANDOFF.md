# Daily brain-surgery handoff: 8 November 2026

## Keep the original research in view

Primary reference: [Sparks of Weight-Space Programming (MIT CSAIL / OASYS)](https://rikiyat.github.io/blog/weight-space-programming/),
checked on 8 November 2026. Keep these three methods distinct:

- **Experiment 4, targeted repair:** inspect a failing trained car policy and
  one episode, then edit a small subset of weights to repair that behavior.
- **Experiment 5, whole-policy construction:** write complete static MinAtar
  game policies from rules/code without practice rollouts or gradients. The
  600→32→32→6 network has 20,486 parameters including biases.
- **Experiment 6, iterative feedback:** construct a larger Go network, inspect
  bounded evaluation feedback, revise weights, and repeat. It is explicitly
  not restricted to one-shot surgery; the paper separates this from training.

These are research precedents, not evidence that A-Life already learns better.
Our target is an embodied recurrent, plastic brain interacting with body state,
episodic recall and changing goals, unlike a fixed game policy with static
parameters during evaluation. A written prior is not acquired learning. Actual
experience-dependent change, retention and useful transfer need their own evidence.
See [implementation](../../ARCHITECTURE.md) and
[organism intent](../../architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md).

## What is established today

The [completed matched world comparison](../world-swap-2026-11-08/completed-matched-23xx-v5/README.md)
and its compressed public data are retained unchanged. Two seeds, one inherited
v12 asset, old/new worlds, 256 primary ticks per case: safe meals 10→14 and 7→13,
harmful meals one in every case. All four cases valid, none unknown or retried;
2,400 preparation advances and 1,024 primary steps total. Final energy rose
.487968→.530196 and .461188→.520480. Old-world held-H/safe-EAT failures were 54
and 68; new-world such failures were zero. Every harmful meal had an exact
Ingest memory episode. More later H acquisitions are not by themselves bad:
H was toxic to eat, not to touch, and held-H time fell 103→46 and 117→53 ticks.

This supports the bundled atomic-replacement/item-grounding world effect on
safe-food access, not learning, population generalization or survival improvement.
No neural defect has been established by these observations.

The [independent reader-v1 archive](reader-v1/README.md) freezes a substantive
negative result. Its strong ingestion-to-grip transfer blocks useful pickup,
interrupts useful transport and obstructs placing safe food in constructed CPU
controls. Reject that fixed reader. The main brain remains unchanged and the
profile-4 candidate remains unpromoted. Do not infer that all memory priors fail.

## Active iteration and overnight/tomorrow continuation

**As of this handoff, v2 is active and pending, not a completed result.** It uses
the same 24 detached contexts with per-gene gains {0, 1/16, 1/4, 1, 4}; no extra
world study, host run or autonomous-life exposure is part of this comparison.
V1 source and results stay frozen. The question is whether amplitude explains
these utility reversals, versus task-insensitive transfer itself.

1. Finish v2, retain the complete gain-by-context results and negative outcomes,
   and distinguish score movement from changed physical fixture choices.
2. Inspect the actual operation-port input range and decoder arithmetic. At gain
   1/16 the eight-gene sum is .5; that alone is not a verified global contribution
   bound until the input range is checked. Beating an artificial task margin of
   1 is not a winner criterion or evidence of autonomous improvement.
3. Use failures to design the next smallest useful intervention: test whether
   current goal/context can distinguish eat, acquire, retain and place without
   erasing manipulation. Avoid blanket aversion or a hidden task oracle. Compare
   against unchanged baselines and retain useful-transport/placement controls.
4. Continue the paper's Experiment-6-style edit/evaluate/revise loop overnight
   and tomorrow. Produce substantive source/results and an updated conclusion,
   not only another plan. Detached evidence may justify a later bounded
   autonomous comparison; this handoff neither allocates nor executes those lives.

## Preservation and next handoff

Keep the accepted baseline and prior experiment bytes immutable. New graph,
interface, schema or inherited coefficients require explicit new identities;
never reinterpret an old identity as a compatible new brain. Preserve current
creatures' acquired weights, recurrent/body/episodic state, eligibility, optimizer
moments and integer ages, checkpoint/save boundaries and recovery copies.
Construct new candidates separately; do not overwrite or migrate existing lives
just to make an experiment pass. Report CPU and GPU evidence separately.

The next daily handoff must retain the original paper link, separate targeted
repair / whole-policy construction / iterative feedback, identify the exact
candidate/source and archive hashes, say what changed and what failed, and state
what remains unproved. Carry forward the main brain's unpromoted status unless
new evidence explicitly changes it. Learning claims require measured acquisition
and retention, not the mere act of writing weights.

Archive documentation base: commit `aa895402f1b2f828b5c90144f27e5264f4309c45`,
tree `54ebe75d9f8c34a190ee5f0d3132f703a7ec5aa5`. Reader-v1 has its own earlier
source anchor and hashes, recorded in its archive rather than conflated with
this documentation commit. This handoff adds no native execution.

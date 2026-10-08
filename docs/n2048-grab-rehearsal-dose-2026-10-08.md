# GrabFood rehearsal dose evidence, October 8, 2026

The 8-epoch trial and paired 16-life PPO versus 32-epoch rehearsal trial executed
the production GPU organism path with the pinned Qwen3.5-0.8B prior. They did
not improve the bounded frozen GrabFood/Eat checks. No candidate was promoted.
The protected trained policy 988 remains the overnight campaign source.

The qualified executable was built from `e8bfdb9c1bb985210539ab2fa6d7f0ed6e1a710a`.
Its SHA256 is `6aa7b3e77d900013637fbb222896e993486bc83fa9b86910d29389936e17438c`.
The starting actor Adam age was 2616 and value Adam age was 1976; objective 2,
founder 539363617, all 32768 weights, and optimizer histories continued.
World/body state was reconstructed per life, rather than retaining one
individual lifetime. The two trials completed 33 native cycle executions across
16 world seeds shared by the paired branches. The first PPO control life was
reused in the 16-life comparison. Manifest record references identify 32
distinct sealed replay sequences: the first 8-epoch and 32-epoch treatment
collections are byte-identical. These counts describe experimental execution
and replay, not 33 different world episodes or additional production dose.

## Observed behavior and replay

Training used temperature 32; frozen evaluations used temperature 1. Each panel
used the same two held-out world seeds, 202610099901 and 202610099902. GrabFood
had 16 decisions per world. EatHeldFood stopped after a real meal or five
decisions.

| Frozen checkpoint panel | Grab acquisitions, two worlds | Held-food meals, two worlds |
| --- | ---: | ---: |
| Protected source, policy 988 | 0 | 2 |
| One PPO-only life | 0 | 0 |
| One life plus 8 rehearsal epochs | 0 | 0 |
| One life plus 32 rehearsal epochs | 0 | 0 |
| Eight PPO-only lives | 0 | 0 |
| Eight rehearsal lives | 0 | 0 |
| Sixteen PPO-only lives | 0 | 0 |
| Sixteen rehearsal lives | 0 | 0 |

Each 16-life branch captured four genuine training acquisitions. The rehearsal
branch performed 128 additional actor updates. Together with the earlier
8-epoch trial, five positive replay windows completed 136 extra actor updates.
Those five windows contain four distinct acquisition loss records; the first
successful record was rehearsed in both dose trials.
Burn-in lengths of zero, one, and five rows executed on GPU. The value
checkpoint was byte-identical across each rehearsal phase. Failed acquisition
lives performed zero extra updates. These are runtime/continuity results;
the behavior gate failed.

The final recorded losses of the four 32-epoch windows remained high:

| Learner life | Acquisition row | First replay loss | Last recorded replay loss |
| --- | ---: | ---: | ---: |
| 1 | 0 | 8.764141 | 7.8755302 |
| 11 | 1 | 11.539467 | 10.0252695 |
| 13 | 5 | 19.614002 | 16.19948 |
| 15 | 1 | 9.54934 | 8.329961 |

These are pre-update likelihood losses for one accepted manipulation event,
coefficient 1 and temperature 1. Even the smallest final recorded loss implies
an event likelihood below 0.0004 in its replayed state. This does not measure
the post-update actor or a new world. It shows that the recorded replay states
were still far from reliably selecting their successful actions.

The L2 difference between the matched actor checkpoints was 0.632599891 after
the first rehearsal life and 2.290319785 after life 16. The difference increased
across each following PPO-only update, rather than disappearing. This weighs
against simple erasure of rehearsal weights; it does not prove the cause of
the failed transfer or eating retention.

## Next bounded dose comparison

The opt-in epoch ceiling is now 512, with default rehearsal still disabled.
No learning loop, action label, optimizer, reward, world legality, or runtime
inference behavior changes. The existing single genuine-acquisition loss row
and bounded same-segment replay remain in use. Every requested epoch is still
separately reported and checked against the actor optimizer step delta.

A larger dose is a hypothesis motivated by the low replay likelihoods, not an
improvement claim. Before a larger dose enters a campaign, execute a private
paired continuation from the same sealed trained source, record actual wall
time and completed offline epochs, verify finite losses and preserved value
state, and repeat frozen GrabFood/Eat checks with the original horizons. Keep
training-world acquisitions separate from held-out behavior. Hold promotion
if grabbing fails or eating retention is lost. The current live overnight
campaign uses its unchanged pinned serial executable.

## Architecture trace and evidence

| Controlling requirement | Evidence and remaining gap |
| --- | --- |
| AOA-FOUND-002 | Labels and gradients are offline; the ordinary runtime receives only the canonical trained asset. No rehearsal runs in the game. |
| AOA-FOUND-004 | Sixteen varied training seeds and two held-out worlds were compared. This small panel does not establish broad transfer. |
| AOA-FOUND-005 | More offline practice is an unverified dose experiment; only 8 and 32 epochs have GPU evidence. |
| AOA-FOUND-007 | Frozen acquisition and short eating retention failed. Founder competence and publication remain unproven. |

The host receipts are under
`D:\A life\target\brain-goal-20261007\qualification-e8bfdb9c`:
`gpu-20261008T053656Z/receipt.json`, `dose32-20261008T054506Z/receipt.json`, and
`qualification.json`. `rehearsal-experiment-accounting.json` records the
manifest-reference duplicate comparison. The first receipt is pinned by the second receipt's
baseline hash. The protected source's four checkpoint hashes still match
after both trials. The qualification provider and native worker exited, and
the shared ownership locks were released before the overnight owner launched.

# Biochemical credit audit — 2026-09-25

Implementation evidence, not a new architecture or founder acceptance result.
Scope: the existing early-mammal graph's 19 active species, world event inputs,
sealed biological consequences, live GPU credit inputs, and founder PPO value.

## Creatures reference and causal rule

The [original C3 brain documentation](https://lisdude.com/cdn/18.html) describes
learning action effects on individual drives separately from general
reinforcement. The [C3/DS Genetics Kit tutorials](https://lisdude.com/cdn/CreaturesGenKitTutorials.pdf)
describe genetic receptors linking chemistry to drives, physical responses,
and organ damage/repair. We use those principles, not their exact chemical IDs
or a claim to have reproduced the whole C3 metabolism.

World code reports physical events. Genetic emitters/reactions change chemistry;
genetic receptors expose its effects. Measured before/after changes supply
biological value. Neural receptor genes project the resulting lanes into local
learning effects. A chemical being high is not automatically an action reward.
This follows AOA-BIO-001/002/010/015/018/024/025/028 and AOA-LEARN-008/009.

## Every active chemical

The reference value weight is per unit of measured change. Every drive and
endocrine channel, including extension lanes, has the same inherited weight
interface; zero means no direct value, not absence of a chemical function.
Existing chemical receptor gains, thresholds, and targets remain inherited.

| Species | Existing function | Default direct learning value |
| --- | --- | --- |
| Hunger | Energy deficit raises it; nutrition lowers it | Reduction +1/7, increase −1/7 |
| Fatigue | Deficit raises it; recovery lowers it | Reduction +1/7, increase −1/7 |
| Fear | Damage raises it; decays toward baseline | Reduction +1/7, increase −1/7 |
| Pain | Damage raises it; recovery/decay lower it; repair receptor | Relief +1/7; fresh harm uses separate injury gain 1 |
| Loneliness | Affiliative contact lowers it | Reduction +1/7, increase −1/7 |
| Curiosity | Current graph has a baseline/drive view | 0; no grounded exploration-satisfaction emitter yet |
| Brain ATP | Nutrient conversion and recovery support neural resources | Increase +1/7, decrease −1/7 |
| Temperature stress | Body thermal stress raises it | Reduction +1/7, increase −1/7 |
| Reproductive drive | Mating opportunity raises arousal/readiness | 0; an opportunity is not successful mating |
| Adrenaline | Damage and neural arousal/surprise; excitability receptor | 0 |
| Cortisol | Damage and thermal stress; aversive plasticity receptor | 0; gates measured negative consequences |
| Dopamine | Appetitive plasticity receptor | 0; gates measured positive consequences |
| Oxytocin | Affiliative contact and neural social state; endocrine view | 0; contact value comes from loneliness relief |
| Serotonin | Consolidation receptor | 0 |
| Acetylcholine | Neural commitment/sustain; attention receptor | 0 |
| Learning signal | Endocrine view; no dedicated learning-rate receptor in reference graph | 0; do not claim active chemical learning-rate control |
| Development signal | Endocrine view/reproductive readiness | 0; age/development expression is separately genetic |
| Sleep pressure | Recovery lowers it; sleep receptor | 0 direct value; fatigue/resource recovery supplies value |
| Nutrient | Material substrate converted to ATP | 0 direct value; avoid counting input and its products as two prizes |

Body energy uses default +1/7 per unit gained. Fresh injury uses the maximum of
pain increase, new tissue damage, and health loss so one injury is not counted
three times and saturated pain cannot hide tissue harm. Its genetic sensitivity
is applied separately from the raw harm measurement. Pain relief is much weaker
than injury onset in the reference profile; injure-then-recover is not a net prize.

Harmless failed actions report a normalized failed-attempt stimulus, without
inventing injury or cortisol. Default inherited disappointment gain is 0.03.
Neural frustration-lane weighting is independently inherited. Neither this
stimulus nor prediction surprise is a hidden correct-action label.

## Repairs and persistence

- Replace the fixed biological value sum with the graph-owned
  `BiologicalValueProfile`. `BiochemicalGraphChromosome::with_value_profile`
  changes either homolog. Existing reproduction selects parental homologs;
  offspring receive no lifetime rewards, weights, or chemical concentrations.
- Snapshot the expressed profile in `BiochemistryState`; reject phenotype
  mismatch or a profile change across one sealed transition. Non-default profiles
  enter existing serialized genotype/biology digests. Missing fields load the
  documented reference defaults; default fields are omitted to preserve legacy
  JSON encodings. Receptor arrays follow the existing channel ABI.
- Gate appetitive/aversive hormone lanes by actual measured consequences.
  Baseline tone alone contributes zero in those lanes. Surprise remains a
  separate, genetically weighted lane; it is not biological benefit.
- Negative social-affinity metadata no longer becomes positive contact through
  `abs()` or manufactures fear/pain. Real damage remains aversive. Learned threat
  associations require the brain's learned context; no identity-threat system
  was added in this repair.
- PPO decisions, skipped sleep intervals, and terminal intervals use the same
  measured frame and inherited action-credit projection as the live action path.
  Individual neural projections may still have different genetic sensitivities;
  PPO uses the actor's action-credit profile, not a mean of every synapse.

The headless world still contains legacy reference drive/hormone descriptions
for diagnostic action fixtures. Registered production biology advances only
from `BodyEventDelta`; sealing replaces those descriptions with measured biology.
They are not chemical-authority inputs or the founder PPO objective.

## Focused evidence and limits

Four valuation tests cover all drive/endocrine gain lanes, genetic inheritance,
save/load, malformed gain rejection, phenotype consistency, tone-only neutrality,
mixed positive/negative consequences, and real biological event proportions.
Existing sealed-credit and harm tests verify raw harm stays inspectable and
unchanged pain is not repeatedly punished. One world test covers harmless
failures and non-threatening negative-affinity contact. The existing hungry/sated
food regression remains the food-relief check.

An N512 founder at tick 600, with each event compared against a neutral advance
to tick 601, produced:

| Event | Biological value minus genetically scaled harm, relative to neutral |
| --- | ---: |
| Damage 0.4 | −0.340000 |
| Affiliative contact 0.2 | +0.017143 |
| Recovery 0.2 | +0.028571 |
| Thermal stress 0.4 | −0.008486 |
| Mating opportunity 0.4 | 0.000000 |

These are one reference genotype and state, before neural projection. They are
not universal scores or learned behavior evidence. Rest has no fixed positive
action bonus; recovery is biological, not a curriculum for prolonged inactivity.
Absence of rest farming across long learned lifetimes is not proven here.

Still incomplete: grounded curiosity satisfaction, reproductive consummation
credit, learned social-threat associations, dedicated learning-signal receptor,
and a full C3-style organ/gut/metabolic repertoire. This audit does not certify
material conservation, 10–30 minute food-free survival, GPU preference changes,
navigation, or player-visible behavior. Those are distinct acceptance work.

The valuation objective changed. Cycle receipts now identify biological objective
version 2. Continuing from an older cycle retains verified actor weights but
automatically starts fresh actor optimizer/value state and records
`objective_state_reset=true`. A newer unknown objective is rejected. A supervised
warm-up's untouched PPO value state remains valid. Old campaign scores and value
estimates are not comparable. The restart branch was compile-checked, not exercised
in a GPU campaign. No new training run or founder promotion was performed.

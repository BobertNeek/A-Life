# Separate test-only retrieval comparison V2: nearby evidence, not policy

Final source ba2092c7f44725e3579d29e3beae75e23331a6de, tree2697c1404e86f7408e953f4fca300a3712d5e6ef. This metric remains test-only and is excluded from the ordinary-main component extraction. The actual V1 store still uses exact context matching.

The frozen helper requires exact organism/profile/action ID/kind/family/tracked target/entity, non-future source age at most1 tick, then applies the existing full family-query weighted cosine and its existing .72 cutoff. No threshold or coefficient was tuned after results. It compares one retained observation, not a population index or pooled reward. Original delta, confidence, source age and similarity remain separate. In particular, confidence is reported but is NOT an admission factor in this helper. Low/zero-confidence measured records are not automatically rejected; a later reader must explicitly weight/gate them. The earlier interface proposal's confidence eligibility is not implemented here.

The three actual next-world queries, which miss under exact V1 lookup, yield:

| Condition | Score | Distance1−score | Source confidence | Original loneliness delta |
|---|---:|---:|---:|---:|
| Peer contact | .9998837 | .00011628866 | .9 | −.007499993 |
| Toy | .99988747 | .00011253357 | .9 | 0 |
| Separated peer | .99987394 | .00012606382 | .75 | 0 |

Wrong entity, wrong family, wrong profile and stale query controls reject. Real measured zero stays known; opposite signed records remain distinct under V1's core contract. No favorable source is selected by outcome sign. This is recovery of nearby same-target evidence, not useful choice or learning.

The first artificial pressure .16 probe accidentally retained tick600, before the source's completed tick601, and correctly returned None via the future-source guard. That result is preserved and does not measure pressure tolerance. R2 fixes only the artificial query clock: scores are .9999958/.9999958/.9999954. R3 adds the parent-requested materially changed loneliness=1 disclosure before measuring it, with no new world condition or action type. Scores remain .9753255/.97527534/.9728827, distances .024674475/.024724662/.027117312, all above .72. **This rule admits substantially different pressure states.** It is not evidence of equal physiology or equal expected relief. The threshold was not changed to force rejection.

Each revision is an ordinary repeat of the same deterministic fixed unit fixture (three prescribed observations and three passive clones), not a new autonomous life or a V4 behavior allocation. All extra pressure/clock queries are detached inputs only, with no new observations, rewards or world intervals. R3 test1/1, strict world lib/tests Clippy and workspace fmt pass. Failed/mis-scoped earlier checks remain in their logs.

Before runtime use: bind store ownership to host world/life/genome/checkpoint lineage; OrganismId alone is not globally unique. Keep raw signed historical consequences separate from current pressure and confidence. A future loneliness reader must not manufacture current relief when loneliness is zero. ATP availability, curiosity/reproduction urges, boredom and reserved lanes cannot receive a uniform deficit sign. Altered target attributes, context aliasing, adverse histories, compact evidence cost, real selection/credit and CPU/GPU parity remain unqualified. No runtime port or store attachment is implemented by this comparison.

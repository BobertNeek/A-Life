# Biological-drive wiring and operating-point study (bounded qualification complete)

Source baseline: recovered `2da9dd323ad6f074556a851174a2edbc9e34e216`, verified source tree `e402bcb5d5fc034ac0244e78d49fa1a0da25ede3` (corrects the earlier erroneous af912 equivalence claim). Separate source copy; no retained creature/checkpoint or optimizer was loaded or modified. Fixed constructor seed 2026110901 and inherited whole-care/v12 genes. These are explicit controlled fixtures, not new autonomous seed allocations or learned behavior.

Original research reference: https://rikiyat.github.io/blog/weight-space-programming/ . This continues targeted repair, whole-network static programming and feedback iterations (experiments 4–6), not a claim that this hand-programmed controller learned.

## Actual drive inventory

Snapshot lanes are hunger0, fatigue1, fear2, pain3, loneliness4, curiosity5, ATP6, temperature stress7, reproductive8, boredom9 in current canonical founders, unused extension10. The encoder prioritizes ports in order0,3,7,1,2,4,5,6,8, then extensions/hormones. Current care constructor correctly looks up `source_index`; earlier cross-talk was already repaired. Its distinct reserve/temporal axes are hunger→Ingest/Approach, fatigue→Rest, fear+pain→Avoid, curiosity→Inspect/Contact.

- Hunger: calibrated energy-deficit emitter .03; nutrition reduces chemical hunger. Encoded path and innate food-attraction path. Valuation −1/7 for increased hunger, energy separately +1/7.
- Fatigue: waking accumulation and sleep-recovery reduction; also limits locomotor capacity and triggers sleep. Encoded Rest axis plus innate Rest gain. Valuation −1/7.
- Fear/pain: acute damage, stress endocrine regulation; pain additionally receives sustained severe-health distress. Encoded Avoid axes and innate danger bias. Pain increase is excluded from drive-change reward because actual injury has a separate penalty; pain relief remains positive.
- Loneliness: social contact relieves it and changes bonding hormones. Encoded but no explicit hand-programmed care axis or innate family bias. Actual peer-touch exists; generic Contact is not equivalent to social care. Valuation −1/7.
- Curiosity: perceived novelty +.04, measured investigation −.08; exploration uses curiosity and suppresses exploration under danger/critical need. Explicit Inspect/Contact axes. Zero direct valuation prevents novelty itself becoming a reward pump.
- ATP: material metabolic supply, measured neural cost, ordinary admission/microstep throttling and forced recovery if work is unaffordable. Encoded but no explicit care axis. Positive valuation +1/7. Do not mistake this resource-availability channel for a deficit channel.
- Temperature stress: actual body stress and cortisol; innate danger/critical-need pathways and valuation −1/7. No explicit care axis. Directional thermal-relief affordances need separate verification; generic avoidance is not proof of successful thermoregulation.
- Reproductive drive: mating opportunity and readiness gates including maturity, health and energy. Encoded but no explicit care axis/innate bias; zero direct reinforcement. Not meaningful to claim reproductive motivation tested in newborns.
- Boredom: canonical founder `with_play_stimulation`, awake +.0002 and actual play −.2, valuation −.08. It is not an unused extension. No explicit whole-care axis. Actual toy play, not pickup alone, supplies stimulation.

Relevant sources: chemistry.rs; biochemical_graph.rs:895–1075; phenotype/io_compile.rs:95,676; phenotype/decoder.rs:82; world/new_game.rs:408–455; world/headless.rs interaction/contact/reproduction handlers; training/cpu/activity.rs and host.rs sleep/admission.

## First result: nominal connectivity hides motor saturation

Detached native CPU replay uses native birth dendrites, three executed microsteps, eight frozen frames, all routes admitted, ordinary intra-frame route cadence, zero innate biases, zero effective offsets and no memory inputs. Physical decoder features are matched synthetic fixtures, not world observations. Each encoded drive is varied independently .1/.5/.9. Checkpoint serialized before and after is identical.

Native gain6 birth branches cover the first64 motor neurons, use self and neighbor weights1, threshold−1, output gain6 (gpu_backend/closed_loop_v11.rs:214–245). At this operating point fatigue .1→.9 changes Rest by only approximately1e−5; hunger barely changes Approach, curiosity barely changes Inspect. Hunger→sweet Ingest remains +.76014; fear or pain→bitter Avoid +.66699. Family-specific decoder references compensate baseline score, but cannot restore a saturated neuron's derivative.

## Failed legal static-offset repairs retained

1. A shared independent tonic→all affected motors bridge does not exist in this sparse graph.
2. Per-motor bridge paths exist, but negative motor-edge weights violate native sign contracts (`PhenotypeCompile`). Validation was not weakened.
3. Moving the negative sign onto a permitted temporal edge then using a positive motor edge is admitted but gives no measurable improvement across offset gains4/6/8/10. Negative temporal input does not produce a usable negative signal here.
4. No saturated inhibitory motor donor exists along current lateral edges for all selected targets. No silent coordinate rewiring was performed.

Raw attempt source copies and logs remain in the isolated study root.

## Controlled birth-dendrite gain interventions

Only detached fixture initial branch output gains are changed for causal diagnosis. This is NOT a valid promoted candidate identity or rewritten saved creature.

Gain6: fatigue→Rest ~.00001; gain4: .00075; gain2: .02726; gain1: .08071; gain.5: .11531; gain0: .15693. Hunger→Approach and curiosity→Inspect similarly recover sensitivity. Ingest/Avoid/Contact are unchanged in these fixtures. Gain.5 retains a nonzero dendritic contribution and is a provisional new-recipe candidate, not established optimal.

At gain.5, selected motor activations for need .1/.5/.9 are approximately .568917/.759063/.799540; native6 selected motor activations are near saturation. Reference activities and old decoder offsets differ by family. Thus simply lowering dendrite gain does not establish calibrated default behavior or sensible competition; world and mixed-drive controls are still required.

Pending: explicit new identity with recipe-bound birth parameters; ordinary biological mixed-drive fixtures, source/serialization/legacy guards, matched physical outcomes and rest/satiation equilibria. No default change, promotion or merge.

## Tonic qualification

The unsensed interoceptive source383 has no inherited neuron bias or incoming recurrent edge. It therefore stays zero from zero initial state in the zero-global-threshold-shift detached fixtures. This explains why those fixture offsets cannot implement an independent constant. It is NOT universally dead in native hosts: biochemical `local_threshold_shift` is subtracted from every neuron input and can activate it. Actual host logs must report both shift and source activation. Earlier no-tonic world effects cannot be dismissed using the zero-shift fixture. A source designated tonic is chemically dependent under the current recipe, rather than a guaranteed constant.

## Completed native-context and physical controls

The explicit new `N2048DriveBalanceCandidateV1` recipe now binds gain0.5, threshold−1 and the full native branch construction rule. Its independent qualification is documented by the integrated identity patch (69 focused tests, including old identities, serialized acquired branches, CPU exact resume and GPU upload/Naga validation; no hardware GPU execution). The old recipe remains unchanged.

Ordinary host effects are interoception0.5, regional excitability0.5, projection gain0.5, global encoded bias−0.025 and local threshold shift+0.25. Source383 becomes approximately−0.159 to−0.395, not zero. With these actual contexts and correct encoding, eight-frame fatigue→Rest sensitivity rises from0.001401 to0.023249, hunger→Approach0.001944 to0.032257, curiosity→Inspect0.000398 to0.006610. These are controlled encoded sensitivities with innate contributions held fixed, not learned policies or proof of selected actions.

Lower gain alone flips some reference motor signs under those real effects. Accordingly a second explicit inherited factor reverses only the Approach/Inspect reference ActionCandidate genes. Rest references, all recurrent genes and every memory readout remain unchanged. This repairs sweet Approach score polarity. The factor is separately recorded from the birth recipe; it does not alter default founders.

Five arms each use the same17 fresh controlled scenarios and eight ordinary CPU policy/world ticks. `world-summary-v1.json` contains exact identities, physical controls, consumption, final displacement and all six selected motor channels. Raw files retain complete perception, logits, receptor effects, native activation, effective plastic offsets and actual physical consequences. Optimizer state is unchanged; ordinary within-life plasticity is enabled. Concentration seeding and fixed scenes are deliberate interventions, not acquired behavior. The bounded scope is136 ticks per arm, not lifetime fitness.

- Baseline, gain-only, reference-recalibrated, soft geometry (taste2.4/distance−16), and distance-only (taste12/distance−80) are all retained.
- Reference-only preserves close and held sweet consumption, avoids bitter+pain consumption where baseline eats, and consumes legal loose maxreach food within eight ticks where baseline does not. This is useful partial evidence, not a universal behavior improvement.
- Both geometry attempts fail the maxreach consumption control. Soft geometry additionally reintroduces bitter+pain consumption. Distance-only preserves bitter avoidance but supplies no extra tired-case Rest. Both are rejected as improvements, despite plausible individual score changes.
- Actual Rest posture selections, not representative action labels, show no reliable tired-versus-alert arbitration repair. Fatigue also directly reduces locomotor capacity, so smaller displacement is not evidence of choosing rest.
- High and low boredom produce identical physical choices in these fixtures. High and low fear both escape the bitter target, where the fixed bitter prior already dominates. No-visible-target fixtures remain still and select Rest once; constant activity is not required.
- All arms eat held/close sweet food. None eats held bitter, loose bitter at maxreach, or blocked-reach food. Physical feasibility probes run on detached scene clones and are never policy inputs.

## Physiology and remaining architecture limits

`physiology-v1.json` is a full-expression chemical upper-envelope fixture, not a lived trajectory: old founder waking fatigue/sleep pressure reaches only0.482879 by expressed lifespan4514, below normal sleep threshold0.6. One full-expression recovery event nearly clears fatigue. This motivated the separate inherited waking/recovery calibration, not a silent change to old chemistry or saved concentration. A separate worker owns its source and28-test qualification. Rest+movement recovery semantics remain a separate world-rule concern.

Loneliness has contact relief and return toward baseline0.15, but no positive Alone emitter. Boredom has accumulation, play relief and valuation but no nonzero programmed route to action in this frozen asset. ATP has actual admission/energy consequences, but lacks an engineered motivational readout. Their encoded silence does not negate these other biological effects.

Temperature stress is unsigned. `BodyState` stores stress rather than a signed core-temperature error. Signed object surface temperature exists (grounded sensing and candidate feature21); nonvisual contact separates positive heat and negative cold (`headless.rs:4123`). This does not establish an integrated signed body/ambient thermal regulation loop, and this study does not demonstrate choosing cooler versus warmer refuge or actual thermal relief. Generic Avoid is insufficient evidence. Peer contact and mating handlers exist, but our isolated newborn fixtures have no accessible social partner or mature reproductive readiness. Social motivation, reproductive behavior and thermoregulation remain unvalidated rather than being declared wired correctly.

The reference factor preserves memory genes exactly; these experiments do not show new learning or validate acquired harm recall in a retained creature. The correct conclusion is a real motor operating-point repair with some bounded physical benefits and unresolved motivational arbitration/affordance gaps. No candidate is promoted, merged, or applied to accepted lives.

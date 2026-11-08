# Failed native learning pilot, frozen result

## Outcome

Native CPU only. Both candidates ate harmful A three times and safe B three times. Original v6 stopped after 42 host ticks (29 decisions); the quarter-action-gain candidate stopped after 57 (44 decisions). The stop was injury above 0.25, with the three-harmful-meal cap also reached. Each completed one explicitly requested recovery sleep and resumed normal action before consuming A twice more. This is a clear negative result for behavioral harmful-food avoidance under this bounded development pilot, not an improvement.

A meals produced native biological credit from -0.08645 to -0.09085, measured aversive harm 0.28, and experienced valence near -0.291. B meals produced positive biological credit near +0.0023. World reward_valence stayed zero. The experiment therefore supplied genuinely adverse experience rather than assuming that a hidden hazard label was bad.

Both arms preserved initial near positive/bitter care selectivity: one near positive meal and no bitter meal over each predeclared 24-tick baseline. Far-food baselines moved 1.1611 m (original) and 1.1127 m (quarter) toward a food initially 2.5 m away. No far consumption within 24 ticks is horizon-censored, not a demonstrated reaching failure. These are competence checks, not independently learned-retention tests.

## What was and was not tested

Each primary life was uninterrupted, fresh, and biologically prepared for 600 ordinary ticks from energy 0.4. Same-object ecological regrowth preserved object identity and experience. The fixed public seed was 2026110901. A and B had equal positive chemistry, nutrition, material and geometry, with stable distinct appearance. The experiment used nutrition 0.05 and A hazard_pain 0.35. It did not use hidden harm cues, force choices, fabricate credit, train Adam, erase private state or teleport the creature.

The prescribed sleep request occurred after the first actual A meal, at world ticks 608 and 619. Consolidation occurred at step indices 16 and 27; normal decisions resumed at 21 and 32. There were 21 and 25 post-sleep decisions. No additional exposure was given after failure.

After the first A meal, original had four A-contact Ingest candidate rows and quarter had eight. Both selected A again. Safe B also remained available and was eaten. There were no simultaneous A/B contact-choice rows after first harm because their regrowth schedules became staggered. This limits paired preference inference, but it does not turn repeated harmful consumption into avoidance. Endpoint inactivity was not used as evidence. No reversal, unrelated acquired retention, GPU deployment or clean whole-life no-learning comparison was tested.

## Mechanistic diagnosis from these fresh records only

The immutable first-row offset-swap assay held perception, recurrent initial state, gates, memory inputs and structural context fixed. Final A-minus-B Ingest margin shifts were -0.000211716 (original) and -0.000016451 (quarter). These are tiny, and most of their direct decoder contribution was direction lane 0. The combined color/shape contribution was positive, not evidence of appearance-specific avoidance. Nonzero banks and episodic storage are not sufficient learning evidence.

ActionCandidate eligibility was active. Original first-A taste eligibility was -0.76734 on the +12 need readout and -0.51184 on the -60 reference readout. Quarter's corresponding values were +0.67609 and -1.0. Color and shape coordinates also had substantial eligibility. Actual biological credit was projected through the named signed-choice receptor profile, with one-tick outcome decay 0.95.

Native effective weight is G + L + alpha F. Alpha was 0.1341599822 for these action readouts. With eta 0.01, the effective Hebbian step on a zero-genetic appearance coordinate is eta * alpha^2 * projected_credit * eligibility * 0.95, about 1.5e-5 at unit eligibility for these meals. Normalization also follows absolute local signed credit. For the original +12 taste coordinate at first harm, the fast Hebbian term was +8.45991e-5 and normalization -6.24176e-6, producing an effective update +1.05124e-5. Its negative source activation means this coefficient increase is not itself an appetitive increase. For the -60 reference coordinate, the effective update was +2.20103e-5, weakening that positive contribution from its negative activation. Independently recomputed update equations agreed within approximately 4e-10 fast-weight units on the watched coordinates.

The banks were nowhere near their clamps. Maximum final absolute fast weight was 0.0045013 and 0.0033554 against a limit of 2; maximum lifetime weight was 0.0013946 and 0.0031230 against a limit of 4. Sleep staged precisely half of each pre-sleep color/taste fast coordinate into lifetime and halved fast. None of those coordinates received direct replay amplification because the bounded capture set selects earlier coordinates. Thus a large limit alone does not imply useful few-shot adaptation.

Episodic recall did work. On first post-sleep A re-encounter, confidence-weighted target fear/pain were approximately +0.028/+0.066, while B's pain was zero. Ingest-family valence/danger were approximately -0.272/+0.272 for A and +0.003/near-zero for B. The v6 constructor had set all genetic MemoryContext readouts to zero. This 32768-synapse phenotype had no CognitiveContext decoder, despite carrying cognitive input lanes. Learned memory readouts reached about -0.08 after sleep and -0.17 later for original, and -0.07/-0.14 for quarter, but they suppressed safe B almost equally. Common novelty, contact, prediction-error and success inputs dominated; this is broad suppression rather than useful target discrimination.

The existing MemoryContext head is bounded to +/-0.5, while original post-harm Ingest led its strongest alternative by roughly 5.1 to 7.6 logits and quarter by roughly 1.94 to 2.35. A readout must have enough influence to affect actual choices; a cap increase alone still supplies no learned information.

Family valence intermittently became benign at pickup. This was not deletion: final banks had zero evictions and retained harmful Ingest records. The latest-compatible-outcome selector can treat a newer successful Ingest preparation (Touch) as a reversal of an older Ingest consumption (Consumed), because both use action210 and exact outcome phase is not stored. Original row23's harmless pickup is followed by positive family expectation on row24 despite retained harm; quarter row55 used Contact211, so its row56 Ingest expectation stayed negative. A separate generic phase-provenance correction is justified. It must not replace this with a valence-strength rule that blocks genuine safe-after-poison reversal.

## Preservation and verification

Source commit: 498d4574118b1fdef7c1fbc332f670d635484681.

Source-only patch: source-frozen-v2.patch, SHA-256 ecea604e40c235fb47bec242f3c035848db28787d2d5f83cd1def4ceb107f5cb.

Frozen executable: frozen/probe_native_learning, SHA-256 b20a3af85f48187b064963a00757b84d802cdb78697c310948df0468c9ea0162.

Original v6 asset: SHA-256 94a70c709661d883d6f146dec773fe120f5f53a8e25949c8c38cf232d0d6b11f, byte-identical to the archived v8 candidate. Quarter asset: SHA-256 608f11b4622436a35f6bee888d19ae12ea6be9cff403ae7006dff3427d40a727.

All eight lives passed exact founder/Adam preservation and exact cold restoration plus the next native step/checkpoint. Three candidate construction tests and three targeted hazard tests passed. Formatting and quick boundary checks passed; full workspace tests were not run. The pilot process exited zero. The working tree was clean at the frozen source; accepted/default assets and PR296 were untouched. No publication occurred.

The authoritative manifest and outcome journals are in pilot-02. journal-review.json is an independent read-only outcome parse. mechanism-review.json and analyze_fresh_mechanisms.py reproduce the arithmetic review. Detailed fresh diagnostic frames, neural banks and checkpoints remain local, outside the repository, and must not be included in source patches or public artifacts. pilot-01 was preflight assets only, with no lives run.

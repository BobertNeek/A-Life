# Rest and locomotion: exact trace and prospective matched motor lobe

Status: published source-only proposal and algebra-test record. No native proposal build, native brain replay, world life, new action selection, or asset/checkpoint edit occurred. The construction sketch produces a manifest, not an admitted brain. The initial six pure Python structural/algebra tests passed. Three mediation-construction tests were added without being run during that refinement; a subsequent verification ran all nine tests successfully. The original six-test log is `algebra-tests.log`; the later nine-test log is `owner-algebra-tests.log`. Publication did not rerun tests. Source, the closed fresh experiment, and the proposed mechanism remain distinct.

This is a publication-derived report. The original prototype, manifests, tests, both test logs, and scalar analysis are byte-for-byte copies. `publication-manifest.json` records original and published report hashes and explains the documentation-only changes.

## Findings first

1. Every one of the 32 moving-Rest rows selects Rest as representative and APPROACH primitive 101 on the independent locomotion channel. None selects FLEE 102. All are tick 4 / sampling counter 3 / tonic ON, across eight object scenes and four bounded×dedicated-B combinations. These are correlated common-random-number outcomes, not 32 independent replications.
2. All eleven stationary Rest rows select NO_LOCOMOTION 103. They have no remaining observed-object locomotion competitors; noLoco therefore has conditional probability 1. The closed run does not demonstrate voluntary stationary rest in the presence of an available object movement choice.
3. The noLoco option is supported and virtually neutral. Its largest absolute logit across the 32 moving rows is 0.0000032306; normal somatic/memory effects mean 'exactly zero' would overstate the recorded total. In 26/32 moving rows noLoco has a higher logit than the selected Approach. This is sampling, not argmax or missing no-op.
4. The new B penalty does not couple Rest to locomotion and its signed neural contrast reverses at startup. A strong Rest logit alone cannot prevent a different channel moving. The correct world normalization cancels sleep recovery, so there are 43 Rest selections, 32 moving/unsuccessful recovery, 11 body.sleeping, 0 scheduled host sleepers, 0 consolidation.
5. A genuine neural arbitration population is justified. A class 5 observed-object gate fixes descriptor-dependent gating but cannot, by itself, fix mismatched neural operating points, independent stochastic channel selection, or create a new motor lobe.

## Evidence and source identity

Closed raw input: the fresh eight-tick CPU assay (`eight-tick-results.json`, not included in this source-only publication), SHA256 `68ffe6229a770a573fc74075afa04dd7d9d267fc680facafa6044fbff1673a20` (rechecked, unchanged). Source execution identity 6dee4ea0; executable SHA256 `5681bc2337705834ac83159d4ec2fbe16ad35a60240577fe4f9bab216c2ffe3d`. The inspected physiology checkout HEAD cdd33942 differs from 6dee4ea0 only by experiment documentation/receipts. `rest-rows-analysis.json` retains the 43 selected rows' scalar analysis, competition probabilities, exact primitive IDs, and case/arm identity. No private retained-life data was inspected.

Production-native CPU selection evidence:
- `crates/alife_training/src/cpu/input.rs:129–145` maps Move to locomotion, Rest/Inspect/Hold to posture, and supplies the deterministic random draw.
- `input.rs:226–245` samples a representative, forces only that representative's slot, then independently samples every remaining nonempty channel from its own mask. Rest therefore forces posture, not locomotion.
- `cpu/host.rs:938–985` uses the actual neural logits at temperature 1, records sampling counter, constructs the factorized bundle, and retains the representative.
- `examples/support/physiology_host.rs:363` gives every trial the same seed-derived sampling seed; each fresh host starts counter 0. Every moving-Rest row is the same counter 3 draw under different logits.
- `alife_world/src/candidate_enumerator.rs:33–69, 350–365` defines APPROACH 101 as familyApproach, FLEE 102 as familyAvoid, and NO_LOCOMOTION 103 as familyApproach/Move with no target. Family alone does not identify a translation command.
- `headless.rs:4898` normalizes the merged body event using each realized displacement; helper at line 7766 clears only sleep_recovery when any segment moved. Do not remove this rule or mask contradictory choices after sampling.

Two complete examples, bounded+B+tonic ON, tick 4:

| Scene | Rest representative total | Rest neural / innate | Approach 101 total | Flee 102 total | noLoco 103 total | Conditional P(noLoco) |
|---|---:|---:|---:|---:|---:|---:|
| food_fatigued | .611144 | .227518 / .383626 | -.180295 | -1.215044 | ~0 | .4691 |
| bitter_fatigued | .611144 | .227518 / .383626 | -.712554 | +1.215657 | ~0 | .2056 |

Both selected Approach despite its lower logit than noLoco. The locomotion mask is 518, with candidates 1/2/9 (Approach/Flee/noLoco); Rest representative index 7. Conditional softmax is exp(logit)/sum exp(logits), T = 1. Readouts alone are not always total logits: memory/cognitive contributions can make small differences. Across 32 moving rows P(Approach) ranges .09173–.47154 and mean P(noLoco)=.32248. In bitter scenes, merely suppressing Approach leaves positively scored Flee, so it cannot deliver stationary recovery.

## Developmental context of the closed assay

The [developmental-context addendum](https://github.com/BobertNeek/A-Life/blob/599ffadd09dd28912c846e675ca73e3d7e05bf06/docs/experiments/ordinary-physiology-b-v1/DEVELOPMENTAL-CONTEXT-ADDENDUM.md) corrects the interpretation of fatigue in these same closed results. All 640 recorded transitions have biochemical expression zero at ages 0–8. Their fatigue changes, including the eleven stationary Rest intervals, are decay only. Canonical waking fatigue production and Rest clearance are both expression-gated during this initial interval.

The assay deliberately seeded fatigue at 0.1 or 0.65, whereas canonical newborn fatigue starts at zero. This is a seeded-newborn context limitation, not a demonstrated ordinary-world fatigue defect. The 32 moving-Rest outcomes remain evidence about independent channel coordination; neither those results nor the eleven stationary Rest intervals prove mature self-care or chemical fatigue relief. The addendum is a source/scalar audit of the existing evidence, not a new native run. Its developmental controls should accompany the future motor-coordination experiments below.

## Why B's sign was not trustworthy

The dedicated source is 1779, reference 1791. The readout addition is `-4 * (a1779-a1791) * B`, where B=(color6–shape14 + contact18)/10. The route into 1779 has gain 4; shared source/relay gains are 2/2. Ordinary physiological modulation is projection gain 0.5, local threshold +0.25, three microsteps; it is not the fixed gain 1/threshold 0 assay.

In bounded+B food_fatigued, the recorded signal-minus-reference is −.076679, −.176253, −.116347, +.113759 at ticks 1–4. Thus the first-tick B addition excites Approach by approximately +.191 even though the decoder coefficient is negative. A nonnegative object basis solves feature polarity only. Equal source/reference dynamics, background, branch state, and reference input meaning must be established separately. Multiplying a signed state by a negative number is not sufficient evidence of inhibition.

Current replay equations are explicit in `alife_training/src/cpu/replay.rs:123–175`: branch contribution is tanh(max(weighted prior activity−threshold, 0))*gain, followed by biochemical projection gain, local threshold, activation function, leak, and metabolic feedback. The current raw motor cells do not form matched pairs. A reference coefficient correction at the final decoder cannot undo all upstream mismatch.

## Concrete source-only construction proposal

`circuit_proposal.py` constructs exact new neuron addresses, paired branch inputs, provisional weights, candidate readouts, and budget counts. `circuit-manifest.json` is its output. `test_circuit_proposal.py` supplies six scalar/structural tests and three mediation-construction tests. This standalone construction specification neither imports nor edits A-Life runtime code and cannot be admitted as an existing asset. All nine tests passed in the later verification recorded above; that result is not native or behavioral validation.

Reserve a new explicit selector, `ProspectiveMatchedMotorArbitrationV1`, with 2112 neurons, retaining 0–2047 exactly. No capacity number or decoder enum discriminant is appropriated. New addresses:
- 2048+2i /2049+2i: drive and declared reference afferents, i=0..10.
- 2070+2i /2071+2i: rectified above-reference drive pairs, i=0..10.
- 2092/2093: ATP deficit pair, reversing only the ATP differential.
- 2094/2095: recovery-state pair R.
- 2096/2097: voluntary-search-state pair V.
- 2098/2099: fear-state pair E.
- 2100–2111 reserved; 52 new active neurons, 64 capacity slots.

All members of each pair start equal. Dynamics: tanh, leak 0.5, bias 0, homeostatic-feedback gain 0, matched receptor/excitability/threshold exposure, identical baseline inputs and branch structure. Retain metabolic-load tracking and charge computational work; disabling its feedback in these new pairs is not free energy. No existing neuron's dynamics changes. Initial mechanism qualification requires explicitly enforced fixed new paired parameters while ordinary learning elsewhere continues. A source manifest, zero alpha, or zero genetic weight does not enforce that freeze. Later learning requires versioned symmetric/tied updates and new lifetime qualification; arbitrary independent somatic drift destroys the cancellation argument. See the lifetime-protection refinement below.

The new reference-afferent contract is essential: drive d and declared setpoint r must pass through the same interoceptive gain, regional gain and offset. Reference input is an explicit versioned constant, not the activity of a supposed silent neuron. This needs a typed encoder extension. Simply adding a constant bias after modulation does NOT meet the contract.

Setpoints in the manifest are provisional starting numbers: H = .35, F = .35, fear = .25, pain = .10, loneliness = .35, curiosity = .35, ATP = .50, thermal stress = .10, readiness = .50, boredom = .35, unbound = .50. They are not measured biological optima. Scan them and individual drive ranges before any behavioral claim.

Each high-drive pair's signal branch receives `+4*afferentSignal -4*afferentReference`, threshold 0, output gain 1. Reference branch has the same number of inputs, all weights 0. ATP-low reverses the two coefficients. For any shared biochemical background b and projection gain g≥0, signal branch output B≥0 and reference branch output 0 imply tanh(b+gB)≥tanh(b). With equal initial activity and identical leaks, the pair difference stays nonnegative by induction. It may be too small or saturated: nonnegative sign is not sufficient useful gain. With varying input, the above-reference signal can lag after onset/washout; latency and hysteresis still require measurement.

Arbitration is a real recurrent dendritic network, not a drive-conditioned host command switch. Every term below is the differential activity of a matched pair; each coefficient expands to opposite signed input weights on the two neuron addresses:
- R receives +1.25F,+1ATPdeficit,+.15chronicPain,−.75H,−1.5E,−.5V.
- V receives +.75H,+.30curiosity,+.20boredom,−1.25R,−.50ATPdeficit,−.75E.
- E receives +1.25fear,−.05R.

Each state has the same rectified branch/reference arrangement. R↔V is reciprocal competition; E strongly suppresses voluntary seeking/recovery, while recovery only weakly suppresses E. These are signed dendritic inputs, permitted by the existing branch primitive. They are not negative MotorProposal edges disguised as accepted topology. The circuit needs at least three microsteps from fresh afferent input to initial arbitration activity, then recurrent settling. With leak 0.5, a three-microstep interval cannot be assumed equilibrated.

Sparse additive readout coefficients (each implemented as +coefficient on signal and −coefficient on reference):
- Rest: +4R−1.5V−4E on intrinsic Rest lane 23, −2thermalstress on observed thermal-contact43.
- Approach: (−4R+1.25V−2E)*object36; +.5H*tastePositive37−.5H*tasteNegative38; −2thermalstress*(hot39+cold40); +.5loneliness*peer41; +.25readiness*peer41.
- Avoid: (−2R+5E)*object36; +2E*tasteNegative38; +3thermalstress*(hot39+cold40).
- Ingest: +4H*tastePositive37−4H*tasteNegative38. No R penalty, so eating in place can coexist with recovery.
- Inspect: (+.75curiosity+.25boredom−.5R−.5E)*object36.
- Other: +.25loneliness*intrinsicVocal42. Contact/Grab gets no invented social bonus; no promised play prediction.

These are starting values, not performance results. Opponent signals are bounded but their useful differences depend on actual biochemistry. Existing learned/raw/memory proposals remain available as a separately logged additive baseline. This first candidate is therefore a supervisory motor sublobe, not a claim that every old pathway has already been rerouted through it. A later experiment can explicitly compare additive supervision with new motor-head mediation of existing proposal populations. Never silently cut learned pathways in retained lives.

## Derived ports and neutrality

The separate class 5 derived-feature proposal keeps raw 0–23, memory 24–35 and cognitive 46–52/grip 53, using derived 36–45: object, positive/negative taste, symmetric hot/cold excess, observed peer match, intrinsic vocal, maximum observed thermal-contact, reserved novelty/play = 0. Its 1280 extra rows initially have zero weights and preserve old neuron addresses. The confirmed exact derived decoder coordinate is 24576+544*f+384+16*(lane−36)+localMotor, but this specification does NOT write those offsets or claim its new neurons fit class 5.

Object 36 is superior to B: it remains 1 for a sensed black/zero-descriptor object and 0 for targetless noLoco. Split taste 37/38 prevents a single signed chemistry feature from reversing hunger/fatigue semantics. Positive/negative taste describes chemistry, not proven nutrition/poison. Both foods in the closed experiment have equal harmless nutrition; no toxin learning was tested.

Every new Approach/Avoid term is gated by actual observed-object/chemistry/thermal/peer ports. NO_LOCOMOTION 103 has all those ports 0, hence its added contribution is exactly 0 for any arbitration activity. Do not apply global constant/family penalties to familyApproach, because that would also suppress the no-op. The six tests verify this algebra; normal old learned contributions may still move its total logit slightly.

The new circuit requires its own decoder head admitting new source addresses, typed reference-afferent semantics, matched dynamics identity, branch-plan identity, and priced capacity/admission. Class 5's feature lanes alone cannot provide these. Minimum added declared cost before padding/scheduling is 64 neuron slots, 22 encoder assignments, 30 branches with 104 inputs, 50 decoder rows; price all additions, tiles, microsteps, eligibility, captures and any optimizer storage. Existing class 5 has 34048 synapses; simply appending 50 readout rows would make 34098, but branch inputs have separate costs and final budget/admission has NOT been implemented. A source manifest is not a budget receipt.

## All eleven drive meanings and unresolved gaps

- Hunger: positive appetite/search with observed positive chemical evidence, negative aversive evidence; low hunger/satiation contributes no new appetite. Need actual ingestion and post-meal persistence tests.
- Fatigue: recovery and object-directed movement suppression; does not suppress noLoco or eating in place.
- Fear: stronger escape-state proposal, weak recovery opposition; broad alarm is not learned target-specific fear.
- Pain: small chronic-repair/recovery term only in this circuit. Fresh localized contact uses the separate short directional reflex; class 5 has no candidate-specific damage-source match, so no invented voluntary source localization.
- Loneliness: sensed-peer approach and intrinsic vocalization; peer is not friendliness or consent, Grab is not social contact.
- Curiosity: weak inspect/search opportunity, not distress; object 36 is not novelty. Reserved 44 stays 0.
- ATP availability: only its BELOW-reference opponent feeds recovery. High ATP is not a deficit and receives no generic urgency reward. Current B1 whole-care lacks an explicit ATP circuit; this is a new dependency, not a solved old route.
- Temperature: unsigned bodily stress gates symmetric avoidance of observed hot/cold excess and discourages recovery under current observed exposure. No inference that a cold body needs warming or vice versa. Real thermal physics and sensor/physics-conflict controls are required.
- Reproductive readiness: weak attention toward an already sensed peer; not distress, mating eligibility or a mating command.
- Extension 0: canonical bound boredom; weak investigation/search while actual play expectancy 45 remains 0. No invented toy or stimulation truth.
- Extension 1: physically carried and recorded but no active decoder or arbitration input until explicitly bound. Behavior must be null under a canonical unbound-lane sweep.

The physical pain path must remain outside recovery competition. In the inspected closed physiology source, `headless/contact_withdrawal.rs` provides the separately admitted body-only contact response; chronic accumulated injury alone does not invoke it, fresh damage interrupts sleep even if movement is blocked. Broader V23 awake contact/thermal withdrawal work is a distinct candidate and must be integrated/tested under its own identity. Do not infer that those awake capabilities were exercised by this harmless eight-tick matrix. Do not duplicate a reflex on top of the existing one, turn fear into an unconditional physical reflex, or give a reflex voluntary learning credit without exact causal accounting.

## Controls and acceptance sequence

Native execution is outside this source-only publication. Before subsequent ordinary fresh experiments, complete these engineering qualifications:

1. Source/identity: class 5/legacy reject new head and new neuron addresses; old assets/saves/weights/optimizer moments and integer ages untouched; same persistent old coordinates; equal new pair starts; validated matched encoder/background/dynamics; source/relay/branch/readout reachability; correct costs and eligibility.
2. Mechanism-only scalar/native replay in the subsequent execution task: compare baseline, feature-gates-only, matched chains without recurrence, full recurrent arbiter, source cut, paired reference cut, R→V/V→R cuts, E→R cut, readout cut, and equal-budget sham. Sham has identical active neurons/branches/routes/readout count and charge, with behavior-bearing inputs/readouts balanced to zero, not a cheap empty graph. Hold native work and old proposal weights common.
3. Gain/threshold: exact ordinary gain 0.5, threshold +0.25 plus allowed biological envelope; zero/positive/negative equal pair starts, warmup/onset/washout, 1/3/full microsteps, common versus deliberately mismatched references, inherited/bounded old dendrites. The mismatch arm should fail the claimed cancellation. Measure each pair differential and derivative, not coefficient labels.
4. Needs: H/F grid(.1,.35,.65,.9), especially highH+highF (not in the closed data), highfear+highfatigue, acute localized/unlocalized pain+fatigue, chronic pain without fresh damage, ATP(.1,.5,.9)×fatigue, curiosity/boredom+fatigue, loneliness+fear, readiness+lowATP, unbound extension sweep. Use chemistry-derived drives, not post-encoder injection, for physiology claims.
5. Scenes: no object/comfortable; no object/fatigued; zero-descriptor observed object; equal-food ±chemistry; distant versus contact food; learned nutritious negative-taste cue; decoys; multiple approach/flee candidates; peer versus matched inert object; cue removal; symmetric hot/cold and insulation/cue/physics conflicts. Keep noLoco present.
6. Recovery and safety: report moving-Rest as numerator and all Rest bundles as denominator, conditional noLoco probability while objects exist, realized displacement per segment, attempted/blocked translations, recovery dose, actual fatigue/ATP/intake, fresh damage/reflex direction, hostsleep and consolidation separately. Eat-in-place/rest can be a valid joint behavior. Reflex interruption is expected safety, not a rest failure to hide.
7. Sampling: use held-out random seeds after common-random-number comparisons; T = 1 is primary, deterministic argmax is only a secondary diagnostic. Do not lower temperature merely to hide contradictions. Independent channel sampling leaves nonzero inconsistent-bundle probability at every finite set of logits; this proposal aims to reduce it, not guarantee zero. If strict same-tick coherence is required, a separately versioned learned shared motor-context/joint distribution must condition BOTH posture and locomotion, with exact joint logprob/eligibility, CPU/WGSL and checkpoint support. A host rule 'if Rest then no movement' is not an acceptable substitute.
8. Once mechanism is qualified, fresh disposable worlds with ordinary learning and independent seeds; no default promotion or retained-life migration. Verify whether freezing/tying only the new matched parameters changes acquisition and compare learned gains honestly. Re-test preservation/save/restore when integration exists.

Inspiration: [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/), experiments 4, 5, and 6: targeted circuit surgery, explicit iterative hypotheses, matched controls and inspection rather than a one-shot success claim. Those controllers are not equivalent to A-Life's recurrent biochemical organism. Their results are motivation, not evidence for this circuit.

## Refinement: actual controller-to-motor mediation

The direct 50-row proposal above is **parallel supervision of candidate logits**. It is not yet a controller driving a distinct motor execution population. Do not describe it as completed motor-control-lobe mediation. `mediation_manifest()` now specifies two exact variants; neither was built, simulated, or admitted in this refinement.

**M0, minimum new-path mediation:** retain the 52 active controller/drive neurons, then append 20 family-specific motor execution pairs. Their exact order is (family enum order, controller signal address), addresses 2100+2j /2101+2j for j = 0–19. Each execution signal has one branch with +4*controllerSignal−4*controllerReference, threshold 0/gain 1; its reference has the same 2 slots with zero weights. Copy the 50 existing proposed readouts onto these execution pairs and disable the original direct controller readouts by head/routing identity. Thus every new biological controller output follows controller→motor execution neurons→decoder. The family mapping in `mediation-m0-manifest.json` is complete. M0 requires 96 new capacity slots (2144 total), 92 active, 70 new branches/184 inputs, 22 encoder assignments and 50 new decoder rows. Relative to direct supervision this adds 32 capacity slots, 40 active neurons, 40 branches/80 inputs, and no readout rows. Old proposal neurons 1728–1855 still have their original separately measured readout pathway. M0 is genuine mediation of new outputs, but **not mediation of the old proposal bank**.

**M1, old proposals mediated too:** preserve each of the 128 old action-proposal neurons individually. No guessed mean, family compression or overwritten old neural state. Add 128 fresh signed execution pairs at 2144+2i /2145+2i, i = 0–127, sourced from 1728+i. Each source has recurrent weights +1 to its signal cell and 0 to its reference cell, with identical pair dynamics/background and explicit parameter protection. Unlike the nonnegative rectified controller pairs, these are signed proposal-transfer pairs: at equal initial state their differential has the input sign, but gain/leak/nonlinearity introduce distortion and latency that must be measured. Reference compensation does not make the old function identical.

For each of class 5's 4352 old action-decoder coordinates (8 families×16 motor sources×34 lanes), preserve its exact source coefficient w and materialize two fresh execution readout rows, +w/−w. `materialize_legacy_readouts()` is a concrete source-only constructor requiring all 4352 unique persistent-address-verified records; it rejects missing/duplicate/invalid rows. It does not read files or supply unverified founder coefficients. The original 4352-row bank remains intact for provenance/diagnostics but cannot emit into decision logits under M1's explicit routing identity. Zeroing that bank's genetic coefficients would not reliably deactivate it under ordinary learning.

M1 totals: 2400 neurons, 352 new capacity slots, 348 active new neurons, 4 reserved slots 2140–2143; 22 new encoder assignments; 70 new branches/184 inputs; 256 recurrent copy rows including explicit zero-reference rows; 8754 added decoder rows (8704 mirrored legacy +50 new controlled readouts). Keeping the class 5 base 34048 rows allocated gives 43058 rows, not including separate dendritic-input storage. Relative to M0, add 256 neuron slots, 256 recurrent rows, 8704 decoder rows. This deliberately preserves the old bank instead of silently reusing its persistent coordinates. Charge storage/capture for retained inactive rows and actual evaluation/diagnostic work honestly; final tile packing, eligibility storage and per-step pricing remain unimplemented. All variants must be compared against equal-cost/shaped shams.

M1's path is now old proposal neurons→fresh signed motor execution pairs→decoder, alongside controller→fresh family-specific execution pairs→decoder. Log old proposal input, transfer-pair differential, inherited coefficient contribution, controlled contribution, and final selected primitive separately. Innate scalar, memory and cognitive candidate terms remain separately audited additive paths; speech-token decoding is also separate. Therefore M1 is action-neuron mediation, not a claim that every decision component or motor token pathway has been centralized. Complete exclusion of those other bypasses would require additional explicit mediation and accounting, not a claim made here.

Both variants add a microstep dependency. The original three-microstep interval cannot be treated as if the new motor layer were already settled. Compare latency/onset/washout at the existing budget first; any extra microsteps or staged updates must be explicitly priced and versioned. No hidden extra settling pass or compensating division by a possibly zero biochemical gain is proposed.

### Lifetime protection is an implementation requirement

Current ordinary learning can update initially zero genetic edges. Zero genetic weights or alpha alone must **not** be treated as a freeze. The scalar order proof applies only while the pair has identical shared dynamics/background and the declared paired parameters are preserved. It is not evidence of lifetime safety under independent somatic drift.

For initial native qualification, implement an explicit versioned fixed-parameter contract for every new matched calibration/reference/branch/copy/readout parameter across ALL ordinary somatic writers, replay/consolidation, structural changes and optimizer writes. Keep ordinary learning in the rest of the brain. For M1, a fixed mirrored copy of old decoder coefficients is a mechanism control, **not preservation of old decoder learning**. A later learning-capable variant must implement and price symmetric/tied updates, eligibility and optimizer moments/ages rather than update the two halves independently. Either choice requires longer offset/onset/washout checks, adversarial branch and fast-weight perturbations, nonzero learning credit, capture/restore equality, and continued-learning checks after restore. A manifest's requested protection is not runtime enforcement.

Scope clarification: this publication archives the prospective design and existing scalar/construction checks. It includes no native implementation, new build, new world execution, study, pull request, merge, or default promotion. The engineering qualification sequence above remains necessary before claiming native or behavioral results.

# Explicit inherited wake/recovery calibration candidate

Base: 2da9dd323ad6f074556a851174a2edbc9e34e216 (tree e402bcb5d5fc034ac0244e78d49fa1a0da25ede3). Separate copy: /workspace/scratch/ee5e08a3ca9b/waking-recovery-candidate-v1. No neural-factor, canonical founder, loaded-save, checkpoint, optimizer, or acquired-state changes. No publication or autonomous lifetime allocation.

## Findings and proposed factor

The existing explicit with_waking_recovery recipe adds .00007 awake FATIGUE and SLEEP_PRESSURE per chemistry tick with retention .99995, but removes .70 fatigue/.75 pressure for each full SleepRecovery event. The independent canonical-founder chemistry envelope (biological-drive-audit-v1/physiology-v1.json) reaches only .482879 at age4514 even held awake/full expression. That is below the actual compiled brain's .6 pressure threshold and .9 fatigue threshold. This was fixed-reserve chemistry, not a living lifespan.

World simulation is20 ticks/second (headless.rs WORLD_TICKS_PER_SECOND). PassiveBodyUpkeepPolicy lifespan is round(1824*(2+lifespan_scale)); the expressed founder4514 gives225.7 simulated seconds. Rendering/wall time can differ. canonical_preparation::sleep_consolidation_config_for takes pressure entry threshold from brain architecture. SleepController::evaluate_homeostasis first checks forced recovery conditions, then normal fatigue/pressure thresholds. Runtime Waking requires both drives below thresholds, and sleep consolidation itself has phase/data requirements.

New API: BiochemicalGraphChromosome::with_waking_recovery_calibration(side, awake_gain, recovery_gain). It installs the existing inherited waking circuit then explicitly changes only Awake and SleepRecovery emitter gains for FATIGUE/SLEEP_PRESSURE. Values must be finite within[0,1]. Actual values reside in inherited serialized emitter genes, not mutable state or an unbound sidecar. Both homologs must be explicitly selected for equal expression. Old with_waking_recovery and all call sites/defaults are untouched. No migration silently changes old genomes. Existing arbitrary gene APIs already permit explicit construction; this adds no state reset.

Proposed distinct fixture recipe: awake_gain=.001, recovery_gain=.01, retention unchanged .99995, developmental expression unchanged. Rationale: a full-expression pressure rise of .4 needs about400 ticks/20 seconds, a modest fraction of4514-tick lifetime; a .01 recovery unit takes tens of ordinary recovery events rather than instant exhaustion reset. This is an analytical initial candidate, not outcome-tuned or behavior-qualified. .0005 and .002 bracket the proposal by2x.

## Native results

Rust1.99.1; jobs1; incremental0; debug0; tests single-threaded. Shared target was reserved through ordinary worker coordination then released. Command:

cargo test -p alife_core --test waking_recovery_calibration --test biochemistry_development --test biochemical_graph_v2 -- --nocapture --test-threads=1

28 passed,0 failed:4 new,13 development/body tests,11 biochemical graph tests. Log: /workspace/scratch/ee5e08a3ca9b/waking-recovery-tests-final.log. First run's3 tests also passed; final run added fresh-development fixture. Final source afterward changed only whitespace in two inserted expressions; git diff --check passes.

Controlled chemistry pressure .6 crossing (expression .25/.5/1):
- .0005:4970/2108/817 ticks. Quarter expression exceeds4514 lifespan.
- .001:2331/1026/405 ticks.
- .002:1132/507/202 ticks.

Fresh-body fixture uses actual developmental advancement and SleepController, with adequate reserve reset to.8 each interval explicitly as a controlled fixture, not a feeding policy. Normal EnteringSleep occurs at735 ticks for expressed lifespans3648,4514,5472. This is36.75 simulated seconds, at most20.2% of tested lifespan. It verifies no forced recovery substitutes for ordinary pressure entry. It is not canonical N2048 policy behavior: this core test uses a fresh N512 founder and the separately measured .6 gate explicitly.

Tests additionally verify:
- Both candidate homologs serialize/deserialize exactly and expressed graph roundtrips.
- Candidate gene bytes differ from baseline; baseline and serialized stored BiochemistryState remain unchanged.
- NaN/infinite/out-of-range gains reject.
- A single recovery event reduces pressure modestly (remains above.55 at first .6 crossing);200 recoveries release both .6 pressure/.9 fatigue gates at each expression.
- Rest preserves resource cost: reserves decline, but less than awake controls; no SleepRecovery→BRAIN_ATP emitter.
- Food raises reserves but does not erase fatigue; existing graph/body regression suites remain green.

## Separate world defect: moving and resting together

Source-confirmed, not native-world reproduced in this factor: apply_registered_motor_bundle_inner executes channels in deterministic order, sums body events, and does not suppress SleepRecovery for locomotion/displacement/effort. Rest contributes1. combine_body_event adds it regardless of motion. BodyState::apply_event sets sleeping=(recovery>0). Thus joint Rest+Locomotion can classify moving work as sleep, obtaining fatigue relief and sleeping metabolic regulation while movement still pays its explicit signed effort.

This factor reduces the per-event relief but does NOT correct false resting classification. It must not be represented as a complete biological drive repair. A separate world transaction rule must resolve realized movement/effort versus valid resting, including blocked motion and carried-load cost. Parent and drive owner were notified. Do not silently combine that world correction into this chemistry factor.

## Other gap

Loneliness has SocialContact→LONELINESS−.60 and decay toward expressed baseline; no Alone source or positive loneliness emitter. After contact it rebounds toward baseline, not growing past it through ongoing isolation. Parent and drive owner notified; no unrelated recipe edit.

## Limits and next evidence

No autonomous seed cohort, full living lifespan, GPU cognition, actual action choice, completed replay/consolidation sleep cycle, carry-work experiment, mixed movement/rest world test, or long repeated sleep/wake policy test was run. Recovery below gate is verified; entire scheduler transition/consolidation is not. Loaded chemistry JSON roundtrip is verified, not a full world save/resume for this new candidate. Source has no loaded-state path changes. Persistent genome identity should be bound through existing canonical genome/phenotype serialization, with full world-source identity guard checked before any exposure.

Next: resolve/measure mixed Rest+movement independently; then fresh fixture of actual canonical candidate genome with exact identity, normal developmental schedule, finite resources, genuine rest and completed sleep/release. Compare unchanged physiology versus calibrated physiology while holding neural genotype/policy fixed. Only then consider a separate cross-factor with dendrite or neural drive changes. No guarantee of continuous action, no hidden reward/oracle, and no claim of emotion from competing concentrations.

Method context only: https://rikiyat.github.io/blog/weight-space-programming/ (Experiment4 repair,5 static whole-policy,6 iterative feedback) does not prove this biological calibration.

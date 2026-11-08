# Harmful grasp memory: source trace and next experiment

Status: source/public-scalar investigation only. No native build, replay, new life, checkpoint access, existing-control edit, or study activation. The completed four-life data remain unchanged.

## Finding

Reject the strong hypothesis that harmful experience has no route into legacy Grab. Do retain the narrower hypothesis that ingestion-specific value is useful to explicit Close/Release independently of target fear/pain changes.

The actual experiment used the frozen v12 constructor, not default compiler weights. In world-swap-matched-source-old-v5/crates/alife_training/examples/support/world_swap_legacy_v12.rs, construct() calls candidate_with_readouts(2026110901, 0.25, true, 12., true). Inherited summed MemoryContext coefficients are:

- Contact (including Grab) and Approach: lane25 = -8; lane26 = -32.
- Ingest: lane25 = -16; lane26 = -64; lane32 = +32.
- Avoid: lane25 = +16; lane26 = +128.
- Other memory coefficients = 0.

These are sums over duplicate rows, not individual weight values. ActionCandidate genes are scaled by 0.25 and memory gain cap is 12. Compiler defaults (-0.15/-0.35 for Contact) are NOT the study's actual inherited readout.

In crates/alife_core/src/memory/candidate_recall.rs: target_latent[1] and [2] are homeostatic fear and pain DELTAS. They are not experienced valence, accumulated pain, or objective object harm. Target retrieval uses a target namespace without family identity (candidate_index.rs:TargetMemoryBucketKey) and aggregates supported target records, so cross-family consequences can already enter Grab. Public confidence/source counts alone do not establish their values or which target episodes contributed.

In crates/alife_training/src/cpu/input.rs: lane25 = target_latent[1] * target_confidence, lane26 = target_latent[2] * target_confidence, lane32 = family_value[0] * family_confidence. Legacy Grab's direct family-valence coefficient is zero. Ingest's family-value coefficient is nonzero. In cpu/replay.rs: each memory contribution is decoder_input * (weight + effective_weight_offset), summed by family and then clamped to the memory gain. The final candidate logit also contains family bias, innate bias, action-neuron readout and separately clamped cognitive contribution. Selection cannot be inferred from memory alone.

## What the existing scalar evidence establishes

Use harmful-food-regrabs-analysis-v1/analysis.json (SHA256 9df23d1c31e08d58784fa324e90c56cda81d14a2282c3f7633385d2a87e02cb5), not a new event count. At all 32 post-harm actual acquisitions, the concurrent H/Eat candidate recalls the exact harmful ingestion, with approximately -0.291 family valence. In 31 selected Grab cases the Contact-family evidence is absent or prior contact actions; one acquisition is Eat preparation. None is another harmful consumption.

At seed1 tick48 the simultaneous Eat family lane32 is -0.2908289134502411 * 0.9270134568214417, approximately -0.2696, corresponding to about -8.63 raw inherited family-value contribution before adding target channels and clamping. Selected Grab's own family-value is zero there. This is an illustrative inherited contribution, NOT an observed Eat logit or proof of causal suppression.

Public rows have no candidate logits, probabilities, target latent values, innate/action/cognitive score components, effective per-lane coefficients, or acquired readout weights. Therefore exact selected-Grab versus alternative-Eat same-frame logit attribution is unavailable within the permitted data. Do not load old lives or private banks to fill that gap. Target source identities also are not exported. The completed experiment supports correct ingestion recall but cannot identify the grasp-scoring bottleneck.

More acquisitions alone are not failure: new world reduces H-held boundary counts 103→46 and 117→53 and permits more safe meals. Optimize actual harmful meals, safe meals, held duration, energy and failed operations, not regrab count in isolation.

## Optional research candidate: qualified explicit operation reader

The current evidence does not warrant changing handling policy. No observed neural defect or causal scoring attribution has been established. If a separate, utility-based handling problem is demonstrated, the already-qualified recovered-alife-grip-integration-20261108 source supplies a testable option, not a recommended default or behaviorally proven replacement. It is a cleaner experiment than doubling legacy Grab target coefficients: those coefficients are already large, the target deltas are unobserved, and Grab conflates acquisition and release.

Exact interface:
- Opt-in SensorProfile::GroundedGripObjectSlotsV1 (profile4); no legacy profile changes.
- Explicit Close action2201/Contact and Release2202/Other; physical heldness appended outside frozen 24 object lanes.
- Grip-only recall projection queries target's Ingest family but stores actual operation outcomes under actual operation, never synthetic Ingest.
- MemoryContext schema3, stride54, operation-only lane53 = recalled ingestion family_value[0] * family_confidence. Non-operation candidates receive exact zero.
- For every family, retarget the eight MemoryContext duplicates with channel7/input_index55 from lane31 to lane53, preserving endpoints, count and zero inherited coefficients: 64 coordinate changes.
- Candidate versus operation-reader-off control differs in exactly 16 weights: eight Contact/lane53 rows from0 to+4 each (sum+32); eight Other/lane53 rows from0 to-4 each (sum-32). All other coefficients, action scaling, recurrence and cap12 identical. No whole-head overwrite between the matched candidate and control.

The existing probe_grip_reader.rs candidate(seed, bool) supplies exactly this pair. Its common constructor reconstructs inherited whole_care genes and v12 readers; that is fresh genetic construction, not a migration or edit of an acquired brain. Verify bitwise common-weight equality and the exact 16-row diff rather than assuming whole_care equivalence across source revisions. Both arms use the same new interface and 64 coordinate changes, so this contrast isolates the operation-value readout, not the combined interface effect against v12.

A fair later v12-versus-operation-off contrast would isolate neither just memory nor just action semantics; label it an interface bundle. Do not substitute old completed lives as matched controls for fresh new-profile creatures.

## Why not smaller legacy changes first

1. Contact/lane32 gain alone reads Contact value, not ingestion value; current near-zero Contact valence predicts little direct effect.
2. Increasing Contact/lane25 or26 gain tests stronger fear/pain-delta weighting. It does not add ingestion-value transfer, and those deltas are currently unknown.
3. Routing legacy Grab to Ingest value changes the meaning of a toggle whose physical action can acquire OR release. A single positive coefficient would suppress both acquiring and releasing a disliked target. Correct sign requires heldness/operation disambiguation, which the qualified explicit interface already provides safely.

## Bounded qualification proposal, not activated

No tests launched by this investigation.

Detached fixtures before any autonomous exposure:
- Assert profile/action isolation: legacy Grab and non-grip Other remain zero on lane53; no change to legacy ABI or checkpoint guard.
- Verify exact 64 coordinate retargets,16 coefficient changes, shared weights/identity rules, endpoint invariance and deterministic fresh construction.
- Opposite valence, zero confidence, missing recall, stale frame and target mismatch: Close negative/Release positive for aversive ingestion; sign reversal for appetitive ingestion; zero without evidence. Check safe objects are not hardcoded.
- Preserve all 24 object lanes and show source retrieval is Ingest while stored operation stays Contact/Other. CPU/GPU encoding and gradient parity; world-owned validation, heldness and atomic swap behavior.
- Save/load correctness with existing supported fixtures only, not any accepted creature load/reset.

If fixtures pass and a bounded fresh-study allocation is coordinated: fixed merged new world only; two fresh seeds crossed with reader-off/on = four fresh lives, 256 primary ticks each, same preparation and ordinary sleep. No retuning, retries, cross-arm state transfer, or trained-checkpoint export. This is a small descriptive test, not promotion evidence.

Add narrowly allowlisted scalar diagnostics prospectively, not raw memory/checkpoint exports: selected and concurrent same-target operation/Eat candidate logits; operation lane53 and target lanes25/26; total effective coefficients for these lanes; raw/clamped memory contribution; other combined logit contribution; selected probability; ownership and executed physical operation; source sequence and exact ingestion flag. Only aggregate scalar effective coefficients, never whole private weight arrays. Keep diagnostic reads passive and prove no effect on sampling/state.

Primary comparisons: safe/harmful physical meals, health/energy/survival, H-held duration, acquisition/release sequences, operation failures, exact-ingestion recall availability and candidate score decomposition. Reader effect can be called mechanistically supported only if the logged operation contribution shifts logits in the expected direction. Benefit remains unproven until physical outcomes improve without safe-food starvation or release oscillation.

## Adversarial interpretation and utility gate

H is harmful to eat, not necessarily harmful to grab. Family-specific aversion can be the correct learned affordance: avoid consuming H while retaining harmless exploration, carrying, relocation, and disposal. The observed absence of repeated harmful meals, reduced held time, and improved safe feeding do not establish a handling deficit. Regrab counts alone provide no reason to intervene. Do not call family separation amnesia or claim the operation reader repairs a defect. The current recommendation is to preserve the accepted behavior and withhold policy promotion.

Before allocating a handling intervention study, use ordinary physical events to define an actual opportunity cost: time with reachable safe food while hungry, delays to safe ingestion attributable to held-object interference, failed feeding operations, displacement of useful actions, and net energy/health. Mere temporal overlap or more manipulation is not causal evidence. Compare matched costs, allowing legitimate curiosity and non-ingest goals. Existing public observations do not separate those motivations or establish such attribution.

Additional bounded controls for any future candidate evaluation:

- Harmless H handling: a task with real physical benefit from moving an inedible/aversive-to-eat object, without consuming it. Compare successful pickup, carry, placement/release and completion cost. Do not label refusal to pick it up a safety success.
- Food relocation: useful carrying of a safe food item to a suitable feeding location, alongside harmless relocation/disposal of H to clear access. Measure task completion and feeding delay, not only reduced holding. Any necessary world task must be explicit and separately qualified, not smuggled into a matched source comparison.
- Task-conditioned release: distinguish releasing H to obtain reachable safe food from indiscriminate immediate release during useful transport. Appetite, current task and affordances must justify the operation; ingestion valence alone does not encode them.
- Other-target control: negative ingestion experience with H must not suppress useful handling of an unrelated object or safe food. Check target binding and score decomposition, with matched counterbalanced identities/positions to exclude a fixed object label rule.
- Unrelated-pain control: pain from a separate source, without an adverse ingestion consequence for the handled target, must not cause blanket handling aversion. Keep targeted acquired consequences distinct from generalized state-dependent caution.
- Existing legacy-toggle sign hazard remains: applying a single positive ingestion-value coefficient to Grab can inhibit release as well as acquisition. Explicit Close/Release avoids that ambiguity but can still impose unjustified object-wide aversion.

Decision rule: treat the 16-gene reader-on/off pair as an optional mechanistic experiment only. Withhold promotion if it merely reduces H acquisitions, suppresses beneficial H handling/food relocation, induces release oscillation, harms safe-food use, or fails to improve the predefined feeding-interruption/utility measures. A logit shift proves that the port acts, not that the behavior is better. Advance only if repeated independently matched evidence shows a practical net benefit on the prespecified task costs while preserving harmless manipulation, unrelated-target handling, physiological outcomes, acquired-state integrity and action-specific learning. A two-seed assay can reject an obviously harmful candidate or justify further investigation; it cannot by itself justify default promotion. If no handling cost is demonstrated, make no neural change.

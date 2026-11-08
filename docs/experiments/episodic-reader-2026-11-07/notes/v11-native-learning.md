# Native memory-reader factorial pilot

## Result

The inherited episodic reader has a real, target-specific behavioral effect after experience and sleep, but it does not produce sustained harmful-food avoidance in this pilot. Keep the result negative on that primary endpoint. All four fresh lives reached three harmful A meals and the vitality safeguard. No dose extension, revised coefficients or replacement lives were run.

| Reader | Memory head cap | Host ticks | A meals | B meals | A meal indices |
|---|---:|---:|---:|---:|---|
| Off | 0.5 | 57 | 3 | 3 | 18, 38, 56 |
| On | 0.5 | 58 | 3 | 3 | 18, 38, 57 |
| Off | 12 | 57 | 3 | 3 | 18, 38, 56 |
| On | 12 | 82 | 3 | 4 | 18, 56, 81 |

Indices are zero-based host ticks after the same 600-tick biological preparation. Every A meal produced measured harm 0.28 and negative native biological credit. World reward_valence remained zero. On/12 A credits were -0.086455, -0.087409 and -0.088675. B credits stayed positive, about 0.00225–0.00234. All lives requested recovery sleep immediately after A at index 18 (world tick 619), consolidated at index 27 and resumed ordinary decisions. On/12 had 50 post-sleep decisions.

No phase fix, geometry patch, new learning rate, artificial reward, forced choice, Adam update or acquired-state reset was used. Foods had equal positive chemistry, nutrition and geometry, distinct observable appearance and tracked identity, and native same-object regrowth. The prior harmful-world correction is included. The seed 2026110901 is development evidence, not a held-out evaluation.

## Preserved baseline and controls

All twelve 24-tick baseline lives passed the same local selectivity checks: one safe positive meal, zero bitter meals with 1.161 m retreat, and 1.113 m progress toward the 2.5 m far food. Far-food consumption was zero; this short horizon does not establish a reaching failure. These outcomes were identical across all four arms.

The unchanged v6 constructor was byte-identical to the archived founder asset before baseline execution. Reader-off/cap 0.5 was byte-identical to the prior quarter-gain candidate. Its 44 native decision rows and 57 world-boundary rows match the frozen v10 life exactly across choices, perception, outcomes, credit, physiology, neural-bank statistics, sleep, position and ecology. Adding diagnostics did not change that life.

All sixteen lives preserved founder weights, Adam moments and integer ages exactly. All sixteen fresh checkpoint round trips and next native transitions matched exactly. Five core identity/graph tests, two CPU checkpoint/cross-restore tests, four constructor tests and thirteen GPU plan/upload/WGSL tests passed, along with formatting and quick boundary checks. GPU tests verify the versioned plan and shader contract, not hardware GPU behavior. Full workspace tests were not run.

## Evidence of acquired-memory influence

After the first A meal, on/12 had 33 visible A Ingest opportunities, including 21 contact opportunities. It selected A-directed Avoid on 12 rows and never selected B-directed Avoid. It continued eating B. This is not endpoint inactivity after injury.

At index 34, the first post-harm contact re-encounter, A's acquired target fear/pain inputs were 0.028532/0.065216. The genetic reader and small native offsets contributed:

| Family | Non-episodic score | Genetic memory contribution | Learned memory contribution | Final score |
|---|---:|---:|---:|---:|
| Approach A | 1.805118 | -2.315176 | -0.071851 | -0.581909 |
| Avoid A | -6.606627 | 8.804190 | about 0.000006 | 2.197568 |
| Ingest A | 3.743999 | -4.630352 | -0.073093 | -0.959445 |

The non-episodic score includes the unchanged action decoder and ordinary biases. This graph has no CognitiveContext decoder. None of these memory contributions hit the cap of 12.

Using the genuine immutable index-34 row, zeroing only explicit episodic input lanes 24–35 changes native sampled Avoid+Inspect(A) to Approach(A). Physiology, recurrent initial state, other inputs, receptor gates, effective offsets, structural context, RNG seed and sampling counter stay unchanged. Replaying the unaltered row exactly reproduces the actual factorized action. The independent arithmetic reconstruction of the memory contribution agrees with native logits within 0.000014. This establishes a current causal contribution of acquired episodic inputs, not general learning success or a whole-life learning-off comparison.

Reader on/off at cap 12 selected identical native actions through index 33. Other fields are not claimed wholly identical: an outcome field first differs at index 33. The same-row ablation does not rely on between-life physiological equivalence.

Both A and B were simultaneously visible as Ingest candidates on indices 49–52 after the first harm. At index 49 A was in contact while B was not; A Ingest scored -0.820 versus B 3.846, and B was selected. At index 52 B was in contact while A was not; B was consumed while locomotion approached A. There were zero post-harm rows with both foods simultaneously in contact, so this is not a balanced simultaneous-consumption-choice assay.

## Why the candidate still failed

### 1. The stored consequence disappears from the target readout under a changed query

The first harmful episode remains stored. There were zero evictions. At index 34 it ranks first by target similarity (0.997373) and enters the four-record target aggregate. At index 56 it ranks seventh (0.837238), below four benign records with similarities 0.946009, 0.945780, 0.943931 and 0.868208. All four selected records have zero pain. The target fear/pain readout becomes -0.000525/0.0, despite contact with the same A.

The reconstruction uses only records unchanged since the inspected boundaries, rather than treating the whole final bank as an earlier bank. The final fresh bank retains every earlier target record unchanged at both inspected boundaries: no record's merge interval crosses either boundary, and no record was evicted. Restricting records to those observed before each query and reproducing the source's similarity, top-four aggregation, confidence and retention equations reproduces all eight logged target inputs within 7e-8. See target-recall-attribution.json and trace_target_recall.py.

CPU uses audited recall, not the runtime approximate cache. The target score is 0.45 times sensory/body cosine over lanes 0–39, plus 0.50 times target-feature cosine over 57–80, plus 0.05 times profile cosine over 81–82. It then retains four matches. Exact individual identity constrains the namespace, but does not make the consequence stable across pose/contact/context changes. At index 56 the A Ingest score rebounds to 4.030774 and its representative probability is 80.004%. The second harmful meal follows.

The family-value lane is also benign at that row (0.0000628), following preparation, under the unchanged phase-insensitive rules. The separately owned phase fix was not included in this result.

### 2. Suppressed ingestion remains stochastic and can coexist with avoidance

At index 81 harm recall is present: fear/pain 0.026443/0.065699. Ingest receives -4.753561 episodic contribution and scores -0.708263, while Avoid scores 1.453693. The Ingest representative probability is still 3.8946%, and it is sampled. Choosing that representative forces its manipulation slot; the locomotion factor independently selects Avoid. The actual joint action consumes A while also avoiding it.

On the immutable same row, removing episodic inputs changes locomotion from Avoid to Approach but leaves Ingest selected. This is not a missing receptor signal, a cap 12 saturation, or a proof that physiology overwhelmed memory. It is the combination of a nonzero stochastic ingestion probability and independent motor factors. The current total score does not encode a guaranteed behavioral veto, and this experiment did not add one.

## Synaptic learning and retention

The four arms retain ordinary somatic learning. Final max absolute fast offsets were 0.003355, 0.003369, 0.003355 and 0.003722 respectively; max lifetime offset was 0.003123 in every arm. These are far below fast/lifetime bounds 2/4. The fixed first-row final A-minus-B Ingest margin in on/12 was +0.00000477, not an acquired synaptic A penalty. The demonstrated discriminative effect is readout of stored episodic experience, not a meaningful learned color synapse or Adam optimization.

There was one explicit sleep/consolidation event and actual post-sleep behavior. Reversal, unrelated acquired-memory retention, held-out seeds, side/cue swaps and sustained survival are untested. No update-off whole-life comparator is claimed. Reader-off changes inherited access to episodic evidence, not the organism's capacity to learn or store it.

## Frozen bindings and artifacts

- Source commit: 12de4b03829be328481008d4072e9ad754225520, experiment/native-learning-probe-20261107. Worktree clean; no push or promotion.
- Frozen executable SHA256: 6c588cdcd43bf28468f0b01e9c5e9087fdca26e9b268376160fdca7d42efd0f4.
- New source-only patch from 498d4574: 2b277171ac27829565481be25db949de8cd9c2faeccc45bd27d810e81c341247.
- Full source-only patch from f4cdbeed: 89051744878171f7ef038c2a879d2f554c34b63b42566aba47add1d25a6ae240.
- Original v6 founder SHA256: 94a70c709661d883d6f146dec773fe120f5f53a8e25949c8c38cf232d0d6b11f.
- Four asset hashes and precise fixture bounds are in pilot-01/manifest.json. Source/build/plan checks are in build-bindings.json and the test logs.
- independent-review.json verifies 70 diagnostic content hashes and native memory-effect arithmetic. mechanism-summary.json separates contributions and choice probabilities. v10-control-equality.json records exact unchanged-control evidence.
- Local fresh diagnostic snapshots remain outside the source tree and source patches. They are not publication artifacts. No private or retained user-life checkpoint was read.

## Code references

- crates/alife_training/examples/probe_native_learning.rs: candidate_with_memory, replay_without_episodic_context, run, cold_check.
- crates/alife_core/src/n2048_memory_gain_candidate.rs and phenotype/memory_channel.rs: explicit selector/recipe and fixed schema 2/cap 12. Existing schema 1/cap 0.5 remains unchanged.
- crates/alife_core/src/memory/candidate_recall.rs: recall_target_channel_with_budget, target_similarity, sort_and_truncate_matches, aggregate_target_matches. This experiment did not edit them.
- crates/alife_runtime/src/canonical_preparation.rs: audited baseline and routed memory recall.
- crates/alife_training/src/cpu/input.rs: encode_cpu_frame confidence-weighting; sample_cpu_joint_action representative and conditional motor draws.
- crates/alife_training/src/cpu/replay.rs: action, episodic and cognitive score assembly with the phenotype-bound cap.
- crates/alife_gpu_backend/shaders/closed_loop_memory_context.wgsl and src/closed_loop_buffers/bucket.rs: exact versioned plan admission and matching gain clamp.

# Bounded finalization fix and two-worker admission design — 2026-10-01

The production change is `4eed7accbd7a034f318ba5182b8f3edc487f0b43`, based on the coherent topology/cleanup candidate `dc3e916eb3d47db936048b93224a85bccec8145b`. It adds three lines in `PreparedMemoryRecall::finalize`: the exact self-validation of a newly constructed private result remains a debug diagnostic. Five existing stale-draft/cognitive-owner/tick cases now also reject through finalization. This is independent of the separate fresh-recall change `e8d5633e`; the integrator owns that change. No production threading, GPU launch source, rendering, gameplay policy, brain promotion, population or tick cadence changed in this slice.

## Finalization guard proof

`finalize` still runs full `validate_for_draft`, authenticates the context digest, invokes `draft.finalize` and constructs every decision key with strict `EpisodicDecisionKeyV2::try_new`. The frame retains that exact owned draft. Finalized recall fields are private, owned and non-deserializable; no callback or interior mutation occurs between these constructors and return.

| Fresh result check | Mandatory guard or construction retained |
| --- | --- |
| Frame, context and frame digest | `draft.finalize` validates the owned base/context and derives the digest. |
| Receipt, cognitive owner/tick, profile, candidate count and base/context identity | Full `PreparedMemoryRecall::validate_for_draft`. |
| Candidate query membership, order and receipt hash | Prepared queries are re-encoded against the immutable draft. |
| Decision-key query integrity and base-versus-final identity | Every strict `EpisodicDecisionKeyV2::try_new`. |
| Key context and final-frame bindings | Digests passed directly from the new context/frame to each key constructor. |

The public `FinalizedMemoryRecall::validate_for_frame` is unchanged and unconditional. The app invokes it after routed finalization; the memory upload constructor invokes it again. Debug builds retain the internal self-check. Invalid inputs retain their unconditional entry checks in release; this removes repeated derivation, not creature memory, attention, prediction, learning or decision work. It preserves `AOA-INV-003/010/011`, `AOA-AUTH-004/005`, `AOA-CTX-001`, `AOA-ATT-002` and `AOA-MEM-001/002` for this mechanism; it is not architecture-wide certification.

## Existing ordering and substantive dependencies

The full CPU suffix is read-only apart from the current early hysteresis assignment. The existing row loop interleaves more than computation:

1. Retained learning retries can apply a sealed GPU outcome and arm irreversible-commit fail-stop.
2. ATP binding, sleep/replay/consolidation mutate the owning resident, memory, topology and canonical body record. GPU commits and accepted persistence work have their existing recovery boundaries.
3. Grounded draft creation mutates owner-scoped entries in the shared world tracked-object registry. Capture each draft at its current place after that owner's housekeeping; collecting every draft after every owner's body update would change the world/body view used by successful perception.
4. `RuntimeSemanticPrior::prepare` consumes asynchronous replies, mutates per-life controllers and shared bounded cache/request order, and may write cache files. Workers must never call it.
5. Pure baseline recall/context, attention, routing/novelty, fresh routed recall, cognitive projection and finalization use owner-specific immutable inputs.
6. `prepare_memory_context_upload` checks device health and live handle/resident/profile before pure CPU encoding. Device-loss observation is stateful. The final backend batch also checks duplicate owners/slots, pending eligibility and activity sequence before submission.

Source locations: `gpu_live_runtime/staged_tick.rs` owner loop; `gpu_live_runtime/semantic_prior.rs::prepare_inner`; `headless.rs::perception_frame_draft_with_index`; `closed_loop_runtime.rs::prepare_memory_context_upload/ensure_ready`; `closed_loop_runtime/tick.rs` preflight; `gpu_live_runtime.rs::retry_retained_learning/StagedSleepAuthority/tick_with_sleep_progress_outcome`.

Different invalid-input error precedence alone does not block parallelism. The acquired-state requirements are: no worker-side authoritative writes; no stale slot or owner admitted; no duplicate neural/learning transaction; explicit failure before dispatch; and recovery through the existing staged host restoration and irreversible-GPU fail-stop. Host staging restores world/residents and cognitive sidecars. It does not undo an already committed GPU operation, an accepted persistence worker or prior/cache I/O. The design must not promise that rollback.

## Minimum production split to implement next

This is a concrete proposed split, not implemented production behavior or a new contract framework.

**Serial collection and admission.** Keep retirement/reconciliation, retained-learning retry, ATP/sleep progress, journal/checkpoint work, receptor derivation, grounded draft creation and per-owner `prior.prepare` calls in canonical owner order. Immediately retain each resulting owned draft, owner/sequence/handle/world identity, receptor frame/effects, homeostatic input, attention policy and initial hysteresis. Do not recreate a draft later. Complete housekeeping before borrowing maps for workers.

After collection, freeze read-only owner-specific banks, topology sidecars and predictors for the preparation scope. Other owners' housekeeping has no identified normal write path into those sidecars or that owner's sequence/predictor; sleep's context is owner-scoped and sealed/replay patch references are read-only. Verify that invariant with mixed sleep/retry traces rather than relying only on awake synthetic fixtures. No bank, topology, learned weights or predictor clone is needed for the pure scope.

Add one narrow backend admission accessor that checks readiness, backend/handle ownership, resident existence and sensor profile, then returns the immutable `GpuBrainSlot` metadata needed for CPU encoding. `GpuBrainSlot` contains records/counts/ranges/identity and decoder stride, not neural values or wgpu resources; its clone is an owned derived view. Bind it to the captured handle and frame owner/tick. Runtime session dereference already exposes backend methods; a second runtime contract or generic scheduler is unnecessary.

**Two bounded CPU workers.** Extract the existing pure suffix into one private app function. Use two contiguous owner chunks, with one scoped child and the calling thread. Jobs borrow only the frozen owner-specific data and receive owned drafts plus copied metadata. Each returns an owned preparation result, its stage/error and an optional staged hysteresis update when attention selection succeeded. Keep both recalls, query authentication, cognitive projection, exact keys, strict upload encoding and receptor binding. No worker receives the runtime, backend, world, semantic service, mutable bank, checkpoint queue or GPU handle operations.

**Ordered collection and live re-admission.** Join the child before returning, publishing effects or submitting. Collect in original owner order. Recheck device health and each live handle/profile and full `GpuBrainSlot` equality against the captured metadata, including ranges, decoder stride, identity and bucket ownership token; retain the complete existing memory-input and backend batch validators. Take the snapshot after all housekeeping. A slot metadata snapshot is not permission to bypass current admission. Keep durable sleep promotion/compaction publication at its existing boundary after CPU preparation. Rejected rows publish the existing failure summary and no dispatch; valid rows enter one ordered batch.

Publish each owner-bound staged attention update in ordered collection when attention selection succeeded, including a typed later routing/finalization/upload error. That preserves the existing explicit hysteresis transition; it is cognitive state, not incidental diagnostic order. A failure before selection has no update. A whole-tick failure or unexpected worker failure discards the private results and uses staged restoration. Workers never leak authoritative writes. Changing the established row-level hysteresis policy would need a separate behavior decision rather than being hidden in a speed optimization.

**Existing deterministic submission and commit.** Leave class grouping, whole-batch GPU preflight, authority nonce allocation, one neural dispatch, pending eligibility, selected-world outcome sealing, memory/topology observation, physiology cost, matching learning/discard, world advance, lifecycle and persistence completion on the existing serial path. A successfully prepared CPU row cannot independently advance acquired state or choose an action. Reuse the existing dispatch/eligibility identities; never retry a completed GPU transaction because a worker failed.

## Successful behavior and asynchronous hints

For identical admitted drafts, hints, receptors, policies, memory/topology/predictor and slot metadata, the same pure suffix should produce identical preparation outputs and payloads. This does not independently prove neural decisions; neural execution remains GPU-authoritative. Capturing each draft during its own serial housekeeping preserves the successful world/body view. Owner-specific hysteresis is not consulted by another owner's normal housekeeping; staging it does not introduce a new cross-owner policy.

Faster serial collection can nevertheless change which external prior reply is already ready when a later owner's `prepare` executes. That can change a successful hint/context and eventually neural behavior, not just which invalid input is reported first. Any optimization can change wall-time arrival relative to polling, but that is not a proof that the roster is unchanged. Keep hint consumption at the explicitly established serial collection point, once per dispatchable owner, and never poll from workers or let completion races choose a hint. Scripted reply-availability traces must verify source owner, causal sequence, expiry, request/delivery count and consumption point. If a common tick-start ready-set is chosen later, document its deliberate potential one-tick delivery shift; it is not required by the minimal split above.

## Failure and resource checks required for threading

| Case | Required outcome |
| --- | --- |
| A row has invalid draft/context/upload | No row dispatch/eligibility/learning. Drop invalid outputs; publish only the explicit owner-bound attention update if selection already succeeded, as today. Other valid rows follow the existing policy. |
| A later serial sleep/retry/checkpoint operation fails | No CPU batch dispatch; existing host restoration and armed GPU fail-stop retain their current responsibilities. |
| Thread creation fails | Recover with serial computation of the same admitted immutable inputs before any row effects; this is CPU scheduling, not a neural fallback. |
| Worker panics or cannot complete | Catch unexpected panics in both the child and calling-thread pure computations, join, and return a controlled failure through staged recovery; never `join().unwrap()` after an irreversible housekeeping commit. No silent successful retry after an indeterminate GPU commit. |
| Handle retires, profile/generation/phenotype changes or device is lost | Live re-admission rejects before dispatch; existing session recovery/fail-stop applies. |
| Eligibility already pending or a duplicate owner/slot is admitted | Existing whole-batch checks reject; no double inference or learning. |

A meaningful CPU gate compares serial and two-worker traces on the same captured inputs: 8/16/32/50 distinct owners, empty/64/256 retained records, mixed awake/sleep/retry/no-dispatch rows, owner-specific learned predictors/hysteresis, scripted ready/stale/disconnected hints, malformed rows in both chunks, creation failure and controlled worker failure. Compare all complete frames/contexts/keys/receipts/payload words, owner order and both successful and typed-late-failure hysteresis transitions. Count every dispatchable CPU job, memory query/similarity receipt, cognitive work, hint request/delivery and admitted row; no success-case work may disappear. Failure cases may compute bounded speculative read-only work which is discarded.

Measure whole admitted preparation including collection, slot metadata, scheduling, joins, output retention and admission; report process CPU, wall time and peak/RSS caveats. Do not add isolated fixture medians into FPS. No pool/framework or persistent task queue is needed for the first two-worker path. Keep one-core and small-batch execution serial when justified by measured scheduling cost. Device execution and rendering responsiveness require later authorized host proof; joining workers on the simulation thread does not itself make rendering responsive.

## Dated CPU comparison and scope

The [raw receipt](evidence/20261001-finalization.json), [dated probe](evidence/20261001-finalization-probe.rs) and [builder](evidence/20261001-finalization-build.py) retain the source comparison and narrow worker screen. The builder reuses the published fixture builder at `61edc70b` with checked substitutions, the protected attention probe and the exact current app helper extraction. It selects actual Cargo compiler-artifact release libraries, opt level 3/debug assertions off, and records their content hashes. Artifact review corrected a wrapper-variable hashing error in the recorded reused-builder SHA after measurement; the original timed wrapper hash and corrected replay wrapper hash are both explicit. Generated Rust, executables, source and raw samples are unchanged. No device, live runtime, training or provider is created.

Cohorts use distinct owners 811 onward, separate actual 0/64/256-record banks, two observed objects, one concept/gap, default predictors/hysteresis, focal capacity one and fixed valid receptors. The existing N512/54-lane asset is used for CPU encoding only. This is not a live population or a new brain-class promotion. The helper chain includes baseline/routed recall, attention/routing, projection, finalization, public final validation and CPU upload encoding/binding. It excludes actual world/perception/receptor derivation, resident mutations, sleep, backend admission, topology observation, GPU inference/readback/learning, sealing/persistence/lifecycle and rendering.

Two source comparison pairs run before→after, then after→before, with nine preparation samples per process/case: 18 per variant/case. Across 24 cases there are 864 timed batch proofs and 22,896 complete typed instance comparisons. All 24 input and core-result/payload digests match across all four processes; each timed result equals its same-process reference. Serialized proofs include attention/hysteresis, frames, recall context/keys/receipts, cognitive projection and literal perception/memory/cognitive upload words/effects. Host-only upload binding/digest/offset metadata is excluded from the serialized digest but included in full typed same-process equality; it is derived from the matching frame and slot. Query counts, similarity-work receipts and every row's candidate keys are preserved by full equality. Fixture construction, proof serialization and a retained reference result are outside clocks.

For 26 candidates and 64 retained records, pooled medians are:

| Independent owners | Helper wall before → after ms | Process CPU before → after ms |
| ---: | ---: | ---: |
| 8 | 10.072 → 9.121 | 10.071 → 9.120 |
| 16 | 20.561 → 18.564 | 20.559 → 18.491 |
| 32 | 41.564 → 36.443 | 41.544 → 36.441 |
| 50 | 66.299 → 57.762 | 66.193 → 57.758 |

At 50 owners the two pair medians are 66.639→57.858 and 65.055→57.165 ms. The pooled wall reduction is 12.9%. Across the grid observed pooled wall reductions range 1.3–17.0%; the 26-candidate/256-record/32-owner case has only a 1.3% signal, so the result is not a universal percentage or confidence interval. One retained public finalized-validator pass attributes about 6 ms at 50 owners/26 candidates/64 records; it is not subtracted from or added to the helper medians. This comparison excludes the separate `e8d5633e` fresh-recall guard now in later candidate `ad49a325`; it does not certify their combined performance.

The narrow screen runs only immutable `PerceptionFrame::validate`, the dominant part of memory-input construction. It is not a test or timing of opaque live handles, complete backend admission or the proposed full two-worker suffix. One scoped child plus the calling thread computes exactly one validator result per row; both return ordered complete result vectors. It adds no production API or fake live-handle constructor. It must not be used to predict scaling of the heavier suffix. Across four processes, 21 samples per mode/case produce 4,032 timed batches and 106,848 validator results. At 50 owners/64 records, pooled serial→two-thread wall is 0.216→0.320 ms for two candidates and 0.974→0.800 ms for 26 candidates. The saved 0.174 ms in the latter case does not justify shipping this isolated thread stage; smaller cases and individual process runs can be slower.

The shared Linux/KVM EPYC 9V74 host exposes five CPUs with a four-core cgroup quota. All runs use affinity 0–3, including the serial variant. Linux process CPU clocks include child CPU; normal timing includes output allocation and scoped-thread creation/join. Peak RSS across the four processes is 108.4–121.8 MiB. RSS includes inputs, retained reference/output vectors, allocator retention and preceding proof/fixture history; VmHWM is cumulative. These are not isolated per-stage allocation peaks or memory-saving claims. The faster 50-owner chain still exceeds the 50 ms tick budget before all excluded work.

Reproduce in separate worktrees at `dc3e916e` and `4eed7acc`, with the published probe/builder available in both. Fetch the pinned `61edc70b` fixture-builder commit if needed. In each worktree, source `/workspace/cloud-cpu-validation/env.sh`, then run:

```bash
cargo test -p alife_game_app --release --lib --features gpu-runtime \
  object_bound_concepts_and_gaps_change_predecision_focus_selectively \
  --message-format=json > /tmp/finalization-artifacts.jsonl
python3 docs/performance/evidence/20261001-finalization-build.py \
  /tmp/finalization-artifacts.jsonl /tmp/finalization-probe
taskset -c 0-3 /tmp/finalization-probe > /tmp/finalization-run.jsonl
```

Preserve each executable/build manifest before rebuilding shared Cargo targets; same-named rlibs may be overwritten, and recorded hashes identify build-time bytes. Run the baseline normal, successor normal, successor `--reverse`, then preserved baseline `--reverse`, with no concurrent local compilation/measurement. `--reverse` reverses the initial narrow-validator mode order; both modes rotate each round. Raw receipt arrays are compacted only for storage; integer samples are unchanged.

## Verification and handoff

Debug and release core query/retrieval suites each passed 42 tests; one pre-existing microbenchmark stayed ignored. The new finalization rejection assertions cover changed geometry, candidate order, remembered novelty and wrong cognitive owner/tick. Existing tests cover exact selected keys, stale contexts, population/owner isolation, persistence, poison/reversal retention, saturation and the 72-case prepared-attention matrix. The CPU app concept/gap test passed on the baseline and successor and preserves full prepared frames/keys/receipts and 54-lane upload comparisons. Strict Clippy passed for core all-targets in debug/release and app all-targets with `gpu-runtime`; formatting, core boundaries and all 77 documentation assertions passed. These checks instantiate no device or provider. The narrow screen does not exercise production spawn/panic recovery, which remains a threading implementation gate.

Mode 1 Micro-Spec, R2 separate read-only Sol6.1 review. No production parallelism has been implemented or certified. The requested 20 FPS with 50 creatures, sustainable 20 TPS and bounded debt/drops remain **Unknown**. Rollback for the shipped source change is reverting `4eed7acc`; no save/schema/GPU ABI migration is needed. The next bounded implementation is the explicit admission split above, with app preparation/collection and one backend metadata accessor as the only production ownership expansion. Rendering and operational launch source remain outside that scope.

The source-only patch applies cleanly to the later coherent fresh-recall candidate `ad49a32558233f6cdabf974eb7361ce0364ffbdd` in a temporary Git index. That is a compatibility check, not a merged or rebuilt combined candidate. Main Jules workflows were untouched. Final R2 artifact review independently recomputed every table, sample/proof count and range, verified source/executable identities and resolved the builder-hash and serialized-digest-scope findings.

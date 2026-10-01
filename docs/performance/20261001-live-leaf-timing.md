# Remaining live CPU attribution — 2026-10-01

This source-only diagnostic patch splits the existing performance recorder's
large preparation and topology-observation timers and measures input validation
before inference. It changes no simulation call, validation policy, authority,
learning, population, tick cadence, or persistence operation. The next dominant
leaf remains Unknown until the desktop recorder supplies these fields.

## Starting evidence and scope

Branch: `codex/cloud-live-leaf-timing-20261001`, based on consolidated
`2bad5e02b6bbf125a10822d3c7effd7debc96d8f`. The frozen CI branch and main were
not changed. Ownership is limited to `gpu_live_runtime.rs`, its `staged_tick.rs`,
the existing `production_voxel_renderer/performance_receipt.rs`, this note, and
the companion `20261001-memory-behavior.md` explanation.

Implementation: `7566a1b88caaeaa44b01e77e9f077f0633e6c07c` (tree
`3f11d89524485ab9e00cdf1cded8d6763e911439`).

The coordinator supplied Windows A/B means from
`C:/Users/PC/Documents/Codex/2026-10-01/task-2/A-Life-memory-ab/target/artifacts/memory-ab/comparison.json`.
The raw file and exact invocation are unavailable in cloud. The supplied result
reports 82.53 ms runtime wall time per tick and 91.25 ms median frame time after
the memory validation change. Preparation fell from 104.77 to 33.32 ms/tick:
topology/concept 43.79→12.98, episodic retrieval 37.89→11.44, and upload
preparation 17.92→3.97 ms. Selection preparation fell 18.75→4.75 ms.

Body/world/biochemistry sealing remained about 8.01 ms/tick. Sealed commit grew
14.45→17.42 ms, tracking topology observation 4.88→7.81 ms. Faster runs reached
older simulation states, so these totals do not establish a regression caused
by the patch. Inference/learning transaction walls were 5.76/2.46 ms, versus
covered GPU inference/plasticity 2.41/0.42 ms. The supplied unexplained gaps are
9.09 ms/tick outside named top-level stages and 7.87 ms/frame outside measured
host updates. Neither is isolated rendering or synchronization time.

This note records coordinator-supplied hardware evidence, not a cloud hardware
measurement or a fresh performance claim for the diagnostic patch.

## Added recorder fields

All values are cumulative nanoseconds over the existing recording interval;
divide by `simulation.completed_world_ticks * 1_000_000` for milliseconds per
completed tick.
Retain frame counts, preparation errors, zero-progress counts, sleep transitions,
and GPU timestamp coverage when interpreting them.

| Receipt path | Included work |
| --- | --- |
| `preparation_leaf_stages.episodic_retrieval.recall_ns` | Baseline `memory.recall_frame`, including its internal validation and bank digest. |
| `…episodic_retrieval.context_ns` | Baseline `cognitive_context_for_recall`. |
| `…episodic_retrieval.finalize_ns` | Prepared/context/draft cloning, cognitive-context attachment, and baseline finalization. |
| `…episodic_retrieval.validate_and_attention_evidence_ns` | Finalized recall validation and `finalized_memory_attention_evidence`. |
| `preparation_leaf_stages.topology_concept.routed_recall_ns` | Routed `memory.recall_frame`, including internal validation and digest. |
| `…topology_concept.context_with_attention_ns` | Routed context construction and attention attachment. |
| `…topology_concept.projection_with_context_ns` | `cognitive_projection_for_draft` and projection attachment, including their internal validation and prediction work. |
| `…topology_concept.finalize_ns` | Routed cognitive-context attachment and finalization. |
| `…topology_concept.validate_ns` | Explicit finalized routed recall validation. |
| `topology_observation_substages.observe_sealed_patch_ns` | Organism-owned topology patch observation. |
| `topology_observation_substages.advance_lifecycle_ns` | Existing lifecycle advancement, including its existing rejection/error result. |
| `internal_tick_stages.neural_input_rows_ns` | Building/validating all input rows and collecting their vector. |
| `internal_tick_stages.neural_input_batch_validation_ns` | Constructing/validating the memory input batch. |

Preparation leaves are children of the existing `episodic_retrieval_ns` and
`topology_concept_ns` totals. They are published only once their enclosing phase
completes successfully, matching the parent's failure accounting. The parent
also includes surrounding bookkeeping and instrumentation. Recall/finalization
remain inclusive operations; these clocks do not isolate their core internals.

Topology children belong to `internal_tick_stages.sidecar_topology_ns`, which
is already included in sealed commit. They retain observation-before-lifecycle
order. Missing owners produce no child sample. The two input stages occur
after preparation and sleep promotion and before inference, so they are separate
top-level tick stages. Training-only snapshot/configuration work is excluded.
Do not add children to their inclusive parents when calculating total time.

The existing `set_performance_measurement_enabled` flag gates every added clock
and cumulative update. `--record-performance` already enables it. With the flag
disabled, there are no new clock reads, allocations, I/O, hashes, or simulation
operations. Storage grows by thirteen `u64` counters (104 bytes) plus bounded
stack locals; there is no per-event collection or new profiler framework.

For desktop testing, use the same existing gameplay recorder command, save,
graphics, build/run mode, population, and diagnostic environment as the prior A/B.
After the receipt is written, this read-only PowerShell extraction retains the
new fields and their denominators:

```powershell
param([Parameter(Mandatory)] [string]$PerformanceReceiptPath)
$receipt = Get-Content $PerformanceReceiptPath -Raw | ConvertFrom-Json
$receipt | Select-Object source_head, build, simulation, internal_tick_stages, preparation_substages, preparation_leaf_stages, topology_observation_substages, gpu_stages, transactional_rollback_clone | ConvertTo-Json -Depth 8
```

Use the actual receipt path from that invocation. No new benchmark or training
launch is required. Cloud source checks cannot replace this desktop attribution.

## Material safeguards and release diagnostics

The user's updated tradeoff permits harmless memory discrepancies while still
requiring a stable playable game. These checks serve different purposes:

| Keep as material protection | May leave a trusted release hot path after bounded review |
| --- | --- |
| Finite scalars, candidate/vector lengths and indices, checked offsets and capacity, local/rebased domains. These prevent invalid GPU addressing, malformed payloads, or unusable numerical state. | Rebuilding an upload just created by the same private constructor solely to compare its complete contents with itself. `GpuMemoryContextUpload::try_from_finalized` validates the recall, builds rows, then `validate_against` repeats recall validation and builds identical rows again (`closed_loop_memory.rs:84–109`). |
| Organism, slot generation, profile, tick, ownership and pending-eligibility/learning transaction identity. These prevent credit or updates going to the wrong creature, brain, or tick. | Revalidating an unchanged internal frame or local upload inside a caller that just completed the same guard. The earlier memory-loop change demonstrates one safe extraction without removing public validation. |
| Save/archive schema and authenticity, restore admission, and transactional failure protection. These prevent durable corruption or partly committed authoritative state. | Recomputing diagnostic sidecar integrity hashes several times during one private mutation. Lifecycle currently clones, decays, refreshes diagnostics, and validates; its measured leaf must first distinguish this work from actual split/merge decisions. |

Public validation of externally mutable uploads and admission/restore validation
remain strict. Removing constructor self-reconstruction would trade repeated
internal bug detection for debug/tests and must retain its input and finite/bounds
checks. This diagnostic patch implements no such policy change and makes no
performance claim for those candidates.

## RAM, GPU buffers, and filesystem

Ordinary episodic recall/update uses organism-owned RAM state:
[`MemorySidecarState::recall_frame` and `observe_sealed_patch`](../../crates/alife_core/src/memory/sidecar.rs#L378).
Memory/topology core files contain no filesystem spill. Predictor, topology,
context, immutable phenotype metadata, host upload staging, compact readbacks,
and transactional/checkpoint copies also occupy RAM. Live mutable neural state
and weights belong to [GPU device storage](../../crates/alife_gpu_backend/src/closed_loop_buffers/bucket.rs#L3930);
physical device-memory residency ultimately depends on the driver. CPU metadata
and snapshots do not constitute an ordinary live CPU neural shadow.

Durable saves/checkpoints and sleep journals write filesystem copies. On baseline
`2bad5e02`, Submitted→Completed sleep transitions request a checkpoint
(`staged_tick.rs:394,1045`); nonempty sleep entries enqueue journal publication
(`:1054,1125`); journal-capacity rollover requests one
(`checkpoint_runtime.rs:80–89`); manual requests enter at
`gpu_live_runtime.rs:7926`. Without attached durability, checkpoint requests
return (`checkpoint_runtime.rs:794`). Publication uses worker threads. These
are conditional persistence events, not per-tick episodic disk reads or spills.
Source cannot establish whether the physical backing drive is an SSD.

The [plain-English memory explanation](20261001-memory-behavior.md) describes
episode fields, admission, bounded retrieval, GPU influence, and forgetting.

Smaller synchronous writes do exist. Completed TerrainVision semantic-prior
replies serialize and rewrite a bounded 128-entry cache
(`semantic_prior.rs:216–224`); GroundedObjectSlots returns before that path
(`:176–177`). Optional `ALIFE_ACTION_TRACE_PATH` writes JSONL once per observed
world tick (`graphics_capture.rs:139`), and optional graphics captures write
sidecar JSON/screenshots. Full-snapshot counters do not account for these
diagnostics. The measured run's trace/capture environment remains Unknown.

## Verification and limits

Mode 1 (Micro-Spec), independent Sol 6.1 R2 source review: no findings. Review
verified every original production line/call remains in order; all thirteen
fields have matching saturating delta and receipt mappings; disabled gating,
inclusive accounting, and observe-before-lifecycle ordering remain correct.
The CPU `gpu-runtime` library compile and strict library Clippy passed. Three
existing CPU tests passed: object-bound concepts/gaps change predecision focus;
phenotype policy and subsystem work remain causal; failed staged commits retain
the live state. A source audit verified all original lines remain in order and
all thirteen new fields have matching delta/receipt mappings. A temporary Rust
check compiled the actual metric/delta source and new receipt JSON sections,
verifying saturating counter deltas and serialization without constructing a
device. Formatting, whitespace, and 77/77 documentation assertions passed.

The compiler commands sourced `/workspace/cloud-cpu-validation/env.sh`:

```bash
cargo check -p alife_game_app --lib --no-default-features --features gpu-runtime
cargo clippy -p alife_game_app --lib --no-default-features --features gpu-runtime -- -D warnings
cargo test -p alife_game_app --lib --no-default-features --features gpu-runtime gpu_live_runtime::tests::object_bound_concepts_and_gaps_change_predecision_focus_selectively
cargo test -p alife_game_app --lib --no-default-features --features gpu-runtime gpu_live_runtime::tests::phenotype_policy_and_subsystem_work_join_are_causal
cargo test -p alife_game_app --lib --no-default-features --features gpu-runtime gpu_live_runtime::tests::failed_staged_runtime_commit_leaves_live_state_unchanged
```

The full graphical Clippy attempt
stopped at a missing Linux `libudev` development package before checking the
receipt writer; no application source error was reported by that attempt.
The temporary receipt-section compile is a narrower substitute, not a complete
Bevy frontend build. The desktop Windows build remains the full-host check.

No cloud GPU/device/hardware test, training campaign, main/CI merge, or PR was
performed. Remaining hardware attribution is Unknown and belongs to the desktop
owner. Rollback is reverting the diagnostic implementation commit.

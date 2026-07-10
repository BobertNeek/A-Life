# GPU Closed-Loop Scaling, Soak, and Promotion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Promote N512, N1024, and N2048 only after the single shared GPU brain
backend proves bounded admission, deterministic activity cost/throttling,
portable restore, populated performance, all-profile soak/replay, and complete
Slice A/B/C/D evidence with no CPU neural authority or fallback surface.

**Architecture:** Slice D extends the `BrainCapacityClass`, `BrainPhenotype`,
`GpuClosedLoopBackend`, `GpuBrainHandle`, `GpuBrainSaveState`, and Slice C
sidecar receipts already established by Slices A–C. It does not introduce a
second brain object, arena, capacity enum, save record, memory receipt, topology
receipt, or GPU owner. Admission and residency are private services inside the
shared backend; the app remains the organism-to-handle owner. Promotion is
derived from an evidence matrix, never asserted from configured class names.

**Tech Stack:** Rust 2021, wgpu 29.0.3, WGSL, serde, bytemuck, PowerShell,
real Vulkan hardware tests, existing benchmark/soak tooling.

## Global Constraints

- Slices A, B, and C and their class/profile-qualified receipts must pass first.
- Production APIs consume `BrainClassId` and `BrainCapacityClass`; legacy
  `BrainScaleTier` is restricted to migration/inspection adapters.
- `GpuClosedLoopBackend` remains the sole device/queue/pipeline/SoA owner.
- `GpuBrainHandle` remains opaque and generation checked.
- Population ceilings come from a versioned runtime/hardware profile, never
  from the capacity class and never from fixed 64/32/16 slot guesses.
- Global VRAM accounting includes immutable topology, every mutable learning
  layer, activations, candidates, memory context, diagnostics, staging, and
  compact readback.
- Every memory receipt distinguishes logical bytes committed to live brains
  from physical bytes currently allocated in retained class chunks. Removing a
  brain reclaims logical commitment immediately; physical allocation may stay
  constant for deterministic reuse. `unused_physical_bytes`, shared bytes, and
  peak physical bytes are reported separately and are never relabelled as free
  device VRAM.
- Slice C owns memory/topology pressure policy. Slice D consumes and verifies
  its receipts; it does not replace them.
- N4096 and larger legacy tiers remain inspection/export only and must not
  compile a production phenotype or allocate a GPU slot.
- Benchmark and soak rows are GPU-only. Unavailable hardware is reported as
  `Unavailable`, never filled with CPU data.
- Final promotion requires independent A/B/C/D evidence for each of N512,
  N1024, and N2048.
- Every evidence-producing command runs from a clean committed HEAD and records
  that commit plus the canonical Git tree object ID as its source-tree digest.
  After the final
  source-changing commit, and after any later merge or tracked edit that changes
  the tree, all affected A/B/C/D, benchmark, soak, and gate evidence is rerun;
  stale artifacts cannot be promoted.
- All four-word evidence/content digests reuse Slice A's domain-separated,
  typed canonical little-endian digest builder: four fixed SplitMix64 streams,
  length-prefixed UTF-8/byte slices, explicit enum discriminants, normalized
  finite floats, and stable vector order. Each record digest excludes only its
  own digest field. Git commit/tree object IDs are never substituted for content
  digests, and JSON text, `DefaultHasher`, map iteration, and display strings are
  never digest inputs.
- Slice C validation is profile-specific: grounded receipts run 10,240 ticks and
  must prove semantic-free upload, target-specific memory, and bounded
  saturation; privileged receipts run 64 ticks and prove only distinct profile
  provenance, GPU authority, internal digest validity, and no grounding claim.
  Promotion rejects either profile if it is validated against the other
  profile's behavioral contract.
- Execute the four plan files strictly A then B then C then D, with one
  implementation subagent at a time and a review gate after every task. Keep the
  committed plan files immutable during execution; track slice-qualified task
  IDs (`A1` through `D9`) in the ignored `.superpowers/sdd/progress.md` ledger so
  checkbox edits cannot dirty a clean evidence HEAD.

## Planned file structure

### Capacity, admission, and activity accounting

- Modify `crates/alife_core/src/brain_class.rs` and `phenotype.rs`.
- Create `crates/alife_core/tests/production_brain_budgets.rs`.
- Modify `crates/alife_gpu_backend/src/closed_loop_buffers.rs` and
  `closed_loop_runtime.rs`.
- Create `crates/alife_gpu_backend/src/closed_loop_admission.rs` and
  `closed_loop_activity.rs`.
- Create `crates/alife_gpu_backend/tests/closed_loop_admission.rs` and
  `closed_loop_activity.rs`.
- Create/extend `crates/alife_gpu_backend/tests/support/scaling.rs`.

### Persistence, benchmarks, soak, and promotion

- Modify `crates/alife_world/src/persistence.rs`.
- Create `crates/alife_world/tests/gpu_brain_vnext_migration.rs`.
- Modify `crates/alife_game_app/src/gpu_live_runtime.rs`,
  `live_brain_bridge.rs`, and `bin/alife_game_app.rs`.
- Create `crates/alife_game_app/src/gpu_closed_loop_promotion.rs` and export it
  from `src/lib.rs`.
- Create `crates/alife_game_app/tests/gpu_closed_loop_soak.rs` and
  `gpu_closed_loop_promotion.rs`.
- Modify `crates/alife_tools/src/benchmark.rs`,
  `src/bin/benchmark_tiers.rs`, and their tests.
- Create `crates/alife_core/src/evidence_digest.rs`; move the engine-neutral
  `PhenotypeEvidenceManifest` contract into `alife_core::phenotype` without
  changing its wire shape, and re-export it from the original app evidence
  module so Slice A-C callers remain source compatible.
- Modify `crates/alife_game_app/src/gpu_evidence.rs` for the shared Slice D
  loader and the engine-neutral manifest/digest re-export.
- Create `configs/gpu_closed_loop_performance_targets_v1.json`.
- Modify `docs/master_spec.md`, `docs/architecture_decisions.md`, and relevant
  local `AGENTS.md` files.

---

### Task 1: Make capacity and compiled budgets one production authority

**Files:**
- Modify: `crates/alife_core/src/brain_class.rs`
- Modify: `crates/alife_core/src/phenotype.rs`
- Modify: `crates/alife_core/src/error.rs`
- Modify: `crates/alife_core/src/lib.rs`
- Create: `crates/alife_core/tests/production_brain_budgets.rs`
- Modify every Slice A-C capacity caller if the final accessor/signature changes:
  `crates/alife_gpu_backend/src/closed_loop_buffers.rs`,
  `closed_loop_pipeline.rs`, `closed_loop_runtime.rs`,
  `closed_loop_learning.rs`, `closed_loop_sleep.rs`, and
  `closed_loop_memory.rs`; `crates/alife_game_app/src/gpu_live_runtime.rs`,
  `live_brain_bridge.rs`, and `bin/alife_game_app.rs`;
  `crates/alife_tools/src/benchmark.rs` and `src/bin/benchmark_tiers.rs`;
  plus every A-C focused test returned by the required caller inventory below.

**Interfaces:**
- Consumes and extends, without redefining in a second module, Slice A's final
  `BrainCapacityClass`, `BrainExecutionBudget`, `BrainPhenotype`,
  `CompiledProjection`, and `CompiledSynapse` authority.
- Produces: scaling validation over Slice A's final `CompiledBudgets`,
  `RouteBudgetReceipt`, `GlobalPhenotypeBudgetReceipt`, and exactly three
  production capacity constructors/accessors; it does not redefine them.

- [ ] **Step 1: Write failing single-authority and budget tests**

```rust
#[test]
fn production_capacity_ids_have_exact_logical_ceilings() {
    let rows = [
        (BrainCapacityClass::n512(), 512, 8_192, 64, 6_144, 1_024, 1_024),
        (BrainCapacityClass::n1024(), 1_024, 16_384, 128, 12_288, 2_048, 2_048),
        (BrainCapacityClass::n2048(), 2_048, 32_768, 192, 24_576, 4_096, 4_096),
    ];
    for (capacity, neurons, synapses, tiles, recurrent, action_decoder, memory_decoder) in rows {
        let execution = capacity.execution();
        assert_eq!(execution.max_neurons(), neurons);
        assert_eq!(execution.max_total_synapses(), synapses);
        assert_eq!(execution.max_recurrent_synapses(), recurrent);
        assert_eq!(execution.max_action_decoder_synapses(), action_decoder);
        assert_eq!(execution.max_memory_decoder_synapses(), memory_decoder);
        assert_eq!(execution.max_active_tiles(), tiles);
        assert_eq!(execution.max_candidates(), 32);
        assert_eq!(execution.max_object_slots(), 16);
        assert_eq!(execution.max_decoder_input_lanes(), 64);
        assert_eq!(execution.max_compact_readback_bytes(), 64);
        assert_eq!(execution.microstep_range(), (2, 4));
        capacity.validate_contract().unwrap();
    }
}

#[test]
fn compiled_route_and_global_receipts_cover_every_payload_once() {
    for capacity in BrainCapacityClass::production_classes() {
        let phenotype = compile_populated_fixture(&capacity, 4404);
        let budgets = phenotype.budgets();
        budgets.validate_against(&capacity).unwrap();
        assert_eq!(budgets.routes.len(), phenotype.projections().len());
        assert_eq!(budgets.sum_route_synapses(), phenotype.synapses().len() as u32);
        assert!(budgets.global.within(capacity.execution()));
        assert!(budgets.routes.iter().all(RouteBudgetReceipt::within_ceiling));
    }
}

#[test]
fn execution_abi_rejects_every_independent_limit_violation() {
    for forged_json in forged_capacity_json_cases() {
        assert!(serde_json::from_value::<BrainCapacityClass>(forged_json).is_err());
    }
}

#[test]
fn production_capacity_source_contains_no_tier_dispatch() {
    let source = include_str!("../src/brain_class.rs");
    let production = source.split("impl BrainCapacityClass").nth(1).unwrap();
    assert!(!production.contains("production_for_tier"));
}
```

Put `compile_populated_fixture` in this test file using only Slice A public
constructors; it is fixture assembly, not a production helper or CPU neural
implementation.

- [ ] **Step 2: Run and verify the scaling assertions fail before D validation**

Run: `cargo test -p alife_core --test production_brain_budgets`

Expected: at least one new exhaustive split/ABI/forged-deserialize assertion
fails until the D validation pass is wired; Slice A's record types already
exist and are not recreated here.

- [ ] **Step 3: Consume Slice A's exact capacity and compiled-budget authority**

Do not duplicate Slice A's records. `BrainCapacityClass` remains private
`{ id, execution }` with `id()`, `execution()`, `validate_contract()`, the three
canonical constructors, validating deserialize, and no `max_gpu_bytes`.
`BrainExecutionBudget` remains private/read-only through accessors for neuron,
recurrent/action-decoder/memory-decoder/total-synapse, tile, candidate,
object-slot, memory-context, replay event, replay-eligibility-sample,
decoder-input-lane, compact-readback, and microstep limits; 16x16/128x128 tile geometry; schema and
GPU layout versions; candidate feature count; storage/uniform/copy-buffer/
  copy-bytes-per-row alignment; required-limit schema and the v1 one-word
  feature mask; max
buffer/binding size; bind-group/bindings-per-group, storage/uniform-buffer,
dynamic-storage/dynamic-uniform and workgroup-storage limits; workgroup x/y/z,
invocation, and workgroups-per-dimension limits. Task 2 validates all of those
requirements against the real adapter. Canonical synapse splits are
N512 `6144/1024/1024`, N1024 `12288/2048/2048`, and N2048
`24576/4096/4096` for recurrent/action-decoder/memory-decoder respectively.

Consume Slice A's `RouteBudgetReceipt` split among recurrent,
action-decoder, and memory-decoder synapses and its
`GlobalPhenotypeBudgetReceipt` with the same split, total union, context, and
replay capacities plus decoder input lanes. Consume `CompiledBudgets { capacity_class_id,
execution_abi_digest, routes, global }`. Every route split sums exactly once to
the global union; memory/action decoder synapses are never hidden second pools.

`BrainCapacityClass` contains only `{ id, execution }`; remove the duplicated
top-level ceilings and Slice A's provisional `max_gpu_bytes`. GPU byte size is
calculated from concrete buffer layouts in Task 2 and checked against the
runtime neural heap. Route ceilings are compiler allocations derived from the
genome's normalized route shares with largest-remainder tie breaking by stable
route index. Their sums cannot exceed the global logical ceiling. Decoder
synapses are part of the global synapse count. Validate every compiled route,
feature count, record schema, alignment, required feature word, required device
limit, replay dimension, and compact-readback dimension individually and in the
aggregate. JSON negative tests independently alter each private canonical
tuple field, forge schemas, omit each required feature word, understate each
required limit, and prove typed rejection before buffer planning.

Keep `LegacyBrainClassAdapter` as the sole `BrainScaleTier` consumer. IDs 1, 2,
and 3 map to N512/N1024/N2048; every other legacy ID returns an inspection-only
classification rather than a production capacity.

- [ ] **Step 4: Migrate every A-C caller and compile the whole workspace**

Run the inventory after Slices A-C exist and retain its output in the
verification notes:

```powershell
$capacityCallers = @(rg -l "BrainCapacityClass|BrainExecutionBudget|CompiledBudgets" crates --glob '*.rs' | Sort-Object -Unique)
if ($LASTEXITCODE -gt 1 -or $capacityCallers.Count -eq 0) { throw "capacity caller inventory failed" }
$capacityCallers
```

Migrate every returned caller in core, GPU backend, app, world persistence,
tools, and all A-C tests to `capacity.id()`, `capacity.execution()`, canonical
`production_for_id`, and the final `CompiledBudgets`. Do not leave duplicate
fields or compatibility constructors. Before committing the breaking change,
run:

```powershell
cargo check --workspace --all-targets --all-features -j 1
cargo test -p alife_core --test production_brain_budgets --test phenotype_compiler --test brain_topology
```

Expected: the whole workspace compiles with no old field/signature consumer,
and all production classes/receipts pass.

- [ ] **Step 5: Commit only after the migration checks are green**

```powershell
git status --short
git diff --check
$capacityCallers = @(rg -l "BrainCapacityClass|BrainExecutionBudget|CompiledBudgets" crates --glob '*.rs' | Sort-Object -Unique)
$dirtyTracked = @(git diff --name-only | ForEach-Object { $_ -replace '/', '\\' })
$dirtyCapacityCallers = @($capacityCallers | Where-Object { $dirtyTracked -contains $_ })
if ($dirtyCapacityCallers.Count -gt 0) { git add -- $dirtyCapacityCallers }
git add -- crates/alife_core/src/brain_class.rs crates/alife_core/src/phenotype.rs crates/alife_core/src/error.rs crates/alife_core/src/lib.rs crates/alife_core/tests/production_brain_budgets.rs
git diff --cached --check
git commit -m "Enforce production brain capacity budgets"
```

Before committing, inspect `git diff --cached --name-only` and require it to
equal the intended core paths plus every dirty path in
`$dirtyCapacityCallers`. Also require no dirty caller remains in `git status`.
This prevents a partial breaking commit and does not stage unrelated paths.

### Task 2: Add private shared-backend admission and complete VRAM accounting

**Files:**
- Create: `crates/alife_gpu_backend/src/closed_loop_admission.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_buffers.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_runtime.rs`
- Modify: `crates/alife_gpu_backend/src/lib.rs`
- Create: `crates/alife_gpu_backend/tests/closed_loop_admission.rs`
- Create: `crates/alife_gpu_backend/tests/support/scaling.rs`
- Modify every `GpuClosedLoopBackend::new_required()` caller introduced by
  Slices A-C, including backend test support/runtime tests, app policy/live
  runtime/sleep/memory tests, tools benchmarks, and acceptance CLIs returned by
  the mandatory caller inventory in Step 4.

**Interfaces:**
- Consumes: one shared `GpuClosedLoopBackend`, opaque `GpuBrainHandle`,
  `BrainPhenotype`, concrete `GpuClassBucketPlan`, and a runtime profile.
- Produces: `GpuRuntimeProfile`, `GpuRuntimeBudget`,
  `GpuSlotAllocationReceipt`, `GpuAdmissionReceipt`, and generation-safe
  release accounting behind `insert_brain`/`remove_brain`.

- [ ] **Step 1: Add deterministic fixture support, then failing admission tests**

`tests/support/scaling.rs` constructs populated Slice A phenotypes and artificial
runtime profiles. First verify those fixtures with existing phenotype tests;
then add:

```rust
#[test]
fn slot_receipt_accounts_for_every_gpu_buffer_category() {
    let plan = GpuClassBucketPlan::for_phenotype(&n512_populated()).unwrap();
    let receipt = plan.slot_allocation_receipt();
    assert_eq!(receipt.logical_slot_commit_bytes,
               receipt.per_slot_component_bytes().checked_sum().unwrap());
    assert!(receipt.shared_class_bytes > 0);
    assert!(receipt.immutable_topology_bytes > 0);
    assert!(receipt.activation_bytes > 0);
    assert!(receipt.learning_bytes > 0);
    assert!(receipt.candidate_and_memory_bytes > 0);
    assert!(receipt.diagnostic_and_readback_bytes > 0);
    assert!(receipt.staging_bytes > 0);
}

#[test]
fn admission_is_runtime_budgeted_and_release_reclaims_exact_bytes() {
    let phenotype = n512_populated();
    let slot_bytes = GpuClassBucketPlan::for_phenotype(&phenotype).unwrap()
        .slot_allocation_receipt().logical_slot_commit_bytes;
    let mut backend = required_backend_with_budget(slot_bytes * 2, 2).unwrap();
    let a = backend.insert_brain(OrganismId(1), phenotype.clone()).unwrap();
    let b = backend.insert_brain(OrganismId(2), phenotype.clone()).unwrap();
    assert!(backend.insert_brain(OrganismId(3), phenotype).is_err());
    let before_release = backend.admission_receipt();
    backend.remove_brain(a).unwrap();
    let after_release = backend.admission_receipt();
    assert_eq!(before_release.logical_committed_bytes - after_release.logical_committed_bytes, slot_bytes);
    assert_eq!(before_release.physical_allocated_bytes, after_release.physical_allocated_bytes);
    assert_eq!(after_release.physical_unused_retained_bytes,
               before_release.physical_unused_retained_bytes + slot_bytes);
    let release = after_release.last_event.unwrap();
    assert_eq!(release.logical_committed_before_bytes, before_release.logical_committed_bytes);
    assert_eq!(release.logical_committed_after_bytes, after_release.logical_committed_bytes);
    assert_eq!(release.physical_allocated_before_bytes, release.physical_allocated_after_bytes);
    assert_eq!(after_release.live_brains, 1);
    let stale = finalized_memory_batch_for(a);
    let live = finalized_memory_batch_for(b);
    assert!(backend.tick_memory_batch(&stale).is_err());
    assert!(backend.tick_memory_batch(&live).is_ok());
}

#[test]
fn shared_and_retained_bytes_are_never_counted_as_logically_committed_twice() {
    let receipt = mixed_class_backend_fixture().admission_receipt();
    assert_eq!(receipt.physical_allocated_bytes,
        receipt.physical_shared_bytes
            + receipt.logical_committed_bytes
            + receipt.physical_unused_retained_bytes
            + receipt.physical_alignment_slack_bytes);
    assert!(receipt.logical_committed_bytes <= receipt.runtime.logical_neural_heap_budget_bytes);
    assert!(receipt.physical_allocated_bytes <= receipt.runtime.physical_allocation_ceiling_bytes);
    assert!(receipt.peak_physical_allocated_bytes >= receipt.physical_allocated_bytes);
}

#[test]
fn adapter_validation_rejects_each_missing_feature_limit_and_alignment() {
    let phenotype = n512_populated();
    for mutation in adapter_requirement_negative_mutations() {
        let adapter = mutation.apply_to(sufficient_adapter_fixture());
        assert!(GpuClassBucketPlan::validate_adapter(&phenotype, &adapter).is_err(),
                "{}", mutation.name());
    }
}
```

Also add a heterogeneous same-class test and a mixed N512/N1024/N2048 test.
Both require one device/queue/pipeline registry, disjoint slot ranges, exact
component accounting, and no cross-slot canary writes. Construct dispatches
through Slice C's final backend-owned `GpuClosedLoopMemoryBatchInput` and
`tick_memory_batch`, preserving finalized recall and memory-upload identity;
the admission suite must not prove isolation only through Slice A's earlier
raw `(handle, frame)` API.

- [ ] **Step 2: Run and verify accounting/admission failures**

Run: `cargo test -p alife_gpu_backend --test closed_loop_admission`

Expected: missing admission/accounting APIs.

- [ ] **Step 3: Implement exact runtime-owned records**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeProfile {
    pub schema_version: u16,
    pub profile_id: u16,
    pub logical_neural_heap_budget_bytes: u64,
    pub physical_allocation_ceiling_bytes: u64,
    pub max_hot_brains: u32,
    pub max_in_flight_batches: u16,
    pub growth_chunk_slots: u16,
    pub retain_empty_chunks: u8,
    pub reserved: [u8; 7],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeBudget {
    pub schema_version: u16,
    pub profile_id: u16,
    pub gpu_layout_version: u16,
    pub required_limits_schema_version: u16,
    pub logical_neural_heap_budget_bytes: u64,
    pub physical_allocation_ceiling_bytes: u64,
    pub max_hot_brains: u32,
    pub max_in_flight_batches: u16,
    pub growth_chunk_slots: u16,
    pub storage_alignment_bytes: u32,
    pub uniform_alignment_bytes: u32,
    pub copy_buffer_alignment_bytes: u32,
    pub copy_bytes_per_row_alignment: u32,
    pub max_buffer_size: u64,
    pub max_storage_buffer_binding_size: u64,
    pub max_bind_groups: u32,
    pub max_bindings_per_bind_group: u32,
    pub max_storage_buffers_per_shader_stage: u32,
    pub max_uniform_buffers_per_shader_stage: u32,
    pub max_dynamic_storage_buffers_per_pipeline_layout: u32,
    pub max_dynamic_uniform_buffers_per_pipeline_layout: u32,
    pub max_compute_workgroup_storage_size: u32,
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroup_size_y: u32,
    pub max_compute_workgroup_size_z: u32,
    pub max_compute_invocations_per_workgroup: u32,
    pub max_compute_workgroups_per_dimension: u32,
    pub required_feature_mask_words: u8,
    pub required_feature_mask: u64,
    pub available_feature_mask: u64,
    pub profile_digest: [u64; 4],
    pub adapter_limits_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuSlotAllocationReceipt {
    pub schema_version: u16,
    pub class_id_raw: u16,
    pub immutable_topology_bytes: u64,
    pub activation_bytes: u64,
    pub learning_bytes: u64,
    pub candidate_and_memory_bytes: u64,
    pub diagnostic_and_readback_bytes: u64,
    pub staging_bytes: u64,
    pub alignment_padding_bytes: u64,
    pub shared_class_bytes: u64,
    pub logical_slot_commit_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuAllocationEventReceipt {
    pub schema_version: u16,
    pub event_kind_raw: u16,
    pub class_id_raw: u16,
    pub handle_slot: u32,
    pub handle_generation: u32,
    pub logical_committed_before_bytes: u64,
    pub logical_committed_after_bytes: u64,
    pub physical_allocated_before_bytes: u64,
    pub physical_allocated_after_bytes: u64,
    pub physical_unused_before_bytes: u64,
    pub physical_unused_after_bytes: u64,
    pub physical_shared_before_bytes: u64,
    pub physical_shared_after_bytes: u64,
    pub event_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuAdmissionReceipt {
    pub schema_version: u16,
    pub runtime: GpuRuntimeBudget,
    pub logical_committed_bytes: u64,
    pub logical_available_bytes: u64,
    pub physical_allocated_bytes: u64,
    pub physical_unused_retained_bytes: u64,
    pub physical_shared_bytes: u64,
    pub physical_alignment_slack_bytes: u64,
    pub peak_logical_committed_bytes: u64,
    pub peak_physical_allocated_bytes: u64,
    pub live_brains: u32,
    pub max_hot_brains: u32,
    pub allocation_generation: u64,
    pub last_event: Option<GpuAllocationEventReceipt>,
}
```

All arithmetic uses `checked_add`/`checked_mul`, 256-byte storage alignment,
and concrete record sizes from the GPU ABI. Count immutable plans, activation
A/B, accumulator, encoded input, lifetime/fast/eligibility, neuron
homeostasis, candidates/features/logits, episodic context, outcome/sleep
headers, diagnostics, compact readback, double-buffered consolidation, and
alignment. Do not count the renderer reserve as available neural heap.

Before planning any allocation, compare every Slice A execution requirement
against `GpuRuntimeBudget`: schema/layout, the exact feature-mask width/value,
buffer and
binding sizes/counts, bind groups, per-stage storage/uniform counts, dynamic
buffer counts, storage/uniform/copy alignments, workgroup storage, workgroup
x/y/z and invocation limits, and workgroups per dimension. The v1 feature mask
is exactly one `u64`: both Slice A and the runtime budget require
`required_feature_mask_words == 1`; unused extension words do not exist in a
parallel D-only representation. The negative table lowers one independent value
at a time (including one feature bit at a time)
and must reject before creating a wgpu buffer or slot.

`logical_slot_commit_bytes` is the per-live-brain reservation against the
profile's logical neural heap. Actual physical growth is event-specific and is
the checked before/after difference in `GpuAllocationEventReceipt`; it can be
zero on slot reuse. Class-shared pipelines/layout metadata and shared
immutable buffers are physically counted once. `remove_brain` releases logical
commitment but may retain scrubbed physical slots. No test or receipt calls
physical allocation "free" merely because a slot is unused.
`logical_available_bytes` is derived only from the logical profile ceiling;
WebGPU does not claim global free VRAM.
Allocation event kinds are fixed: 1 admit-from-new-chunk, 2
admit-from-retained-slot, 3 release-to-retained-slot, and 4 release-and-drop-
empty-chunk. Validate `retain_empty_chunks` as 0 or 1 and require every event's
before/after values to reconcile exactly with the following aggregate receipt.

`GpuRuntimeProfile` is selected by product/runtime configuration. Do not infer
dedicated VRAM from an API that does not expose it, and do not put population
limits in `BrainCapacityClass`. `GpuClosedLoopBackend::new_required(profile)`
owns the private admission planner. No public `GpuBrainArena` exists.
Profile validation rejects zero budgets/counts, unknown schema/profile IDs,
`growth_chunk_slots=0`, invalid retain flags, logical single-slot requests over
the logical ceiling, and checked initial/growth chunk allocation over the
physical ceiling before device mutation.

- [ ] **Step 4: Migrate the constructor, run hardware isolation, and compile all
  callers before committing**

Inventory and migrate every old constructor call; do not retain a no-argument
production overload:

```powershell
$requiredCallers = @(rg -l "GpuClosedLoopBackend::new_required\(" crates --glob '*.rs' | Sort-Object -Unique)
if ($LASTEXITCODE -gt 1 -or $requiredCallers.Count -eq 0) { throw "new_required caller inventory failed" }
$requiredCallers
```

Every production caller receives a validated `GpuRuntimeProfile` from product
configuration. Test helpers use an explicit bounded fixture profile. Then run:

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_admission -- --nocapture`

Run: `cargo check --workspace --all-targets --all-features -j 1`

Expected: one Vulkan backend owns all tested slots; over-budget admission is a
typed error, valid release/reuse is generation safe, and no caller uses the old
signature.

- [ ] **Step 5: Commit**

```powershell
git status --short
git diff --check
$requiredCallers = @(rg -l "GpuClosedLoopBackend::new_required\(" crates --glob '*.rs' | Sort-Object -Unique)
$dirtyTracked = @(git diff --name-only | ForEach-Object { $_ -replace '/', '\\' })
$dirtyRequiredCallers = @($requiredCallers | Where-Object { $dirtyTracked -contains $_ })
if ($dirtyRequiredCallers.Count -gt 0) { git add -- $dirtyRequiredCallers }
git add crates/alife_gpu_backend/src/closed_loop_admission.rs crates/alife_gpu_backend/src/closed_loop_buffers.rs crates/alife_gpu_backend/src/closed_loop_runtime.rs crates/alife_gpu_backend/src/lib.rs crates/alife_gpu_backend/tests/closed_loop_admission.rs crates/alife_gpu_backend/tests/support/scaling.rs
git diff --cached --check
git commit -m "Budget shared GPU brain admission"
```

Require `git diff --cached --name-only` to equal the intended backend paths plus
every dirty path in `$dirtyRequiredCallers`. Do not leave any caller dirty or
stage unrelated files.

### Task 3: Charge executed neural work and throttle deterministically

**Files:**
- Create: `crates/alife_core/src/activity.rs`
- Modify: `crates/alife_core/src/lib.rs`
- Create: `crates/alife_gpu_backend/src/closed_loop_activity.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_pipeline.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_runtime.rs`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_recurrent.wgsl`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Modify: `crates/alife_world/src/persistence.rs`
- Create: `crates/alife_gpu_backend/tests/closed_loop_activity.rs`

**Interfaces:**
- Consumes: per-dispatch executed work counters, phenotype microstep bounds,
  compiled route priority, homeostatic BrainATP, and a recorded pressure input.
- Produces: `BrainWorkCounters`, `BrainAtpCostModel`, `GpuPressureSample`,
  `NeuralThrottleDecision`, exact ATP debit, and replayable route schedule.

- [ ] **Step 1: Write failing cost, throttle, and replay tests**

```rust
#[test]
fn repeated_microsteps_charge_repeated_executed_work() {
    let one = work_fixture(1, 512, 64, 8_192);
    let three = work_fixture(3, 512, 64, 8_192);
    assert_eq!(three.neuron_updates, one.neuron_updates * 3);
    assert_eq!(three.tile_visits, one.tile_visits * 3);
    assert_eq!(three.synapse_ops, one.synapse_ops * 3);
    assert!(cost_model().neural_cost_q24(&three).unwrap() > cost_model().neural_cost_q24(&one).unwrap());
}

#[test]
fn throttle_never_violates_microstep_floor_or_drops_essential_routes() {
    let phenotype = priority_mixed_n1024();
    let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id()).unwrap();
    for pressure in pressure_samples_all_buckets() {
        let decision = policy_v1().derive(&phenotype, capacity.execution(), pressure).unwrap();
        let (min_microsteps, max_microsteps) = capacity.execution().microstep_range();
        assert!((min_microsteps..=max_microsteps).contains(&decision.microsteps));
        assert!(contains_all(&decision.enabled_route_ids, phenotype.essential_route_ids()));
        assert!(contains_all(&decision.enabled_route_ids, phenotype.mandatory_route_ids()));
        decision.validate_for(&phenotype, capacity.execution()).unwrap();
    }
}

#[test]
fn replay_injects_recorded_pressure_and_reproduces_throttle_sequence() {
    let recorded = run_pressure_sequence_fixture(4414).unwrap();
    let replayed = replay_with_pressure_sequence(&recorded.samples).unwrap();
    assert_eq!(recorded.decisions, replayed.decisions);
    assert_eq!(recorded.work_receipts, replayed.work_receipts);
}

#[test]
fn throttle_and_work_receipts_cannot_cross_apply_between_same_class_slots() {
    let (a, b) = two_n512_slot_receipts();
    assert_ne!(a.handle_slot, b.handle_slot);
    assert!(apply_throttle_receipt_to_slot(&a, b.handle_slot, b.handle_generation).is_err());
    assert!(apply_work_receipt_to_slot(&a.work, b.handle_slot, b.handle_generation).is_err());
}

#[test]
fn every_pressure_bucket_truth_table_row_and_q24_rounding_is_exact() {
    assert_exact_pressure_truth_table(policy_v1());
    assert_eq!(policy_v1().cost_q24(&BrainWorkCounters::default()).unwrap(), 0);
    assert_eq!(q24_to_atp_q16_round_half_up(0x00000080).unwrap(), 1);
    assert!(policy_v1().cost_q24(&overflow_work_fixture()).is_err());
}
```

- [ ] **Step 2: Run and verify activity accounting is absent**

Run: `cargo test -p alife_gpu_backend --test closed_loop_activity`

Expected: missing work/cost/throttle types.

- [ ] **Step 3: Implement checked fixed-point cost semantics**

Define the portable records below in `alife_core::activity`; the GPU backend
owns their collection/dispatch implementation. This keeps
`alife_world::persistence` independent of `alife_gpu_backend` and wgpu.

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainWorkCounters {
    pub microsteps: u32,
    pub neuron_updates: u64,
    pub tile_visits: u64,
    pub synapse_ops: u64,
    pub decoder_candidate_ops: u64,
    pub memory_context_ops: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainAtpCostModel {
    pub schema_version: u16,
    pub q_fraction_bits: u8,
    pub rounding_mode_raw: u8,
    pub neuron_update_q24: u64,
    pub tile_visit_q24: u64,
    pub synapse_op_q24: u64,
    pub decoder_candidate_op_q24: u64,
    pub memory_context_op_q24: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainActivityPolicyV1 {
    pub schema_version: u16,
    pub policy_version: u16,
    pub cost: BrainAtpCostModel,
    pub gpu_time_threshold_ns: [u64; 3],
    pub queue_depth_thresholds: [u32; 3],
    pub logical_heap_pressure_thresholds_q16: [u32; 3],
    pub atp_remaining_thresholds_q16_desc: [u32; 3],
    pub policy_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeuralThrottleLevel { Full, Reduced, EssentialOnly }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuPressureSample {
    pub schema_version: u16,
    pub policy_version: u16,
    pub organism_id_raw: u64,
    pub tick: u64,
    pub class_id_raw: u16,
    pub handle_slot: u32,
    pub handle_generation: u32,
    pub sequence_cursor: u64,
    pub source_dispatch_generation: u64,
    pub source_frame_digest: [u64; 4],
    pub completed_gpu_time_ns: u64,
    pub queue_depth: u32,
    pub logical_heap_pressure_q16: u32,
    pub brain_atp_fraction_q16: u32,
    pub completed_gpu_time_bucket: u16,
    pub queue_depth_bucket: u8,
    pub neural_heap_pressure_bucket: u8,
    pub brain_atp_bucket: u8,
    pub sample_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeuralThrottleDecision {
    pub schema_version: u16,
    pub policy_version: u16,
    pub organism_id_raw: u64,
    pub tick: u64,
    pub class_id_raw: u16,
    pub handle_slot: u32,
    pub handle_generation: u32,
    pub sequence_cursor: u64,
    pub dispatch_generation: u64,
    pub frame_digest: [u64; 4],
    pub level: NeuralThrottleLevel,
    pub pressure: GpuPressureSample,
    pub microsteps: u8,
    pub enabled_route_ids: Vec<u16>,
    pub pressure_digest: [u64; 4],
    pub route_schedule_digest: [u64; 4],
    pub policy_digest: [u64; 4],
    pub decision_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainWorkReceipt {
    pub schema_version: u16,
    pub class_id_raw: u16,
    pub organism_id_raw: u64,
    pub tick: u64,
    pub handle_slot: u32,
    pub handle_generation: u32,
    pub dispatch_generation: u64,
    pub frame_digest: [u64; 4],
    pub sequence_cursor: u64,
    pub counters: BrainWorkCounters,
    pub route_schedule_digest: [u64; 4],
    pub neural_cost_q24: u64,
    pub atp_before_q16: u32,
    pub atp_debit_q16: u32,
    pub atp_after_q16: u32,
    pub receipt_digest: [u64; 4],
}
```

The one production policy is `BrainActivityPolicyV1` with schema version 1,
Q24 rates `{ neuron=32, tile=256, synapse=4, decoder_candidate=128,
memory_context=64 }`, `q_fraction_bits=24`, and rounding mode 1 (`round half
up`) when converting Q24 cost to the world's Q16 ATP ledger:

```text
cost_q24 = checked_u128_sum(counter_i * rate_i), then checked u64 conversion
atp_debit_q16 = checked((cost_q24 + 0x80) >> 8)
atp_after_q16 = checked(atp_before_q16 - atp_debit_q16)
```

No floating-point value participates in cost, pressure quantization, throttle,
save, or replay. The policy's canonical little-endian fields produce
`policy_digest`; loading a cursor with another policy version/digest is a typed
error.

Pressure buckets are left-closed/right-open except the final bucket:

| input | bucket 0 | bucket 1 | bucket 2 | bucket 3 |
|---|---|---|---|---|
| completed prior GPU time | `<2 ms` | `2..<4 ms` | `4..<8 ms` | `>=8 ms` |
| queue depth before dispatch | `0` | `1` | `2..=3` | `>=4` |
| logical heap pressure Q16 | `<32768` | `32768..<49152` | `49152..<58982` | `>=58982` |
| ATP remaining Q16 | `>=49152` | `32768..49151` | `16384..32767` | `<16384` |

Compute both fractions by checked `u128(numerator) << 16`, floor-divide by the
nonzero denominator, and clamp to 65,535; zero budget/capacity is invalid.

Let severity be the maximum of the four buckets. The complete truth table is
severity 0 -> `Full`, severity 1 -> `Reduced`, severity 2 or 3 ->
`EssentialOnly`. `Full` uses the phenotype's configured microsteps and every
due route. `Reduced` uses `max(min_microsteps, configured_microsteps - 1)` and
removes only `BiologicalPriority::NonEssential`. `EssentialOnly` uses
`min_microsteps` and retains only `BiologicalPriority::Essential` plus the
compiler-validated mandatory sensor-input, homeostasis, and motor-decoder
routes. Mandatory routes can never compile as `NonEssential`; route IDs in a
decision are sorted, unique, due on that tick, present in the phenotype, and
their schedule digest is validated by the dispatch shader header.

The sample for dispatch `g` uses the last completed GPU timing, queue depth,
logical admission pressure, and ATP visible before `g`; it binds the source
generation/frame, monotonic cursor, and sample digest. The decision binds the
target dispatch generation and final frame digest. The work receipt binds the same
generation/frame/cursor/route-schedule digest before ATP can be debited. The
backend derives counters from the validated encoded dispatch plan and commits
them only after successful GPU completion; it does not add a neural-buffer
readback or expand the <=64-byte selection readback. A
late, duplicate, skipped-cursor, mismatched-frame, mismatched-policy, or
mismatched-route receipt rejects the staged tick. All three records also bind
portable organism/tick/class plus opaque handle slot/generation. At runtime the
backend additionally validates its private nonserialized backend-instance ID;
same-class batches cannot cross-apply receipts between slots.
For the first dispatch after birth/restore when no prior timing exists, encode
`source_dispatch_generation=0`, a zero source-frame digest and time 0 (bucket
0); this bootstrap is distinct from timestamp-query unavailability, which is a
typed required-GPU error. If timing completion lags, use the most recent
completed generation named in the sample, never a wall-clock estimate.

Counters mean operations actually executed across all microsteps, not unique
neurons/tiles/synapses. Cost is a checked Q24 `u64` sum of per-operation rates.
The world charges its existing basal organism ATP once per world tick, then the
GPU neural cost once per completed neural dispatch; no basal term is duplicated.
Overflow rejects the staged tick.

Route identity comes from the compiled phenotype, not semantic tier names.
Record the raw fixed-width input, quantized pressure sample, and decision before
dispatch. Replay injects the saved samples at their exact cursors and never
reads wall-clock timing.

- [ ] **Step 4: Run core, GPU, and save roundtrip tests**

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_activity -- --nocapture`

Run: `cargo test -p alife_world --test gpu_brain_persistence`

Expected: exact cost/throttle receipts and persisted pressure/decision state
pass.

- [ ] **Step 5: Commit**

```powershell
git add crates/alife_core/src/activity.rs crates/alife_core/src/lib.rs crates/alife_gpu_backend/src/closed_loop_activity.rs crates/alife_gpu_backend/src/closed_loop_pipeline.rs crates/alife_gpu_backend/src/closed_loop_runtime.rs crates/alife_gpu_backend/shaders/closed_loop_recurrent.wgsl crates/alife_gpu_backend/tests/closed_loop_activity.rs crates/alife_game_app/src/gpu_live_runtime.rs crates/alife_world/src/persistence.rs
git commit -m "Charge and throttle executed GPU brain work"
```

### Task 4: Extend portable saves without losing Slice B or C state

**Files:**
- Modify: `crates/alife_world/src/persistence.rs`
- Create: `crates/alife_world/tests/gpu_brain_vnext_migration.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Modify: `crates/alife_game_app/tests/gpu_sleep_restore.rs`
- Modify every A-C `GpuBrainSaveState` literal/loader returned by the Step 4
  caller inventory.

**Interfaces:**
- Consumes: Slice B `GpuBrainSaveState`, all mutable GPU assets and exactly-once
  sleep state, Slice C profile/memory/topology/tracker state, Task 3 throttle
  replay state, genome/development/seed inputs, and backend provenance.
- Produces: a strict version extension of `GpuBrainSaveState`,
  fixed-width `ProductionNeuralAvailability`, and inspection-only large-tier
  migration.

- [ ] **Step 1: Write failing exhaustive roundtrip/migration tests**

```rust
#[test]
fn vnext_roundtrip_preserves_every_prior_slice_field() {
    let save = full_gpu_brain_save_fixture();
    let loaded = roundtrip(&save).unwrap();
    assert_eq!(loaded, save);
    assert_eq!(loaded.phenotype_hash, expected_fixture_phenotype_hash());
    assert_eq!(loaded.memory.compaction, save.memory.compaction);
    assert_eq!(
        loaded.memory.retained_learning,
        save.memory.retained_learning,
    );
    assert_eq!(loaded.sleep, save.sleep);
    assert_eq!(loaded.pending_eligibility, save.pending_eligibility);
    assert_eq!(loaded.throttle_replay, save.throttle_replay);
    assert_eq!(loaded.activity_policy_version, save.activity_policy_version);
    assert_eq!(loaded.activity_policy_digest, save.activity_policy_digest);
    assert_eq!(loaded.runtime_profile_digest, save.runtime_profile_digest);
}

#[test]
fn n4096_legacy_save_loads_for_inspection_without_compile_or_gpu_allocation() {
    let result = load_legacy_large_tier_for_inspection(legacy_n4096_save()).unwrap();
    assert!(matches!(result.availability, ProductionNeuralAvailability::InspectionOnly { .. }));
    assert_eq!(result.phenotype_compile_count, 0);
    assert_eq!(result.gpu_admission_count, 0);
}

#[test]
fn portable_throttle_checkpoint_contains_no_live_handle_fields() {
    let source = include_str!("../src/persistence.rs");
    let body = struct_body(source, "PortableThrottleCheckpoint");
    assert!(!body.contains("handle_slot"));
    assert!(!body.contains("handle_generation"));
    assert!(!body.contains("backend_instance"));
}
```

Add table rows for legacy IDs 1/2/3 deterministic compile with an exact
expected per-fixture hash, IDs 4+ inspection-only, unknown ID rejection, profile mismatch,
asset digest mismatch, pending eligibility, every sleep/compaction phase, and
all Task 3 throttle levels. Include Slice C retained-learning states at attempt
counts 0 through 3 and prove the sealed patch asset, pending eligibility,
exclusion state, and forced-recovery disposition survive vNext migration
without replaying memory/topology observation. Add cursor tests for duplicate, skipped, late,
wrong-frame, wrong-dispatch, wrong-policy, and sequence-asset mismatch. The
adapter display name may change without invalidating portable neural state;
required feature/limit and same-adapter replay digests may not.

- [ ] **Step 2: Run and verify missing vNext fields**

Run: `cargo test -p alife_world --test gpu_brain_vnext_migration`

Expected: compile/assertion failure for missing extension fields.

- [ ] **Step 3: Version-extend, never replace, `GpuBrainSaveState`**

Preserve every Slice B/C field and add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuBackendProvenanceSave {
    pub schema_version: u16,
    pub backend_api_raw: u16,
    pub vendor_id: u32,
    pub device_id: u32,
    pub backend_version_major: u16,
    pub backend_version_minor: u16,
    pub backend_version_patch: u16,
    pub adapter_name_len: u16,
    pub adapter_name_utf8: [u8; 128],
    pub driver_digest: [u64; 4],
    pub required_features_digest: [u64; 4],
    pub required_limits_digest: [u64; 4],
    pub available_features_digest: [u64; 4],
    pub adapter_limits_digest: [u64; 4],
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum NeuralGpuBackendApi {
    Vulkan = 1,
}

impl NeuralGpuBackendApi {
    pub const fn raw(self) -> u16;
    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError>;
    pub fn try_from_slug(slug: &str) -> Result<Self, ScaffoldContractError>;
    pub const fn slug(self) -> &'static str;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableThrottleCheckpoint {
    pub schema_version: u16,
    pub policy_version: u16,
    pub organism_id_raw: u64,
    pub tick: u64,
    pub class_id_raw: u16,
    pub sequence_cursor: u64,
    pub dispatch_generation: u64,
    pub frame_digest: [u64; 4],
    pub source_dispatch_generation: u64,
    pub source_frame_digest: [u64; 4],
    pub completed_gpu_time_ns: u64,
    pub queue_depth: u32,
    pub logical_heap_pressure_q16: u32,
    pub brain_atp_fraction_q16: u32,
    pub level: NeuralThrottleLevel,
    pub microsteps: u8,
    pub enabled_route_ids: Vec<u16>,
    pub route_schedule_digest: [u64; 4],
    pub work: BrainWorkCounters,
    pub neural_cost_q24: u64,
    pub atp_before_q16: u32,
    pub atp_debit_q16: u32,
    pub atp_after_q16: u32,
    pub policy_digest: [u64; 4],
    pub portable_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThrottleReplaySaveState {
    pub schema_version: u16,
    pub policy_version: u16,
    pub next_sequence_cursor: u64,
    pub last_committed_sequence_cursor: Option<u64>,
    pub policy_digest: [u64; 4],
    pub sequence_digest: [u64; 4],
    pub sequence_asset: GpuBrainAssetRef,
    pub last_checkpoint: Option<PortableThrottleCheckpoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionNeuralAvailability {
    ReadyGpu {
        class_id_raw: u16,
        runtime_profile_digest: [u64; 4],
        adapter_limits_digest: [u64; 4],
    },
    InspectionOnly {
        legacy_class_id_raw: u16,
        reason_code: u16,
    },
    Unavailable {
        class_id_raw: u16,
        reason_code: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionOnlyLegacyBrainState {
    pub source_schema: u16,
    pub legacy_class_id_raw: u16,
    pub raw_brain_asset: GpuBrainAssetRef,
    pub inspection_reason_code: u16,
    pub canonical_digest: [u64; 4],
}
```

Add these exact fields to the Slice B/C `GpuBrainSaveState`; do not create a
wrapper save type:

```rust
pub backend_provenance: GpuBackendProvenanceSave,
pub runtime_profile_id: u16,
pub runtime_profile_digest: [u64; 4],
pub activity_policy_version: u16,
pub activity_policy_digest: [u64; 4],
pub throttle_replay: ThrottleReplaySaveState,
```

All Slice B fields (`phenotype_hash`, capacity, mutable assets/generations,
checkpoint tick, learning sequence, pending eligibility, sleep, replay and
staging assets) and Slice C fields (sensor profile, memory bank/compaction,
topology, tracked-object registry) remain byte-for-byte represented and covered
by equality tests. `next_sequence_cursor` is the only cursor accepted for the
next dispatch after restore; it advances only when the bound work/ATP receipt
commits.
Keep Slice B's singular canonical
`phenotype_compiler_inputs: GpuBrainAssetRef` as the only compiler-input
authority. D resolves that asset as A's validated `PhenotypeCompilerInputs`,
recomputes its digest, recompiles the immutable phenotype, and requires exact
equality; it does not add a second genome/development/seed DTO or similarly
named save field. If a future compiler schema needs another manifest version,
it must schema-bump that canonical core asset and migrate it in one place.

The sequence asset contains `PortableThrottleCheckpoint` records, never runtime
`GpuPressureSample`, `NeuralThrottleDecision`, `BrainWorkReceipt`, backend
instance IDs, handle slots, or handle generations. Before saving, validate the
runtime slot binding, then canonicalize only organism/tick/class/cursor/
dispatch/frame/policy/route/work data. Restore validates the portable digest and
rebinds it to the newly allocated opaque handle, regenerating runtime receipt
digests. A checkpoint cannot itself authorize a backend call or cross-apply to
another current slot.

Do not serialize current execution availability, live handles, wgpu objects,
or an unavailable/fallback result. Load validates portable data first, then
recomputes availability against current production IDs, features, adapter, and
runtime budget. N4096+ inspection uses a separate DTO and never calls
`PhenotypeCompiler::compile` or `insert_brain`.
`adapter_name_utf8` is bounded human provenance only: it is emitted in reports
but is excluded from phenotype identity, policy identity, save compatibility,
and promotion equality. Required features/limits and the same-adapter replay
receipt provide the machine checks.
Vendor/device/backend, driver, and available-feature/limit fields are also
recorded provenance; ordinary restore validates the saved *required*
feature/limit digests against the current adapter, while a same-adapter replay
claim additionally requires all recorded adapter identity/provenance digests to
match. No display string decides compatibility.
`NeuralGpuBackendApi` has custom deserialization through `try_from_raw`; the
only v1 mapping is raw `1` <-> exact lowercase slug `vulkan`. Every
`backend_api_raw` lane in save, soak, benchmark, gate, and promotion DTOs uses
this mapping, and every A-C `GpuHardwareReceipt.backend_api` string is parsed
through `try_from_slug` before adapter identity is computed. Unknown case,
spelling, or numeric values are rejected rather than hashed as a new backend.
Require `adapter_name_len <= 128`, strict UTF-8 in the used prefix, and zero
padding after the prefix. Availability reason codes are fixed: 1 unsupported
production class, 2 required feature missing, 3 required limit/alignment
missing, 4 runtime admission budget exceeded, and 5 adapter/device unavailable.

- [ ] **Step 4: Run real restore and migration matrices**

```powershell
$saveCallers = @(rg -l "GpuBrainSaveState" crates --glob '*.rs' | Sort-Object -Unique)
if ($LASTEXITCODE -gt 1 -or $saveCallers.Count -eq 0) { throw "save caller inventory failed" }
$saveCallers
```

Migrate every returned literal/loader; no defaulted field may silently erase
prior slice state.

Run: `cargo test -p alife_world --test gpu_brain_vnext_migration --test gpu_brain_persistence --test gpu_memory_grounding_persistence`

Run: `cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test gpu_sleep_restore -j 1 -- --nocapture`

Run: `cargo check --workspace --all-targets --all-features -j 1`

Expected: all phases restore exactly once and large tiers remain inspection
only.

- [ ] **Step 5: Commit**

```powershell
$saveCallers = @(rg -l "GpuBrainSaveState" crates --glob '*.rs' | Sort-Object -Unique)
$dirtyTracked = @(git diff --name-only | ForEach-Object { $_ -replace '/', '\\' })
$dirtySaveCallers = @($saveCallers | Where-Object { $dirtyTracked -contains $_ })
if ($dirtySaveCallers.Count -gt 0) { git add -- $dirtySaveCallers }
git add crates/alife_world/src/persistence.rs crates/alife_world/tests/gpu_brain_vnext_migration.rs crates/alife_game_app/src/gpu_live_runtime.rs crates/alife_game_app/tests/gpu_sleep_restore.rs
git diff --cached --check
git commit -m "Extend portable GPU brain saves"
```

Inspect the cached paths and require no dirty `$saveCallers` entry remains.

### Task 5: Benchmark populated causal brains with honest statuses

**Files:**
- Create: `crates/alife_core/src/evidence_digest.rs`
- Modify: `crates/alife_core/src/phenotype.rs`
- Modify: `crates/alife_core/src/lib.rs`
- Create: `configs/gpu_closed_loop_performance_targets_v1.json`
- Modify: `crates/alife_tools/src/benchmark.rs`
- Modify: `crates/alife_tools/src/bin/benchmark_tiers.rs`
- Modify: `crates/alife_tools/tests/benchmark_tiers.rs`
- Modify: `crates/alife_game_app/src/gpu_evidence.rs`
- Modify: `crates/alife_game_app/src/bin/alife_game_app.rs`

**Interfaces:**
- Consumes: production phenotypes, shared backend, both sensor profiles,
  runtime admission receipts, populated world candidates, sealed outcomes, and
  exact performance targets.
- Produces: `GpuClosedLoopBenchmarkRow`, smoke and manual benchmark manifests,
  `GpuClosedLoopBenchmarkProtocolV1`, row-local environment/adapter bindings,
  embedded phenotype manifests, and Completed/Missed/Unavailable statuses
  without CPU substitutions.

- [ ] **Step 1: Write failing row/cardinality/causality tests**

```rust
#[test]
fn benchmark_matrix_has_exact_rows_and_no_cpu_fields() {
    let manifest = benchmark_manifest_fixture();
    assert_eq!(manifest.rows.len(), 3 * 2 * 6);
    assert!(manifest.rows.iter().all(|row| row.population > 0));
    let json = serde_json::to_string(&manifest).unwrap();
    assert!(!json.contains("cpu_shadow"));
    assert!(!json.contains("cpu_fallback"));
    assert!(!json.contains("cpu_ms"));
}

#[test]
fn completed_rows_prove_real_causal_work() {
    for row in benchmark_completed_rows_fixture() {
        assert!(row.environment.adapter.is_some());
        assert!(row.admission.is_some());
        assert_eq!(
            row.phenotype_manifest.manifest_digest,
            row.phenotype_manifest_digest,
        );
        assert_eq!(row.gpu_selections, row.executed_actions);
        assert_eq!(row.executed_actions, row.sealed_patches);
        assert_eq!(row.raw_inference_timestamp_ticks.len(), 1_024);
        assert_eq!(row.raw_plasticity_timestamp_ticks.len(), 1_024);
        assert_eq!(row.raw_neural_tick_ns.len(), 1_024);
        assert!(row.learning_commits > 0);
        assert!(row.distinct_selected_families >= 2);
        assert!(row.active_synapses > 0);
    }
}

#[test]
fn unavailable_rows_are_honest_and_do_not_forge_adapter_or_admission() {
    let row = benchmark_no_adapter_row_fixture();
    assert!(matches!(row.status, GpuBenchmarkStatus::Unavailable { reason_code: 1 }));
    assert!(row.environment.adapter.is_none());
    assert!(row.admission.is_none());
    assert!(row.measured_p95_ns.is_none());
    assert!(row.raw_inference_timestamp_ticks.is_empty());
    assert!(row.raw_plasticity_timestamp_ticks.is_empty());
    assert!(row.raw_neural_tick_ns.is_empty());
    assert_eq!(row.phenotype_manifest.manifest_digest, row.phenotype_manifest_digest);
}
```

Also assert zero rows is failure, every expected key occurs exactly once, and
`Missed` is derived only from an executed row exceeding its exact target. Add
nearest-rank p95 tests for boundary/equal samples, exact sample cardinality,
warmup exclusion, row-order independence, and timestamp-unavailable status.
Add tamper cases for the embedded phenotype manifest, row adapter/environment
digest, optional admission rules, manifest adapter disagreement, row/manifest
content digests, and the fixed base seed.

- [ ] **Step 2: Run and verify the old procedural fixture fails**

Run: `cargo test -p alife_tools --test benchmark_tiers`

Expected: failure because existing rows do not prove populated causal work.

- [ ] **Step 3: Add exact versioned targets and statuses**

The JSON target table contains both profiles and these p95 neural-tick targets
in milliseconds for populations `[1,10,50,100,250,500]`:

```text
N512:  [2, 4, 8, 12, 25, 50]
N1024: [3, 6, 12, 20, 40, 80]
N2048: [4, 8, 20, 35, 70, 140]
```

The same class table applies independently to each of the two profile IDs,
producing 36 exact target rows.

They are performance gates, not inferred claims. `Completed` means the row ran
and met target; `Missed` means it ran and exceeded target; `Unavailable` means
the required GPU/profile could not admit the row and records the typed reason.
No status is synthesized from absent rows.

Each populated row must enumerate candidates, select on GPU, execute actions,
seal patches, apply at least one learning update, and select at least two action
families over the sample. Use the unconditional `alife_gpu_backend` dependency;
do not pass a nonexistent `alife_tools --features gpu-runtime` flag.
The benchmark manifest records schema version, clean Git commit, and canonical
source-tree digest shared by every row.

Use these portable shapes (bounded vectors contain fixed-width entries):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuClosedLoopBenchmarkProtocolV1 {
    pub schema_version: u16,
    pub protocol_version: u16,
    pub warmup_ticks: u32,
    pub measured_ticks: u32,
    pub samples_per_tick: u16,
    pub nearest_rank_percentile: u16,
    pub timestamp_scope_raw: u16,
    pub base_seed: u64,
    pub protocol_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuBenchmarkStatus {
    Completed,
    Missed,
    Unavailable { reason_code: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuBenchmarkEnvironmentReceipt {
    pub schema_version: u16,
    pub availability_reason_code: u16,
    pub adapter: Option<GpuBackendProvenanceSave>,
    pub adapter_identity_digest_or_zero: [u64; 4],
    pub environment_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuClosedLoopBenchmarkRow {
    pub schema_version: u16,
    pub class_id_raw: u16,
    pub sensor_profile_id_raw: u16,
    pub sensor_profile_schema: u16,
    pub sensory_abi_raw: u16,
    pub population: u32,
    pub fixture_seed: u64,
    pub phenotype_hash: PhenotypeHash,
    pub phenotype_manifest: PhenotypeEvidenceManifest,
    pub phenotype_manifest_digest: [u64; 4],
    pub capacity_digest: [u64; 4],
    pub runtime_profile_digest: [u64; 4],
    pub activity_policy_digest: [u64; 4],
    pub protocol_digest: [u64; 4],
    pub target_p95_ns: u64,
    pub measured_p95_ns: Option<u64>,
    pub timestamp_period_ns_q24: u64,
    pub raw_inference_timestamp_ticks: Vec<u64>,
    pub raw_plasticity_timestamp_ticks: Vec<u64>,
    pub raw_neural_tick_ns: Vec<u64>,
    pub environment: GpuBenchmarkEnvironmentReceipt,
    pub admission: Option<GpuAdmissionReceipt>,
    pub gpu_selections: u64,
    pub executed_actions: u64,
    pub sealed_patches: u64,
    pub learning_commits: u64,
    pub distinct_selected_families: u16,
    pub active_synapses: u32,
    pub status: GpuBenchmarkStatus,
    pub row_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuClosedLoopBenchmarkManifest {
    pub schema_version: u16,
    pub git_commit: String,
    pub source_tree_digest: String,
    pub adapter: Option<GpuBackendProvenanceSave>,
    pub adapter_identity_digest_or_zero: [u64; 4],
    pub protocol: GpuClosedLoopBenchmarkProtocolV1,
    pub rows: Vec<GpuClosedLoopBenchmarkRow>,
    pub manifest_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuPerformanceTargetRowV1 {
    pub class_id_raw: u16,
    pub sensor_profile_id_raw: u16,
    pub population: u32,
    pub target_p95_ns: u64,
}
```

The target JSON is schema 1 and contains exactly 36 sorted
`GpuPerformanceTargetRowV1` values plus the protocol digest; duplicate, missing,
unknown-class/profile/population, zero, or extra rows are errors. The benchmark
manifest commit/tree strings are strict lowercase 40-hex Git object IDs.
All organisms in a row use the same compiled populated phenotype fixture;
`phenotype_hash`, `phenotype_manifest_digest`, and `capacity_digest` are
recomputed/validated before admission and cover every slot in that row. Move
the engine-neutral `PhenotypeEvidenceManifest` and canonical evidence-digest
builder from the app module into `alife_core`; the app re-exports the unchanged
type so A-C wire formats and imports do not fork. Every benchmark row embeds the
full manifest rather than trusting a producer-supplied digest.

Each private child records its own environment. `Completed` and `Missed`
require `environment.adapter` and `admission` to be present, mutually
consistent, and bound into `row_digest`. `Unavailable` reason codes are exactly
1 no adapter/device, 2 missing required feature/limit/timestamp support, and 3
runtime admission budget exceeded. A no-adapter row stores neither adapter nor
admission; a feature/admission-unavailable row may store the observed adapter
and current admission snapshot but cannot invent a successful admission. The
matrix manifest stores `adapter = Some(...)` only when every observed adapter
identity agrees; all 36 passing promotion rows must name that one exact adapter.
Display names remain provenance and do not enter adapter identity.

The protocol is exactly: base seed 4404; one fresh process/backend per row;
deterministic row seed
`4404 ^ (class_id << 48) ^ (profile_id << 32) ^ population`; 256
unmeasured production ticks followed immediately by 1,024 measured production
ticks; one logical timing sample per complete population neural tick. Each
sample is the checked sum of two non-overlapping GPU timestamp spans: inference
starts before the first encoder pass and ends after decoder selection plus
eligibility accumulation; post-outcome plasticity starts after the sealed
outcome-credit upload and ends after the plasticity diagnostics/commit pass.
CPU candidate enumeration, compact readback mapping, world execution, patch
sealing, queue idle between spans, file I/O, and sleep-only jobs are outside
both spans. `raw_inference_timestamp_ticks` and
`raw_plasticity_timestamp_ticks` each contain exactly 1,024 deltas for an
executed row; `raw_neural_tick_ns[i]` is the checked nanosecond conversion of
each delta followed by checked addition. `timestamp_scope_raw` is exactly 2
for this split-span v1 protocol. Resolve timestamp queries asynchronously after
measurement without `device.poll(Wait)` inside either span. Require WebGPU
timestamp-query support and record the adapter timestamp period; otherwise the
row is `Unavailable` with zero samples. Sort all 1,024 `u64` nanosecond samples
and choose nearest rank `ceil(0.95 * 1024) - 1 = 972` (zero-based). No trimming,
averaging, interpolation, repetition selection, or CPU stopwatch substitution
is allowed.
Convert the finite positive adapter timestamp period to Q24 nanoseconds once by
round-half-up; reject zero, NaN, infinity, or overflow. Convert each inference
and plasticity timestamp delta independently with checked
`u128(delta_ticks) * period_ns_q24`, add `1 << 23`, shift right 24, and
checked-convert to `u64` nanoseconds; then checked-add the pair. This is the
sole timing rounding path. A continuous span across the CPU/world gap or a CPU
stopwatch substitute is invalid.
The benchmark phenotype/world fixture starts fully rested and uses a validated
production endocrine profile whose fatigue threshold cannot enter sleep within
1,280 ticks at the row's bounded work cost. A surprise non-awake tick aborts
the benchmark command without a manifest; it is not a skipped sample or an
`Unavailable` substitution. Each executed row therefore has
exactly 1,024 samples.
The matrix CLI is an orchestrator: it stable-sorts the expected keys and spawns
the same binary's private `--single-row` child once per key, passing an exact
temporary row path. It validates each child receipt, deletes staging on any
failure, then atomically assembles the manifest. The private mode rejects more
than one key and cannot write the final manifest directly.
It rejects a child whose adapter/environment digest differs from the manifest's
single adapter, whose embedded phenotype manifest does not recompute exactly,
or whose row digest does not cover every raw timing sample and optional field.
`protocol_digest`, each `environment_digest`, each `row_digest`, and
`manifest_digest` use the shared typed canonical digest implementation and
exclude only their own field.
Expose `benchmark_tiers --validate <manifest-path>` as a read-only validator
over the same code; it reruns no benchmark and rewrites no artifact.

- [ ] **Step 4: Run focused tests against the final benchmark source**

Run: `cargo test -p alife_tools --test benchmark_tiers`

Run: `cargo check --workspace --all-targets --all-features -j 1`

Expected: the manifest/digest ownership move and every A-C re-export compile,
and benchmark tests are green immediately before the commit. Any fix repeats
this step.

- [ ] **Step 5: Commit the benchmark implementation**

```powershell
git add crates/alife_core/src/evidence_digest.rs crates/alife_core/src/phenotype.rs crates/alife_core/src/lib.rs crates/alife_game_app/src/gpu_evidence.rs configs/gpu_closed_loop_performance_targets_v1.json crates/alife_tools/src/benchmark.rs crates/alife_tools/src/bin/benchmark_tiers.rs crates/alife_tools/tests/benchmark_tiers.rs crates/alife_game_app/src/bin/alife_game_app.rs
git commit -m "Benchmark populated GPU brain classes"
```

- [ ] **Step 6: Require clean committed HEAD, then emit smoke and full artifacts**

```powershell
if (git status --short) { throw "benchmark evidence requires clean HEAD" }
$benchmarkHead = git rev-parse HEAD
$benchmarkTree = git rev-parse 'HEAD^{tree}'
cargo run -p alife_tools --bin benchmark_tiers -- --backend gpu-closed-loop --base-seed 4404 --populations 1,10 --classes n512,n1024,n2048 --sensor-profiles privileged-affordance-v1,grounded-object-slots-v1 --targets configs/gpu_closed_loop_performance_targets_v1.json --output target/artifacts/gpu-closed-loop-benchmark-smoke.json
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo run -p alife_tools --bin benchmark_tiers -- --backend gpu-closed-loop --base-seed 4404 --populations 1,10,50,100,250,500 --classes n512,n1024,n2048 --sensor-profiles privileged-affordance-v1,grounded-object-slots-v1 --targets configs/gpu_closed_loop_performance_targets_v1.json --output target/artifacts/gpu-closed-loop-benchmark-full.json
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
```

Expected: smoke has exactly 12 honest rows and full has exactly 36; unavailable
populations remain explicit and are not backfilled. Both headers equal
`$benchmarkHead`/`$benchmarkTree`. These artifacts are provisional until the
mandatory final-HEAD refresh in Task 9.

### Task 6: Prove bounded all-profile soak and deterministic replay for every class

**Files:**
- Create: `crates/alife_game_app/tests/gpu_closed_loop_soak.rs`
- Modify: `crates/alife_game_app/src/gpu_evidence.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Modify: `crates/alife_game_app/src/bin/alife_game_app.rs`

**Interfaces:**
- Consumes: A/B/C receipts and runtime paths, Task 1/2 budget receipts, Task 3
  pressure replay, Task 4 save restore, and Slice C bounded sidecars.
- Produces: six `GpuClosedLoopSoakReceipt` artifacts keyed by profile/class and
  same-adapter tolerance replay for all three classes, using Slice C's final
  `TopologyCapacityReceipt` and the shared profiled evidence header/loader.

- [ ] **Step 1: Write failing complete-bound assertions**

```rust
#[test]
fn ten_thousand_tick_receipt_proves_every_bound_and_gpu_authority() {
    let receipt = run_soak(test_soak_options()).unwrap();
    assert_eq!(receipt.header.common.slice_raw, 4);
    assert_eq!(receipt.header.common.status_raw, 1);
    assert_eq!(
        receipt.header.common.phenotype_manifest_digest,
        receipt.phenotype_manifest.manifest_digest,
    );
    assert_eq!(
        receipt.header.common.artifact_digest,
        receipt.recompute_artifact_digest().unwrap(),
    );
    assert_eq!(receipt.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
    assert_eq!(receipt.completed_ticks, 10_240);
    assert!(receipt.truncation.max_candidates <= receipt.capacity.execution().max_candidates());
    assert!(receipt.truncation.max_object_slots <= receipt.capacity.execution().max_object_slots());
    assert!(receipt.truncation.max_decoder_input_lanes <= receipt.capacity.execution().max_decoder_input_lanes());
    assert!(receipt.memory.final_record_count <= receipt.memory.capacity);
    assert!(receipt.topology.capacity.contains(
        receipt.topology.final_counts,
        receipt.topology.max_observed_bindings_per_kind,
    ));
    assert!(receipt.route_budgets.iter().all(RouteBudgetReceipt::within_ceiling));
    assert!(receipt.global_budget.within(receipt.capacity.execution()));
    assert!(receipt.admission.peak_logical_committed_bytes <= receipt.admission.logical_budget_bytes);
    assert!(receipt.admission.peak_physical_allocated_bytes <= receipt.admission.physical_ceiling_bytes);
    assert_eq!(receipt.admission.post_warmup_physical_min_bytes,
               receipt.admission.post_warmup_physical_max_bytes);
    assert_eq!(receipt.admission.post_warmup_logical_min_bytes,
               receipt.admission.post_warmup_logical_max_bytes);
    assert!(receipt.process_memory.rss_high_water_bytes <= receipt.process_memory.rss_budget_bytes);
    assert!(receipt.process_memory.post_warmup_growth_bytes <= receipt.process_memory.growth_envelope_bytes);
    assert!(receipt.gpu_selections > 0);
    assert!(receipt.activity.learning_commits > 0);
    assert!(receipt.save_restore.sleep_cycles > 0);
    assert!(receipt.save_restore.restore_receipts.iter().all(|r| r.passed));
    assert!(receipt.activity.raw_dispatch_samples.iter().all(DispatchAccountingSample::bindings_match));
    assert_eq!(receipt.activity.raw_dispatch_samples.len() as u64,
               receipt.authoritative_gpu_dispatches);
    assert_eq!(receipt.admission.raw_samples.len(), 157);
    assert_eq!(receipt.process_memory.raw_samples.len(), 157);
    assert_eq!(receipt.replay.raw_comparisons.len() as u64,
               receipt.replay.compared_dispatches);
    assert_eq!(receipt.policy_switch.switch_count, 0);
    assert_eq!(receipt.terminal_capacity_errors, 0);
}
```

Add assertions for topology concepts/edges/simplexes/gaps separately, memory
merge/eviction/compaction activity, candidate/context truncation receipts,
compact readback <=64 bytes, no device-policy switch, save/restore during one
sleep phase, typed migration receipts, and injected pressure sequence equality
in replay. Assert every raw sample tick/cursor is strictly increasing and every
subreceipt digest contributes to the top-level canonical digest.

- [ ] **Step 2: Run and verify the comprehensive receipt is absent**

Run: `cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test gpu_closed_loop_soak -j 1 -- --nocapture`

Expected: missing runner/receipt fields.

- [ ] **Step 3: Implement bounded soak semantics**

Use one explicit top-level receipt composed only from typed subreceipts; do not
flatten unrelated counters into ad-hoc JSON maps:

Reuse Slice C's final public `TopologyCapacityReceipt` exactly; Slice D does
not redeclare it. That shared record includes concepts, edges, simplexes,
unresolved gaps, and per-kind binding ceilings plus `contains` validation.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuAllocationSample {
    pub tick: u64,
    pub logical_committed_bytes: u64,
    pub physical_allocated_bytes: u64,
    pub physical_unused_retained_bytes: u64,
    pub physical_shared_bytes: u64,
    pub physical_alignment_slack_bytes: u64,
    pub peak_logical_committed_bytes: u64,
    pub peak_physical_allocated_bytes: u64,
    pub allocation_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionSoakReceipt {
    pub schema_version: u16,
    pub logical_budget_bytes: u64,
    pub physical_ceiling_bytes: u64,
    pub peak_logical_committed_bytes: u64,
    pub peak_physical_allocated_bytes: u64,
    pub post_warmup_logical_min_bytes: u64,
    pub post_warmup_logical_max_bytes: u64,
    pub post_warmup_physical_min_bytes: u64,
    pub post_warmup_physical_max_bytes: u64,
    pub raw_events: Vec<GpuAllocationEventReceipt>,
    pub raw_samples: Vec<GpuAllocationSample>,
    pub samples_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRssSample {
    pub tick: u64,
    pub rss_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessMemorySoakReceipt {
    pub schema_version: u16,
    pub rss_budget_bytes: u64,
    pub rss_high_water_bytes: u64,
    pub growth_envelope_bytes: u64,
    pub post_warmup_growth_bytes: u64,
    pub first_quartile_mean_bytes: u64,
    pub last_quartile_mean_bytes: u64,
    pub raw_samples: Vec<ProcessRssSample>,
    pub samples_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchAccountingSample {
    pub tick: u64,
    pub pressure: GpuPressureSample,
    pub throttle: NeuralThrottleDecision,
    pub work: BrainWorkReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivitySoakReceipt {
    pub schema_version: u16,
    pub activity_policy_version: u16,
    pub activity_policy_digest: [u64; 4],
    pub total_work: BrainWorkCounters,
    pub total_neural_cost_q24: u64,
    pub total_atp_debit_q16: u64,
    pub full_dispatches: u64,
    pub reduced_dispatches: u64,
    pub essential_only_dispatches: u64,
    pub learning_commits: u64,
    pub raw_dispatch_samples: Vec<DispatchAccountingSample>,
    pub sequence_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemorySoakReceipt {
    pub schema_version: u16,
    pub capacity: u32,
    pub final_record_count: u32,
    pub merges: u64,
    pub evictions: u64,
    pub compactions: u64,
    pub raw_updates: Vec<MemoryUpdateReceipt>,
    pub raw_compactions: Vec<MemoryCompactionReceipt>,
    pub receipts_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologySoakReceipt {
    pub schema_version: u16,
    pub capacity: TopologyCapacityReceipt,
    pub final_counts: TopologyCounts,
    pub max_observed_bindings_per_kind: u32,
    pub degradations: u64,
    pub raw_observations: Vec<TopologyObservationReceipt>,
    pub receipts_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruncationEventReceipt {
    pub tick: u64,
    pub kind_raw: u16,
    pub requested: u32,
    pub retained: u32,
    pub dropped: u32,
    pub input_digest: [u64; 4],
    pub output_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruncationSoakReceipt {
    pub schema_version: u16,
    pub max_candidates: u16,
    pub max_object_slots: u16,
    pub max_memory_context_records: u16,
    pub max_decoder_input_lanes: u16,
    pub compact_readback_bytes: u32,
    pub candidate_truncations: u64,
    pub object_slot_truncations: u64,
    pub memory_context_truncations: u64,
    pub topology_binding_truncations: u64,
    pub raw_events: Vec<TruncationEventReceipt>,
    pub events_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveMigrationReceipt {
    pub source_schema: u16,
    pub target_schema: u16,
    pub legacy_class_id_raw: u16,
    pub classification_raw: u16,
    pub phenotype_compile_count: u32,
    pub gpu_admission_count: u32,
    pub phenotype_hash_or_zero: [u64; 4],
    pub receipt_digest: [u64; 4],
    pub passed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveRestoreReceipt {
    pub save_tick: u64,
    pub restore_tick: u64,
    pub sleep_phase_raw: u16,
    pub consolidation_state_raw: u16,
    pub expected_remaining_swaps: u16,
    pub observed_remaining_swaps: u16,
    pub pre_save_state_digest: [u64; 4],
    pub post_restore_state_digest: [u64; 4],
    pub receipt_digest: [u64; 4],
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveRestoreSoakReceipt {
    pub schema_version: u16,
    pub sleep_cycles: u64,
    pub save_count: u32,
    pub restore_count: u32,
    pub restore_receipts: Vec<SaveRestoreReceipt>,
    pub migration_receipts: Vec<SaveMigrationReceipt>,
    pub receipts_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicySwitchEventReceipt {
    pub tick: u64,
    pub from_policy_raw: u16,
    pub to_policy_raw: u16,
    pub reason_code: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicySwitchSoakReceipt {
    pub schema_version: u16,
    pub initial_policy_raw: u16,
    pub final_policy_raw: u16,
    pub switch_count: u32,
    pub raw_events: Vec<PolicySwitchEventReceipt>,
    pub events_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayComparisonSample {
    pub sequence_cursor: u64,
    pub dispatch_generation: u64,
    pub source_candidate_index: u16,
    pub replay_candidate_index: u16,
    pub max_abs_logit_delta_f32_bits: u32,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SameAdapterReplayReceipt {
    pub schema_version: u16,
    pub vendor_id: u32,
    pub device_id: u32,
    pub backend_api_raw: u16,
    pub driver_digest: [u64; 4],
    pub feature_digest: [u64; 4],
    pub limits_digest: [u64; 4],
    pub checkpoint_digest: [u64; 4],
    pub pressure_sequence_digest: [u64; 4],
    pub source_selection_digest: [u64; 4],
    pub replay_selection_digest: [u64; 4],
    pub source_work_digest: [u64; 4],
    pub replay_work_digest: [u64; 4],
    pub compared_dispatches: u64,
    pub selection_mismatches: u32,
    pub logit_tolerance_f32_bits: u32,
    pub max_abs_logit_delta_f32_bits: u32,
    pub logit_tolerance_violations: u32,
    pub raw_comparisons: Vec<ReplayComparisonSample>,
    pub comparisons_digest: [u64; 4],
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuClosedLoopSoakReceipt {
    #[serde(flatten)]
    pub header: ProfiledBehaviorReceiptHeader,
    pub phenotype_manifest: PhenotypeEvidenceManifest,
    pub sensor_profile: SensorProfileIdentity,
    pub capacity_class_slug: String,
    pub policy_backend: PolicyBackend,
    pub adapter: GpuBackendProvenanceSave,
    pub capacity: BrainCapacityClass,
    pub completed_ticks: u64,
    pub route_budgets: Vec<RouteBudgetReceipt>,
    pub global_budget: GlobalPhenotypeBudgetReceipt,
    pub admission: AdmissionSoakReceipt,
    pub process_memory: ProcessMemorySoakReceipt,
    pub activity: ActivitySoakReceipt,
    pub memory: MemorySoakReceipt,
    pub topology: TopologySoakReceipt,
    pub truncation: TruncationSoakReceipt,
    pub save_restore: SaveRestoreSoakReceipt,
    pub policy_switch: PolicySwitchSoakReceipt,
    pub gpu_selections: u64,
    pub authoritative_gpu_dispatches: u64,
    pub terminal_capacity_errors: u64,
    pub replay: SameAdapterReplayReceipt,
}
```

The flattened shared header uses Slice D = 4 and passing status = 1. Its stable
artifact slug is exactly
`gpu-closed-loop-slice-d-{profile_slug}-{capacity_slug}`; its profile/class,
lowercase 40-hex Git commit/tree IDs, phenotype hash/manifest digest, canonical
capacity digest, and artifact digest must match the body before writing. Embed
the complete `PhenotypeEvidenceManifest`, not only its digest. Extend the shared
`gpu-evidence-validate --slice a|b|c|d --input <path>` loader; Slice D validation
recomputes the manifest, capacity, every typed subreceipt digest, adapter
identity, and the shared artifact digest before accepting status 1.
It also requires `header.adapter_backend ==
NeuralGpuBackendApi::try_from_raw(adapter.backend_api_raw)?.slug()` and
`header.adapter_name` to equal the strict used UTF-8 prefix of
`adapter.adapter_name_utf8`. Vendor/device, driver, feature, and limit digests
must equal the shared runtime `GpuHardwareReceipt` used by the run. Header/body
display-name or backend mismatches are tamper errors even though the display
name is excluded from machine adapter identity.

Translate Slice C's final shared topology-capacity record with checked
`u32::try_from` for every value; an unrepresentable host value is a
configuration error, never wrapping receipt data. Every `samples_digest`,
`receipts_digest`, `events_digest`, `sequence_digest`, `comparisons_digest`, and
the top-level shared artifact digest uses the canonical evidence builder and
covers all ordered raw entries. No summary counter can pass if its raw vector or
subreceipt digest disagrees.
`TopologySoakReceipt.max_observed_bindings_per_kind` is recomputed from all
surviving concepts and ordered raw observation receipts, must be no greater
than `capacity.max_bindings_per_kind`, and is passed to the shared
two-argument `TopologyCapacityReceipt::contains` check. A receipt that proves
only concepts/edges/simplexes/gaps but omits binding pressure is invalid.

Warm up 256 ticks, then sample logical commitment, physical allocation and all
physical subcategories plus process RSS every 64 ticks. The fixed population's
logical commitment and retained physical allocation must each be exactly
constant after warmup; they are independent bounds and are never subtracted
from one another. Mutable content/generation digests are not allocation
digests. Configure an RSS
ceiling and a growth envelope of `max(16 MiB, warmup_rss / 20)`; require the
post-warmup high-water minus low-water and last-quartile minus first-quartile
mean to remain inside that envelope. Quartiles are the first/last 39 of the 157
ordered samples and means use checked integer sum with floor division by 39.
Record the raw samples.
Sample ticks are exactly `256, 320, ..., 10_240` (157 samples) for both
allocation and RSS receipts. `raw_dispatch_samples` contains one entry for
every completed neural dispatch and none for non-awake world ticks; its count
equals `authoritative_gpu_dispatches`.

Run exactly the production world->frame->GPU->world->sealed patch->learning
loop. Consume Slice C `MemoryUpdateReceipt`, `MemoryCompactionReceipt`, and
`TopologyObservationReceipt`; do not define replacements. Replay starts from a
real checkpoint, injects the recorded `GpuPressureSample` sequence, and compares
selected candidates exactly and logits within the same-adapter tolerance of
f32 `1e-5` (`0x3727c5ac`). "Same adapter" is validated by vendor/device/backend,
driver, feature and limits digests; the display name is provenance only.

The run deliberately saturates candidates, object slots, memory contexts and
topology bindings so every truncation kind emits at least one raw typed event.
Truncation kind codes are 1 candidate, 2 grounded object slot, 3 episodic
memory context, and 4 topology binding; every event requires `requested =
retained + dropped` and binds input/output digests.
At deterministic checkpoints it performs one vNext save/restore in a real
non-awake sleep/consolidation phase, validates production legacy migrations for
IDs 1/2/3, validates inspection-only migrations for IDs 4+, and records every
receipt. It never requests a neural policy switch; the initial/final raw policy
ID is `NeuralClosedLoopGpu`, the event vector is empty and count is zero.
Migration classification codes are 1 production-recompiled, 2
inspection-only, and 3 rejected-unknown; a passing production row requires one
compile/admission, while inspection-only requires zero of both.

- [ ] **Step 4: Run the focused soak test against the final source**

Run: `cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test gpu_closed_loop_soak -j 1 -- --nocapture`

Expected: green immediately before commit. Any fix repeats this step.

- [ ] **Step 5: Commit the soak implementation**

```powershell
git add crates/alife_game_app/src/gpu_evidence.rs crates/alife_game_app/src/gpu_live_runtime.rs crates/alife_game_app/src/bin/alife_game_app.rs crates/alife_game_app/tests/gpu_closed_loop_soak.rs
git commit -m "Prove bounded GPU brain soak and replay"
```

- [ ] **Step 6: Require clean committed HEAD, then run six fail-fast receipts**

```powershell
if (git status --short) { throw "soak evidence requires clean HEAD" }
$soakHead = git rev-parse HEAD
$soakTree = git rev-parse 'HEAD^{tree}'
$classes = @('n512', 'n1024', 'n2048')
$profiles = @('privileged-affordance-v1', 'grounded-object-slots-v1')
foreach ($profile in $profiles) {
    foreach ($class in $classes) {
        $output = "target/artifacts/gpu-closed-loop-slice-d-$profile-$class.json"
        cargo run -p alife_game_app --features "gpu-runtime gpu-tests" --bin alife_game_app -- gpu-closed-loop-soak --class $class --sensor-profile $profile --ticks 10240 --seed 4505 --output $output
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
}
```

Expected: six profile/class-qualified Vulkan receipts with no overwrite and
headers equal to `$soakHead`/`$soakTree`. They are provisional until Task 9's
mandatory final-HEAD refresh.

### Task 7: Perform the residual authority and compatibility audit

**Files:**
- Create: `crates/alife_game_app/tests/gpu_brain_authority_audit.rs`
- Modify: `crates/alife_world/src/legacy_neural_policy_v1.rs` only if the audit finds a defect in its private deserialize-only historical mappings.
  private deserialize-only historical enum/string mappings.
- Modify: `crates/alife_world/src/lib.rs` only to keep that module crate-private.
- Modify only files named by a failing audit; do not repeat already completed
  Slice A/B deletions.

**Interfaces:**
- Consumes: the completed A/B/C/D production source tree.
- Produces: positive GPU-authority assertions and a guarded absence scan.

- [ ] **Step 1: Add the failing positive boundary test**

The test walks production modules, excluding itself and the explicitly private
legacy deserializer module, and asserts:

```rust
assert_eq!(receipt.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
assert!(receipt.authoritative_gpu_dispatches > 0);
assert_eq!(receipt.authoritative_gpu_dispatches, receipt.gpu_selections);
assert!(receipt.truncation.compact_readback_bytes <= 64);
```

It rejects production identifiers or product text matching underscore, dash,
space, or camel-case forms of CPU shadow/reference, neural fallback, and parity
gate/gated/gating, plus `AutoWithCpuFallback` and `FullGpuRuntimeMode`. Do not retain zero-valued
compatibility counters and do not add a fallback UI field.
The test explicitly excludes only
`alife_world/src/legacy_neural_policy_v1.rs`, whose module is crate-private and
may be called only by versioned load migration tests.
Both the Rust test and PowerShell gate construct the forbidden expression from
separate string fragments so neither embeds a self-match. No script is
excluded.

- [ ] **Step 2: Run the audit and migrate only actual residuals**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_brain_authority_audit -j 1`

Expected initially: any genuine residual is named. Remove or migrate that exact
surface and its callers; existing A/B private legacy migration input strings
remain scoped to the excluded deserializer.

- [ ] **Step 3: Run the guarded repository scan**

```powershell
$forbiddenTerms = @(
    ('cpu' + '[-_ ]?' + 'shadow'),
    ('AutoWith' + 'CpuFallback'),
    ('Cpu' + '[-_ ]?' + 'Reference'),
    ('neural' + '[-_ ]?' + 'fallback'),
    ('FullGpu' + 'RuntimeMode'),
    ('parity' + '[-_ ]?' + 'gat(?:e|ed|ing)')
)
$forbiddenPattern = $forbiddenTerms -join '|'
$rawMatches = & rg -n -i $forbiddenPattern crates/alife_core/src crates/alife_gpu_backend/src crates/alife_world/src crates/alife_game_app/src crates/alife_tools/src scripts
$scanExit = $LASTEXITCODE
if ($scanExit -gt 1) { throw "authority scan failed with exit $scanExit" }
$matches = @($rawMatches | Where-Object {
    $_ -notmatch 'crates[\\/]alife_world[\\/]src[\\/]legacy_neural_policy_v1.rs:'
})
if ($matches.Count -ne 0) { $matches; throw "superseded neural authority surface remains" }
```

Expected: no production matches. This scan is repeated after the last source
change in Task 9.

- [ ] **Step 4: Re-run the focused audit and self-safe scan after all residual
  fixes**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_brain_authority_audit -j 1`

Run the exact Step 3 PowerShell block again.

Expected: both are green immediately before staging. Any new residual returns
to Step 2; do not commit a known failing audit.

- [ ] **Step 5: Commit exact audit paths**

Stage the new test plus only the residual production files actually changed;
do not use a broad deletion list or re-delete A/B files.

```powershell
git status --short
git diff --check
$residualPaths = @(git diff --name-only -- crates/alife_core/src crates/alife_gpu_backend/src crates/alife_world/src crates/alife_game_app/src crates/alife_tools/src scripts | Sort-Object -Unique)
$residualPaths
git add crates/alife_game_app/tests/gpu_brain_authority_audit.rs crates/alife_world/src/legacy_neural_policy_v1.rs crates/alife_world/src/lib.rs
if ($residualPaths.Count -gt 0) { git add -- $residualPaths }
git diff --cached --check
git commit -m "Audit GPU brain authority surfaces"
```

Inspect `git diff --cached --name-only` and confirm each staged residual is one
named by the audit; do not stage an unrelated path.

### Task 8: Add ADR-026 and derive promotion from the A/B/C/D evidence matrix

**Files:**
- Modify: `docs/architecture_decisions.md`
- Modify: `docs/master_spec.md`
- Modify: `AGENTS.md`
- Modify: `docs/AGENTS.md`
- Modify: `crates/alife_core/AGENTS.md`
- Modify: `crates/alife_gpu_backend/AGENTS.md`
- Modify: `crates/alife_world/AGENTS.md`
- Modify: `crates/alife_game_app/AGENTS.md`
- Modify: `crates/alife_tools/AGENTS.md`
- Create: `crates/alife_game_app/src/gpu_closed_loop_promotion.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Modify: `crates/alife_game_app/src/bin/alife_game_app.rs`
- Create: `crates/alife_game_app/tests/gpu_closed_loop_promotion.rs`
- Create: `scripts/run_gpu_closed_loop_gates.ps1`

**Interfaces:**
- Consumes: explicit class/profile-qualified A, B, C, and D artifact paths plus
  benchmark/save/authority receipts.
- Produces: ADR-026, `GpuClosedLoopPromotionManifest`, and promoted classes
  derived only from complete evidence.

- [ ] **Step 1: Write failing manifest-ingestion tests**

```rust
#[test]
fn no_class_promotes_without_complete_abcd_evidence() {
    let mut inputs = valid_receipt_set();
    inputs.remove_slice_b(BrainCapacityClass::N1024_ID);
    let manifest = ingest_promotion_evidence(inputs).unwrap();
    assert!(!manifest.promoted_classes.contains(&BrainCapacityClass::N1024_ID));
}

#[test]
fn complete_matrix_promotes_exactly_three_classes() {
    let manifest = ingest_promotion_evidence(valid_receipt_set()).unwrap();
    assert_eq!(manifest.promoted_classes, vec![
        BrainCapacityClass::N512_ID,
        BrainCapacityClass::N1024_ID,
        BrainCapacityClass::N2048_ID,
    ]);
    assert!(manifest.rows.iter().all(PromotionEvidenceRow::all_required_gates_pass));
}

#[test]
fn different_valid_per_slice_phenotype_hashes_are_allowed_but_tampering_is_not() {
    let inputs = valid_receipt_set_with_distinct_slice_seeds();
    assert!(ingest_promotion_evidence(inputs.clone()).is_ok());
    let tampered = inputs.with_slice_c_phenotype_hash_bit_flipped();
    assert!(ingest_promotion_evidence(tampered).is_err());
}

#[test]
fn promotion_retains_exact_gate_benchmark_and_adapter_bindings() {
    let manifest = ingest_promotion_evidence(valid_receipt_set()).unwrap();
    assert_ne!(manifest.gate.receipt_digest, [0; 4]);
    assert_ne!(manifest.benchmark.manifest_digest, [0; 4]);
    assert!(manifest.rows.iter().all(|row| row.benchmark_rows.len() == 12));
    assert!(manifest.rows.iter().flat_map(|row| &row.artifact_bindings)
        .all(|binding| binding.adapter == manifest.adapter));
    assert!(manifest.rows.iter().flat_map(|row| &row.benchmark_rows)
        .all(|binding| binding.adapter == manifest.adapter));
    assert_eq!(manifest.gate.adapter, manifest.adapter);
    assert_eq!(manifest.benchmark.adapter, manifest.adapter);
}
```

Add mismatch tests for schema, class, profile, artifact-internal phenotype hash,
canonical capacity tuple/digest, adapter/backend,
source-tree digest, non-ancestor evidence commits, artifact digest, tolerance,
tick count, and non-passing status. Add cardinality/key/tamper cases for all 12
benchmark rows per class, benchmark/gate content digests, command-list digest,
adapter identity on every artifact kind, and grounded-versus-privileged Slice C
validation rules.
The integration test imports the focused library module; it does not call a
binary-private helper.

- [ ] **Step 2: Run and verify the aggregator is absent**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_closed_loop_promotion -j 1`

Expected: unresolved promotion module.

- [ ] **Step 3: Implement manifest ingestion, not hidden reruns**

For each class, require:

```text
Slice A: causal + GPU authority + replay
Slice B: immediate learning + sleep + restore
Slice C: grounded 10,240-tick behavior/saturation + separate privileged
         64-tick provenance/GPU-authority receipt with no grounding claim
Slice D: logical/VRAM/admission budgets + ATP/throttle + save migration
         + benchmark row/status + both-profile 10,240 soak + replay
Global:  authority scan + docs/boundary gates + exact git commit
```

For a class to promote, all 12 benchmark rows for that class (two profiles by
six populations) must be present and `Completed`. `Missed` or `Unavailable`
remains valid honest evidence but blocks that class; it is never converted to a
passing row or silently omitted.

The CLI accepts explicit `--slice-a`, `--slice-b`, `--slice-c`, `--slice-d`,
`--benchmark`, `--gates`, and `--output` paths. Flags for slices are repeatable.
Validate canonical JSON digests and write
`target/artifacts/gpu-closed-loop-promotion.json` atomically. Do not seed
`promoted_classes` from config/test options.

Each artifact validates its own phenotype hash against its linked canonical
phenotype manifest, compile-input digest, mutable-state/assets where applicable,
class ID, and Slice A canonical capacity tuple. Slice fixtures intentionally use
different seeds, so A/B/C/D phenotype hashes are not required to equal one
another. Promotion rejects an internally tampered or wrong-class hash, not a
valid cross-slice difference.

Use fixed-width receipt identities:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitObjectId(pub [u8; 20]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceAdapterBinding {
    pub vendor_id: u32,
    pub device_id: u32,
    pub backend_api_raw: u16,
    pub driver_digest: [u64; 4],
    pub feature_digest: [u64; 4],
    pub limits_digest: [u64; 4],
    pub identity_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceArtifactBinding {
    pub slice_raw: u16,
    pub class_id_raw: u16,
    pub profile_id_raw: u16,
    pub profile_schema: u16,
    pub artifact_schema: u16,
    pub evidence_commit: GitObjectId,
    pub source_tree: GitObjectId,
    pub artifact_digest: [u64; 4],
    pub phenotype_hash: PhenotypeHash,
    pub phenotype_manifest_digest: [u64; 4],
    pub capacity_digest: [u64; 4],
    pub adapter: EvidenceAdapterBinding,
    pub status_raw: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkRowBinding {
    pub class_id_raw: u16,
    pub profile_id_raw: u16,
    pub profile_schema: u16,
    pub population: u32,
    pub status_raw: u16,
    pub row_digest: [u64; 4],
    pub phenotype_hash: PhenotypeHash,
    pub phenotype_manifest_digest: [u64; 4],
    pub capacity_digest: [u64; 4],
    pub protocol_digest: [u64; 4],
    pub adapter: EvidenceAdapterBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkManifestBinding {
    pub evidence_commit: GitObjectId,
    pub source_tree: GitObjectId,
    pub manifest_digest: [u64; 4],
    pub protocol_digest: [u64; 4],
    pub adapter: EvidenceAdapterBinding,
    pub row_bindings_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateEvidenceBinding {
    pub evidence_commit: GitObjectId,
    pub source_tree: GitObjectId,
    pub receipt_digest: [u64; 4],
    pub gate_script_digest: [u64; 4],
    pub commands_digest: [u64; 4],
    pub adapter: EvidenceAdapterBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionEvidenceRow {
    pub class_id_raw: u16,
    pub canonical_capacity_digest: [u64; 4],
    pub artifact_bindings: Vec<EvidenceArtifactBinding>,
    pub benchmark_rows: Vec<BenchmarkRowBinding>,
    pub required_gate_bits: u64,
    pub passed_gate_bits: u64,
    pub row_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuClosedLoopPromotionManifest {
    pub schema_version: u16,
    pub promotion_commit: GitObjectId,
    pub source_tree_digest: GitObjectId,
    pub adapter: EvidenceAdapterBinding,
    pub gate: GateEvidenceBinding,
    pub benchmark: BenchmarkManifestBinding,
    pub rows: Vec<PromotionEvidenceRow>,
    pub promoted_classes: Vec<BrainClassId>,
    pub manifest_digest: [u64; 4],
}
```

All A/B/C/D JSON headers serialize `git_commit` and `source_tree_digest` as
lowercase 40-hex Git object IDs. Ingestion strictly parses both into
`GitObjectId([u8; 20])`. Four-word digests are reserved for canonical artifact,
phenotype-manifest, capacity, row, and manifest content digests; they are never
used as a substitute Git tree identity. Profile-independent Slice A/B bindings
use profile ID/schema zero; profile-qualified C/D and benchmark bindings must
use their exact nonzero profile identity. Binding status codes are 1 passing, 2
missed target, 3 unavailable, and 4 failed; only 1 sets a passed gate bit.
`GitObjectId` uses custom serde to serialize and deserialize one lowercase
40-hex string, never a JSON byte array.

The validated A/B/C/D body loader extracts one `EvidenceAdapterBinding` from
the machine fields covered by each artifact digest. Display names are excluded.
For each class, `artifact_bindings` contains exactly A, B, both C profiles, and
both D profiles; `benchmark_rows` contains exactly the two profiles by six
populations. The manifest retains the benchmark-manifest and gate-receipt
bindings explicitly rather than reducing them to opaque gate bits. Every
passing slice, benchmark row, benchmark manifest, gate receipt, and the
promotion output must share one exact adapter identity. All binding/row/
manifest digests use the shared canonical evidence builder.

Implement `scripts/run_gpu_closed_loop_gates.ps1` as the sole gate-receipt
writer. It runs the exact Task 9 validation commands, stops on the first
nonzero exit, records command/exit/duration plus clean git commit and Vulkan
adapter receipt plus canonical source-tree digest, and atomically writes
`target/artifacts/gpu-closed-loop-gates.json` only after all commands pass. A
failed or interrupted run removes staging and cannot leave a passing receipt.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateCommandReceipt {
    pub command_id: u16,
    pub argv_utf8: Vec<u8>,
    pub started_monotonic_ns: u64,
    pub ended_monotonic_ns: u64,
    pub exit_code: i32,
    pub stdout_digest: [u64; 4],
    pub stderr_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuGateAdapterReceipt {
    pub vendor_id: u32,
    pub device_id: u32,
    pub backend_api_raw: u16,
    pub adapter_name_len: u16,
    pub adapter_name_utf8: [u8; 128],
    pub driver_digest: [u64; 4],
    pub feature_digest: [u64; 4],
    pub limits_digest: [u64; 4],
    pub identity_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuClosedLoopGateReceipt {
    pub schema_version: u16,
    pub git_commit: GitObjectId,
    pub source_tree_digest: GitObjectId,
    pub adapter: GpuGateAdapterReceipt,
    pub gate_script_digest: [u64; 4],
    pub commands: Vec<GateCommandReceipt>,
    pub commands_digest: [u64; 4],
    pub passed: bool,
    pub receipt_digest: [u64; 4],
}
```

`argv_utf8` is exact UTF-8 joined with NUL separators, bounded to 2,048 bytes,
and command IDs 1-12 are unique/in order. `stdout_digest` and `stderr_digest`
cover the exact raw captured byte streams; `commands_digest` covers all twelve
ordered command records; `receipt_digest` covers every field except itself.
All use the shared canonical evidence digest implementation. The gate JSON uses
the exact external field `source_tree_digest` and serializes both Git IDs as
lowercase 40-hex strings, matching the final PowerShell consumer.

The committed script has this exact ordered command list; command IDs and argv
are serialized verbatim, and no command is inferred dynamically:

```text
01-fmt: cargo fmt --all -- --check
02-check: cargo check --workspace --all-targets --all-features -j 1
03-workspace-tests: cargo test --workspace --all-features -j 1
04-core-brain: cargo test -p alife_core --test production_brain_budgets --test phenotype_compiler --test brain_topology
05-gpu-brain: cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_runtime --test closed_loop_admission --test closed_loop_activity --test closed_loop_gpu_behavior --test closed_loop_eligibility --test closed_loop_fast_plasticity --test closed_loop_sleep --test closed_loop_memory_context -- --nocapture
06-world-save: cargo test -p alife_world --test gpu_brain_persistence --test gpu_brain_vnext_migration --test gpu_memory_grounding_persistence
07-app-brain: cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test gpu_closed_loop_acceptance --test gpu_learning_sleep_acceptance --test gpu_memory_grounding_acceptance --test gpu_sleep_restore --test gpu_closed_loop_soak --test gpu_brain_authority_audit --test gpu_closed_loop_promotion -j 1 -- --nocapture
08-tools-benchmark: cargo test -p alife_tools --test benchmark_tiers
09-docs: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
10-boundaries: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
11-authority-scan: internal exact scan below
12-diff: git diff --check origin/main...HEAD
```

Before command 01 the script requires `git status --porcelain=v1` to be empty,
resolves `HEAD^{tree}`, records HEAD and the tree digest, and deletes only its
own `.staging` output. It rechecks unchanged HEAD/tree and a clean status after
command 12 and before atomic rename. Command 11 constructs its expression without embedding a
forbidden term and excludes only the private legacy deserializer:

```powershell
$terms = @(
    ('cpu' + '[-_ ]?' + 'shadow'), ('AutoWith' + 'CpuFallback'),
    ('Cpu' + '[-_ ]?' + 'Reference'),
    ('neural' + '[-_ ]?' + 'fallback'),
    ('FullGpu' + 'RuntimeMode'),
    ('parity' + '[-_ ]?' + 'gat(?:e|ed|ing)')
)
$pattern = $terms -join '|'
$raw = & rg -n -i $pattern crates/alife_core/src crates/alife_gpu_backend/src crates/alife_world/src crates/alife_game_app/src crates/alife_tools/src scripts
if ($LASTEXITCODE -gt 1) { throw "authority scan execution failed" }
$bad = @($raw | Where-Object { $_ -notmatch 'crates[\\/]alife_world[\\/]src[\\/]legacy_neural_policy_v1.rs:' })
if ($bad.Count -ne 0) { $bad; throw "authority scan found residuals" }
```

The gate receipt also records per-command start/end monotonic nanoseconds,
exit code, stdout/stderr digest, Vulkan vendor/device/backend/driver/features/
limits receipt, gate-script digest, HEAD, and source-tree digest. It cannot mark
passing unless all twelve IDs occur exactly once in order.

- [ ] **Step 4: Append ADR-026 and update the controlling spec**

Append:

```markdown
## ADR-026: GPU Closed-Loop Scaling Is Evidence- and Budget-Gated

Decision: N512, N1024, and N2048 are the only initial production neural
capacity classes. Each is promoted independently only after GPU-authoritative
causal, learning/sleep, grounding/saturation, save, global/per-route budget,
VRAM admission, ATP/throttle, populated benchmark, soak, and replay evidence
passes. Runtime population limits come from an explicit neural-heap profile,
not the capacity class. N4096 and larger legacy tiers remain inspection/export
only. No CPU neural shadow, parity gate, or automatic CPU neural fallback is a
promotion mechanism.
```

ADR-026 supersedes the benchmark/fallback portions of ADR-016, ADR-019,
ADR-021, and ADR-022 that conflict with ADR-024 through ADR-026. Keep renderer
fallback decisions separate.

Update master-spec sections 8, 21–28, 33–36, 40, glossary, and validation
matrix so the controlling text matches the implemented GPU-authoritative
architecture rather than the original scaffold phase.

Update every local instruction file whose ownership changed: core owns
capacity/evidence contracts, GPU owns the only neural execution backend, world
owns unscored candidates/legality, app owns scheduling/policy/evidence
ingestion, tools owns GPU-only benchmark artifacts, and docs records the
superseding ADRs. No local AGENTS file may retain a scaffold-only, CPU-fallback,
or CPU-parity instruction after ADR-026.

- [ ] **Step 5: Run docs and promotion tests, then commit**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
cargo test -p alife_game_app --features gpu-runtime --test gpu_closed_loop_promotion -j 1
cargo check --workspace --all-targets --all-features -j 1
git add docs/architecture_decisions.md docs/master_spec.md AGENTS.md docs/AGENTS.md crates/alife_core/AGENTS.md crates/alife_gpu_backend/AGENTS.md crates/alife_world/AGENTS.md crates/alife_game_app/AGENTS.md crates/alife_tools/AGENTS.md crates/alife_game_app/src/gpu_closed_loop_promotion.rs crates/alife_game_app/src/lib.rs crates/alife_game_app/src/bin/alife_game_app.rs crates/alife_game_app/tests/gpu_closed_loop_promotion.rs scripts/run_gpu_closed_loop_gates.ps1
git commit -m "Gate GPU brain class promotion on evidence"
```

### Task 9: Run final gates, commit, push, and merge without touching FVR11 work

**Files:**
- No planned runtime changes. Any source change discovered here returns to its
  owning task and reruns that task's tests before this task restarts.
- Runtime artifact: `target/artifacts/gpu-closed-loop-promotion.json`.

**Interfaces:**
- Consumes: the clean, committed A/B/C/D source tree and all final-HEAD evidence producers.
- Produces: one final promotion manifest, verified branch integration, and an untouched FVR11 worktree.

- [ ] **Step 1: Verify the exact isolated worktree and branch**

```powershell
$expectedRoot = (Resolve-Path 'D:\A life-brain-gpu-closed-loop').Path
if ((Get-Location).Path -ne $expectedRoot) { throw "wrong worktree" }
if ((git branch --show-current) -ne 'codex/brain-gpu-closed-loop') { throw "wrong branch" }
if (git status --short) { throw "worktree must be clean before final gates" }
git worktree list --porcelain
```

Expected: this task is on the isolated feature worktree. `D:\A life` remains
the separate FVR11 worktree and is never used for integration.

- [ ] **Step 2: Regenerate all A/B/C/D and benchmark evidence from the final
  clean source commit**

This is mandatory after Task 8 because Task 8 changes source/docs after the
provisional Slice A-D artifacts. Delete only the exact known outputs, then run
every producer from the same clean committed HEAD:

```powershell
if (git status --short) { throw "final evidence requires clean HEAD" }
$evidenceHead = git rev-parse HEAD
$evidenceTree = git rev-parse 'HEAD^{tree}'
$evidenceStarted = [DateTime]::UtcNow
$classes = @('n512', 'n1024', 'n2048')
$profiles = @('privileged-affordance-v1', 'grounded-object-slots-v1')
$artifacts = @(
    'target/artifacts/gpu-closed-loop-benchmark-full.json',
    'target/artifacts/gpu-closed-loop-gates.json',
    'target/artifacts/gpu-closed-loop-promotion.json'
)
foreach ($class in $classes) {
    $artifacts += "target/artifacts/gpu-closed-loop-slice-a-$class.json"
    $artifacts += "target/artifacts/gpu-learning-sleep-slice-b-$class.json"
    foreach ($profile in $profiles) {
        $artifacts += "target/artifacts/gpu-memory-grounding-slice-c-$profile-$class.json"
        $artifacts += "target/artifacts/gpu-closed-loop-slice-d-$profile-$class.json"
    }
}
foreach ($path in $artifacts) {
    if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path }
}

foreach ($class in $classes) {
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-closed-loop-acceptance --class $class --ticks 64 --seed 4101 --sensor-profile privileged-affordance-v1 --output "target/artifacts/gpu-closed-loop-slice-a-$class.json"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-learning-sleep-acceptance --class $class --seed 4202 --output "target/artifacts/gpu-learning-sleep-slice-b-$class.json"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-memory-grounding-acceptance --class $class --ticks 10240 --seed 4303 --sensor-profile grounded-object-slots-v1
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-memory-grounding-acceptance --class $class --ticks 64 --seed 4303 --sensor-profile privileged-affordance-v1
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    foreach ($profile in $profiles) {
        cargo run -p alife_game_app --features "gpu-runtime gpu-tests" --bin alife_game_app -- gpu-closed-loop-soak --class $class --sensor-profile $profile --ticks 10240 --seed 4505 --output "target/artifacts/gpu-closed-loop-slice-d-$profile-$class.json"
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
}
cargo run -p alife_tools --bin benchmark_tiers -- --backend gpu-closed-loop --base-seed 4404 --populations 1,10,50,100,250,500 --classes n512,n1024,n2048 --sensor-profiles privileged-affordance-v1,grounded-object-slots-v1 --targets configs/gpu_closed_loop_performance_targets_v1.json --output target/artifacts/gpu-closed-loop-benchmark-full.json
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$evidenceArtifacts = @($artifacts | Where-Object { $_ -notmatch 'gpu-closed-loop-(gates|promotion)\.json$' })
foreach ($path in $evidenceArtifacts) {
    if (-not (Test-Path -LiteralPath $path)) { throw "missing evidence $path" }
    if ((Get-Item -LiteralPath $path).LastWriteTimeUtc -lt $evidenceStarted) { throw "stale evidence $path" }
    $json = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    $header = if ($null -ne $json.header) { $json.header } else { $json }
    if ($header.git_commit -ne $evidenceHead) { throw "wrong evidence commit $path" }
    if ($header.source_tree_digest -ne $evidenceTree) { throw "wrong evidence tree $path" }
}
foreach ($class in $classes) {
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-evidence-validate --slice a --input "target/artifacts/gpu-closed-loop-slice-a-$class.json"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-evidence-validate --slice b --input "target/artifacts/gpu-learning-sleep-slice-b-$class.json"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    foreach ($profile in $profiles) {
        cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-evidence-validate --slice c --input "target/artifacts/gpu-memory-grounding-slice-c-$profile-$class.json"
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-evidence-validate --slice d --input "target/artifacts/gpu-closed-loop-slice-d-$profile-$class.json"
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
}
cargo run -p alife_tools --bin benchmark_tiers -- --validate target/artifacts/gpu-closed-loop-benchmark-full.json
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
```

Slice C intentionally has no `--output` override: its acceptance CLI derives
and validates the exact
`gpu-memory-grounding-slice-c-{profile}-{class}.json` path used in `$artifacts`.

Expected: every producer reports the exact `$evidenceHead` and Git tree object
ID `$evidenceTree`; all paths are newer than the refresh start, and every
artifact's canonical content/manifest/capacity/adapter binding validates. Any source fix
invalidates the entire set: commit the fix in its owning task, return to Step 1,
delete the exact outputs, and rerun this whole step.

- [ ] **Step 3: Run the authoritative post-source audit and validation gates**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_gpu_closed_loop_gates.ps1 -Output target/artifacts/gpu-closed-loop-gates.json
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
```

Expected: the script's committed command list (authority scan, fmt, workspace
check/tests, focused real-GPU tests, boundary/docs checks, and diff check) all
pass and it atomically writes a gate receipt for the current clean commit and
real Vulkan adapter.

- [ ] **Step 4: Update Graphify when available**

```powershell
$cmd = Get-Command graphify -ErrorAction SilentlyContinue
$graphifyExe = if ($cmd) { $cmd.Source } elseif (Test-Path "$HOME\.local\bin\graphify.exe") { (Resolve-Path "$HOME\.local\bin\graphify.exe").Path }
if ($graphifyExe) { & $graphifyExe update .; if ($LASTEXITCODE -ne 0) { throw "graphify update failed" } }
```

Graphify remains optional and is not a Cargo prerequisite. If it changes only
ignored `graphify-out/`, do not stage those files. If it changes any tracked
file, return that change to its owning task, test and commit it, then restart
Task 9 from Step 1 and regenerate all evidence.

- [ ] **Step 5: Generate and validate the final promotion manifest**

Run the manifest-ingestion CLI with all explicit A/B/C/D and benchmark artifact
paths. Then validate:

```powershell
$promotionArgs = @(
    'gpu-closed-loop-promote',
    '--gates', 'target/artifacts/gpu-closed-loop-gates.json',
    '--benchmark', 'target/artifacts/gpu-closed-loop-benchmark-full.json'
)
foreach ($class in @('n512', 'n1024', 'n2048')) {
    $promotionArgs += @('--slice-a', "target/artifacts/gpu-closed-loop-slice-a-$class.json")
    $promotionArgs += @('--slice-b', "target/artifacts/gpu-learning-sleep-slice-b-$class.json")
}
foreach ($profile in @('privileged-affordance-v1', 'grounded-object-slots-v1')) {
    foreach ($class in @('n512', 'n1024', 'n2048')) {
        $promotionArgs += @('--slice-c', "target/artifacts/gpu-memory-grounding-slice-c-$profile-$class.json")
        $promotionArgs += @('--slice-d', "target/artifacts/gpu-closed-loop-slice-d-$profile-$class.json")
    }
}
$promotionArgs += @('--output', 'target/artifacts/gpu-closed-loop-promotion.json')
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- @promotionArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
```

Expected: exactly N512/N1024/N2048 are promoted and every evidence row names
the current commit or a validated ancestor with the identical source-tree ID,
Vulkan adapter, correct class/profile, and passing status. The gate and
promotion receipts themselves name the current HEAD.

- [ ] **Step 6: Reject final tracked changes and verify clean**

Artifacts under `target/` remain untracked. There is no post-evidence metadata
commit. If Graphify or validation produced an intended tracked change, return
it to its owning task and restart the full evidence cycle. Then:

```powershell
git diff --check origin/main...HEAD
if (git status --short) { throw "feature worktree is not clean" }
```

- [ ] **Step 7: Synchronize, re-evidence a changed tree, and push the feature branch**

```powershell
$preSyncHead = git rev-parse HEAD
$preSyncTree = git rev-parse 'HEAD^{tree}'
git fetch origin
git merge --no-edit origin/main
$postSyncHead = git rev-parse HEAD
$postSyncTree = git rev-parse 'HEAD^{tree}'
if ($postSyncTree -ne $preSyncTree) {
    throw "STOP BEFORE PUSH: merged tree changed; restart Task 9 and regenerate all evidence"
}
if ($postSyncHead -ne $preSyncHead) {
    throw "STOP BEFORE PUSH: HEAD changed with identical tree; rerun Step 3 gates and Step 5 promotion, then restart this block"
}
```

If `$postSyncTree -ne $preSyncTree`, restart at Step 1 and rerun the complete
Step 2 A/B/C/D/benchmark refresh, Step 3 gates, and Step 5 manifest against the
merged tree. If only HEAD changes while the tree is identical, Slice evidence
may remain because its commits are ancestors and tree IDs match, but rerun Step
3 and Step 5 so gates/promotion bind the new current commit. Ingestion rejects
stale tree IDs and non-ancestor commits.
Then:

```powershell
git push -u origin codex/brain-gpu-closed-loop
```

- [ ] **Step 8: Merge from a separate clean integration worktree**

Do not checkout or modify `D:\A life`. From the feature worktree:

```powershell
$integration = 'D:\A life-gpu-brain-integration'
if (Test-Path -LiteralPath $integration) { throw "integration path already exists" }
git fetch origin
git worktree add -b codex/gpu-brain-integration-20260709 $integration origin/main
Set-Location $integration
git merge --no-ff codex/brain-gpu-closed-loop -m "Merge GPU-authoritative closed-loop brain"
$featureGate = Get-Content -Raw -LiteralPath 'D:\A life-brain-gpu-closed-loop\target\artifacts\gpu-closed-loop-gates.json' | ConvertFrom-Json
$validatedFeatureTree = $featureGate.source_tree_digest
$integrationTree = git rev-parse 'HEAD^{tree}'
$evidenceArtifactRoot = if ($integrationTree -eq $validatedFeatureTree) {
    'D:\A life-brain-gpu-closed-loop\target\artifacts'
} else {
    'D:\A life-gpu-brain-integration\target\artifacts'
}
```

Record the integration merge tree. If it differs from `$validatedFeatureTree`, run the
complete Step 2 evidence refresh inside the integration worktree before
anything else and set `$evidenceArtifactRoot` to
`D:\A life-gpu-brain-integration\target\artifacts`. If it matches, the feature
evidence remains valid because its commits are ancestors and its tree IDs
match; set `$evidenceArtifactRoot` to
`D:\A life-brain-gpu-closed-loop\target\artifacts`. In either case, run Step
3's exact gate script again so the gate receipt binds the integration merge
commit. Re-run Step 5 inside the integration worktree, replacing every A/B/C/D
and benchmark `target/artifacts` input with `$evidenceArtifactRoot`, while the
new gate input and promotion output remain integration-local. Evidence commits
must be ancestors and their source-tree digests must match. If it passes, protect
against a moving remote and push without force:

```powershell
$beforeRemoteMergeHead = git rev-parse HEAD
$beforeRemoteMergeTree = git rev-parse 'HEAD^{tree}'
git fetch origin
git merge --no-edit origin/main
$afterRemoteMergeHead = git rev-parse HEAD
$afterRemoteMergeTree = git rev-parse 'HEAD^{tree}'
if ($afterRemoteMergeTree -ne $beforeRemoteMergeTree) {
    throw "STOP BEFORE PUSH: tree changed; run complete Step 2 evidence refresh, Step 3 gates, and Step 5 promotion, then restart this block"
} elseif ($afterRemoteMergeHead -ne $beforeRemoteMergeHead) {
    throw "STOP BEFORE PUSH: HEAD changed; rerun Step 3 gates and Step 5 promotion, then restart this block"
}
git push origin HEAD:main
$local = git rev-parse HEAD
$remote = ((git ls-remote origin refs/heads/main) -split "`t")[0]
if ($local -ne $remote) { throw "remote main SHA mismatch" }
if (git status --short) { throw "integration worktree is dirty" }
```

The comments above are mandatory branches, not optional notes. Do not push
until the applicable refresh commands have completed. If the non-force push is
rejected because `origin/main` moved again, repeat the fetch/merge/tree-compare
branch; never force-push and never reuse evidence across a changed tree.

- [ ] **Step 9: Remove only the clean temporary integration worktree**

Return to the feature worktree and verify the resolved removal target equals
the exact integration path before removal:

```powershell
Set-Location 'D:\A life-brain-gpu-closed-loop'
$resolvedIntegration = (Resolve-Path -LiteralPath 'D:\A life-gpu-brain-integration').Path
if ($resolvedIntegration -ne 'D:\A life-gpu-brain-integration') { throw "unexpected integration path" }
git worktree remove 'D:\A life-gpu-brain-integration'
git branch -D codex/gpu-brain-integration-20260709
git status --short
git worktree list
```

Expected: feature worktree clean, temporary integration worktree removed,
remote `main` at the verified merge SHA, and the unrelated FVR11 worktree and
branch untouched.

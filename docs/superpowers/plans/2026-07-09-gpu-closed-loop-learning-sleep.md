# GPU Closed-Loop Learning and Sleep Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make GPU neural behavior change immediately from sealed outcomes, then consolidate and retain that learning through the canonical automatic sleep cycle.

**Architecture:** The shared GPU backend accumulates recurrent and decoder-feature eligibility in class-bucketed brain slots. After the world seals an `ExperiencePatch`, the app addresses the matching `GpuBrainHandle`, uploads one replay-protected outcome-credit record derived only from that patch, and a WGSL kernel updates immediately active `H_fast`; sleep later promotes bounded fast state into lifetime weights through an exactly-once GPU consolidation job and generation-safe slot buffer swap.

**Tech Stack:** Rust 2021, wgpu 29.0.3, WGSL, serde, bytemuck, existing `ExperiencePatch` and sleep contracts, real Vulkan hardware tests.

## Global Constraints

- Slice A must be complete and its GPU causal acceptance receipt must pass.
- `W_effective = W_genetic + W_lifetime + alpha * H_fast` in every waking recurrent and decoder dispatch.
- `H_shadow` may be an audit journal only; it cannot be the only waking update or a live action source.
- Plasticity applies only after a matching sealed patch and only once per sequence ID.
- No CPU plasticity shadow, CPU parity gate, or CPU neural fallback.
- Sleep progresses through the normal runtime scheduler and emits no actions outside `Awake`.
- Consolidation dispatches exactly once per sleep cycle ID and never mutates genetic weights.
- Active-loop readback remains compact; weight snapshots are save/sleep/manual boundaries only.

## Planned file structure

- Create `crates/alife_core/src/learning.rs`: neuromodulator and patch-gated credit contracts.
- Modify `crates/alife_core/src/phenotype.rs`: eligibility/learning receptor plan.
- Modify `crates/alife_core/src/neural.rs` and `genome.rs`: fast-weight
  semantics; delete the old CPU Oja executor after GPU replacement tests pass.
- Create `crates/alife_gpu_backend/src/closed_loop_learning.rs`: GPU outcome upload and plasticity dispatch.
- Create `crates/alife_gpu_backend/shaders/closed_loop_eligibility.wgsl`.
- Create `crates/alife_gpu_backend/shaders/closed_loop_plasticity.wgsl`.
- Create `crates/alife_gpu_backend/src/closed_loop_sleep.rs`.
- Create `crates/alife_gpu_backend/shaders/closed_loop_consolidate.wgsl`.
- Modify `crates/alife_game_app/src/live_brain_bridge.rs` and `gpu_live_runtime.rs`: post-seal learning and automatic sleep schedule.
- Modify `crates/alife_world/src/persistence.rs`: portable fast/lifetime/sleep checkpoint records.

---

### Task 1: Define patch-gated three-factor learning contracts

**Files:**
- Create: `crates/alife_core/src/learning.rs`
- Create: `crates/alife_core/tests/three_factor_learning.rs`
- Modify: `crates/alife_core/src/error.rs`
- Modify: `crates/alife_core/src/version.rs`
- Modify: `crates/alife_core/src/lib.rs`

**Interfaces:**
- Consumes: sealed `ExperiencePatch` whose `DecisionEvidence` is `NeuralClosedLoopGpu`.
- Produces: `NeuromodulatorSample`, `OutcomeCreditPacket`, `OutcomeCreditReplayKey`, `LearningSequenceGuard`, `FastWeightSemantics`.

- [ ] **Step 1: Write failing modulation, evidence, and replay tests**

```rust
#[test]
fn pain_and_reward_create_opposite_bounded_modulators() {
    let reward = NeuromodulatorSample::from_components(0.8, 0.0, 0.2, 0.0, 0.1).unwrap();
    let pain = NeuromodulatorSample::from_components(-0.2, 0.9, -0.4, 0.3, 0.0).unwrap();
    assert!(reward.value() > 0.0);
    assert!(pain.value() < 0.0);
    assert!((-1.0..=1.0).contains(&reward.value()));
    assert!((-1.0..=1.0).contains(&pain.value()));
}

#[test]
fn serialized_modulator_recomputes_and_rejects_a_forged_value() {
    let sample = NeuromodulatorSample::from_components(0.8, 0.0, 0.2, 0.0, 0.1).unwrap();
    let mut json = serde_json::to_value(sample).unwrap();
    json["value"] = serde_json::json!(-1.0);
    assert!(serde_json::from_value::<NeuromodulatorSample>(json).is_err());
}

#[test]
fn credit_packet_requires_matching_sealed_patch() {
    let patch = sealed_patch_fixture();
    let packet = OutcomeCreditPacket::from_sealed_patch(&patch).unwrap();
    assert_eq!(packet.sequence_id(), patch.header().sequence_id);
    assert_eq!(packet.outcome_tick(), patch.outcome().outcome_tick);
    assert_eq!(packet.phenotype_hash(), patch.decision().neural_evidence().unwrap().phenotype_hash);
    assert_eq!(packet.selected_candidate(), patch.decision().neural_evidence().unwrap().candidate_index);
    assert_eq!(packet.selected_family(), patch.decision().neural_evidence().unwrap().action_family);
    assert_eq!(packet.active_activation_side(), patch.decision().neural_evidence().unwrap().active_activation_side);
}
```

Also write wrong-organism, phenotype-hash, frame-digest, action-ID, tick,
sequence, heuristic-evidence, unsealed-patch, NaN, duplicate-sequence, and
older-sequence rejection cases before implementation. Each asserts no mutation
receipt and an unchanged `LearningSequenceGuard`.

- [ ] **Step 2: Verify missing API**

Run: `cargo test -p alife_core --test three_factor_learning`

Expected: unresolved learning types.

- [ ] **Step 3: Implement exact public records**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct NeuromodulatorSample {
    reward_prediction_error: f32,
    pain: f32,
    homeostatic_improvement: f32,
    frustration: f32,
    novelty: f32,
    value: f32,
}

impl NeuromodulatorSample {
    pub fn from_components(
        reward_prediction_error: f32,
        pain: f32,
        homeostatic_improvement: f32,
        frustration: f32,
        novelty: f32,
    ) -> Result<Self, ScaffoldContractError>;
    pub const fn reward_prediction_error(self) -> f32;
    pub const fn pain(self) -> f32;
    pub const fn homeostatic_improvement(self) -> f32;
    pub const fn frustration(self) -> f32;
    pub const fn novelty(self) -> f32;
    pub const fn value(self) -> f32;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutcomeCreditPacket {
    schema_version: u16,
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
    sequence_id: ExperienceSequenceId,
    originating_tick: Tick,
    outcome_tick: Tick,
    frame_digest: PerceptionFrameDigest,
    active_activation_side: u8,
    selected_candidate: u16,
    selected_family: CandidateActionFamily,
    selected_action: ActionId,
    candidate_feature_digest: CandidateFeatureDigest,
    dispatch_generation: u64,
    modulator: NeuromodulatorSample,
}

impl OutcomeCreditPacket {
    pub fn from_sealed_patch(
        patch: &ExperiencePatch,
    ) -> Result<Self, ScaffoldContractError>;
    pub const fn schema_version(&self) -> u16;
    pub const fn organism_id(&self) -> OrganismId;
    pub const fn phenotype_hash(&self) -> PhenotypeHash;
    pub const fn sequence_id(&self) -> ExperienceSequenceId;
    pub const fn originating_tick(&self) -> Tick;
    pub const fn outcome_tick(&self) -> Tick;
    pub const fn frame_digest(&self) -> PerceptionFrameDigest;
    pub const fn active_activation_side(&self) -> u8;
    pub const fn selected_candidate(&self) -> u16;
    pub const fn selected_family(&self) -> CandidateActionFamily;
    pub const fn selected_action(&self) -> ActionId;
    pub const fn candidate_feature_digest(&self) -> CandidateFeatureDigest;
    pub const fn dispatch_generation(&self) -> u64;
    pub const fn modulator(&self) -> NeuromodulatorSample;
    pub const fn replay_key(&self) -> OutcomeCreditReplayKey;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutcomeCreditReplayKey {
    pub organism_id: OrganismId,
    pub phenotype_hash: PhenotypeHash,
    pub sequence_id: ExperienceSequenceId,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LearningCommitToken {
    expected_previous: Option<OutcomeCreditReplayKey>,
    next: OutcomeCreditReplayKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningSequenceGuard {
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
    last_committed: Option<OutcomeCreditReplayKey>,
}

impl LearningSequenceGuard {
    pub fn new(organism_id: OrganismId, phenotype_hash: PhenotypeHash) -> Self;
    pub fn restore_validated(
        organism_id: OrganismId,
        phenotype_hash: PhenotypeHash,
        last_committed: Option<OutcomeCreditReplayKey>,
    ) -> Result<Self, ScaffoldContractError>;
    pub const fn last_committed(&self) -> Option<OutcomeCreditReplayKey>;
    pub fn validate_next(
        &self,
        next: OutcomeCreditReplayKey,
    ) -> Result<LearningCommitToken, ScaffoldContractError>;
    pub fn commit_validated(
        &mut self,
        token: LearningCommitToken,
    ) -> Result<(), ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FastWeightSemantics {
    ImmediateThreeFactor,
}
```

The GPU backend creates one guard when Slice A's
`insert_brain(organism_id, phenotype)` installs the private slot-ownership row;
it passes exactly that organism and phenotype hash to `new`. Every eligibility,
credit, sleep, checkpoint, and restore method preflights its handle/packet
against the same row before consulting the guard. Restore uses
`restore_validated` only after the organism-bound slot and immutable phenotype
are verified. The backend never infers guard ownership from the first credit
packet and never permits a same-phenotype handle to change organisms.

Use a documented weighted sum and clamp to `[-1, 1]`:

```rust
let value = (reward_prediction_error
    - pain
    + 0.75 * homeostatic_improvement
    - 0.5 * frustration
    + 0.2 * novelty)
    .clamp(-1.0, 1.0);
```

All five component inputs must be finite and in `[-1, 1]`.
`NeuromodulatorSample` has private fields and no unchecked construction path.
Its custom `Deserialize` reads a private six-field wire DTO, recomputes `value`
from the five components with the exact formula above, and rejects the row
unless the serialized value is finite, bounded, and bit-equal to the
recomputed value. GPU upload and replay serialization use the accessors, never
the redundant wire value without this validation.

- [ ] **Step 4: Implement replay and mismatch validation**

`OutcomeCreditPacket::from_sealed_patch` has no selection/hash override
parameters; it derives all identity and target evidence from the sealed patch.
The packet fields remain private, the type does not expose public serde
deserialization, and read-only accessors are the only public field access.
Task 4 owns the backend-side POD conversion and must use those accessors. This
makes `from_sealed_patch` the sole public construction path and
prevents callers from fabricating credited neural evidence with a struct
literal or deserialized JSON.
The accessors include `frame_digest()` and `active_activation_side()`; both come
from the sealed neural decision evidence and participate in the full
pending-eligibility match. The side must be 0 or 1 and is the same compact GPU
selection lane carried by Slice A, never reconstructed from a new dispatch.
`LearningSequenceGuard::validate_next` is read-only and returns the opaque,
non-`Clone`, non-`Copy`, non-serializable `LearningCommitToken`. It rejects an
organism or phenotype different from the guard's bound identity and any
sequence not strictly newer than `last_committed`. Only core can construct the
token. After GPU staging passes finite diagnostics and the active-bank
selectors have been compare-and-swapped, the backend consumes that exact token
with `commit_validated`; core rechecks `expected_previous` against current
state before advancing `last_committed`. A stale/reused token fails, and every
rejected case returns a typed contract error without a state mutation receipt.

Add typed `LearningEvidenceMismatch`, `LearningReplayRejected`, and
`ConsolidationGenerationMismatch` variants to `ScaffoldContractError`; callers
must not collapse them into a generic GPU failure.

- [ ] **Step 5: Run focused tests**

Run: `cargo test -p alife_core --test three_factor_learning --test experience_three_phase`

Expected: pass.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_core/src/learning.rs crates/alife_core/src/error.rs crates/alife_core/src/version.rs crates/alife_core/src/lib.rs crates/alife_core/tests/three_factor_learning.rs
git commit -m "Add sealed outcome credit contracts"
```

### Task 2: Add fast-weight and eligibility GPU storage

**Files:**
- Modify: `crates/alife_gpu_backend/src/closed_loop_buffers.rs`
- Create: `crates/alife_gpu_backend/tests/closed_loop_learning_buffers.rs`
- Modify: `crates/alife_core/src/brain_class.rs`
- Modify: `crates/alife_core/src/genome.rs`
- Modify: `crates/alife_core/src/phenotype.rs`
- Modify: `crates/alife_core/tests/genome_weight_split.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_pipeline.rs`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_recurrent.wgsl`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_decode.wgsl`
- Modify: `crates/alife_gpu_backend/tests/closed_loop_wgsl.rs`
- Modify: `crates/alife_gpu_backend/tests/closed_loop_gpu_behavior.rs`

**Interfaces:**
- Consumes: `CompiledSynapse`, per-route plasticity plan, capacity budgets.
- Produces: one lifetime and fast value per active synapse, separate recurrent/decoder eligibility layouts, and optional audit values.

- [ ] **Step 1: Add failing buffer-separation tests**

```rust
#[test]
fn every_active_synapse_has_separate_mutable_learning_layers() {
    let plan = buffer_plan(&n512_capacity());
    assert_eq!(plan.lifetime_weight_count(), plan.synapse_count());
    assert_eq!(plan.fast_weight_count(), plan.synapse_count());
    assert_eq!(
        plan.recurrent_eligibility_count() + plan.decoder_eligibility_count(),
        plan.synapse_count(),
    );
    assert_eq!(plan.genetic_weight_count(), plan.synapse_count());
    assert_ne!(plan.genetic_weights_view(), plan.fast_weights_view());
}

#[test]
fn slot_extension_has_stable_offsets_for_learning_sleep_and_memory() {
    assert_eq!(std::mem::size_of::<GpuBrainSlotExtensionRecord>(), 80);
    assert_eq!(std::mem::align_of::<GpuBrainSlotExtensionRecord>(), 16);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotExtensionRecord, decoder_synapse_local_start), 8);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotExtensionRecord, receptor_offset), 16);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotExtensionRecord, recurrent_eligibility_bank_1_offset), 32);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotExtensionRecord, fast_bank_1_offset), 40);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotExtensionRecord, memory_plan_offset), 52);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotExtensionRecord, learning_state_offset), 60);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotExtensionRecord, pending_eligibility_offset), 64);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotExtensionRecord, replay_plan_identity_offset), 68);
    assert_eq!(std::mem::size_of::<GpuSynapseLearningMetadata>(), 32);
    assert_eq!(std::mem::align_of::<GpuSynapseLearningMetadata>(), 16);
    assert_eq!(std::mem::size_of::<GpuDecoderEligibilityMetadata>(), 32);
    assert_eq!(std::mem::align_of::<GpuDecoderEligibilityMetadata>(), 16);
    assert_eq!(std::mem::size_of::<GpuPlasticityReceptorRecord>(), 32);
    assert_eq!(std::mem::align_of::<GpuPlasticityReceptorRecord>(), 16);
    assert_eq!(std::mem::offset_of!(GpuPlasticityReceptorRecord, fast_min), 20);
    assert_eq!(std::mem::size_of::<GpuSleepParameterRecord>(), 32);
    assert_eq!(std::mem::align_of::<GpuSleepParameterRecord>(), 16);
    assert_eq!(std::mem::offset_of!(GpuSleepParameterRecord, eligibility_reset_policy), 16);
    assert_eq!(std::mem::size_of::<GpuReplayEventRecord>(), 96);
    assert_eq!(std::mem::align_of::<GpuReplayEventRecord>(), 16);
    assert_eq!(std::mem::size_of::<GpuSlotLearningStateRecord>(), 96);
    assert_eq!(std::mem::align_of::<GpuSlotLearningStateRecord>(), 16);
    assert_eq!(std::mem::offset_of!(GpuSlotLearningStateRecord, replay_generation_lo), 40);
    assert_eq!(std::mem::offset_of!(GpuSlotLearningStateRecord, pending_eligibility_offset), 84);
    assert_eq!(std::mem::size_of::<GpuReplayCaptureIdentityRecord>(), 32);
    assert_eq!(std::mem::align_of::<GpuReplayCaptureIdentityRecord>(), 16);
    assert_eq!(
        unpack_replay_eligibility_sample(pack_replay_eligibility_sample(7, -12_345)),
        (7, -12_345),
    );
}

#[test]
fn every_effective_weight_reader_uses_the_active_weight_bank_selector() {
    for source in [CLOSED_LOOP_RECURRENT_WGSL, CLOSED_LOOP_DECODE_WGSL] {
        assert!(source.contains("active_weight_bank"));
        assert!(source.contains("fast_bank_1_offset"));
        assert!(source.contains("lifetime_bank_1_offset"));
        assert!(!reads_bank_zero_mutable_weights_directly(source));
    }
}
```

- [ ] **Step 2: Run and observe missing eligibility API**

Run: `cargo test -p alife_gpu_backend --test closed_loop_learning_buffers`

Expected: compile failure.

- [ ] **Step 3: Extend phenotype learning plans**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PlasticityReceptorPlan {
    eligibility_decay: f32,
    learning_rate: f32,
    sleep_replay_rate: f32,
    normalization_rate: f32,
    modulator_sign: f32,
    fast_weight_min: f32,
    fast_weight_max: f32,
}

impl PlasticityReceptorPlan {
    pub fn try_new(
        eligibility_decay: f32,
        learning_rate: f32,
        sleep_replay_rate: f32,
        normalization_rate: f32,
        modulator_sign: f32,
        fast_weight_min: f32,
        fast_weight_max: f32,
    ) -> Result<Self, ScaffoldContractError>;
    pub const fn eligibility_decay(&self) -> f32;
    pub const fn learning_rate(&self) -> f32;
    pub const fn sleep_replay_rate(&self) -> f32;
    pub const fn normalization_rate(&self) -> f32;
    pub const fn modulator_sign(&self) -> f32;
    pub const fn fast_weight_bounds(&self) -> (f32, f32);
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplayCapturePlan {
    schema_version: u16,
    global_synapse_ids: Vec<u32>,
    samples_per_event: u16,
    event_capacity: u32,
    sample_capacity: u32,
    canonical_digest: [u64; 4],
}

impl ReplayCapturePlan {
    pub const fn schema_version(&self) -> u16;
    pub fn global_synapse_ids(&self) -> &[u32];
    pub const fn samples_per_event(&self) -> u16;
    pub const fn event_capacity(&self) -> u32;
    pub const fn sample_capacity(&self) -> u32;
    pub const fn canonical_digest(&self) -> [u64; 4];
    pub fn validate_against(
        &self,
        phenotype: &BrainPhenotype,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SleepConsolidationPlan {
    schema_version: u16,
    staging_rate: f32,
    weight_limit: f32,
    fast_decay_rate: f32,
    eligibility_reset_policy_raw: u16,
    replay_consume_policy_raw: u16,
    canonical_digest: [u64; 4],
}

impl SleepConsolidationPlan {
    pub fn try_new_v1(
        staging_rate: f32,
        weight_limit: f32,
        fast_decay_rate: f32,
    ) -> Result<Self, ScaffoldContractError>;
    pub const fn schema_version(&self) -> u16;
    pub const fn staging_rate(&self) -> f32;
    pub const fn weight_limit(&self) -> f32;
    pub const fn fast_decay_rate(&self) -> f32;
    pub const fn eligibility_reset_policy_raw(&self) -> u16;
    pub const fn replay_consume_policy_raw(&self) -> u16;
    pub const fn canonical_digest(&self) -> [u64; 4];
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError>;
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuPlasticityReceptorRecord {
    pub eligibility_decay: f32,
    pub learning_rate: f32,
    pub sleep_replay_rate: f32,
    pub normalization_rate: f32,
    pub modulator_sign: f32,
    pub fast_min: f32,
    pub fast_max: f32,
    pub reserved: f32,
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuSleepParameterRecord {
    pub schema_version: u32,
    pub staging_rate: f32,
    pub weight_limit: f32,
    pub fast_decay_rate: f32,
    pub eligibility_reset_policy: u32,
    pub replay_consume_policy: u32,
    pub reserved: [u32; 2],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuReplayEventRecord {
    pub sequence_id: [u32; 2],
    pub originating_tick: [u32; 2],
    pub frame_digest: [u32; 8],
    pub candidate_feature_digest: [u32; 4],
    pub action_id: u32,
    pub family: u32,
    pub reward_prediction_error: f32,
    pub pain: f32,
    pub homeostatic_improvement: f32,
    pub frustration: f32,
    pub novelty: f32,
    pub modulator_value: f32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuReplaySynapseSpanRecord {
    pub local_synapse_id: u32,
    pub sample_start: u32,
    pub sample_count: u32,
    pub reserved: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuSlotLearningStateRecord {
    pub schema_version: u32,
    pub active_weight_bank: u32,
    pub active_eligibility_bank: u32,
    pub pending_valid: u32,
    pub active_weight_generation_lo: u32,
    pub active_weight_generation_hi: u32,
    pub active_eligibility_generation_lo: u32,
    pub active_eligibility_generation_hi: u32,
    pub inactive_eligibility_generation_lo: u32,
    pub inactive_eligibility_generation_hi: u32,
    pub replay_generation_lo: u32,
    pub replay_generation_hi: u32,
    pub replay_cursor: u32,
    pub replay_event_count: u32,
    pub replay_event_capacity: u32,
    pub replay_sample_capacity: u32,
    pub replay_span_count: u32,
    pub replay_event_rows_offset: u32,
    pub replay_sample_offset: u32,
    pub replay_span_offset: u32,
    pub replay_plan_identity_offset: u32,
    pub pending_eligibility_offset: u32,
    pub transaction_generation_lo: u32,
    pub transaction_generation_hi: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuReplayCaptureIdentityRecord {
    pub replay_capture_plan_digest: [u32; 8],
}

pub fn pack_replay_eligibility_sample(
    event_index: u16,
    eligibility_q15: i16,
) -> u32;
pub fn unpack_replay_eligibility_sample(word: u32) -> (u16, i16);

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuBrainSlotExtensionRecord {
    pub schema_version: u32,
    pub projection_count: u32,
    pub decoder_synapse_local_start: u32,
    pub decoder_synapse_count: u32,
    pub receptor_offset: u32,
    pub decoder_input_plan_offset: u32,
    pub decoder_metadata_offset: u32,
    pub synapse_metadata_offset: u32,
    pub recurrent_eligibility_bank_1_offset: u32,
    pub decoder_eligibility_bank_1_offset: u32,
    pub fast_bank_1_offset: u32,
    pub lifetime_bank_1_offset: u32,
    pub sleep_parameter_offset: u32,
    pub memory_plan_offset: u32,
    pub memory_weight_map_offset: u32,
    pub learning_state_offset: u32,
    pub pending_eligibility_offset: u32,
    pub replay_plan_identity_offset: u32,
    pub reserved0: u32,
    pub reserved1: u32,
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuSynapseLearningMetadata {
    pub global_synapse_id: u32,
    pub kind: u32,
    pub source_neuron: u32,
    pub target_neuron: u32,
    pub receptor_index: u32,
    pub eligibility_local_index: u32,
    pub decoder_metadata_local_or_max: u32,
    pub reserved: u32,
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuDecoderEligibilityMetadata {
    pub global_synapse_id: u32,
    pub decoder_head: u32,
    pub family: u32,
    pub input_lane: u32,
    pub motor_index: u32,
    pub receptor_index: u32,
    pub eligibility_local_index: u32,
    pub reserved: u32,
}
```

Mirror `GpuSlotLearningStateRecord`, `GpuReplayCaptureIdentityRecord`,
`GpuReplayEventRecord`, and `GpuReplaySynapseSpanRecord` byte-for-byte in the
replay/plasticity/sleep WGSL modules and reflect exact member offsets with Naga.
The extension's `learning_state_offset` points to the sole selector/generation/
journal/pending-valid authority. That row supplies event/sample/span offsets,
counts, cursor, capacities, generations, and the immutable capture-plan
identity offset. Reserved extension lanes are zero and all arithmetic from
state -> event/span/sample rows is checked before dispatch. Selector flips,
generation increments, replay append/reset, and pending-valid/scrub are one
checked slot-local transaction; tests inject failures at each boundary and
prove the active state row remains byte-identical.

Compile the plan from the genome plasticity mask, alpha mask, developmental
critical periods, and route type. Decoder projection weights must occupy
addressable entries in the same SoA weight/eligibility pools so selected
candidate features receive target-specific credit; do not hide decoder weights
in an immutable side buffer. Nonplastic routes retain aligned buffers but bind
a canonical disabled receptor with `learning_rate == 0`,
`normalization_rate == 0`, and `sleep_replay_rate == 0`; eligibility decay may
remain finite for layout parity but the complete waking and sleep delta is
therefore exactly zero. Add a selector-flip hardware test that gives a disabled
route nonzero activity, eligibility, and modulator and proves both fast banks
remain bit-identical.

Reuse Slice A's private `CompiledSynapseKind::{Recurrent, Decoder(..)}` and
`DecoderHeadKind::{ActionCandidate, MemoryContext}` coordinates, and add a
validated private `receptor_index` to each `CompiledSynapse`. Slice B compiles only `ActionCandidate`; Slice C may
compile `MemoryContext` entries, but both occupy the same canonical global
synapse/weight/eligibility union. Add private
`plasticity_receptors: Vec<PlasticityReceptorPlan>` and
`replay_capture_plan: ReplayCapturePlan` plus
`sleep_consolidation_plan: SleepConsolidationPlan` fields to `BrainPhenotype`,
with read-only `plasticity_receptors()`, `replay_capture_plan()`, and
`sleep_consolidation_plan()` accessors plus
`plasticity_plan_digest()` for the exact ordered receptor-plus-sleep aggregate,
and one
validated receptor reference for every compiled synapse. Both nested types use
private fields, validated construction/custom deserialization, finite and range
checks, and canonical digests; callers cannot splice unchecked receptor or
capture rows into a phenotype.

Use exact stable GPU discriminants:

```rust
impl CompiledSynapseKind {
    pub const RECURRENT_RAW: u32 = 1;
    pub const DECODER_RAW: u32 = 2;
    pub const fn kind_raw(&self) -> u32;
    pub fn validate_kind_raw(raw: u32) -> Result<(), ScaffoldContractError>;
}

impl DecoderHeadKind {
    pub const ACTION_CANDIDATE_RAW: u32 = 1;
    pub const MEMORY_CONTEXT_RAW: u32 = 2;
    pub const fn raw(self) -> u32;
    pub fn try_from_raw(raw: u32) -> Result<Self, ScaffoldContractError>;
}
```

Host conversion writes only those codes to `GpuSynapseLearningMetadata.kind`
and `GpuDecoderEligibilityMetadata.decoder_head`; WGSL constants use the same
values. Custom serde/phenotype validation rejects zero or unknown codes, and
table tests round-trip 1/2 while rejecting 0/3/`u32::MAX`.

Also compile a `ReplayCapturePlan`: a stable sorted subset of replay-eligible
global synapse IDs, a per-event sample quota, event-ring capacity, and total
sample capacity, all within Slice A's replay ceilings. Selection is determined
at phenotype compile time from explicit receptor/route priority and stable
global ID tie breaking; no active-tick atomic race decides which synapses are
remembered. The plan and its capacities participate in the phenotype hash and
`CompiledBudgets`. Extend Slice A's private phenotype wire DTO, canonical
rehash, custom deserializer, backend-insertion validation, immutable phenotype
asset, and compiler-input recompile/equality check to cover every receptor
field plus the complete replay-capture plan. A receptor/capture mutation with
an old phenotype hash is rejected before allocation. The same coverage applies
to every sleep-plan field and digest.

Add one private, versioned `PlasticityGenomeParameters` field to `BrainGenome`
with exact lanes `eligibility_decay`, `base_learning_rate`,
`normalization_rate`, `sleep_replay_rate`, `modulator_sign`, `fast_min`,
`fast_max`, `sleep_staging_rate`, `sleep_weight_limit`, and
`sleep_fast_decay_rate`. Validate decay/rates in `[0,1]` (base learning and
sleep staging are nonzero), modulator sign as exactly `-1` or `1`, ordered fast
bounds inside `[-8,8]`, and sleep weight limit in `(0,8]`. Give the nested type
private fields, read-only accessors, custom validated serde, explicit schema,
and canonical genome hashing. The compiler derives every enabled-route
receptor lane directly from those genes plus the existing route plasticity
mask and deterministic developmental critical-period multiplier; disabled
routes use the exact zero-delta receptor above. `SleepConsolidationPlan` maps
the three sleep genes directly. No host default authors a missing receptor or
sleep lane. Table-driven genome serde/mutation tests change each gene in turn,
require a changed receptor/sleep digest and phenotype hash, and reject stale
schema/hash data.
V1 requires finite `0 < staging_rate <= 1`, finite `0 < weight_limit <= 8`,
finite `0 <= fast_decay_rate <= 1`, eligibility reset policy 1 (zero both
banks), and replay consume policy 1 (consume the complete captured journal).
Its private wire DTO/custom deserializer recomputes the digest and rejects
unknown policies. The immutable phenotype asset, compiler-input asset,
manifest `plasticity_plan_digest`, and evidence loader all bind it.

Define one 32-byte `GpuDecoderEligibilityMetadata` row containing global
synapse ID, decoder-head discriminant, family, flattened input lane, motor
index, receptor index, and reserved lanes. Candidate decode materializes the
exact selected-head inputs into a common class-bucketed
`decoder_learning_inputs` span. Slice C extends that flattened input layout and
metadata count for target-latent/family-value lanes; it must not add a second
eligibility kernel, weight pool, save counter, or staging path.

- [ ] **Step 4: Allocate unified double-bank mutable SoA buffers**

Initialize lifetime, fast, and eligibility arrays to zero. Keep genetic and alpha buffers immutable. An optional audit journal stores bounded deltas, not a second effective-weight layer.
Append bank-1 lifetime, fast, recurrent-eligibility, and decoder-eligibility
spans to the same four class-bucket pools that already contain Slice A's bank-0
spans. Do not create separately bound staging buffers. Allocate one fixed
`GpuSlotLearningStateRecord` per slot, initialize both selector lanes to zero,
all generations/counters to their validated initial values, and validate each
selector as exactly 0 or 1. The canonical resolver
functions are `active_weight_bases`, `inactive_weight_bases`,
`active_eligibility_bases`, and `inactive_eligibility_bases`; all host code and
WGSL use them. The first pair selects lifetime and fast together from
`brain.{lifetime,fast}_weight_offset` for bank 0 or
`extension.{lifetime,fast}_bank_1_offset` for bank 1, selected by the learning
state row. The second pair does the
same for recurrent/decoder eligibility. Winner selection writes only the
resolved inactive eligibility bank. Initialize only the Slice C memory-plan
fields to `u32::MAX`; populate the learning-state replay offsets/capacities,
extension pending/state/identity offsets, immutable replay-plan identity, and
mutable replay span rows from the compiled capture plan in this task. Leave
`sleep_parameter_offset` at the sentinel
until Task 7 installs the validated plan. Validate every extension
offset against its class-bucket pool and require the A slot record's
`extension_record_offset` to resolve this exact record.
Convert every private `PlasticityReceptorPlan` into one exact
`GpuPlasticityReceptorRecord`, require reserved zero plus finite/range-valid
lanes, and bounds-check `receptor_offset + receptor_count` before any shader
read. Task 3 adds this 32-byte row to the shared WGSL ABI prefix and adds Naga member/order/size
tests. Task 7 converts the phenotype-owned `SleepConsolidationPlan` into the
exact 32-byte `GpuSleepParameterRecord`, populates `sleep_parameter_offset`,
and adds/tests that record in the shared WGSL ABI prefix; no shader constant or app default authors
these values.
Update `closed_loop_pipeline.rs` in this task, before any shader reads an
extension. Preserve Slice A's exact seven physical class-bucket bindings:
extension, learning-state, pending, replay event/sample/span rows and bank
selectors occupy checked word spans in `mutable_state_words`; receptor,
synapse/decoder metadata, replay-plan digest identity, and later
sleep-parameter rows occupy `immutable_plan_words`; learning headers
and outcome rows use the existing dispatch-header/frame-payload bindings. The
WGSL semantic helper `load_slot_extension` replaces the illustrative
`slot_extensions[...]` expression below and reconstructs the exact 80-byte row
from `mutable_state_words`. Increment the GPU layout version, update the bind
assembly/min-binding-size contract, and add Rust-versus-Naga tests proving all
seven group/binding/access rows plus the extension helper offsets before the
forward hardware tests run. There is no extra extension buffer binding and no
per-creature bind group.
Update each canonical `BrainCapacityClass` constructor's private
`gpu_layout_version`, recompute its complete execution-ABI digest, recompile the
phenotype/encoder/decoder evidence fixtures, and reject a Slice-A layout or
capacity digest before allocation. The layout bump is one atomic contract
change, not a backend-only constant that can drift from the core capacity.
In this task, migrate every existing Slice A effective-weight reader—not only
the new plasticity pass. `closed_loop_recurrent.wgsl` and
`closed_loop_decode.wgsl` resolve lifetime/fast bases through
`active_weight_bank` before loading `genetic + lifetime + alpha * fast`.
No waking inference shader may continue reading the bank-0 mutable offsets
directly after a selector flip. Slice C applies the same rule to its later
memory-context reader. Parser/source tests and a learned-behavior hardware case
must fail if any active inference path remains pinned to bank 0.
Allocate one bounded per-slot GPU replay journal using the compiled capture
plan: event rows, one synapse-identity span per compiled capture ID, plus packed
event-index/Q15-eligibility samples inside those spans, a cursor, and a journal
generation. It is not synchronously read during waking
ticks and is distinct from the optional human audit journal.
The journal uses the 96-byte `GpuReplayEventRecord`, one preallocated 16-byte
span per replay-capture synapse, and exact low-16-bit event index/high-16-bit
signed Q15 packing helpers defined in this task. Synapse identity is the span's
`local_synapse_id`; it is never squeezed into the packed sample word. Task 4
writes only this ABI; Task 7 reuses it and adds only sleep header/request records. There
is no provisional replay wire format between commits.

- [ ] **Step 5: Run buffer and phenotype tests**

Run: `cargo test -p alife_gpu_backend --test closed_loop_learning_buffers`

Run: `cargo test -p alife_core --test phenotype_compiler`

Run: `cargo test -p alife_core --test genome_weight_split`

Run: `cargo test -p alife_gpu_backend --test closed_loop_wgsl`

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_gpu_behavior -- --nocapture`

Expected: pass.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_core/src/brain_class.rs crates/alife_core/src/genome.rs crates/alife_core/src/phenotype.rs crates/alife_core/tests/genome_weight_split.rs crates/alife_gpu_backend/src/closed_loop_buffers.rs crates/alife_gpu_backend/src/closed_loop_pipeline.rs crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl crates/alife_gpu_backend/shaders/closed_loop_recurrent.wgsl crates/alife_gpu_backend/shaders/closed_loop_decode.wgsl crates/alife_gpu_backend/tests/closed_loop_learning_buffers.rs crates/alife_gpu_backend/tests/closed_loop_wgsl.rs crates/alife_gpu_backend/tests/closed_loop_gpu_behavior.rs
git commit -m "Add GPU fast-weight eligibility storage"
```

### Task 3: Accumulate candidate-specific eligibility on GPU

**Files:**
- Create: `crates/alife_gpu_backend/shaders/closed_loop_eligibility.wgsl`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl`
- Create: `crates/alife_gpu_backend/src/closed_loop_learning.rs`
- Create: `crates/alife_gpu_backend/tests/closed_loop_eligibility.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_pipeline.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_runtime.rs`
- Modify: `crates/alife_gpu_backend/src/lib.rs`

**Interfaces:**
- Consumes: pre/post activation buffers, selected candidate record and feature vector, compiled recurrent/decoder metadata, receptor plan.
- Produces: updated recurrent eligibility, updated decoder-feature eligibility, compact diagnostics, and slot-local `PendingEligibilityReceipt`.

- [ ] **Step 1: Add failing eligibility tests**

```rust
#[test]
fn selected_candidate_builds_target_specific_eligibility() {
    let result = run_eligibility_fixture(0).unwrap();
    assert!(result.recurrent_changed_count > 0);
    assert!(result.decoder_changed_count > 0);
    assert_eq!(result.selected_feature_digest, candidate_feature_digest(0));
    assert_eq!(result.selected_family, candidate_family(0));
    assert_eq!(
        result.tick.pending_eligibility.identity().dispatch_generation(),
        result.tick.dispatch_generation,
    );
}

#[test]
fn eligibility_decay_is_seeded_and_bounded() {
    let first = run_repeated_eligibility_fixture(77).unwrap();
    let second = run_repeated_eligibility_fixture(77).unwrap();
    assert_eq!(first.compact_receipt, second.compact_receipt);
    assert!(first.max_abs <= 1.0);
}

#[test]
fn recurrent_eligibility_uses_explicit_learning_metadata_for_source_and_target() {
    let result = run_non_aliasing_recurrent_fixture().unwrap();
    assert_ne!(result.metadata_source_neuron, result.metadata_target_neuron);
    assert_ne!(result.unrelated_source_pool_offset, result.metadata_offset);
    assert_eq!(result.changed_local_synapse, result.expected_local_synapse);
    assert_eq!(result.guard_canary_violations, 0);
}

#[test]
fn eligibility_uses_final_and_prior_banks_for_two_three_and_four_microsteps() {
    for microsteps in 2..=4 {
        let receipt = run_side_sensitive_eligibility_fixture(microsteps).unwrap();
        assert_eq!(receipt.active_activation_side, expected_final_side(microsteps));
        assert_eq!(receipt.recurrent_digest, expected_pre_post_digest(microsteps));
        assert_eq!(receipt.decoder_digest, expected_decoder_digest(microsteps));
    }
}

#[test]
fn two_same_class_slots_keep_eligibility_isolated() {
    let before_b = two_slot_fixture().slot_b_full_digest();
    let only_a = dispatch_eligibility_only_for_a().unwrap();
    assert!(only_a.slot_a.changed_count > 0);
    assert_eq!(only_a.slot_b_full_digest, before_b);
    let batched = dispatch_eligibility_for_a_and_b().unwrap();
    let independent_a = dispatch_single_slot_reference(Slot::A).unwrap();
    let independent_b = dispatch_single_slot_reference(Slot::B).unwrap();
    assert_eq!(batched.slot_a_full_digest, independent_a.full_digest);
    assert_eq!(batched.slot_b_full_digest, independent_b.full_digest);
    assert_eq!(batched.guard_canary_violations, 0);
}

#[test]
fn learning_header_layout_matches_wgsl() {
    assert_eq!(std::mem::size_of::<GpuLearningHeader>(), 80);
    assert_eq!(std::mem::align_of::<GpuLearningHeader>(), 16);
    assert_eq!(std::mem::offset_of!(GpuLearningHeader, brain_slot_index), 16);
    assert_eq!(std::mem::offset_of!(GpuLearningHeader, outcome_offset), 48);
    assert_eq!(std::mem::offset_of!(GpuLearningHeader, decoder_input_stride), 60);
    assert_eq!(std::mem::offset_of!(GpuLearningHeader, pending_eligibility_offset), 64);
    assert_eq!(std::mem::size_of::<GpuPendingEligibilityRecord>(), 144);
    assert_eq!(std::mem::align_of::<GpuPendingEligibilityRecord>(), 16);
    assert_eq!(std::mem::offset_of!(GpuPendingEligibilityRecord, frame_digest), 72);
    assert_eq!(std::mem::offset_of!(GpuPendingEligibilityRecord, candidate_index_and_family), 104);
}
```

The isolation fixture snapshots one common two-slot neural checkpoint (the two
slots have distinct required organism ownership but byte-identical neural
arrays), then restores that exact checkpoint for the A-only, batched, and
independent-reference runs.
`full_digest` covers active and staging recurrent/decoder eligibility, fast and
lifetime banks, activation A/B, slot/extension records, generation counters,
diagnostics, and guard canaries. It is not a digest of only the expected output
slice.

- [ ] **Step 2: Verify missing kernel**

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_eligibility -- --nocapture`

Expected: missing entry point/API.

- [ ] **Step 3: Implement eligibility WGSL**

Define the exact host/WGSL row first:

```rust
#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuLearningHeader {
    pub schema_version: u32,
    pub class_id: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub brain_slot_index: u32,
    pub active_activation_side: u32,
    pub dispatch_generation_lo: u32,
    pub dispatch_generation_hi: u32,
    pub candidate_count: u32,
    pub candidate_offset: u32,
    pub decoder_learning_input_offset: u32,
    pub selection_offset: u32,
    pub outcome_offset: u32,
    pub recurrent_synapse_count: u32,
    pub decoder_synapse_count: u32,
    pub decoder_input_stride: u32,
    pub pending_eligibility_offset: u32,
    pub reserved: [u32; 3],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuPendingEligibilityRecord {
    pub schema_version: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub active_activation_side: u32,
    pub phenotype_hash: [u32; 8],
    pub organism_id: [u32; 2],
    pub dispatch_generation: [u32; 2],
    pub originating_tick: [u32; 2],
    pub frame_digest: [u32; 8],
    pub candidate_index_and_family: u32,
    pub action_id: u32,
    pub candidate_feature_digest: [u32; 4],
    pub active_eligibility_generation: [u32; 2],
    pub staging_eligibility_generation: [u32; 2],
}
```

Mirror field order in WGSL and add host `size_of`/`offset_of` plus naga layout
tests before dispatch tests.

```wgsl
struct GpuPlasticityReceptorRecord {
    eligibility_decay: f32,
    learning_rate: f32,
    sleep_replay_rate: f32,
    normalization_rate: f32,
    modulator_sign: f32,
    fast_min: f32,
    fast_max: f32,
    reserved: f32,
}

struct GpuPendingEligibilityRecord {
    schema_version: u32,
    slot: u32,
    slot_generation: u32,
    active_activation_side: u32,
    phenotype_hash: array<u32, 8>,
    organism_id: vec2<u32>,
    dispatch_generation: vec2<u32>,
    originating_tick: vec2<u32>,
    frame_digest: array<u32, 8>,
    candidate_index_and_family: u32,
    action_id: u32,
    candidate_feature_digest: vec4<u32>,
    active_eligibility_generation: vec2<u32>,
    staging_eligibility_generation: vec2<u32>,
}
```

`load_plasticity_receptor` reconstructs this exact 32-byte mirror from eight
words in `immutable_plan_words`. Naga member offsets must be
`0,4,8,12,16,20,24,28`; the host converter requires `reserved == 0.0`, finite
lanes, valid bounds, and a receptor index inside the phenotype-owned receptor
span before dispatch.
The pending-row Naga size is exactly 144 bytes with the same offsets asserted
above. Candidate index occupies bits 0..15 and family bits 16..23; bits 24..31
must be zero. One fixed pending row is reserved per occupied slot in
`mutable_state_words` and its checked word offset is carried in the learning
header.
For Slice B, `decoder_input_stride` is exactly the checked conversion of core
`CANDIDATE_FEATURE_COUNT`; no second feature-count header member exists. Slice
C may increase the compiled stride only
after extending the phenotype budget and materialized decoder-input layout.
Every metadata input lane must be below that per-slot stride before any read.
Header recurrent/decoder counts must equal the A slot's recurrent count and
`synapse_count - recurrent_synapse_count`, and the decoder count/start must also
equal the extension record. A mismatch rejects the row before dispatch.
Header `pending_eligibility_offset` must equal both the extension and the
loaded learning-state row. The extension's replay-plan identity offset must
also equal the learning-state row and resolve the phenotype's full capture-plan
digest. Any mismatch rejects the batch before a mutable load or write.

Both entry points use `global_invocation_id.y` to select a validated
slot/generation header and add that header's offsets to every activation,
synapse, receptor, eligibility, candidate, and feature access. Dispatch one
class-bucketed batch; never bind a per-creature buffer set.
Recurrent source, target, receptor, and eligibility-local identity come from
the explicit `GpuSynapseLearningMetadata` base at
`extension.synapse_metadata_offset`. The learning kernel never treats
`brain.source_indices_offset` as a target or metadata base; that A-side CSR
offset remains projection-only.
For recurrent eligibility, the final active side is the postsynaptic bank and
the opposite side is the immediately preceding microstep's presynaptic bank.
Two-, three-, and four-microstep tests cover both parities; no stale fixed A/B
binding or separately maintained CPU "previous activation" exists.

```wgsl
@compute @workgroup_size(64)
fn accumulate_recurrent_eligibility(@builtin(global_invocation_id) gid: vec3<u32>) {
    let header = learning_headers[gid.y];
    let brain = brain_slots[header.brain_slot_index];
    if (brain.slot != header.slot || brain.slot_generation != header.slot_generation) { return; }
    let extension = slot_extensions[brain.extension_record_offset];
    let learning_state = load_slot_learning_state(extension.learning_state_offset);
    let post_activation_offset = select(
        brain.activation_a_offset,
        brain.activation_b_offset,
        header.active_activation_side == 1u,
    );
    let pre_activation_offset = select(
        brain.activation_b_offset,
        brain.activation_a_offset,
        header.active_activation_side == 1u,
    );
    let local_synapse = gid.x;
    if (local_synapse >= header.recurrent_synapse_count) { return; }
    let metadata = synapse_metadata[extension.synapse_metadata_offset + local_synapse];
    if (metadata.kind != SYNAPSE_KIND_RECURRENT
        || metadata.global_synapse_id != local_synapse) { return; }
    let source = pre_activation_offset + metadata.source_neuron;
    let target = post_activation_offset + metadata.target_neuron;
    let active_base = select(
        brain.recurrent_eligibility_offset,
        extension.recurrent_eligibility_bank_1_offset,
        learning_state.active_eligibility_bank == 1u,
    );
    let inactive_base = select(
        extension.recurrent_eligibility_bank_1_offset,
        brain.recurrent_eligibility_offset,
        learning_state.active_eligibility_bank == 1u,
    );
    let active_index = active_base + metadata.eligibility_local_index;
    let staging_index = inactive_base + metadata.eligibility_local_index;
    let receptor_index = extension.receptor_offset
        + metadata.receptor_index;
    let local = activations[source] * activations[target];
    recurrent_eligibility[staging_index] = clamp(
        receptor[receptor_index].eligibility_decay
            * recurrent_eligibility[active_index]
        + local,
        -1.0,
        1.0,
    );
}

@compute @workgroup_size(64)
fn accumulate_decoder_eligibility(@builtin(global_invocation_id) gid: vec3<u32>) {
    let header = learning_headers[gid.y];
    let brain = brain_slots[header.brain_slot_index];
    if (brain.slot != header.slot || brain.slot_generation != header.slot_generation) { return; }
    let extension = slot_extensions[brain.extension_record_offset];
    let learning_state = load_slot_learning_state(extension.learning_state_offset);
    let activation_offset = select(
        brain.activation_a_offset,
        brain.activation_b_offset,
        header.active_activation_side == 1u,
    );
    let local_synapse = gid.x;
    if (local_synapse >= header.decoder_synapse_count) { return; }
    let metadata = decoder_metadata[
        extension.decoder_metadata_offset + local_synapse
    ];
    if (metadata.global_synapse_id
            != extension.decoder_synapse_local_start + local_synapse) { return; }
    let selection = selections[header.selection_offset];
    if (selection.candidate_index >= header.candidate_count) { return; }
    let selected = candidates[header.candidate_offset + selection.candidate_index];
    let active_base = select(
        brain.decoder_eligibility_offset,
        extension.decoder_eligibility_bank_1_offset,
        learning_state.active_eligibility_bank == 1u,
    );
    let inactive_base = select(
        extension.decoder_eligibility_bank_1_offset,
        brain.decoder_eligibility_offset,
        learning_state.active_eligibility_bank == 1u,
    );
    let active_index = active_base + metadata.eligibility_local_index;
    let staging_index = inactive_base + metadata.eligibility_local_index;
    var local = 0.0;
    if (metadata.family == selected.family && metadata.input_lane < header.decoder_input_stride) {
        let feature = decoder_learning_inputs[
            header.decoder_learning_input_offset
            + selection.candidate_index * header.decoder_input_stride
            + metadata.input_lane
        ];
        local = activations[activation_offset + metadata.motor_index] * feature;
    }
    decoder_eligibility[staging_index] = clamp(
        receptor[extension.receptor_offset + metadata.receptor_index].eligibility_decay
            * decoder_eligibility[active_index]
            + local,
        -1.0,
        1.0,
    );
}
```

- [ ] **Step 4: Dispatch eligibility after winner selection**

Recurrent eligibility uses causal waking activity without a transient candidate
mapping. Decoder eligibility reads the selected candidate record and its
features, keyed by persistent decoder candidate-family/feature metadata. A
tick-local candidate index is never compared to a persistent route index.
Neither buffer updates weights until the matching sealed outcome-credit packet
arrives.

Before dispatch, the backend materializes the exact
`GpuPendingEligibilityRecord` from the organism-bound slot ownership, validated
frame, compact selection, and current eligibility generations and uploads it to
that slot's fixed `pending_eligibility_offset`. This is identity transport, not
CPU neural math. Eligibility kernels may read transient candidate/features only
while producing the staged eligibility in the same submission; after clean
diagnostics, the pending row and staged eligibility become one atomic pending
transaction. A failed submission scrubs both. The public
`PendingEligibilityIdentity` and its receipt digest are decoded from that row,
never reconstructed independently. Until apply or discard, the backend rejects
another frame for the slot, so activation state and the pending row cannot be
overwritten.
`pending_valid` is validated as exactly 0 or 1. The clean eligibility commit
sets it to 1 and advances `transaction_generation` with the pending-row write;
apply, discard, and sleep reset set it to 0, scrub the row, and advance the same
generation in their atomic state commit. Plasticity requires 1 before any
mutable read. Add a corruption test with `pending_valid == 0` and nonzero stale
row bytes that produces a typed diagnostic and leaves all banks unchanged.

Store `PendingEligibilityReceipt { handle_generation, phenotype_hash,
dispatch_generation, originating_tick, frame_digest, active_activation_side, candidate_index,
action_id, action_family, candidate_feature_digest,
active_eligibility_generation, staging_eligibility_generation }` for the
slot. Only one unresolved receipt is permitted per slot in Slice B; the live
loop must seal/apply or explicitly discard it before submitting that slot's
next waking frame.

Use these exact backend-owned types and entry points rather than reconstructing
the identity from a subset of tick fields:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingEligibilityIdentity {
    handle_generation: u32,
    phenotype_hash: PhenotypeHash,
    dispatch_generation: u64,
    originating_tick: Tick,
    frame_digest: PerceptionFrameDigest,
    active_activation_side: u8,
    candidate_index: u16,
    action_id: ActionId,
    action_family: CandidateActionFamily,
    candidate_feature_digest: CandidateFeatureDigest,
    active_eligibility_generation: u64,
    staging_eligibility_generation: u64,
}

impl PendingEligibilityIdentity {
    pub const fn handle_generation(&self) -> u32;
    pub const fn phenotype_hash(&self) -> PhenotypeHash;
    pub const fn dispatch_generation(&self) -> u64;
    pub const fn originating_tick(&self) -> Tick;
    pub const fn frame_digest(&self) -> PerceptionFrameDigest;
    pub const fn active_activation_side(&self) -> u8;
    pub const fn candidate_index(&self) -> u16;
    pub const fn action_id(&self) -> ActionId;
    pub const fn action_family(&self) -> CandidateActionFamily;
    pub const fn candidate_feature_digest(&self) -> CandidateFeatureDigest;
    pub const fn active_eligibility_generation(&self) -> u64;
    pub const fn staging_eligibility_generation(&self) -> u64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingEligibilityReceipt {
    identity: PendingEligibilityIdentity,
    receipt_digest: [u64; 4],
}

impl PendingEligibilityReceipt {
    pub const fn identity(&self) -> &PendingEligibilityIdentity;
    pub const fn receipt_digest(&self) -> [u64; 4];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingEligibilityDiscardReceipt {
    pub identity: PendingEligibilityIdentity,
    pub discarded_staging_generation: u64,
    pub hardware_receipt_generation: u64,
    pub receipt_digest: [u64; 4],
}

impl GpuClosedLoopBackend {
    pub fn apply_sealed_outcome(
        &mut self,
        handle: GpuBrainHandle,
        patch: &ExperiencePatch,
    ) -> Result<GpuLearningReceipt, ScaffoldContractError>;
    pub fn pending_eligibility(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<Option<PendingEligibilityReceipt>, ScaffoldContractError>;
    pub fn discard_pending_eligibility(
        &mut self,
        handle: GpuBrainHandle,
        identity: &PendingEligibilityIdentity,
    ) -> Result<PendingEligibilityDiscardReceipt, ScaffoldContractError>;
}
```

Slice B extends Slice A's `GpuClosedLoopTick` with
`pub pending_eligibility: PendingEligibilityReceipt`; it is the exact receipt
created by that dispatch, not a second query of mutable slot state. The
identity and receipt fields are private with read-only accessors for every lane
used by C, save, and telemetry; only the backend constructs them. The canonical
receipt digest covers every identity field.
The GPU row additionally carries full organism identity and the slot-local
generation; host preflight binds both to Slice A's private ownership row before
returning the receipt.
On sealed credit success, apply plasticity from staging and atomically promote
staging eligibility to active with the fast-weight commit. On transaction/seal
failure, drop staging and keep the prior active generation unchanged. Ordinary
world illegality or execution failure is a measured negative outcome and must
still seal and receive credit; it is not a discard case.
The receipt is created only from the validated `GpuClosedLoopTick` compact
selection, including its 0/1 activation side. The eligibility header, decision
evidence, sealed outcome packet, and plasticity header must all match that same
side; a side mismatch leaves both active banks and the pending receipt intact.

- [ ] **Step 5: Run hardware tests**

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_eligibility -- --nocapture`

Expected: pass on the real adapter.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl crates/alife_gpu_backend/shaders/closed_loop_eligibility.wgsl crates/alife_gpu_backend/src/closed_loop_learning.rs crates/alife_gpu_backend/src/closed_loop_pipeline.rs crates/alife_gpu_backend/src/closed_loop_runtime.rs crates/alife_gpu_backend/src/lib.rs crates/alife_gpu_backend/tests/closed_loop_eligibility.rs
git commit -m "Accumulate GPU action eligibility"
```

### Task 4: Apply sealed three-factor fast plasticity on GPU

**Files:**
- Create: `crates/alife_gpu_backend/shaders/closed_loop_plasticity.wgsl`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl`
- Create: `crates/alife_gpu_backend/tests/closed_loop_fast_plasticity.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_learning.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_pipeline.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_runtime.rs`
- Retire/replace: `crates/alife_gpu_backend/src/plasticity.rs`
- Modify: `crates/alife_gpu_backend/src/lib.rs`
- Modify: `crates/alife_gpu_backend/tests/support/mod.rs`
- Delete after replacement tests pass: `crates/alife_gpu_backend/tests/plasticity_oja_parity.rs`

**Interfaces:**
- Consumes: `GpuBrainHandle`, validated `OutcomeCreditPacket`, eligibility, alpha, receptor plan, current fast/effective weights.
- Produces: `GpuClosedLoopBackend::apply_sealed_outcome`, immediate slot-local fast-weight mutation and `GpuLearningReceipt`.

- [ ] **Step 1: Write failing immediate-effect tests**

Use `const GPU_LEARNING_TOLERANCE: f32 = 1.0e-5;` for same-adapter paired
comparisons.
The zero-modulator patch is a normally sealed neutral world outcome, not a
fabricated credit packet. Both ablated and no-credit rows therefore execute the
same unmodulated normalization term; equality between them isolates the missing
three-factor credit while the active row proves the modulated effect.

```rust
#[test]
fn rewarding_outcome_changes_next_encounter_before_sleep() {
    let mut brain = gpu_learning_fixture().unwrap();
    let frame = target_frame();
    let before_tick = brain.tick(&frame).unwrap();
    let before = before_tick.selection;
    let patch = successful_reward_patch(&brain, &frame, &before_tick);
    let receipt = brain.apply_sealed_outcome(&patch).unwrap();
    let after = brain.tick(&target_frame()).unwrap().selection;
    assert!(receipt.fast_weights_changed > 0);
    assert!(after.logit > before.logit);
}

#[test]
fn neuromodulator_ablation_removes_learning_effect() {
    let checkpoint = gpu_learning_checkpoint().unwrap();
    let mut active = restore_gpu_learning_fixture(&checkpoint, 1.0).unwrap();
    let mut ablated = restore_gpu_learning_fixture(&checkpoint, 0.0).unwrap();
    let mut no_credit = restore_gpu_learning_fixture(&checkpoint, 1.0).unwrap();
    let frame = target_frame();
    let active_tick = active.tick(&frame).unwrap();
    let ablated_tick = ablated.tick(&frame).unwrap();
    let no_credit_tick = no_credit.tick(&frame).unwrap();
    let active_before = active_tick.selection.logit;
    let ablated_before = ablated_tick.selection.logit;
    let no_credit_before = no_credit_tick.selection.logit;
    let active_patch = successful_reward_patch(&active, &frame, &active_tick);
    let ablated_patch = successful_reward_patch(&ablated, &frame, &ablated_tick);
    let no_credit_patch = sealed_zero_modulator_patch(&no_credit, &frame, &no_credit_tick);
    active.apply_sealed_outcome(&active_patch).unwrap();
    ablated.apply_sealed_outcome(&ablated_patch).unwrap();
    no_credit.apply_sealed_outcome(&no_credit_patch).unwrap();
    let active_delta = active.tick(&target_frame()).unwrap().selection.logit - active_before;
    let ablated_delta = ablated.tick(&target_frame()).unwrap().selection.logit - ablated_before;
    let no_credit_delta = no_credit.tick(&target_frame()).unwrap().selection.logit - no_credit_before;
    assert!(active_delta > ablated_delta + GPU_LEARNING_TOLERANCE);
    assert!((ablated_delta - no_credit_delta).abs() <= GPU_LEARNING_TOLERANCE);
}

#[test]
fn heterogeneous_same_class_slots_keep_fast_plasticity_isolated() {
    let before_b = two_slot_learning_fixture().slot_b_full_digest();
    let only_a = apply_credit_only_to_a().unwrap();
    assert!(only_a.slot_a.fast_weights_changed > 0);
    assert_eq!(only_a.slot_b_full_digest, before_b);
    let batched = apply_credit_to_a_and_b().unwrap();
    assert_eq!(batched.slot_a_full_digest, apply_single_slot_reference(Slot::A).unwrap().full_digest);
    assert_eq!(batched.slot_b_full_digest, apply_single_slot_reference(Slot::B).unwrap().full_digest);
    assert_eq!(batched.guard_canary_violations, 0);
}
```

As in Task 3, derive A-only, batched, and independent runs from one byte-identical
checkpoint. Full digests include both active/staging fast and eligibility
banks, lifetime/activation state, slot/extension records, sequence guards,
generation counters, diagnostics, and canaries; this test must fail on any
cross-slot write even if selected logits remain unchanged.

- [ ] **Step 2: Run and confirm failure**

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_fast_plasticity -- --nocapture`

Expected: missing sealed-outcome method.

- [ ] **Step 3: Implement outcome-credit GPU record**

Define `GpuOutcomeCreditRecord::try_from(&OutcomeCreditPacket)` in
`closed_loop_learning.rs`, with an ABI layout test in
`closed_loop_fast_plasticity.rs`; the conversion uses only core read-only
accessors. Use a 160-byte, 16-byte-aligned POD record carrying
schema/candidate-family/activation-side lanes,
organism ID, the full four-word phenotype hash split into `u32` lanes,
sequence/origin/outcome ticks, action ID, full candidate-feature and perception-
frame digests, dispatch generation, and all six modulator floats. Validate the
full core packet before upload and compare the full hash against Slice A's
shared `GpuPhenotypeIdentityRecord` at `brain_slot_index`; a hash prefix is
diagnostic display data only.
The WGSL/POD lane names are exactly
`reward_prediction_error`, `pain`, `homeostatic_improvement`, `frustration`,
`novelty`, and `modulator_value`; the first five remain provenance/diagnostic
components and learning multiplies only the already validated bounded
`modulator_value`.

```rust
#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuOutcomeCreditRecord {
    pub schema_version: u32,
    pub selected_candidate_and_family: u32,
    pub organism_id: [u32; 2],
    pub phenotype_hash: [u32; 8],
    pub sequence_id: [u32; 2],
    pub originating_tick: [u32; 2],
    pub outcome_tick: [u32; 2],
    pub selected_action: u32,
    pub active_activation_side: u32,
    pub candidate_feature_digest: [u32; 4],
    pub frame_digest: [u32; 8],
    pub dispatch_generation: [u32; 2],
    pub reward_prediction_error: f32,
    pub pain: f32,
    pub homeostatic_improvement: f32,
    pub frustration: f32,
    pub novelty: f32,
    pub modulator_value: f32,
}
```

Pack candidate index in bits 0..15 and the validated `CandidateActionFamily`
discriminant in bits 16..23 of `selected_candidate_and_family`; bits 24..31
must be zero. Host conversion rejects overflow or an unknown family, WGSL
unpacks both lanes, and the pre-dispatch pending-receipt comparison uses both.

Assert alignment 16, size 160, and offsets 76/80/96/128/156 for activation
side, candidate digest, frame digest, dispatch generation, and
`modulator_value`; mirror them in WGSL and naga layout tests.

- [ ] **Step 4: Implement plasticity WGSL**

Use `global_invocation_id.y` to select the slot/generation learning header and
apply all SoA offsets; reject a stale handle generation before touching
staging. Batch compatible class slots in one dispatch.
Every identity guard shown below writes a slot-local typed diagnostic before
returning; the host requires a clean completion status and never interprets a
guarded no-op as a successful bank commit.

```wgsl
@compute @workgroup_size(64)
fn apply_three_factor_plasticity(@builtin(global_invocation_id) gid: vec3<u32>) {
    let header = learning_headers[gid.y];
    let brain = brain_slots[header.brain_slot_index];
    if (brain.slot != header.slot || brain.slot_generation != header.slot_generation) { return; }
    let extension = slot_extensions[brain.extension_record_offset];
    let learning_state = load_slot_learning_state(extension.learning_state_offset);
    let activation_offset = select(
        brain.activation_a_offset,
        brain.activation_b_offset,
        header.active_activation_side == 1u,
    );
    let local_synapse = gid.x;
    if (local_synapse >= brain.synapse_count) { return; }
    let metadata = synapse_metadata[extension.synapse_metadata_offset + local_synapse];
    let outcome = outcome_credits[header.outcome_offset];
    let identity = phenotype_identities[header.brain_slot_index];
    let pending = load_pending_eligibility(header.pending_eligibility_offset);
    let outcome_choice = unpack_candidate_and_family(outcome.selected_candidate_and_family);
    if (pending.schema_version != PENDING_ELIGIBILITY_SCHEMA_V1
        || outcome.schema_version != OUTCOME_CREDIT_SCHEMA_V1
        || learning_state.pending_valid != 1u
        || pending.slot != header.slot
        || pending.slot_generation != header.slot_generation
        || pending.active_activation_side != header.active_activation_side
        || pending.dispatch_generation[0] != header.dispatch_generation_lo
        || pending.dispatch_generation[1] != header.dispatch_generation_hi
        || !u64_pair_equal(
            pending.active_eligibility_generation,
            learning_state_active_eligibility_generation(learning_state),
        )
        || !u64_pair_equal(
            pending.staging_eligibility_generation,
            learning_state_inactive_eligibility_generation(learning_state),
        )
        || !full_hash_matches(outcome.phenotype_hash, identity.phenotype_hash)
        || !full_hash_matches(outcome.phenotype_hash, pending.phenotype_hash)
        || !u64_pair_equal(outcome.organism_id, pending.organism_id)
        || outcome.active_activation_side != pending.active_activation_side
        || !u64_pair_equal(outcome.dispatch_generation, pending.dispatch_generation)
        || !u64_pair_equal(outcome.originating_tick, pending.originating_tick)
        || !digest_equal(outcome.frame_digest, pending.frame_digest)
        || outcome_choice.candidate_index != unpack_candidate_index(pending)
        || outcome_choice.family != unpack_candidate_family(pending)
        || outcome.selected_action != pending.action_id
        || !candidate_digest_equal(
            outcome.candidate_feature_digest,
            pending.candidate_feature_digest,
        )) { return; }
    let post = activations[activation_offset + metadata.target_neuron];
    let active_fast_base = select(
        brain.fast_weight_offset,
        extension.fast_bank_1_offset,
        learning_state.active_weight_bank == 1u,
    );
    let inactive_fast_base = select(
        extension.fast_bank_1_offset,
        brain.fast_weight_offset,
        learning_state.active_weight_bank == 1u,
    );
    let active_lifetime_base = select(
        brain.lifetime_weight_offset,
        extension.lifetime_bank_1_offset,
        learning_state.active_weight_bank == 1u,
    );
    let inactive_lifetime_base = select(
        extension.lifetime_bank_1_offset,
        brain.lifetime_weight_offset,
        learning_state.active_weight_bank == 1u,
    );
    let effective = genetic[brain.genetic_weight_offset + local_synapse]
        + lifetime[active_lifetime_base + local_synapse]
        + alpha[brain.alpha_offset + local_synapse]
            * fast[active_fast_base + local_synapse];
    let eligibility_value = load_staging_eligibility(extension, metadata, local_synapse);
    let receptor_plan = receptor[extension.receptor_offset + metadata.receptor_index];
    let signed_modulator = outcome.modulator_value * receptor_plan.modulator_sign;
    let delta = receptor_plan.learning_rate
        * alpha[brain.alpha_offset + local_synapse]
        * signed_modulator
        * eligibility_value
        - receptor_plan.normalization_rate * post * post * effective;
    lifetime[inactive_lifetime_base + local_synapse] =
        lifetime[active_lifetime_base + local_synapse];
    fast[inactive_fast_base + local_synapse] = clamp(
        fast[active_fast_base + local_synapse] + delta,
        receptor_plan.fast_min,
        receptor_plan.fast_max,
    );
}
```

Plasticity reads the durable slot-local pending row plus the fresh outcome row;
it must not reread `selection_offset`, `candidate_offset`, candidate features,
or decoder-learning inputs from a transient dispatch ring. Those values have
already been folded into staged eligibility. Successful apply promotes the
staged bank and scrubs the pending row in the same commit; protected discard
scrubs the row and invalid staged generation without changing active banks.

Before dispatch, match the full `OutcomeCreditPacket` against the slot's
`PendingEligibilityReceipt` and generation, including phenotype, dispatch,
organism, origin tick, final frame digest, activation side, selected candidate/action/feature digest,
selected family, and both eligibility generations. Call the slot's core-owned
`LearningSequenceGuard::validate_next` before dispatch and retain the returned
opaque `LearningCommitToken`. Commit the inactive weight bank and matching
inactive eligibility bank only after diagnostics are finite. Then consume that
token with `commit_validated` in the same slot-local host critical section. On
any mismatch or stale token, leave active selectors, generations, replay
journal, and the private guard unchanged and return a typed error.

The commit is a slot-local selector swap at a neural tick boundary: flip
`active_weight_bank` and `active_eligibility_bank` together, then advance all
three generations, replay-journal state, and the guard token. Offsets never
move and never cross buffer bindings. Every staging kernel writes the entire
resolved inactive lifetime/fast/eligibility spans from the current active
banks before applying its delta, so the old active banks are safe as the next
inactive banks. A diagnostic or submission failure changes no selector or
active generation; it scrubs/invalidates only the attempted inactive
generation before retry. This is not an in-place active-weight update.

In the same staged transaction, upload the sealed event metadata and have the
GPU capture Q15 eligibility only for the compiled replay-capture synapse IDs
into the next deterministic journal span. Advancing the replay cursor and
journal generation is part of the same commit as the fast/eligibility swap and
sequence guard. Rejected, replayed, or failed credits append nothing. Waking
ticks never map this journal to the host.

`load_staging_eligibility` switches only on the validated synapse metadata kind
and local eligibility index. It treats action and future Slice C memory decoder
rows identically after the shared flattened decoder input has been materialized;
the kernel never assumes decoder count equals only the original 24 action
features.

Return this compact host receipt; it contains no weight values or hardware
strings:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuLearningReceipt {
    pub handle: GpuBrainHandle,
    pub sequence_id: ExperienceSequenceId,
    pub dispatch_generation: u64,
    pub active_activation_side: u8,
    pub input_fast_generation: u64,
    pub output_fast_generation: u64,
    pub output_eligibility_generation: u64,
    pub replay_journal_generation: u64,
    pub fast_weights_changed: u32,
    pub max_abs_delta: f32,
    pub hardware_receipt_generation: u64,
}
```

Register `closed_loop_plasticity.wgsl` in `GpuClosedLoopPipelines` in this task,
using the same seven-binding layout and generated heap helpers established in
Tasks 2-3. The pipeline constructor validates the shader's reflected
group/binding/access map and minimum word spans before it can be stored. Add a
source/layout test that fails if `apply_fast_plasticity` is absent, registered
under a different layout version, or reaches an eighth buffer binding.

Validate finite receipt floats and exact `input + 1 == output` generations
before returning it.

Extend `GpuTestBrain` with a delegate that calls
`backend.apply_sealed_outcome(handle, patch)`; the wrapper contains no learning
math.

- [ ] **Step 5: Remove product Oja/H-shadow update path**

Delete the superseded CPU Oja executor instead of adding a dormant
`reference-debug` Cargo feature. Remove post-seal GPU conversion that writes
only `h_shadow` and remove `FullGpuRuntimePlasticityReport` dependencies. Once
the new fast-plasticity hardware tests pass, delete
`plasticity_oja_parity.rs`; it must not remain as a product gate, fallback, or
live shadow contract. Pure scalar formula fixtures may live directly inside
unit tests, but no reusable CPU neural/plasticity runtime remains.

- [ ] **Step 6: Run learning tests**

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_fast_plasticity -- --nocapture`

Expected: reward, pain, target specificity, replay rejection, and paired
checkpoint ablation tests pass within the documented GPU tolerance. Exact float
equality is not required.

- [ ] **Step 7: Commit**

```powershell
git add crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl crates/alife_gpu_backend/shaders/closed_loop_plasticity.wgsl crates/alife_gpu_backend/src/closed_loop_learning.rs crates/alife_gpu_backend/src/closed_loop_pipeline.rs crates/alife_gpu_backend/src/closed_loop_runtime.rs crates/alife_gpu_backend/src/plasticity.rs crates/alife_gpu_backend/src/lib.rs crates/alife_gpu_backend/tests/closed_loop_fast_plasticity.rs crates/alife_gpu_backend/tests/plasticity_oja_parity.rs crates/alife_gpu_backend/tests/support/mod.rs crates/alife_core/src/neural.rs
git commit -m "Apply immediate three-factor plasticity on GPU"
```

### Task 5: Wire post-seal GPU learning into the live world loop

**Files:**
- Modify: `crates/alife_game_app/src/live_brain_bridge.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Create: `crates/alife_game_app/tests/gpu_learning_loop.rs`

**Interfaces:**
- Consumes: world execution/outcome, sealed patch, `GpuClosedLoopBackend::apply_sealed_outcome(handle, patch)`.
- Produces: one patch then one matching learning receipt; no learning on failed sealing.

- [ ] **Step 1: Add failing ordering tests**

```rust
#[test]
fn live_loop_seals_before_gpu_learning_commit() {
    let run = run_rewarding_gpu_encounter().unwrap();
    assert_eq!(run.events, ["gpu-select", "world-execute", "patch-seal", "gpu-learn"]);
    assert_eq!(run.learning_receipt.unwrap().sequence_id, run.patch.unwrap().header().sequence_id);
}

#[test]
fn world_illegality_is_sealed_as_negative_credit_not_discarded() {
    let run = run_world_illegal_gpu_encounter().unwrap();
    assert!(run.patch.unwrap().outcome().failure_reason.is_some());
    assert_eq!(run.pending_eligibility_discards, 0);
    assert_eq!(run.learning_receipts, 1);
}

#[test]
fn failed_sealing_discards_only_the_matching_pending_eligibility() {
    let mut loop_ = live_gpu_loop().unwrap();
    let failed = loop_.run_encounter_with_forced_seal_failure().unwrap_err();
    assert_eq!(failed.fast_weight_commits, 0);
    assert_eq!(failed.pending_eligibility_discards, 1);
    assert!(loop_.backend().pending_eligibility(failed.handle).unwrap().is_none());
    assert!(loop_.run_valid_encounter().unwrap().selection.is_some());
}
```

- [ ] **Step 2: Run and verify missing event/receipt wiring**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_learning_loop`

Expected: failure.

- [ ] **Step 3: Implement strict live ordering**

After GPU selection, preserve the existing world legality/execution and
`ExperiencePatchBuilder`; call GPU learning only on the sealed patch. World
legality/execution rejection produces a measured negative outcome and still
seals. Only an infrastructure transaction failure or patch-sealing contract
failure calls
`GpuClosedLoopBackend::discard_pending_eligibility(handle,
gpu_tick.pending_eligibility.identity())`.
That method requires an exact match with the slot's complete stored identity;
mismatch leaves the receipt
untouched and returns a typed error. A successful discard clears eligibility
for only that slot/dispatch so the next waking frame can run. Store learning or
discard receipt IDs in telemetry, not weight values.

- [ ] **Step 4: Run app tests**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_learning_loop`

Expected: pass.

- [ ] **Step 5: Commit**

```powershell
git add crates/alife_game_app/src/live_brain_bridge.rs crates/alife_game_app/src/gpu_live_runtime.rs crates/alife_game_app/tests/gpu_learning_loop.rs
git commit -m "Apply GPU learning after sealed outcomes"
```

### Task 6: Advance sleep automatically in the canonical scheduler

**Files:**
- Modify: `crates/alife_core/src/sleep.rs:21-312`
- Modify: `crates/alife_core/src/reference_brain.rs` only to retire external production sleep hooks.
- Modify: `crates/alife_game_app/src/live_brain_bridge.rs`
- Modify: `crates/alife_world/src/headless.rs:1717-1762`
- Create: `crates/alife_game_app/tests/automatic_sleep_scheduler.rs`

**Interfaces:**
- Consumes: homeostasis, tick, existing `SleepController` transitions.
- Produces: `GpuSleepScheduleEvent`, no-action sleeping ticks, unique cycle IDs,
  a single GPU-neutral `ConsolidationIntent`, and crash-safe
  `ConsolidationState`.

- [ ] **Step 1: Write the failing scheduler-cycle test against a test driver**

```rust
#[test]
fn fatigue_enters_sleep_requests_once_emits_no_actions_and_wakes_after_completion() {
    let mut scheduler = fatigued_sleep_scheduler();
    let mut driver = RecordingConsolidationDriver::completes_valid_jobs();
    let events = scheduler.run_until_awake_after_cycle(&mut driver, 64).unwrap();
    assert!(events.iter().any(|e| e.phase == SleepPhase::EnteringSleep));
    assert_eq!(driver.intents().len(), 1);
    let consolidating = events.iter()
        .find(|event| event.phase == SleepPhase::Consolidating)
        .unwrap();
    assert_eq!(driver.intents()[0].cycle_id, consolidating.cycle_id);
    assert_eq!(events.last().unwrap().phase, SleepPhase::Awake);
    assert!(events.iter().filter(|e| e.phase != SleepPhase::Awake).all(|e| e.selected_action.is_none()));
}
```

`RecordingConsolidationDriver` is test support only. It records one typed
intent, fabricates deterministic test-only prepared/submitted/completed events,
and injects a validated completion receipt so this task can prove the
state machine without calling a GPU consolidation API that Task 7 has not yet
implemented. The production scheduler emits intents and consumes validated
prepared/submitted/completed driver events; only the backend authors requests.
The scheduler does not own wgpu resources.

- [ ] **Step 2: Confirm current harness stalls**

Run: `cargo test -p alife_game_app --features gpu-runtime --test automatic_sleep_scheduler`

Expected: failure because the canonical scheduler intent/driver seam does not
exist.

- [ ] **Step 3: Extend sleep state with crash-safe consolidation identity**

Add the following versioned state with serde migration from the old phase-only
record:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsolidationState {
    None,
    Pending {
        intent: ConsolidationIntent,
        replay_digest: [u64; 4],
        replay_event_count: u32,
        replay_eligibility_sample_count: u32,
    },
    Prepared {
        request: GpuConsolidationRequest,
    },
    Submitted {
        request: GpuConsolidationRequest,
        job_id: ConsolidationJobId,
    },
    Completed {
        request: GpuConsolidationRequest,
        staged: ConsolidationStagedOutput,
    },
    Committed {
        cycle_id: u64,
        output_generation: u64,
        output_digest: [u64; 4],
    },
}

impl ConsolidationState {
    pub const fn kind_raw(&self) -> u16;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsolidationIntent {
    pub cycle_id: u64,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct ConsolidationJobId(NonZeroU64);

impl ConsolidationJobId {
    pub fn try_from_raw(value: u64) -> Result<Self, ScaffoldContractError>;
    pub const fn raw(self) -> u64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuConsolidationRequest {
    pub schema_version: u16,
    pub request_flags: u16,
    pub cycle_id: u64,
    pub phenotype_hash: PhenotypeHash,
    pub input_generation: u64,
    pub expected_output_generation: u64,
    pub input_digest: [u64; 4],
    pub replay_digest: [u64; 4],
    pub max_replay_events: u32,
    pub max_replay_eligibility_samples: u32,
    pub request_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsolidationStagedOutput {
    pub job_id: ConsolidationJobId,
    pub output_generation: u64,
    pub output_weight_bank: u8,
    pub output_digest: [u64; 4],
    pub eligibility_reset_generation: u64,
    pub output_eligibility_bank: u8,
    pub eligibility_output_digest: [u64; 4],
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub replay_journal_output_digest: [u64; 4],
    pub staging_digest: [u64; 4],
    pub promoted_fast_l1_bits: u32,
    pub replay_induced_fast_l1_bits: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SleepReplayEvent {
    pub sequence_id: ExperienceSequenceId,
    pub originating_tick: Tick,
    pub frame_digest: PerceptionFrameDigest,
    pub candidate_feature_digest: CandidateFeatureDigest,
    pub action_id: ActionId,
    pub family: CandidateActionFamily,
    pub modulator: NeuromodulatorSample,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayEligibilitySample {
    pub event_index: u16,
    pub eligibility_q15: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplaySynapseSpan {
    pub local_synapse_id: u32,
    pub sample_start: u32,
    pub sample_count: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundedReplayBatch {
    pub schema_version: u16,
    pub events: Vec<SleepReplayEvent>,
    pub synapse_spans: Vec<ReplaySynapseSpan>,
    pub eligibility_samples: Vec<ReplayEligibilitySample>,
    pub canonical_digest: [u64; 4],
}
```

Give `SleepPhase` an explicit `#[repr(u16)]` mapping Awake=1,
EnteringSleep=2, Consolidating=3, Waking=4, ForcedRecoverySleep=5 plus
`raw()`/validated `try_from_raw()`. `ConsolidationState::kind_raw()` maps
None/Pending/Prepared/Submitted/Completed/Committed to 0/1/2/3/4/5. Add exact
round-trip tests; these values are persistence/evidence ABI, never declaration
order casts.

`ConsolidationJobId` implements custom `Deserialize` by reading a `u64` and
calling `try_from_raw`; zero is rejected and there is no unchecked serde path.
Add negative JSON and binary-format tests proving zero cannot round-trip into a
job ID. It is
process-local provenance, not a capability after restart; a restored Submitted
state follows the recorded-request rebuild rule in Task 8 rather than assuming
the old job object still exists.

`BoundedReplayBatch::validate_contract(max_events, max_samples, synapse_count)`
requires ordered unique sequence IDs, finite bounded modulators, sorted unique
in-range span synapse IDs, monotonic disjoint sample spans with exact full sample
coverage, in-range event indices, Q15 eligibility values, all length limits,
and a matching canonical little-endian digest. Samples preserve selected saved
eligibility by replay event; they do not contain a CPU-computed
eligibility-times-modulator result. The GPU replay kernel performs that product
and per-synapse reduction. `GpuConsolidationRequest.replay_digest` must equal that digest
and both maxima must equal Slice A's canonical phenotype execution budget.
`ReplayCapturePlan` requires a nonempty sorted capture-ID set,
`samples_per_event == global_synapse_ids.len()` by checked `u16` conversion,
`1 <= event_capacity <= 65_536`, and checked
`sample_capacity == event_capacity * samples_per_event` within the phenotype
budget. The portable `ReplaySynapseSpan.sample_count` is `u32`, exactly matching
the GPU row; `reserved` must be zero in both forms.

The journal is a deterministic physical ring. The next event writes physical
slot `cursor`, and each capture span overwrites exactly
`sample_start + cursor`; then `cursor = (cursor + 1) % event_capacity` and the
logical event count saturates at capacity. On overwrite, every span's sample
for that physical event slot is replaced in the same commit, so no old sample
can remain attached to a new event. `build_sleep_replay_batch` emits events in
oldest-to-newest order (`0..count` before first wrap, otherwise
`cursor..capacity` then `0..cursor`) and canonically reindexes every packed
physical event index to the emitted `u16` batch index. Q15 encoding first clamps
finite eligibility to `[-1,1]`, multiplies by 32_767, rounds half away from
zero, and clamps to `[-32_767,32_767]`; decoding divides by 32_767. Add exact
tests for -1, -0.5, negative zero, 0, 0.5, 1, the 65_536-event boundary,
checked-multiply overflow, one wrap, multiple wraps, chronological reindexing,
and replacement of every overwritten span sample.
`schema_version` is exact and unknown request flags are rejected.
`expected_output_generation` is checked `input_generation + 1` and
`request_digest` is a domain-separated canonical digest of every other request
field. The serializable request is a portable validation record, not a
capability: the scheduler has no constructor/transition that authors one, and
every backend entry point rebinds every field to its private handle/slot state
before dispatch. A fabricated but internally self-consistent DTO is rejected
unless it exactly equals that backend-derived binding.

`GpuConsolidationRequest.input_digest` has one exact definition shared by
prepare, persistence, restart recovery, acceptance, and evidence validation.
Using domain `ALIFE-GPU-SLEEP-INPUT-V1`, hash the complete phenotype hash,
capacity/class ID, active weight generation, active 0/1 weight-bank selector,
synapse count, then every resolved active lifetime and fast `f32` lane in
global-synapse order after finite validation and negative-zero normalization.
Length-prefix both arrays; never include allocation-local offsets or the
inactive bank. The staged output digest uses the parallel
`ALIFE-GPU-SLEEP-OUTPUT-V1` domain, expected output generation/new selector,
and every resolved inactive lifetime/fast lane. One canonical helper computes
each digest at all boundaries; a summary-only generation or producer-supplied
digest is not accepted.

`ConsolidationStagedOutput::staging_digest` uses domain
`ALIFE-GPU-SLEEP-STAGING-V1` and hashes, in order: request digest, saved job ID,
cycle ID, phenotype hash, output weight generation/selector/digest, eligibility
reset generation/selector/full-output digest, replay journal
generation/cursor/event-count/full-output digest, the phenotype-owned
eligibility-reset and replay-consume policy raws, `promoted_fast_l1_bits`, and
`replay_induced_fast_l1_bits`. Both diagnostic floats are finite nonnegative
values stored as canonical normalized bits. `GpuSleepConsolidationReceipt`'s
`commit_digest` uses domain `ALIFE-GPU-SLEEP-COMMIT-V1` and hashes that staging
digest plus the committed phenotype/organism/cycle, active weight and
eligibility generations/selectors, replay generation/cursor/event count, and
the complete post-commit mutable-state digest. The same named helpers are used
by poll, Completed persistence/load, process-loss staging restoration, commit,
final manifest CAS, and evidence validation; no layer recomputes an informal
subset.

`SleepState` stores `active_cycle_id`, `last_consolidated_cycle_id`, and
`consolidation`. Increment the cycle only on Awake-to-sleep transition.
On transition into Consolidating, the scheduler first persists
`ConsolidationState::None` and emits the intent. The driver snapshots the
committed replay journal, durably writes the exact
bounded replay asset, and only then persists `Pending` with that asset's digest
and exact counts. A crash before the Pending write restarts from EnteringSleep
or Consolidating/None and may deterministically rebuild the asset; a persisted Pending can never lack
its matching digest-checked replay asset. Only the backend may transition
Pending to Prepared by binding current phenotype/weight generation and that
same replay digest. Prepared may
submit; `Submitted` may only poll or recover the recorded job;
`Completed` may validate and swap exactly the recorded generations; `Committed`
may advance toward Waking but never submit or swap again.

- [ ] **Step 4: Integrate evaluation and advance into every scheduled tick**

The app evaluates homeostasis before waking dispatch, advances non-awake phases
on every world tick, and emits no command while sleeping. On entering
`Consolidating` with an unseen cycle ID it emits exactly one
`ConsolidationIntent`, remains durably Consolidating/None until the driver's replay
asset receipt is valid, then atomically persists the receipt and `Pending`.
It accepts a backend-authored
Prepared request, and advances to Submitted only after a driver supplies a job
ID. It advances to Completed/Committed/Waking
only from matching validated driver events. Persist each progress transition
before the next transition. Remove `HeadlessBrainHarness` forced-sleep
behavior. Task 7 binds this seam to the shared GPU backend and owns device-loss
recovery/generation compare-and-swap.

- [ ] **Step 5: Run sleep tests**

Run: `cargo test -p alife_core --test sleep_consolidation`

Run: `cargo test -p alife_game_app --features gpu-runtime --test automatic_sleep_scheduler`

Expected: pass.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_core/src/sleep.rs crates/alife_core/src/reference_brain.rs crates/alife_world/src/headless.rs crates/alife_game_app/src/live_brain_bridge.rs crates/alife_game_app/tests/automatic_sleep_scheduler.rs
git commit -m "Advance sleep in the canonical GPU brain loop"
```

### Task 7: Consolidate GPU fast weights exactly once and swap safely

**Files:**
- Create: `crates/alife_gpu_backend/src/closed_loop_sleep.rs`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl`
- Create: `crates/alife_gpu_backend/shaders/closed_loop_consolidate.wgsl`
- Create: `crates/alife_gpu_backend/shaders/closed_loop_replay_learning.wgsl`
- Create: `crates/alife_gpu_backend/tests/closed_loop_sleep.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_runtime.rs`
- Modify: `crates/alife_gpu_backend/src/recompaction.rs`
- Modify: `crates/alife_gpu_backend/src/lib.rs`
- Modify: `crates/alife_gpu_backend/tests/support/mod.rs`
- Modify: `crates/alife_game_app/src/live_brain_bridge.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Create: `crates/alife_game_app/tests/automatic_gpu_sleep.rs`
- Modify: `crates/alife_game_app/Cargo.toml`

**Interfaces:**
- Consumes: `GpuBrainHandle`, `ConsolidationIntent`, backend-authored
  `GpuConsolidationRequest`,
  slot-local lifetime/fast buffers and bounded replay batch.
- Produces: `GpuSleepStagingReceipt`, staged lifetime/fast buffers,
  `GpuSleepConsolidationReceipt`, and an exactly-once selector commit with no
  structural phenotype change.

The production backend API is exactly:

```rust
pub type GpuSleepJobId = ConsolidationJobId;

impl GpuClosedLoopBackend {
    pub fn build_sleep_replay_batch(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<BoundedReplayBatch, ScaffoldContractError>;

    pub fn prepare_sleep_consolidation(
        &self,
        handle: GpuBrainHandle,
        intent: ConsolidationIntent,
        replay: &BoundedReplayBatch,
    ) -> Result<GpuConsolidationRequest, ScaffoldContractError>;

    pub fn submit_sleep_consolidation(
        &mut self,
        handle: GpuBrainHandle,
        request: &GpuConsolidationRequest,
        replay: &BoundedReplayBatch,
    ) -> Result<GpuSleepJobId, ScaffoldContractError>;

    pub fn recover_submitted_sleep_consolidation(
        &mut self,
        handle: GpuBrainHandle,
        request: &GpuConsolidationRequest,
        replay: &BoundedReplayBatch,
        lost_process_job_id: GpuSleepJobId,
    ) -> Result<GpuSleepJobId, ScaffoldContractError>;

    pub fn poll_sleep_consolidation(
        &mut self,
        handle: GpuBrainHandle,
        job_id: GpuSleepJobId,
    ) -> Result<Option<GpuSleepStagingReceipt>, ScaffoldContractError>;

    pub fn commit_sleep_consolidation(
        &mut self,
        handle: GpuBrainHandle,
        request: &GpuConsolidationRequest,
        staged: &ConsolidationStagedOutput,
    ) -> Result<GpuSleepConsolidationReceipt, ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuSleepStagingReceipt {
    pub handle: GpuBrainHandle,
    pub cycle_id: u64,
    pub phenotype_hash: PhenotypeHash,
    pub input_generation: u64,
    pub input_digest: [u64; 4],
    pub replay_digest: [u64; 4],
    pub staged: ConsolidationStagedOutput,
    pub hardware_receipt_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuSleepConsolidationReceipt {
    pub staged: GpuSleepStagingReceipt,
    pub output_generation: u64,
    pub output_digest: [u64; 4],
    pub promoted_fast_l1: f32,
    pub replay_induced_fast_l1: f32,
    pub generation_swaps: u32,
    pub active_weight_bank: u8,
    pub eligibility_reset_generation: u64,
    pub active_eligibility_bank: u8,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub commit_digest: [u64; 4],
}
```

Polling never changes an active selector. It returns `None` before staging is
complete and thereafter the same `GpuSleepStagingReceipt`, whose output and
staging digests cover the complete inactive lifetime/fast banks. The app
durably persists its engine-neutral `ConsolidationStagedOutput` plus staging
assets as `Completed` before calling
`commit_sleep_consolidation`. Commit compare-and-swaps the exact input
generation/digest/selector to the staged output once and returns
`GpuSleepConsolidationReceipt { generation_swaps: 1, .. }`. A retry after the
same output is already active returns the identical commit receipt with no
second flip; any different request/staging tuple is rejected. Neither receipt
contains full hardware strings or weight arrays.
Sleep may begin only with no unresolved waking `PendingEligibilityReceipt`.
The staged commit deterministically zeroes both recurrent-eligibility banks and
both decoder-eligibility banks, sets the active eligibility selector to zero,
and advances the eligibility reset generation once. It also consumes the
entire bounded replay journal snapshot used by the request, clears its event
and sample rows, resets its cursor, and advances the journal generation once.
It preserves (or deterministically rebuilds from the immutable
`ReplayCapturePlan`) the sorted one-per-capture-synapse identity span rows,
including each `local_synapse_id` and reserved sample range, while setting
every `sample_count` to zero. The extension's `replay_span_offset` therefore
continues to resolve a valid append layout after wake.
The final receipt and commit digest cover both resets. The first post-wake
outcome therefore cannot reuse pre-sleep credit, and a later sleep cycle cannot
replay the same journal rows.
`build_sleep_replay_batch` is a sleep-boundary operation: it waits for the
slot's last waking learning generation, maps only the bounded replay journal,
orders/validates the already captured event/eligibility records, and computes
their canonical serialization digest. It performs no neural formula,
eligibility-times-modulator multiplication, or weight update on CPU. The
returned batch generation/digest is bound by `prepare_sleep_consolidation` and
the saved `GpuConsolidationRequest`.

`recover_submitted_sleep_consolidation` is the only restart path for a
persisted Submitted state. It requires a newly restored handle whose active
generation/digest equal the request input, validates the immutable phenotype,
exact replay batch, cycle, request digest, and lost nonzero job ID, scrubs the
inactive banks, and resubmits the same request bytes to a new process-local job
ID. It persists the replacement Submitted job ID before polling. The ordinary
submit API rejects an already-recorded Submitted request; recovery cannot run
when the active generation is already the expected output generation.

- [ ] **Step 1: Add failing exactly-once tests**

```rust
#[test]
fn consolidation_promotes_fast_without_mutating_genetic_weights() {
    let mut brain = learned_gpu_fixture().unwrap();
    let genetic_before = brain.manual_genetic_digest().unwrap();
    let intent = ConsolidationIntent { cycle_id: 1 };
    let request = brain.prepare_sleep_consolidation(intent, &replay_batch()).unwrap();
    let receipt = brain.submit_and_complete_sleep_consolidation(&request, &replay_batch()).unwrap();
    assert!(receipt.promoted_fast_l1 > 0.0);
    assert_eq!(genetic_before, brain.manual_genetic_digest().unwrap());
    assert!(brain.submit_and_complete_sleep_consolidation(&request, &replay_batch()).is_err());
}

#[test]
fn invalid_staging_output_rolls_back_without_generation_swap() {
    let mut brain = learned_gpu_fixture_with_corrupt_staging().unwrap();
    let before = brain.active_learning_generation_digest().unwrap();
    let intent = ConsolidationIntent { cycle_id: 2 };
    let request = brain.prepare_sleep_consolidation(intent, &replay_batch()).unwrap();
    assert!(brain.submit_and_complete_sleep_consolidation(&request, &replay_batch()).is_err());
    assert_eq!(before, brain.active_learning_generation_digest().unwrap());
    assert_eq!(brain.generation_swap_count(), 0);
}

#[test]
fn wake_starts_with_no_pre_sleep_eligibility_or_replay_rows() {
    let result = complete_learned_sleep_cycle().unwrap();
    assert_eq!(result.recurrent_eligibility_nonzero, 0);
    assert_eq!(result.decoder_eligibility_nonzero, 0);
    assert_eq!(result.replay_journal_rows, 0);
    assert!(result.eligibility_reset_generation > result.input_eligibility_generation);
}

#[test]
fn second_sleep_without_new_learning_cannot_replay_the_first_cycle() {
    let first = complete_learned_sleep_cycle().unwrap();
    let second = complete_sleep_cycle_without_new_waking_credit(first.checkpoint).unwrap();
    assert_eq!(second.replay_event_count, 0);
    assert_eq!(second.replay_induced_fast_l1, 0.0);
    assert_ne!(first.cycle_id, second.cycle_id);
}

#[test]
fn two_sleeping_same_class_slots_consolidate_without_cross_slot_writes() {
    let before_b = two_sleep_slot_fixture().slot_b_full_digest();
    let only_a = consolidate_only_a().unwrap();
    assert_eq!(only_a.slot_a.generation_swaps, 1);
    assert_eq!(only_a.slot_b_full_digest, before_b);
    let batched = consolidate_a_and_b().unwrap();
    assert_eq!(batched.slot_a_full_digest, consolidate_single_reference(Slot::A).unwrap().full_digest);
    assert_eq!(batched.slot_b_full_digest, consolidate_single_reference(Slot::B).unwrap().full_digest);
    assert_eq!(batched.guard_canary_violations, 0);
}

#[test]
fn replay_learning_payload_changes_post_wake_behavior() {
    let checkpoint = learned_sleep_checkpoint().unwrap();
    let replayed = consolidate_from_checkpoint(&checkpoint, replay_batch()).unwrap();
    let ablated = consolidate_from_checkpoint(&checkpoint, replay_batch_with_zero_samples()).unwrap();
    assert!(replayed.post_wake_target_delta > ablated.post_wake_target_delta + replayed.tolerance);
}

#[test]
fn sleep_header_layout_matches_wgsl() {
    assert_eq!(std::mem::size_of::<GpuSleepHeader>(), 80);
    assert_eq!(std::mem::align_of::<GpuSleepHeader>(), 16);
    assert_eq!(std::mem::offset_of!(GpuSleepHeader, brain_slot_index), 16);
    assert_eq!(std::mem::offset_of!(GpuSleepHeader, request_offset), 20);
    assert_eq!(std::mem::offset_of!(GpuSleepHeader, replay_span_offset), 32);
    assert_eq!(std::mem::offset_of!(GpuSleepHeader, replay_sample_count), 44);
    assert_eq!(std::mem::offset_of!(GpuSleepHeader, job_id_lo), 56);
    assert_eq!(std::mem::offset_of!(GpuSleepHeader, flags), 72);
    assert_eq!(std::mem::size_of::<GpuConsolidationRequestRecord>(), 176);
    assert_eq!(std::mem::align_of::<GpuConsolidationRequestRecord>(), 16);
    assert_eq!(std::mem::offset_of!(GpuConsolidationRequestRecord, phenotype_hash), 16);
    assert_eq!(std::mem::offset_of!(GpuConsolidationRequestRecord, request_digest), 136);
    assert_eq!(std::mem::size_of::<GpuReplayEventRecord>(), 96);
    assert_eq!(std::mem::align_of::<GpuReplayEventRecord>(), 16);
    assert_eq!(std::mem::offset_of!(GpuReplayEventRecord, modulator_value), 92);
    assert_eq!(std::mem::size_of::<GpuReplaySynapseSpanRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuReplaySynapseSpanRecord>(), 16);
}
```

The sleep isolation fixture restores one neural-state-identical two-slot
checkpoint with distinct organism ownership for
A-only, batched, and independent runs. Its full digest covers active/staging
lifetime and fast banks, eligibility and activation banks, replay uploads,
slot/extension records, job/cycle/generation state, diagnostics, and guard
canaries. The A-only dispatch must leave all B bytes and generations unchanged.

Also write the failing retained-behavior case now: train a target preference,
consolidate, resume through Waking to Awake, and assert the post-wake target
logit remains changed relative to the pre-training checkpoint.

In `automatic_gpu_sleep.rs`, prewrite the real hardware cycle: fatigue a live
N512 handle, require one Submitted GPU job and one committed generation swap,
automatic wake, no non-awake action, and retained learned behavior. This is the
first test that expects the scheduler intent/driver seam to call GPU
consolidation.

- [ ] **Step 2: Verify missing consolidation API**

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_sleep -- --nocapture`

Expected: missing method.

- [ ] **Step 3: Implement GPU consolidation kernel**

Define and mirror this exact row:

Reuse Task 2's single `GpuReplayEventRecord`,
`GpuReplaySynapseSpanRecord`, and packed-sample helpers from
`closed_loop_buffers`; Task 7 must not redefine them. Its naga tests repeat the
event/span layouts and pack/unpack assertions against the sleep shader before
adding only these sleep-specific records:

```rust
#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuSleepHeader {
    pub schema_version: u32,
    pub class_id: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub brain_slot_index: u32,
    pub request_offset: u32,
    pub replay_event_offset: u32,
    pub replay_event_count: u32,
    pub replay_span_offset: u32,
    pub replay_span_count: u32,
    pub replay_sample_offset: u32,
    pub replay_sample_count: u32,
    pub synapse_count: u32,
    pub completion_offset: u32,
    pub job_id_lo: u32,
    pub job_id_hi: u32,
    pub cycle_id_lo: u32,
    pub cycle_id_hi: u32,
    pub flags: u32,
    pub reserved: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuConsolidationRequestRecord {
    pub schema_version: u32,
    pub request_flags: u32,
    pub cycle_id_lo: u32,
    pub cycle_id_hi: u32,
    pub phenotype_hash: [u32; 8],
    pub input_generation_lo: u32,
    pub input_generation_hi: u32,
    pub expected_output_generation_lo: u32,
    pub expected_output_generation_hi: u32,
    pub input_digest: [u32; 8],
    pub replay_digest: [u32; 8],
    pub max_replay_events: u32,
    pub max_replay_eligibility_samples: u32,
    pub request_digest: [u32; 8],
    pub reserved_tail: [u32; 2],
}

```

Add the Task 2 sleep parameter row once to `closed_loop_abi.wgsl`; both sleep
shader assemblies consume that shared prefix:

```wgsl
struct GpuSleepParameterRecord {
    @align(16) schema_version: u32,
    staging_rate: f32,
    weight_limit: f32,
    fast_decay_rate: f32,
    eligibility_reset_policy: u32,
    replay_consume_policy: u32,
    reserved: vec2<u32>,
}
```

At brain insertion, convert the private phenotype
`SleepConsolidationPlan` to this exact row and replace the extension's sentinel
with its bounds-checked pool offset. Require schema 1, policies 1/1, reserved
zero, finite/range-valid floats, matching plan digest, and naga/bytemuck
size/alignment/member-order parity before dispatch.

The backend validates the sparse batch and uploads its events, unique
synapse spans, and event-indexed Q15 eligibility samples to the three exact
header offsets. It does not multiply eligibility by a modulator or aggregate a
neural weight delta on CPU. First, one invocation per global synapse copies the
resolved active lifetime/fast spans into the resolved inactive spans in their
shared pools. Then `replay_sleep_learning` dispatches one
invocation per unique span; that invocation loops only its span's samples,
loads each referenced replay event, computes
`eligibility_q15 * event.modulator_value`, reduces in stable event order, applies
the synapse alpha/receptor replay rate, clamps, and writes its one staging
synapse. No float atomic or cross-span write is required. Consolidation runs
after both passes. This is the required behavioral use of replay, not
provenance-only authentication.

Use this computation shape in `closed_loop_replay_learning.wgsl` (the host
packs `event_index` and signed Q15 eligibility into one documented little-endian
`u32` and layout tests cover unpacking):

```wgsl
@compute @workgroup_size(64)
fn replay_sleep_learning(@builtin(global_invocation_id) gid: vec3<u32>) {
    let header = sleep_headers[gid.y];
    let brain = brain_slots[header.brain_slot_index];
    if (brain.slot != header.slot || brain.slot_generation != header.slot_generation) { return; }
    if (gid.x >= header.replay_span_count) { return; }
    let extension = slot_extensions[brain.extension_record_offset];
    let span = replay_spans[header.replay_span_offset + gid.x];
    if (span.local_synapse_id >= header.synapse_count) { return; }
    let metadata = synapse_metadata[
        extension.synapse_metadata_offset + span.local_synapse_id
    ];
    let receptor_plan = receptor[
        extension.receptor_offset + metadata.receptor_index
    ];
    var replay_credit = 0.0;
    for (var i = 0u; i < span.sample_count; i++) {
        let sample = unpack_replay_sample(
            replay_samples[header.replay_sample_offset + span.sample_start + i]
        );
        if (sample.event_index >= header.replay_event_count) { return; }
        let event = replay_events[header.replay_event_offset + sample.event_index];
        replay_credit += sample.eligibility
            * event.modulator_value
            * receptor_plan.modulator_sign;
    }
    let synapse = span.local_synapse_id;
    let inactive_fast_base = inactive_fast_weight_base(brain, extension);
    fast[inactive_fast_base + synapse] = clamp(
        fast[inactive_fast_base + synapse]
            + receptor_plan.sleep_replay_rate
                * alpha[brain.alpha_offset + synapse]
                * replay_credit,
        receptor_plan.fast_min,
        receptor_plan.fast_max,
    );
}
```

Use `global_invocation_id.y` to select the sleeping slot/generation header and
offset every lifetime/fast access into the shared class bucket. Consolidation
may batch compatible sleeping slots but validates and commits each slot
generation independently.

```wgsl
@compute @workgroup_size(64)
fn consolidate_fast_weights(@builtin(global_invocation_id) gid: vec3<u32>) {
    let header = sleep_headers[gid.y];
    let brain = brain_slots[header.brain_slot_index];
    if (brain.slot != header.slot || brain.slot_generation != header.slot_generation) { return; }
    let extension = slot_extensions[brain.extension_record_offset];
    let learning_state = load_slot_learning_state(extension.learning_state_offset);
    let request = sleep_requests[header.request_offset];
    let identity = phenotype_identities[header.brain_slot_index];
    let active_generation = learning_state_active_weight_generation(learning_state);
    if (request.schema_version != GPU_CONSOLIDATION_REQUEST_SCHEMA
        || request.request_flags != 0u
        || !full_hash_matches(request.phenotype_hash, identity.phenotype_hash)
        || request.cycle_id_lo != header.cycle_id_lo
        || request.cycle_id_hi != header.cycle_id_hi
        || request.input_generation_lo != active_generation.lo
        || request.input_generation_hi != active_generation.hi) { return; }
    let parameters = sleep_parameters[extension.sleep_parameter_offset];
    let local_synapse = gid.x;
    if (local_synapse >= brain.synapse_count) { return; }
    let active_lifetime_base = active_lifetime_weight_base(brain, extension);
    let inactive_lifetime_base = inactive_lifetime_weight_base(brain, extension);
    let inactive_fast_base = inactive_fast_weight_base(brain, extension);
    let active_lifetime_index = active_lifetime_base + local_synapse;
    let inactive_lifetime_index = inactive_lifetime_base + local_synapse;
    let inactive_fast_index = inactive_fast_base + local_synapse;
    let replayed_fast = fast[inactive_fast_index];
    lifetime[inactive_lifetime_index] = clamp(
        lifetime[active_lifetime_index] + parameters.staging_rate * replayed_fast,
        -parameters.weight_limit,
        parameters.weight_limit,
    );
    fast[inactive_fast_index] = replayed_fast
        * (1.0 - parameters.fast_decay_rate);
}
```

The host validates the full request cycle/generation/input/replay digests before
dispatch and stores their compact lanes at `request_offset`; the shader writes
only the row's staging span. Per-row canaries and generation-tagged completion
records prove batched sleeping slots cannot alias.

Validate finite/range diagnostics, output digest, input generation, and cycle
ID before emitting `GpuSleepStagingReceipt`. The output digest covers the
resolved inactive lifetime and fast spans and the expected new selector, but
polling does not flip it. After the app persists `Completed` and its staging
assets, `commit_sleep_consolidation` performs the host-side compare-and-swap of
`active_weight_bank` and returns the final commit receipt. Reuse recompaction's
all-or-nothing sleep-boundary compare-and-swap utility only, but remove
assumptions that only `h_shadow` changed. Slice B never changes projections,
synapse IDs/counts, lobe layout, compiler inputs, phenotype hash, or immutable
phenotype assets; structural recompilation remains a separate future
transaction. Any failure leaves the active lifetime/fast bindings and generation
unchanged and records no committed consolidation.

Implement the production consolidation driver in `gpu_live_runtime.rs`. It
asks the backend to build the bounded batch from that slot's committed replay
journal at the sleep boundary, then prepare/bind each scheduler intent and
submit the request through
`GpuClosedLoopBackend::submit_sleep_consolidation(handle, &request,
&bounded_replay_batch)`, reports the
backend job/completion identity back to the scheduler, and on device loss
discards uncommitted staging and recovers from the recorded input generation
and digest. After process loss it calls
`recover_submitted_sleep_consolidation`, persists the replacement job ID, and
only then polls. On staging completion it writes the staging assets and
`Completed` state durably, invokes `commit_sleep_consolidation`, then persists
`Committed` through the atomic save-manifest promotion defined in Task 8. A
generation compare-and-swap prevents a second runtime promotion.

Extend `GpuTestBrain` with a delegate that calls
`backend.prepare_sleep_consolidation(handle, intent, replay)` followed by
`backend.submit_sleep_consolidation(handle, request,
bounded_replay_batch)`, polls the returned job to a validated staging receipt,
then calls `commit_sleep_consolidation` to obtain the compact final receipt. No
backend sleep entry point accepts a bare cycle ID; it must compare
the full request generation/input/replay digests with the handle's slot state.

Add `gpu-tests = ["gpu-runtime", "alife_gpu_backend/gpu-tests"]` to
`alife_game_app` so its real-hardware sleep/restore tests can enable the
backend's explicit test seams without inventing an invalid Cargo feature.

- [ ] **Step 4: Implement retained behavior across the swap**

Restore the consolidated lifetime/decayed-fast generation into the waking
pipeline so the prewritten post-wake target-logit test passes.

- [ ] **Step 5: Run GPU sleep tests**

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_sleep -- --nocapture`

Expected: pass.

Run: `cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test automatic_gpu_sleep -j 1 -- --nocapture`

Expected: the real Vulkan cycle submits and commits once, wakes automatically,
and retains the learned target preference.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_gpu_backend/src/closed_loop_sleep.rs crates/alife_gpu_backend/src/closed_loop_runtime.rs crates/alife_gpu_backend/src/recompaction.rs crates/alife_gpu_backend/src/lib.rs crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl crates/alife_gpu_backend/shaders/closed_loop_consolidate.wgsl crates/alife_gpu_backend/shaders/closed_loop_replay_learning.wgsl crates/alife_gpu_backend/tests/closed_loop_sleep.rs crates/alife_gpu_backend/tests/support/mod.rs crates/alife_game_app/Cargo.toml crates/alife_game_app/src/live_brain_bridge.rs crates/alife_game_app/src/gpu_live_runtime.rs crates/alife_game_app/tests/automatic_gpu_sleep.rs
git commit -m "Consolidate GPU fast learning during sleep"
```

### Task 8: Persist learning and every sleep phase

**Files:**
- Modify: `crates/alife_gpu_backend/src/closed_loop_runtime.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_learning.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_sleep.rs`
- Create: `crates/alife_gpu_backend/tests/closed_loop_checkpoint.rs`
- Modify: `crates/alife_world/src/persistence.rs:726-1088`
- Create: `crates/alife_world/tests/gpu_brain_persistence.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Modify: `crates/alife_game_app/src/save_load_ux.rs`
- Modify: `crates/alife_game_app/src/production_voxel_frontend.rs`
- Create: `crates/alife_game_app/tests/gpu_sleep_restore.rs`
- Create visual blueprint before GUI edits: `docs/superpowers/assets/gpu-sleep-restore-blueprint.png`

**Interfaces:**
- Consumes: organism-bound `GpuBrainHandle`, phenotype hash, GPU checkpoint asset refs/digests, last learning sequence, sleep phase/cycle IDs.
- Produces: backend-owned `GpuBrainCheckpointSnapshot`/`GpuBrainRestoreRequest`/`GpuBrainRestoreReceipt`, versioned `GpuBrainSaveState`, and deterministic restore request.

- [ ] **Step 1: Use the image-generation skill for the required visual blueprint**

Inspect the current production save/load developer surface, then generate a
high-detail blueprint for a compact GPU brain checkpoint row showing phenotype
hash prefix, mutable-state checkpoint tick, sleep phase, consolidation state,
and a clear `GPU required` recovery status. It must not add CPU-shadow,
fallback, or parity language, and it must remain developer-only.

- [ ] **Step 2: Write failing phase-roundtrip and real restore tests**

```rust
#[test]
fn every_sleep_phase_roundtrips_without_duplicate_consolidation() {
    for phase in [
        SleepPhase::Awake,
        SleepPhase::EnteringSleep,
        SleepPhase::Consolidating,
        SleepPhase::Waking,
        SleepPhase::ForcedRecoverySleep,
    ] {
        let save = gpu_brain_save_fixture(phase);
        let json = serde_json::to_string(&save).unwrap();
        let loaded: GpuBrainSaveState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.sleep.phase, phase);
        assert_eq!(loaded.sleep.last_consolidated_cycle_id, save.sleep.last_consolidated_cycle_id);
    }
}
```

Add table-driven contract cases for every `ConsolidationState` variant and
reject impossible phase/progress combinations.
Add two manifest-crash fixtures around the final sleep commit. Before the
manifest CAS, the durable state is `Completed` with old main weight refs plus
validated staging refs and restore commits/promotes once. After the manifest
CAS, the durable state is `Committed`, the main refs/selectors/generation name
the staged output, staging refs are cleared, and restore performs zero GPU
promotion while retaining the new behavior.

In `gpu_sleep_restore.rs`, define the failing hardware matrix up front. For
N512, save and restore a learned real runtime at Awake, EnteringSleep, every
`ConsolidationState` variant, Waking, and ForcedRecoverySleep. For N1024 and
N2048, at minimum run the crash-sensitive Submitted-with-lost-job and Completed
cases used by Slice B acceptance. Resume each case to Awake and assert the table's exact
remaining work: Awake, Committed, and Waking perform zero new promotions/swaps;
EnteringSleep and Pending/Prepared/Submitted/Completed perform exactly one if
their cycle has not committed; ForcedRecoverySleep follows its nested
consolidation state. Always require unchanged genetic digest, no action while
non-awake, and retained learned behavior. The Submitted case
simulates process restart with no live job object and must recover from its
recorded input generation without double application.

- [ ] **Step 3: Verify the save contract is absent**

Run: `cargo test -p alife_world --test gpu_brain_persistence`

Expected: unresolved `GpuBrainSaveState`.

Run: `cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test gpu_sleep_restore -j 1 -- --nocapture`

Expected: compile failure for the missing GPU checkpoint/restore API.

- [ ] **Step 4: Implement portable records**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuBrainAssetRef {
    pub asset_id: String,
    pub digest: PortableAssetDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableActivationBanksV1 {
    pub schema_version: u16,
    pub phenotype_hash: PhenotypeHash,
    pub neuron_count: u32,
    pub active_side: u8,
    pub logical_dispatch_generation: u64,
    pub activation_a_bits: Vec<u32>,
    pub activation_b_bits: Vec<u32>,
    pub canonical_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableNeuronHomeostasisV1 {
    pub schema_version: u16,
    pub phenotype_hash: PhenotypeHash,
    pub neuron_count: u32,
    pub lanes_per_neuron: u16,
    pub value_bits: Vec<u32>,
    pub canonical_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableDualWeightBankV1 {
    pub schema_version: u16,
    pub layer_raw: u16,
    pub phenotype_hash: PhenotypeHash,
    pub synapse_count: u32,
    pub active_generation: u64,
    pub active_bank: u8,
    pub bank_0_bits: Vec<u32>,
    pub bank_1_bits: Vec<u32>,
    pub canonical_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableEligibilityBanksV1 {
    pub schema_version: u16,
    pub phenotype_hash: PhenotypeHash,
    pub recurrent_count: u32,
    pub decoder_count: u32,
    pub active_generation: u64,
    pub inactive_generation: u64,
    pub active_bank: u8,
    pub recurrent_bank_0_bits: Vec<u32>,
    pub recurrent_bank_1_bits: Vec<u32>,
    pub decoder_bank_0_bits: Vec<u32>,
    pub decoder_bank_1_bits: Vec<u32>,
    pub canonical_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableReplayJournalV1 {
    pub schema_version: u16,
    pub phenotype_hash: PhenotypeHash,
    pub replay_capture_plan_digest: [u64; 4],
    pub generation: u64,
    pub cursor: u32,
    pub event_count: u32,
    pub events: Vec<SleepReplayEvent>,
    pub synapse_spans: Vec<ReplaySynapseSpan>,
    pub eligibility_samples: Vec<ReplayEligibilitySample>,
    pub canonical_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingEligibilityCheckpoint {
    pub dispatch_generation: u64,
    pub originating_tick: Tick,
    pub frame_digest: PerceptionFrameDigest,
    pub active_activation_side: u8,
    pub candidate_index: u16,
    pub action_id: ActionId,
    pub action_family: CandidateActionFamily,
    pub candidate_feature_digest: CandidateFeatureDigest,
    pub active_eligibility_generation: u64,
    pub staging_eligibility_generation: u64,
}

impl PendingEligibilityCheckpoint {
    pub fn try_new(
        dispatch_generation: u64,
        originating_tick: Tick,
        frame_digest: PerceptionFrameDigest,
        active_activation_side: u8,
        candidate_index: u16,
        action_id: ActionId,
        action_family: CandidateActionFamily,
        candidate_feature_digest: CandidateFeatureDigest,
        active_eligibility_generation: u64,
        staging_eligibility_generation: u64,
    ) -> Result<Self, ScaffoldContractError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuSleepAssetState {
    pub replay_batch: Option<GpuBrainAssetRef>,
    pub lifetime_staging: Option<GpuBrainAssetRef>,
    pub fast_staging: Option<GpuBrainAssetRef>,
    pub eligibility_staging: Option<GpuBrainAssetRef>,
    pub replay_journal_staging: Option<GpuBrainAssetRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuBrainSaveState {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub phenotype_hash: PhenotypeHash,
    pub capacity_class_id: BrainClassId,
    pub immutable_phenotype: GpuBrainAssetRef,
    pub phenotype_compiler_inputs: GpuBrainAssetRef,
    pub active_weight_generation: u64,
    pub active_weight_bank: u8,
    pub active_eligibility_bank: u8,
    pub learning_transaction_generation: u64,
    pub lifetime_weights: GpuBrainAssetRef,
    pub fast_weights: GpuBrainAssetRef,
    pub eligibility: GpuBrainAssetRef,
    pub replay_journal: GpuBrainAssetRef,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub activation_state: GpuBrainAssetRef,
    pub neuron_homeostasis: GpuBrainAssetRef,
    pub checkpoint_tick: Tick,
    pub last_learning_replay_key: Option<OutcomeCreditReplayKey>,
    pub pending_eligibility: Option<PendingEligibilityCheckpoint>,
    pub pending_experience_transaction: Option<GpuBrainAssetRef>,
    pub sleep: SleepState,
    pub sleep_assets: GpuSleepAssetState,
}
```

In `alife_gpu_backend`, add the exact dependency-safe checkpoint boundary. The
backend types do not derive serde and never import `alife_world`:

```rust
pub struct GpuBrainCheckpointParts {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub phenotype_hash: PhenotypeHash,
    pub checkpoint_tick: Tick,
    pub active_activation_side: u8,
    pub logical_dispatch_generation: u64,
    pub activation_a_bits: Vec<u32>,
    pub activation_b_bits: Vec<u32>,
    pub neuron_homeostasis_bits: Vec<u32>,
    pub active_weight_generation: u64,
    pub active_weight_bank: u8,
    pub lifetime_bank_0_bits: Vec<u32>,
    pub lifetime_bank_1_bits: Vec<u32>,
    pub fast_bank_0_bits: Vec<u32>,
    pub fast_bank_1_bits: Vec<u32>,
    pub active_eligibility_generation: u64,
    pub inactive_eligibility_generation: u64,
    pub active_eligibility_bank: u8,
    pub learning_transaction_generation: u64,
    pub recurrent_eligibility_bank_0_bits: Vec<u32>,
    pub recurrent_eligibility_bank_1_bits: Vec<u32>,
    pub decoder_eligibility_bank_0_bits: Vec<u32>,
    pub decoder_eligibility_bank_1_bits: Vec<u32>,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub replay_events: Vec<GpuReplayEventRecord>,
    pub replay_spans: Vec<GpuReplaySynapseSpanRecord>,
    pub replay_samples: Vec<u32>,
    pub last_learning_replay_key: Option<OutcomeCreditReplayKey>,
    pub pending_eligibility: Option<PendingEligibilityRestoreParts>,
}

pub struct GpuBrainCheckpointSnapshot {
    schema_version: u16,
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
    checkpoint_tick: Tick,
    active_activation_side: u8,
    logical_dispatch_generation: u64,
    activation_a_bits: Vec<u32>,
    activation_b_bits: Vec<u32>,
    neuron_homeostasis_bits: Vec<u32>,
    active_weight_generation: u64,
    active_weight_bank: u8,
    lifetime_bank_0_bits: Vec<u32>,
    lifetime_bank_1_bits: Vec<u32>,
    fast_bank_0_bits: Vec<u32>,
    fast_bank_1_bits: Vec<u32>,
    active_eligibility_generation: u64,
    inactive_eligibility_generation: u64,
    active_eligibility_bank: u8,
    learning_transaction_generation: u64,
    recurrent_eligibility_bank_0_bits: Vec<u32>,
    recurrent_eligibility_bank_1_bits: Vec<u32>,
    decoder_eligibility_bank_0_bits: Vec<u32>,
    decoder_eligibility_bank_1_bits: Vec<u32>,
    replay_journal_generation: u64,
    replay_journal_cursor: u32,
    replay_journal_event_count: u32,
    replay_events: Vec<GpuReplayEventRecord>,
    replay_spans: Vec<GpuReplaySynapseSpanRecord>,
    replay_samples: Vec<u32>,
    last_learning_replay_key: Option<OutcomeCreditReplayKey>,
    pending_eligibility: Option<PendingEligibilityRestoreParts>,
    canonical_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingEligibilityRestoreParts {
    dispatch_generation: u64,
    originating_tick: Tick,
    frame_digest: PerceptionFrameDigest,
    active_activation_side: u8,
    candidate_index: u16,
    action_id: ActionId,
    action_family: CandidateActionFamily,
    candidate_feature_digest: CandidateFeatureDigest,
    active_eligibility_generation: u64,
    staging_eligibility_generation: u64,
}

impl PendingEligibilityRestoreParts {
    pub fn try_new(
        dispatch_generation: u64,
        originating_tick: Tick,
        frame_digest: PerceptionFrameDigest,
        active_activation_side: u8,
        candidate_index: u16,
        action_id: ActionId,
        action_family: CandidateActionFamily,
        candidate_feature_digest: CandidateFeatureDigest,
        active_eligibility_generation: u64,
        staging_eligibility_generation: u64,
    ) -> Result<Self, ScaffoldContractError>;
    // Read-only accessors exist for every lane above.
}

pub struct GpuBrainRestoreRequest {
    snapshot: GpuBrainCheckpointSnapshot,
}

pub struct GpuBrainRestoreReceipt {
    pub handle: GpuBrainHandle,
    pub pending_eligibility: Option<PendingEligibilityReceipt>,
    pub active_weight_generation: u64,
    pub active_weight_bank: u8,
    pub active_eligibility_generation: u64,
    pub active_eligibility_bank: u8,
    pub learning_transaction_generation: u64,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub checkpoint_digest: [u64; 4],
}

pub struct GpuCompletedSleepStagingParts {
    output_weight_bank: u8,
    lifetime_bank_0_bits: Vec<u32>,
    lifetime_bank_1_bits: Vec<u32>,
    fast_bank_0_bits: Vec<u32>,
    fast_bank_1_bits: Vec<u32>,
    eligibility_reset_generation: u64,
    output_eligibility_bank: u8,
    recurrent_eligibility_bank_0_bits: Vec<u32>,
    recurrent_eligibility_bank_1_bits: Vec<u32>,
    decoder_eligibility_bank_0_bits: Vec<u32>,
    decoder_eligibility_bank_1_bits: Vec<u32>,
    replay_journal_generation: u64,
    replay_journal_cursor: u32,
    replay_journal_event_count: u32,
    replay_events: Vec<GpuReplayEventRecord>,
    replay_spans: Vec<GpuReplaySynapseSpanRecord>,
    replay_samples: Vec<u32>,
    canonical_digest: [u64; 4],
}

pub struct GpuCompletedSleepStagingInputParts {
    pub output_weight_bank: u8,
    pub lifetime_bank_0_bits: Vec<u32>,
    pub lifetime_bank_1_bits: Vec<u32>,
    pub fast_bank_0_bits: Vec<u32>,
    pub fast_bank_1_bits: Vec<u32>,
    pub eligibility_reset_generation: u64,
    pub output_eligibility_bank: u8,
    pub recurrent_eligibility_bank_0_bits: Vec<u32>,
    pub recurrent_eligibility_bank_1_bits: Vec<u32>,
    pub decoder_eligibility_bank_0_bits: Vec<u32>,
    pub decoder_eligibility_bank_1_bits: Vec<u32>,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub replay_events: Vec<GpuReplayEventRecord>,
    pub replay_spans: Vec<GpuReplaySynapseSpanRecord>,
    pub replay_samples: Vec<u32>,
}

impl GpuCompletedSleepStagingParts {
    pub fn try_from_parts(parts: GpuCompletedSleepStagingInputParts)
        -> Result<Self, ScaffoldContractError>;
    pub const fn canonical_digest(&self) -> [u64; 4];
}

impl GpuBrainCheckpointSnapshot {
    pub fn try_from_parts(parts: GpuBrainCheckpointParts) -> Result<Self, ScaffoldContractError>;
    pub const fn canonical_digest(&self) -> [u64; 4];
    pub fn into_parts(self) -> GpuBrainCheckpointParts;
}

impl GpuBrainRestoreRequest {
    pub fn try_new(snapshot: GpuBrainCheckpointSnapshot) -> Result<Self, ScaffoldContractError>;
}

impl GpuClosedLoopBackend {
    pub fn snapshot_brain(
        &mut self,
        handle: GpuBrainHandle,
        checkpoint_tick: Tick,
    ) -> Result<GpuBrainCheckpointSnapshot, ScaffoldContractError>;

    pub fn restore_brain(
        &mut self,
        organism_id: OrganismId,
        phenotype: BrainPhenotype,
        request: GpuBrainRestoreRequest,
    ) -> Result<GpuBrainRestoreReceipt, ScaffoldContractError>;

    pub fn restore_completed_sleep_staging(
        &mut self,
        handle: GpuBrainHandle,
        request: &GpuConsolidationRequest,
        staged: &ConsolidationStagedOutput,
        parts: GpuCompletedSleepStagingParts,
    ) -> Result<GpuSleepStagingReceipt, ScaffoldContractError>;
}
```

`GpuBrainCheckpointParts` is the sole checked app-to-backend constructor
boundary; it has no serde
implementation and `try_from_parts` validates all counts, floats, selectors,
generations, replay spans, ownership, phenotype identity, and the complete
canonical digest domain. `snapshot_brain` waits only at an explicit
save/checkpoint boundary, reads the complete occupied slot spans, and includes
any pending eligibility identity from backend-owned state. Active ticks never
call it.

`restore_brain` first performs Slice A's organism-bound insertion, but exposes
no partially initialized handle. It uploads every bank and journal span into
the newly allocated offsets, installs selectors/generations/activation side,
revalidates the ReplayCapturePlan identity layout, and initializes
`LearningSequenceGuard::restore_validated` from the slot ownership plus the
saved full replay key. The app converts the world
`PendingEligibilityCheckpoint` to `PendingEligibilityRestoreParts::try_new`;
that type deliberately omits the old process-local handle generation,
organism, and phenotype. If pending parts exist, the backend binds the already
validated outer organism/phenotype and newly allocated handle generation,
validates the tuple
against the restored frame/selector/generation state, writes the exact new
`GpuPendingEligibilityRecord`, and mints the only new
private `PendingEligibilityReceipt` bound to the new slot generation. On any
failure it scrubs and removes the provisional slot atomically. The app cannot
write GPU offsets, forge a pending receipt, or restore bank zero by omission.
Roundtrip tests compare complete pre-save/post-restore state digests, then apply
or discard a restored pending receipt and prove the next GPU transition is
accepted exactly once.

`GpuCompletedSleepStagingInputParts` is filled by the app only after decoding all
four Completed staging envelopes. `restore_completed_sleep_staging` requires
the restored slot still names the request's old input generation/digest,
recomputes every full-bank/reset/journal digest and the staged-output digest,
uploads only the validated inactive/output and zero/reset spans, and registers
the saved nonzero job ID as provenance rather than a live capability. It
returns a new-process staging receipt suitable for the existing explicit
`commit_sleep_consolidation` compare-and-swap. Any mismatch scrubs the restored
staging spans and leaves active state untouched. A Completed crash test must
restore this receipt, commit exactly once, then prove a retry performs no
second selector flip; it may not pretend the old active snapshot already
contains the staged output.

Store bulk arrays outside JSON behind digest-checked asset refs. The six
mutable refs decode only as the exact portable envelopes above: `activation_state` is
`PortableActivationBanksV1`; `lifetime_weights` and `fast_weights` are
`PortableDualWeightBankV1` with layer codes 1 and 2; `eligibility` is
`PortableEligibilityBanksV1`; and `replay_journal` is
`PortableReplayJournalV1`; `neuron_homeostasis` is
`PortableNeuronHomeostasisV1`. The activation envelope records the active
ping/pong side and logical dispatch generation, and the eligibility envelope
preserves pending target-specific credit. Save boundaries may perform explicit
GPU-to-host snapshots; active ticks may not. Restore validates all digests and
uploads every mutable array before the next dispatch, so an Awake checkpoint
does not silently reset recurrent or learning state.
`PortableNeuronHomeostasisV1` requires `lanes_per_neuron == 2` and exactly
`neuron_count * 2` finite canonical bits in Slice A's target-major
`activity_ema, metabolic_load` order. Snapshot and restore bind it to
`GpuBrainSlotRecord.neuron_homeostasis_offset`; no accumulator, body-drive
snapshot, or CPU-recomputed substitute satisfies this asset.
The replay envelope requires `event_count <= event_capacity`, cursor in range,
the exact physical ring storage/count semantics defined above, one mutable span row
per immutable capture ID, and the full capture-plan digest. Its generation,
cursor, event count, span sample counts, and packed samples all participate in
the canonical asset digest and the `GpuSlotLearningStateRecord` roundtrip.
`immutable_phenotype` resolves to Slice A's custom-deserialized
`BrainPhenotype`, and `phenotype_compiler_inputs` resolves to the complete
immutable `PhenotypeCompilerInputs`. Restore resolves the canonical capacity,
validates both asset digests and both custom deserializers, recompiles the
phenotype, and requires equal canonical serialized bytes, compiler-input
digest, and phenotype hash before slot insertion. Missing inputs, a hash-only
phenotype, or a mutable checkpoint claiming different immutable structure is
rejected. Lifetime, fast, and eligibility assets contain both banks from each
shared pool; their phenotype/count/generation fields and 0/1 selectors must
match each other and the top-level save fields. Runtime extension records and
allocation-local offsets are never serialized. Restore first allocates fresh
class-bucket spans from the validated immutable phenotype, rebuilds the slot
and extension offsets, then uploads the portable banks and applies the
validated selectors.
Every envelope validator requires the exact phenotype-derived vector length,
known selector/layer codes, finite `f32::from_bits` values with canonical
negative-zero normalization, matching phenotype/count/generation fields, and a
fresh canonical-digest recomputation before any upload.
When consolidation reaches Completed, `lifetime_staging` and `fast_staging`
each reference a full output `PortableDualWeightBankV1`, not a single inactive
span. The envelope contains the unchanged old bank, the newly staged bank, the
expected output selector/generation, layer code, phenotype, and both-bank
canonical digest. Tests reject a staging ref whose schema decodes as a
single-bank payload. This makes the final manifest CAS type-safe: the exact
validated staging ref can become the corresponding main dual-bank ref without
changing its payload type.
The same Completed manifest requires `eligibility_staging` as a full
`PortableEligibilityBanksV1` with both banks zero and the expected reset
generation/selector, plus `replay_journal_staging` as a full empty
`PortableReplayJournalV1` with the advanced generation, reset cursor, zero
events/samples, and the complete ReplayCapturePlan-derived identity spans with
zero sample counts. No
single-bank or summary-only reset asset is accepted.
The replay-journal asset includes the bounded event/sample rows and compiled
capture-plan identity; its explicit generation/cursor must match the asset
digest and are restored before another learning commit or sleep batch build.
Add a hardware test that wakes after consolidation, commits one new outcome,
asserts it appends through the preserved identity spans, then verifies the next
sleep batch contains only that new event and no consumed pre-sleep row.
The active activation side is part of `activation_state`. The eligibility asset
contains both bank spans plus both generations. Sleep replay/request state is
not digest-only: Pending through Completed persist the exact bounded replay
batch. Pending's stored digest/counts must match
`sleep_assets.replay_batch`, and deserialization rejects Pending when that ref
is absent. Prepared/Submitted/Completed persist the backend-authored request
inside `SleepState`; Completed also requires all four weight/eligibility/
replay staging assets. No redundant
`backend_required` flag is serialized—neural policy and save-state kind imply
the GPU requirement.
After `commit_sleep_consolidation` flips the runtime selector, durability uses
one atomic save-manifest compare-and-swap bound to the exact Completed input
manifest digest. The replacement manifest simultaneously moves the validated
lifetime/fast staging asset refs into the main `lifetime_weights` and
`fast_weights` fields, moves eligibility/replay staging refs into the main
`eligibility` and `replay_journal` fields, writes all output generations,
weight/eligibility selectors, replay cursor, and replay event count, sets
`ConsolidationState::Committed`, and clears only those staging refs. It
cannot publish Committed with old main refs. A crash before atomic rename leaves
the prior Completed manifest intact and restart commits/promotes once; a crash
after rename loads the output banks and skips promotion. Retrying the CAS with
the same input/output digests is idempotent, while any changed manifest is a
typed conflict.
If `pending_eligibility` is present, its organism/phenotype come from the outer
save, while tick, final frame digest, exact 0/1 activation side, family,
candidate/action/feature identity, dispatch, and eligibility generations come
from the checkpoint. Restore validates that tuple against the saved activation
and eligibility assets, then rebinds it to the newly allocated private handle
generation; it never substitutes the slot's current side. The full
`OutcomeCreditReplayKey`, not only its sequence integer, restores
`LearningSequenceGuard` through
`LearningSequenceGuard::restore_validated(organism_id, phenotype_hash,
last_learning_replay_key)`, so the guard is identity-bound even when no prior
sequence exists.
`alife_gpu_backend` never imports this world-persistence DTO. The app's
`pending_checkpoint_from_receipt` adapter reads Slice B's public
`PendingEligibilityIdentity` accessors and calls
`PendingEligibilityCheckpoint::try_new`; restore performs the reverse field
comparison after backend allocation. This keeps the dependency direction
core/world -> app -> GPU and prevents a backend-to-world cycle.
`pending_eligibility` and `pending_experience_transaction` must be both absent
or both present. The transaction asset is the digest-checked portable
pre-action/decision/world-execution builder state needed either to finish
sealing the exact outcome or to perform the protected discard; a checkpoint
may not restore target-specific pending eligibility with no causal transaction
that can resolve it. Normal user saves wait for the tick boundary and therefore
store neither.

- [ ] **Step 5: Implement interrupted-sleep restore and recovery**

Restore and recompile the immutable phenotype/compiler inputs first, allocate a
new private handle, then upload all mutable assets and selectors into the
recorded logical generation before dispatch. For Pending, validate replay and
ask the new backend to prepare once. For
Prepared, submit the saved request once. For Submitted with no recoverable live
job, call `recover_submitted_sleep_consolidation`; it rebuilds the inactive
banks from the saved active input assets plus replay batch, reuses the exact
request/cycle, returns a new process-local nonzero job ID, and requires the
replacement Submitted state to be persisted before polling. For Completed,
require and validate all four lifetime/fast/eligibility/replay staging
envelopes and their output digests, convert them through
`GpuCompletedSleepStagingParts::try_from_parts`, call
`restore_completed_sleep_staging`, and pass its restored staging receipt to
the explicit compare-and-swap/reset once. For
Committed, require the main refs/selectors/generations/cursor to name those
outputs, skip
promotion and continue toward Waking. Reject any generation/digest mismatch
before mutating active buffers.

- [ ] **Step 6: Match and verify the production visual surface**

Match the developer-only checkpoint row to
`docs/superpowers/assets/gpu-sleep-restore-blueprint.png`, then capture the real
production surface with:

```powershell
$developerShot = 'target/artifacts/fvr03/MinimumSettings30x30_runtime_screenshot_fvr05_gpu_panel.png'
$playerShot = 'target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot.png'
foreach ($shot in @($developerShot, $playerShot)) {
    if (Test-Path -LiteralPath $shot) { Remove-Item -LiteralPath $shot }
}
$captureStarted = [DateTime]::UtcNow
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -Profile MinimumSettings30x30 -BrainPolicy gpu-required -GraphicsBackend vulkan -RequireGpu -DeveloperOverlay -RecordPerformance -SmokeSeconds 12
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -Profile MinSpecComfort1080p -BrainPolicy gpu-required -GraphicsBackend vulkan -RequireGpu -RecordPerformance -SmokeSeconds 12
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
foreach ($shot in @($developerShot, $playerShot)) {
    if ((Get-Item -LiteralPath $shot).LastWriteTimeUtc -lt $captureStarted) { throw "stale screenshot: $shot" }
}
```

Inspect
`target/artifacts/fvr03/MinimumSettings30x30_runtime_screenshot_fvr05_gpu_panel.png`
at high detail and compare it to the blueprint. Require all checkpoint fields,
readable hierarchy, no CPU-shadow/fallback wording, and no clipping. Also
inspect `target/artifacts/fvr03/MinSpecComfort1080p_runtime_screenshot.png` and
require the developer-only checkpoint row to be absent from the clean player
view. Iterate and recapture until both conditions pass.

- [ ] **Step 7: Run persistence tests**

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_checkpoint -- --nocapture`

Run: `cargo test -p alife_world --test gpu_brain_persistence`

Run: `cargo test -p alife_game_app --features gpu-runtime save_load -j 1`

Expected: pass.

Run: `cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test gpu_sleep_restore -j 1 -- --nocapture`

Expected: every real GPU restore case reaches Awake with the phase-dependent
zero-or-one remaining promotion/swap defined above.

- [ ] **Step 8: Commit**

```powershell
git add crates/alife_gpu_backend/src/closed_loop_runtime.rs crates/alife_gpu_backend/src/closed_loop_learning.rs crates/alife_gpu_backend/src/closed_loop_sleep.rs crates/alife_gpu_backend/tests/closed_loop_checkpoint.rs crates/alife_world/src/persistence.rs crates/alife_world/tests/gpu_brain_persistence.rs crates/alife_game_app/src/gpu_live_runtime.rs crates/alife_game_app/src/save_load_ux.rs crates/alife_game_app/src/production_voxel_frontend.rs crates/alife_game_app/tests/gpu_sleep_restore.rs docs/superpowers/assets/gpu-sleep-restore-blueprint.png
git commit -m "Persist GPU learning and sleep cycles"
```

### Task 9: Prove Slice B learning and sleep on real hardware

**Files:**
- Modify: `crates/alife_game_app/src/gpu_evidence.rs`
- Create: `crates/alife_game_app/tests/gpu_learning_sleep_acceptance.rs`
- Modify: `crates/alife_game_app/src/bin/alife_game_app.rs`
- Runtime artifacts: `target/artifacts/gpu-learning-sleep-slice-b-<class>.json`

**Interfaces:**
- Consumes: completed Slice B.
- Produces: immediate reward/pain learning, modulator ablation, automatic sleep, exactly-once consolidation, wake retention receipts.

Slice B embeds Slice A's flattened `GpuSliceEvidenceHeader` and
`PhenotypeEvidenceManifest`, plus this exact restore evidence in the canonical
artifact body:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpuSleepRestoreEvidence {
    pub checkpoint_phase_raw: u16,
    pub consolidation_state_raw: u16,
    pub cycle_id: u64,
    pub input_generation: u64,
    pub output_generation: u64,
    pub expected_remaining_swaps: u32,
    pub actual_remaining_swaps: u32,
    pub duplicate_swaps: u32,
    pub actions_while_non_awake: u32,
    pub save_asset_digest: [u64; 4],
    pub genetic_digest_before: [u64; 4],
    pub genetic_digest_after: [u64; 4],
    pub retained_target_delta: f32,
    pub tolerance: f32,
    pub reached_awake: bool,
    pub passed: bool,
}
```

The class artifact always checkpoints a real `Submitted` non-awake cycle,
simulates process loss of the live job, restores through the exact saved
request/replay/active assets, and reaches Awake with exactly one remaining swap.
All fields, the embedded manifest, and the shared header participate in the
artifact digest; Slice B's validating loader rejects a passing status unless
restore evidence passes.
`save_asset_digest` is the canonical digest of the complete portable save
manifest plus every referenced asset ID, declared digest, canonical decoded
bytes, and semantic field path. It resolves, in fixed field-path order,
`immutable_phenotype`, `phenotype_compiler_inputs`, activation, neuron
homeostasis, lifetime, fast, eligibility, replay journal, pending experience
transaction, replay batch, and every lifetime/fast/eligibility/replay staging
ref present in `GpuSleepAssetState`. Inline sleep requests and state are already
covered by the manifest bytes. Validation rejects a missing asset, a declared
digest/content mismatch, a wrong decoded envelope, or any changed immutable as
strictly as a changed mutable bank.

Slice B's phenotype rehash exposes
`BrainPhenotype::plasticity_plan_digest()`, the canonical domain-separated
aggregate of the schema, ordered receptor-plan digests, and the complete
`SleepConsolidationPlan` digest. The evidence builder must replace Slice A's
explicit-None tag with that exact value and set
`replay_capture_plan_digest` to
`phenotype.replay_capture_plan().canonical_digest()`. It recomputes the
manifest digest afterward; no app-authored summary or all-zero placeholder is
accepted.

- [ ] **Step 1: Add a failing receipt test**

```rust
#[test]
fn gpu_learning_sleep_receipt_proves_causal_learning() {
    for class in production_capacity_fixtures() {
        let receipt = run_gpu_learning_sleep_acceptance(options(class)).unwrap();
        let phenotype = recompile_acceptance_phenotype(class, 4202).unwrap();
        assert_eq!(receipt.header.slice_raw, 2);
        assert_eq!((receipt.header.profile_id_raw, receipt.header.profile_schema), (0, 0));
        assert_eq!(receipt.header.status_raw, 1);
        assert_eq!(receipt.header.class_id_raw, class.id().raw());
        assert_eq!(receipt.header.capacity_digest, class.canonical_digest());
        assert_eq!(receipt.header.phenotype_manifest_digest, receipt.phenotype_manifest.manifest_digest);
        assert_eq!(receipt.header.artifact_digest, receipt.recompute_artifact_digest().unwrap());
        assert_eq!(
            receipt.phenotype_manifest.plasticity_plan_digest,
            phenotype.plasticity_plan_digest(),
        );
        assert_eq!(
            receipt.phenotype_manifest.replay_capture_plan_digest,
            phenotype.replay_capture_plan().canonical_digest(),
        );
        assert_eq!(receipt.capacity_class_id, class.id());
        assert!(receipt.reward_target_delta > 0.0);
        assert!(receipt.pain_avoidance_delta > 0.0);
        assert!(receipt.unrelated_target_delta.abs() < receipt.reward_target_delta.abs());
        assert!(receipt.reward_target_delta > receipt.modulator_ablation_delta + receipt.tolerance);
        assert_eq!(receipt.consolidation_dispatches, 1);
        assert_eq!(receipt.genetic_digest_before, receipt.genetic_digest_after);
        assert!(receipt.post_wake_retained_delta > 0.0);
        assert!(receipt.replay_event_count > 0);
        assert!(receipt.replay_sample_count > 0);
        assert!(receipt.replay_induced_fast_l1 > receipt.tolerance);
        assert!(receipt.replay_vs_zero_sample_post_wake_delta > receipt.tolerance);
        assert_eq!(receipt.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
        assert!(receipt.gpu_learning_dispatches > 0);
        assert!(receipt.restore.passed);
        assert_eq!(receipt.restore.checkpoint_phase_raw, 3);
        assert_eq!(receipt.restore.consolidation_state_raw, 3);
        assert_eq!(receipt.restore.expected_remaining_swaps, 1);
        assert_eq!(receipt.restore.actual_remaining_swaps, 1);
        assert_eq!(receipt.restore.duplicate_swaps, 0);
        assert_eq!(receipt.restore.actions_while_non_awake, 0);
        assert!(receipt.restore.reached_awake);
        assert!(receipt.restore.retained_target_delta > receipt.restore.tolerance);
    }
}
```

In the same test file, remove each save ref one at a time and mutate the bytes
behind each ref one at a time. Include the immutable phenotype and compiler
inputs in that table. Every row must make receipt loading fail before status is
accepted; the untouched fixture must recompute the exact
`save_asset_digest` above.

- [ ] **Step 2: Run and verify the acceptance API is missing**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_learning_sleep_acceptance -j 1 -- --nocapture`

Expected: compile failure for the missing runner/receipt.

- [ ] **Step 3: Implement the acceptance runner**

Add `gpu-learning-sleep-acceptance --class n512|n1024|n2048 --seed 4202
--output <path>`. Atomically write one class-qualified receipt. Emit
before/after reward logits, before/after pain avoidance logits,
unrelated-target delta, ablation-control delta, sleep transition sequence,
consolidation count, genetic digest before/after, post-wake retained delta,
captured replay event/sample counts, `replay_induced_fast_l1`, and a
same-checkpoint replay-versus-zero-sample post-wake target-logit delta,
schema version, exact lowercase capacity-class slug, clean Git commit, and
canonical source-tree digest.
For each requested class, also run the Submitted-process-loss restore described
above and embed `GpuSleepRestoreEvidence`, the canonical capacity, the full
phenotype evidence manifest, and Slice B's shared flattened header. Extend
`gpu-evidence-validate --slice b` to validate every body invariant and recompute
manifest/capacity/artifact digests. Compute status 1 only after immediate
learning, nonzero replay effect, sleep, restore, and digest validation all
pass. The zero-sample comparison restores the same pre-sleep checkpoint and
uses the same request/seed/hardware while replacing only the validated replay
sample set with an empty set; ordinary fast promotion is therefore shared and
cannot satisfy the replay-effect gate by itself.

- [ ] **Step 4: Run focused gates**

```powershell
cargo fmt --all -- --check
cargo test -p alife_core --all-targets
cargo test -p alife_world --all-targets
cargo test -p alife_gpu_backend --features gpu-tests --all-targets -- --nocapture
cargo test -p alife_game_app --features gpu-runtime --test gpu_learning_sleep_acceptance -j 1 -- --nocapture
$rawMatches = & rg -n "cpu_shadow|CpuShadow|AutoWithCpuFallback|CpuReference|neural_fallback|FullGpuRuntimeMode|parity.gat" crates/alife_core/src crates/alife_gpu_backend/src crates/alife_game_app/src crates/alife_world/src
$scanExit = $LASTEXITCODE
if ($scanExit -gt 1) { throw "authority scan failed with exit $scanExit" }
$matches = @($rawMatches | Where-Object { $_ -notmatch 'crates[\\/]alife_world[\\/]src[\\/]legacy_neural_policy_v1.rs:' })
if ($matches.Count -ne 0) { $matches; throw "Slice B reintroduced a superseded authority surface" }
```

Expected: all pass on the real adapter.

- [ ] **Step 5: Commit the tested acceptance source**

```powershell
git add crates/alife_game_app/src/gpu_evidence.rs crates/alife_game_app/src/bin/alife_game_app.rs crates/alife_game_app/tests/gpu_learning_sleep_acceptance.rs
git commit -m "Prove GPU learning and sleep retention"
```

- [ ] **Step 6: Require a clean committed evidence source**

```powershell
if (git status --short) { throw "Slice B evidence requires a clean worktree" }
$evidenceCommit = git rev-parse HEAD
$evidenceTree = git rev-parse 'HEAD^{tree}'
```

- [ ] **Step 7: Run all three real receipts from that commit**

```powershell
foreach ($class in @('n512', 'n1024', 'n2048')) {
    $output = "target/artifacts/gpu-learning-sleep-slice-b-$class.json"
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-learning-sleep-acceptance --class $class --seed 4202 --output $output
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
```

Expected for every class: Vulkan GPU, immediate target-specific learning, one
consolidation, automatic wake, retained behavior, and the exact clean
commit/tree.

- [ ] **Step 8: Validate artifact provenance and class separation**

```powershell
foreach ($class in @('n512', 'n1024', 'n2048')) {
    $path = "target/artifacts/gpu-learning-sleep-slice-b-$class.json"
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-evidence-validate --slice b --input $path
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $receipt = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    if ($receipt.git_commit -ne $evidenceCommit) { throw "stale commit: $path" }
    if ($receipt.source_tree_digest -ne $evidenceTree) { throw "stale tree: $path" }
    if ($receipt.capacity_class -ne $class) { throw "wrong class: $path" }
    if ($receipt.consolidation_dispatches -ne 1) { throw "wrong consolidation count: $path" }
    if (-not $receipt.restore.passed) { throw "restore evidence failed: $path" }
    if ($receipt.restore.actual_remaining_swaps -ne 1 -or $receipt.restore.duplicate_swaps -ne 0) { throw "restore swap mismatch: $path" }
}
if (git status --short) { throw "tracked worktree changed after evidence" }
```

# GPU Closed-Loop Causal Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace caller-scored proposals and CPU-shadow-guarded GPU diagnostics with an authoritative GPU sensorimotor loop for N512, N1024, and N2048 brains.

**Architecture:** `alife_world` emits one same-tick perception frame with unscored candidates, `alife_core` owns versioned phenotype/candidate contracts, and one shared `alife_gpu_backend::GpuClosedLoopBackend` owns the device, pipelines, and class-bucketed SoA pools. Lightweight `GpuBrainHandle` values address per-creature slots; batched GPU dispatch performs encoding, recurrent microsteps, candidate decoding, and winner selection. The app selects either GPU neural policy or an explicitly labelled heuristic baseline; GPU failure never invokes a CPU neural fallback.

**Tech Stack:** Rust 2021, wgpu 29.0.3, WGSL, Bevy 0.18.0, serde, bytemuck, naga WGSL parser, PowerShell validation on Windows.

## Global Constraints

- Production neural behavior runs exactly once on GPU; no live CPU shadow, CPU parity gate, or CPU neural fallback.
- `alife_core` and `alife_world` must not depend on wgpu, Bevy, renderer types, or OS handles.
- The world remains authoritative for candidate enumeration, action legality, execution, and outcome.
- Candidates contain observations and command transport fields, never caller-provided utility scores.
- Initial neural capacity classes are N512, N1024, and N2048; larger classes return a typed production-gate error.
- Active-loop readback is limited to one selected-action record and bounded diagnostic counters.
- Production shaders are WGSL only.
- Use `cargo test -j 1` for Bevy-heavy all-feature gates on this Windows host.
- Do not modify or merge the unrelated FVR11 worktree.

## Planned file structure

### Core contracts

- Create `crates/alife_core/src/perception.rs`: sensor profile, body snapshot, unscored candidate, perception frame, selected neural action.
- Create `crates/alife_core/src/phenotype.rs`: stable phenotype identity in Task 2, then compiled phenotype records and deterministic compiler in Task 3.
- Modify `crates/alife_core/src/brain_class.rs`: add `BrainCapacityClass`, production capacity gate, and a legacy `BrainClassSpec` adapter.
- Modify `crates/alife_core/src/error.rs`: typed phenotype/backend/candidate errors.
- Modify `crates/alife_core/src/version.rs`: perception and phenotype schema kinds.
- Modify `crates/alife_core/src/lib.rs`: focused module exports.

### GPU implementation

- Create `crates/alife_gpu_backend/src/closed_loop_buffers.rs`: GPU ABI records and single-owner SoA buffer plan.
- Create `crates/alife_gpu_backend/src/closed_loop_pipeline.rs`: wgpu pipeline creation and dispatch ordering.
- Create `crates/alife_gpu_backend/src/closed_loop_runtime.rs`: shared required-GPU backend, brain handles, class buckets, and compact tick results.
- Create `crates/alife_gpu_backend/shaders/closed_loop_encode.wgsl`: clamp current perception into input populations.
- Create `crates/alife_gpu_backend/shaders/closed_loop_recurrent.wgsl`: sparse leaky recurrent microstep.
- Create `crates/alife_gpu_backend/shaders/closed_loop_decode.wgsl`: candidate-conditioned logits and deterministic winner.
- Modify `crates/alife_gpu_backend/src/lib.rs`: expose only the new product runtime API.
- Retire `crates/alife_gpu_backend/src/full_runtime.rs` and the product use of `static_forward.rs`; keep only narrowly reused lower-level utilities.

### World and app integration

- Create `crates/alife_world/src/candidate_enumerator.rs`: profile-aware, unscored candidate enumeration.
- Modify `crates/alife_world/src/headless.rs`: produce one `PerceptionFrame` and execute selected commands.
- Create `crates/alife_game_app/src/brain_policy.rs`: explicit neural versus heuristic selection.
- Rewrite `crates/alife_game_app/src/live_brain_bridge.rs`: no default proposal scores; GPU tick orchestration.
- Replace CPU-shadow modes in `crates/alife_game_app/src/graphical_playground.rs` and `gpu_live_runtime.rs`.
- Migrate affected CLI, product telemetry, prerequisite diagnostics, and tests.

---

### Task 1: Supersede scaffold-only and CPU-shadow architecture rules

**Files:**
- Modify: `docs/architecture_decisions.md:154`
- Modify: `docs/master_spec.md:1-845,1029-1075`
- Modify: `docs/AGENTS.md:1-30`
- Modify: `AGENTS.md:1-35`
- Modify: `crates/alife_gpu_backend/AGENTS.md:1-20`
- Modify: `crates/alife_core/AGENTS.md:1-20`
- Test: `scripts/docs_check.ps1`

**Interfaces:**
- Consumes: approved design `docs/superpowers/specs/2026-07-09-gpu-closed-loop-brain-design.md`.
- Produces: ADR-024 and local instructions authorizing GPU neural kernels while prohibiting CPU shadow execution.

- [ ] **Step 1: Add the controlling ADR text**

Append this decision verbatim, adjusting only line wrapping:

```markdown
## ADR-024: Closed-Loop Neural Cognition Is GPU-Authoritative

Decision: The production neural policy gathers current perception and unscored
world candidates before dispatch, then performs encoding, recurrent dynamics,
candidate scoring, winner selection, waking plasticity, and sleep
consolidation through WGSL pipelines. Production does not run a live CPU neural
shadow, parity-gated duplicate brain, or automatic CPU neural fallback.

`HeuristicBaseline` remains explicit and separately labelled. GPU unavailability
returns a typed unavailable result. N512, N1024, and N2048 are the initial
production neural capacity classes; larger classes remain research-gated.

This decision supersedes the CPU consolidation authority in ADR-014, the P14
CPU-schema ownership clause in ADR-015, GPU parity gating in ADR-016, CPU
fallback in ADR-019 and ADR-021, and the CPU-shadow/parity authority clauses in
ADR-023. Their save-safety, sparse-layout, world-authority, and sealed-patch
boundaries remain in force where they do not conflict with ADR-024.
```

- [ ] **Step 2: Update the controlling master specification**

Revise the status/objective and sections 1-4, 7-8, 15, 20-21, 24-28, 32-35,
39-40, Appendix B, and the deep-dive text so the controlling document states:

- production cognition is the reviewed GPU-authoritative closed loop;
- CPU neural math is test/developer reference only;
- class-bucketed sparse GPU pools and unscored same-tick candidates are
  mandatory;
- effective weights use genetic + lifetime + alpha * fast;
- automatic GPU sleep consolidation replaces CPU H-shadow authority;
- N512/N1024/N2048 are the only promoted production classes;
- behavioral, hardware, save, soak, and performance tests replace CPU parity
  as the acceptance oracle.

Remove obsolete statements that this repository must not implement real neural
kernels or that production GPU execution must preserve CPU fallback. Keep
historical milestone text only when explicitly labelled superseded by ADR-024.

- [ ] **Step 3: Update repository instructions**

Replace the obsolete scaffold restriction with these exact rules:

```markdown
- Production neural execution is GPU-authoritative WGSL; do not add a live CPU
  shadow, parity gate, or automatic CPU neural fallback.
- Keep pure CPU neural helpers test-only or developer-only.
- World code enumerates unscored candidates and remains authoritative for
  legality and outcomes.
- Promote only N512, N1024, and N2048 until larger tiers pass the documented
  causal and performance gates.
```

- [ ] **Step 4: Run documentation validation**

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`

Expected: exit 0 and a printed Git Bash path.

- [ ] **Step 5: Commit**

```powershell
git add AGENTS.md crates/alife_core/AGENTS.md crates/alife_gpu_backend/AGENTS.md docs/AGENTS.md docs/architecture_decisions.md docs/master_spec.md
git commit -m "Document GPU-authoritative brain runtime"
```

### Task 2: Add unscored perception and action-candidate contracts

**Files:**
- Create: `crates/alife_core/src/perception.rs`
- Create: `crates/alife_core/src/phenotype.rs`
- Create: `crates/alife_core/tests/perception_candidates.rs`
- Create: `crates/alife_core/tests/experience_neural_decision.rs`
- Modify: `crates/alife_core/src/experience.rs:1-310`
- Modify: `crates/alife_core/src/memory.rs`
- Modify: `crates/alife_core/src/topology.rs`
- Modify: `crates/alife_core/src/packed_log.rs`
- Modify: `crates/alife_core/src/reference_brain.rs`
- Modify: `crates/alife_core/src/error.rs:8-70`
- Modify: `crates/alife_core/src/version.rs:7-75`
- Modify: `crates/alife_core/src/lib.rs:1-157`
- Modify: `crates/alife_core/tests/experience_three_phase.rs`
- Modify: `crates/alife_core/tests/memory_expectancy.rs`
- Modify: `crates/alife_core/tests/packed_experience_logging.rs`
- Modify: `crates/alife_core/tests/post_seal_lifetime_deltas.rs`
- Modify: `crates/alife_core/tests/sleep_consolidation.rs`
- Modify: `crates/alife_core/tests/topological_map.rs`

**Interfaces:**
- Consumes: `ActionId`, `ActionKind`, `ActionTarget`, `HomeostaticSnapshot`, `SensorySnapshot`, `Tick`, `Vec3f`, `Velocity`.
- Produces: `SensorProfile`, `CandidateActionFamily`, `BodySnapshot`,
  `CandidateFeatureVector`, `ActionCandidate`, immutable
  `PerceptionFrameDraft`/`PerceptionContextBlock`/`PerceptionFrame` digest flow,
  `NeuralActionSelection`, `PreActionBrainEvidence`, `DecisionEvidence`,
  `NeuralDecisionEvidence`, and `PolicyBackend`.

- [ ] **Step 1: Write failing candidate-contract tests**

Create tests that use the following public API:

```rust
use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, CandidateActionFamily,
    CandidateObservationRef,
    CandidateFeatureVector, Confidence, DurationTicks, HomeostaticSnapshot, NormalizedScalar,
    OrganismId, PerceptionContextBlock, PerceptionContextKind, PerceptionFrame,
    PolicyBackend, SensorProfile, SensoryChannels,
    SensorySnapshot, Tick, Pose, Vec3f, Velocity,
};

#[test]
fn candidate_is_unscored_and_frame_is_same_tick() {
    let tick = Tick::new(7);
    let sensory = SensorySnapshot::new(
        OrganismId(1), tick, Vec3f::ZERO, SensoryChannels::ZERO, Default::default(),
    ).unwrap();
    let candidate = ActionCandidate::new(
        0,
        ActionId(101),
        ActionKind::Move,
        CandidateActionFamily::Approach,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector::zero(),
        Confidence::new(1.0).unwrap(),
        NormalizedScalar::new(0.1).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(1),
    ).unwrap();
    let frame = PerceptionFrame::new(
        OrganismId(1),
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory,
        BodySnapshot { pose: Pose::IDENTITY, velocity: Velocity::ZERO },
        HomeostaticSnapshot::baseline(tick),
        vec![candidate],
    ).unwrap();
    assert_eq!(frame.tick(), frame.sensory().tick);
    assert_eq!(frame.candidates()[0].candidate_index, 0);
    assert_eq!(PolicyBackend::default(), PolicyBackend::NeuralClosedLoopGpu);
    let command = frame.candidates()[0]
        .to_command(OrganismId(1), Confidence::new(0.8).unwrap())
        .unwrap();
    assert_eq!(command.action_id, ActionId(101));
    assert_eq!(command.duration_ticks, DurationTicks::new(1));
}

#[test]
fn candidate_validation_rejects_duplicate_indices_and_non_finite_features() {
    let mut features = CandidateFeatureVector::zero();
    features.0[0] = f32::NAN;
    assert!(features.validate().is_err());
}

#[test]
fn candidate_family_raw_mapping_is_stable_and_total() {
    for raw in 0u8..=7 {
        let family = CandidateActionFamily::try_from_raw(raw).unwrap();
        assert_eq!(family.raw(), raw);
    }
    assert!(CandidateActionFamily::try_from_raw(8).is_err());
    assert_eq!(SensorProfile::PrivilegedAffordanceV1.raw(), 1);
    assert_eq!(SensorProfile::GroundedObjectSlotsV1.raw(), 2);
    assert!(SensorProfile::try_from_raw(0).is_err());
    assert!(SensorProfile::try_from_raw(3).is_err());
}

#[test]
fn sealed_patch_binds_gpu_selection_to_the_perception_frame() {
    let frame = perception_fixture();
    let selection = neural_selection_fixture(&frame, 1);
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        selection,
        command_for_candidate(&frame.candidates()[1]),
    ).unwrap();
    let patch = seal_with_decision(frame.clone(), decision).unwrap();
    let evidence = patch.decision().neural_evidence().unwrap();
    assert_eq!(evidence.base_digest, patch.pre_action().base_digest().unwrap());
    assert_eq!(evidence.frame_digest, patch.pre_action().frame_digest().unwrap());
    assert_eq!(evidence.action_id, frame.candidates()[1].action_id);
}

#[test]
fn mismatched_candidate_or_command_cannot_be_sealed() {
    let frame = perception_fixture();
    let selection = neural_selection_fixture(&frame, 1);
    let wrong = command_for_candidate(&frame.candidates()[0]);
    assert!(DecisionSnapshot::from_neural_selection(
        sequence_id(), phenotype_hash(), 7, 1, &frame, selection, wrong,
    ).is_err());
}

#[test]
fn base_digest_precedes_context_and_final_digest_without_a_cycle() {
    let draft = perception_draft_fixture();
    let base = draft.base_digest();
    let empty = draft.clone().finalize(PerceptionContextBlock::empty()).unwrap();
    let recalled = draft.finalize(
        PerceptionContextBlock::try_new(
            1,
            PerceptionContextKind::EpisodicCandidateV1,
            vec![0.25, -0.5],
        ).unwrap(),
    ).unwrap();
    assert_eq!(empty.base_digest(), base);
    assert_eq!(recalled.base_digest(), base);
    assert_ne!(empty.frame_digest(), recalled.frame_digest());
}

#[test]
fn tampered_serialized_base_context_or_final_digest_is_rejected() {
    for tampered in tampered_perception_digest_json_rows() {
        assert!(serde_json::from_value::<PerceptionFrame>(tampered).is_err());
    }
}

#[test]
fn action_candidate_source_has_no_score_field() {
    let source = include_str!("../src/perception.rs");
    let candidate = source.split("pub struct ActionCandidate").nth(1).unwrap();
    let candidate = candidate.split('}').next().unwrap();
    assert!(!candidate.contains("score:"));
    assert!(!candidate.contains("utility:"));
}
```

- [ ] **Step 2: Run the tests and verify the API is missing**

Run: `cargo test -p alife_core --test perception_candidates --test experience_neural_decision`

Expected: compile failure for unresolved imports from `alife_core`.

- [ ] **Step 3: Implement the contracts**

Use these exact constants and shapes:

```rust
pub const CANDIDATE_FEATURE_COUNT: usize = 24;
pub const MAX_ACTION_CANDIDATES: usize = 32;

#[repr(u16)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorProfile {
    #[default]
    PrivilegedAffordanceV1 = 1,
    GroundedObjectSlotsV1 = 2,
}

impl SensorProfile {
    pub const fn raw(self) -> u16 { self as u16 }
    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyBackend {
    #[default]
    NeuralClosedLoopGpu,
    HeuristicBaseline,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateActionFamily {
    Idle = 0,
    Rest = 1,
    Inspect = 2,
    Approach = 3,
    Avoid = 4,
    Contact = 5,
    Ingest = 6,
    Other = 7,
}

impl CandidateActionFamily {
    pub const fn raw(self) -> u8 { self as u8 }
    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateObservationRef {
    None,
    ObjectSlot(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BodySnapshot {
    pub pose: Pose,
    pub velocity: Velocity,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CandidateFeatureVector(pub [f32; CANDIDATE_FEATURE_COUNT]);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionCandidate {
    pub candidate_index: u16,
    pub action_id: ActionId,
    pub kind: ActionKind,
    pub family: CandidateActionFamily,
    pub observation: CandidateObservationRef,
    pub target: ActionTarget,
    pub features: CandidateFeatureVector,
    pub sensor_confidence: Confidence,
    pub required_effort: NormalizedScalar,
    pub min_duration: DurationTicks,
    pub max_duration: DurationTicks,
}

impl ActionCandidate {
    pub fn to_command(
        self,
        organism_id: OrganismId,
        neural_confidence: Confidence,
    ) -> Result<ActionCommand, ScaffoldContractError> {
        ActionCommand::structured(
            organism_id,
            self.action_id,
            self.kind,
            self.target,
            Intensity::new(1.0)?,
            self.min_duration,
            neural_confidence,
            0,
            None,
            None,
            None,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerceptionFrameDraft {
    schema_version: u16,
    organism_id: OrganismId,
    tick: Tick,
    sensor_profile: SensorProfile,
    sensory: SensorySnapshot,
    body: BodySnapshot,
    homeostasis: HomeostaticSnapshot,
    candidates: Vec<ActionCandidate>,
    base_digest: PerceptionBaseDigest,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerceptionContextBlock {
    schema_version: u16,
    context_kind: PerceptionContextKind,
    values: Vec<f32>,
    canonical_digest: PerceptionContextDigest,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PerceptionContextKind {
    None = 0,
    EpisodicCandidateV1 = 1,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerceptionFrame {
    base: PerceptionFrameDraft,
    context: PerceptionContextBlock,
    frame_digest: PerceptionFrameDigest,
}

impl PerceptionFrameDraft {
    pub fn new(
        organism_id: OrganismId,
        tick: Tick,
        sensor_profile: SensorProfile,
        sensory: SensorySnapshot,
        body: BodySnapshot,
        homeostasis: HomeostaticSnapshot,
        candidates: Vec<ActionCandidate>,
    ) -> Result<Self, ScaffoldContractError>;
    pub const fn base_digest(&self) -> PerceptionBaseDigest;
    pub fn finalize(
        self,
        context: PerceptionContextBlock,
    ) -> Result<PerceptionFrame, ScaffoldContractError>;
}

impl PerceptionContextBlock {
    pub fn empty() -> Self;
    pub fn try_new(
        schema_version: u16,
        context_kind: PerceptionContextKind,
        values: Vec<f32>,
    ) -> Result<Self, ScaffoldContractError>;
    pub const fn canonical_digest(&self) -> PerceptionContextDigest;
    pub fn values(&self) -> &[f32];
}

impl PerceptionFrame {
    pub fn new(
        organism_id: OrganismId,
        tick: Tick,
        sensor_profile: SensorProfile,
        sensory: SensorySnapshot,
        body: BodySnapshot,
        homeostasis: HomeostaticSnapshot,
        candidates: Vec<ActionCandidate>,
    ) -> Result<Self, ScaffoldContractError>;
    pub const fn organism_id(&self) -> OrganismId;
    pub const fn tick(&self) -> Tick;
    pub const fn sensor_profile(&self) -> SensorProfile;
    pub fn sensory(&self) -> &SensorySnapshot;
    pub const fn body(&self) -> BodySnapshot;
    pub fn homeostasis(&self) -> &HomeostaticSnapshot;
    pub fn candidates(&self) -> &[ActionCandidate];
    pub fn context(&self) -> &PerceptionContextBlock;
    pub const fn base_digest(&self) -> PerceptionBaseDigest;
    pub const fn frame_digest(&self) -> PerceptionFrameDigest;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NeuralActionSelection {
    pub candidate_index: u16,
    pub logit: f32,
    pub confidence: Confidence,
    pub active_tiles: u32,
    pub active_synapses: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhenotypeHash(pub [u64; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PerceptionBaseDigest(pub [u64; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PerceptionContextDigest(pub [u64; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PerceptionFrameDigest(pub [u64; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CandidateFeatureDigest(pub [u64; 2]);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreActionBrainEvidence {
    NeuralClosedLoopGpu {
        capacity_class_id: BrainClassId,
        phenotype_hash: PhenotypeHash,
        sensor_profile: SensorProfile,
        base_digest: PerceptionBaseDigest,
        frame_digest: PerceptionFrameDigest,
    },
    HeuristicBaseline {
        baseline_schema_version: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NeuralDecisionEvidence {
    pub phenotype_hash: PhenotypeHash,
    pub dispatch_generation: u64,
    pub base_digest: PerceptionBaseDigest,
    pub frame_digest: PerceptionFrameDigest,
    pub active_activation_side: u8,
    pub candidate_index: u16,
    pub action_id: ActionId,
    pub action_family: CandidateActionFamily,
    pub candidate_feature_digest: CandidateFeatureDigest,
    pub logit: f32,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DecisionEvidence {
    NeuralClosedLoopGpu(NeuralDecisionEvidence),
    HeuristicBaseline(HeuristicDecisionEvidence),
}
```

`HeuristicDecisionEvidence` owns the legacy proposal/ranking/arbitration fields;
they are no longer mandatory fields on every `DecisionSnapshot`.
Define a matching `HeuristicPreActionEvidence` that owns the legacy
lobe/routing/weight-split and scalar memory snapshot needed by the explicit
baseline during this intermediate commit. Provide typed
`pre_action.heuristic_evidence()` and `decision.heuristic_evidence()` accessors;
memory, topology, packed logging, post-seal lifetime, sleep, and the temporary
`reference_brain` baseline must branch on those accessors rather than reading
universal fields. Neural evidence returns a typed `EvidenceKindMismatch` from
baseline-only accessors. This adapter is temporary production support for
`HeuristicBaseline`, not a CPU neural shadow, and Slice A Task 10 removes any
baseline fields that are no longer used.
Replace the universal `PreActionSnapshot` lobe/routing/weight-split and scalar
`MemoryExpectancySnapshot` fields with `brain_evidence`, the validated
`PerceptionFrame`, genome/development identity, body, and homeostasis. Legacy
baseline-only details live inside the versioned heuristic evidence adapter;
the GPU patch never snapshots or clones CPU projection schemas or weight
layers.
World extraction first creates an immutable `PerceptionFrameDraft`; its
`PerceptionBaseDigest` is computed exactly once from the ordered
world/body/homeostasis/candidate bytes. Slice A finalizes that draft with
`PerceptionContextBlock::empty()`. Slice C may query memory with only the draft
and its base digest, then consume the draft exactly once by calling `finalize`
with the returned canonical context block. There is no API that attaches
context to an already-finalized frame or changes either digest in place.
Custom `Deserialize` implementations for the draft, context block, and final
frame recompute and validate their private digest fields; derived unchecked
deserialization is forbidden.

`DecisionSnapshot::from_neural_selection` receives the validated finalized
`PerceptionFrame`, its selected candidate, the compact GPU receipt, and the
constructed `ActionCommand`. It recomputes both base and final digests and
rejects any action ID, target, organism, tick, phenotype, activation-side, or
candidate mismatch before sealing. Pre-action and decision neural evidence
carry both digests; Binds for eligibility/credit use the final digest while
Slice C retrieval keys use only the immutable base digest.
Its scalar arguments are `(sequence_id, phenotype_hash,
dispatch_generation, active_activation_side, &frame, selection, command)`;
`active_activation_side` accepts only 0 or 1. Task 9 passes these values from
one validated `GpuClosedLoopTick`, never from independent telemetry fields.
`CandidateFeatureDigest` hashes action family, action kind, and observed feature
and duration fields, never the transported raw entity ID.
`PerceptionBaseDigest` hashes canonical little-endian base fields in its own
domain and excludes every digest field and all episodic context.
`PerceptionContextDigest` hashes the context schema/kind/value bytes and
excludes its own digest field. `PerceptionFrameDigest` hashes the base digest,
the canonical context bytes, and a distinct final-frame domain tag; it excludes
its own digest field. This base -> context -> final order is the only digest
flow, so memory lookup cannot depend circularly on the final digest it helps
create. Slice A's empty context still yields a domain-distinct final digest.

Validation must require finite feature values in `[-1.0, 1.0]`, contiguous
unique candidate indices starting at zero, no more than 32 candidates,
matching organism/tick IDs, at least one candidate, positive ordered duration
bounds, and exact digest/evidence consistency. Family/kind compatibility is
explicit: Idle→Idle, Rest→Rest, Inspect→Inspect, Approach/Avoid→Move,
Contact/Ingest→Interact; `Other` accepts the remaining structured kinds.
Slice A permits `None`; Slice C validates every `ObjectSlot(index)` against its
same-tick grounded slot table. The reference is included in frame/decision
digests but not the decoder feature vector; raw `WorldEntityId` remains only in
`ActionTarget` transport.

Add `SchemaKind::Perception` and `SchemaKind::Phenotype`, and add both fields to
`SchemaVersions`. `ContractVersion::V1` already exists and is reused rather
than redefined. Bump the Experience schema for the evidence enum and provide a
tested legacy baseline migration. Add
`ScaffoldContractError::InvalidPerceptionFrame`,
`InvalidActionCandidate`, `InvalidDecisionEvidence`, `PhenotypeCompile`,
`UnsupportedProductionBrainClass`, `GpuLayoutMismatch`,
`SensorProfileMismatch`, `BrainOwnershipMismatch`, and
`NeuralBackendUnavailable`.

- [ ] **Step 4: Run focused and core tests**

Run: `cargo test -p alife_core --test perception_candidates --test experience_neural_decision`

Expected: all new tests pass.

Run: `cargo test -p alife_core --all-targets`

Expected: all existing and new core tests pass.

- [ ] **Step 5: Commit**

```powershell
git add crates/alife_core/src/perception.rs crates/alife_core/src/phenotype.rs crates/alife_core/src/experience.rs crates/alife_core/src/memory.rs crates/alife_core/src/topology.rs crates/alife_core/src/packed_log.rs crates/alife_core/src/reference_brain.rs crates/alife_core/src/error.rs crates/alife_core/src/version.rs crates/alife_core/src/lib.rs crates/alife_core/tests/perception_candidates.rs crates/alife_core/tests/experience_neural_decision.rs crates/alife_core/tests/experience_three_phase.rs crates/alife_core/tests/memory_expectancy.rs crates/alife_core/tests/packed_experience_logging.rs crates/alife_core/tests/post_seal_lifetime_deltas.rs crates/alife_core/tests/sleep_consolidation.rs crates/alife_core/tests/topological_map.rs
git commit -m "Add unscored brain perception contracts"
```

### Task 3: Compile genomes into nonempty production phenotypes

**Files:**
- Modify: `crates/alife_core/src/phenotype.rs`
- Create: `crates/alife_core/tests/phenotype_compiler.rs`
- Modify: `crates/alife_core/src/brain_class.rs:10-350`
- Modify: `crates/alife_core/src/genome.rs:11-269`
- Modify: `crates/alife_core/src/lobe.rs:308-560`
- Modify: `crates/alife_core/src/neural.rs`
- Modify: `crates/alife_core/src/routing.rs`
- Modify: `crates/alife_core/src/lib.rs`

**Interfaces:**
- Consumes: `BrainGenome`, `BrainCapacityClass`, `DevelopmentState`, `SensorProfile`, routing and lobe contracts.
- Produces: final `BrainCapacityClass`/`BrainExecutionBudget` authority,
  `PhenotypeCompiler::compile`, `BrainPhenotype`, `CompiledProjection`,
  `CompiledSynapse`, `CompiledBudgets`, route/global budget receipts,
  `NeuronDynamics`, `SensorEncoderPlan`, `CandidateDecoderPlan`, exact synapse
  coordinates, stable GPU enum mappings, and `PhenotypeHash`.

- [ ] **Step 1: Write failing phenotype tests**

```rust
use alife_core::{
    BrainCapacityClass, BrainClassId, BrainGenome, BrainScaleTier, DevelopmentState,
    LegacyBrainClassAdapter, NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
    CANDIDATE_FEATURE_COUNT,
};

fn compile(class_id: BrainClassId, seed: u64) -> alife_core::BrainPhenotype {
    let capacity = BrainCapacityClass::production_for_id(class_id).unwrap();
    let genome = BrainGenome::scaffold(seed, capacity.id());
    let development = DevelopmentState::new(
        genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap(),
    );
    PhenotypeCompiler::compile(
        &genome, &capacity, &development, SensorProfile::PrivilegedAffordanceV1,
    ).unwrap()
}

#[test]
fn production_classes_compile_nonempty_with_stable_hashes() {
    for class_id in [
        BrainCapacityClass::N512_ID,
        BrainCapacityClass::N1024_ID,
        BrainCapacityClass::N2048_ID,
    ] {
        let one = compile(class_id, 41);
        let two = compile(class_id, 41);
        assert!(!one.projections().is_empty());
        assert!(one.synapses().len() >= 128);
        assert_eq!(one.phenotype_hash(), two.phenotype_hash());
        assert_eq!(one.budgets().global.total_synapses as usize, one.synapses().len());
        assert!(one.budgets().global.total_synapses
            <= BrainCapacityClass::production_for_id(class_id).unwrap().execution().max_total_synapses());
    }
}

#[test]
fn connectome_and_density_mutations_change_phenotype() {
    let capacity = BrainCapacityClass::n512();
    let mut genome = BrainGenome::scaffold(9, capacity.id());
    let development = DevelopmentState::new(
        genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap(),
    );
    let before = PhenotypeCompiler::compile(&genome, &capacity, &development, SensorProfile::PrivilegedAffordanceV1).unwrap();
    genome.sparse_density_priors[0].density = NormalizedScalar::new(0.08).unwrap();
    let after = PhenotypeCompiler::compile(&genome, &capacity, &development, SensorProfile::PrivilegedAffordanceV1).unwrap();
    assert_ne!(before.phenotype_hash(), after.phenotype_hash());
    assert_ne!(before.synapses().len(), after.synapses().len());
}

#[test]
fn large_named_tiers_are_research_gated() {
    for tier in [
        BrainScaleTier::Large4096,
        BrainScaleTier::Cognitive32768,
        BrainScaleTier::Student131k,
        BrainScaleTier::Ascended1M,
        BrainScaleTier::Ascended5M,
        BrainScaleTier::ResearchCustom,
    ] {
        let legacy_id = LegacyBrainClassAdapter::capacity_id_for_tier(tier);
        assert!(BrainCapacityClass::production_for_id(legacy_id).is_err());
    }
}

#[test]
fn same_id_with_forged_capacity_limits_is_rejected() {
    let mut json = serde_json::to_value(BrainCapacityClass::n512()).unwrap();
    json["execution"]["max_total_synapses"] = serde_json::json!(u32::MAX);
    assert!(serde_json::from_value::<BrainCapacityClass>(json).is_err());
}

#[test]
fn serialized_phenotype_is_rehashed_and_cannot_carry_stale_content() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 41);
    let mut json = serde_json::to_value(&phenotype).unwrap();
    json["microstep_count"] = serde_json::json!(4);
    assert!(serde_json::from_value::<alife_core::BrainPhenotype>(json).is_err());
}

#[test]
fn candidate_decoder_plan_covers_exactly_the_action_decoder_synapses() {
    let phenotype = compile(BrainCapacityClass::N512_ID, 41);
    let decoder = phenotype.candidate_decoder();
    decoder.validate_against(&phenotype).unwrap();
    assert_eq!(
        decoder.decoder_synapse_count(),
        phenotype.budgets().global.action_decoder_synapses,
    );
    assert_eq!(decoder.feature_count(), CANDIDATE_FEATURE_COUNT as u16);
}
```

Add a table-driven wire-DTO negative test that changes each execution field one
at a time (schema/layout versions, logical and recurrent/action/memory split
counts, memory/replay counts, all four alignments, feature mask width/value,
limits schema, both buffer limits, every binding/dynamic-buffer count, every
compute limit and x/y/z dimension, tile edges, feature width, microstep bounds,
maximum decoder-input stride, and compact-readback ceiling) and
asserts custom deserialization rejects every row. Add route/global budget tests
that reject overlap, omission, overflow, decoder double-counting, and an ABI
digest from another capacity.

In the same failing test file, add a table-driven causality test that mutates
every accepted production input family: lobe allocation, macro-connectome
enablement, route density, alpha, sensor manifest, motor affordance, and
development gate. Each row records the expected changed allocation and asserts
a changed phenotype hash. Add a neutral-field row that must be rejected rather
than silently accepted.

- [ ] **Step 2: Verify the compiler API is absent**

Run: `cargo test -p alife_core --test phenotype_compiler`

Expected: unresolved imports for phenotype types.

- [ ] **Step 3: Implement focused phenotype records**

Use these public shapes:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BrainExecutionBudget {
    schema_version: u16,
    gpu_layout_version: u16,
    max_neurons: u32,
    max_total_synapses: u32,
    max_recurrent_synapses: u32,
    max_action_decoder_synapses: u32,
    max_memory_decoder_synapses: u32,
    max_active_tiles: u32,
    max_candidates: u16,
    max_object_slots: u16,
    max_memory_context_records: u16,
    min_microsteps: u8,
    max_microsteps: u8,
    max_replay_events: u32,
    max_replay_eligibility_samples: u32,
    max_compact_readback_bytes: u32,
    microtile_edge: u16,
    supertile_edge: u16,
    candidate_feature_count: u16,
    max_decoder_input_lanes: u16,
    required_limits_schema_version: u16,
    required_feature_mask_words: u8,
    required_feature_mask: u64,
    required_max_buffer_size: u64,
    required_max_storage_buffer_binding_size: u64,
    required_max_bind_groups: u32,
    required_max_bindings_per_bind_group: u32,
    required_max_storage_buffers_per_shader_stage: u32,
    required_max_uniform_buffers_per_shader_stage: u32,
    required_max_dynamic_storage_buffers_per_pipeline_layout: u32,
    required_max_dynamic_uniform_buffers_per_pipeline_layout: u32,
    required_max_compute_workgroup_storage_size: u32,
    required_max_compute_workgroup_size_x: u32,
    required_max_compute_workgroup_size_y: u32,
    required_max_compute_workgroup_size_z: u32,
    required_max_compute_invocations_per_workgroup: u32,
    required_max_compute_workgroups_per_dimension: u32,
    storage_offset_alignment_bytes: u32,
    uniform_offset_alignment_bytes: u32,
    copy_buffer_alignment_bytes: u32,
    copy_bytes_per_row_alignment: u32,
}

impl BrainExecutionBudget {
    pub const fn schema_version(&self) -> u16;
    pub const fn gpu_layout_version(&self) -> u16;
    pub const fn max_neurons(&self) -> u32;
    pub const fn max_total_synapses(&self) -> u32;
    pub const fn max_recurrent_synapses(&self) -> u32;
    pub const fn max_action_decoder_synapses(&self) -> u32;
    pub const fn max_memory_decoder_synapses(&self) -> u32;
    pub const fn max_active_tiles(&self) -> u32;
    pub const fn max_candidates(&self) -> u16;
    pub const fn max_object_slots(&self) -> u16;
    pub const fn max_memory_context_records(&self) -> u16;
    pub const fn microstep_range(&self) -> (u8, u8);
    pub const fn max_replay_events(&self) -> u32;
    pub const fn max_replay_eligibility_samples(&self) -> u32;
    pub const fn max_compact_readback_bytes(&self) -> u32;
    pub const fn microtile_edge(&self) -> u16;
    pub const fn supertile_edge(&self) -> u16;
    pub const fn candidate_feature_count(&self) -> u16;
    pub const fn max_decoder_input_lanes(&self) -> u16;
    pub const fn required_limits_schema_version(&self) -> u16;
    pub const fn required_feature_mask_words(&self) -> u8;
    pub const fn required_feature_mask(&self) -> u64;
    pub const fn required_max_buffer_size(&self) -> u64;
    pub const fn required_max_storage_buffer_binding_size(&self) -> u64;
    pub const fn required_max_bind_groups(&self) -> u32;
    pub const fn required_max_bindings_per_bind_group(&self) -> u32;
    pub const fn required_max_storage_buffers_per_shader_stage(&self) -> u32;
    pub const fn required_max_uniform_buffers_per_shader_stage(&self) -> u32;
    pub const fn required_max_dynamic_storage_buffers_per_pipeline_layout(&self) -> u32;
    pub const fn required_max_dynamic_uniform_buffers_per_pipeline_layout(&self) -> u32;
    pub const fn required_max_compute_workgroup_storage_size(&self) -> u32;
    pub const fn required_max_compute_workgroup_size_x(&self) -> u32;
    pub const fn required_max_compute_workgroup_size_y(&self) -> u32;
    pub const fn required_max_compute_workgroup_size_z(&self) -> u32;
    pub const fn required_max_compute_invocations_per_workgroup(&self) -> u32;
    pub const fn required_max_compute_workgroups_per_dimension(&self) -> u32;
    pub const fn storage_offset_alignment_bytes(&self) -> u32;
    pub const fn uniform_offset_alignment_bytes(&self) -> u32;
    pub const fn copy_buffer_alignment_bytes(&self) -> u32;
    pub const fn copy_bytes_per_row_alignment(&self) -> u32;
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BrainCapacityClass {
    id: BrainClassId,
    execution: BrainExecutionBudget,
}

impl BrainCapacityClass {
    pub const N512_ID: BrainClassId = BrainClassId(1);
    pub const N1024_ID: BrainClassId = BrainClassId(2);
    pub const N2048_ID: BrainClassId = BrainClassId(3);

    pub fn n512() -> Self;
    pub fn n1024() -> Self;
    pub fn n2048() -> Self;
    pub fn production_for_id(id: BrainClassId) -> Result<Self, ScaffoldContractError>;
    pub fn production_classes() -> [Self; 3];
    pub const fn id(&self) -> BrainClassId;
    pub const fn execution(&self) -> &BrainExecutionBudget;
    pub fn canonical_digest(&self) -> [u64; 4];
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteBudgetReceipt {
    pub route_index: u16,
    pub active_tiles: u32,
    pub recurrent_synapses: u32,
    pub action_decoder_synapses: u32,
    pub memory_decoder_synapses: u32,
    pub immutable_payload_words: u32,
    pub tile_ceiling: u32,
    pub synapse_ceiling: u32,
    pub payload_word_ceiling: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalPhenotypeBudgetReceipt {
    pub neuron_count: u32,
    pub active_tiles: u32,
    pub recurrent_synapses: u32,
    pub action_decoder_synapses: u32,
    pub memory_decoder_synapses: u32,
    pub total_synapses: u32,
    pub immutable_payload_words: u32,
    pub candidate_capacity: u16,
    pub object_slot_capacity: u16,
    pub memory_context_capacity: u16,
    pub decoder_input_lanes: u16,
    pub replay_event_capacity: u32,
    pub replay_eligibility_sample_capacity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledBudgets {
    pub capacity_class_id: BrainClassId,
    pub execution_abi_digest: [u64; 4],
    pub routes: Vec<RouteBudgetReceipt>,
    pub global: GlobalPhenotypeBudgetReceipt,
}

impl CompiledBudgets {
    pub fn validate_against(
        &self,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct NeuronDynamics {
    bias: f32,
    leak: f32,
    activation: ActivationFunction,
    activity_ema_decay: f32,
    metabolic_decay: f32,
    homeostatic_gain: f32,
}

impl NeuronDynamics {
    pub const fn bias(&self) -> f32;
    pub const fn leak(&self) -> f32;
    pub const fn activation(&self) -> ActivationFunction;
    pub const fn activity_ema_decay(&self) -> f32;
    pub const fn metabolic_decay(&self) -> f32;
    pub const fn homeostatic_gain(&self) -> f32;
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensorEncoderSourceGroup {
    SensoryChannel = 1,
    Body = 2,
    Homeostasis = 3,
}

impl SensorEncoderSourceGroup {
    pub const fn raw(self) -> u16 { self as u16 }
    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SensorEncoderAssignment {
    source_group: SensorEncoderSourceGroup,
    source_index: u16,
    target_neuron: u32,
    scale: f32,
    bias: f32,
    clamp_min: f32,
    clamp_max: f32,
}

impl SensorEncoderAssignment {
    pub const fn source_group(&self) -> SensorEncoderSourceGroup;
    pub const fn source_index(&self) -> u16;
    pub const fn target_neuron(&self) -> u32;
    pub const fn scale(&self) -> f32;
    pub const fn bias(&self) -> f32;
    pub const fn clamp_range(&self) -> (f32, f32);
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SensorEncoderPlan {
    schema_version: u16,
    sensor_profile: SensorProfile,
    sensory_lane_count: u16,
    body_lane_count: u16,
    homeostasis_lane_count: u16,
    assignments: Vec<SensorEncoderAssignment>,
    canonical_digest: [u64; 4],
}

impl SensorEncoderPlan {
    pub const fn schema_version(&self) -> u16;
    pub const fn sensor_profile(&self) -> SensorProfile;
    pub const fn sensory_lane_count(&self) -> u16;
    pub const fn body_lane_count(&self) -> u16;
    pub const fn homeostasis_lane_count(&self) -> u16;
    pub fn assignments(&self) -> &[SensorEncoderAssignment];
    pub const fn canonical_digest(&self) -> [u64; 4];
    pub fn validate_against(
        &self,
        phenotype: &BrainPhenotype,
    ) -> Result<(), ScaffoldContractError>;
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DecoderHeadKind {
    ActionCandidate = 1,
    MemoryContext = 2,
}

impl DecoderHeadKind {
    pub const fn raw(self) -> u8 { self as u8 }
    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct DecoderSynapseCoordinate {
    head: DecoderHeadKind,
    family: CandidateActionFamily,
    input_lane: u16,
    motor_index: u16,
}

impl DecoderSynapseCoordinate {
    pub const fn head(&self) -> DecoderHeadKind;
    pub const fn family(&self) -> CandidateActionFamily;
    pub const fn input_lane(&self) -> u16;
    pub const fn motor_index(&self) -> u16;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum CompiledSynapseKind {
    Recurrent,
    Decoder(DecoderSynapseCoordinate),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CompiledSynapse {
    source: u32,
    target: u32,
    genetic_weight: f32,
    alpha: f32,
    route_index: u16,
    kind: CompiledSynapseKind,
}

impl CompiledSynapse {
    pub const fn source(&self) -> u32;
    pub const fn target(&self) -> u32;
    pub const fn genetic_weight(&self) -> f32;
    pub const fn alpha(&self) -> f32;
    pub const fn route_index(&self) -> u16;
    pub const fn kind(&self) -> CompiledSynapseKind;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompiledProjection {
    route_index: u16,
    source_lobe: LobeKind,
    target_lobe: LobeKind,
    projection_type: ProjectionType,
    active_tile_policy: ActiveTilePolicy,
    update_cadence: UpdateCadence,
    priority: BiologicalPriority,
    delay_microsteps: u8,
    synapse_start: u32,
    synapse_len: u32,
    active_tile_count: u32,
}

impl CompiledProjection {
    pub const fn route_index(&self) -> u16;
    pub const fn source_lobe(&self) -> LobeKind;
    pub const fn target_lobe(&self) -> LobeKind;
    pub const fn projection_type(&self) -> ProjectionType;
    pub const fn active_tile_policy(&self) -> ActiveTilePolicy;
    pub const fn update_cadence(&self) -> UpdateCadence;
    pub const fn priority(&self) -> BiologicalPriority;
    pub const fn delay_microsteps(&self) -> u8;
    pub const fn synapse_range(&self) -> (u32, u32);
    pub const fn active_tile_count(&self) -> u32;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CandidateDecoderFamilyPlan {
    family: CandidateActionFamily,
    bias: f32,
    decoder_synapse_start: u32,
    decoder_synapse_count: u32,
}

impl CandidateDecoderFamilyPlan {
    pub const fn family(&self) -> CandidateActionFamily;
    pub const fn bias(&self) -> f32;
    pub const fn decoder_synapse_start(&self) -> u32;
    pub const fn decoder_synapse_count(&self) -> u32;
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CandidateDecoderPlan {
    schema_version: u16,
    motor_start: u32,
    motor_width: u16,
    feature_count: u16,
    flattened_input_lane_count: u16,
    families: Vec<CandidateDecoderFamilyPlan>,
    canonical_digest: [u64; 4],
}

impl CandidateDecoderPlan {
    pub const fn motor_start(&self) -> u32;
    pub const fn motor_width(&self) -> u16;
    pub const fn feature_count(&self) -> u16;
    pub const fn flattened_input_lane_count(&self) -> u16;
    pub fn families(&self) -> &[CandidateDecoderFamilyPlan];
    pub fn decoder_synapse_count(&self) -> u32;
    pub const fn canonical_digest(&self) -> [u64; 4];
    pub fn validate_against(
        &self,
        phenotype: &BrainPhenotype,
    ) -> Result<(), ScaffoldContractError>;
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PhenotypeCompilerInputs {
    schema_version: u16,
    genome: BrainGenome,
    development: DevelopmentState,
    sensor_profile: SensorProfile,
    capacity_class_id: BrainClassId,
    capacity_digest: [u64; 4],
    canonical_digest: [u64; 4],
}

impl PhenotypeCompilerInputs {
    pub fn try_new(
        genome: BrainGenome,
        capacity: &BrainCapacityClass,
        development: DevelopmentState,
        sensor_profile: SensorProfile,
    ) -> Result<Self, ScaffoldContractError>;
    pub const fn canonical_digest(&self) -> [u64; 4];
    pub const fn sensor_profile(&self) -> SensorProfile;
    pub const fn capacity_class_id(&self) -> BrainClassId;
    pub fn validate_against(
        &self,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError>;
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BrainPhenotype {
    schema_version: u16,
    compiler_inputs_digest: [u64; 4],
    brain_class_id: BrainClassId,
    neuron_count: u32,
    microstep_count: u8,
    sensor_profile: SensorProfile,
    lobe_layout: LobeLayout,
    projections: Vec<CompiledProjection>,
    synapses: Vec<CompiledSynapse>,
    neuron_dynamics: Vec<NeuronDynamics>,
    sensor_encoder: SensorEncoderPlan,
    decoder: CandidateDecoderPlan,
    budgets: CompiledBudgets,
    phenotype_hash: PhenotypeHash,
}

impl BrainPhenotype {
    pub const fn schema_version(&self) -> u16;
    pub const fn brain_class_id(&self) -> BrainClassId;
    pub const fn neuron_count(&self) -> u32;
    pub const fn microstep_count(&self) -> u8;
    pub const fn sensor_profile(&self) -> SensorProfile;
    pub const fn compiler_inputs_digest(&self) -> [u64; 4];
    pub const fn phenotype_hash(&self) -> PhenotypeHash;
    pub const fn lobe_layout(&self) -> &LobeLayout;
    pub fn projections(&self) -> &[CompiledProjection];
    pub fn synapses(&self) -> &[CompiledSynapse];
    pub fn neuron_dynamics(&self) -> &[NeuronDynamics];
    pub fn sensor_encoder(&self) -> &SensorEncoderPlan;
    pub fn candidate_decoder(&self) -> &CandidateDecoderPlan;
    pub const fn budgets(&self) -> &CompiledBudgets;
    pub fn recompute_phenotype_hash(&self) -> Result<PhenotypeHash, ScaffoldContractError>;
    pub fn validate_against(
        &self,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError>;
}

pub struct PhenotypeCompiler;

impl PhenotypeCompiler {
    pub fn compile_validated(
        inputs: &PhenotypeCompilerInputs,
        capacity: &BrainCapacityClass,
    ) -> Result<BrainPhenotype, ScaffoldContractError>;
    pub fn compile(
        genome: &BrainGenome,
        capacity: &BrainCapacityClass,
        development: &DevelopmentState,
        sensor_profile: SensorProfile,
    ) -> Result<BrainPhenotype, ScaffoldContractError>;
}
```

`BrainCapacityClass` contains only `{ id, execution }`; `BrainExecutionBudget`
contains logical ceilings plus the complete versioned storage/dispatch ABI
floor. It must not contain a lobe layout, routing matrix, population limit, GPU
heap allocation, or semantic capability flag. The three constructors set exact
logical tuples `(neurons, total synapses, active tiles)` to
`(512, 8_192, 64)`, `(1_024, 16_384, 128)`, and
`(2_048, 32_768, 192)`. All use 32 candidates, 16 object slots, two-to-four
microsteps, 16x16 microtiles, 128x128 supertiles, the core
`CANDIDATE_FEATURE_COUNT`, a 64-lane maximum flattened decoder input stride, a
64-byte compact-readback ceiling, bounded
memory/replay counts, and recurrent/action-decoder/memory-decoder ceilings whose
sum equals the total (respectively 6_144/1_024/1_024,
12_288/2_048/2_048, and 24_576/4_096/4_096). They also carry explicit
storage/uniform/copy-offset/copy-row alignment, schema and GPU-layout versions,
a versioned fixed-width required feature mask, and every wgpu limit consumed by
buffer binding or x/y/z dispatch (including binding counts, dynamic counts,
workgroup storage, all workgroup dimensions, invocations, and workgroups per
dimension).
Runtime heap/population admission remains Slice D's responsibility.

`CandidateDecoderPlan` is not an informal shader convention. Its family rows
are sorted by the stable `CandidateActionFamily` discriminant, contain finite
biases, cover a contiguous and disjoint action-decoder global-synapse segment,
and bind the exact motor range, feature width, and flattened input-lane width
used by WGSL. Validation rejects duplicate or missing production families, an
out-of-lobe motor range, a feature width different from the capacity ABI, an
input stride above `max_decoder_input_lanes`, any gap/overlap, and a decoder
count different from `CompiledBudgets.global.action_decoder_synapses`. Its
digest participates in the phenotype hash. Both the plan and family-row fields
remain private, but the read-only accessors above expose every value needed by
the GPU compiler without permitting an alternate construction or mutation
path.

`SensorEncoderPlan` is the corresponding compiler-owned input contract. Its
three source groups have the stable raw mapping above. The canonical perception
upload is exactly `SensoryChannels::as_flat_array()`, then the 13 body lanes
`translation.xyz, rotation.xyzw, linear_velocity.xyz, angular_velocity.xyz`,
then `DriveSnapshot::to_array()` followed by
`EndocrineSnapshot::to_array()`. The plan records and validates those exact
group widths for its sensory ABI version. Assignments are sorted by
`(target_neuron, source_group.raw(), source_index)`, finite, non-overlapping,
and in range; every enabled input gene must produce at least one assignment and
every assignment target must lie in an enabled input lobe. Scale/bias and clamp
bounds are compiler outputs, not shader defaults. Custom deserialization uses a
private DTO, revalidates every assignment and profile-specific lane, recomputes
the canonical digest, and rejects a changed or stale digest. The encoder-plan
digest participates in the phenotype hash and the immutable phenotype evidence
manifest. The GPU upload is derived only from these read-only accessors.

`NeuronDynamics`, `CompiledProjection`, and `CompiledSynapse` are likewise
compiler-owned private records with private DTO deserializers. Stable raw
conversions are total and tested for every enum that crosses the GPU boundary.
Dynamics validation requires finite bias/leak/gain, leak and both decay lanes
in `[0,1]`, and homeostatic gain in `[0,2]`; every lane participates in the
phenotype hash and exact GPU record.
Every decoder synapse carries its exact head/family/input/motor coordinate, so
the decoder weight-index view is deterministically reconstructible from the
serialized phenotype and bound by its hash. Slice A accepts only
`DecoderHeadKind::ActionCandidate`; Slice C activates the reserved memory head.
For this first causal slice, the compiler accepts only
`delay_microsteps == 0` and rejects nonzero delay genes. A later delay-ring
slice must add explicit history storage before enabling nonzero values. Cadence
changes dispatch participation, active-tile policy changes compiled tile
membership, projection type enforces route/sign behavior, and biological
priority breaks deterministic route admission when a capacity ceiling binds;
none of those accepted fields may remain metadata-only.

All capacity and execution fields are private. Implement a custom
`Deserialize` for `BrainCapacityClass` through a private wire DTO: deserialize,
resolve the canonical constructor by ID, compare every nested field, and reject
any mismatch. `BrainExecutionBudget` has no public constructor or unchecked
deserializer. `validate_contract` repeats the complete comparison at every
phenotype/buffer/backend authority boundary; callers can inspect only the
stable accessors. A same-ID record with one altered alignment, feature, replay,
storage, alignment, feature-mask width/value, ABI version, compute dimension,
decoder split, or logical ceiling is therefore neither constructible with a
literal nor accepted from persistence.

`CompiledBudgets::validate_against` requires unique route IDs, exact route
coverage, and checked sums. Its global total must equal recurrent + action
decoder + memory decoder synapses and the exact compiled synapse vector length;
the three sets are a disjoint global-ID union. Slice A emits zero memory-decoder
synapses, and Slice C increases that lane inside the same total rather than
creating a parallel budget. Every global candidate/object/context/replay value
must be within the canonical execution budget and the execution ABI digest must
match the canonical little-endian budget tuple.
`BrainCapacityClass::canonical_digest` is that same four-stream,
domain-separated little-endian digest over every private class/execution field;
`CompiledBudgets.execution_abi_digest` must equal it exactly.

Keep
`BrainClassSpec` readable only as a versioned legacy adapter until Slice D
finishes save migration. Keep `BrainScaleTier` inside that legacy adapter;
production phenotype, backend, benchmark, and acceptance APIs consume
`BrainClassId`/`BrainCapacityClass` and never call `production_for_tier`.
`PhenotypeCompiler::compile`, buffer planning, and backend insertion call the
validation before using any limit.

`BrainPhenotype` is compiler-authored: every field is private, it has no public
literal constructor, and it does not derive unchecked `Deserialize`.
Implement custom deserialization through a private wire DTO. The loader
resolves the canonical `BrainCapacityClass`, validates every nested range,
budget, projection, sensor-encoder assignment, decoder row, finite float,
sorted ID, and input digest, then recomputes the complete canonical phenotype
hash and requires exact equality with the serialized hash. The hash excludes
only its own field. A
stale hash, a changed nested value with the old hash, or an unknown compiler
input/schema version is rejected before a phenotype can reach buffer planning.
The read-only accessors expose the complete immutable execution view needed by
`alife_gpu_backend` (schema/class, neuron and microstep counts, sensor profile,
lobe ranges, projections, synapses, neuron dynamics, sensor encoder, decoder, budgets, and
canonical identity) while preserving compiler-only construction.
Crate-local test helpers that lesion a phenotype must rebuild it through a
validated test-only compiler transform and rehash; they may not mutate fields
through public access.

`PhenotypeCompilerInputs` is the immutable, versioned compilation-provenance
record. Its custom deserializer recomputes its digest over the complete genome,
development state, sensor profile, capacity ID, and canonical capacity digest.
`compile` is a convenience wrapper that first creates this record;
`compile_validated` is the single implementation. Persistence in Slice B stores
both this compiler-input asset and the immutable serialized phenotype asset.
Restore validates both assets, recompiles from the inputs, and requires
canonical byte-for-byte equality and equal phenotype hashes before allocating
a GPU slot. The mutable checkpoint never becomes an alternate phenotype
source.

The implementation must compile lobe ratios, enabled macro masks, density shares, alpha, active sensors, active motor affordances, and developmental gates. Decoder projection entries are compiled synapses in the same global budget and SoA weight pools, not a second hidden weight store. Generate at least 128 synapses for N512 and scale within the global ceiling. Sort routes and synapses by stable IDs before hashing. Hash canonical little-endian fields with four fixed-seed SplitMix64 streams; do not use `DefaultHasher`.

The implementation must make every table row pass. If an accepted field cannot
affect the phenotype, remove or reject it instead of allowing a neutral
production gene.

- [ ] **Step 4: Validate macro connectome masks**

Replace the unconditional validator with checks that source and target differ only when the projection type allows it, keys are unique, enabled masks reference enabled compiled lobes, and every density prior references an enabled mask.

- [ ] **Step 5: Run phenotype and topology tests**

Run: `cargo test -p alife_core --test phenotype_compiler --test brain_topology --test genome_weight_split`

Expected: all tests pass.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_core/src/phenotype.rs crates/alife_core/src/brain_class.rs crates/alife_core/src/genome.rs crates/alife_core/src/lobe.rs crates/alife_core/src/neural.rs crates/alife_core/src/routing.rs crates/alife_core/src/lib.rs crates/alife_core/tests/phenotype_compiler.rs
git commit -m "Compile genomes into production brain phenotypes"
```

### Task 4: Replace CPU-schema uploads with phenotype-owned GPU buffers

**Files:**
- Create: `crates/alife_gpu_backend/src/closed_loop_buffers.rs`
- Create: `crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl`
- Create: `crates/alife_gpu_backend/tests/closed_loop_buffer_contracts.rs`
- Modify: `crates/alife_gpu_backend/Cargo.toml`
- Modify: `crates/alife_gpu_backend/src/lib.rs`
- Modify: `crates/alife_gpu_backend/src/buffers.rs:423-677` (remove after all callers use the phenotype-owned pools)

**Interfaces:**
- Consumes: `BrainPhenotype`, `PerceptionFrame`, `MAX_ACTION_CANDIDATES`, `CANDIDATE_FEATURE_COUNT`.
- Produces: `GpuPhenotypeUpload`, `GpuPerceptionUpload`, `GpuSelectionRecord`, `GpuClassBucketPlan`, `GpuClassBucketBuffers`, `GpuBrainSlot`.

- [ ] **Step 1: Add failing ABI tests**

```rust
use alife_gpu_backend::{
    GpuBrainSlotRecord, GpuCandidateRecord, GpuDecoderFamilyRecord,
    GpuDecoderPlanRecord, GpuDecoderWeightIndexRecord, GpuEncoderAssignmentRecord, GpuEncoderPlanRecord,
    GpuNeuronDynamicsRecord, GpuPerceptionHeader, GpuPhenotypeIdentityRecord,
    GpuProjectionRecord, GpuRouteMetadataRecord, GpuSelectionRecord,
    GPU_BRAIN_SLOT_RECORD_BYTES, GPU_CANDIDATE_RECORD_BYTES, GPU_PERCEPTION_HEADER_BYTES,
    GPU_SELECTION_RECORD_BYTES,
};

#[test]
fn closed_loop_records_have_stable_aligned_sizes() {
    assert_eq!(std::mem::size_of::<GpuPerceptionHeader>(), GPU_PERCEPTION_HEADER_BYTES);
    assert_eq!(std::mem::size_of::<GpuBrainSlotRecord>(), GPU_BRAIN_SLOT_RECORD_BYTES);
    assert_eq!(std::mem::size_of::<GpuCandidateRecord>(), GPU_CANDIDATE_RECORD_BYTES);
    assert_eq!(std::mem::size_of::<GpuSelectionRecord>(), GPU_SELECTION_RECORD_BYTES);
    assert_eq!(std::mem::size_of::<GpuPhenotypeIdentityRecord>(), 32);
    assert_eq!(GPU_PERCEPTION_HEADER_BYTES, 64);
    assert_eq!(GPU_BRAIN_SLOT_RECORD_BYTES, 144);
    assert_eq!(GPU_CANDIDATE_RECORD_BYTES, 32);
    assert_eq!(GPU_SELECTION_RECORD_BYTES, 48);
    assert_eq!(std::mem::align_of::<GpuPerceptionHeader>(), 16);
    assert_eq!(std::mem::align_of::<GpuBrainSlotRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuCandidateRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuSelectionRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuPhenotypeIdentityRecord>(), 16);
    assert_eq!(std::mem::size_of::<GpuEncoderPlanRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuEncoderAssignmentRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuNeuronDynamicsRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuProjectionRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuRouteMetadataRecord>(), 48);
    assert_eq!(std::mem::size_of::<GpuDecoderPlanRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuDecoderFamilyRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuDecoderWeightIndexRecord>(), 16);
    assert_eq!(std::mem::offset_of!(GpuPerceptionHeader, active_activation_side), 28);
    assert_eq!(std::mem::offset_of!(GpuPerceptionHeader, brain_slot_index), 48);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotRecord, recurrent_synapse_count), 28);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotRecord, recurrent_eligibility_offset), 100);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotRecord, neuron_homeostasis_offset), 124);
    assert_eq!(std::mem::offset_of!(GpuBrainSlotRecord, extension_record_offset), 128);
    assert_eq!(std::mem::offset_of!(GpuSelectionRecord, dispatch_generation_lo), 36);
    assert_eq!(std::mem::offset_of!(GpuSelectionRecord, active_activation_side), 44);
}
```

In the same failing file, add N512/N1024/N2048 byte-count, candidate overflow,
single genetic owner, recurrent `target_offsets`, exact disjoint synapse-view
coverage, two-slot range isolation, and cross-slot mutation isolation tests.
Assert `target_offsets.last() == recurrent_synapse_count`, decoder synapse IDs
never appear in recurrent CSR, and the disjoint union of recurrent CSR IDs and
decoder-index IDs equals every global compiled synapse ID exactly once.

- [ ] **Step 2: Verify the new records are absent**

Run: `cargo test -p alife_gpu_backend --test closed_loop_buffer_contracts`

Expected: unresolved imports.

- [ ] **Step 3: Implement the GPU ABI and SoA plan**

Add `bytemuck.workspace = true` and use explicit `#[repr(C)]` POD records:

```rust
#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuPerceptionHeader {
    pub schema_version: u32,
    pub class_id: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub neuron_count: u32,
    pub candidate_count: u32,
    pub microstep_count: u32,
    pub active_activation_side: u32,
    pub tick_lo: u32,
    pub tick_hi: u32,
    pub sensory_offset: u32,
    pub candidate_offset: u32,
    pub brain_slot_index: u32,
    pub reserved: [u32; 3],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuBrainSlotRecord {
    pub schema_version: u32,
    pub class_id: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub neuron_count: u32,
    pub microstep_count: u32,
    pub synapse_count: u32,
    pub recurrent_synapse_count: u32,
    pub encoder_plan_offset: u32,
    pub neuron_dynamics_offset: u32,
    pub projection_offset: u32,
    pub route_metadata_offset: u32,
    pub target_offsets_offset: u32,
    pub source_indices_offset: u32,
    pub route_indices_offset: u32,
    pub decoder_plan_offset: u32,
    pub decoder_family_offset: u32,
    pub decoder_weight_indices_offset: u32,
    pub genetic_weight_offset: u32,
    pub alpha_offset: u32,
    pub activation_a_offset: u32,
    pub activation_b_offset: u32,
    pub accumulator_offset: u32,
    pub lifetime_weight_offset: u32,
    pub fast_weight_offset: u32,
    pub recurrent_eligibility_offset: u32,
    pub decoder_eligibility_offset: u32,
    pub encoded_input_offset: u32,
    pub candidate_logit_offset: u32,
    pub diagnostic_offset: u32,
    pub selection_offset: u32,
    pub neuron_homeostasis_offset: u32,
    pub extension_record_offset: u32,
    pub reserved: [u32; 3],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuPhenotypeIdentityRecord {
    pub phenotype_hash: [u32; 8],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuCandidateRecord {
    pub action_id: u32,
    pub kind: u32,
    pub family: u32,
    pub candidate_index: u32,
    pub feature_offset: u32,
    pub observation_slot_or_max: u32,
    pub confidence_q16: u32,
    pub effort_q16: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuSelectionRecord {
    pub slot: u32,
    pub slot_generation: u32,
    pub candidate_index: u32,
    pub logit_bits: u32,
    pub confidence_q16: u32,
    pub status: u32,
    pub active_tiles: u32,
    pub active_synapses: u32,
    pub finite_rejections: u32,
    pub dispatch_generation_lo: u32,
    pub dispatch_generation_hi: u32,
    pub active_activation_side: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuEncoderPlanRecord {
    pub schema_version: u32,
    pub sensor_profile_raw: u32,
    pub assignment_offset: u32,
    pub assignment_count: u32,
    pub target_offsets_offset: u32,
    pub sensory_lane_count: u32,
    pub body_lane_count: u32,
    pub homeostasis_lane_count: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuEncoderAssignmentRecord {
    pub source_group_raw: u32,
    pub source_index: u32,
    pub target_neuron: u32,
    pub reserved0: u32,
    pub scale_bits: u32,
    pub bias_bits: u32,
    pub clamp_min_bits: u32,
    pub clamp_max_bits: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuNeuronDynamicsRecord {
    pub bias_bits: u32,
    pub leak_bits: u32,
    pub activation_raw: u32,
    pub homeostatic_gain_bits: u32,
    pub activity_ema_decay_bits: u32,
    pub metabolic_decay_bits: u32,
    pub reserved0: u32,
    pub reserved1: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuProjectionRecord {
    pub route_index: u32,
    pub source_lobe_raw: u32,
    pub target_lobe_raw: u32,
    pub synapse_start: u32,
    pub synapse_count: u32,
    pub active_tile_count: u32,
    pub reserved0: u32,
    pub reserved1: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuRouteMetadataRecord {
    pub route_index: u32,
    pub projection_type_raw: u32,
    pub active_tile_policy_raw: u32,
    pub update_cadence_raw: u32,
    pub biological_priority_raw: u32,
    pub delay_microsteps: u32,
    pub source_start: u32,
    pub source_count: u32,
    pub target_start: u32,
    pub target_count: u32,
    pub reserved0: u32,
    pub reserved1: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuDecoderPlanRecord {
    pub schema_version: u32,
    pub motor_start: u32,
    pub motor_width: u32,
    pub feature_count: u32,
    pub flattened_input_lane_count: u32,
    pub family_offset: u32,
    pub family_count: u32,
    pub decoder_synapse_count: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuDecoderFamilyRecord {
    pub family_raw: u32,
    pub bias_bits: u32,
    pub decoder_synapse_start: u32,
    pub decoder_synapse_count: u32,
    pub weight_index_start: u32,
    pub weight_index_count: u32,
    pub reserved0: u32,
    pub reserved1: u32,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuDecoderWeightIndexRecord {
    pub global_synapse_id: u32,
    pub input_lane: u32,
    pub motor_index: u32,
    pub reserved0: u32,
}
```

Define byte-for-byte WGSL mirrors for all eight plan records in the shared
`closed_loop_abi.wgsl` prefix. Rust concatenates that exact prefix with each
entry-point module before Naga/pipeline creation; WGSL entry files never copy
or redefine the records. Use only `u32` fields in WGSL and recover floats with
`bitcast<f32>` after finite validation on upload. Add stable, total
`raw`/`try_from_raw` mappings for `ActivationFunction`, `LobeKind`,
`ProjectionType`, `ActiveTilePolicy`, `UpdateCadence`, and
`BiologicalPriority`, just as Task 2 does for action family and sensor profile;
reject every unknown raw value. The ABI test must assert every field offset,
the exact Rust size/alignment, and Naga-reflected WGSL member offsets/sizes. A
checked conversion test round-trips every enum row and rejects the first
unknown value.

Use one fixed, versioned class-bucket bind layout for every Slice-A entry
point; do not bind one resource set per creature:

```wgsl
@group(0) @binding(0) var<storage, read> brain_slots: array<GpuBrainSlotRecord>;
@group(0) @binding(1) var<storage, read> phenotype_identities: array<GpuPhenotypeIdentityRecord>;
@group(0) @binding(2) var<storage, read> immutable_plan_words: array<u32>;
@group(0) @binding(3) var<storage, read> immutable_weight_words: array<u32>;
@group(0) @binding(4) var<storage, read> dispatch_header_words: array<u32>;
@group(0) @binding(5) var<storage, read> frame_payload_words: array<u32>;
@group(0) @binding(6) var<storage, read_write> mutable_state_words: array<u32>;
```

All slot offsets except `brain_slot_index` are checked `u32`-word offsets into
one of those heaps, never host pointers or byte guesses. Generated
`load_encoder_plan`, `load_encoder_assignment`, `load_neuron_dynamics`,
`load_projection`, `load_route_metadata`, `load_decoder_plan`, and
`load_decoder_family` helpers reconstruct the exact WGSL structs from
`immutable_plan_words`; float payloads are bitcast only after the upload was
validated. Equivalent typed helpers read headers/frame rows and mutable state.
The semantic array names in later pseudocode denote those checked heap helpers,
not extra bindings. `GpuClassBucketBuffers` owns exactly these seven physical
buffers plus staging/readback buffers that are never simultaneously bound to a
neural pipeline. Slice B places its slot-extension and learning rows in the
same plan/state heaps and must update this layout version and helper set rather
than adding an eighth per-creature binding. Task 5 adds layout-reflection tests
that assert group/binding number, access mode, minimum binding size, and all
seven entries against both Rust descriptors and Naga.

`GpuClassBucketPlan::new` must allocate shared slot and sparse payload pools for
one `BrainCapacityClass`. `insert_phenotype` gives every recurrent/decoder
synapse one stable global synapse ID, then builds two disjoint views. Canonical
IDs put recurrent synapses first and decoder synapses second; each segment is
independently sorted by stable target/source/route or head/family/input/motor
identity. Thus
decoder local start equals `recurrent_synapse_count` without relying on route
interleaving, and the ordering itself participates in the phenotype hash. Recurrent
CSR contains only recurrent IDs in target-major spans: `target_offsets` of
length `neuron_count + 1` plus source/route arrays. A recurrent CSR entry's
zero-based ordinal is its global synapse ID, so no unaddressed parallel global-ID
array or extra offset exists; `target_offsets.last() == recurrent_synapse_count`.
Decoder family/feature maps
contain only decoder global IDs. Their disjoint union covers the global genetic,
alpha, lifetime, and fast pools exactly once. Every span is monotonic and in
bounds. Allocate topology/genetic/alpha payloads once per inserted
phenotype and mutable activation A/B, accumulator, two-lane-per-neuron
homeostasis (`activity_ema`, `metabolic_load`), lifetime, fast, eligibility,
encoded-input, candidate, selection, and diagnostic arrays as class-bucketed SoA pools. It
must not accept `CpuNeuralState` or `NeuralProjectionSchema`, and a slot must not
own a device, queue, or pipeline.

The plan pools are equally explicit: one `GpuEncoderPlanRecord` per phenotype,
one encoder target-offset span of `neuron_count + 1` plus its assignment rows,
one dynamics row per neuron, one projection and one route-metadata row per
compiled projection, one decoder-plan row, and one decoder-family row per
compiled family plus one decoder-weight-index row per decoder synapse.
`encoder_plan_offset`, `neuron_dynamics_offset`,
`projection_offset`, `route_metadata_offset`, and `decoder_plan_offset` point
to those shared pools; `decoder_family_offset` must equal the family base named
by the loaded decoder plan. `GpuPhenotypeUpload::try_from(&BrainPhenotype)` is the
only builder and derives every row from private validated accessors. It rejects
a count, ordering, range, digest, enum, float, or target-offset mismatch before
buffer allocation.
`neuron_homeostasis_offset` points to exactly `neuron_count * 2` mutable
`f32` lanes in target-major order. The slot's three reserved lanes are zero and
validated in Rust and WGSL.

The Slice-A lifetime, fast, recurrent-eligibility, and decoder-eligibility
offsets in `GpuBrainSlotRecord` are bank-0 bases. Slice B appends bank-1 spans
to those same four class-bucket storage buffers and records their bases plus
validated 0/1 active-bank selectors in the one slot-extension row. It must not
introduce separately bound `fast_staging`, `lifetime_staging`, or eligibility-
staging buffers. All waking, decode, learning, sleep, persistence, and digest
code resolves active/inactive bases through the shared selector helpers, so a
commit flips selectors rather than exchanging offsets across different buffer
bindings.
Store one full `GpuPhenotypeIdentityRecord` per slot in a shared class-bucketed
identity array indexed by `brain_slot_index`. It is initialized/scrubbed with
the generation-checked slot and lets later learning/sleep kernels validate the
complete phenotype hash without widening every dynamic header or trusting a
display prefix.

Every dispatch first resolves `brain_slots[header.brain_slot_index]`, then
requires matching class, slot, and generation. All immutable phenotype-plan and
mutable-state offsets come from that slot record; dynamic frame offsets come
from `GpuPerceptionHeader`. No kernel reads a singleton encoder, dynamics,
route, decoder, bias, weight-index, or candidate-logit base. Task B may append a
versioned record at `extension_record_offset` for receptor/learning data and
Task C may extend that record for memory-plan data, but neither may create a
per-creature resource set.
During Slice A alone, `extension_record_offset == u32::MAX` is the sole
validated no-extension sentinel and no A shader dereferences it. Slice B
allocates exactly one extension row per occupied slot, replaces the sentinel
before any learning dispatch, and thereafter forbids the sentinel for that
slot; Slice C extends the same row rather than replacing it.

- [ ] **Step 4: Implement ownership and bounds validation**

Make the prewritten tests pass: reject candidate count above 32, preserve one
immutable genetic owner, eliminate mutable projection duplication, enforce
`target_offsets.len() == neuron_count + 1`, monotonic recurrent spans, exact
disjoint-union synapse coverage, disjoint slot/payload ranges, and cross-slot
mutation isolation.

- [ ] **Step 5: Run buffer tests**

Run: `cargo test -p alife_gpu_backend --test closed_loop_buffer_contracts`

Expected: all tests pass.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_gpu_backend/Cargo.toml crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl crates/alife_gpu_backend/src/buffers.rs crates/alife_gpu_backend/src/closed_loop_buffers.rs crates/alife_gpu_backend/src/lib.rs crates/alife_gpu_backend/tests/closed_loop_buffer_contracts.rs
git commit -m "Add phenotype-owned GPU brain buffers"
```

### Task 5: Implement GPU sensory encoding and recurrent microsteps

**Files:**
- Create: `crates/alife_gpu_backend/shaders/closed_loop_encode.wgsl`
- Create: `crates/alife_gpu_backend/shaders/closed_loop_recurrent.wgsl`
- Create: `crates/alife_gpu_backend/src/closed_loop_pipeline.rs`
- Create: `crates/alife_gpu_backend/tests/closed_loop_wgsl.rs`
- Create: `crates/alife_gpu_backend/tests/support/mod.rs`
- Modify: `crates/alife_gpu_backend/src/lib.rs`

**Interfaces:**
- Consumes: `GpuClassBucketBuffers`, compiled projection descriptors, neuron dynamics, current perception uploads and active slot list.
- Produces: `GpuClosedLoopPipelines::encode`, `dispatch_microsteps`, active tile/synapse counters.

- [ ] **Step 1: Add failing WGSL parse and entry-point tests**

```rust
#[test]
fn closed_loop_wgsl_parses_and_exposes_required_entries() {
    for (source, entry) in [
        (alife_gpu_backend::CLOSED_LOOP_ENCODE_WGSL, "encode_perception"),
        (alife_gpu_backend::CLOSED_LOOP_RECURRENT_WGSL, "recurrent_microstep"),
    ] {
        let module = naga::front::wgsl::parse_str(source).unwrap();
        assert!(module.entry_points.iter().any(|point| point.name == entry));
    }
}
```

Under `#[cfg(feature = "gpu-tests")]`, also write the failing N512 hardware
test now: upload a nonzero frame, dispatch encode plus three microsteps, read a
manual diagnostic sample outside the product tick, and assert finite nonzero
activity plus nonzero active-synapse counters.

- [ ] **Step 2: Verify the shader constants are absent**

Run: `cargo test -p alife_gpu_backend --test closed_loop_wgsl`

Expected: unresolved constants.

- [ ] **Step 3: Implement `encode_perception`**

The kernel must treat `global_invocation_id.y` as the active-batch header index,
bounds-check `x` against that slot's neuron count, clamp input values, clear
the destination input range, and write current sensory/body/homeostatic values
before any recurrent dispatch. Dispatch dimensions are
`ceil(max_neurons / 64) x active_slot_count x 1`. Use this entry shape:

```wgsl
@compute @workgroup_size(64)
fn encode_perception(@builtin(global_invocation_id) gid: vec3<u32>) {
    let header = perception_headers[gid.y];
    let brain = brain_slots[header.brain_slot_index];
    if (brain.slot != header.slot || brain.slot_generation != header.slot_generation) { return; }
    let index = gid.x;
    if (index >= brain.neuron_count) { return; }
    let encoder = load_encoder_plan(brain.encoder_plan_offset);
    let begin = immutable_plan_words[encoder.target_offsets_offset + index];
    let end = immutable_plan_words[encoder.target_offsets_offset + index + 1u];
    var value = 0.0;
    for (var cursor = begin; cursor < end; cursor++) {
        let assignment = load_encoder_assignment(encoder.assignment_offset + cursor * 8u);
        if (assignment.target_neuron != index) { return; }
        let source_lane = resolve_encoder_source_lane(encoder, assignment);
        let source = load_frame_f32(header.sensory_offset + source_lane);
        value += clamp(
            source * bitcast<f32>(assignment.scale_bits)
                + bitcast<f32>(assignment.bias_bits),
            bitcast<f32>(assignment.clamp_min_bits),
            bitcast<f32>(assignment.clamp_max_bits),
        );
    }
    store_state_f32(brain.encoded_input_offset + index, clamp(value, -1.0, 1.0));
}
```

`resolve_encoder_source_lane` implements the exact three-group concatenation
from Task 3 and rejects an unknown group or out-of-range source during upload;
the shader still bounds-checks every resolved lane against the encoded group
widths. Header `sensory_offset` is the base of that complete canonical
sensory/body/homeostasis frame payload despite the retained field name. The
target-offset span has length `neuron_count + 1`, ends at
`assignment_count`, and makes each invocation visit only its compiled
assignments.

- [ ] **Step 4: Implement `recurrent_microstep`**

Dispatch one invocation per target neuron and active batch row. Resolve and
generation-check the `GpuBrainSlotRecord`, then iterate only
`target_offsets[brain.target_offsets_offset + target]` through the next offset,
skip routes whose cadence
does not fire on the current microstep, and accumulate locally before writing
that target's unique output slot; do not rely on floating-point atomics. Use
activation ping-pong buffers, immutable source/route indices, mutable
lifetime/fast weights, per-neuron leak/bias, route cadence flags, and finite
diagnostics. Effective weights must use
`genetic + lifetime + alpha * fast` using the slot record's genetic, alpha,
lifetime, and fast bases even though Slice A initializes mutable
layers to zero. Each microstep adds
`encoded_inputs[brain.encoded_input_offset + target]` to the local sum before
leak/bias/activation finalization, so current
perception is causally present throughout the configured two-to-four
microsteps without overwriting recurrent state.
For each target, load its two homeostatic lanes, subtract
`homeostatic_gain * metabolic_load` from the biased pre-activation value, apply
leak/activation, then update the unique target lanes as
`activity_ema = activity_ema_decay * old + (1-activity_ema_decay) * abs(output)` and
`metabolic_load = metabolic_decay * old + (1-metabolic_decay) * output^2`.
Clamp both to `[0,1]` and emit a finite diagnostic on violation. These rows are
part of mutable checkpoint state; they are not CPU estimates or audit-only
telemetry.
The upload validator requires every Slice-A route delay to be zero and the
shader treats a nonzero raw delay as an invalid-plan diagnostic rather than
silently ignoring it. Do not enable nonzero delay genes until a later plan adds
an explicitly sized, selector-safe activation-history ring and its save ABI.
The backend tracks the active ping/pong side per slot. It flips the side after
each completed microstep and writes the resulting 0/1 lane into the dynamic
header used by decode; configured counts 2, 3, and 4 therefore end on the
correct side without a bulk copy. `select_candidate` copies that validated lane
into `GpuSelectionRecord.active_activation_side`; the compact readback is the
authoritative side receipt for decision evidence and Slice B learning. The
backend also retains the same side in private slot metadata so the next frame
starts from the prior frame's final recurrent bank rather than resetting to A.

- [ ] **Step 5: Implement GPU test plumbing and satisfy the prewritten smoke test**

Implement `tests/support/mod.rs::GpuPipelineFixture` as the shared GPU-only
test harness. Its `new(phenotype)` creates a real adapter/device and bucket,
and `run_encode_microsteps(frame)` performs only upload, encode, the configured
microsteps, and a manual bounded diagnostic readback. Task 5 must not expose a
decode/select helper before Task 6 implements that pipeline. The fixture
contains upload/readback plumbing only and no CPU neural math.

- [ ] **Step 6: Run parser and hardware tests**

Run: `cargo test -p alife_gpu_backend --test closed_loop_wgsl`

Expected: parser tests pass.

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_wgsl -- --nocapture`

Expected on this workstation: the hardware smoke names an adapter and passes.

- [ ] **Step 7: Commit**

```powershell
git add crates/alife_gpu_backend/shaders/closed_loop_encode.wgsl crates/alife_gpu_backend/shaders/closed_loop_recurrent.wgsl crates/alife_gpu_backend/src/closed_loop_pipeline.rs crates/alife_gpu_backend/src/lib.rs crates/alife_gpu_backend/tests/closed_loop_wgsl.rs crates/alife_gpu_backend/tests/support/mod.rs
git commit -m "Run perception and recurrent brain steps on GPU"
```

### Task 6: Implement candidate-conditioned GPU decoding and winner selection

**Files:**
- Create: `crates/alife_gpu_backend/shaders/closed_loop_decode.wgsl`
- Create: `crates/alife_gpu_backend/tests/closed_loop_gpu_behavior.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_pipeline.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_buffers.rs`
- Modify: `crates/alife_gpu_backend/tests/support/mod.rs`
- Modify: `crates/alife_gpu_backend/src/lib.rs`

**Interfaces:**
- Consumes: motor activation range, `GpuCandidateRecord`, candidate feature buffer, compiled decoder plan.
- Produces: one `GpuSelectionRecord` selected entirely by GPU dispatch.

- [ ] **Step 1: Write the causal and bank-side failing tests**

Create hardware tests named exactly:

```rust
#[test]
fn same_candidates_different_sensory_frames_change_gpu_logits() {
    let checkpoint = causal_n512_gpu_checkpoint().unwrap();
    let (mut first_gpu, mut second_gpu) =
        restore_same_adapter_pair_from_checkpoint(&checkpoint).unwrap();
    let first_frame = two_candidate_frame([0.9, 0.0]);
    let second_frame = two_candidate_frame([0.0, 0.9]);
    assert_base_frames_differ_only_in_sensory(&first_frame, &second_frame);
    assert_eq!(first_frame.candidates(), second_frame.candidates());
    let first = first_gpu.run_frame(&first_frame).unwrap();
    let second = second_gpu.run_frame(&second_frame).unwrap();
    assert_ne!(first.selection.logit.to_bits(), second.selection.logit.to_bits());
}

#[test]
fn lesioning_motor_weights_changes_gpu_selection() {
    let frame = two_candidate_frame([0.8, -0.3]);
    let mut intact = GpuPipelineFixture::new(causal_n512_phenotype()).unwrap();
    let before = intact.run_frame(&frame).unwrap().selection;
    let mut lesioned = GpuPipelineFixture::new(lesion_motor_decoder(causal_n512_phenotype())).unwrap();
    let after = lesioned.run_frame(&frame).unwrap().selection;
    assert_ne!(before.candidate_index, after.candidate_index);
}

#[test]
fn zero_neural_weights_remove_non_idle_behavior() {
    let mut gpu = GpuPipelineFixture::new(zero_all_weights_and_biases(causal_n512_phenotype())).unwrap();
    let result = gpu.run_frame(&two_candidate_frame([0.8, -0.3])).unwrap();
    assert_eq!(result.selection.candidate_index, 0);
    assert_eq!(result.selection.logit, 0.0);
}

#[test]
fn same_adapter_replay_matches_the_declared_tolerance() {
    let phenotype = causal_n512_phenotype();
    let frames = deterministic_frame_sequence(64, 4101);
    let first = run_fresh_gpu_sequence(phenotype.clone(), &frames).unwrap();
    let second = run_fresh_gpu_sequence(phenotype, &frames).unwrap();
    assert_eq!(first.adapter_identity, second.adapter_identity);
    assert_eq!(first.selected_candidates, second.selected_candidates);
    for (a, b) in first.selected_logits.iter().zip(&second.selected_logits) {
        assert!((a - b).abs() <= first.tolerance);
    }
}

#[test]
fn decoder_reads_the_final_ping_pong_side_for_two_three_and_four_microsteps() {
    for microsteps in 2..=4 {
        let mut gpu = GpuPipelineFixture::new(side_sensitive_n512_phenotype(microsteps)).unwrap();
        let result = gpu.run_frame(&side_sensitive_frame()).unwrap();
        assert_eq!(result.active_activation_side, expected_final_side(microsteps));
        assert_eq!(result.selection.candidate_index, expected_candidate_for_side(microsteps));
    }
}
```

Each test must instantiate a real `GpuClosedLoopPipelines` and may read only the compact selection record.
The sensory-causality pair uses two slots on one adapter restored from identical
activation/weight/generation bytes; each slot receives exactly one frame. The
slots use distinct organism IDs required by ownership, but organism identity is
transport/evidence only and never an encoder lane. The helper asserts
tick/profile/body/homeostasis/context/candidate features and every neural input
other than sensory channels are identical, so ownership and recurrent history
cannot explain the logit difference.

- [ ] **Step 2: Run and verify missing decoder support**

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_gpu_behavior -- --nocapture`

Expected: compile failure for missing decode dispatch.

- [ ] **Step 3: Implement candidate decode WGSL**

Use one work item per candidate to calculate its feature-conditioned logit, then a bounded reduction entry `select_candidate` with stable lowest-index tie breaking. Entity IDs must not be bound as decoder features.

```wgsl
@compute @workgroup_size(32)
fn decode_candidates(@builtin(global_invocation_id) gid: vec3<u32>) {
    let header = perception_headers[gid.y];
    let brain = brain_slots[header.brain_slot_index];
    if (brain.slot != header.slot || brain.slot_generation != header.slot_generation) { return; }
    let decoder = load_decoder_plan(brain.decoder_plan_offset);
    let activation_offset = select(
        brain.activation_a_offset,
        brain.activation_b_offset,
        header.active_activation_side == 1u,
    );
    let candidate = gid.x;
    if (candidate >= header.candidate_count) { return; }
    let candidate_record = load_candidate(header.candidate_offset + candidate);
    let family = find_decoder_family(decoder, candidate_record.family);
    var logit = bitcast<f32>(family.bias_bits);
    for (var i = 0u; i < family.weight_index_count; i++) {
        let map = load_decoder_weight_index(family.weight_index_start + i * 4u);
        if (map.input_lane >= decoder.flattened_input_lane_count
            || map.input_lane >= decoder.feature_count
            || map.motor_index >= decoder.motor_width) { return; }
        let motor = load_state_f32(
            activation_offset + decoder.motor_start + map.motor_index
        );
        let feature = load_frame_f32(
            candidate_record.feature_offset + map.input_lane
        );
        let weight_index = map.global_synapse_id;
        let weight = load_genetic(brain.genetic_weight_offset + weight_index)
            + load_lifetime(brain.lifetime_weight_offset + weight_index)
            + load_alpha(brain.alpha_offset + weight_index)
                * load_fast(brain.fast_weight_offset + weight_index);
        logit += motor * feature * weight;
    }
    store_state_f32(brain.candidate_logit_offset + candidate, logit);
}
```

`find_decoder_family` searches only the decoder's validated sorted family span
and fails closed if the candidate raw value is absent. For each family,
`weight_index_count == decoder_synapse_count`; its map rows are sorted by
`(input_lane, motor_index, global_synapse_id)`, carry exactly the global IDs in
that family's contiguous `decoder_synapse_start..+count` range, and reproduce
the `DecoderSynapseCoordinate` in the hashed phenotype. Slice A requires
`flattened_input_lane_count == feature_count == CANDIDATE_FEATURE_COUNT`;
Slice C extends that validated lane space for memory context. No rectangular
`family * motor_width`, modulo-feature addressing, or shader-invented decoder
coordinate is permitted.

- [ ] **Step 4: Implement deterministic GPU winner selection**

Reject non-finite logits through a diagnostic counter, apply lateral-inhibition
metadata, choose maximum logit, and use the smallest candidate index for exact
ties. Write one 48-byte slot/generation-tagged selection record including the
full dispatch generation and final 0/1 activation side.

- [ ] **Step 5: Run causal hardware tests**

First extend `GpuPipelineFixture` with `run_frame(frame)`, which performs the
already-proven encode/microstep sequence followed by the new decode/select
passes and compact selection readback. This is the first task in which that
helper exists.

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_gpu_behavior -- --nocapture`

Expected: all five causal tests pass and print the real adapter identifier.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_gpu_backend/shaders/closed_loop_decode.wgsl crates/alife_gpu_backend/src/closed_loop_pipeline.rs crates/alife_gpu_backend/src/closed_loop_buffers.rs crates/alife_gpu_backend/src/lib.rs crates/alife_gpu_backend/tests/closed_loop_gpu_behavior.rs crates/alife_gpu_backend/tests/support/mod.rs
git commit -m "Decode and select action candidates on GPU"
```

### Task 7: Expose a shared required-GPU closed-loop backend with no fallback

**Files:**
- Create: `crates/alife_gpu_backend/src/closed_loop_runtime.rs`
- Create: `crates/alife_gpu_backend/tests/closed_loop_runtime.rs`
- Modify: `crates/alife_gpu_backend/tests/support/mod.rs`
- Modify: `crates/alife_gpu_backend/src/runtime.rs:18-267`
- Modify: `crates/alife_gpu_backend/src/lib.rs`

**Interfaces:**
- Consumes: `BrainPhenotype`, `PerceptionFrame`, `GpuClassBucketBuffers`, `GpuClosedLoopPipelines`.
- Produces: `GpuClosedLoopBackend::new_required`, `insert_brain`,
  generation-checked `remove_brain`, `tick_batch`, opaque `GpuBrainHandle`,
  `hardware_receipt`, and `GpuClosedLoopTick`.

- [ ] **Step 1: Write failing required-GPU tests**

Put the unavailable-factory test in `#[cfg(test)] mod tests` inside
`closed_loop_runtime.rs` so the factory seam stays crate-private. Put the
source-boundary test in `tests/closed_loop_runtime.rs`.

```rust
#[test]
fn unavailable_gpu_returns_typed_error_instead_of_cpu_fallback() {
    let result = GpuClosedLoopBackend::new_with_factory(&UnavailableGpuFactory);
    assert!(matches!(
        result,
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    ));
}

#[test]
fn product_runtime_has_no_cpu_execution_variant() {
    let source = include_str!("../src/closed_loop_runtime.rs");
    for forbidden in [
        concat!("Cpu", "Reference"),
        concat!("cpu_", "shadow"),
        concat!("AutoWithCpu", "Fallback"),
        concat!("FullGpuRuntime", "Mode"),
    ] {
        assert!(!source.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn pipeline_layout_mismatch_is_rejected_before_dispatch() {
    let mut backend = required_test_backend().unwrap();
    let result = backend.insert_brain(OrganismId(1), phenotype_with_wrong_gpu_layout_version());
    assert!(matches!(result, Err(ScaffoldContractError::GpuLayoutMismatch)));
    assert_eq!(backend.completed_dispatch_count(), 0);
}

#[test]
fn device_loss_stops_learned_actions_without_switching_policy() {
    let mut fixture = required_backend_with_one_brain().unwrap();
    fixture.force_device_lost_after_next_submit();
    let result = fixture.backend.tick_batch(&[(fixture.handle, test_frame())]);
    assert!(matches!(result, Err(ScaffoldContractError::NeuralBackendUnavailable)));
    assert!(matches!(fixture.backend.state(), GpuBackendState::DeviceLost { .. }));
    assert_eq!(fixture.backend.completed_selection_count(), 0);
}

#[test]
fn removed_handle_is_stale_after_generation_checked_slot_reuse() {
    let mut backend = required_test_backend().unwrap();
    let first = backend.insert_brain(OrganismId(1), causal_n512_phenotype()).unwrap();
    backend.remove_brain(first).unwrap();
    let second = backend.insert_brain(OrganismId(2), other_causal_n512_phenotype()).unwrap();
    assert_eq!(first.slot(), second.slot());
    assert_ne!(first.generation(), second.generation());
    assert!(backend.tick_batch(&[(first, test_frame())]).is_err());
}

#[test]
fn handle_from_another_backend_is_rejected_even_when_slot_tuple_matches() {
    let mut a = required_test_backend().unwrap();
    let mut b = required_test_backend().unwrap();
    let handle_a = a.insert_brain(OrganismId(1), causal_n512_phenotype()).unwrap();
    let handle_b = b.insert_brain(OrganismId(1), causal_n512_phenotype()).unwrap();
    assert_eq!((handle_a.class_id(), handle_a.slot(), handle_a.generation()),
               (handle_b.class_id(), handle_b.slot(), handle_b.generation()));
    assert!(b.tick_batch(&[(handle_a, test_frame())]).is_err());
}

#[test]
fn frame_profile_mismatch_is_rejected_before_upload_or_dispatch() {
    let mut backend = required_test_backend().unwrap();
    let phenotype = causal_n512_phenotype_for(SensorProfile::GroundedObjectSlotsV1);
    let handle = backend.insert_brain(OrganismId(1), phenotype).unwrap();
    let frame = test_frame_for(SensorProfile::PrivilegedAffordanceV1);
    let result = backend.tick_batch(&[(handle, frame)]);
    assert!(matches!(result, Err(ScaffoldContractError::SensorProfileMismatch)));
    assert_eq!(backend.completed_dispatch_count(), 0);
    assert_eq!(backend.perception_upload_count(), 0);
}

#[test]
fn brain_handle_is_bound_to_exactly_one_organism() {
    let mut backend = required_test_backend().unwrap();
    let phenotype = causal_n512_phenotype();
    let handle = backend.insert_brain(OrganismId(1), phenotype.clone()).unwrap();
    assert_eq!(handle.organism_id(), OrganismId(1));
    assert!(backend.insert_brain(OrganismId(1), phenotype).is_err());
    let wrong_frame = test_frame_for_organism(OrganismId(2));
    assert!(matches!(
        backend.tick_batch(&[(handle, wrong_frame)]),
        Err(ScaffoldContractError::BrainOwnershipMismatch)
    ));
    assert_eq!(backend.perception_upload_count(), 0);
}
```

Also write a failing hardware test that inserts two heterogeneous N512
phenotypes with different phenotype hashes, encoder assignments, route spans,
and decoder layouts into one backend. Submit both handles in one `tick_batch`,
assert exactly one adapter/device/pipeline set, receive two
slot/generation-matched records, prove each selection matches its own compiled
metadata, and prove repeated ticks do not alias mutable state.

- [ ] **Step 2: Verify the service is absent**

Run: `cargo test -p alife_gpu_backend --test closed_loop_runtime`

Expected: unresolved `GpuClosedLoopBackend` and `GpuBrainHandle`.

- [ ] **Step 3: Implement the service API**

```rust
pub struct GpuClosedLoopBackend {
    backend_instance_id: NonZeroU64,
    hardware: GpuHardwareReceipt,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: GpuClosedLoopPipelines,
    class_buckets: BTreeMap<u16, GpuClassBucketBuffers>,
    state: GpuBackendState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuHardwareReceipt {
    pub schema_version: u16,
    pub generation: u64,
    pub backend_api: String,
    pub adapter_name: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub driver_digest: [u64; 4],
    pub feature_digest: [u64; 4],
    pub limits_digest: [u64; 4],
    pub gpu_layout_version: u16,
    pub backend_version: String,
}

pub enum GpuBackendState {
    Ready,
    DeviceLost { last_checkpoint_digest: Option<[u64; 4]> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpuBrainHandle {
    backend_instance_id: NonZeroU64,
    class_id: BrainClassId,
    slot: u32,
    generation: u32,
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
}

impl GpuBrainHandle {
    pub const fn class_id(self) -> BrainClassId;
    pub const fn slot(self) -> u32;
    pub const fn generation(self) -> u32;
    pub const fn organism_id(self) -> OrganismId;
    pub const fn phenotype_hash(self) -> PhenotypeHash;
}

impl GpuClosedLoopBackend {
    pub fn new_required() -> Result<Self, ScaffoldContractError>;
    pub fn insert_brain(&mut self, organism_id: OrganismId, phenotype: BrainPhenotype) -> Result<GpuBrainHandle, ScaffoldContractError>;
    pub fn remove_brain(&mut self, handle: GpuBrainHandle) -> Result<(), ScaffoldContractError>;
    pub fn tick_batch(&mut self, batch: &[(GpuBrainHandle, PerceptionFrame)]) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError>;
    pub fn hardware_receipt(&self) -> &GpuHardwareReceipt;
    pub fn state(&self) -> &GpuBackendState;
    pub fn completed_dispatch_count(&self) -> u64;
    pub fn completed_selection_count(&self) -> u64;
}

pub struct GpuClosedLoopTick {
    pub handle: GpuBrainHandle,
    pub dispatch_generation: u64,
    pub base_digest: PerceptionBaseDigest,
    pub frame_digest: PerceptionFrameDigest,
    pub active_activation_side: u8,
    pub selection: NeuralActionSelection,
    pub compact_readback_bytes: usize,
    pub hardware_receipt_generation: u64,
}
```

Add this GPU-only test façade in `tests/support/mod.rs` after the production
backend API exists:

```rust
pub struct GpuTestBrain {
    pub backend: GpuClosedLoopBackend,
    pub handle: GpuBrainHandle,
}

impl GpuTestBrain {
    pub fn from_phenotype(organism_id: OrganismId, phenotype: BrainPhenotype) -> Result<Self, ScaffoldContractError>;
    pub fn tick(&mut self, frame: &PerceptionFrame) -> Result<GpuClosedLoopTick, ScaffoldContractError>;
}
```

Slice B extends the façade with learning/sleep delegates. It never computes a
CPU neural result.

`GpuHardwareReceipt` (including adapter/backend strings and limits) is owned
once by the shared backend and referenced by batch/acceptance telemetry. A
per-brain `GpuClosedLoopTick` carries only its small
`hardware_receipt_generation`; it never clones adapter strings or a full
hardware record per organism tick.
Receipt strings are length-bounded UTF-8, backend API is a stable validated
slug (the local evidence path requires `vulkan`), and feature/limit digests are
computed from the exact enabled/requested device contract. Generation is
process-local nonzero and every tick's small generation must match this shared
receipt.
Class buckets use `BrainClassId::raw()` as the `BTreeMap<u16, _>` key; they do
not assume the domain newtype implements `Ord`.

`new_required` must use the production `WgpuDeviceFactory` and return the typed error for adapter request failure, device request failure, or missing limits. `insert_brain` rejects an invalid or already-resident organism ID, unsupported capacity, and bucket exhaustion. A crate-private `GpuDeviceFactory` seam and `UnavailableGpuFactory` test double exercise the unavailable path without mutating process-wide environment variables. The runtime must not contain a CPU execution enum variant.

Validate shader ABI/layout versions against the phenotype before insertion.
Before allocating staging, writing perception uploads, incrementing dispatch
generation, or submitting any command, `tick_batch` performs a complete
all-or-nothing host preflight. Each occupied slot has one private immutable
`GpuBrainSlotOwnership { organism_id, phenotype_hash }` row created at
insertion. For every batch row, the backend resolves the private handle and
requires handle, ownership row, and `frame.organism_id()` to carry the same
organism plus `frame.sensor_profile() == slot.phenotype.sensor_profile()`. A single
profile mismatch rejects the entire batch with `SensorProfileMismatch` and
leaves every slot, counter, upload ring, and queue untouched. The shader never
interprets one profile's bytes under another profile's encoder plan.
An organism mismatch returns `BrainOwnershipMismatch` at the same preflight
boundary; ownership is never inferred from the first frame or credit packet.
Slice B initializes and restores its `LearningSequenceGuard` from this slot
ownership row and preflights every patch against it. Removal scrubs the
ownership row atomically with mutable slot state, and save restore calls the
same explicit organism-bound insertion/rebind API.
Every public backend method first compares the handle's private
`backend_instance_id` with the backend instance. IDs come from a process-local
monotonic nonzero allocator and are never serialized. `remove_brain` validates
class/slot/generation/hash, waits for that slot's last
submitted generation, scrubs mutable ranges, retires the generation, and only
then returns the slot to the bucket free list. Reuse increments the generation;
stale or forged handles are rejected by every backend entry point. Handle fields
are private so callers cannot construct a capability with a struct literal.
Any device-lost submission transitions the backend to `DeviceLost`, discards
the staged selection, returns `NeuralBackendUnavailable`, and rejects all later
ticks until explicit checkpoint-based recovery. Slice B extends recovery to
mutable learning state; no path resets state or changes policy implicitly.

Make the prewritten two-brain hardware test pass with one shared backend and
disjoint slot state.

- [ ] **Step 4: Add readback policy enforcement**

Assert each active slot reads exactly one `GpuSelectionRecord` (48 bytes),
validate slot/generation/dispatch identity before mapping it to a handle, and
reject activation, per-synapse, per-lobe, or weight readback requests outside
manual diagnostic boundaries.
Carry the validated full `dispatch_generation`, uploaded base/final frame
digests, and final active activation side into `GpuClosedLoopTick`; Task 9 uses those values when constructing
`NeuralDecisionEvidence`, and Slice B uses the same values for
pending-eligibility/credit matching and protected discard.

- [ ] **Step 5: Run runtime tests**

Run: `cargo test -p alife_gpu_backend --test closed_loop_runtime`

Expected: typed-unavailable and source-boundary tests pass.

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_runtime -- --nocapture`

Expected: required-GPU tick passes on the local adapter.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_gpu_backend/src/closed_loop_runtime.rs crates/alife_gpu_backend/src/runtime.rs crates/alife_gpu_backend/src/lib.rs crates/alife_gpu_backend/tests/closed_loop_runtime.rs crates/alife_gpu_backend/tests/support/mod.rs
git commit -m "Add required GPU closed-loop runtime"
```

### Task 8: Enumerate unscored candidates from one world snapshot

**Files:**
- Create: `crates/alife_world/src/candidate_enumerator.rs`
- Create: `crates/alife_world/tests/perception_candidates.rs`
- Modify: `crates/alife_world/src/lib.rs`
- Modify: `crates/alife_world/src/headless.rs:103-119,522-650,1681-1762`

**Interfaces:**
- Consumes: `HeadlessSensoryReport`, `VisibleWorldEntity`, organism body/homeostasis state.
- Produces: `CandidateEnumerator`, `HeadlessWorld::perception_frame_draft`, the
  Slice-A empty-context `perception_frame` convenience, and ordered
  `ActionCandidate` records.

- [ ] **Step 1: Write failing world-frame tests**

```rust
#[test]
fn perception_and_candidates_share_the_authoritative_tick() {
    let (world, organism) = fixture_world();
    let frame = world.perception_frame(
        organism, Tick::new(3), SensorProfile::PrivilegedAffordanceV1,
        HomeostaticSnapshot::baseline(Tick::new(3)),
    ).unwrap();
    assert_eq!(frame.tick(), frame.sensory().tick);
    assert_eq!(frame.candidates().iter().map(|c| c.candidate_index).collect::<Vec<_>>(), (0..frame.candidates().len() as u16).collect::<Vec<_>>());
    assert_eq!(frame.candidates()[0].kind, ActionKind::Idle);
}

#[test]
fn candidate_enumerator_emits_observations_not_scores() {
    let source = include_str!("../src/candidate_enumerator.rs");
    assert!(!source.contains("0.72"));
    assert!(!source.contains("food_score"));
    assert!(!source.contains("hazard_score"));
}

#[test]
fn semantic_relabelling_changes_features_not_candidate_availability() {
    let food = report_with_one_object(WorldObjectKind::Food);
    let hazard = report_with_one_object(WorldObjectKind::Hazard);
    let food_candidates = enumerate(food, SensorProfile::PrivilegedAffordanceV1);
    let hazard_candidates = enumerate(hazard, SensorProfile::PrivilegedAffordanceV1);
    assert_eq!(candidate_transport_signature(&food_candidates), candidate_transport_signature(&hazard_candidates));
    assert_ne!(food_candidates[1].features, hazard_candidates[1].features);
}
```

- [ ] **Step 2: Verify missing world API**

Run: `cargo test -p alife_world --test perception_candidates`

Expected: missing `perception_frame`.

- [ ] **Step 3: Implement enumeration**

Define:

```rust
pub trait CandidateEnumerator {
    fn enumerate_candidates(
        &self,
        report: &HeadlessSensoryReport,
        profile: SensorProfile,
    ) -> Result<Vec<ActionCandidate>, ScaffoldContractError>;
}
```

Always emit the explicit idle candidate first, at index zero. Stable-sort
visible objects by distance and entity ID, retain at most
`floor((MAX_ACTION_CANDIDATES - 1) / 5) == 6`, and emit the same five
mechanical families for every retained object:

```rust
[
    (ActionKind::Inspect.canonical_id(), ActionKind::Inspect, CandidateActionFamily::Inspect),
    (HeadlessActionIds::APPROACH, ActionKind::Move, CandidateActionFamily::Approach),
    (HeadlessActionIds::FLEE, ActionKind::Move, CandidateActionFamily::Avoid),
    (HeadlessActionIds::EAT, ActionKind::Interact, CandidateActionFamily::Ingest),
    (HeadlessActionIds::GRAB, ActionKind::Interact, CandidateActionFamily::Contact),
]
```

Semantic class changes may alter privileged feature bits, never candidate
availability or ordering. The world is allowed to reject an attempted Eat or
Grab and return the measured outcome. Populate distance, bearing, relative
velocity, evidence, and observed-affordance features without assigning
utility. With zero weights and biases, lowest-index tie breaking selects
documented idle behavior. Slice C replaces privileged class bits with grounded
object-slot features while preserving the same family set.

- [ ] **Step 4: Remove harness-forced proposal production**

Add `HeadlessWorld::perception_frame_draft` from one immutable world snapshot.
Implement `perception_frame` only as a Slice-A convenience that consumes that
draft with `PerceptionContextBlock::empty()`. Slice C replaces the convenience
in the live neural path with base-digest retrieval followed by one `finalize`;
it does not rebuild the base snapshot. Leave action execution APIs intact. Do
not call `CreatureMind::tick` from the new GPU path.

- [ ] **Step 5: Run world tests**

Run: `cargo test -p alife_world --test perception_candidates --test headless_world_harness`

Expected: all tests pass.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_world/src/candidate_enumerator.rs crates/alife_world/src/headless.rs crates/alife_world/src/lib.rs crates/alife_world/tests/perception_candidates.rs
git commit -m "Enumerate unscored world action candidates"
```

### Task 9: Cut the live app over to explicit GPU neural policy

**Files:**
- Create: `crates/alife_game_app/src/brain_policy.rs`
- Create: `crates/alife_game_app/tests/gpu_closed_loop_policy.rs`
- Modify: `crates/alife_game_app/src/app_shell.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Rewrite: `crates/alife_game_app/src/live_brain_bridge.rs`
- Rewrite product subset: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Modify: `crates/alife_game_app/src/graphical_playground.rs:12-58,181-248`
- Modify: `crates/alife_game_app/src/bin/alife_game_app.rs:1040-1360`
- Modify: `crates/alife_world/src/persistence.rs:281-310,570-670`
- Create: `crates/alife_world/src/legacy_neural_policy_v1.rs`
- Modify: `crates/alife_world/src/lib.rs`
- Modify: `crates/alife_world/tests/save_load_roundtrip.rs`
- Modify: `crates/alife_world/tests/fixtures/p34/tiny_config.json`
- Modify: `crates/alife_world/tests/fixtures/p34/tiny_save.json`

**Interfaces:**
- Consumes: Slice A's empty-context `HeadlessWorld::perception_frame`, shared
  `GpuClosedLoopBackend`, `GpuBrainHandle`, and `PolicyBackend` (Slice C later
  cuts this call to draft -> recall -> finalize).
- Produces: `BrainPolicyRuntime`, `GpuLiveBrainRuntime`, versioned `BrainPolicyConfig`, explicit `gpu-required` and `heuristic-baseline` launch modes.

- [ ] **Step 1: Write failing policy tests**

```rust
#[test]
fn graphical_default_is_gpu_required() {
    let launch = GraphicalPlaygroundLaunchConfig::interactive(fixture_root());
    assert_eq!(launch.brain_policy, PolicyBackend::NeuralClosedLoopGpu);
    assert!(launch.brain_policy.requires_gpu());
}

#[test]
fn neural_failure_does_not_select_heuristic_baseline() {
    let result = run_gpu_closed_loop_smoke_with_factory(
        test_launch(), &UnavailableGpuFactory,
    );
    assert!(matches!(result, Err(GameAppShellError::NeuralBackendUnavailable { .. })));
}

#[test]
fn organism_despawn_retires_its_gpu_handle_before_slot_reuse() {
    let mut runtime = live_runtime_with_two_organisms().unwrap();
    let retired = runtime.handle_for(OrganismId(1)).unwrap();
    runtime.world_mut().remove_organism(OrganismId(1)).unwrap();
    runtime.reconcile_population().unwrap();
    assert!(runtime.handle_for(OrganismId(1)).is_none());
    assert!(runtime.test_tick_retired_handle(retired, test_frame()).is_err());
}
```

In `save_load_roundtrip.rs`, write the failing `BrainPolicyConfig` round-trip
and legacy-migration tests now. Assert policy-derived `requires_gpu()` is true
only for neural mode, serialized vNext JSON has no `fallback_to_cpu`,
`gpu_feature_enabled`, or persisted `require_gpu` field, and
old `CpuReference`/GPU selections migrate to explicit policies without
performing a runtime switch.

- [ ] **Step 2: Run and verify old modes fail expectations**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_closed_loop_policy`

Expected: missing fields/types or old default assertion failure.

Run: `cargo test -p alife_world --test save_load_roundtrip`

Expected: compile failure for missing `BrainPolicyConfig` or failure on the old
fallback field.

- [ ] **Step 3: Implement explicit policy runtime**

```rust
pub enum BrainPolicyRuntime {
    #[cfg(feature = "gpu-runtime")]
    Neural(GpuLiveBrainRuntime),
    Heuristic(HeuristicBaselineLoop),
}

pub struct GpuLiveBrainRuntime {
    backend: GpuClosedLoopBackend,
    handles: BTreeMap<u64, GpuBrainHandle>,
}

impl BrainPolicyRuntime {
    pub fn tick(&mut self) -> Result<LiveBrainTickSummary, GameAppShellError>;
}
```

The runtime fields remain private. `handle_for`, `world_mut`, forced device
loss, and retired-handle tick helpers used above are crate-private
`#[cfg(test)]` seams, never product capability accessors. Product code can
observe only compact telemetry/receipts; it cannot borrow the backend, mutate
the handle map, or construct a handle.
The handle map is keyed by `OrganismId::raw()` in `BTreeMap<u64, _>` and every
lookup/convergence path performs that conversion at the boundary; it does not
require `OrganismId: Ord`.

`GpuLiveBrainRuntime::tick` gathers same-tick frames for scheduled organisms,
submits one class-grouped `tick_batch` per active capacity bucket, maps each
slot-matched selection to `ActionCommand`, executes it, observes the outcome,
and seals the patch. Do not call `current_context_proposals_with_scores`,
`tick_with_proposals`, or `CreatureMind::tick`.

Before gathering frames, `reconcile_population` deterministically compares the
world's live organism IDs with the handle map in ascending `raw()` order. It
inserts compiled births through
`backend.insert_brain(organism_id, phenotype)`, requires the returned handle's
organism accessor to match the map key, and
calls generation-checked `backend.remove_brain(handle)` for deaths/despawns
before deleting each map entry. A failed removal stops neural scheduling for
that organism; it never leaves a reusable untracked slot.

- [ ] **Step 4: Replace launch modes**

Replace `GraphicalGpuRuntimeMode` variants with:

```rust
pub enum GraphicalBrainPolicyMode {
    GpuRequired,
    HeuristicBaseline,
}
```

Accepted CLI labels are exactly `gpu-required` and `heuristic-baseline`. Production defaults to `gpu-required`; headless tests that do not request the `gpu-runtime` feature must explicitly use `heuristic-baseline`.

Replace the persistence-layer neural backend selection with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainPolicyConfig {
    pub schema_version: u16,
    pub policy: PolicyBackend,
}
```

`PolicyBackend::requires_gpu()` returns true exactly for
`NeuralClosedLoopGpu`; build-feature/adapter availability is current runtime
provenance and is never serialized as intent. Remove `fallback_to_cpu`,
`gpu_feature_enabled`, and persisted `require_gpu` from the current schema. A
versioned legacy deserializer may
map old `CpuReference` saves to `HeuristicBaseline` and old GPU selections to
`NeuralClosedLoopGpu`; this is a load migration, never a runtime policy switch.
Isolate those historical enum/string values in crate-private
`legacy_neural_policy_v1.rs`; no production policy module may import it.
Update the P34 fixtures and assert newly serialized configs contain no
`fallback_to_cpu` field.

- [ ] **Step 5: Run policy and live-loop tests**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_closed_loop_policy`

Expected: all policy tests pass.

Run: `cargo test -p alife_game_app --features gpu-runtime --lib live_brain`

Expected: GPU live-loop tests pass; historical heuristic tests are explicitly labelled baseline.

Run: `cargo test -p alife_world --test save_load_roundtrip`

Expected: new policy configs round-trip without a fallback field and legacy
fixtures migrate to an explicit policy.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_game_app/src/app_shell.rs crates/alife_game_app/src/brain_policy.rs crates/alife_game_app/src/live_brain_bridge.rs crates/alife_game_app/src/gpu_live_runtime.rs crates/alife_game_app/src/graphical_playground.rs crates/alife_game_app/src/bin/alife_game_app.rs crates/alife_game_app/src/lib.rs crates/alife_game_app/tests/gpu_closed_loop_policy.rs crates/alife_world/src/persistence.rs crates/alife_world/src/legacy_neural_policy_v1.rs crates/alife_world/src/lib.rs crates/alife_world/tests/save_load_roundtrip.rs crates/alife_world/tests/fixtures/p34/tiny_config.json crates/alife_world/tests/fixtures/p34/tiny_save.json
git commit -m "Make GPU closed-loop brain the live policy"
```

### Task 10: Remove CPU-shadow product code, tests, telemetry, and visual labels

**Files:**
- Delete or replace: `crates/alife_gpu_backend/src/full_runtime.rs`
- Delete: `crates/alife_gpu_backend/tests/static_forward_parity.rs`
- Keep temporarily as test-only reference: `crates/alife_gpu_backend/tests/plasticity_oja_parity.rs`; Slice B Task 4 retires it only after immediate three-factor GPU tests pass.
- Delete: `crates/alife_gpu_backend/tests/property_fuzz_parity_gating.rs`
- Modify: `crates/alife_gpu_backend/src/lib.rs`
- Modify: `crates/alife_gpu_backend/src/runtime.rs`
- Modify: `crates/alife_game_app/src/alpha_tick_stability.rs`
- Modify: `crates/alife_game_app/src/bevy_shell.rs`
- Modify: `crates/alife_game_app/src/bin/alife_game_app.rs`
- Modify: `crates/alife_game_app/src/camera_inspector.rs`
- Modify: `crates/alife_game_app/src/creature_animation_style.rs`
- Modify: `crates/alife_game_app/src/drive_coupled_audio_vfx.rs`
- Modify: `crates/alife_game_app/src/ecological_soak.rs`
- Modify: `crates/alife_game_app/src/gpu_graphics_performance.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Modify: `crates/alife_game_app/src/gpu_product_telemetry.rs`
- Modify: `crates/alife_game_app/src/graphical_ecology.rs`
- Modify: `crates/alife_game_app/src/graphical_playground.rs`
- Modify: `crates/alife_game_app/src/graphical_population.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Modify: `crates/alife_game_app/src/neural_activity_profiler.rs`
- Modify: `crates/alife_game_app/src/onboarding_tutorial.rs`
- Modify: `crates/alife_game_app/src/procedural_world_streaming.rs`
- Modify: `crates/alife_game_app/src/production_voxel_frontend.rs`
- Modify: `crates/alife_game_app/src/runtime_prereq_diagnostics.rs`
- Modify: `crates/alife_game_app/src/soak_isolation.rs`
- Modify: `crates/alife_game_app/src/tests.rs`
- Modify: `crates/alife_game_app/src/world_art_style.rs`
- Modify: `crates/alife_game_app/tests/app_shell.rs`
- Modify: `crates/alife_gpu_backend/src/timing.rs`
- Modify: `crates/alife_tools/src/benchmark.rs`
- Modify: `crates/alife_tools/src/bin/benchmark_tiers.rs`
- Modify: `crates/alife_tools/src/p35_playground.rs`
- Modify: `crates/alife_tools/tests/benchmark_tiers.rs`
- Modify: `crates/alife_tools/tests/playground_examples.rs`
- Modify: `crates/alife_world/src/persistence.rs`
- Modify: `crates/alife_world/tests/fixtures/gpu_alpha/tiny_config.json`
- Modify: `crates/alife_world/tests/fixtures/gpu_alpha/tiny_save.json`
- Modify: `crates/alife_world/tests/fixtures/production_voxel/tiny_config.json`
- Modify: `crates/alife_world/tests/fixtures/production_voxel/tiny_save.json`
- Modify: `scripts/package_windows_alpha.ps1`
- Modify: `scripts/run_windows_alpha_package.ps1`
- Modify: `scripts/run_production_voxel_frontend.ps1`
- Create: `crates/alife_game_app/tests/no_cpu_shadow_runtime.rs`
- Create visual blueprint before GUI edits: `docs/superpowers/assets/gpu-closed-loop-status-blueprint.png`

**Interfaces:**
- Consumes: new GPU closed-loop telemetry and explicit policy mode.
- Produces: product source with no CPU-shadow/fallback neural contract and a GPU-authoritative status surface.

- [ ] **Step 1: Use the image-generation skill for the required visual blueprint**

Generate a compact developer-overlay blueprint showing `GPU neural:
authoritative`, adapter name, phenotype hash prefix, active class, selected
candidate/logit, and `failure policy: stop learned actions`. Do not retain a
fallback status field. Keep the player view free of this overlay unless
developer mode is enabled.

- [ ] **Step 2: Add the failing source-boundary test**

```rust
#[test]
fn production_runtime_contains_no_cpu_shadow_or_neural_fallback_contract() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in ["src/gpu_live_runtime.rs", "src/live_brain_bridge.rs", "src/graphical_playground.rs"] {
        let source = std::fs::read_to_string(root.join(relative)).unwrap();
        assert!(!source.to_ascii_lowercase().contains("cpu_shadow"), "{relative}");
        assert!(!source.contains("AutoWithCpuFallback"), "{relative}");
        assert!(!source.contains("CpuReference"), "{relative}");
    }
}
```

- [ ] **Step 3: Run and confirm the boundary test fails**

Run: `cargo test -p alife_game_app --test no_cpu_shadow_runtime`

Expected: failure naming existing CPU-shadow source.

- [ ] **Step 4: Remove superseded backend modes and reports**

Delete `FullGpuRuntimeMode`, `FullGpuRuntimeProductClaim`, CPU execution variants, fallback reports, CPU timing fields, parity reports, and functions that turn action summaries back into heuristic scores. Preserve reusable no-readback and wgpu adapter helpers under neutral names.

- [ ] **Step 5: Migrate product telemetry and UI text**

Replace parity/fallback fields with:

```rust
pub struct GpuBrainAuthorityTelemetry {
    pub authoritative: bool,
    pub adapter: String,
    pub phenotype_hash_prefix: String,
    pub capacity_class: String,
    pub selected_candidate: Option<u16>,
    pub selected_logit: Option<f32>,
    pub compact_readback_bytes: usize,
    pub finite_rejections: u32,
}
```

Match the developer overlay implementation to the generated blueprint. Player
view remains visually unchanged apart from removal of CPU-shadow wording; Step
7 captures the real renderer after the launch script is migrated.

- [ ] **Step 6: Update scripts and fixtures**

Replace neural `-GpuMode auto-with-cpu-fallback` defaults with `-BrainPolicy
gpu-required`. Add a production-script `-DeveloperOverlay` switch whose default
is off; the minimum-settings capture opts in and the player capture omits it.
Keep renderer fallback flags separate when they refer to graphics rather than
neural cognition.

- [ ] **Step 7: Capture and compare fresh production screenshots**

Delete only the two named old artifacts, record the capture start time, and run
the real production renderer twice through the migrated script:

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
    $item = Get-Item -LiteralPath $shot
    if ($item.LastWriteTimeUtc -lt $captureStarted) { throw "stale screenshot: $shot" }
}
```

Inspect the developer shot with the image-view tool at high detail and compare
it to `docs/superpowers/assets/gpu-closed-loop-status-blueprint.png`. Require
every blueprint field, readable hierarchy, no CPU-shadow/fallback label, and no
clipping. Inspect the player shot and require the developer overlay to be
absent. Iterate and recapture until both checks pass; record both exact paths.

- [ ] **Step 8: Run the source scan and app tests**

```powershell
$rawMatches = & rg -n "cpu_shadow|CpuShadow|AutoWithCpuFallback|CpuReference|cpu-reference|neural_fallback|FullGpuRuntimeMode|parity.gat" crates/alife_gpu_backend/src crates/alife_game_app/src crates/alife_world/src scripts
$scanExit = $LASTEXITCODE
if ($scanExit -gt 1) { throw "source scan failed with exit $scanExit" }
$matches = @($rawMatches | Where-Object { $_ -notmatch 'crates[\\/]alife_world[\\/]src[\\/]legacy_neural_policy_v1.rs:' })
if ($matches.Count -ne 0) { $matches; throw "superseded neural authority surface remains" }
```

Expected: no production neural-runtime matches; allowed historical design documentation is outside these paths.

Run: `cargo test -p alife_game_app --features gpu-runtime --test no_cpu_shadow_runtime`

Expected: pass.

- [ ] **Step 9: Commit**

```powershell
git add -u crates/alife_gpu_backend crates/alife_game_app crates/alife_tools crates/alife_world scripts
git add crates/alife_game_app/tests/no_cpu_shadow_runtime.rs docs/superpowers/assets/gpu-closed-loop-status-blueprint.png
git diff --cached --name-only
git commit -m "Remove CPU shadow neural runtime"
```

### Task 11: Prove Slice A on real GPU hardware

**Files:**
- Create: `crates/alife_game_app/src/gpu_evidence.rs`
- Modify: `crates/alife_game_app/src/lib.rs`
- Create: `crates/alife_game_app/tests/gpu_closed_loop_acceptance.rs`
- Modify: `crates/alife_game_app/src/bin/alife_game_app.rs`
- Create artifacts at runtime only: `target/artifacts/gpu-closed-loop-slice-a-<class>.json`

**Interfaces:**
- Consumes: completed Slice A runtime.
- Produces: N512/N1024/N2048 causal receipts, Vulkan hardware receipt, deterministic replay receipt, no-fallback receipt.

Task 11 creates these shared A/B evidence types in `gpu_evidence.rs`; Slice B
reuses them without redefining its JSON header:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuSliceEvidenceHeader {
    pub artifact_schema: u16,
    pub slice_raw: u16,
    pub class_id_raw: u16,
    pub profile_id_raw: u16,
    pub profile_schema: u16,
    pub status_raw: u16,
    pub git_commit: String,
    pub source_tree_digest: String,
    pub artifact_digest: [u64; 4],
    pub phenotype_hash: PhenotypeHash,
    pub phenotype_manifest_digest: [u64; 4],
    pub capacity_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhenotypeEvidenceManifest {
    pub schema_version: u16,
    pub class_id_raw: u16,
    pub phenotype_sensor_profile_raw: u16,
    pub phenotype_hash: PhenotypeHash,
    pub compile_inputs_digest: [u64; 4],
    pub capacity_digest: [u64; 4],
    pub lobe_layout_digest: [u64; 4],
    pub projection_plan_digest: [u64; 4],
    pub synapse_payload_digest: [u64; 4],
    pub encoder_plan_digest: [u64; 4],
    pub decoder_plan_digest: [u64; 4],
    pub plasticity_plan_digest: [u64; 4],
    pub replay_capture_plan_digest: [u64; 4],
    pub manifest_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpuSameAdapterReplayEvidence {
    pub adapter_identity_digest: [u64; 4],
    pub initial_state_digest: [u64; 4],
    pub frame_sequence_digest: [u64; 4],
    pub selected_candidate_digest: [u64; 4],
    pub first_logit_digest: [u64; 4],
    pub second_logit_digest: [u64; 4],
    pub tolerance: f32,
    pub max_abs_error: f32,
    pub passed: bool,
}
```

Stable constants are Slice A = 1, Slice B = 2, passing status = 1. A/B are
profile-independent promotion bindings, so their header profile ID/schema are
both exactly zero even though each manifest records the actual phenotype sensor
profile used by its fixture. `class_id_raw`, phenotype hash, manifest digest,
and capacity digest must agree across header, embedded manifest, compiled
phenotype, and canonical `BrainCapacityClass` before writing.
Slice-specific receipt structs store `#[serde(flatten)] pub header:
GpuSliceEvidenceHeader`, so the JSON identity fields are top-level and D can
ingest the same exact shape from A and B.

All four-word evidence digests use named domain tags and four fixed SplitMix64
seeds over typed canonical little-endian fields and length-prefixed UTF-8; never
`DefaultHasher` or map iteration. `manifest_digest` excludes only itself.
`artifact_digest` covers the header except its own digest plus the complete
embedded manifest and slice-specific receipt body, including replay/hardware
evidence. JSON field order/whitespace is therefore irrelevant. Load recomputes
both digests, requires lowercase 40-hex Git commit/tree IDs, rejects unknown
schema/slice/status/profile values and non-finite floats, and verifies the
capacity digest with `BrainCapacityClass::canonical_digest()`.
For a component not yet present in Slice A (plasticity or replay capture), the
manifest uses the canonical domain digest of an explicit `None` tag, never an
ambiguous all-zero digest; Slice B replaces it with the real component digest.

- [ ] **Step 1: Add failing integration assertions**

```rust
#[test]
fn gpu_closed_loop_slice_a_receipt_is_authoritative() {
    let receipt = run_gpu_closed_loop_acceptance(test_options()).unwrap();
    assert_eq!(receipt.header.slice_raw, 1);
    assert_eq!(receipt.header.profile_id_raw, 0);
    assert_eq!(receipt.header.profile_schema, 0);
    assert_eq!(receipt.header.status_raw, 1);
    assert_eq!(receipt.header.class_id_raw, receipt.capacity_class_id.raw());
    assert_eq!(receipt.header.capacity_digest, receipt.capacity.canonical_digest());
    assert_eq!(receipt.header.phenotype_manifest_digest, receipt.phenotype_manifest.manifest_digest);
    assert_eq!(receipt.header.artifact_digest, receipt.recompute_artifact_digest().unwrap());
    assert_eq!(receipt.backend_api, "vulkan");
    assert!(receipt.authoritative);
    assert_eq!(receipt.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
    assert_eq!(receipt.neural_dispatch_count, receipt.requested_ticks);
    assert_eq!(receipt.gpu_selection_count, receipt.requested_ticks);
    assert!(receipt.compact_readback_bytes <= 64);
    assert!(receipt.active_synapses > 0);
    assert!(receipt.replay.passed);
    assert!(receipt.replay.max_abs_error <= receipt.replay.tolerance);
}
```

- [ ] **Step 2: Run and verify the acceptance API is missing**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_closed_loop_acceptance -j 1 -- --nocapture`

Expected: compile failure for the missing acceptance runner/receipt.

- [ ] **Step 3: Implement the acceptance subcommand**

Add `gpu-closed-loop-acceptance` with `--class n512|n1024|n2048`,
`--ticks`, `--seed`, `--sensor-profile privileged-affordance-v1`, and a required
`--output <path>`. It atomically writes exactly one class-qualified receipt and
must emit JSON containing adapter backend/name, phenotype hash, selected
logits/candidates, active tiles/synapses, readback bytes, `policy_backend`,
`neural_dispatch_count`, `gpu_selection_count`, schema version, clean Git commit,
the exact lowercase `capacity_class` slug, and the canonical source-tree digest
(Git tree object ID). Do not preserve
compatibility counters named after removed CPU-shadow or fallback modes.
Embed the exact shared header, canonical capacity record, and
`PhenotypeEvidenceManifest`. Run the same-adapter replay from one identical
initial checkpoint and frame sequence, record both logit digests, exact shared
candidate digest, declared tolerance/max error, and `replay.passed`; receipt
creation fails rather than writing status 1 when replay fails. Compute
manifest/artifact digests only after every body field is final, then serialize
atomically and read it back through the validating loader before success.
Add `gpu-evidence-validate --slice a|b --input <path>` as a read-only subcommand
over the same library loader; it recomputes every shared/body digest and exits
nonzero on any mismatch. Slice B extends the body validator in its acceptance
task.

- [ ] **Step 4: Run focused crate gates**

Run: `cargo fmt --all -- --check`

Expected: pass.

Run: `cargo test -p alife_core --all-targets`

Expected: pass.

Run: `cargo test -p alife_world --all-targets`

Expected: pass.

Run: `cargo test -p alife_gpu_backend --features gpu-tests --all-targets -- --nocapture`

Expected: pass on the real adapter.

Run: `cargo test -p alife_game_app --features gpu-runtime --all-targets -j 1 -- --nocapture`

Expected: pass.

Run: `cargo check --workspace --all-targets`

Expected: every caller migrated from the superseded runtime APIs compiles.

Run: `cargo test -p alife_tools --all-targets -j 1`

Expected: benchmark/playground tooling uses explicit policy names and passes.

Run: `cargo test --workspace --all-targets -j 1`

Expected: the default-feature workspace is green; hardware-feature evidence is
provided by the focused GPU commands above.

- [ ] **Step 5: Run architecture gates against the source to be committed**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
$matches = rg -n "cpu_shadow|AutoWithCpuFallback|CpuReference|neural_fallback|FullGpuRuntimeMode|parity.gat" crates/alife_gpu_backend/src crates/alife_game_app/src crates/alife_tools/src
if ($LASTEXITCODE -eq 0) { $matches; throw "superseded neural authority surface remains" }
if ($LASTEXITCODE -ne 1) { throw "authority source scan failed" }
git diff --check
```

Expected: all commands exit 0.

- [ ] **Step 6: Commit Slice A acceptance**

```powershell
git add crates/alife_game_app/src/gpu_evidence.rs crates/alife_game_app/src/lib.rs crates/alife_game_app/src/bin/alife_game_app.rs crates/alife_game_app/tests/gpu_closed_loop_acceptance.rs
git commit -m "Prove GPU closed-loop causal behavior"
```

- [ ] **Step 7: Require a clean committed evidence source**

```powershell
if (git status --short) { throw "Slice A evidence requires a clean worktree" }
$evidenceCommit = git rev-parse HEAD
$evidenceTree = git rev-parse 'HEAD^{tree}'
```

Expected: both IDs resolve from the commit containing the acceptance runner.

- [ ] **Step 8: Run all three real hardware receipts from that commit**

```powershell
foreach ($class in @('n512', 'n1024', 'n2048')) {
    $output = "target/artifacts/gpu-closed-loop-slice-a-$class.json"
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-closed-loop-acceptance --class $class --ticks 64 --seed 4101 --sensor-profile privileged-affordance-v1 --output $output
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
```

Expected: each reports Vulkan, GPU authority, the exact clean commit/tree,
nonempty phenotype, active synapses, and compact readback only.

- [ ] **Step 9: Validate artifact provenance and class separation**

```powershell
foreach ($class in @('n512', 'n1024', 'n2048')) {
    $path = "target/artifacts/gpu-closed-loop-slice-a-$class.json"
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-evidence-validate --slice a --input $path
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $receipt = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    if ($receipt.git_commit -ne $evidenceCommit) { throw "stale commit: $path" }
    if ($receipt.source_tree_digest -ne $evidenceTree) { throw "stale tree: $path" }
    if ($receipt.capacity_class -ne $class) { throw "wrong class: $path" }
    if (-not $receipt.authoritative) { throw "non-authoritative receipt: $path" }
    if (-not $receipt.replay.passed) { throw "same-adapter replay failed: $path" }
}
```

- [ ] **Step 10: Verify the committed Slice A diff remains clean**

```powershell
git diff --check origin/main...HEAD
if (git status --short) { throw "tracked worktree changed after evidence" }
```

Expected: the committed range has no whitespace errors and status is clean.

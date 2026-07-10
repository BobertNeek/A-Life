# GPU Closed-Loop Memory and Grounding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace prefix-truncated, candidate-invariant recall with profile-provenanced grounded perception and candidate-conditioned episodic context that influences the authoritative GPU brain through bounded neural channels while memory and topology saturation remain nonfatal.

**Architecture:** `alife_world` emits either privileged affordances or generic grounded object slots from one authoritative snapshot and assigns tick-local targets stable tracked-object bindings. `alife_core` owns versioned state-action-target query and sidecar contracts, deterministic memory and topology degradation policies, and provenance; `alife_gpu_backend` consumes per-candidate memory latents/values through explicit bounded WGSL input and decoder projections. The app performs retrieval before the one GPU neural dispatch, observes sealed outcomes afterward, and never lets sidecar pressure select a CPU policy or abort a valid brain tick.

**Tech Stack:** Rust 2021, wgpu 29.0.3, WGSL, serde, bytemuck, existing Slice A perception/phenotype contracts, existing Slice B GPU learning/sleep contracts, real Vulkan hardware tests, PowerShell validation on Windows.

## Global Constraints

- Slice A and Slice B must be complete and their real-GPU acceptance receipts must pass before this plan starts.
- Production action selection remains `NeuralClosedLoopGpu`: no live CPU neural shadow, CPU parity gate, CPU scoring path, or silent CPU neural fallback.
- CPU-side episodic and topology structures may only produce bounded context/diagnostic records; learned neural state, candidate logits, and winner selection remain GPU-authoritative.
- Initial production neural capacity classes are N512, N1024, and N2048; every Slice C acceptance command names one of these classes.
- `PrivilegedAffordanceV1` and `GroundedObjectSlotsV1` are distinct, versioned profiles, and every frame, patch, save, benchmark, and behavioral receipt records profile provenance.
- Grounded uploads contain bearing, distance, relative velocity, color, material, shape, chemical, contact, proprioception, temperature, and terrain features, but no food, hazard, teacher, or object-class truth channel.
- Raw `WorldEntityId` values are command-transport data only; they never enter candidate feature vectors, memory query vectors, or persistent topology bindings.
- Memory influence is per candidate and passes through explicit bounded latent/value channels; no scalar is added uniformly to all candidates.
- Recall queries bind a `PerceptionBaseDigest` computed before episodic context exists. The authoritative GPU upload, selection receipt, and sealed patch bind the distinct final `PerceptionFrameDigest` computed after the exact context rows have been installed.
- Candidate memory never enters recurrent state in Slice C. Each candidate receives its own target-local latent and exact-family value vector only inside the GPU candidate decoder.
- Memory and topology capacities use deterministic merge, eviction, compaction, or summary replacement. Saturation is a reported degradation condition, never a terminal brain-tick error.
- Tracked-object, memory, compaction, and topology state is owned by portable `OrganismId`, never by a transient `GpuBrainHandle`, slot, adapter, or process-local job ID.
- Slice C reuses Slice B's exact 80-byte `GpuBrainSlotExtensionRecord`; it populates the reserved memory offsets and extends decoder metadata without creating a second slot-extension ABI.
- Active-loop readback remains one selected-action record plus bounded counters. Bulk neural, memory, or weight readback is limited to save, sleep, manual diagnostic, or acceptance boundaries.
- Production shaders are WGSL only, and `alife_core`/`alife_world` remain free of wgpu, Bevy, renderer, and OS-handle types.
- Use `cargo test -j 1` for Bevy-heavy all-feature gates on this Windows host.
- Do not modify or merge the unrelated FVR11 worktree.

## Planned file structure

### Core contracts and bounded sidecars

- Create `crates/alife_core/src/grounding.rs`: profile provenance, tracked-object ID, grounded object-slot layout, and exact 24-feature encoding.
- Create `crates/alife_core/src/memory_query.rs`: versioned 96-element state-action-target query layout and per-candidate retrieval context.
- Modify `crates/alife_core/src/memory.rs`: candidate-conditioned records,
  exact bucket keys/receipts, matching, merge/eviction, and crash-safe sleep
  compaction checkpoint contracts.
- Modify `crates/alife_core/src/topology.rs`: organism-owned `TopologySidecar`, tracked-object bindings, private mutation planning, and bounded replacement primitives.
- Modify `crates/alife_core/src/perception.rs`, `experience.rs`, `ids.rs`, `version.rs`, and `lib.rs`: carry profile, query, and retrieval evidence across boundaries.

### World extraction and persistence

- Create `crates/alife_world/src/grounded_sensing.rs`: semantic-free physical observation extraction.
- Create `crates/alife_world/src/tracked_objects.rs`: deterministic organism-local tracked-object registry.
- Modify `crates/alife_world/src/candidate_enumerator.rs`: generic inspect/approach/avoid/contact/ingest families for every grounded slot.
- Modify `crates/alife_world/src/headless.rs`: physical object properties, profile-aware perception, and poisoned-ingest outcomes.
- Modify `crates/alife_world/src/persistence.rs`: profile, memory, topology, and tracked-object summaries without persistent raw entity bindings.

### GPU and app integration

- Create `crates/alife_gpu_backend/src/closed_loop_memory.rs`: memory-context upload ABI and dispatch integration.
- Create `crates/alife_gpu_backend/shaders/closed_loop_memory_context.wgsl`: bounded candidate-local target-latent and family-value decoder projections.
- Modify `crates/alife_gpu_backend/src/closed_loop_buffers.rs`, `closed_loop_pipeline.rs`, `closed_loop_runtime.rs`, `closed_loop_sleep.rs`, and `lib.rs`.
- Modify `crates/alife_game_app/src/live_brain_bridge.rs` and `gpu_live_runtime.rs`: retrieve, dispatch once on GPU, seal, learn, then update sidecars.
- Modify `crates/alife_game_app/src/bin/alife_game_app.rs`: Slice C acceptance runner and profile-labelled receipts.

---

### Task 1: Supersede scalar-memory and terminal-topology architecture decisions

**Files:**
- Modify: `docs/architecture_decisions.md`
- Modify: `docs/master_spec.md:416-490,756-781`
- Test: `scripts/docs_check.ps1`

**Interfaces:**
- Consumes: approved design `docs/superpowers/specs/2026-07-09-gpu-closed-loop-brain-design.md` and ADR-024 from Slice A.
- Produces: ADR-025, which supersedes the action-path portions of ADR-012 and ADR-013 without weakening sealed-patch or GPU-authority rules.

- [ ] **Step 1: Confirm the new decision is absent**

Run: `rg -n "ADR-025: Candidate-Conditional Memory and Grounded Profiles" docs/architecture_decisions.md`

Expected: exit 1 with no match.

- [ ] **Step 2: Append the controlling decision**

Append this text verbatim, adjusting only line wrapping:

```markdown
## ADR-025: Candidate-Conditional Memory and Grounded Profiles

Decision: Episodic recall uses a versioned state-action-target query and returns
bounded per-candidate latent/value context. Production consumes that context
only through explicit GPU candidate-decoder channels; no memory context is
pooled into recurrent/global inputs, and no candidate-invariant memory or
topology scalar is added to action scores. Queries bind the pre-context
perception base digest; GPU dispatch and sealing bind the separately computed
final frame digest after consume-once context finalization.

`PrivilegedAffordanceV1` and `GroundedObjectSlotsV1` are separately provenanced.
Grounded slots contain physical observations and no semantic class labels.
Topology is a bounded diagnostic sidecar over sealed patches. Memory and
topology saturation deterministically merge, evict, compact, or summarize and
cannot abort a valid neural tick. Persistent bindings use tracked-object or
episodic IDs rather than raw world entity IDs. Tracker, memory, compaction, and
topology owners are portable organism IDs rather than GPU handles.
```

- [ ] **Step 3: Run documentation validation**

Before validation, update master-spec sections 17, 20, 29, 33, and 36 to
replace candidate-invariant `MemoryExpectancy` action bias and terminal bounded
topology behavior with ADR-025's state-action-target retrieval, explicit
profile provenance, and nonfatal diagnostic sidecar rules. Preserve the sealed
patch and GPU-authority boundaries established by ADR-024.

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`

Expected: exit 0 and a printed Git Bash path.

- [ ] **Step 4: Commit**

```powershell
git add docs/architecture_decisions.md docs/master_spec.md
git commit -m "Document grounded candidate memory contracts"
```

### Task 2: Define profile provenance and exact grounded object-slot contracts

**Files:**
- Create: `crates/alife_core/src/grounding.rs`
- Create: `crates/alife_core/tests/sensory_profiles.rs`
- Modify: `crates/alife_core/src/ids.rs`
- Modify: `crates/alife_core/src/perception.rs`
- Modify: `crates/alife_core/src/version.rs`
- Modify: `crates/alife_core/src/lib.rs`

**Interfaces:**
- Consumes: Slice A `SensorProfile`, `PerceptionFrameDraft`,
  `CandidateFeatureVector`, `CANDIDATE_FEATURE_COUNT`, `SensoryAbiVersion`,
  `Tick`, `Confidence`, and `OrganismId::raw`.
- Produces: `TrackedObjectId`, `SensorProfileId`, `SensorProfileIdentity`, `SensorProfileProvenance`, `GroundedObjectSlotV1`, `MAX_GROUNDED_OBJECT_SLOTS`, and `GroundedObjectSlotV1::candidate_features`.

- [ ] **Step 1: Write failing profile and slot-layout tests**

```rust
use alife_core::{
    Confidence, GroundedObjectSlotV1, SensorProfile, SensorProfileProvenance,
    SensorProfileId, SensoryAbiVersion, Tick, TrackedObjectId,
};

#[test]
fn grounded_slot_maps_every_named_group_into_exactly_twenty_four_features() {
    let slot = GroundedObjectSlotV1 {
        slot_index: 0,
        tracked_object_id: TrackedObjectId(9),
        bearing: [-0.75, 0.25],
        distance: 0.5,
        relative_velocity: [-0.3, 0.2, 0.1],
        color: [0.1, 0.2, 0.3],
        material: [0.4, 0.5, 0.6],
        shape: [0.7, 0.8, 0.9],
        chemical: [-0.9, 0.25, 0.75],
        contact: 1.0,
        proprioception: [0.33, -0.2],
        temperature: -0.4,
        terrain: [0.6, 0.2],
        confidence: Confidence::new(0.8).unwrap(),
    };
    assert_eq!(
        slot.candidate_features().unwrap().0,
        [
            -0.75, 0.25, 0.5, -0.3, 0.2, 0.1, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6,
            0.7, 0.8, 0.9, -0.9, 0.25, 0.75, 1.0, 0.33, -0.2, -0.4, 0.6, 0.2,
        ]
    );
}

#[test]
fn provenance_names_profile_version_and_sensory_abi() {
    let provenance = SensorProfileProvenance::new(
        SensorProfile::GroundedObjectSlotsV1,
        SensoryAbiVersion::CURRENT,
        Tick::new(17),
    ).unwrap();
    assert_eq!(provenance.schema_version, 1);
    assert_eq!(provenance.profile, SensorProfile::GroundedObjectSlotsV1);
    assert_eq!(provenance.source_tick, Tick::new(17));
}

#[test]
fn slot_and_provenance_validation_reject_invalid_boundaries() {
    assert!(slot_fixture(TrackedObjectId(0), 0, [0.0; 3]).validate_contract().is_err());
    assert!(slot_fixture(TrackedObjectId(1), 16, [0.0; 3]).validate_contract().is_err());
    assert!(slot_fixture(TrackedObjectId(1), 0, [-1.01, 0.0, 0.0]).validate_contract().is_err());
    assert!(slot_fixture(TrackedObjectId(1), 0, [1.01, 0.0, 0.0]).validate_contract().is_err());
    assert!(mismatched_profile_tick_or_abi_fixture().validate_contract().is_err());
}

#[test]
fn profile_ids_are_stable_and_unknown_values_are_rejected() {
    assert_eq!(SensorProfileId::from(SensorProfile::PrivilegedAffordanceV1).raw(), 1);
    assert_eq!(SensorProfileId::from(SensorProfile::GroundedObjectSlotsV1).raw(), 2);
    assert_eq!(SensorProfile::try_from(SensorProfileId(2)).unwrap(), SensorProfile::GroundedObjectSlotsV1);
    assert!(SensorProfile::try_from(SensorProfileId(99)).is_err());
    assert_eq!(OrganismId(42).raw(), 42);
}
```

Add failing `PerceptionFrameDraft` cases for a dangling object-slot index, slot/
candidate feature mismatch, grounded non-idle `None`, grounded idle with a
slot, and privileged candidate with a slot.

- [ ] **Step 2: Run and verify the contracts are absent**

Run: `cargo test -p alife_core --test sensory_profiles`

Expected: compile failure for unresolved `GroundedObjectSlotV1`, `SensorProfileProvenance`, and `TrackedObjectId`.

- [ ] **Step 3: Implement the exact core records**

```rust
pub const MAX_GROUNDED_OBJECT_SLOTS: usize = 16;
pub const GROUNDED_OBJECT_SLOT_SCHEMA_VERSION: u16 = 1;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SensorProfileId(pub u16);

impl SensorProfileId {
    pub const fn raw(self) -> u16 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SensorProfileIdentity {
    pub profile_id: SensorProfileId,
    pub profile_schema_version: u16,
    pub sensory_abi_version: u16,
}

impl SensorProfileIdentity {
    pub fn profile(self) -> Result<SensorProfile, ScaffoldContractError>;
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TrackedObjectId(pub u64);

impl TrackedObjectId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorProfileProvenance {
    pub schema_version: u16,
    pub profile: SensorProfile,
    pub sensory_abi_version: SensoryAbiVersion,
    pub source_tick: Tick,
}

impl SensorProfileProvenance {
    pub fn new(
        profile: SensorProfile,
        sensory_abi_version: SensoryAbiVersion,
        source_tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        let provenance = Self {
            schema_version: GROUNDED_OBJECT_SLOT_SCHEMA_VERSION,
            profile,
            sensory_abi_version,
            source_tick,
        };
        provenance.validate_contract()?;
        Ok(provenance)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GroundedObjectSlotV1 {
    pub slot_index: u16,
    pub tracked_object_id: TrackedObjectId,
    pub bearing: [f32; 2],
    pub distance: f32,
    pub relative_velocity: [f32; 3],
    pub color: [f32; 3],
    pub material: [f32; 3],
    pub shape: [f32; 3],
    pub chemical: [f32; 3],
    pub contact: f32,
    pub proprioception: [f32; 2],
    pub temperature: f32,
    pub terrain: [f32; 2],
    pub confidence: Confidence,
}

impl GroundedObjectSlotV1 {
    pub fn candidate_features(self) -> Result<CandidateFeatureVector, ScaffoldContractError> {
        let features = CandidateFeatureVector([
            self.bearing[0], self.bearing[1], self.distance,
            self.relative_velocity[0], self.relative_velocity[1], self.relative_velocity[2],
            self.color[0], self.color[1], self.color[2],
            self.material[0], self.material[1], self.material[2],
            self.shape[0], self.shape[1], self.shape[2],
            self.chemical[0], self.chemical[1], self.chemical[2],
            self.contact, self.proprioception[0], self.proprioception[1],
            self.temperature, self.terrain[0], self.terrain[1],
        ]);
        features.validate()?;
        Ok(features)
    }
}
```

Add `SensorProfileProvenance::identity() -> SensorProfileIdentity`. Memory
matching, save compatibility, benchmark aggregation, and report buckets use
the stable identity and never include `source_tick`; the source tick remains
transaction evidence only.
The identity stores only stable primitive IDs so its ordering is portable:
`profile_id` uses the explicit 1/2 map and `sensory_abi_version` stores
`SensoryAbiVersion::raw()`. Do not derive ordering from Rust enum declaration
order.
Use Slice A's `OrganismId::raw() -> u64` accessor in ordered indices and
persistence code instead of deriving `Ord` from a domain wrapper or accessing
tuple fields directly.
Implement explicit mappings 1→PrivilegedAffordanceV1 and
2→GroundedObjectSlotsV1 by delegating to Slice A
`SensorProfile::{raw, try_from_raw}`; reject every other ID. `SensorProfileId`
is the serialized newtype wrapper around that exact raw value, not a second
mapping. GPU, packed-log, and artifact code must call these APIs and must not
cast the Rust enum discriminant.

`GroundedObjectSlotV1::validate_contract` must require a nonzero tracked ID,
slot index below 16, finite values, bearing/velocity/chemical gradients/
temperature/proprioception in `[-1, 1]`, color/material/shape/contact/terrain in
`[0, 1]`, and valid confidence. Add `SchemaKind::SensorProfile` at version 1.

- [ ] **Step 4: Extend immutable `PerceptionFrameDraft` base evidence**

Add `profile_provenance: SensorProfileProvenance` and
`grounded_object_slots: Vec<GroundedObjectSlotV1>` to Slice A's private-field
`PerceptionFrameDraft`. Its constructor validation must require matching
profile/tick/ABI, contiguous unique slot indices, at most 16 slots, an empty
grounded-slot vector for `PrivilegedAffordanceV1`, and no privileged visual-
affordance or food/hazard/teacher bits for `GroundedObjectSlotsV1`.

Use this final constructor/accessor shape and migrate all Slice A callers in
the same commit (privileged callers pass a validated same-tick provenance and
an empty slot vector):

```rust
impl PerceptionFrameDraft {
    pub fn new(
        organism_id: OrganismId,
        tick: Tick,
        sensor_profile: SensorProfile,
        sensory: SensorySnapshot,
        body: BodySnapshot,
        homeostasis: HomeostaticSnapshot,
        candidates: Vec<ActionCandidate>,
        profile_provenance: SensorProfileProvenance,
        grounded_object_slots: Vec<GroundedObjectSlotV1>,
    ) -> Result<Self, ScaffoldContractError>;
    pub const fn organism_id(&self) -> OrganismId;
    pub const fn tick(&self) -> Tick;
    pub const fn sensor_profile(&self) -> SensorProfile;
    pub fn sensory(&self) -> &SensorySnapshot;
    pub const fn body(&self) -> BodySnapshot;
    pub fn homeostasis(&self) -> &HomeostaticSnapshot;
    pub fn candidates(&self) -> &[ActionCandidate];
    pub const fn profile_provenance(&self) -> SensorProfileProvenance;
    pub fn grounded_object_slots(&self) -> &[GroundedObjectSlotV1];
    pub const fn base_digest(&self) -> PerceptionBaseDigest;
}
```
Every grounded non-idle candidate must carry an in-range
`CandidateObservationRef::ObjectSlot(index)` whose slot features exactly match
the candidate feature vector; grounded idle and every privileged candidate use
`None`. Reject dangling, mismatched, or cross-profile references before memory
encoding. Include profile provenance and the complete ordered grounded-slot
records in Slice A's canonical base-digest bytes. The finalized
`PerceptionFrame` continues to own the draft as its
immutable base; Slice C adds no setter or mutating context attachment API.
Add read-only `PerceptionFrameDraft::{organism_id, tick, sensor_profile,
sensory, body, homeostasis, candidates, profile_provenance,
grounded_object_slots}` accessors, and matching delegated accessors on
`PerceptionFrame`; fields and digest construction remain private.

- [ ] **Step 5: Run focused and core tests**

Run: `cargo test -p alife_core --test sensory_profiles --test perception_candidates`

Expected: all profile, slot-layout, and Slice A perception tests pass.

Run: `cargo test -p alife_core --all-targets`

Expected: pass.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_core/src/grounding.rs crates/alife_core/src/ids.rs crates/alife_core/src/perception.rs crates/alife_core/src/version.rs crates/alife_core/src/lib.rs crates/alife_core/tests/sensory_profiles.rs
git commit -m "Add grounded sensor profile contracts"
```

### Task 3: Extract grounded slots and generic candidates from one world snapshot

**Files:**
- Create: `crates/alife_world/src/grounded_sensing.rs`
- Create: `crates/alife_world/src/tracked_objects.rs`
- Create: `crates/alife_world/tests/grounded_object_slots.rs`
- Modify: `crates/alife_world/src/candidate_enumerator.rs`
- Modify: `crates/alife_world/src/headless.rs`
- Modify: `crates/alife_world/src/persistence.rs`
- Modify: `crates/alife_world/src/lib.rs`
- Modify: `crates/alife_world/tests/save_load_roundtrip.rs`

**Interfaces:**
- Consumes: semantic-free `PhysicalObservationSnapshot`, observer pose/velocity, Slice A `CandidateEnumerator`, and Task 2 grounded contracts.
- Produces: `GroundedPhysicalProperties`, `PhysicalTrackingProvenance`,
  canonical `PhysicalTrackingKey`, `PhysicalObservedObject`,
  `PhysicalObservationSnapshot`, `StablePhysicalDescriptor`,
  `TrackedObjectRecord`, `TrackedObjectObservationReceipt`, bounded
  `TrackedObjectRegistry::observe`,
  `GroundedSensorExtractor::extract`, `HeadlessWorld::perception_frame_draft`,
  and generic candidate families for bounded visible slots.

- [ ] **Step 1: Write failing grounded-world tests**

```rust
#[test]
fn grounded_profile_exposes_physics_not_world_object_kind() {
    let (mut world, organism, food_id, hazard_id) = two_physical_object_fixture();
    let frame = world.perception_frame_draft(
        organism,
        Tick::new(5),
        SensorProfile::GroundedObjectSlotsV1,
        HomeostaticSnapshot::baseline(Tick::new(5)),
    ).unwrap();
    assert_eq!(frame.grounded_object_slots().len(), 2);
    assert!(frame.sensory().channels.visual_affordance.iter().all(|value| *value == 0.0));
    assert_eq!(candidate_action_ids_for_target(&frame, food_id), generic_candidate_ids());
    assert_eq!(candidate_action_ids_for_target(&frame, hazard_id), generic_candidate_ids());
}

#[test]
fn tracked_ids_are_stable_but_raw_entity_ids_never_enter_features() {
    let (mut world, organism, _food_id, _) = two_physical_object_fixture();
    let first = grounded_frame(&mut world, organism, Tick::new(5));
    let second = grounded_frame(&mut world, organism, Tick::new(6));
    assert_eq!(first.grounded_object_slots()[0].tracked_object_id, second.grounded_object_slots()[0].tracked_object_id);
    assert_eq!(first.grounded_object_slots()[0].candidate_features().unwrap(), second.grounded_object_slots()[0].candidate_features().unwrap());
}

#[test]
fn relabelling_all_private_world_semantics_cannot_change_a_grounded_frame() {
    let (mut first, mut second, organism) = identical_physics_different_semantics_fixture();
    let a = grounded_frame(&mut first, organism, Tick::new(5));
    let b = grounded_frame(&mut second, organism, Tick::new(5));
    assert_eq!(grounded_frame_without_transport(&a), grounded_frame_without_transport(&b));
    assert!(all_semantic_truth_channels_are_zero(&a));
}

#[test]
fn sixteen_slots_yield_six_complete_family_groups_and_never_a_partial_group() {
    let frame = grounded_frame_with_object_count(16);
    assert_eq!(frame.grounded_object_slots().len(), 16);
    assert_eq!(frame.candidates().len(), 1 + 6 * 5);
    assert_eq!(frame.candidates()[0].family, CandidateActionFamily::Idle);
    for group in frame.candidates()[1..].chunks_exact(5) {
        assert_eq!(group.iter().map(|c| c.family).collect::<Vec<_>>(), generic_families());
        assert!(group.iter().all(|c| c.observation == group[0].observation));
    }
}

#[test]
fn duplicate_looking_objects_keep_distinct_tracked_bindings() {
    let frame = duplicate_descriptor_grounded_frame();
    assert_eq!(frame.candidates()[1].features, frame.candidates()[6].features);
    assert_ne!(frame.candidates()[1].observation, frame.candidates()[6].observation);
    assert_ne!(tracked_id_for_candidate(&frame, 1), tracked_id_for_candidate(&frame, 6));
}

#[test]
fn per_organism_tracked_ids_do_not_depend_on_cross_organism_schedule_order() {
    let first = observe_schedule([OrganismId(1), OrganismId(2)]);
    let second = observe_schedule([OrganismId(2), OrganismId(1)]);
    assert_eq!(first.for_organism(OrganismId(1)), second.for_organism(OrganismId(1)));
    assert_eq!(first.for_organism(OrganismId(2)), second.for_organism(OrganismId(2)));
}

#[test]
fn tracker_capacity_evicts_deterministically_and_never_reuses_an_id() {
    let mut first = tracked_registry_fixture(2);
    let mut second = tracked_registry_fixture(2);
    let a = observe_three_objects(&mut first, [9, 10, 11]);
    let b = observe_three_objects(&mut second, [9, 10, 11]);
    assert_eq!(a, b);
    assert_eq!(a[2].evicted, Some(a[0].tracked_object_id));
    assert_eq!(first.records_for(OrganismId(1)).unwrap().len(), 2);
    let reappeared = observe_object(&mut first, 9, Tick::new(12)).unwrap();
    assert!(reappeared.tracked_object_id.raw() > a[2].tracked_object_id.raw());
    assert_ne!(reappeared.tracked_object_id, a[0].tracked_object_id);
}

#[test]
fn tracker_record_keeps_portable_provenance_descriptor_and_last_seen_tick() {
    let mut tracker = tracked_registry_fixture(8);
    let provenance = tracking_provenance(9);
    let descriptor = stable_descriptor_fixture(0.25);
    let receipt = tracker.observe(
        OrganismId(7), provenance, descriptor, Tick::new(44),
    ).unwrap();
    let record = tracker.record(OrganismId(7), receipt.tracked_object_id).unwrap();
    assert_eq!(record.tracking_provenance, provenance);
    assert_eq!(record.tracking_key, provenance.canonical_key());
    assert_eq!(record.stable_physical_descriptor, descriptor);
    assert_eq!(record.last_seen_tick, Tick::new(44));
}

#[test]
fn world_save_roundtrip_preserves_grounded_physical_properties() {
    let save = grounded_world_save_fixture();
    let loaded = roundtrip_world_save(&save).unwrap();
    assert_eq!(loaded.objects[0].grounded_physical, save.objects[0].grounded_physical);
}

#[test]
fn portable_tracking_key_is_identical_after_world_save_reload() {
    let world = grounded_world_with_spawn_provenance();
    let before = world.objects()[0].tracking_provenance.canonical_key();
    let loaded = roundtrip_world(&world).unwrap();
    let after = loaded.objects()[0].tracking_provenance.canonical_key();
    assert_eq!(before, after);
    assert_eq!(loaded.objects()[0].tracking_key, after);
}
```

The fixture must give the two objects explicit physical signatures: cyan/bitter `color=[0.0,1.0,1.0]`, `chemical=[-0.9,0.1,0.0]` and amber/sweet `color=[1.0,0.55,0.1]`, `chemical=[0.4,0.0,0.0]`. Their private `WorldObjectKind` values may differ, but the grounded extractor receives only physical fields.

- [ ] **Step 2: Run and verify profile extraction is missing**

Run: `cargo test -p alife_world --test grounded_object_slots`

Expected: compile failure for missing grounded world fields and extractor.

- [ ] **Step 3: Add physical observation state and deterministic tracking**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GroundedPhysicalProperties {
    pub velocity: Vec3f,
    pub color: [f32; 3],
    pub material: [f32; 3],
    pub shape: [f32; 3],
    pub chemical: [f32; 3],
    pub surface_temperature: f32,
    pub terrain: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PhysicalTrackingKey(pub [u64; 2]);

pub const PHYSICAL_TRACKING_PROVENANCE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PhysicalTrackingProvenance {
    pub schema_version: u16,
    pub world_seed: u64,
    pub zone_id: u32,
    pub spawn_sequence: u64,
    pub lineage_key: u64,
}

impl PhysicalTrackingProvenance {
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError>;
    pub fn canonical_key(&self) -> PhysicalTrackingKey;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalObservationSnapshot {
    pub observer: OrganismId,
    pub tick: Tick,
    pub observer_pose: Pose,
    pub observer_velocity: Velocity,
    pub visible: Vec<PhysicalObservedObject>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalObservedObject {
    pub transport_entity: WorldEntityId,
    pub tracking_provenance: PhysicalTrackingProvenance,
    pub tracking_key: PhysicalTrackingKey,
    pub position: Vec3f,
    pub properties: GroundedPhysicalProperties,
    pub contact: bool,
    pub confidence: Confidence,
}

pub const DEFAULT_TRACKED_OBJECT_CAPACITY_PER_ORGANISM: u32 = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StablePhysicalDescriptor(pub [f32; 15]);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrackedObjectRecord {
    pub tracked_object_id: TrackedObjectId,
    pub tracking_provenance: PhysicalTrackingProvenance,
    pub tracking_key: PhysicalTrackingKey,
    pub last_seen_tick: Tick,
    pub stable_physical_descriptor: StablePhysicalDescriptor,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrackedObjectObservationReceipt {
    pub organism_id_raw: u64,
    pub tracked_object_id: TrackedObjectId,
    pub inserted: bool,
    pub evicted: Option<TrackedObjectId>,
    pub record_count: u32,
    pub next_id: u64,
    pub before_digest: [u64; 4],
    pub after_digest: [u64; 4],
}

pub struct TrackedObjectRegistry {
    world_seed: u64,
    per_organism_capacity: u32,
    organisms: BTreeMap<u64, OrganismTrackedObjects>,
}

pub struct OrganismTrackedObjects {
    organism_id_raw: u64,
    world_seed: u64,
    capacity: u32,
    next_id: u64,
    records_by_key: BTreeMap<PhysicalTrackingKey, TrackedObjectRecord>,
}

impl TrackedObjectRegistry {
    pub fn observe(
        &mut self,
        observer: OrganismId,
        provenance: PhysicalTrackingProvenance,
        descriptor: StablePhysicalDescriptor,
        tick: Tick,
    ) -> Result<TrackedObjectObservationReceipt, ScaffoldContractError> {
        let world_seed = self.world_seed;
        let capacity = self.per_organism_capacity;
        let observer_raw = observer.raw();
        let state = self.organisms.entry(observer_raw).or_insert_with(|| {
            OrganismTrackedObjects::new(observer_raw, world_seed, capacity)
        });
        state.observe(provenance, descriptor, tick)
    }

    pub fn records_for(
        &self,
        observer: OrganismId,
    ) -> Option<impl ExactSizeIterator<Item = &TrackedObjectRecord>>;

    pub fn record(
        &self,
        observer: OrganismId,
        tracked_object_id: TrackedObjectId,
    ) -> Option<&TrackedObjectRecord>;
}
```

`PhysicalObservationSnapshot` is built directly from geometry/contact/physical
components and must not call or embed `HeadlessSensoryReport`; its types contain
no `WorldObjectKind`, affordance, lesson, teacher, nutrition, or toxicity field.
Every `WorldObject` and `WorldObjectSaveState` stores
`PhysicalTrackingProvenance` plus its derived `PhysicalTrackingKey`.
`canonical_key` hashes exactly this canonical byte sequence: ASCII domain
`ALIFE-PHYSICAL-TRACK-V1`, then little-endian schema `u16`, world seed `u64`,
zone ID `u32`, spawn sequence `u64`, and lineage key `u64`. Fold those bytes,
in order, through two SplitMix64 streams seeded with
`0xA11F_EA7E_D00D_0001` and `0xC0DE_CAFE_51A7_0002`; each eight-byte chunk is
zero-padded, interpreted little-endian, XORed into the stream, then followed by
one SplitMix64 round. The final pair is `PhysicalTrackingKey([first, second])`;
an all-zero pair is remapped by one additional round. Do not use
`DefaultHasher` or `WorldEntityId`.

The v1 world-save migration derives provenance deterministically from saved
world seed, saved zone ID, the object's stable ordinal in canonical saved-
object order as spawn sequence, and lineage key zero, then writes it on the
next save. It never hashes floating-point world coordinates. Reject a saved key that
does not equal the canonical key derived from saved provenance. The registry
uses the key only for reassociation; neither provenance nor the key enters
candidate/query features. `StablePhysicalDescriptor` is exactly color,
material, shape, chemical, surface temperature, and terrain in that order and
is validated as finite and within the same ranges as the source fields.

`OrganismTrackedObjects::observe` first validates provenance schema/world seed,
recomputes the key, validates descriptor and nondecreasing tick, and computes
the before digest without mutating state. An existing key retains its ID and
updates descriptor/last-seen. A novel key allocates the current nonzero
`next_id` with `checked_add`; ID exhaustion is a typed identity-exhaustion
error, never a capacity error. If the organism-local capacity is full, evict
the minimum `(last_seen_tick.raw(), tracked_object_id.raw(), tracking_key)`
record before insertion. Evicted IDs are never reused, and re-observing an
evicted key allocates a new ID. The receipt binds before/after canonical
little-endian digests, eviction, record count, and next ID.

Initialize `next_id` with the exact deterministic nonzero function
`initial_tracked_object_id(world_seed, organism_id_raw) = 1 +
(splitmix64(world_seed ^ organism_id_raw.rotate_left(23) ^
0xA11F_7A4C_0B1E_C701) & 0x0000_FFFF_FFFF_FFFF)`. This reserves ample
checked-increment headroom and can never produce zero. Constrain registry
capacity to `1..=DEFAULT_TRACKED_OBJECT_CAPACITY_PER_ORGANISM`. The outer map
is `BTreeMap<u64, _>` keyed by `OrganismId::raw()`. Copy world seed/capacity
before `entry` and delegate mutation to the organism state, as shown, so no
closure or simultaneous borrow captures `self`. Task 9 persists complete
records plus next ID/capacity and reconstructs the key index; it never
serializes a raw entity association.

- [ ] **Step 4: Implement semantic-free slot extraction**

For each visible object, build the exact 15-value stable descriptor and call
`TrackedObjectRegistry::observe(observer, tracking_provenance, descriptor,
tick)` before constructing its slot; use only the returned tracked ID. Compute
bearing as `[sin(theta), cos(theta)]`, normalize distance by the active vision
radius, clamp relative velocity by the profile velocity ceiling, copy the named
physical groups, derive contact from the same snapshot, and derive
proprioception from observer linear/angular speed. `grounded_sensing.rs` must
not import `WorldObjectKind` or `AffordanceBits`.

- [ ] **Step 5: Enumerate equal generic families within the candidate budget**

Preserve Slice A's deterministic candidate-bearing object budget of six:
stable-sort all 16 sensed slots by distance then tracked-object ID, retain the
first `floor((MAX_ACTION_CANDIDATES - 1) / 5)`, and for every retained slot emit
exactly inspect, approach, avoid, contact, and ingest plus one frame-level idle
candidate. Use action IDs `ActionKind::Inspect.canonical_id()`,
`HeadlessActionIds::APPROACH`, `HeadlessActionIds::FLEE`,
`HeadlessActionIds::EAT`, and `HeadlessActionIds::GRAB` in the exact Slice A
family order Inspect, Approach, Avoid, Ingest, Contact. Set the corresponding
`CandidateActionFamily` and `CandidateObservationRef::ObjectSlot(slot_index)`
on every candidate; copy only
`slot.candidate_features()` into the candidate decoder vector and retain
`WorldEntityId` solely in `ActionTarget` transport. Candidate-bearing slot
selection cannot inspect `WorldObjectKind`, affordance bits, or semantic
properties.

When ingesting an object, apply both its nutrition and pain/toxicity outcome fields so one object can be mechanically edible and poisonous without exposing that truth in perception.

Version `WorldObjectSaveState` and its conversions to preserve
`GroundedPhysicalProperties`, `PhysicalTrackingProvenance`, and its canonical
key; provide deterministic physical/provenance defaults only in the tested
legacy migration. Update all object initializers/fixtures in the same commit so
the workspace never has an intermediate lossy save format.

- [ ] **Step 6: Run world and source-boundary tests**

Run: `cargo test -p alife_world --test grounded_object_slots --test perception_candidates --test headless_world_harness`

Expected: pass.

Run: `rg -n -i "HeadlessSensoryReport|WorldObjectKind|AffordanceBits|food|hazard|teacher|lesson|nutrition|toxicity" crates/alife_world/src/grounded_sensing.rs`

Expected: no matches.

- [ ] **Step 7: Commit**

```powershell
git add crates/alife_world/src/grounded_sensing.rs crates/alife_world/src/tracked_objects.rs crates/alife_world/src/candidate_enumerator.rs crates/alife_world/src/headless.rs crates/alife_world/src/persistence.rs crates/alife_world/src/lib.rs crates/alife_world/tests/grounded_object_slots.rs crates/alife_world/tests/save_load_roundtrip.rs
git commit -m "Extract grounded object-slot candidates"
```

### Task 4: Replace prefix queries with versioned state-action-target encoding

**Files:**
- Create: `crates/alife_core/src/memory_query.rs`
- Create: `crates/alife_core/tests/candidate_memory_queries.rs`
- Modify: `crates/alife_core/src/experience.rs`
- Modify: `crates/alife_core/src/version.rs`
- Modify: `crates/alife_core/src/lib.rs`

**Interfaces:**
- Consumes: immutable Slice A `PerceptionFrameDraft`, `ActionCandidate`,
  `GroundedObjectSlotV1`, `SensorProfileProvenance`, `DriveSnapshot`,
  `EndocrineSnapshot`, and sealed `DecisionSnapshot`.
- Produces: `MemoryQueryVersion::StateActionTargetV2`,
  `CandidateMemoryQueryV2`, Slice A `PerceptionContextDigest`,
  `EpisodicDecisionKeyV2`, `CandidateMemoryContextV1`,
  `EpisodicRetrievalContextV1`, and
  `MemoryQueryEncoderV2::encode_candidate`.

- [ ] **Step 1: Write failing stratification tests**

```rust
#[test]
fn late_drive_hormone_action_and_target_strata_survive_full_visual_input() {
    let (draft, candidate) = fully_populated_grounded_draft();
    let query = MemoryQueryEncoderV2::encode_candidate(&draft, &candidate).unwrap();
    assert_eq!(query.features().len(), MEMORY_QUERY_V2_FEATURE_COUNT);
    assert!(query.features()[12..23].iter().any(|value| *value != 0.0));
    assert!(query.features()[23..34].iter().any(|value| *value != 0.0));
    assert!(query.features()[40..49].iter().any(|value| *value == 1.0));
    assert!(query.features()[49..57].iter().any(|value| *value == 1.0));
    assert_eq!(&query.features()[57..81], &candidate.features.0);
}

#[test]
fn opposite_families_with_the_same_action_kind_have_different_queries() {
    let (draft, approach, avoid, contact, ingest) = same_target_family_draft_fixture();
    assert_eq!(approach.kind, avoid.kind);
    assert_eq!(contact.kind, ingest.kind);
    assert_ne!(encode(&draft, &approach).features(), encode(&draft, &avoid).features());
    assert_ne!(encode(&draft, &contact).features(), encode(&draft, &ingest).features());
}

#[test]
fn target_and_action_change_queries_but_transport_entity_id_does_not() {
    let (draft, candidate) = fully_populated_grounded_draft();
    let mut different_target = candidate;
    different_target.features.0[6] = 0.95;
    let mut different_transport = candidate;
    different_transport.target = ActionTarget::new(Some(WorldEntityId(999)), candidate.target.position);
    assert_ne!(
        MemoryQueryEncoderV2::encode_candidate(&draft, &candidate).unwrap().features(),
        MemoryQueryEncoderV2::encode_candidate(&draft, &different_target).unwrap().features(),
    );
    assert_eq!(
        MemoryQueryEncoderV2::encode_candidate(&draft, &candidate).unwrap().features(),
        MemoryQueryEncoderV2::encode_candidate(&draft, &different_transport).unwrap().features(),
    );
}
```

Add this digest-lifecycle test to the same file:

```rust
#[test]
fn query_binds_base_digest_and_sealed_key_binds_final_gpu_input_digest() {
    let (draft, candidate) = fully_populated_grounded_draft();
    let base = draft.base_digest();
    let query = MemoryQueryEncoderV2::encode_candidate(&draft, &candidate).unwrap();
    assert_eq!(query.base_frame_digest(), base);
    let prepared = prepared_recall_fixture_for_draft(&draft);
    let (frame, finalized) = prepared.finalize(draft).unwrap();
    let key = &finalized.candidate_keys()[candidate.candidate_index as usize];
    assert_eq!(key.query(), &query);
    assert_eq!(key.retrieval_context_digest(), finalized.context_digest());
    assert_eq!(key.final_frame_digest(), frame.frame_digest());
    assert_ne!(digest_domain_bytes(base), digest_domain_bytes(key.final_frame_digest()));
}
```

Also write failing sealed-decision tests now: an episodic key whose organism,
tick, candidate index, action ID/kind/family, profile, base digest, context
digest, final digest, or candidate feature digest differs from the frame or
`NeuralDecisionEvidence` must be rejected; a
matching key seals successfully and contains no `WorldEntityId`.

- [ ] **Step 2: Run and verify the V2 encoder is absent**

Run: `cargo test -p alife_core --test candidate_memory_queries`

Expected: compile failure for unresolved V2 memory-query types.

- [ ] **Step 3: Implement the fixed stratified layout**

```rust
pub const MEMORY_QUERY_V2_FEATURE_COUNT: usize = 96;
pub const MEMORY_STATE_SENSORY_RANGE: Range<usize> = 0..12;
pub const MEMORY_DRIVE_RANGE: Range<usize> = 12..23;
pub const MEMORY_HORMONE_RANGE: Range<usize> = 23..34;
pub const MEMORY_BODY_RANGE: Range<usize> = 34..40;
pub const MEMORY_ACTION_KIND_RANGE: Range<usize> = 40..49;
pub const MEMORY_ACTION_FAMILY_RANGE: Range<usize> = 49..57;
pub const MEMORY_TARGET_RANGE: Range<usize> = 57..81;
pub const MEMORY_PROFILE_RANGE: Range<usize> = 81..83;
pub const MEMORY_RESERVED_RANGE: Range<usize> = 83..96;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryQueryVersion {
    StateActionTargetV2 = 2,
}

impl MemoryQueryVersion {
    pub const fn raw(self) -> u16 {
        match self { Self::StateActionTargetV2 => 2 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CandidateMemoryQueryV2 {
    schema_version: u16,
    organism_id: OrganismId,
    tick: Tick,
    profile: SensorProfileIdentity,
    candidate_index: u16,
    action_id: ActionId,
    action_kind: ActionKind,
    action_family: CandidateActionFamily,
    base_frame_digest: PerceptionBaseDigest,
    candidate_feature_digest: CandidateFeatureDigest,
    tracked_object_id: Option<TrackedObjectId>,
    features: [f32; MEMORY_QUERY_V2_FEATURE_COUNT],
    canonical_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EpisodicDecisionKeyV2 {
    query: CandidateMemoryQueryV2,
    retrieval_context_digest: PerceptionContextDigest,
    final_frame_digest: PerceptionFrameDigest,
    canonical_digest: [u64; 4],
}

impl CandidateMemoryQueryV2 {
    fn try_new(encoded: EncodedCandidateMemoryQueryV2)
        -> Result<Self, ScaffoldContractError>;
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError>;
    pub fn validate_against_frame(
        &self,
        frame: &PerceptionFrame,
        candidate: &ActionCandidate,
    ) -> Result<(), ScaffoldContractError>;
    pub const fn organism_id(&self) -> OrganismId;
    pub const fn tick(&self) -> Tick;
    pub const fn profile(&self) -> SensorProfileIdentity;
    pub const fn candidate_index(&self) -> u16;
    pub const fn action_id(&self) -> ActionId;
    pub const fn action_kind(&self) -> ActionKind;
    pub const fn action_family(&self) -> CandidateActionFamily;
    pub const fn base_frame_digest(&self) -> PerceptionBaseDigest;
    pub const fn candidate_feature_digest(&self) -> CandidateFeatureDigest;
    pub const fn tracked_object_id(&self) -> Option<TrackedObjectId>;
    pub fn features(&self) -> &[f32; MEMORY_QUERY_V2_FEATURE_COUNT];
    pub const fn canonical_digest(&self) -> [u64; 4];
}

impl EpisodicDecisionKeyV2 {
    fn try_new(
        query: CandidateMemoryQueryV2,
        retrieval_context_digest: PerceptionContextDigest,
        final_frame_digest: PerceptionFrameDigest,
    ) -> Result<Self, ScaffoldContractError>;
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError>;
    pub fn query(&self) -> &CandidateMemoryQueryV2;
    pub const fn retrieval_context_digest(&self) -> PerceptionContextDigest;
    pub const fn final_frame_digest(&self) -> PerceptionFrameDigest;
    pub const fn canonical_digest(&self) -> [u64; 4];
}
```

Encode mean/max summaries for visual, auditory, smell, and tactile into indices
0-7; pain, novelty, affordance-presence count, and context confidence into
8-11; all 11 drive and 11 hormone channels into their reserved ranges; linear
and angular velocity into 34-39; one-hot all nine `ActionKind` variants into
40-48; one-hot all eight `CandidateActionFamily` variants into 49-56; copy the
complete 24-element target vector into 57-80; set exactly one profile bit at
81 or 82; leave 83-95 zero for a future version bump. Add
`SchemaKind::MemoryQuery` at version 2 and never accept a caller-provided
maximum length.
Only `MemoryQueryEncoderV2` may call the private query constructor, and only
`PreparedMemoryRecall::finalize` may call the private decision-key constructor.
Neither type derives `Deserialize`. Their custom deserializers use private wire
DTOs, call the same validated constructors, and reject a stored canonical
digest that does not equal a fresh digest of all preceding fields. Canonical query/key
digests encode all integers little-endian and use explicit
raw mappings for profile, sensory ABI, action kind/family, action ID, and
tracked ID; they never hash serde JSON, Rust enum memory, or raw entity IDs.
The query digest uses domain `ALIFE-MEMORY-QUERY-V2`; the key digest uses domain
`ALIFE-EPISODIC-DECISION-KEY-V2` and includes the validated query digest,
retrieval-context digest, and final-frame digest. Tests and callers inspect
queries/keys through accessors rather than public-field mutation.
`validate_against_frame` re-encodes all 96 lanes from the finalized frame's
base sensory/body/drive/endocrine data and the exact candidate, ignoring the
attached episodic context, then requires bit-normalized feature equality and
the same query digest. `DecisionSnapshot` and full `ExperiencePatch` custom
deserialization perform this complete re-encoding; matching only the outer
base/context/final digests is insufficient.

For `CandidateObservationRef::ObjectSlot(index)`, resolve
`tracked_object_id` only by indexing the validated same-tick
`draft.grounded_object_slots()[index]`; never search by feature equality or raw
transport entity ID. `None` yields no tracked binding. Store the canonical
`PerceptionBaseDigest` and candidate-feature digest in the query so recall is
bound to the exact world/body/homeostasis/candidate input that existed before
memory context. Never store a `PerceptionFrameDigest` in a query: it includes
episodic context and is unknowable until recall has finished. Base and final
digests use distinct hash domains even when context is all zero, so accidental
interchange is rejected rather than silently comparing equal bytes.

- [ ] **Step 4: Define bounded retrieval records**

```rust
pub const MEMORY_LATENT_V1_COUNT: usize = 8;
pub const MEMORY_VALUE_V1_COUNT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CandidateMemoryContextV1 {
    pub candidate_index: u16,
    pub target_latent: [f32; MEMORY_LATENT_V1_COUNT],
    pub family_value: [f32; MEMORY_VALUE_V1_COUNT],
    pub target_confidence: Confidence,
    pub family_confidence: Confidence,
    pub target_source_count: u16,
    pub family_source_count: u16,
    pub best_target_source: Option<MemoryId>,
    pub best_family_source: Option<MemoryId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpisodicRetrievalContextV1 {
    pub schema_version: u16,
    pub tick: Tick,
    pub profile: SensorProfileIdentity,
    pub candidates: Vec<CandidateMemoryContextV1>,
}
```

Target-latent order is hunger delta, fear delta, pain delta, curiosity delta,
BrainATP delta, novelty, contact likelihood, and prediction error. It is
retrieved by target identity/signature without an action-family filter, so all
candidates for the same tracked target receive the same cross-family evidence.
Family-value order is expected valence, success likelihood, danger/pain, and
energy delta; it is retrieved only from the candidate's exact family (and
exact action ID for `Other`). Clamp every output to `[-1, 1]` and require one
context per frame candidate in candidate-index order. Best-source IDs remain
diagnostic retrieval evidence; they are not neural input and therefore are not
part of the finalized frame's context digest.

For the authoritative GPU input, implement
`EpisodicRetrievalContextV1::to_perception_context_block()`: emit exactly 16
`f32` lanes per candidate in candidate-index order—target latent 0..7, family
value 0..3, target confidence, family confidence, target source count, family
source count. Counts are validated `u16` and therefore exactly representable as
`f32`. Candidate indices are implicit in the validated contiguous row order.
Best-source IDs and recall search statistics remain in the receipt and are not
GPU input. Use Slice A `PerceptionContextBlock::try_new(1,
PerceptionContextKind::EpisodicCandidateV1, values)`; its
`PerceptionContextDigest` is the only
context digest stored in the final episodic key.

- [ ] **Step 5: Seal the selected query with the decision**

Add `episodic_key: Option<EpisodicDecisionKeyV2>` to `DecisionSnapshot` and a
`with_finalized_memory_recall(frame, recall, selected_candidate_index)`
constructor. It first runs `FinalizedMemoryRecall::validate_for_frame`, selects
the private key at the exact contiguous candidate index, re-encodes and compares
all 96 query lanes, then verifies organism, tick, action ID/kind/family,
feature digest, base digest, retrieval-context digest, final frame digest, and
profile. It reads `PerceptionFrame::base_digest`,
`frame.context().canonical_digest()`, and `PerceptionFrame::frame_digest`; then
requires the base/final pair to equal
`NeuralDecisionEvidence.{base_digest, frame_digest}` and the GPU tick
receipt, and requires both base/final digests to equal the
corresponding Slice A `PreActionBrainEvidence::NeuralClosedLoopGpu` fields. A
base/final swap, context changed after recall, or key from another candidate is
rejected before sealing.
`EpisodicDecisionKeyV2` stores `TrackedObjectId` and query features, never
`WorldEntityId`. No public sealing API accepts a caller-supplied standalone key;
the unforgeable finalized recall remains present through decision construction.

- [ ] **Step 6: Run query and experience tests**

Run: `cargo test -p alife_core --test candidate_memory_queries --test experience_three_phase`

Expected: pass, including rejection of mismatched candidate/profile evidence.

- [ ] **Step 7: Commit**

```powershell
git add crates/alife_core/src/memory_query.rs crates/alife_core/src/experience.rs crates/alife_core/src/version.rs crates/alife_core/src/lib.rs crates/alife_core/tests/candidate_memory_queries.rs
git commit -m "Add stratified candidate memory queries"
```

### Task 5: Make episodic retrieval candidate-conditioned and deterministically bounded

**Files:**
- Modify: `crates/alife_core/src/memory.rs`
- Replace: `crates/alife_core/tests/memory_expectancy.rs`
- Create: `crates/alife_core/tests/candidate_memory_retrieval.rs`

**Interfaces:**
- Consumes: sealed `ExperiencePatch` with `EpisodicDecisionKeyV2` and a current
  immutable `PerceptionFrameDraft`.
- Produces: `MemoryBank::recall_frame`, `PreparedMemoryRecall`,
  `PreparedMemoryRecall::finalize`, `FinalizedMemoryRecall`,
  `MemorySidecarState`, `observe_sealed_patch`, `compact_deterministic`, exact
  `MemoryRecallReceipt`, `MemoryUpdateReceipt`, `MemoryCompactionReceipt`, and
  `EpisodicRetrievalContextV1`.

- [ ] **Step 1: Write failing target-specific and saturation tests**

```rust
#[test]
fn poisoned_target_memory_does_not_bias_an_unrelated_candidate() {
    let mut bank = memory_bank_with_capacity(8);
    bank.observe_sealed_patch(&poisoned_cyan_ingest_patch()).unwrap();
    let draft = cyan_and_amber_grounded_draft();
    let prepared = bank.recall_frame(&draft).unwrap();
    let cyan_ingest = prepared.context().for_target_family(CYAN, CandidateActionFamily::Ingest);
    let cyan_avoid = prepared.context().for_target_family(CYAN, CandidateActionFamily::Avoid);
    let amber_ingest = prepared.context().for_target_family(AMBER, CandidateActionFamily::Ingest);
    assert!(cyan_ingest.family_value[0] < 0.0);
    assert!(cyan_ingest.family_value[2] > 0.0);
    assert_eq!(cyan_avoid.family_value, [0.0; MEMORY_VALUE_V1_COUNT]);
    assert_eq!(cyan_ingest.target_latent, cyan_avoid.target_latent);
    assert!(cyan_avoid.target_latent[2] > 0.0);
    assert_eq!(amber_ingest.family_value, [0.0; MEMORY_VALUE_V1_COUNT]);
    assert_eq!(amber_ingest.target_latent, [0.0; MEMORY_LATENT_V1_COUNT]);
}

#[test]
fn identical_saturation_runs_merge_evict_and_compact_to_the_same_digest() {
    let first = run_memory_saturation_fixture(77, 4, 10_240).unwrap();
    let second = run_memory_saturation_fixture(77, 4, 10_240).unwrap();
    assert_eq!(first.digest, second.digest);
    assert_eq!(first.len, 4);
    assert!(first.merges > 0);
    assert!(first.evictions > 0);
    assert!(first.compactions > 0);
    assert_eq!(first.terminal_errors, 0);
}

#[test]
fn opposite_candidate_families_never_merge_or_share_recall() {
    let mut bank = memory_bank_with_capacity(8);
    bank.observe_sealed_patch(&poisoned_cyan_ingest_patch()).unwrap();
    let draft = same_target_all_family_draft();
    let prepared = bank.recall_frame(&draft).unwrap();
    let ingest = prepared.context().for_family(CandidateActionFamily::Ingest);
    let contact = prepared.context().for_family(CandidateActionFamily::Contact);
    let approach = prepared.context().for_family(CandidateActionFamily::Approach);
    let avoid = prepared.context().for_family(CandidateActionFamily::Avoid);
    assert!(ingest.family_value[2] > 0.0);
    assert_eq!(contact.family_value, [0.0; 4]);
    assert_eq!(approach.family_value, [0.0; 4]);
    assert_eq!(avoid.family_value, [0.0; 4]);
    assert_eq!(ingest.target_latent, contact.target_latent);
    assert_eq!(ingest.target_latent, approach.target_latent);
    assert_eq!(ingest.target_latent, avoid.target_latent);
}

#[test]
fn bounded_shortlist_can_retrieve_the_only_match_after_memory_id_sixty_four() {
    let bank = bank_with_distractors_then_match(80, 65);
    let prepared = bank.recall_frame(&matching_draft()).unwrap();
    assert_eq!(prepared.context().candidates[1].best_family_source, Some(MemoryId(65)));
    assert!(prepared.context().candidates[1].family_confidence.raw() > 0.0);
    assert!(prepared.receipt().similarity_evaluations <= MEMORY_TOTAL_SEARCH_CAP as u32);
}

#[test]
fn million_record_index_keeps_similarity_work_bounded() {
    let bank = synthetic_indexed_bank(1_000_000, matching_record_at(900_001));
    let prepared = bank.recall_frame(&matching_draft()).unwrap();
    assert_eq!(prepared.context().candidates[1].best_family_source, Some(MemoryId(900_001)));
    assert!(prepared.receipt().similarity_evaluations <= MEMORY_TOTAL_SEARCH_CAP as u32);
}

#[test]
fn prepared_recall_cannot_finalize_a_different_base_draft() {
    let draft = matching_draft();
    let prepared = memory_bank_fixture().recall_frame(&draft).unwrap();
    let changed = matching_draft_with_candidate_distance(0.1);
    assert!(prepared.finalize(changed).is_err());
}
```

- [ ] **Step 2: Run and confirm legacy recall fails the new expectations**

Run: `cargo test -p alife_core --test candidate_memory_retrieval`

Expected: compile failure for missing `recall_frame`, receipts, and V2 records.

- [ ] **Step 3: Replace legacy records and matching**

Store schema version, primitive profile identity, action ID/kind/family,
tracked-object binding, the full 96-element query, eight latent outcomes, four
value outcomes, confidence, salience, observation count, first/last tick, and
the sealed sequence ID that produced the record. One observation updates two
indices atomically:

- `TargetMemoryBucketKey` omits action kind/family and supplies the candidate's
  cross-family `target_latent`.
- `MemoryBucketKey` includes exact family plus exact action ID for `Other` and
  supplies only that candidate's `family_value`.

For family-value matches, filter by organism, query version, primitive profile,
exact family, and `Other` action ID, then compute:

```rust
let score = 0.30 * cosine(&query.features()[0..40], &record.features[0..40])
    + 0.10 * cosine(&query.features()[40..49], &record.features[40..49])
    + 0.20 * cosine(&query.features()[49..57], &record.features[49..57])
    + 0.35 * cosine(&query.features()[57..81], &record.features[57..81])
    + 0.05 * cosine(&query.features()[81..83], &record.features[81..83]);
```

For target-latent matches, filter only by organism, query version, primitive
profile, and target bucket, then compute
`0.45 * cosine(0..40) + 0.50 * cosine(57..81) + 0.05 * cosine(81..83)`;
action-kind and action-family strata are deliberately excluded. Maintain both
`BTreeMap<MemoryBucketKey, Vec<MemoryId>>` and
`BTreeMap<TargetMemoryBucketKey, Vec<MemoryId>>`. Each bucket retains at most 64
IDs by descending `(salience_q16, last_tick, observation_count)` then ascending
`MemoryId`. On insert/merge/evict/compaction, update record store and both
indices in one validated mutation.

For each index, query its exact key plus the exact three neighbors defined
below, merge/deduplicate retained IDs, and select at most
`MEMORY_FAMILY_SEARCH_CAP = 64` or `MEMORY_TARGET_SEARCH_CAP = 64` by target-
signature L1 distance then `MemoryId`; therefore
`MEMORY_TOTAL_SEARCH_CAP = 128` per candidate. Perform no similarity
calculation outside those shortlists. Require `MEMORY_MIN_SIMILARITY = 0.72`,
sort matches by descending score then ascending `MemoryId`, and take
`MEMORY_RECALL_TOP_K = 4` independently for each channel. If no record reaches
threshold, that channel returns zero values, empty confidence, zero sources,
and no best source. Never pool any target latent across candidates or targets.
Candidates without a tracked-object binding skip the target index entirely and
receive a zero target latent; their exact-family path may still retrieve
state/action memory (for example Idle or Rest).

`recall_frame` returns:

```rust
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRecallChannel {
    TargetLatent = 1,
    FamilyValue = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryBucketReceiptKey {
    pub organism_id_raw: u64,
    pub profile_id_raw: u16,
    pub profile_schema_version: u16,
    pub sensory_abi_version_raw: u16,
    pub query_version_raw: u16,
    pub tracked_object_id_raw: u64,
    pub family_raw: u16,
    pub other_action_id_raw: u32,
    pub target_bins: [i8; CANDIDATE_FEATURE_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetMemoryBucketReceiptKey {
    pub organism_id_raw: u64,
    pub profile_id_raw: u16,
    pub profile_schema_version: u16,
    pub sensory_abi_version_raw: u16,
    pub query_version_raw: u16,
    pub tracked_object_id_raw: u64,
    pub target_bins: [i8; CANDIDATE_FEATURE_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRecallDegradation {
    SearchShortlisted {
        candidate_index: u16,
        channel: MemoryRecallChannel,
        eligible: u32,
        searched: u32,
    },
    EmptyAfterCapacityPressure {
        candidate_index: u16,
        channel: MemoryRecallChannel,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateMemoryRecallReceipt {
    pub candidate_index: u16,
    pub query_digest: [u64; 4],
    pub target_bucket: TargetMemoryBucketReceiptKey,
    pub family_bucket: MemoryBucketReceiptKey,
    pub target_eligible: u32,
    pub target_searched: u32,
    pub target_matches: u16,
    pub family_eligible: u32,
    pub family_searched: u32,
    pub family_matches: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRecallReceipt {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub input_generation: u64,
    pub bank_digest: [u64; 4],
    pub base_frame_digest: PerceptionBaseDigest,
    pub context_digest: PerceptionContextDigest,
    pub candidate_count: u16,
    pub exact_bucket_reads: u32,
    pub neighbor_bucket_reads: u32,
    pub similarity_evaluations: u32,
    pub candidates: Vec<CandidateMemoryRecallReceipt>,
    pub degradations: Vec<MemoryRecallDegradation>,
}

pub struct PreparedMemoryRecall {
    context: EpisodicRetrievalContextV1,
    candidate_queries: Vec<CandidateMemoryQueryV2>,
    base_frame_digest: PerceptionBaseDigest,
    receipt: MemoryRecallReceipt,
}

pub struct FinalizedMemoryRecall {
    context: EpisodicRetrievalContextV1,
    base_frame_digest: PerceptionBaseDigest,
    context_digest: PerceptionContextDigest,
    final_frame_digest: PerceptionFrameDigest,
    candidate_keys: Vec<EpisodicDecisionKeyV2>,
    receipt: MemoryRecallReceipt,
}

impl PreparedMemoryRecall {
    pub fn finalize(
        self,
        draft: PerceptionFrameDraft,
    ) -> Result<(PerceptionFrame, FinalizedMemoryRecall), ScaffoldContractError>;
    pub fn validate_for_draft(&self, draft: &PerceptionFrameDraft)
        -> Result<(), ScaffoldContractError>;
    pub const fn base_frame_digest(&self) -> PerceptionBaseDigest;
    pub fn context(&self) -> &EpisodicRetrievalContextV1;
    pub fn receipt(&self) -> &MemoryRecallReceipt;
}

impl FinalizedMemoryRecall {
    pub fn validate_for_frame(&self, frame: &PerceptionFrame)
        -> Result<(), ScaffoldContractError>;
    pub const fn base_frame_digest(&self) -> PerceptionBaseDigest;
    pub const fn context_digest(&self) -> PerceptionContextDigest;
    pub const fn final_frame_digest(&self) -> PerceptionFrameDigest;
    pub fn context(&self) -> &EpisodicRetrievalContextV1;
    pub fn candidate_keys(&self) -> &[EpisodicDecisionKeyV2];
    pub fn receipt(&self) -> &MemoryRecallReceipt;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MemoryBucketKey {
    organism_id_raw: u64,
    profile_id_raw: u16,
    profile_schema_version: u16,
    sensory_abi_version_raw: u16,
    query_version_raw: u16,
    tracked_object_id_raw: u64,
    family_raw: u16,
    other_action_id_raw: u32,
    target_bins: [i8; CANDIDATE_FEATURE_COUNT],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TargetMemoryBucketKey {
    organism_id_raw: u64,
    profile_id_raw: u16,
    profile_schema_version: u16,
    sensory_abi_version_raw: u16,
    query_version_raw: u16,
    tracked_object_id_raw: u64,
    target_bins: [i8; CANDIDATE_FEATURE_COUNT],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MemoryRecordIdentity {
    organism_id_raw: u64,
    profile_id_raw: u16,
    profile_schema_version: u16,
    sensory_abi_version_raw: u16,
    query_version_raw: u16,
    tracked_object_id_raw: u64,
    family_raw: u16,
    other_action_id_raw: u32,
    exact_target_bins: [i8; CANDIDATE_FEATURE_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryUpdateKind {
    Inserted { inserted: MemoryId },
    Merged { into: MemoryId },
    Evicted { removed: MemoryId, inserted: MemoryId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryUpdateReceipt {
    pub sealed_sequence_id: ExperienceSequenceId,
    pub organism_id_raw: u64,
    pub bucket: MemoryBucketReceiptKey,
    pub target_bucket: TargetMemoryBucketReceiptKey,
    pub input_generation: u64,
    pub output_generation: u64,
    pub kind: MemoryUpdateKind,
    pub record_count: u32,
    pub capacity: u32,
    pub merge_count: u64,
    pub eviction_count: u64,
    pub before_digest: [u64; 4],
    pub after_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCompactionReceipt {
    pub identity: MemoryCompactionIdentity,
    pub output_generation: u64,
    pub output_digest: [u64; 4],
    pub merged: u32,
    pub evicted: u32,
    pub record_count: u32,
    pub capacity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCompactionIdentity {
    pub organism_id_raw: u64,
    pub cycle_id: u64,
    pub policy_version: u16,
    pub max_records_after: u32,
    pub input_generation: u64,
    pub input_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryCompactionPhase {
    Idle,
    Pending {
        cycle_id: u64,
        input_generation: u64,
        input_digest: [u64; 4],
        max_records_after: u32,
        policy_version: u16,
    },
    Staged {
        cycle_id: u64,
        input_generation: u64,
        output_generation: u64,
        input_digest: [u64; 4],
        output_digest: [u64; 4],
        receipt: MemoryCompactionReceipt,
    },
    Committed {
        cycle_id: u64,
        output_generation: u64,
        output_digest: [u64; 4],
        receipt: MemoryCompactionReceipt,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCompactionCheckpoint {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub active_generation: u64,
    pub active_digest: [u64; 4],
    pub last_committed_cycle_id: Option<u64>,
    pub next_cycle_id: u64,
    pub phase: MemoryCompactionPhase,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedMemoryCompaction {
    staged_bank: MemoryBank,
    checkpoint: MemoryCompactionCheckpoint,
    receipt: MemoryCompactionReceipt,
}

pub struct MemorySidecarState {
    organism_id: OrganismId,
    bank: MemoryBank,
    compaction: MemoryCompactionCheckpoint,
    staged_bank: Option<MemoryBank>,
}

impl MemorySidecarState {
    pub const fn organism_id(&self) -> OrganismId;
    pub fn bank(&self) -> &MemoryBank;
    pub fn compaction_checkpoint(&self) -> &MemoryCompactionCheckpoint;
    pub fn recall_frame(
        &self,
        draft: &PerceptionFrameDraft,
    ) -> Result<PreparedMemoryRecall, ScaffoldContractError>;
    pub fn observe_sealed_patch(
        &mut self,
        patch: &ExperiencePatch,
    ) -> Result<MemoryUpdateReceipt, ScaffoldContractError>;
    pub fn prepare_compaction(
        &self,
        cycle_id: u64,
        max_records_after: u32,
        policy_version: u16,
    ) -> Result<PreparedMemoryCompaction, ScaffoldContractError>;
    pub fn commit_compaction(
        &mut self,
        prepared: PreparedMemoryCompaction,
    ) -> Result<MemoryCompactionReceipt, ScaffoldContractError>;
}
```

Both private bucket keys and their public receipt DTOs use only primitive stable
IDs: `OrganismId::raw()`, the three raw profile fields, query-version raw ID,
the optional `TrackedObjectId::raw()` (zero for no binding), the explicit
`CandidateActionFamily::raw()` mapping, and `ActionId::raw()` for `Other` (zero
otherwise). No ordered key embeds a domain enum/newtype whose
derived ordering could drift. `MemoryBucketKey::for_candidate` maps every finite normalized target feature
`x` to `round(clamp(x, -1, 1) * 7)` as `i8`. `other_action_id` is `Some` only
for `CandidateActionFamily::Other` and `None` otherwise. Query keys are emitted
and deduplicated in this exact order: the exact key; distance bin at feature 2
saturating minus one; distance bin saturating plus one; and a bearing-neutral
key with bins 0 and 1 set to zero. Index vectors and query key lists use this
stable order, never hash-map iteration order.

All receipt digests use the same canonical little-endian memory-bank encoding.
Finite floats encode as IEEE-754 `to_bits()` after normalizing negative zero to
positive zero; NaN/Inf were rejected before digesting. Do not digest serde
output or platform hash maps.
Every receipt is exact rather than summary-only: recall names the immutable
canonical query digest plus both bucket keys
and per-channel eligible/searched/match counts for every candidate; update names
the two mutated bucket keys and precise insert/merge/evict IDs; compaction names
organism, policy, target capacity, generations, digests, and exact counts.
`MemoryCompactionCheckpoint::validate_contract` requires monotonic generations,
matching phase/receipt cycle and digests, `output_generation =
input_generation + 1`, and a Committed phase whose output equals the active
generation/digest and last committed cycle. A compaction cycle is nonzero and
must equal `next_cycle_id`; starting it increments `next_cycle_id` with checked
arithmetic exactly once. While Pending or Staged, a different cycle is rejected.
Retrying the same cycle is accepted only when the complete
`MemoryCompactionIdentity` is byte-for-byte equal; changed policy, target,
generation, or digest is a typed conflict. Committed retry returns the stored
receipt, and a cycle less than `next_cycle_id` with no matching committed
receipt is rejected as stale. Pending also binds the compaction
target and policy version. `prepare_compaction(cycle_id,
max_records_after, policy_version)` returns `PreparedMemoryCompaction` without mutating the
active bank. `MemorySidecarState` is the only owner of the bank/checkpoint and
is keyed by `OrganismId::raw()` in the app. It atomically swaps the staged bank
before recording Committed; retrying a committed cycle returns the recorded
receipt. These portable core types contain no app, GPU handle, slot, adapter,
or wgpu type.
All sidecar fields and prepared-compaction contents are private. App code uses
the validated recall/observe/prepare/commit methods above; no external caller
can replace a bank, checkpoint, staged bank, or receipt independently.

The module validates `PreparedMemoryRecall` and `FinalizedMemoryRecall` at
construction and again at their consumption boundary. Their fields are private;
deserialization is not implemented for either transient type. No caller can
replace a context, query, key, receipt, or digest after recall.

Encode every candidate query exactly once against the validated draft and
preserve those exact queries in candidate-index order.
`PreparedMemoryRecall::finalize` first requires the consumed draft's immutable
base digest to equal `base_frame_digest`, validates tick/profile/candidate rows,
materializes the exact memory rows as a `PerceptionContextBlock` with context
kind `PerceptionContextKind::EpisodicCandidateV1`, and calls Slice A
`PerceptionFrameDraft::finalize(self, context)` exactly once. It then constructs
each final episodic key from the retained query, context-block digest, and final
full GPU-input digest. All validation and context serialization happens before
consuming the draft; on failure no finalized frame exists. Slice C adds no API
that mutates a finalized frame. Never recompute a query after finalization.
`MemoryBank::recall_frame(&PerceptionFrameDraft) ->
Result<PreparedMemoryRecall, ScaffoldContractError>` returns capacity/search
pressure inside `receipt.degradations`; only malformed frame/profile/query
contracts use `Err`. A degraded no-match produces validated candidate-local
zero rows and the complete retained query vector.

`observe_sealed_patch` consumes only the selected sealed
`EpisodicDecisionKeyV2`; it does not re-encode the current world. Before any
mutation it revalidates query base digest, key context/final digests, selected
candidate/profile/family/action/tracked binding, patch pre-action base/final
digests, and monotonic sequence ID. Its outcome-to-latent/value encoding is
versioned and finite/bounded. Any mismatch leaves generation, both indices,
records, counters, and digest unchanged.

- [ ] **Step 4: Implement deterministic merge and eviction**

Merge only when the complete `MemoryRecordIdentity` is equal -- including
organism, query version, all profile fields, tracked target (zero only for no
binding), family, `Other` action ID, and exact target bins -- and weighted query
similarity is at least `0.98`; neighboring keys are recall-only and never make
records merge-compatible. Preserve the lower memory ID,
increment observation count, and use count-weighted means for query/outcome
fields. If full and no merge is possible, evict the minimum tuple
`(salience_q16, last_tick.raw(), memory_id.raw())`. Return one of `Inserted`,
`Merged { into }`, or `Evicted { removed, inserted }`; capacity never returns an
error. Reject duplicate/non-monotonic sealed sequence IDs before changing the
bank generation.

- [ ] **Step 5: Implement deterministic sleep compaction**

Sort by the complete `MemoryRecordIdentity`, then memory ID; fold only adjacent
records with identical identity that satisfy the same `0.98` merge predicate;
sort survivors by
descending retention tuple; truncate to `max_records_after`; finally restore
ascending last-tick/memory-ID order. Return exact merged/evicted counts and a
canonical little-endian digest. The receipt's `MemoryCompactionIdentity` must
equal the checkpoint identity and is included in its canonical digest.

- [ ] **Step 6: Retire candidate-invariant production APIs**

Remove `MemoryQuery::from_pre_action(max_feature_len)`, `feature_vector_from_pre_action`, `MemoryExpectancy` from the production path, and any function that computes one `memory_delta` for every proposal. If legacy fixtures still need them, move them behind `#[cfg(any(test, feature = "reference-debug"))]` and keep them out of `GpuClosedLoopBackend` and app runtime dependencies.

- [ ] **Step 7: Run memory and core tests**

Run: `cargo test -p alife_core --test candidate_memory_queries --test candidate_memory_retrieval --test experience_three_phase`

Expected: pass.

Run: `rg -n "from_pre_action\(|memory_delta|bias_proposals" crates/alife_core/src crates/alife_game_app/src`

Expected: no production action-path matches; a test-only `reference-debug` match is allowed only under its explicit cfg.

- [ ] **Step 8: Commit**

```powershell
git add -A crates/alife_core/src/memory.rs crates/alife_core/tests/memory_expectancy.rs crates/alife_core/tests/candidate_memory_retrieval.rs
git commit -m "Make episodic recall candidate conditional"
```

### Task 6: Feed memory through bounded GPU neural and decoder channels

**Files:**
- Create: `crates/alife_gpu_backend/src/closed_loop_memory.rs`
- Create: `crates/alife_gpu_backend/shaders/closed_loop_memory_context.wgsl`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl`
- Create: `crates/alife_gpu_backend/tests/closed_loop_memory_context.rs`
- Modify: `crates/alife_core/src/phenotype.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_buffers.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_pipeline.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_runtime.rs`
- Modify: `crates/alife_gpu_backend/src/closed_loop_learning.rs`
- Modify: `crates/alife_gpu_backend/shaders/closed_loop_eligibility.wgsl`
- Modify: `crates/alife_gpu_backend/src/lib.rs`
- Modify: `crates/alife_gpu_backend/tests/support/mod.rs`

**Interfaces:**
- Consumes: finalized `EpisodicRetrievalContextV1`, Slice B decoder metadata,
  eligibility/fast-weight pools, and candidate decoder state.
- Produces: `GpuCandidateMemoryRecord`, `GpuMemoryContextUpload`, populated
  memory fields in Slice B `GpuBrainSlotExtensionRecord`, `MemoryChannelPlan`,
  `GpuMemoryChannelPlan`,
  plastic `CompiledSynapseKind::Decoder` rows with
  `DecoderHeadKind::MemoryContext`, `GpuPerceptionFrameBinding`,
  `GpuMemoryContextDispatchReceipt`, `GpuClosedLoopMemoryTickInput`,
  `GpuClosedLoopMemoryBatchInput`, and
  `add_candidate_memory_context`.

- [ ] **Step 1: Write failing ABI, shader, and hardware tests**

```rust
#[test]
fn candidate_memory_record_is_exactly_sixty_four_bytes() {
    assert_eq!(std::mem::size_of::<GpuCandidateMemoryRecord>(), 64);
    assert_eq!(std::mem::size_of::<GpuMemoryContextHeader>(), 64);
    assert_eq!(std::mem::size_of::<GpuBrainSlotExtensionRecord>(), 80);
    assert_eq!(std::mem::size_of::<GpuMemoryChannelPlan>(), 32);
    assert_eq!(std::mem::align_of::<GpuCandidateMemoryRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuMemoryContextHeader>(), 16);
    assert_eq!(std::mem::align_of::<GpuMemoryChannelPlan>(), 16);
    assert_eq!(std::mem::offset_of!(GpuCandidateMemoryRecord, target_latent), 16);
    assert_eq!(std::mem::offset_of!(GpuCandidateMemoryRecord, family_value), 48);
    assert_eq!(std::mem::offset_of!(GpuMemoryContextHeader, brain_slot_index), 48);
    assert_eq!(std::mem::offset_of!(GpuMemoryContextHeader, decoder_learning_input_offset), 52);
    assert_eq!(std::mem::offset_of!(GpuMemoryContextHeader, perception_header_index), 56);
    assert_eq!(std::mem::offset_of!(GpuMemoryChannelPlan, max_candidate_gain), 16);
    assert_eq!(std::mem::offset_of!(GpuMemoryChannelPlan, reserved), 24);
}

#[test]
fn memory_wgsl_struct_lane_order_matches_host_abi() {
    let module = naga::front::wgsl::parse_str(CLOSED_LOOP_MEMORY_CONTEXT_WGSL).unwrap();
    assert_eq!(
        wgsl_member_names(&module, "GpuCandidateMemoryRecord"),
        ["candidate_index", "target_confidence", "family_confidence", "source_counts_packed", "target_latent", "family_value"],
    );
    assert_eq!(
        wgsl_member_names(&module, "GpuMemoryContextHeader"),
        ["schema_version", "class_id", "slot", "slot_generation", "tick_lo", "tick_hi", "candidate_count", "memory_context_offset", "candidate_offset", "profile_id", "profile_schema_version", "sensory_abi_version", "brain_slot_index", "decoder_learning_input_offset", "perception_header_index", "reserved"],
    );
    assert_wgsl_struct_size_and_alignment(&module, "GpuCandidateMemoryRecord", 64, 16);
    assert_wgsl_struct_size_and_alignment(&module, "GpuMemoryContextHeader", 64, 16);
    assert_eq!(
        wgsl_member_names(&module, "GpuMemoryChannelPlan"),
        ["schema_version", "target_latent_lane_start", "family_value_lane_start", "decoder_input_stride", "max_candidate_gain", "memory_decoder_synapse_count", "reserved"],
    );
    assert_wgsl_struct_size_and_alignment(&module, "GpuMemoryChannelPlan", 32, 16);
}

#[test]
fn memory_shader_exposes_only_candidate_local_decoder_entry() {
    let module = naga::front::wgsl::parse_str(CLOSED_LOOP_MEMORY_CONTEXT_WGSL).unwrap();
    assert!(module.entry_points.iter().any(|entry| entry.name == "add_candidate_memory_context"));
    assert!(!module.entry_points.iter().any(|entry| entry.name == "encode_memory_state"));
    assert!(!CLOSED_LOOP_MEMORY_CONTEXT_WGSL.contains("encoded_inputs"));
    assert!(!CLOSED_LOOP_MEMORY_CONTEXT_WGSL.contains("activations["));
}

#[test]
fn conditioning_one_candidate_changes_gpu_selection_without_touching_the_other_context() {
    let neutral = run_memory_context_fixture([zero_memory_row(), zero_memory_row()]).unwrap();
    let poisoned = run_memory_context_fixture([poisoned_target_row(), zero_memory_row()]).unwrap();
    assert_ne!(neutral.selection.candidate_index, poisoned.selection.candidate_index);
    assert_eq!(poisoned.uploaded_contexts[1].target_latent, [0.0; 8]);
    assert_eq!(poisoned.uploaded_contexts[1].family_value, [0.0; 4]);
    assert_eq!(neutral.recurrent_activation_digest, poisoned.recurrent_activation_digest);
    assert_eq!(poisoned.compact_readback_bytes, GPU_SELECTION_RECORD_BYTES);
}

#[test]
fn zero_memory_projection_ablation_removes_the_conditioning_effect() {
    let neutral = run_memory_context_fixture_with_scale([zero_memory_row(), zero_memory_row()], 0.0).unwrap();
    let conditioned = run_memory_context_fixture_with_scale([poisoned_target_row(), zero_memory_row()], 0.0).unwrap();
    assert_eq!(neutral.selection.candidate_index, conditioned.selection.candidate_index);
    assert!((neutral.selection.logit - conditioned.selection.logit).abs() <= neutral.tolerance);
}

#[test]
fn memory_decoder_is_plastic_through_slice_b_eligibility_and_fast_weights() {
    let mut brain = memory_decoder_learning_fixture().unwrap();
    let tick = brain.tick(&conditioned_target_frame()).unwrap();
    let before = brain.memory_decoder_fast_digest();
    let receipt = brain.apply_sealed_outcome(&painful_patch(&tick)).unwrap();
    assert!(receipt.decoder_fast_weights_changed > 0);
    assert_ne!(brain.memory_decoder_fast_digest(), before);
    assert!(brain.pending_memory_decoder_eligibility().is_none());
}

#[test]
fn memory_plan_changes_phenotype_hash_and_compiled_decoder_budget() {
    let enabled = compile_memory_phenotype(memory_plan_fixture()).unwrap();
    let disabled = compile_memory_phenotype(no_memory_plan_fixture()).unwrap();
    assert_ne!(enabled.phenotype_hash(), disabled.phenotype_hash());
    assert_eq!(enabled.budgets().global.memory_decoder_synapses, 8 * (8 + 4));
    assert_eq!(enabled.budgets().global.decoder_input_lanes, 36);
    assert_eq!(disabled.budgets().global.memory_decoder_synapses, 0);
    assert_eq!(
        disabled.budgets().global.decoder_input_lanes,
        u16::try_from(CANDIDATE_FEATURE_COUNT).unwrap(),
    );
}

#[test]
fn upload_and_tick_receipt_bind_base_context_final_and_perception_header() {
    let fixture = finalized_memory_upload_fixture();
    let tick = run_memory_upload(fixture.upload.clone()).unwrap();
    assert_eq!(tick.memory_context_binding.base_frame_digest, fixture.base_digest);
    assert_eq!(tick.memory_context_binding.context_digest, fixture.context_digest);
    assert_eq!(tick.memory_context_binding.final_frame_digest, fixture.final_digest);
    assert_eq!(tick.memory_context_binding.perception_header_index, fixture.perception_header_index);
    for malformed in mismatched_digest_or_header_uploads(&fixture) {
        assert!(run_memory_upload(malformed).is_err());
    }
}
```

- [ ] **Step 2: Run and verify GPU memory support is absent**

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_memory_context -- --nocapture`

Expected: compile failure for missing records, constants, and shader entry points.

- [ ] **Step 3: Add the upload ABI and phenotype plan**

```rust
#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuCandidateMemoryRecord {
    pub candidate_index: u32,
    pub target_confidence: f32,
    pub family_confidence: f32,
    pub source_counts_packed: u32,
    pub target_latent: [f32; 8],
    pub family_value: [f32; 4],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuMemoryContextHeader {
    pub schema_version: u32,
    pub class_id: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub tick_lo: u32,
    pub tick_hi: u32,
    pub candidate_count: u32,
    pub memory_context_offset: u32,
    pub candidate_offset: u32,
    pub profile_id: u32,
    pub profile_schema_version: u32,
    pub sensory_abi_version: u32,
    pub brain_slot_index: u32,
    pub decoder_learning_input_offset: u32,
    pub perception_header_index: u32,
    pub reserved: u32,
}

#[derive(Clone)]
pub struct GpuMemoryContextUpload {
    pub header: GpuMemoryContextHeader,
    pub records: Vec<GpuCandidateMemoryRecord>,
    pub base_frame_digest: PerceptionBaseDigest,
    pub context_digest: PerceptionContextDigest,
    pub final_frame_digest: PerceptionFrameDigest,
    pub perception_header_index: u32,
}

pub struct GpuClosedLoopMemoryTickInput<'a> {
    handle: GpuBrainHandle,
    frame: &'a PerceptionFrame,
    memory_upload: &'a GpuMemoryContextUpload,
}

impl<'a> GpuClosedLoopMemoryTickInput<'a> {
    pub fn try_new(
        handle: GpuBrainHandle,
        frame: &'a PerceptionFrame,
        memory_upload: &'a GpuMemoryContextUpload,
    ) -> Result<Self, ScaffoldContractError>;
}

pub struct GpuClosedLoopMemoryBatchInput<'a> {
    class_id: BrainClassId,
    perception_upload: &'a GpuPerceptionUpload,
    members: Vec<GpuClosedLoopMemoryTickInput<'a>>,
}

impl<'a> GpuClosedLoopMemoryBatchInput<'a> {
    pub fn try_new(
        class_id: BrainClassId,
        perception_upload: &'a GpuPerceptionUpload,
        members: Vec<GpuClosedLoopMemoryTickInput<'a>>,
    ) -> Result<Self, ScaffoldContractError>;
}

impl GpuClosedLoopBackend {
    pub fn tick_memory_batch(
        &mut self,
        batch: &GpuClosedLoopMemoryBatchInput<'_>,
    ) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuPerceptionFrameBinding {
    pub perception_header_index: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub tick: Tick,
    pub candidate_count: u16,
    pub base_frame_digest: PerceptionBaseDigest,
    pub context_digest: PerceptionContextDigest,
    pub final_frame_digest: PerceptionFrameDigest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuMemoryContextDispatchReceipt {
    pub slot: u32,
    pub slot_generation: u32,
    pub perception_header_index: u32,
    pub base_frame_digest: PerceptionBaseDigest,
    pub context_digest: PerceptionContextDigest,
    pub final_frame_digest: PerceptionFrameDigest,
    pub candidate_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MemoryChannelPlan {
    schema_version: u16,
    target_latent_lane_start: u32,
    family_value_lane_start: u32,
    decoder_input_stride: u32,
    max_candidate_gain: f32,
    memory_decoder_synapse_count: u32,
    canonical_digest: [u64; 4],
}

impl MemoryChannelPlan {
    pub fn try_new_v1() -> Result<Self, ScaffoldContractError>;
    pub const fn schema_version(&self) -> u16;
    pub const fn target_latent_lane_start(&self) -> u32;
    pub const fn family_value_lane_start(&self) -> u32;
    pub const fn decoder_input_stride(&self) -> u32;
    pub const fn max_candidate_gain(&self) -> f32;
    pub const fn memory_decoder_synapse_count(&self) -> u32;
    pub const fn canonical_digest(&self) -> [u64; 4];
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError>;
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuMemoryChannelPlan {
    pub schema_version: u32,
    pub target_latent_lane_start: u32,
    pub family_value_lane_start: u32,
    pub decoder_input_stride: u32,
    pub max_candidate_gain: f32,
    pub memory_decoder_synapse_count: u32,
    pub reserved: [u32; 2],
}
```

Compile `target_latent_lane_start=CANDIDATE_FEATURE_COUNT` (24),
`family_value_lane_start=32`, `decoder_input_stride=36`,
`max_candidate_gain=0.50`, and exactly `8 * (8 + 4) = 96` memory-decoder
synapses for N512/N1024/N2048. Use Slice B's exact
`CompiledSynapseKind::Decoder { head: DecoderHeadKind::MemoryContext, family,
input_lane }`; lanes are 24..31 for target latent and 32..35 for family value.
Each family has one synapse for every memory lane. Allocate them in
the same phenotype-owned global genetic/lifetime/alpha/fast/eligibility SoA
pools and include them in Slice B's contiguous decoder-synapse span; do not
create a memory-weight store.

Slice C extends Slice A's private `CandidateDecoderPlan` with private
`memory_channel: Option<MemoryChannelPlan>` and a read-only
`memory_channel()` accessor. `None` is the Slice A layout with flattened input
stride 24 and zero memory-decoder synapses; `Some(v1)` requires stride 36 and
the exact 96-row head above. `MemoryChannelPlan` does not derive unchecked
deserialization: its private wire DTO is rebuilt through `try_new_v1` and all
fields plus its digest must match the one canonical v1 tuple. Update
`CandidateDecoderPlan::canonical_digest`, `BrainPhenotype` canonical rehash and
custom deserializer, compiler-input equality, immutable phenotype asset, and
backend insertion validation to cover the optional plan and every added
memory-decoder synapse. No app or GPU caller mutates a compiled phenotype to
enable memory.

Populate Slice B's existing `GpuDecoderEligibilityMetadata.decoder_head` with
`MemoryContext` and its `input_lane` with the flattened lane above. Populate
the existing 80-byte `GpuBrainSlotExtensionRecord` fields
`memory_plan_offset` and `memory_weight_map_offset`; the map stores global
slot-local synapse IDs in family-major, target-latent-then-family-value order.
Leave the record size and every field offset unchanged, and add a source test
proving no `GpuBrainMemoryExtensionRecord` or second extension pool exists.

The compiler validates channel indices, family coverage, receptor references,
the 96-synapse budget against
`BrainExecutionBudget::max_memory_decoder_synapses()`, candidate context count
against `max_memory_context_records()`, the global synapse budget, and all map
IDs. Require exact used `CompiledBudgets.global.decoder_input_lanes=36` and
validate it against `BrainExecutionBudget::max_decoder_input_lanes()` (64).
Memory plan,
decoder metadata, receptors, alpha, and initial genetic weights are part of the
canonical phenotype hash. A plan mutation must change the hash and budget.
Initial memory-context genetic weights come from the genome/phenotype compiler,
not from WGSL constants or a host score. The deterministic acceptance genome
explicitly maps target pain/danger positively to Avoid and negatively to Ingest
and maps family expected valence/success to the matching family. Mutation tests
must show these genes alter weights/hash/behavior; zeroing the head removes the
effect.
Populate Slice B `GpuLearningHeader.decoder_input_stride` from the slot's
compiled used-lane count (36 with memory, 24 without); the class-bucketed
allocation uses the capacity ceiling 64 and every dynamic offset is validated
against that physical stride.
Use Task 4's exact memory `PerceptionContextBlock` serialization. Only
`PreparedMemoryRecall::finalize` may supply that block while consuming a
`PerceptionFrameDraft`; a finalized frame is immutable.

- [ ] **Step 4: Implement bounded WGSL channels**

The single entry uses `global_invocation_id.y` to select the active dynamic
header, generation-check its Slice A `GpuBrainSlotRecord`, and resolve Slice
B's exact slot extension and the memory plan. Memory-context/candidate offsets
are dynamic; logit/weight bases are slot-owned. Dispatch a class-bucketed batch;
never bind a per-creature device or buffer set.

```wgsl
struct GpuCandidateMemoryRecord {
    @align(16) candidate_index: u32,
    target_confidence: f32,
    family_confidence: f32,
    source_counts_packed: u32,
    target_latent: array<f32, 8>,
    family_value: array<f32, 4>,
}

struct GpuMemoryContextHeader {
    @align(16) schema_version: u32,
    class_id: u32,
    slot: u32,
    slot_generation: u32,
    tick_lo: u32,
    tick_hi: u32,
    candidate_count: u32,
    memory_context_offset: u32,
    candidate_offset: u32,
    profile_id: u32,
    profile_schema_version: u32,
    sensory_abi_version: u32,
    brain_slot_index: u32,
    decoder_learning_input_offset: u32,
    perception_header_index: u32,
    reserved: u32,
}

struct GpuMemoryChannelPlan {
    @align(16) schema_version: u32,
    target_latent_lane_start: u32,
    family_value_lane_start: u32,
    decoder_input_stride: u32,
    max_candidate_gain: f32,
    memory_decoder_synapse_count: u32,
    reserved: vec2<u32>,
}

@compute @workgroup_size(32)
fn add_candidate_memory_context(@builtin(global_invocation_id) gid: vec3<u32>) {
    let header = memory_headers[gid.y];
    let brain = brain_slots[header.brain_slot_index];
    if (brain.slot != header.slot || brain.slot_generation != header.slot_generation) { return; }
    let extension = slot_extensions[brain.extension_record_offset];
    let learning_state = load_slot_learning_state(extension.learning_state_offset);
    let memory_plan = memory_plans[extension.memory_plan_offset];
    let candidate = gid.x;
    if (candidate >= header.candidate_count) { return; }
    let candidate_record = candidates[header.candidate_offset + candidate];
    let family = candidate_record.family;
    let context = memory_context[header.memory_context_offset + candidate];
    let channel_count = memory_plan.decoder_input_stride
        - memory_plan.target_latent_lane_start;
    var delta = 0.0;
    for (var channel = 0u; channel < channel_count; channel++) {
        let map_index = family * channel_count + channel;
        let local_synapse = memory_weight_indices[
            extension.memory_weight_map_offset + map_index
        ];
        var sample = 0.0;
        if (channel < 8u) {
            sample = context.target_latent[channel]
                * clamp(context.target_confidence, 0.0, 1.0);
        } else {
            let family_index = channel - 8u;
            sample = context.family_value[family_index]
                * clamp(context.family_confidence, 0.0, 1.0);
        }
        let input_lane = memory_plan.target_latent_lane_start + channel;
        decoder_learning_inputs[
            header.decoder_learning_input_offset
            + candidate * memory_plan.decoder_input_stride
            + input_lane
        ] = sample;
        let active_lifetime_base = select(
            brain.lifetime_weight_offset,
            extension.lifetime_bank_1_offset,
            learning_state.active_weight_bank == 1u,
        );
        let active_fast_base = select(
            brain.fast_weight_offset,
            extension.fast_bank_1_offset,
            learning_state.active_weight_bank == 1u,
        );
        let effective = genetic[brain.genetic_weight_offset + local_synapse]
            + lifetime[active_lifetime_base + local_synapse]
            + alpha[brain.alpha_offset + local_synapse]
                * fast[active_fast_base + local_synapse];
        delta += sample * effective;
    }
    let logit_index = brain.candidate_logit_offset + candidate;
    candidate_logits[logit_index] += clamp(
        delta,
        -memory_plan.max_candidate_gain,
        memory_plan.max_candidate_gain,
    );
}
```

This shader reuses Slice B's exact `GpuSlotLearningStateRecord` and generated
heap loader through `extension.learning_state_offset`; it does not assume the
removed selector fields exist in the extension. Extend the WGSL/Naga and
selector-flip source tests so memory-context logits read the same active
lifetime/fast bank as recurrent and action decode, and reject a layout-version
or learning-state offset mismatch before dispatch.
`GpuCandidateMemoryRecord`, `GpuMemoryContextHeader`, and
`GpuMemoryChannelPlan` are declared once in the modified shared
`closed_loop_abi.wgsl` prefix; the entry-point file contains only memory helper
and compute code and cannot redefine those records. The combined code block
above describes the assembled source, not duplicate declarations in the entry
file.

Pack target and family source counts into the low/high 16 bits of
`source_counts_packed` after
validating both counts. Dispatch `add_candidate_memory_context` after base
candidate decoding but before lateral inhibition/winner selection. There is no
memory dispatch before or during recurrent microsteps and no write to encoded
inputs or activations.

The same pass materializes each candidate's 12 confidence-weighted memory lanes
at offsets 24..35 in Slice B's common class-bucketed
`decoder_learning_inputs` row. Slice B's existing
`accumulate_decoder_eligibility` kernel reads the selected candidate's row by
`GpuDecoderEligibilityMetadata.input_lane`; `decoder_head=MemoryContext` and
exact family prove its persistent head identity. The existing receptor
metadata, staging eligibility generation, sealed outcome-credit validation,
three-factor fast update, and sleep/save assets then cover memory-context
decoder rows exactly like action-candidate decoder rows.
No C-specific learning kernel, pending receipt, weight pool, or checkpoint is
allowed.

Extend the one shared decoder-eligibility entry with the derivative associated
with each validated head. `ActionCandidate` retains Slice B's
`motor_activation * input_lane` local factor because its forward term is
`motor * feature * weight`. `MemoryContext` uses exactly `input_lane` because
`add_candidate_memory_context` contributes `input_lane * weight` directly to
the candidate logit. Any unknown head writes a typed diagnostic and makes the
dispatch ineligible for commit. Add a finite-difference hardware assertion for
both heads so forward math and accumulated eligibility cannot silently drift.

Reject mismatched slot/generation, candidate counts/indices, ticks, profiles,
ABIs, final-frame digest, non-finite rows, bad source-count packing, or out-of-
range gains before upload. Add a two-N512-slot hardware case with different
phenotype memory plans and context values; assert each header selects its own
plan, slot/generation-tagged selections match, all context/logit/eligibility
offsets remain isolated, and recurrent activation digests are unchanged by
context. Add a source scan that rejects `encode_memory_state`, memory writes to
`encoded_inputs`, and any second extension record.

`GpuMemoryContextUpload::try_from_finalized` receives the finalized frame,
`FinalizedMemoryRecall`, and matched Slice A dynamic perception-header index.
It requires base/context/final digests to match the frame and recall, requires
the context block's exact 16-lane rows to re-encode every host record, and
requires the referenced `GpuPerceptionHeader` to match slot/generation/tick/
candidate count. Extend Slice A's host-only `GpuPerceptionUpload` with one
`GpuPerceptionFrameBinding` per dynamic header; the binding at that exact index
must match all three digests plus the same scalar identity. The 64-byte POD
stays fixed and does not pretend to carry digests; full digest identity lives
in the two host wrappers and is copied into
`GpuMemoryContextDispatchReceipt`. Add that receipt as
`GpuClosedLoopTick.memory_context_binding`; sealing compares it with the tick's
existing final frame digest. Context rows from another finalized frame or A
header are rejected before command encoding.

- [ ] **Step 5: Make the prewritten ablation and specificity tests pass**

The product tick may read only `GpuSelectionRecord`; test setup may inspect the
uploaded host-side context records. Explicit save/acceptance boundaries may
compare GPU-provided state digests, but the waking product loop may not read
activation, eligibility, or weight buffers.

- [ ] **Step 6: Run parser, ABI, and hardware tests**

Run: `cargo test -p alife_gpu_backend --test closed_loop_memory_context`

Expected: parser and ABI tests pass.

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_memory_context -- --nocapture`

Expected: candidate-specific and ablation cases pass and print the Vulkan adapter identifier.

Run: `cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_eligibility --test closed_loop_fast_plasticity -- --nocapture`

Expected: the pre-existing Slice B tests remain green with the expanded decoder
metadata, and the memory-decoder case changes fast weights only after its exact
sealed outcome.

- [ ] **Step 7: Commit**

```powershell
git add crates/alife_core/src/phenotype.rs crates/alife_gpu_backend/src/closed_loop_memory.rs crates/alife_gpu_backend/src/closed_loop_buffers.rs crates/alife_gpu_backend/src/closed_loop_pipeline.rs crates/alife_gpu_backend/src/closed_loop_runtime.rs crates/alife_gpu_backend/src/closed_loop_learning.rs crates/alife_gpu_backend/src/lib.rs crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl crates/alife_gpu_backend/shaders/closed_loop_memory_context.wgsl crates/alife_gpu_backend/shaders/closed_loop_eligibility.wgsl crates/alife_gpu_backend/tests/closed_loop_memory_context.rs crates/alife_gpu_backend/tests/support/mod.rs
git commit -m "Route candidate memory through GPU channels"
```

### Task 7: Wire recall and observation into the sealed GPU live loop

**Files:**
- Create: `crates/alife_game_app/tests/gpu_memory_loop.rs`
- Modify: `crates/alife_game_app/src/app_shell.rs`
- Modify: `crates/alife_game_app/src/live_brain_bridge.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`

**Interfaces:**
- Consumes: `MemoryBank::recall_frame`, `GpuClosedLoopBackend::tick_batch` with a `GpuBrainHandle`, sealed patch learning from Slice B, and memory observation/compaction receipts.
- Produces: `PreparedGpuBrainFrame`, rich `PreparedClassBatchMember`,
  `GpuClosedLoopMemoryBatchInput`, strict live
  ordering, per-organism preparation/transaction dispositions,
  candidate-specific context upload, post-seal memory updates, and
  organism-owned sleep-boundary memory compaction.

- [ ] **Step 1: Write failing ordering and no-CPU-selection tests**

```rust
#[test]
fn memory_recall_precedes_the_one_gpu_dispatch_and_observation_follows_sealing() {
    let run = run_grounded_gpu_memory_encounter().unwrap();
    assert_eq!(
        run.events,
        [
            "world-frame",
            "memory-recall",
            "gpu-select",
            "world-execute",
            "patch-seal",
            "gpu-learn",
            "memory-observe",
        ]
    );
    assert_eq!(run.class_batch_submissions, 1);
    assert_eq!(run.gpu_selection_count, run.scheduled_brain_count);
    assert_eq!(run.cpu_action_selections, 0);
    assert_eq!(run.patch.decision().episodic_key.as_ref().unwrap(), &run.pre_dispatch_candidate_keys[run.selected_candidate]);
}

#[test]
fn memory_capacity_pressure_is_telemetry_not_a_tick_error() {
    let run = run_gpu_loop_with_memory_capacity(2, 64).unwrap();
    assert_eq!(run.completed_ticks, 64);
    assert!(run.memory_degradations > 0);
    assert_eq!(run.terminal_tick_errors, 0);
}

#[test]
fn two_same_class_brains_batch_once_and_keep_memory_context_isolated() {
    let run = run_two_brain_memory_batch().unwrap();
    assert_eq!(run.class_batch_submissions, 1);
    assert_eq!(run.gpu_selection_count, 2);
    assert_ne!(run.brains[0].memory_context_digest, run.brains[1].memory_context_digest);
    assert_eq!(run.cross_slot_writes, 0);
}

#[test]
fn rejected_memory_observation_does_not_rewrite_a_completed_transaction() {
    let run = run_invalid_post_seal_memory_observation().unwrap();
    assert!(run.patch.is_sealed());
    assert_eq!(run.world_actions_executed, 1);
    assert_eq!(run.memory_observation_rejections, 1);
    assert_eq!(run.terminal_tick_errors, 0);
}

#[test]
fn malformed_preparation_rejects_only_that_organism() {
    let run = run_two_brains_with_one_bad_recall_contract().unwrap();
    assert_eq!(run.preparation_rejections, vec![OrganismId(1)]);
    assert_eq!(run.brains[1].completed_ticks, 1);
    assert_eq!(run.brains[1].gpu_selection_count, 1);
}

#[test]
fn post_seal_learning_failure_is_honest_and_memory_still_observes() {
    let run = run_post_seal_learning_rejection().unwrap();
    assert!(run.patch.is_sealed());
    assert!(matches!(
        run.learning_disposition,
        PostSealLearningDisposition::Rejected { .. },
    ));
    assert_eq!(run.memory_observations, 1);
    assert_eq!(run.completed_transactions, 1);
}

#[test]
fn memory_compaction_cycle_is_replay_safe_across_restore() {
    let checkpoint = staged_memory_compaction_checkpoint();
    let first = resume_memory_compaction(checkpoint).unwrap();
    let second = resume_memory_compaction(first.saved_checkpoint.clone()).unwrap();
    assert_eq!(first.receipt, second.receipt);
    assert_eq!(first.compaction_commits, 1);
    assert_eq!(second.compaction_prepares, 0);
}

#[test]
fn compaction_ownership_survives_gpu_handle_reallocation() {
    let checkpoint = organism_owned_pending_compaction(OrganismId(7));
    let restored = restore_with_different_gpu_handle(checkpoint).unwrap();
    assert_eq!(restored.owner, OrganismId(7));
    assert_eq!(restored.compaction_commits, 1);
    assert_ne!(restored.old_handle, restored.new_handle);
}

#[test]
fn rich_batch_member_retains_frame_finalized_recall_and_exact_memory_upload() {
    let batch = prepared_two_brain_class_batch().unwrap();
    for member in &batch.members {
        member.prepared.memory_recall
            .validate_for_frame(&member.prepared.frame).unwrap();
        assert_eq!(member.memory_upload.base_frame_digest,
            member.prepared.memory_recall.base_frame_digest());
        assert_eq!(member.memory_upload.context_digest,
            member.prepared.memory_recall.context_digest());
        assert_eq!(member.memory_upload.final_frame_digest,
            member.prepared.memory_recall.final_frame_digest());
        assert_eq!(member.memory_upload.perception_header_index,
            member.perception_header_index);
    }
}

#[test]
fn duplicate_missing_or_unexpected_gpu_output_executes_no_world_command() {
    for corruption in batch_output_corruptions() {
        let run = run_corrupt_batch_output(corruption).unwrap();
        assert_eq!(run.world_actions_executed, 0);
        assert_eq!(run.completed_transactions, 0);
        assert!(run.batch_reconciliation_rejected);
        assert_eq!(run.pending_receipts_after_typed_cleanup, 0);
    }
}

#[test]
fn retained_learning_blocks_new_batches_and_escalates_after_three_retries() {
    let run = run_retained_learning_recovery_fixture(3).unwrap();
    assert_eq!(run.recovery_attempts, 3);
    assert_eq!(run.waking_dispatches_while_recovering, 0);
    assert_eq!(run.memory_observations, 1);
    assert_eq!(run.topology_observations, 1);
    assert_eq!(run.final_phase, SleepPhase::ForcedRecoverySleep);
}
```

- [ ] **Step 2: Run and verify the live ordering is incomplete**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_memory_loop -j 1`

Expected: failure because the live frame does not yet carry retrieval context or memory receipts.

- [ ] **Step 3: Implement the exact waking order**

```rust
let mut prepared = Vec::new();
for organism_id in scheduled_organisms {
    match self.prepare_gpu_brain_frame(organism_id, tick) {
        Ok(frame) => {
            self.telemetry.observe_memory_recall(frame.memory_recall.receipt());
            prepared.push(frame);
        }
        Err(error) => {
            self.telemetry.reject_brain_preparation(organism_id, error);
            continue;
        }
    }
}

for frames in group_prepared_by_class(prepared) {
    let class_group = match PreparedClassBatch::try_new(frames) {
        Ok(batch) => batch,
        Err(error) => {
            self.telemetry.reject_class_batch_preparation(error);
            continue;
        }
    };
    let batch_input = class_group.tick_input();
    let selections = match self.backend.tick_memory_batch(&batch_input) {
        Ok(selections) => selections,
        Err(error) => {
            for member in &class_group.members {
                self.telemetry.reject_gpu_batch_member(
                    member.prepared.organism_id,
                    &error,
                );
            }
            continue;
        }
    };
    let reconciled = match class_group.reconcile_outputs(selections) {
        Ok(reconciled) => reconciled,
        Err(failure) => {
            self.handle_atomic_batch_reconciliation_failure(failure);
            continue;
        }
    };
    for (prepared_frame, gpu_tick) in reconciled {
        match self.complete_selected_transaction(prepared_frame, gpu_tick) {
            Ok(receipt) => self.telemetry.complete_gpu_transaction(&receipt),
            Err(error) => {
                self.telemetry.abort_pre_seal_transaction(
                    prepared_frame.organism_id,
                    error,
                );
            }
        }
    }
}

fn prepare_gpu_brain_frame(
    &mut self,
    organism_id: OrganismId,
    tick: Tick,
) -> Result<PreparedGpuBrainFrame, GameAppShellError> {
    let owner = organism_id.raw();
    let handle = *self.handles.get(&owner)
        .ok_or(GameAppShellError::MissingBrainState)?;
    let draft = self.world.perception_frame_draft(
        organism_id,
        tick,
        self.profiles.get(&owner)
            .ok_or(GameAppShellError::MissingSensorProfile)?.profile,
        self.homeostasis.get(&owner)
            .ok_or(GameAppShellError::MissingHomeostasis)?,
    )?;
    let prepared_recall = self.memories.get(&owner)
        .ok_or(GameAppShellError::MissingMemorySidecar)?
        .recall_frame(&draft)?;
    let (frame, memory_recall) = prepared_recall.finalize(draft)?;
    memory_recall.validate_for_frame(&frame)?;
    Ok(PreparedGpuBrainFrame {
        organism_id,
        handle,
        frame,
        memory_recall,
    })
}

fn complete_selected_transaction(
    &mut self,
    prepared: &PreparedGpuBrainFrame,
    gpu_tick: GpuClosedLoopTick,
) -> Result<CompletedGpuTransactionReceipt, GameAppShellError> {
    let owner = prepared.organism_id.raw();
    if gpu_tick.base_digest != prepared.memory_recall.base_frame_digest()
        || gpu_tick.frame_digest != prepared.memory_recall.final_frame_digest()
        || gpu_tick.memory_context_binding.base_frame_digest
            != prepared.memory_recall.base_frame_digest()
        || gpu_tick.memory_context_binding.context_digest
            != prepared.memory_recall.context_digest()
        || gpu_tick.memory_context_binding.final_frame_digest
            != prepared.memory_recall.final_frame_digest()
    {
        return Err(self.discard_before_seal(
            &gpu_tick,
            GameAppShellError::FinalFrameDigestMismatch,
        ));
    }
    let selected_index = gpu_tick.selection.candidate_index as usize;
    let selected = match prepared.frame.candidates().get(selected_index) {
        Some(candidate) => candidate,
        None => {
            return Err(self.discard_before_seal(
                &gpu_tick,
                GameAppShellError::MissingSelectedCandidate,
            ));
        }
    };
    if prepared.memory_recall.candidate_keys().get(selected_index).is_none() {
        return Err(self.discard_before_seal(
            &gpu_tick,
            GameAppShellError::MissingMemoryQuery,
        ));
    }
    let command = match selected.to_command(
        prepared.organism_id,
        gpu_tick.selection.confidence,
    ) {
        Ok(command) => command,
        Err(error) => {
            return Err(self.discard_before_seal(&gpu_tick, error.into()));
        }
    };
    let execution = match self.world.apply_command(&command) {
        Ok(measured_outcome) => measured_outcome,
        Err(error) => {
            return Err(self.discard_before_seal(&gpu_tick, error.into()));
        }
    };
    let patch = match self.seal_neural_patch(
        &prepared.frame,
        &gpu_tick,
        selected,
        execution,
        &prepared.memory_recall,
        selected_index,
    ) {
        Ok(patch) => patch,
        Err(error) => {
            return Err(self.discard_before_seal(&gpu_tick, error));
        }
    };

    let learning = match self.backend.apply_sealed_outcome(gpu_tick.handle, &patch) {
        Ok(receipt) => PostSealLearningDisposition::Applied(receipt),
        Err(error) => {
            self.telemetry.reject_post_seal_learning(prepared.organism_id, &error);
            self.retained_learning.insert(owner, RetainedLearningRecovery {
                organism_id: prepared.organism_id,
                handle: gpu_tick.handle,
                pending: gpu_tick.pending_eligibility,
                sealed_patch: patch.clone(),
                attempts: 0,
                last_error_code: error.stable_code(),
            });
            PostSealLearningDisposition::Rejected {
                pending_eligibility: PendingEligibilityDisposition::RejectedRetainedForRecovery,
                error_code: error.stable_code(),
            }
        }
    };
    let memory = match self.memories.get_mut(&owner) {
        Some(sidecar) => match sidecar.observe_sealed_patch(&patch) {
            Ok(receipt) => MemoryObservationDisposition::Applied(receipt),
            Err(error) => MemoryObservationDisposition::Rejected(error.stable_code()),
        },
        None => MemoryObservationDisposition::Rejected("missing-memory-sidecar"),
    };
    Ok(CompletedGpuTransactionReceipt {
        organism_id: prepared.organism_id,
        patch,
        learning,
        memory,
        recall: prepared.memory_recall.receipt().clone(),
    })
}
```

Define the focused app records rather than leaving batching helpers implicit:

```rust
struct PreparedGpuBrainFrame {
    organism_id: OrganismId,
    handle: GpuBrainHandle,
    frame: PerceptionFrame,
    memory_recall: FinalizedMemoryRecall,
}

struct PreparedClassBatchMember {
    prepared: PreparedGpuBrainFrame,
    perception_header_index: u32,
    memory_upload: GpuMemoryContextUpload,
}

struct PreparedClassBatch {
    class_id: BrainClassId,
    perception_upload: GpuPerceptionUpload,
    members: Vec<PreparedClassBatchMember>,
}

enum PendingEligibilityDisposition {
    Committed,
    RejectedRetainedForRecovery,
    DiscardedBeforeSeal(PendingEligibilityDiscardReceipt),
}

enum PreSealDiscardDisposition {
    Discarded(PendingEligibilityDiscardReceipt),
    Rejected {
        identity: PendingEligibilityIdentity,
        error_code: &'static str,
    },
}

struct PreSealTransactionFailure {
    cause_code: &'static str,
    discard: PreSealDiscardDisposition,
}

enum PostSealLearningDisposition {
    Applied(GpuLearningReceipt),
    Rejected {
        pending_eligibility: PendingEligibilityDisposition,
        error_code: &'static str,
    },
}

struct RetainedLearningRecovery {
    organism_id: OrganismId,
    handle: GpuBrainHandle,
    pending: PendingEligibilityReceipt,
    sealed_patch: ExperiencePatch,
    attempts: u8,
    last_error_code: &'static str,
}

enum MemoryObservationDisposition {
    Applied(MemoryUpdateReceipt),
    Rejected(&'static str),
}

struct CompletedGpuTransactionReceipt {
    organism_id: OrganismId,
    patch: ExperiencePatch,
    learning: PostSealLearningDisposition,
    memory: MemoryObservationDisposition,
    recall: MemoryRecallReceipt,
}

impl PreparedClassBatch {
    fn try_new(frames: Vec<PreparedGpuBrainFrame>)
        -> Result<Self, GameAppShellError>;
    fn tick_input(&self) -> GpuClosedLoopMemoryBatchInput<'_>;
    fn reconcile_outputs(
        &self,
        outputs: Vec<GpuClosedLoopTick>,
    ) -> Result<Vec<(&PreparedGpuBrainFrame, GpuClosedLoopTick)>, BatchReconciliationFailure>;
}
```

`PreparedClassBatch::try_new` stable-sorts the owned frames by
`(class_id, slot, generation)`, builds one Slice A `GpuPerceptionUpload`, then
builds each `GpuMemoryContextUpload::try_from_finalized` from the still-owned
`PerceptionFrame` and `FinalizedMemoryRecall` at that member's exact dynamic
header index. The rich backend input borrows all three; no `(handle, frame)`
projection may drop the finalized recall or its memory upload before dispatch.
Each member revalidates base/context/final digests, candidate count,
slot/generation/tick/profile, and perception-header identity when the batch is
encoded.
Both borrowed input types and `tick_memory_batch` are public
`alife_gpu_backend` APIs created in Task 6; the app only calls their validated
constructors. The backend never imports or names an app-owned batch type.

`reconcile_outputs` performs a complete validation pass before executing any
world command. It requires output count equal member count, unique input and
output handles, exactly one output per member, no unexpected handle, matching
class/slot/generation/phenotype, both frame digests, memory-context receipt,
candidate range, dispatch generation, and current shared hardware-receipt
generation. It then returns pairs in member order. A missing, duplicate,
unexpected, or mismatched output rejects the entire class batch atomically;
no member is partially executed. The failure carries every returned protected
pending-eligibility identity so `handle_atomic_batch_reconciliation_failure`
can request typed per-slot discards. The backend's batch API also guarantees
that an `Err` leaves no newly pending receipt; partial staging is rolled back
inside the backend before returning.

`group_prepared_by_class` stable-sorts by `(class_id, slot, generation)`.
`seal_neural_patch` is a focused app method that builds `PreActionSnapshot`,
calls `DecisionSnapshot::from_neural_selection`, attaches the exact retained
episodic key with `with_finalized_memory_recall` using the still-owned recall
and selected index, records the measured outcome through
`ExperiencePatchBuilder`, and returns only a sealed patch.

The CPU lookup selects the finalized key/context row by the GPU-returned
candidate index only after dispatch; it never compares logits or substitutes a
candidate. `GpuClosedLoopTick.frame_digest` must equal the finalized full digest
before world execution. Capacity pressure during recall returns validated
candidate-local zero rows plus exact degradation/statistics receipts, which
remain stored in `PreparedGpuBrainFrame` and flow into the completed
transaction receipt. True frame/profile/query errors reject only that
organism's pre-dispatch preparation; the outer scheduler never uses `?` to
abort unrelated organisms.

Slice C requires `GpuClosedLoopTick` to carry Slice B's complete protected
`PendingEligibilityReceipt` (not a reconstructed subset). The app derives
the private `PendingEligibilityIdentity` reference from that receipt and calls
`discard_pending_eligibility(handle, &identity)`. The backend compares every
field shown above with the private pending receipt before clearing staging.
A mismatch leaves both eligibility generations and the pending receipt intact
and returns a typed `PendingEligibilityDiscardError`; the app wraps the
original transaction cause plus `PreSealDiscardDisposition` and never uses `?`
to hide a failed discard.

Any failure after GPU selection but before patch sealing explicitly discards
that dispatch generation's staging eligibility. Ordinary world illegality is a
measured outcome, seals normally, and is not a discard. Once the patch seals,
the world transaction is complete: GPU learning and memory observation each
produce an explicit disposition and no post-seal `?` can rewrite or hide it. A learning rejection retains the exact pending
eligibility for the concrete bounded retry/recovery path below and prevents
that organism's next waking dispatch until resolved; it does not prevent memory
from observing the sealed patch. Missing memory state is reported honestly
rather than fabricated as a receipt. The memory owner map is `BTreeMap<u64, _>` keyed by
`OrganismId::raw()`. Task 8 extends the same post-seal disposition pattern to
topology without changing this transaction ordering.

Define `MAX_RETAINED_LEARNING_RETRIES = 3` and one organism-owned
`RetainedLearningRecovery` containing the sealed patch, complete Slice B
pending-eligibility identity, original GPU handle identity, attempt count, and
last typed error. Before grouping a waking frame, the scheduler excludes every
owner with a recovery entry and performs at most one replay-protected
`apply_sealed_outcome` retry for that owner in the current scheduler tick.
Success (or a typed `AlreadyCommitted` whose full replay key equals the saved
patch) records the learning receipt and removes the entry without re-observing
memory/topology. A transient failure increments the checked attempt count.
At three failures the organism enters `ForcedRecoverySleep`, checkpoints the
pending eligibility plus sealed transaction using Slice B's save contract, and
remains excluded from waking batches until restore/recovery resolves it. A
permanent identity mismatch is immediately forced into the same recovery state.
The entry is persisted in Task 9; it is never discarded merely to unblock a
new tick, and recovery work is bounded to one attempt per owner per scheduler
tick.

- [ ] **Step 4: Compact memory at the existing exactly-once sleep boundary**

The app owns one Task 5 `MemorySidecarState` per organism raw ID, never per GPU
handle. After a successful unique-cycle GPU consolidation receipt, transition
Idle/Committed to Pending with organism/cycle/input generation+digest/policy/
target capacity, compact into a separate staged bank, persist active and staged
assets at the save boundary, validate Staged, atomically swap, then record
Committed. Reallocation of the organism's GPU slot/handle cannot change or
restart this state.

A retry/restore of the same committed cycle returns the exact recorded receipt
without compacting or swapping again. Pending is restartable from the validated
active bank. Staged is restartable only from validated active-input plus staged-
output assets. A failed staging validation leaves the active bank/generation/
digest untouched. Task 9 defines every phase-to-asset restore case. Do not put
CPU sidecar state in `GpuSleepConsolidationReceipt`.

- [ ] **Step 5: Run live, learning, and sleep tests**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_memory_loop --test gpu_learning_loop --test automatic_gpu_sleep -j 1`

Expected: pass.

Run: `rg -n "cpu_shadow|AutoWithCpuFallback|CpuReference|bias_proposals|memory_delta" crates/alife_game_app/src/live_brain_bridge.rs crates/alife_game_app/src/gpu_live_runtime.rs`

Expected: no matches.

- [ ] **Step 6: Commit**

```powershell
git add crates/alife_game_app/src/app_shell.rs crates/alife_game_app/src/live_brain_bridge.rs crates/alife_game_app/src/gpu_live_runtime.rs crates/alife_game_app/tests/gpu_memory_loop.rs
git commit -m "Integrate memory with the sealed GPU loop"
```

### Task 8: Convert topology into a nonfatal tracked-object diagnostic sidecar

**Files:**
- Create: `crates/alife_core/tests/topology_sidecar_degradation.rs`
- Modify: `crates/alife_core/src/topology.rs`
- Modify: `crates/alife_core/src/lib.rs`
- Modify: `crates/alife_game_app/src/live_brain_bridge.rs`
- Modify: `crates/alife_game_app/tests/gpu_memory_loop.rs`

**Interfaces:**
- Consumes: sealed patch `EpisodicDecisionKeyV2` and its optional `TrackedObjectId`.
- Produces: organism-owned `TopologySidecar`, exact
  `TopologyObservationReceipt`, `TopologyDegradationKind`, private atomic
  planning internals in `topology.rs`, bounded counters, and diagnostics-only
  per-organism app wiring.

- [ ] **Step 1: Write failing saturation and action-boundary tests**

```rust
#[test]
fn ten_thousand_topology_observations_stay_bounded_and_never_return_capacity_error() {
    let mut sidecar = TopologySidecar::new(OrganismId(7), TopologicalMapConfig {
        max_concepts: 2,
        max_edges: 2,
        max_simplexes: 2,
        max_unresolved_gaps: 1,
        ..TopologicalMapConfig::default()
    }).unwrap();
    let mut degraded = 0;
    for index in 0..10_240_u64 {
        let receipt = sidecar.observe_sealed_patch(&tracked_patch(index));
        degraded += u64::from(!receipt.degradations.is_empty());
        assert!(receipt.after_counts.within(&sidecar.config()));
    }
    assert!(degraded > 0);
    assert_eq!(sidecar.diagnostics().terminal_errors, 0);
}

#[test]
fn topology_receipt_has_no_action_or_score_output() {
    let source = include_str!("../src/topology.rs");
    assert!(!source.contains("ActionCommand"));
    assert!(!source.contains("score_delta"));
    assert!(!source.contains("candidate_logits"));
}

#[test]
fn topology_sidecars_are_owned_and_isolated_by_organism() {
    let mut sidecars = BTreeMap::<u64, TopologySidecar>::new();
    sidecars.insert(7, TopologySidecar::new(OrganismId(7), tiny_config()).unwrap());
    sidecars.insert(9, TopologySidecar::new(OrganismId(9), tiny_config()).unwrap());
    let before_nine = sidecars[&9].diagnostics().canonical_digest;
    let receipt = sidecars.get_mut(&7).unwrap().observe_sealed_patch(&tracked_patch_for(7));
    assert_eq!(receipt.organism_id_raw, 7);
    assert_eq!(sidecars[&9].diagnostics().canonical_digest, before_nine);
    assert!(sidecars.get_mut(&7).unwrap().observe_sealed_patch(&tracked_patch_for(9)).rejected_invalid);
}

#[test]
fn post_seal_learning_rejection_still_updates_memory_and_topology() {
    let run = run_post_seal_learning_rejection().unwrap();
    assert!(run.patch.is_sealed());
    assert!(matches!(
        run.learning_disposition,
        PostSealLearningDisposition::Rejected { .. },
    ));
    assert_eq!(run.memory_observations, 1);
    assert_eq!(run.topology_observations, 1);
    assert_eq!(run.completed_transactions, 1);
}

#[test]
fn topology_rejects_duplicate_and_out_of_order_sealed_sequences_atomically() {
    let mut sidecar = TopologySidecar::new(OrganismId(7), tiny_config()).unwrap();
    let patch = tracked_patch_with_sequence(7, 9);
    let first = sidecar.observe_sealed_patch(&patch);
    let duplicate = sidecar.observe_sealed_patch(&patch);
    let stale = sidecar.observe_sealed_patch(&tracked_patch_with_sequence(7, 8));
    assert!(!first.replay_rejected);
    assert!(duplicate.replay_rejected);
    assert!(stale.replay_rejected);
    assert_eq!(duplicate.before_digest, duplicate.after_digest);
    assert_eq!(stale.before_digest, stale.after_digest);
}
```

Also prewrite an invalid-observation atomicity test that records the canonical
digest/counts before observation and requires them unchanged afterward. Add a
source boundary asserting `ConceptSignature`, `primary_signature`, and
`bindings_from_patch` contain no `WorldEntityId` object path and consume the
sealed episodic tracked binding. Place the post-seal learning-rejection test in
`crates/alife_game_app/tests/gpu_memory_loop.rs`; the other Task 8 tests live in
the core test target.

- [ ] **Step 2: Run and confirm current capacity errors fail the test**

Run: `cargo test -p alife_core --test topology_sidecar_degradation`

Expected: compile failure for missing sidecar API; the migrated legacy saturation test would otherwise return `TopologyCapacityExceeded`.

- [ ] **Step 3: Define nonfatal receipts**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologyDegradationKind {
    ConceptMergedIntoSummary,
    EdgeEvicted,
    SimplexReplaced,
    GapReplaced,
    PrimaryBindingTruncated,
    ActionBindingTruncated,
    InvalidObservationRejected,
    ReplayRejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologyObservationReceipt {
    pub organism_id_raw: u64,
    pub sealed_sequence_id: ExperienceSequenceId,
    pub update: Option<TopologyUpdate>,
    pub degradations: Vec<TopologyDegradationKind>,
    pub before_counts: TopologyCounts,
    pub after_counts: TopologyCounts,
    pub before_next_ids: TopologyIdCounters,
    pub after_next_ids: TopologyIdCounters,
    pub before_digest: [u64; 4],
    pub after_digest: [u64; 4],
    pub rejected_invalid: bool,
    pub replay_rejected: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyCounts {
    pub concepts: u32,
    pub edges: u32,
    pub simplexes: u32,
    pub unresolved_gaps: u32,
}

impl TopologyCounts {
    pub fn within(self, config: &TopologicalMapConfig) -> bool;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyIdCounters {
    pub next_concept_id: u64,
    pub next_edge_id: u64,
    pub next_simplex_id: u64,
    pub next_gap_id: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySidecarDiagnostics {
    pub organism_id_raw: u64,
    pub observations: u64,
    pub degradations: u64,
    pub invalid_rejections: u64,
    pub replay_rejections: u64,
    pub terminal_errors: u64,
    pub canonical_digest: [u64; 4],
}

pub struct TopologySidecar {
    organism_id: OrganismId,
    map: TopologicalMap,
    diagnostics: TopologySidecarDiagnostics,
    last_observed_sequence_id: Option<ExperienceSequenceId>,
    last_observed_key_digest: Option<[u64; 4]>,
}

#[derive(Debug, Clone, PartialEq)]
struct TopologyObservationBindings {
    primary_bindings: ConceptBindings,
    action_bindings: ConceptBindings,
}

#[derive(Debug, Clone, PartialEq)]
enum TopologyReplacement {
    Concept {
        index: u32,
        expected_id: Option<ConceptCellId>,
        value: ConceptCell,
    },
    Edge {
        index: u32,
        expected_id: Option<CognitiveEdgeId>,
        value: CognitiveEdge,
    },
    Simplex {
        index: u32,
        expected_id: Option<CognitiveSimplexId>,
        value: CognitiveSimplex,
    },
    Gap {
        index: u32,
        expected_id: Option<UnresolvedGapId>,
        value: UnresolvedGap,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct TopologyMutationPlan {
    expected_digest: [u64; 4],
    final_digest: [u64; 4],
    expected_counts: TopologyCounts,
    final_counts: TopologyCounts,
    expected_next_ids: TopologyIdCounters,
    final_next_ids: TopologyIdCounters,
    primary_signature: ConceptSignature,
    action_signature: ConceptSignature,
    primary_bindings: ConceptBindings,
    action_bindings: ConceptBindings,
    replacements: Vec<TopologyReplacement>,
    degradations: Vec<TopologyDegradationKind>,
    update: TopologyUpdate,
}

impl TopologySidecar {
    pub fn new(
        organism_id: OrganismId,
        config: TopologicalMapConfig,
    ) -> Result<Self, ScaffoldContractError>;
    pub fn observe_sealed_patch(&mut self, patch: &ExperiencePatch) -> TopologyObservationReceipt;
    pub fn config(&self) -> &TopologicalMapConfig;
    pub fn counts(&self) -> TopologyCounts;
    pub fn next_ids(&self) -> TopologyIdCounters;
    pub fn diagnostics(&self) -> TopologySidecarDiagnostics;
}
```

Define `TopologySidecar`, `TopologyObservationBindings`,
`TopologyReplacement`, and `TopologyMutationPlan` in `topology.rs`; do not put
planning/replacement state in a sibling wrapper module. The sidecar validates
that patch organism belongs to its private `organism_id` and applies a private
monotonic replay guard before asking its map to plan. The guard binds
`(sealed_sequence_id, episodic_key.canonical_digest())`. A sequence lower than
the last committed sequence, the same sequence with a different key, or an
exact duplicate returns `ReplayRejected`, sets `replay_rejected`, and leaves
the map, IDs, counts, and canonical digest unchanged. Only a successfully
committed observation advances the guard. Task 9 persists and validates both
guard fields so restore cannot apply the same sealed patch twice.

Each replacement is an append when `index == current_len` and `expected_id` is
`None`, or an in-place replacement when the indexed current ID equals
`expected_id`. Stable operation rank is Concept, Edge, Simplex, Gap; sort by
`(rank, index)` and reject duplicate targets. Plan validation checks the
expected digest/counts/all four next-ID counters, every referenced final ID,
all capacities, separate primary/action bindings, exact degradations, final
counts/counters, and the canonical final digest before mutation. The subsequent
commit only applies the prevalidated replacements in order and assigns the
precomputed update/counts/counters/digest; it has no fallible allocation or
lookup. Appending consumes exactly the relevant preplanned next ID; replacement
does not rewind or reuse any ID.
New maps initialize all four counters to 1. Every append uses the current
nonzero counter and advances with `checked_add`; exhaustion is recorded as an
invalid rejected observation with no mutation. Eviction/replacement never
decrements a counter, and restore requires each counter to be strictly greater
than every surviving ID in its domain.

The method has no `Result` return. It validates sealed input, captures invalid
observations as `InvalidObservationRejected`, and returns an exact receipt whose
before/after digest, counts, and counters prove atomicity. Rejected input keeps
all before/after fields equal. It never exposes an action, action hint, or scalar
score.

- [ ] **Step 4: Implement deterministic capacity policies**

Replace `ConceptBindings.objects: Vec<WorldEntityId>` with
`Vec<TrackedObjectId>` and add `ConceptCell::is_summary`. Replace private
`ConceptSignature::Object(WorldEntityId)` with
`TrackedObject(TrackedObjectId)`; action signatures use
`CandidateActionFamily` plus action ID. Rewrite `primary_signature` and
`bindings_from_patch` to produce `TopologyObservationBindings` with independent
primary/action vectors and consume only the selected sealed
`EpisodicDecisionKeyV2` for object identity. Never add selected/outcome target
entities, token source entities, or social body entities to object bindings;
word IDs and `OrganismId` agent bindings remain separate non-object concepts.

Build and fully validate a `TopologyMutationPlan` without mutating the map.
The plan chooses every insert/merge/replacement and final ID first; commit it
in one infallible step. Invalid/late capacity cases return a rejection receipt
with the pre-observation digest unchanged.

Remove the old public `TopologicalMap::apply_patch` entry point. If a legacy
unit test still needs direct map mutation during migration, expose it only as
`#[cfg(test)] pub(crate) fn apply_patch_legacy_test_only`; production callers
can mutate topology only through `TopologySidecar::observe_sealed_patch`, and
the legacy helper delegates to the same plan/validate/commit path.

Existing tracked signatures merge normally. At concept capacity, merge the
new observation into the minimum `(salience_q16, last_tick, concept_id)`
concept after clearing identity-specific refs and set `is_summary = true`. At
edge capacity, evict the minimum `(strength_q16, last_tick, edge_id)` edge.
Replace the oldest simplex by `(tick, simplex_id)`. Replace the minimum
`(curiosity_q16, salience_q16, last_tick, gap_id)` gap. For each 32-entry
binding vector, insert, stable-sort, deduplicate, retain the first 32, and emit
the matching `PrimaryBindingTruncated` or `ActionBindingTruncated` receipt.
Require every configured topology capacity to be at least one so each
replacement policy has a defined target.

- [ ] **Step 5: Wire the sidecar after the causal transaction without `?` propagation**

```rust
let owner = patch.organism_id().raw();
let topology_disposition = match self.topologies.get_mut(&owner) {
    Some(sidecar) => TopologyObservationDisposition::Observed(
        sidecar.observe_sealed_patch(&patch),
    ),
    None => TopologyObservationDisposition::RejectedMissingOwner,
};
self.telemetry.observe_topology(topology_disposition);
```

The app field is `BTreeMap<u64, TopologySidecar>` and creation/restore requires
the map key, sidecar organism ID, and saved owner ID to match. There is no
singular `self.topology` shared across organisms.

Insert this observation immediately after Task 7's memory disposition for
every sealed patch, regardless of whether the learning disposition was Applied
or Rejected. Extend `CompletedGpuTransactionReceipt` with
`topology: TopologyObservationDisposition`, and update the Task 7 ordering test
to append `"topology-observe"` after `"memory-observe"`. Brain creation/load
preflights a matching topology owner, but a post-seal missing-owner invariant
violation remains an explicit rejected disposition rather than `?` propagation.

Do not feed `curiosity_biases`, topology salience, or receipt fields into `PerceptionFrame::episodic_context`, candidate features, GPU logits, or winner selection. Topology is diagnostic-only in Slice C.

- [ ] **Step 6: Run topology and live-loop tests**

Run: `cargo test -p alife_core --test topology_sidecar_degradation --test topological_map`

Expected: all migrated tests pass and saturation receipts are deterministic.

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_memory_loop -j 1`

Expected: topology degradation does not reduce completed GPU ticks.

- [ ] **Step 7: Commit**

```powershell
git add crates/alife_core/src/topology.rs crates/alife_core/src/lib.rs crates/alife_core/tests/topology_sidecar_degradation.rs crates/alife_core/tests/topological_map.rs crates/alife_game_app/src/live_brain_bridge.rs crates/alife_game_app/tests/gpu_memory_loop.rs
git commit -m "Make topology a nonfatal diagnostic sidecar"
```

### Task 9: Persist and report profile, memory, topology, and tracked-object provenance

**Files:**
- Create: `crates/alife_world/tests/gpu_memory_grounding_persistence.rs`
- Modify: `crates/alife_game_app/tests/gpu_sleep_restore.rs`
- Modify: `crates/alife_core/tests/experience_three_phase.rs`
- Modify: `crates/alife_core/tests/packed_experience_logging.rs`
- Modify: `crates/alife_core/src/experience.rs`
- Modify: `crates/alife_core/src/packed_log.rs`
- Modify: `crates/alife_world/src/persistence.rs`
- Modify: `crates/alife_game_app/src/gpu_live_runtime.rs`
- Modify: `crates/alife_game_app/src/gpu_evidence.rs`
- Modify: `crates/alife_game_app/src/bin/alife_game_app.rs`

**Interfaces:**
- Consumes: Task 2 profile provenance, Task 5 memory diagnostics, Task 8 topology diagnostics, and Slice B `GpuBrainSaveState`.
- Consumes additionally: Slice A shared `GpuSliceEvidenceHeader`,
  `PhenotypeEvidenceManifest`, `BrainCapacityClass::canonical_digest`, and
  validating evidence loader.
- Produces: profile-labelled patches/logs/saves/receipts, Slice C support in
  the shared evidence loader, `MemorySidecarSaveSummary`,
  `TopologySidecarSaveSummary`, and `TrackedObjectSaveRecord`.

- [ ] **Step 1: Write failing provenance roundtrip tests**

```rust
#[test]
fn both_profiles_roundtrip_with_separate_sidecar_provenance() {
    for profile in [SensorProfile::PrivilegedAffordanceV1, SensorProfile::GroundedObjectSlotsV1] {
        let save = gpu_memory_grounding_save_fixture(profile);
        let json = serde_json::to_string(&save).unwrap();
        let loaded: GpuBrainSaveState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.sensor_profile.profile, profile);
        assert_eq!(loaded.memory.summary.profile.profile().unwrap(), profile);
        assert_eq!(loaded.topology.profile.profile().unwrap(), profile);
    }
}

#[test]
fn persistent_tracked_object_records_do_not_store_world_entity_ids() {
    let source = include_str!("../src/persistence.rs");
    let record = source.split("pub struct TrackedObjectSaveRecord").nth(1).unwrap();
    let record = record.split('}').next().unwrap();
    assert!(!record.contains("WorldEntityId"));
}

#[test]
fn committed_memory_compaction_roundtrips_and_is_not_dispatched_again() {
    let save = save_with_committed_memory_compaction();
    let loaded = roundtrip_gpu_brain_save(&save).unwrap();
    let resumed = resume_gpu_sleep_restore(loaded).unwrap();
    assert_eq!(resumed.memory_compaction_prepares, 0);
    assert_eq!(
        resumed.memory_compaction_receipt,
        save.memory.compaction.checkpoint.committed_receipt(),
    );
}

#[test]
fn tracker_roundtrip_preserves_next_id_and_reassociates_without_world_entity_id() {
    let save = tracker_save_fixture();
    let mut loaded = roundtrip_tracker(&save).unwrap();
    let existing_record = save.records[0].clone();
    assert_eq!(existing_record.tracking_provenance.canonical_key(), existing_record.tracking_key);
    let existing = loaded.observe(
        save.organism_id,
        existing_record.tracking_provenance,
        existing_record.stable_physical_descriptor,
        Tick::new(existing_record.last_seen_tick.raw() + 1),
    ).unwrap();
    let novel = loaded.observe(
        save.organism_id,
        novel_tracking_provenance(),
        stable_descriptor_fixture(0.75),
        Tick::new(existing_record.last_seen_tick.raw() + 2),
    ).unwrap();
    assert_eq!(existing.tracked_object_id, existing_record.tracked_object_id);
    assert_eq!(novel.tracked_object_id.raw(), save.next_id);
}

#[test]
fn tracker_restore_rejects_wrong_world_capacity_key_and_reused_next_id() {
    let save = tracker_save_fixture();
    for malformed in [
        with_wrong_tracker_world_seed(save.clone()),
        with_zero_tracker_capacity(save.clone()),
        with_noncanonical_tracking_key(save.clone()),
        with_next_id_equal_to_max_saved_id(save),
    ] {
        assert!(TrackedObjectRegistry::restore(malformed).is_err());
    }
}

#[test]
fn every_memory_compaction_asset_case_restores_exactly_once() {
    let idle = restore_memory_sidecar(idle_save_with_active_input()).unwrap();
    assert_eq!((idle.prepares, idle.swaps), (0, 0));
    let pending = restore_memory_sidecar(pending_save_with_active_input()).unwrap();
    assert_eq!((pending.prepares, pending.swaps), (1, 1));
    let staged_input = restore_memory_sidecar(staged_save_with_active_input_and_output_stage()).unwrap();
    assert_eq!((staged_input.prepares, staged_input.swaps), (0, 1));
    let staged_output = restore_memory_sidecar(staged_save_with_output_already_active()).unwrap();
    assert_eq!((staged_output.prepares, staged_output.swaps), (0, 0));
    let committed = restore_memory_sidecar(committed_save_with_active_output()).unwrap();
    assert_eq!((committed.prepares, committed.swaps), (0, 0));
    assert!(restore_memory_sidecar(staged_save_with_wrong_active_digest()).is_err());
    assert!(restore_memory_sidecar(staged_save_without_stage_asset()).is_err());
}

#[test]
fn compaction_cycles_are_monotonic_and_retries_bind_the_full_identity() {
    let mut state = memory_sidecar_with_next_cycle(4);
    let prepared = state.prepare_compaction(4, 32, 1).unwrap();
    assert_eq!(prepared.receipt.identity.cycle_id, 4);
    assert!(state.prepare_compaction(5, 32, 1).is_err());
    assert!(state.prepare_compaction(4, 31, 1).is_err());
    assert!(state.prepare_compaction(4, 32, 2).is_err());
    let committed = state.commit_prepared(prepared).unwrap();
    assert_eq!(state.retry_committed(4).unwrap(), committed);
    assert!(state.prepare_compaction(4, 32, 1).is_err());
    assert_eq!(state.checkpoint().next_cycle_id, 5);
}

#[test]
fn portable_sidecar_assets_are_primitive_digest_checked_and_handle_free() {
    let save = gpu_memory_grounding_save_fixture(SensorProfile::GroundedObjectSlotsV1);
    let assets = decode_portable_sidecar_assets(&save).unwrap();
    assert_eq!(assets.memory.organism_id_raw, save.organism_id.raw());
    assert_eq!(assets.topology.organism_id_raw, save.organism_id.raw());
    assert!(portable_sidecar_dto_source_has_no_runtime_handles_or_world_entities());
    assert!(decode_portable_sidecar_assets(&tamper_one_record_digest(save)).is_err());
}

#[test]
fn artifact_header_uses_exact_stable_slug_and_complete_provenance() {
    let (header, manifest) = profiled_evidence_header_fixture(GROUNDED_N512);
    assert_eq!(header.common.artifact_schema, 1);
    assert_eq!(header.common.slice_raw, 3);
    assert_eq!(header.common.status_raw, 1);
    assert_eq!(header.artifact_slug, "gpu-memory-grounding-slice-c-grounded-object-slots-v1-n512");
    assert_eq!(header.backend_api_slug, "gpu-closed-loop-v1");
    assert_eq!(header.common.class_id_raw, BrainCapacityClass::N512_ID.raw());
    assert_eq!(header.common.profile_id_raw, SensorProfile::GroundedObjectSlotsV1.raw());
    assert_eq!(header.common.profile_schema, 1);
    assert_ne!(header.common.phenotype_manifest_digest, [0; 4]);
    assert_ne!(header.common.capacity_digest, [0; 4]);
    assert_eq!(manifest.manifest_digest, header.common.phenotype_manifest_digest);
    assert_eq!(header.common.git_commit.len(), 40);
    assert_eq!(header.common.source_tree_digest.len(), 40);
    assert_ne!(header.common.artifact_digest, [0; 4]);
}

#[test]
fn plastic_memory_decoder_roundtrip_preserves_learning_and_phenotype_budget() {
    let before = learned_memory_decoder_save_fixture();
    let restored = restore_gpu_brain(roundtrip_gpu_brain_save(&before).unwrap()).unwrap();
    assert_eq!(restored.phenotype_hash(), before.phenotype_hash);
    assert_eq!(restored.memory_decoder_synapse_count(), 96);
    assert_eq!(restored.fast_weight_digest(), before.fast_weights.digest);
    assert_eq!(restored.eligibility_digest(), before.eligibility.digest);
    assert_eq!(restored.probe_selection(), before.expected_probe_selection);
}
```

In `experience_three_phase.rs`, prewrite legacy-v1 to profiled-v2 migration and
unknown-profile-ID rejection cases. In `packed_experience_logging.rs`, assert
the exact `sensor_profile_id` and `sensor_profile_schema_version` field offsets,
unchanged total packed-frame byte size (consume reserved bytes), stable IDs
1/2, and round-trip derivation from both profile patches.

Also write failing tests now that reject a grounded save loaded into a
privileged phenotype (and the reverse), and assert privileged and grounded
receipts are stored in distinct aggregate buckets keyed by full profile
identity (profile + profile schema + sensory ABI, excluding source tick).

- [ ] **Step 2: Run and verify save fields are absent**

Run: `cargo test -p alife_world --test gpu_memory_grounding_persistence`

Expected: compile failure for missing save summaries and profile fields.

Run: `cargo test -p alife_core --test experience_three_phase --test packed_experience_logging`

Expected: compile/assertion failure for missing profiled schema fields and
migration.

- [ ] **Step 3: Add portable save summaries**

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemorySidecarSaveSummary {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub profile: SensorProfileIdentity,
    pub capacity: u32,
    pub record_count: u32,
    pub merge_count: u64,
    pub eviction_count: u64,
    pub compaction_count: u64,
    pub active_generation: u64,
    pub active_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologySidecarSaveSummary {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub profile: SensorProfileIdentity,
    pub counts: TopologyCounts,
    pub next_concept_id_raw: u64,
    pub next_edge_id_raw: u64,
    pub next_simplex_id_raw: u64,
    pub next_gap_id_raw: u64,
    pub max_bindings_per_kind: u32,
    pub has_last_observation: bool,
    pub last_observed_sequence_id_raw: u64,
    pub last_observed_key_digest: [u64; 4],
    pub degradation_count: u64,
    pub replay_rejection_count: u64,
    pub canonical_digest: [u64; 4],
    pub summary_asset: GpuBrainAssetRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackedObjectSaveRecord {
    pub tracked_object_id: TrackedObjectId,
    pub tracking_provenance: PhysicalTrackingProvenance,
    pub tracking_key: PhysicalTrackingKey,
    pub last_seen_tick: Tick,
    pub stable_physical_descriptor: StablePhysicalDescriptor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackedObjectRegistrySaveState {
    pub schema_version: u16,
    pub world_seed: u64,
    pub organism_id: OrganismId,
    pub capacity: u32,
    pub next_id: u64,
    pub records: Vec<TrackedObjectSaveRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryCompactionSaveState {
    pub checkpoint: MemoryCompactionCheckpoint,
    pub active_bank_asset: GpuBrainAssetRef,
    pub staged_bank_asset: Option<GpuBrainAssetRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemorySidecarSaveState {
    pub summary: MemorySidecarSaveSummary,
    pub compaction: MemoryCompactionSaveState,
    pub retained_learning: Option<RetainedLearningRecoverySaveState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableMemoryRecordV2 {
    pub memory_id_raw: u64,
    pub organism_id_raw: u64,
    pub profile_id_raw: u16,
    pub profile_schema_version: u16,
    pub sensory_abi_version_raw: u16,
    pub query_version_raw: u16,
    pub tracked_object_id_raw: u64,
    pub family_raw: u16,
    pub other_action_id_raw: u32,
    pub target_bins: [i8; CANDIDATE_FEATURE_COUNT],
    pub query_digest: [u64; 4],
    pub query_features: [f32; MEMORY_QUERY_V2_FEATURE_COUNT],
    pub target_latent: [f32; MEMORY_LATENT_V1_COUNT],
    pub family_value: [f32; MEMORY_VALUE_V1_COUNT],
    pub confidence: f32,
    pub salience_q16: u16,
    pub observation_count: u32,
    pub first_tick_raw: u64,
    pub last_tick_raw: u64,
    pub sealed_sequence_id_raw: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableMemoryBankAssetV2 {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub profile_id_raw: u16,
    pub profile_schema_version: u16,
    pub sensory_abi_version_raw: u16,
    pub capacity: u32,
    pub generation: u64,
    pub next_memory_id_raw: u64,
    pub last_observed_sequence_id_raw: u64,
    pub merge_count: u64,
    pub eviction_count: u64,
    pub compaction_count: u64,
    pub records: Vec<PortableMemoryRecordV2>,
    pub canonical_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableTopologyBindingSetV1 {
    pub tracked_object_ids_raw: Vec<u64>,
    pub word_ids_raw: Vec<u32>,
    pub drive_ids_raw: Vec<u16>,
    pub action_ids_raw: Vec<u32>,
    pub location_bits: Vec<[u32; 3]>,
    pub agent_ids_raw: Vec<u64>,
    pub semantic_concept_ids_raw: Vec<u64>,
    pub cluster_ids_raw: Vec<u64>,
    pub affordance_bits_raw: u64,
    pub mean_valence_bits: u32,
    pub mean_prediction_error_bits: u32,
    pub emotion_observation_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableTopologyConceptV1 {
    pub id_raw: u64,
    pub is_summary: bool,
    pub bindings: PortableTopologyBindingSetV1,
    pub observation_count: u32,
    pub first_tick_raw: u64,
    pub last_tick_raw: u64,
    pub confidence_bits: u32,
    pub salience_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableTopologyEdgeV1 {
    pub id_raw: u64,
    pub from_raw: u64,
    pub to_raw: u64,
    pub relation_raw: u16,
    pub strength_bits: u32,
    pub evidence_count: u32,
    pub first_tick_raw: u64,
    pub last_tick_raw: u64,
    pub confidence_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableTopologySimplexV1 {
    pub id_raw: u64,
    pub concept_ids_raw: Vec<u64>,
    pub observation_count: u32,
    pub mean_valence_bits: u32,
    pub mean_prediction_error_bits: u32,
    pub salience_bits: u32,
    pub first_tick_raw: u64,
    pub last_tick_raw: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableTopologyGapV1 {
    pub id_raw: u64,
    pub source_concept_ids_raw: Vec<u64>,
    pub contradiction_raw: u16,
    pub prediction_error_bits: u32,
    pub curiosity_voltage_bits: u32,
    pub salience_bits: u32,
    pub first_tick_raw: u64,
    pub last_tick_raw: u64,
    pub confidence_bits: u32,
    pub status_raw: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableTopologySidecarAssetV1 {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub profile_id_raw: u16,
    pub profile_schema_version: u16,
    pub sensory_abi_version_raw: u16,
    pub max_concepts: u32,
    pub max_edges: u32,
    pub max_simplexes: u32,
    pub max_unresolved_gaps: u32,
    pub max_bindings_per_kind: u32,
    pub edge_decay_bits: u32,
    pub has_last_observation: bool,
    pub last_observed_sequence_id_raw: u64,
    pub last_observed_key_digest: [u64; 4],
    pub next_concept_id_raw: u64,
    pub next_edge_id_raw: u64,
    pub next_simplex_id_raw: u64,
    pub next_gap_id_raw: u64,
    pub concepts: Vec<PortableTopologyConceptV1>,
    pub edges: Vec<PortableTopologyEdgeV1>,
    pub simplexes: Vec<PortableTopologySimplexV1>,
    pub gaps: Vec<PortableTopologyGapV1>,
    pub observation_count: u64,
    pub degradation_count: u64,
    pub invalid_rejection_count: u64,
    pub replay_rejection_count: u64,
    pub canonical_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetainedLearningRecoverySaveState {
    pub organism_id_raw: u64,
    pub pending: PendingEligibilityCheckpoint,
    pub sealed_patch_asset: GpuBrainAssetRef,
    pub attempts: u8,
    pub last_error_code: String,
}
```

At the explicit save boundary,
`pending_checkpoint_from_receipt(&runtime.pending)` uses only Slice B's
read-only identity accessors and the validated world-persistence constructor;
the waking runtime retains the backend-owned receipt itself. A conversion
failure aborts the save before writing a manifest and never discards the live
pending eligibility. Restore allocates the new handle, reconstructs the
backend pending identity from the checkpoint plus outer organism/phenotype,
and requires exact equality before retry.

Add `sensor_profile`, `memory: MemorySidecarSaveState`, `topology`, and
`tracked_objects` to Slice B `GpuBrainSaveState`. Preserve every Slice B
mutable asset/digest/generation field. Store bulk banks behind digest-checked
asset refs. The existing lifetime, fast, eligibility, and sleep assets already
contain every plastic `DecoderHeadKind::MemoryContext` synapse; do not add a separate memory-
decoder weight asset. Restore validates the memory plan/hash/budget before
accepting those arrays. Reject a loaded
profile that differs from the saved phenotype/profile manifest; do not silently
translate privileged evidence into grounded evidence.
`PortableMemoryBankAssetV2` carries the complete top-level raw profile identity
even when `records` is empty. Its canonical digest covers those three fields,
and every nonempty record must repeat the same profile/query ABI. Restore
requires the asset profile to equal the save summary, phenotype sensor profile,
and manifest before constructing a bank; an empty bank from another profile is
therefore not interchangeable.
The bytes behind the memory/topology refs decode only into the exact primitive
DTOs above, never directly into private `MemoryBank`, `TopologySidecar`,
`ConceptCell`, or enum/newtype memory. Every enum uses an explicit validated raw
mapping; every float is finite and stores canonical `to_bits()` where the DTO
declares bits; every vector is sorted, unique where required, and bounded before
core state is constructed. Recompute query, record-identity, bank, binding,
topology, and asset digests from canonical little-endian fields and reject any
mismatch before rebuilding private indices. No DTO contains a GPU handle, slot,
adapter, `WorldEntityId`, process job ID, map iterator order, or serde-derived
digest.
Decode topology only after its asset owner, profile, capacities (including
`max_bindings_per_kind`), replay guard, counts, all four next-ID counters,
and canonical digest equal the summary. Reconstruct app owner maps as
`BTreeMap<u64, _>` and reject any key that differs from the memory/topology/
tracker organism ID. Require the saved last topology sequence/key pair to match
the last committed observation evidence and every surviving ID to be below its
saved next ID. GPU handle and slot values are freshly allocated restore
state and never appear in a sidecar asset.
The 15-value tracked descriptor contains color, material, shape, chemical,
temperature, and terrain only; it excludes observer-relative bearing,
distance, velocity, contact, and proprioception. Persist each organism's
schema/world seed/capacity/`next_id`, provenance, and canonical
`PhysicalTrackingKey` associations so load cannot reuse an ID or depend on raw
world entity values. Restore requires schema 1, matching world seed, capacity
in `1..=1024`, one owner, sorted unique keys/IDs, valid descriptors/ticks, every
provenance world seed equal to the registry world seed, every saved key equal
to `tracking_provenance.canonical_key()`, and `next_id` strictly greater than
every saved tracked ID with no overflow. Rebuild `records_by_key` only after
the complete DTO validates.

The `MemoryCompactionCheckpoint` nested in the save state is the portable
organism-owned `alife_core` type defined in Task 5. Decode and digest-check both
asset refs before constructing `MemorySidecarState`, then apply this exact
restore table:

- Idle: active asset equals checkpoint active generation/digest; no staged
  asset; perform zero prepare/swap operations.
- Pending: active asset equals pending input and checkpoint active; no staged
  asset; deterministically prepare once, validate output, swap once, commit.
- Staged with input still active: active asset equals staged input and staged
  asset equals output; swap once and commit without preparing again.
- Staged with output already active (crash after swap): active asset equals
  staged output and staged asset also validates that output; record Committed
  without preparing or swapping again.
- Committed: active asset equals committed output/active checkpoint; no staged
  asset; reuse the receipt with zero work.

Reject every other active/staged presence, generation, digest, owner, cycle, or
receipt combination before mutating state. Never mutate an active bank in place
or prepare/swap the same compaction cycle again.

- [ ] **Step 4: Carry provenance through patches, packed logs, and receipts**

Add `SensorProfileProvenance` to `ExperiencePatchHeader`, bump the Experience
and packed-log schemas with the prewritten migration tests, and encode
`SensorProfileId` plus profile schema in explicit reserved offsets of the fixed
packed frame. Every behavioral/benchmark JSON header must contain:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfiledBehaviorReceiptHeader {
    #[serde(flatten)]
    pub common: GpuSliceEvidenceHeader,
    pub artifact_slug: String,
    pub backend_api_version: u16,
    pub backend_api_slug: String,
    pub adapter_name: String,
    pub adapter_backend: String,
    pub run_seed: u64,
}

```

Use the shared constants `SLICE_C_RAW = 3` and `EVIDENCE_STATUS_PASS_RAW = 1`.
The common header is not redefined or copied into a parallel DTO. Its
`artifact_schema`, class/profile IDs, status, Git IDs, artifact digest,
phenotype hash/manifest digest, and capacity digest remain the exact top-level
JSON fields ingested by Slice D. Task 10's receipt flattens this header and
embeds one `PhenotypeEvidenceManifest`, which must match those common fields
and the compiled phenotype. The body
`SensorProfileIdentity` supplies sensory ABI provenance and must match the
common profile ID/schema.

The only v1 backend slug is `gpu-closed-loop-v1`. Profile slugs are exactly
`privileged-affordance-v1` and `grounded-object-slots-v1`; capacity slugs are
exactly `n512`, `n1024`, and `n2048`; Slice C artifact slugs are exactly
`gpu-memory-grounding-slice-c-{profile_slug}-{capacity_slug}`. Validate the
slug by deriving it from primitive identity/class fields, never accept an
arbitrary caller string. Shared `GpuSliceEvidenceHeader.artifact_digest` hashes
the header except that digest plus the complete embedded manifest and Slice C
body. Git commit/tree validation, canonical capacity digest, manifest digest,
domain tags/seeds, float normalization, and JSON-independent digesting all use
the shared `gpu_evidence` implementation. Require clean lowercase 40-hex Git
IDs, nonempty Vulkan adapter/backend fields, backend API version 1, passing
status, and nonzero shared digests before a receipt is promotion-eligible.
Extend the shared validating loader with slice 3 body validation; do not create
a C-only identity/digest loader.

- [ ] **Step 5: Implement mismatch rejection and separate reporting**

Make the prewritten mismatch and aggregate-bucket tests pass; no migration may
reinterpret privileged evidence as grounded evidence. Aggregate keys use the
validated artifact slug plus phenotype hash and source-tree digest, so profile,
class, phenotype, or build provenance can never overwrite another bucket.
Add explicit tamper cases for phenotype-manifest digest and canonical capacity
digest; both must fail before artifact aggregation.

- [ ] **Step 6: Run persistence and app tests**

Run: `cargo test -p alife_core --test experience_three_phase --test packed_experience_logging`

Expected: profile ID/offset/migration tests pass.

Run: `cargo test -p alife_world --test gpu_memory_grounding_persistence --test gpu_brain_persistence --test save_load_roundtrip`

Expected: pass.

Run: `cargo test -p alife_game_app --features gpu-runtime save_load -j 1`

Expected: pass.

Run: `cargo test -p alife_game_app --features gpu-runtime gpu_evidence -j 1`

Expected: shared A/B evidence fixtures remain valid and Slice C profile/
manifest/capacity/artifact tamper cases are rejected by the same loader.

Run: `cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test gpu_sleep_restore -j 1 -- --nocapture`

Expected: restored memory compaction reuses its committed receipt and never
prepares or swaps twice.

- [ ] **Step 7: Commit**

```powershell
git add crates/alife_core/src/experience.rs crates/alife_core/src/packed_log.rs crates/alife_core/tests/experience_three_phase.rs crates/alife_core/tests/packed_experience_logging.rs crates/alife_world/src/persistence.rs crates/alife_world/tests/gpu_memory_grounding_persistence.rs crates/alife_game_app/src/gpu_evidence.rs crates/alife_game_app/src/gpu_live_runtime.rs crates/alife_game_app/src/bin/alife_game_app.rs crates/alife_game_app/tests/gpu_sleep_restore.rs
git commit -m "Persist memory grounding provenance"
```

### Task 10: Prove Slice C grounding and saturation on real GPU hardware

**Files:**
- Create: `crates/alife_game_app/tests/gpu_memory_grounding_acceptance.rs`
- Modify: `crates/alife_game_app/src/bin/alife_game_app.rs`
- Runtime artifacts: `target/artifacts/gpu-memory-grounding-slice-c-<profile>-<class>.json` for each of the two profiles and three classes.

**Interfaces:**
- Consumes: completed Slice C runtime.
- Produces: candidate-specific poisoned-food avoidance, profile provenance, semantic-free grounded upload, deterministic saturation, bounded storage, and positive GPU-authority receipts for all initial production tiers.

- [ ] **Step 1: Add the failing acceptance assertions**

```rust
#[test]
fn slice_c_receipt_proves_candidate_specific_grounded_memory() {
    let receipt = run_gpu_memory_grounding_acceptance(test_options()).unwrap();
    assert_eq!(receipt.sensor_profile.profile().unwrap(), SensorProfile::GroundedObjectSlotsV1);
    let saturation = receipt.capacity_saturation.as_ref().unwrap();
    assert_eq!(saturation.grounded_semantic_label_channels_nonzero, 0);
    assert!(receipt.poisoned_ingest_logit_after < receipt.poisoned_ingest_logit_before);
    assert!(receipt.poisoned_avoid_logit_after > receipt.poisoned_avoid_logit_before);
    assert!(receipt.safe_ingest_delta.abs() < receipt.poisoned_ingest_delta.abs());
    assert!(receipt.cyan_avoid_target_latent[2] > 0.0);
    assert_eq!(receipt.cyan_ingest_target_latent, receipt.cyan_avoid_target_latent);
    assert!(receipt.cyan_ingest_family_value[2] > 0.0);
    assert_eq!(receipt.cyan_avoid_family_value, [0.0; 4]);
    assert_eq!(receipt.amber_target_latent, [0.0; 8]);
    assert_eq!(receipt.memory_enabled.recurrent_activation_digest, receipt.memory_ablated.recurrent_activation_digest);
    assert_ne!(receipt.post_learning_selection, receipt.poisoned_ingest_candidate);
    assert_eq!(receipt.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
    assert_eq!(receipt.gpu_selection_count, receipt.completed_waking_ticks);
    assert_eq!(receipt.memory_enabled.fast_weight_digest, receipt.memory_ablated.fast_weight_digest);
    assert_eq!(receipt.memory_enabled.phenotype_hash, receipt.memory_ablated.phenotype_hash);
    assert!(receipt.memory_enabled.poisoned_ingest_delta
        < receipt.memory_ablated.poisoned_ingest_delta - receipt.tolerance);
    assert!((receipt.memory_enabled.safe_ingest_delta
        - receipt.memory_ablated.safe_ingest_delta).abs() <= receipt.tolerance);
}

#[test]
fn slice_c_soak_degrades_without_terminal_capacity_failure() {
    let receipt = run_gpu_memory_grounding_acceptance(test_options()).unwrap();
    assert_eq!(receipt.completed_ticks, 10_240);
    let saturation = receipt.capacity_saturation.as_ref().unwrap();
    assert!(saturation.memory_records <= saturation.memory_capacity);
    assert!(saturation.tracked_object_records <= saturation.tracked_object_capacity);
    assert!(saturation.tracked_object_evictions > 0);
    assert_eq!(saturation.tracked_object_id_reuse_count, 0);
    assert!(saturation.topology_capacity.contains(
        saturation.topology_counts,
        saturation.max_observed_bindings_per_kind,
    ));
    assert!(saturation.memory_merges + saturation.memory_evictions > 0);
    assert!(saturation.topology_degradations > 0);
    assert_eq!(saturation.terminal_capacity_errors, 0);
    assert!(receipt.compact_readback_bytes <= 64);
}

#[test]
fn privileged_and_grounded_receipts_use_distinct_profile_qualified_artifacts() {
    let grounded = acceptance_options(SensorProfile::GroundedObjectSlotsV1);
    let privileged = acceptance_options(SensorProfile::PrivilegedAffordanceV1);
    assert_ne!(grounded.artifact_path(), privileged.artifact_path());
    assert_ne!(grounded.aggregate_key(), privileged.aggregate_key());
    assert_eq!(
        grounded.artifact_slug(),
        "gpu-memory-grounding-slice-c-grounded-object-slots-v1-n512",
    );
    let privileged_receipt = run_gpu_memory_grounding_acceptance(privileged).unwrap();
    assert_eq!(privileged_receipt.completed_ticks, 64);
    assert!(privileged_receipt.capacity_saturation.is_none());
}
```

- [ ] **Step 2: Run and verify the acceptance runner is absent**

Run: `cargo test -p alife_game_app --features gpu-runtime --test gpu_memory_grounding_acceptance -j 1 -- --nocapture`

Expected: compile failure for missing acceptance runner and receipt.

- [ ] **Step 3: Implement the acceptance subcommand**

Add `gpu-memory-grounding-acceptance --class n512|n1024|n2048 --ticks
10240 --seed 4303 --sensor-profile <profile>`. Derive the artifact path as
`target/artifacts/gpu-memory-grounding-slice-c-<profile>-<class>.json` and
include the same key in the aggregate manifest, so no profile can overwrite
another. The deterministic scenario presents cyan/bitter and amber/sweet
edible objects, records a painful cyan ingest, re-presents both with identical
world legality, and requires the GPU decoder to reduce cyan ingest/increase
cyan avoidance without applying the same delta to amber ingest. The receipt
must show that cyan candidates share one nonzero target-local pain latent,
only cyan ingest receives the exact-family value, amber stays zero, and
recurrent activation digest is unchanged between context-enabled/ablated
probes. Any pooled/global recurrent memory contribution fails acceptance.
Continue with unique tracked objects until tracker, memory, and topology
capacities all degrade deterministically without ID reuse or terminal failure.

After the painful encounter and Slice B fast-weight update, checkpoint the full
GPU slot. Fork two real-GPU branches with the exact same phenotype/hash,
activations, lifetime/fast/eligibility state, and weight digests. The
memory-enabled branch uploads the recalled context; the ablated branch uploads
the validated all-zero context through an explicit test-only context gate. Do
not change compiled projection scale or phenotype state. Hold sidecar
observation disabled during this paired probe. Require the poisoned-target
conditioning difference to disappear under ablation while unrelated candidate
logits remain within the same-adapter tolerance, and record matching phenotype
and mutable-state digests before the probe.

Implement the exact receipt DTOs in the acceptance module:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyCapacityReceipt {
    pub max_concepts: u32,
    pub max_edges: u32,
    pub max_simplexes: u32,
    pub max_unresolved_gaps: u32,
    pub max_bindings_per_kind: u32,
}

impl TopologyCapacityReceipt {
    pub fn contains(&self, counts: TopologyCounts, observed_max_bindings: u32) -> bool;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapacitySaturationEvidence {
    pub grounded_semantic_label_channels_nonzero: u32,
    pub memory_records: u32,
    pub memory_capacity: u32,
    pub memory_merges: u64,
    pub memory_evictions: u64,
    pub tracked_object_records: u32,
    pub tracked_object_capacity: u32,
    pub tracked_object_evictions: u64,
    pub tracked_object_id_reuse_count: u64,
    pub topology_counts: TopologyCounts,
    pub topology_capacity: TopologyCapacityReceipt,
    pub max_observed_bindings_per_kind: u32,
    pub topology_degradations: u64,
    pub terminal_capacity_errors: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryContextProbeEvidence {
    pub phenotype_hash: PhenotypeHash,
    pub phenotype_manifest_digest: [u64; 4],
    pub activation_digest: [u64; 4],
    pub recurrent_activation_digest: [u64; 4],
    pub lifetime_weight_digest: [u64; 4],
    pub fast_weight_digest: [u64; 4],
    pub eligibility_digest: [u64; 4],
    pub poisoned_ingest_delta: f32,
    pub safe_ingest_delta: f32,
    pub selected_candidate: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuMemoryGroundingEvidenceReceipt {
    #[serde(flatten)]
    pub header: ProfiledBehaviorReceiptHeader,
    pub phenotype_manifest: PhenotypeEvidenceManifest,
    pub sensor_profile: SensorProfileIdentity,
    pub capacity_class_slug: String,
    pub policy_backend: PolicyBackend,
    pub hardware: GpuHardwareReceipt,
    pub completed_ticks: u64,
    pub completed_waking_ticks: u64,
    pub gpu_selection_count: u64,
    pub poisoned_ingest_candidate: u16,
    pub post_learning_selection: u16,
    pub poisoned_ingest_logit_before: f32,
    pub poisoned_ingest_logit_after: f32,
    pub poisoned_avoid_logit_before: f32,
    pub poisoned_avoid_logit_after: f32,
    pub poisoned_ingest_delta: f32,
    pub safe_ingest_delta: f32,
    pub cyan_ingest_target_latent: [f32; 8],
    pub cyan_avoid_target_latent: [f32; 8],
    pub cyan_ingest_family_value: [f32; 4],
    pub cyan_avoid_family_value: [f32; 4],
    pub amber_target_latent: [f32; 8],
    pub memory_enabled: MemoryContextProbeEvidence,
    pub memory_ablated: MemoryContextProbeEvidence,
    pub capacity_saturation: Option<CapacitySaturationEvidence>,
    pub compact_readback_bytes: u32,
    pub tolerance: f32,
}
```

Construct every field before computing the shared artifact digest; the shared
loader rejects NaN/Inf, counter inconsistencies, failed behavioral inequalities,
nonpassing common status, and any manifest/header/body identity mismatch. It
also requires `header.adapter_name == hardware.adapter_name`,
`header.adapter_backend == hardware.backend_api`, and the hardware generation
captured for the run to equal every GPU tick's
`hardware_receipt_generation`; copied or stale adapter text is invalid.

Validation branches on the exact profile identity. A grounded receipt requires
`completed_ticks == 10_240`, `capacity_saturation.is_some()`, zero semantic
labels, actual tracker/memory/topology pressure, bounded counts and bindings,
nonzero degradation/eviction evidence, no tracked-ID reuse, and zero terminal
capacity errors. A privileged receipt requires `completed_ticks == 64`, the
same GPU behavior/provenance and header/hardware checks, and
`capacity_saturation.is_none()`; it makes no semantic-free perception or
saturation claim. The runner and validating loader reject any other tick count
or profile/evidence combination.

The JSON receipt must include class, profile and schema, Vulkan adapter,
phenotype hash, before/after selected logits, per-target deltas, selected
candidate, the profile-appropriate optional saturation block, completed ticks,
compact readback bytes,
`policy_backend`, `gpu_selection_count`, and `completed_waking_ticks`. Removed
CPU-shadow/fallback compatibility counters must not reappear.
Embed the exact Task 9 `ProfiledBehaviorReceiptHeader`, including stable
artifact slug, backend API version/slug, adapter name/backend, and seed around
the flattened shared Slice A evidence header. That shared header supplies
schema/slice/status, class/profile IDs, 40-hex clean Git commit/tree object IDs,
artifact digest, phenotype hash/manifest digest, and canonical capacity digest;
the body supplies the full profile identity and embedded phenotype manifest.
Final promotion rejects a stale or
mis-slugged artifact after any source merge.
Extend the shared `gpu-evidence-validate --slice a|b|c --input <path>` command
and loader with the Slice C body. For `--slice c` it resolves canonical capacity
by `class_id_raw`, recomputes shared phenotype manifest/capacity/artifact
digests, validates nonzero profile ID/schema against the body sensor identity,
and validates slug/backend/adapter/seed plus the full behavior/saturation
contract. It exits nonzero on any mismatch, performs no scenario run, and does
not rewrite the artifact.

- [ ] **Step 4: Run focused crate gates**

```powershell
cargo fmt --all -- --check
cargo test -p alife_core --all-targets
cargo test -p alife_world --all-targets
cargo test -p alife_gpu_backend --features gpu-tests --all-targets -- --nocapture
cargo test -p alife_game_app --features gpu-runtime --test gpu_memory_grounding_acceptance -j 1 -- --nocapture
cargo test -p alife_game_app --features "gpu-runtime gpu-tests" --test gpu_sleep_restore -j 1 -- --nocapture
cargo check --workspace --all-targets
cargo test --workspace --all-targets -j 1
```

Expected: all pass on the real adapter.

Focused tests in this step use a temp-directory/non-promotion sink and do not
write the six final artifact paths while source changes are uncommitted.

- [ ] **Step 5: Commit the acceptance runner before producing evidence**

```powershell
git diff --check
git add crates/alife_game_app/src/bin/alife_game_app.rs crates/alife_game_app/tests/gpu_memory_grounding_acceptance.rs
git commit -m "Prove GPU memory grounding degradation"
```

Expected: commit succeeds; no promotion artifact has yet been emitted for this
source revision.

- [ ] **Step 6: Assert clean committed evidence provenance**

```powershell
$dirty = git status --porcelain=v1
if ($dirty) { $dirty; throw 'Slice C evidence requires a clean worktree' }
$commit = (git rev-parse HEAD).Trim()
if ($commit -notmatch '^[0-9a-f]{40}$') { throw "invalid commit: $commit" }
$tree = (git rev-parse 'HEAD^{tree}').Trim()
if ($tree -notmatch '^[0-9a-f]{40}$') { throw "invalid tree: $tree" }
git diff --check HEAD
```

Expected: worktree is clean, commit and tree are exact lowercase 40-hex, and diff check
passes. The runner must refuse promotion-artifact output if this precondition
is not true.

- [ ] **Step 7: Run grounded 10,240-tick receipts for all production tiers**

```powershell
$classes = @('n512', 'n1024', 'n2048')
foreach ($class in $classes) {
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-memory-grounding-acceptance --class $class --ticks 10240 --seed 4303 --sensor-profile grounded-object-slots-v1
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
```

Expected: each receipt names Vulkan and its exact class/profile, proves
target-specific poisoned-food avoidance, completes all ticks, reports bounded
degradation, reads back at most 64 bytes per active tick, reports a GPU
selection for every waking tick, and reports zero terminal capacity errors.

- [ ] **Step 8: Run separate privileged provenance receipts**

```powershell
$classes = @('n512', 'n1024', 'n2048')
foreach ($class in $classes) {
    cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-memory-grounding-acceptance --class $class --ticks 64 --seed 4303 --sensor-profile privileged-affordance-v1
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
```

Expected: each report is explicitly labelled `PrivilegedAffordanceV1`, is stored separately from grounded results, and makes no perceptual-grounding claim.

- [ ] **Step 9: Run architecture, source, and artifact-provenance gates**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
$forbidden = rg -n --glob '!legacy_neural_policy_v1.rs' "AutoWithCpuFallback|CpuReference|cpu_shadow_ms|cpu_shadow_checked|cpu_shadow_parity|bias_proposals|memory_delta|encode_memory_state|GpuBrainMemoryExtensionRecord" crates/alife_gpu_backend/src crates/alife_game_app/src
$scanExit = $LASTEXITCODE
if ($scanExit -eq 0) { $forbidden; throw "forbidden production brain contract remains" }
if ($scanExit -ne 1) { throw "source scan failed with exit $scanExit" }
$commit = (git rev-parse HEAD).Trim()
$tree = (git rev-parse 'HEAD^{tree}').Trim()
$profiles = @('grounded-object-slots-v1', 'privileged-affordance-v1')
$classes = @('n512', 'n1024', 'n2048')
$classIds = @{ n512 = 1; n1024 = 2; n2048 = 3 }
$profileIds = @{ 'privileged-affordance-v1' = 1; 'grounded-object-slots-v1' = 2 }
foreach ($profile in $profiles) {
    foreach ($class in $classes) {
        $slug = "gpu-memory-grounding-slice-c-$profile-$class"
        $path = "target/artifacts/$slug.json"
        if (-not (Test-Path -LiteralPath $path)) { throw "missing artifact: $path" }
        $receipt = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
        $header = $receipt
        if ($header.artifact_schema -ne 1 -or $header.slice_raw -ne 3 -or $header.status_raw -ne 1) { throw "bad shared evidence identity: $path" }
        if ($header.artifact_slug -ne $slug) { throw "bad slug: $path" }
        if ($header.class_id_raw -ne $classIds[$class]) { throw "bad class ID: $path" }
        if ($header.profile_id_raw -ne $profileIds[$profile] -or $header.profile_schema -ne 1) { throw "bad profile binding: $path" }
        if ($receipt.sensor_profile.profile_id -ne $profileIds[$profile] -or $receipt.sensor_profile.profile_schema_version -ne 1 -or $receipt.sensor_profile.sensory_abi_version -le 0) { throw "bad body profile identity: $path" }
        if ($header.git_commit -ne $commit) { throw "stale commit: $path" }
        if ($header.source_tree_digest -ne $tree) { throw "stale tree: $path" }
        if ($header.backend_api_version -ne 1 -or $header.backend_api_slug -ne 'gpu-closed-loop-v1') { throw "bad backend ABI: $path" }
        if ($header.adapter_backend -notmatch '(?i)vulkan' -or [string]::IsNullOrWhiteSpace($header.adapter_name)) { throw "non-Vulkan artifact: $path" }
        if ($header.adapter_name -ne $receipt.hardware.adapter_name -or $header.adapter_backend -ne $receipt.hardware.backend_api) { throw "header/hardware adapter mismatch: $path" }
        if ($profile -eq 'grounded-object-slots-v1') {
            if ($receipt.completed_ticks -ne 10240 -or $null -eq $receipt.capacity_saturation) { throw "grounded evidence lacks exact saturation run: $path" }
            if ($receipt.capacity_saturation.grounded_semantic_label_channels_nonzero -ne 0 -or $receipt.capacity_saturation.terminal_capacity_errors -ne 0) { throw "bad grounded saturation evidence: $path" }
            if ($receipt.capacity_saturation.max_observed_bindings_per_kind -gt $receipt.capacity_saturation.topology_capacity.max_bindings_per_kind) { throw "topology binding capacity exceeded: $path" }
        } else {
            if ($receipt.completed_ticks -ne 64 -or $null -ne $receipt.capacity_saturation) { throw "privileged receipt made a saturation claim: $path" }
        }
        if (($header.phenotype_manifest_digest -join ',') -eq '0,0,0,0') { throw "zero phenotype manifest digest: $path" }
        if (($header.capacity_digest -join ',') -eq '0,0,0,0') { throw "zero capacity digest: $path" }
        if (($header.artifact_digest -join ',') -eq '0,0,0,0') { throw "zero artifact digest: $path" }
        if (($receipt.phenotype_manifest.manifest_digest -join ',') -ne ($header.phenotype_manifest_digest -join ',')) { throw "manifest binding mismatch: $path" }
        cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-evidence-validate --slice c --input $path
        if ($LASTEXITCODE -ne 0) { throw "artifact verification failed: $path" }
    }
}
git diff --check HEAD
$dirty = git status --porcelain=v1
if ($dirty) { $dirty; throw 'worktree changed while collecting Slice C evidence' }
```

Expected: boundary/docs/diff commands exit 0; the guarded source scan finds no
production matches; all six artifacts bind the current clean commit and exact
Git tree object ID, stable slug, backend ABI, body digest, and Vulkan adapter.

- [ ] **Step 10: Verify the committed Slice C diff**

```powershell
git diff --check origin/main...HEAD
git status --short
```

Expected: diff check exits 0 and status is clean.

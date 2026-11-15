//! Deterministic, score-free action candidates derived from one sensory report.

use std::collections::BTreeSet;

use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, AffordanceBits, CandidateActionFamily,
    CandidateFeatureVector, CandidateObservationRef, CandidateOpportunityCoverageV1, Confidence,
    DurationTicks, NormalizedScalar, PerceptionFrameDraft, ScaffoldContractError, SensorProfile,
    TrackedObjectId, Validate, Vec3f, WorldEntityId, CANDIDATE_FEATURE_COUNT,
    MAX_ACTION_CANDIDATES,
};

use crate::{
    headless::HEADLESS_CONTACT_RADIUS, GroundedSensingFrame, HeadlessActionIds,
    HeadlessSensoryReport, VisibleWorldEntity,
};

pub const HEADLESS_VISION_RADIUS: f32 = 8.0;
pub const CANDIDATE_FEATURE_BEARING_SIN_LANE: usize = 0;
pub const CANDIDATE_FEATURE_BEARING_COS_LANE: usize = 1;
pub const CANDIDATE_FEATURE_DISTANCE_LANE: usize = 2;
pub const CANDIDATE_FEATURE_RELATIVE_VELOCITY_X_LANE: usize = 3;
pub const CANDIDATE_FEATURE_RELATIVE_VELOCITY_Y_LANE: usize = 4;
pub const CANDIDATE_FEATURE_RELATIVE_VELOCITY_Z_LANE: usize = 5;
pub const CANDIDATE_FEATURE_AFFORDANCE_START_LANE: usize = 6;
pub const CANDIDATE_FEATURE_AFFORDANCE_COUNT: usize = 10;
pub const CANDIDATE_FEATURE_CONTACT_LANE: usize = 16;
pub const CANDIDATE_FEATURE_EVIDENCE_LANE: usize = 17;
pub const CANDIDATE_FEATURE_RESERVED_START_LANE: usize = 18;
// Activation and gripping use separate existing readout families. All 24 object
// features keep their physical meanings, including angular speed in lane 20.
const INTRINSIC_CANDIDATE_COUNT: usize = 3;
const OBJECT_ACTIONS: [(ActionId, ActionKind, CandidateActionFamily, f32); 6] = [
    (
        ActionKind::Inspect.canonical_id(),
        ActionKind::Inspect,
        CandidateActionFamily::Inspect,
        0.1,
    ),
    (
        HeadlessActionIds::APPROACH,
        ActionKind::Move,
        CandidateActionFamily::Approach,
        0.3,
    ),
    (
        HeadlessActionIds::FLEE,
        ActionKind::Move,
        CandidateActionFamily::Avoid,
        0.3,
    ),
    (
        HeadlessActionIds::EAT,
        ActionKind::Interact,
        CandidateActionFamily::Ingest,
        0.2,
    ),
    (
        HeadlessActionIds::GRAB,
        ActionKind::Interact,
        CandidateActionFamily::Contact,
        0.2,
    ),
    (
        HeadlessActionIds::PLAY,
        ActionKind::Interact,
        CandidateActionFamily::Other,
        0.2,
    ),
];
const MAX_CANDIDATE_OBJECTS: usize = (MAX_ACTION_CANDIDATES - INTRINSIC_CANDIDATE_COUNT) / 6;
const KNOWN_AFFORDANCE_MASK: u32 = (1 << CANDIDATE_FEATURE_AFFORDANCE_COUNT) - 1;

/// Explicit transient selection phase supplied from the persisted experience sequence.
/// This plan never owns a mutable counter and never consumes the sampling RNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateOpportunityCoveragePlan {
    phase: u64,
}
impl CandidateOpportunityCoveragePlan {
    pub const fn new(phase: u64) -> Self {
        Self { phase }
    }
    pub const fn phase(&self) -> u64 {
        self.phase
    }
}

/// Diagnostic outcome of the additive active-opportunity overlay, never a score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActiveApproachOpportunityReason {
    Uncrowded, NoActive, Unobserved, AlreadySelected, NoSpareCapacity, Admitted,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveApproachOpportunityReceipt {
    pub requested_id: Option<TrackedObjectId>,
    pub observed_slot: Option<u16>,
    pub original_selected_slots: Vec<u16>,
    pub final_selected_slots: Vec<u16>,
    pub protected_contact: Option<u16>,
    pub protected_focal: Option<u16>,
    pub protected_rotation: Option<u16>,
    pub admitted: bool,
    pub reason: ActiveApproachOpportunityReason,
    /// Additional logical overlay operations; CPU charges once, separately from V1 coverage work.
    pub work_units: u64,
}

/// Transient world-owned target transport from the exact observed snapshot.
/// Kept outside the sensory/checkpoint ABI and never resolved through tracking
/// history or a later world lookup during cognition preparation.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundedCandidateBinding {
    grounded: GroundedSensingFrame,
    profile: SensorProfile,
    active_contact: Option<WorldEntityId>,
    grounded_orientation_v3: bool,
    coverage_plan: Option<CandidateOpportunityCoveragePlan>,
    active_approach: Option<Option<TrackedObjectId>>,
    maze_heading2346: bool,
}

impl GroundedCandidateBinding {
    pub(crate) fn new(
        grounded: GroundedSensingFrame,
        profile: SensorProfile,
        active_contact: Option<WorldEntityId>,
        grounded_orientation_v3: bool,
    ) -> Self {
        Self {
            grounded,
            profile,
            active_contact,
            grounded_orientation_v3,
            coverage_plan: None,
            active_approach: None,
            maze_heading2346: false,
        }
    }

    pub(crate) fn with_maze_heading2346(mut self)->Self{self.grounded_orientation_v3=true;self.maze_heading2346=true;self}
    pub fn with_coverage(mut self, plan: CandidateOpportunityCoveragePlan) -> Self {
        let capacity = if self.profile.has_terrain_vision() || self.grounded_orientation_v3 {
            3
        } else {
            MAX_CANDIDATE_OBJECTS
        };
        self.coverage_plan = (self.grounded.slots().len() > capacity).then_some(plan);
        self
    }
    pub const fn coverage_plan(&self) -> Option<CandidateOpportunityCoveragePlan> {
        self.coverage_plan
    }

    /// Explicit new adapter activation; Some(None) is enabled without an incumbent.
    pub fn with_active_approach(mut self, active: Option<TrackedObjectId>) -> Self {
        self.active_approach = Some(active);
        self
    }
    pub const fn active_approach_enabled(&self) -> bool { self.active_approach.is_some() }

    /// Validate the unchanged initial V1 draft, recompute its normal focal selection,
    /// then replace only an unprotected nearest-prefill tail slot if possible.
    #[allow(clippy::type_complexity)]
    pub fn candidates_for_focus_with_active_opportunity(
        &self, draft: &PerceptionFrameDraft, focal_object: Option<TrackedObjectId>,
        already_charged_work: u64,
    ) -> Result<(Vec<ActionCandidate>, Option<CandidateOpportunityCoverageV1>, ActiveApproachOpportunityReceipt), ScaffoldContractError> {
        let active = self.active_approach.ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
        if self.coverage_plan.is_none() {
            if already_charged_work != 0 { return Err(ScaffoldContractError::InvalidPerceptionFrame); }
            let candidates = self.candidates_for_focus(draft, focal_object)?;
            let mut proof = ActiveApproachOpportunityReceipt {
                requested_id: active, observed_slot: None, original_selected_slots: Vec::new(),
                final_selected_slots: Vec::new(), protected_contact: None, protected_focal: None,
                protected_rotation: None, admitted: false, reason: ActiveApproachOpportunityReason::Uncrowded,
                work_units: 12, // receipt fields plus mode/initial-work branch checks
            };
            // All observed objects are already admitted; no replacement or synthetic protection.
            for candidate in &candidates {
                proof.work_units += 2; // candidate visit and observation match
                if let CandidateObservationRef::ObjectSlot(index) = candidate.observation {
                    let mut present = false;
                    for &old in &proof.original_selected_slots {
                        proof.work_units += 1;
                        if old == index { present = true; break; }
                    }
                    if !present {
                        proof.original_selected_slots.push(index);
                        proof.final_selected_slots.push(index); proof.work_units += 2;
                    }
                }
            }
            for slot in self.grounded.slots() {
                proof.work_units += 2; // visit and identity comparison
                if Some(slot.tracked_object_id) == active {
                    proof.observed_slot = Some(slot.slot_index); proof.work_units += 1;
                }
            }
            return Ok((candidates, None, proof));
        }
        let (original, receipt) = self.candidates_for_focus_with_coverage(draft, focal_object, already_charged_work)?;
        let receipt = receipt.ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
        let mut overlay = active_opportunity_overlay(self.grounded.slots(), &receipt, active)?;
        let mut candidates = original;
        if overlay.admitted {
            // Replace exactly one existing six-action object group, not a second
            // full candidate build. Original indices and all primitive semantics stay.
            let observed = overlay.observed_slot.ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
            let mut displaced = None;
            for (&old, &new) in overlay.original_selected_slots.iter().zip(&overlay.final_selected_slots) {
                overlay.work_units += 2; // pair visit/equality
                if old != new { displaced = Some(old); overlay.work_units += 1; break; }
            }
            let displaced = displaced.ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
            let slot = &self.grounded.slots()[observed as usize];
            let transport = &self.grounded.transports()[observed as usize];
            overlay.work_units += 3; // two bounded lookups and agreement predicate
            if slot.slot_index != observed || transport.slot_index != observed {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
            let features = slot.candidate_features()?;
            let target = ActionTarget::new(Some(transport.transport_entity),Some(transport.target_position));
            // One bounded feature materialization/validation and target construction.
            overlay.work_units += 2 * CANDIDATE_FEATURE_COUNT as u64 + 2;
            let mut changed = 0;
            for c in &mut candidates {
                overlay.work_units += 2; // visit and observation comparison
                if c.observation == CandidateObservationRef::ObjectSlot(displaced) {
                    *c = ActionCandidate::new(c.candidate_index,c.action_id,c.kind,c.family,
                        CandidateObservationRef::ObjectSlot(observed),target,features,
                        slot.confidence,c.required_effort,c.min_duration,c.max_duration)?;
                    // Eleven field writes, bounded descriptor validation, replacement write.
                    overlay.work_units += 12 + CANDIDATE_FEATURE_COUNT as u64;
                    changed += 1;
                }
            }
            overlay.work_units += 1;
            if changed != OBJECT_ACTIONS.len() { return Err(ScaffoldContractError::InvalidPerceptionFrame); }
        }
        let bound = if overlay.admitted {
            // Seven receipt fields plus bounded selected-index copy/validation.
            overlay.work_units += 7 + 3 * overlay.final_selected_slots.len() as u64;
            CandidateOpportunityCoverageV1::new(receipt.observed_slots(), receipt.capacity(),
                receipt.contact_slot(), receipt.focal_slot(), receipt.phase(), overlay.final_selected_slots.clone(), receipt.work_units())?
        } else { receipt };
        Ok((candidates, Some(bound), overlay))
    }

    /// Recompute using the current FIRST focal object, even if it was initially admitted.
    /// Validate against the exact initial coverage selection, not the legacy nearest set.
    pub fn candidates_for_focus_with_coverage(
        &self,
        draft: &PerceptionFrameDraft,
        focal_object: Option<TrackedObjectId>,
        already_charged_work: u64,
    ) -> Result<(Vec<ActionCandidate>, Option<CandidateOpportunityCoverageV1>), ScaffoldContractError>
    {
        let plan = self
            .coverage_plan
            .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
        draft.validate()?;
        if draft.organism_id() != self.grounded.sensory().organism_id
            || draft.tick() != self.grounded.sensory().tick
            || draft.sensor_profile() != self.profile
            || draft.grounded_orientation_repertoire() != self.grounded_orientation_v3
            || draft.maze_heading2346() != self.maze_heading2346
            || draft.body() != self.grounded.body()
            || draft.grounded_object_slots() != self.grounded.slots()
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let (initial, initial_receipt) = enumerate_grounded_candidates_with_coverage_impl(
            &self.grounded,
            self.profile,
            self.active_contact,
            None,
            self.grounded_orientation_v3,
            plan,
            0,
            self.maze_heading2346,
        )?;
        if draft.candidate_opportunity_coverage() != initial_receipt.as_ref()
            || already_charged_work != initial_receipt.as_ref().map_or(0, |r| r.work_units())
            || draft.candidates() != initial
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        // The validation selector really ran, so its work is additional, not free.
        let validation_work = initial_receipt.as_ref().map_or(0, |r| r.work_units());
        let carried = already_charged_work
            .checked_add(validation_work)
            .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
        enumerate_grounded_candidates_with_coverage_impl(
            &self.grounded,
            self.profile,
            self.active_contact,
            focal_object,
            self.grounded_orientation_v3,
            plan,
            carried,
            self.maze_heading2346,
        )
    }

    pub fn candidates_for_focus(
        &self,
        draft: &PerceptionFrameDraft,
        focal_object: Option<TrackedObjectId>,
    ) -> Result<Vec<ActionCandidate>, ScaffoldContractError> {
        if self.coverage_plan.is_some() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        // Semantic context/remembered novelty may be attached after capture;
        // observed bodies, slots and original opportunities must still match.
        if draft.organism_id() != self.grounded.sensory().organism_id
            || draft.tick() != self.grounded.sensory().tick
            || draft.sensor_profile() != self.profile
            || draft.grounded_orientation_repertoire() != self.grounded_orientation_v3
            || draft.maze_heading2346() != self.maze_heading2346
            || draft.body() != self.grounded.body()
            || draft.grounded_object_slots() != self.grounded.slots()
            || draft.candidates()
                != enumerate_grounded_candidates_impl_heading(
                    &self.grounded,
                    self.profile,
                    self.active_contact,
                    None,
                    self.grounded_orientation_v3,
                    self.maze_heading2346,
                )?
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        enumerate_grounded_candidates_impl_heading(
            &self.grounded,
            self.profile,
            self.active_contact,
            focal_object,
            self.grounded_orientation_v3,
            self.maze_heading2346,
        )
    }
}

pub fn enumerate_privileged_candidates(
    report: &HeadlessSensoryReport,
    profile: SensorProfile,
) -> Result<Vec<ActionCandidate>, ScaffoldContractError> {
    if profile != SensorProfile::PrivilegedAffordanceV1 {
        return Err(ScaffoldContractError::SensorProfileMismatch);
    }
    validate_report(report)?;

    let mut visible = report.visible_entities.iter().collect::<Vec<_>>();
    visible.sort_by(|left, right| {
        left.distance
            .total_cmp(&right.distance)
            .then_with(|| left.id.raw().cmp(&right.id.raw()))
    });
    visible.truncate(MAX_CANDIDATE_OBJECTS);

    let mut candidates = Vec::with_capacity(INTRINSIC_CANDIDATE_COUNT + visible.len() * 6);
    push_intrinsic_candidates(&mut candidates)?;

    for entity in visible {
        let features = features_for(report, entity)?;
        let target_position = add(
            report.core_snapshot.observer_position,
            entity.relative_position,
        );
        let target = ActionTarget::new(Some(entity.id), Some(target_position));
        let observation = CandidateObservationRef::None;
        for (action_id, kind, family, effort) in OBJECT_ACTIONS {
            candidates.push(ActionCandidate::new(
                u16::try_from(candidates.len())
                    .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
                action_id,
                kind,
                family,
                observation,
                target,
                features,
                Confidence::new(1.0)?,
                NormalizedScalar::new(effort)?,
                DurationTicks::new(1),
                DurationTicks::new(1),
            )?);
        }
    }
    Ok(candidates)
}

/// Reserves at most one already observed, calibrated contact opportunity.
/// The caller may nominate a physically held target from the same snapshot;
/// hidden, occluded, or non-contact targets cannot enter the candidate set.
pub fn enumerate_grounded_candidates(
    grounded: &GroundedSensingFrame,
    profile: SensorProfile,
    active_contact: Option<alife_core::WorldEntityId>,
) -> Result<Vec<ActionCandidate>, ScaffoldContractError> {
    enumerate_grounded_candidates_with_focus(grounded, profile, active_contact, None)
}

/// Reserve current observed contact and primary focal objects within the same
/// object budget. Identity alone cannot introduce an unobserved target.
pub fn enumerate_grounded_candidates_with_focus(
    grounded: &GroundedSensingFrame,
    profile: SensorProfile,
    active_contact: Option<WorldEntityId>,
    focal_object: Option<TrackedObjectId>,
) -> Result<Vec<ActionCandidate>, ScaffoldContractError> {
    enumerate_grounded_candidates_impl(grounded, profile, active_contact, focal_object, false)
}

/// Fresh-genome V3 repertoire. This grants unscored body/head primitives, not
/// terrain sensing or target labels. World callers bind architecture to owner.
pub fn enumerate_grounded_candidates_with_orientation_v3(
    grounded: &GroundedSensingFrame,
    profile: SensorProfile,
    active_contact: Option<WorldEntityId>,
    focal_object: Option<TrackedObjectId>,
    architecture: &alife_core::genome::CognitiveArchitectureGenomeParameters,
) -> Result<Vec<ActionCandidate>, ScaffoldContractError> {
    architecture.validate_contract()?;
    if !architecture.grounded_orientation_repertoire()
        || profile != SensorProfile::GroundedObjectSlotsV1
    {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }
    enumerate_grounded_candidates_impl(grounded, profile, active_contact, focal_object, true)
}

/// Under crowding, preserve observed contact and first focus, then grant one
/// rotating residual opportunity. Remaining capacity uses distance/identity order.
/// Work tariff counts loop visits, scalar key comparisons, index writes, membership
/// comparisons, and the one modulo. It is logical work, not elapsed runtime.
#[allow(clippy::too_many_arguments)]
pub fn enumerate_grounded_candidates_with_coverage(
    grounded: &GroundedSensingFrame,
    profile: SensorProfile,
    active_contact: Option<WorldEntityId>,
    focal_object: Option<TrackedObjectId>,
    grounded_orientation_v3: bool,
    plan: CandidateOpportunityCoveragePlan,
    already_charged_work: u64,
) -> Result<(Vec<ActionCandidate>, Option<CandidateOpportunityCoverageV1>), ScaffoldContractError> {
    enumerate_grounded_candidates_with_coverage_impl(grounded,profile,active_contact,focal_object,grounded_orientation_v3,plan,already_charged_work,false)
}
pub fn enumerate_maze_heading2346(grounded:&GroundedSensingFrame,profile:SensorProfile,active_contact:Option<WorldEntityId>,focal_object:Option<TrackedObjectId>,plan:CandidateOpportunityCoveragePlan,already_charged_work:u64)->Result<(Vec<ActionCandidate>,Option<CandidateOpportunityCoverageV1>),ScaffoldContractError>{
    enumerate_grounded_candidates_with_coverage_impl(grounded,profile,active_contact,focal_object,true,plan,already_charged_work,true)
}
#[allow(clippy::too_many_arguments)]
fn enumerate_grounded_candidates_with_coverage_impl(
    grounded: &GroundedSensingFrame,
    profile: SensorProfile,
    active_contact: Option<WorldEntityId>,
    focal_object: Option<TrackedObjectId>,
    grounded_orientation_v3: bool,
    plan: CandidateOpportunityCoveragePlan,
    already_charged_work: u64,
    heading_only: bool,
) -> Result<(Vec<ActionCandidate>, Option<CandidateOpportunityCoverageV1>), ScaffoldContractError> {
    let capacity = if profile.has_terrain_vision() || grounded_orientation_v3 {
        3
    } else {
        MAX_CANDIDATE_OBJECTS
    };
    if grounded.slots().len() <= capacity {
        if already_charged_work != 0 {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        return Ok((
            enumerate_grounded_candidates_impl_heading(
                grounded,
                profile,
                active_contact,
                focal_object,
                grounded_orientation_v3,
                heading_only,
            )?,
            None,
        ));
    }
    if grounded.slots().len() != grounded.transports().len()
        || grounded.slots().len() > alife_core::MAX_GROUNDED_OBJECT_SLOTS
    {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }
    let (selected, contact, focal, work) = coverage_selection(
        grounded,
        active_contact,
        focal_object,
        capacity,
        plan.phase(),
    )?;
    let work = already_charged_work
        .checked_add(work)
        .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
    let receipt = CandidateOpportunityCoverageV1::new(
        grounded.slots().len() as u16,
        capacity as u8,
        contact.map(|i| i as u16),
        focal.map(|i| i as u16),
        plan.phase(),
        selected.iter().map(|&i| i as u16).collect(),
        work,
    )?;
    let candidates = enumerate_grounded_candidates_selected(
        grounded,
        profile,
        active_contact,
        focal_object,
        grounded_orientation_v3,
        Some(selected),
        heading_only,
    )?;
    Ok((candidates, Some(receipt)))
}

/// Overlay tariff: ten fixed receipt-field writes plus two dispatch checks; two copies per selected index;
/// each examined slot/index, equality or reservation predicate and each state write
/// below is charged explicitly. This is logical selector work, not runtime timing.
fn active_opportunity_overlay(
    slots: &[alife_core::GroundedObjectSlotV1], base: &CandidateOpportunityCoverageV1,
    active: Option<TrackedObjectId>,
) -> Result<ActiveApproachOpportunityReceipt, ScaffoldContractError> {
    let mut r = ActiveApproachOpportunityReceipt {
        requested_id: active, observed_slot: None,
        original_selected_slots: base.selected_slots().to_vec(), final_selected_slots: base.selected_slots().to_vec(),
        protected_contact: base.contact_slot(), protected_focal: base.focal_slot(), protected_rotation: None,
        admitted: false, reason: ActiveApproachOpportunityReason::NoActive,
        work_units: 12 + 2 * base.selected_slots().len() as u64,
    };
    // The V1 selector writes unique contact/focal then exactly one rotating residual.
    // Recover that first nonreserved member, without recomputing or changing rotation.
    for &slot in base.selected_slots() {
        r.work_units += 3;
        let contact = Some(slot) == r.protected_contact;
        let focal = Some(slot) == r.protected_focal;
        if !contact && !focal { r.protected_rotation = Some(slot); r.work_units += 1; break; }
    }
    r.work_units += 1;
    let Some(active) = active else { return Ok(r); };
    for slot in slots {
        r.work_units += 2;
        if slot.tracked_object_id == active { r.observed_slot = Some(slot.slot_index); r.work_units += 1; break; }
    }
    r.work_units += 1;
    let Some(observed) = r.observed_slot else {
        r.reason = ActiveApproachOpportunityReason::Unobserved; r.work_units += 1; return Ok(r);
    };
    for &slot in &r.final_selected_slots {
        r.work_units += 2;
        if slot == observed { r.reason = ActiveApproachOpportunityReason::AlreadySelected; r.work_units += 1; return Ok(r); }
    }
    for index in (0..r.final_selected_slots.len()).rev() {
        let slot = r.final_selected_slots[index]; r.work_units += 4;
        let contact = Some(slot) == r.protected_contact;
        let focal = Some(slot) == r.protected_focal;
        let rotation = Some(slot) == r.protected_rotation;
        if !contact && !focal && !rotation {
            r.final_selected_slots[index] = observed;
            r.admitted = true; r.reason = ActiveApproachOpportunityReason::Admitted; r.work_units += 3;
            return Ok(r);
        }
    }
    r.reason = ActiveApproachOpportunityReason::NoSpareCapacity; r.work_units += 1;
    Ok(r)
}

#[allow(clippy::type_complexity)]
fn coverage_selection(
    grounded: &GroundedSensingFrame,
    active_contact: Option<WorldEntityId>,
    focal_object: Option<TrackedObjectId>,
    capacity: usize,
    phase: u64,
) -> Result<(Vec<usize>, Option<usize>, Option<usize>, u64), ScaffoldContractError> {
    let n = grounded.slots().len();
    let mut work = 0_u64;
    let mut contact = None;
    let mut focal = None;
    let mut roster = Vec::with_capacity(n);
    for (i, (slot, transport)) in grounded
        .slots()
        .iter()
        .zip(grounded.transports())
        .enumerate()
    {
        work += 4; // visit, slot/transport agreement, contact identity, focal identity
        if slot.slot_index as usize != i || transport.slot_index != slot.slot_index {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        if Some(transport.transport_entity) == active_contact {
            work += 1;
            if slot.contact > 0.0 {
                contact = Some(i);
                work += 1;
            }
        }
        if Some(slot.tracked_object_id) == focal_object {
            focal = Some(i);
            work += 1;
        }
        roster.push(i);
        work += 1;
    }
    // Deterministic insertion sort: count every actual comparison and move.
    for i in 1..roster.len() {
        let mut j = i;
        while j > 0 {
            work += 2; // duplicate equality and ordering comparison
            let a = grounded.slots()[roster[j - 1]].tracked_object_id.raw();
            let b = grounded.slots()[roster[j]].tracked_object_id.raw();
            if a == b {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
            if a < b {
                break;
            }
            roster.swap(j - 1, j);
            work += 2;
            j -= 1;
        }
    }
    let mut selected = Vec::with_capacity(capacity);
    if let Some(i) = contact {
        selected.push(i);
        work += 1;
    }
    if let Some(i) = focal {
        work += 1;
        if Some(i) != contact {
            selected.push(i);
            work += 1;
        }
    }
    let mut residual = Vec::new();
    for i in roster {
        work += 3; // visit and two reservation comparisons
        // Evaluate both billed identity comparisons, even for the contact slot.
        let is_contact = Some(i) == contact;
        let is_focal = Some(i) == focal;
        if !is_contact && !is_focal {
            residual.push(i);
            work += 1;
        }
    }
    // No reservation is sacrificed to make room. At least one residual opportunity
    // is mandatory whenever any unreserved observed object remains.
    if selected.len() > capacity || (!residual.is_empty() && selected.len() == capacity) {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }
    if !residual.is_empty() {
        let index = (phase % residual.len() as u64) as usize;
        work += 1;
        selected.push(residual[index]);
        work += 1;
    }
    let mut nearest = (0..n).collect::<Vec<_>>();
    work += n as u64;
    for i in 1..nearest.len() {
        let mut j = i;
        while j > 0 {
            let a = &grounded.slots()[nearest[j - 1]];
            let b = &grounded.slots()[nearest[j]];
            work += 1;
            let mut ordering = a.distance.total_cmp(&b.distance);
            if ordering.is_eq() {
                work += 1;
                ordering = a.tracked_object_id.raw().cmp(&b.tracked_object_id.raw());
            }
            if !ordering.is_gt() {
                break;
            }
            nearest.swap(j - 1, j);
            work += 2;
            j -= 1;
        }
    }
    for i in nearest {
        work += 1;
        if selected.len() == capacity {
            break;
        }
        let mut found = false;
        for &s in &selected {
            work += 1;
            if i == s {
                found = true;
                break;
            }
        }
        if !found {
            selected.push(i);
            work += 1;
        }
    }
    Ok((selected, contact, focal, work))
}

fn enumerate_grounded_candidates_impl(
    grounded: &GroundedSensingFrame,
    profile: SensorProfile,
    active_contact: Option<WorldEntityId>,
    focal_object: Option<TrackedObjectId>,
    grounded_orientation_v3: bool,
) -> Result<Vec<ActionCandidate>, ScaffoldContractError> {
    enumerate_grounded_candidates_impl_heading(grounded,profile,active_contact,focal_object,grounded_orientation_v3,false)
}
fn enumerate_grounded_candidates_impl_heading(
    grounded: &GroundedSensingFrame,
    profile: SensorProfile,
    active_contact: Option<WorldEntityId>,
    focal_object: Option<TrackedObjectId>,
    grounded_orientation_v3: bool,
    heading_only: bool,
) -> Result<Vec<ActionCandidate>, ScaffoldContractError> {
    enumerate_grounded_candidates_selected(
        grounded,
        profile,
        active_contact,
        focal_object,
        grounded_orientation_v3,
        None,
        heading_only,
    )
}

fn enumerate_grounded_candidates_selected(
    grounded: &GroundedSensingFrame,
    profile: SensorProfile,
    active_contact: Option<WorldEntityId>,
    focal_object: Option<TrackedObjectId>,
    grounded_orientation_v3: bool,
    selected_override: Option<Vec<usize>>,
    heading_only: bool,
) -> Result<Vec<ActionCandidate>, ScaffoldContractError> {
    if !matches!(
        profile,
        SensorProfile::GroundedObjectSlotsV1
            | SensorProfile::GroundedTerrainVisionV1
            | SensorProfile::GroundedTerrainObjectPortsV2
    ) {
        return Err(ScaffoldContractError::SensorProfileMismatch);
    }
    if grounded.slots().len() != grounded.transports().len()
        || grounded.slots().len() > alife_core::MAX_GROUNDED_OBJECT_SLOTS
    {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }

    let terrain_vision = profile.has_terrain_vision() || grounded_orientation_v3;
    let object_limit = if terrain_vision {
        // Leave room for locomotion, body/head orientation, and an explicit
        // hold-gaze command within the frozen 32-candidate foundation ABI.
        3
    } else {
        MAX_CANDIDATE_OBJECTS
    };
    let mut candidates = Vec::with_capacity(
        INTRINSIC_CANDIDATE_COUNT
            + grounded.slots().len().min(object_limit) * 6
            + 3
            + if terrain_vision { 7 } else { 0 },
    );
    push_intrinsic_candidates(&mut candidates)?;

    if terrain_vision {
        for (action_id, kind, family, bearing_sin, bearing_cos) in [
            (
                HeadlessActionIds::STEP_FORWARD,
                ActionKind::Move,
                CandidateActionFamily::Approach,
                0.0,
                1.0,
            ),
            (
                HeadlessActionIds::TURN_LEFT,
                ActionKind::Look,
                CandidateActionFamily::Inspect,
                1.0,
                0.0,
            ),
            (
                HeadlessActionIds::TURN_RIGHT,
                ActionKind::Look,
                CandidateActionFamily::Inspect,
                -1.0,
                0.0,
            ),
        ] {
            let mut features = [0.0; CANDIDATE_FEATURE_COUNT];
            features[CANDIDATE_FEATURE_BEARING_SIN_LANE] = bearing_sin;
            features[CANDIDATE_FEATURE_BEARING_COS_LANE] = bearing_cos;
            // Existing intrinsic body-primitive basis. Not an exclusive kind bit:
            // observed object lanes 19/23 can also be nonzero.
            features[CANDIDATE_FEATURE_RESERVED_START_LANE + 1] = 1.0;
            candidates.push(ActionCandidate::new(
                u16::try_from(candidates.len())
                    .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
                action_id,
                kind,
                family,
                CandidateObservationRef::None,
                ActionTarget::NONE,
                CandidateFeatureVector(features).with_intrinsic_action_basis(kind),
                Confidence::new(1.0)?,
                NormalizedScalar::new(0.1)?,
                DurationTicks::new(1),
                DurationTicks::new(1),
            )?);
        }
        if !heading_only { for (action_id, bearing_sin, bearing_cos) in [
            (HeadlessActionIds::LOOK_LEFT, 1.0, 0.0),
            (HeadlessActionIds::LOOK_RIGHT, -1.0, 0.0),
            (HeadlessActionIds::LOOK_CENTER, 0.0, 1.0),
            (HeadlessActionIds::HOLD_GAZE, 0.0, 0.0),
        ] {
            // The GPU decoder scores features, not ActionId. Without a
            // distinct feature vectors the gaze choices would tie forever.
            let mut look_features = [0.0; CANDIDATE_FEATURE_COUNT];
            look_features[CANDIDATE_FEATURE_BEARING_SIN_LANE] = bearing_sin;
            look_features[CANDIDATE_FEATURE_BEARING_COS_LANE] = bearing_cos;
            look_features[CANDIDATE_FEATURE_RESERVED_START_LANE] = 1.0;
            candidates.push(ActionCandidate::new(
                u16::try_from(candidates.len())
                    .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
                action_id,
                ActionKind::Look,
                CandidateActionFamily::Inspect,
                CandidateObservationRef::None,
                ActionTarget::NONE,
                CandidateFeatureVector(look_features).with_intrinsic_action_basis(ActionKind::Look),
                Confidence::new(1.0)?,
                NormalizedScalar::new(0.02)?,
                DurationTicks::new(1),
                DurationTicks::new(1),
            )?);
        }
    }

    }
    let selected = if let Some(selected) = selected_override {
        selected
    } else {
        let mut selected = (0..grounded.slots().len().min(object_limit)).collect::<Vec<_>>();
        let contact_index = grounded
            .transports()
            .iter()
            .position(|transport| Some(transport.transport_entity) == active_contact)
            .filter(|index| grounded.slots()[*index].contact > 0.0);
        if let Some(index) = contact_index {
            if !selected.contains(&index) {
                if let Some(last) = selected.last_mut() {
                    *last = index;
                }
            }
        }
        if let Some(index) = grounded
            .slots()
            .iter()
            .position(|slot| Some(slot.tracked_object_id) == focal_object)
        {
            if !selected.contains(&index) {
                // Contact retains its reservation when focus competes for capacity.
                if let Some(replaced) = selected
                    .iter_mut()
                    .rev()
                    .find(|i| Some(**i) != contact_index)
                {
                    *replaced = index;
                }
            }
        }
        selected
    };
    for index in selected {
        let slot = &grounded.slots()[index];
        let transport = &grounded.transports()[index];
        if slot.slot_index != transport.slot_index {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let features = slot.candidate_features()?;
        let target = ActionTarget::new(
            Some(transport.transport_entity),
            Some(transport.target_position),
        );
        let observation = CandidateObservationRef::ObjectSlot(slot.slot_index);
        for (action_id, kind, family, effort) in OBJECT_ACTIONS {
            candidates.push(ActionCandidate::new(
                u16::try_from(candidates.len())
                    .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
                action_id,
                kind,
                family,
                observation,
                target,
                features,
                slot.confidence,
                NormalizedScalar::new(effort)?,
                DurationTicks::new(1),
                DurationTicks::new(1),
            )?);
        }
    }
    // A visible object must not force movement, rest, or manipulation. These
    // targetless choices remain in the existing motor channels.
    candidates.push(ActionCandidate::new(
        u16::try_from(candidates.len())
            .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
        HeadlessActionIds::NO_LOCOMOTION,
        ActionKind::Move,
        CandidateActionFamily::Approach,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector::zero().with_intrinsic_action_basis(ActionKind::Move),
        Confidence::new(1.0)?,
        NormalizedScalar::new(0.0)?,
        DurationTicks::new(1),
        DurationTicks::new(1),
    )?);
    candidates.push(ActionCandidate::new(
        u16::try_from(candidates.len())
            .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
        HeadlessActionIds::NO_MANIPULATION,
        ActionKind::Interact,
        CandidateActionFamily::Contact,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector::zero().with_intrinsic_action_basis(ActionKind::Interact),
        Confidence::new(1.0)?,
        NormalizedScalar::new(0.0)?,
        DurationTicks::new(1),
        DurationTicks::new(1),
    )?);
    candidates.push(ActionCandidate::new(
        u16::try_from(candidates.len())
            .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
        HeadlessActionIds::NO_POSTURE,
        ActionKind::Hold,
        CandidateActionFamily::Other,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector::zero().with_intrinsic_action_basis(ActionKind::Hold),
        Confidence::new(1.0)?,
        NormalizedScalar::new(0.0)?,
        DurationTicks::new(1),
        DurationTicks::new(1),
    )?);
    Ok(candidates)
}

fn push_intrinsic_candidates(
    candidates: &mut Vec<ActionCandidate>,
) -> Result<(), ScaffoldContractError> {
    for (kind, family, effort) in [
        (ActionKind::Idle, CandidateActionFamily::Idle, 0.0),
        (ActionKind::Rest, CandidateActionFamily::Rest, 0.05),
        (ActionKind::Vocalize, CandidateActionFamily::Other, 0.05),
    ] {
        candidates.push(ActionCandidate::new(
            u16::try_from(candidates.len())
                .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?,
            kind.canonical_id(),
            kind,
            family,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            CandidateFeatureVector::zero().with_intrinsic_action_basis(kind),
            Confidence::new(1.0)?,
            NormalizedScalar::new(effort)?,
            DurationTicks::new(1),
            DurationTicks::new(1),
        )?);
    }
    Ok(())
}

fn validate_report(report: &HeadlessSensoryReport) -> Result<(), ScaffoldContractError> {
    report
        .core_snapshot
        .validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    report
        .ecology
        .validate()
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    let mut ids = BTreeSet::new();
    for entity in &report.visible_entities {
        entity
            .id
            .validate()
            .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
        entity
            .relative_position
            .validate()
            .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
        let measured_distance = length(entity.relative_position);
        if !entity.distance.is_finite()
            || entity.distance < 0.0
            || (entity.distance - measured_distance).abs() > 1e-5
            || entity.distance > HEADLESS_VISION_RADIUS
            || entity.affordances.raw() & !KNOWN_AFFORDANCE_MASK != 0
            || !ids.insert(entity.id.raw())
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
    }
    let mut contact_ids = BTreeSet::new();
    for contact_id in &report.contact_entities {
        contact_id
            .validate()
            .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
        let Some(visible) = report
            .visible_entities
            .iter()
            .find(|entity| entity.id == *contact_id)
        else {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        };
        if !contact_ids.insert(contact_id.raw()) || visible.distance > HEADLESS_CONTACT_RADIUS {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
    }
    let geometric_contact_ids = report
        .visible_entities
        .iter()
        .filter(|entity| entity.distance <= HEADLESS_CONTACT_RADIUS)
        .map(|entity| entity.id.raw())
        .collect::<BTreeSet<_>>();
    if contact_ids != geometric_contact_ids {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }
    Ok(())
}

fn features_for(
    report: &HeadlessSensoryReport,
    entity: &VisibleWorldEntity,
) -> Result<CandidateFeatureVector, ScaffoldContractError> {
    debug_assert_eq!(CANDIDATE_FEATURE_RESERVED_START_LANE, 18);
    debug_assert_eq!(CANDIDATE_FEATURE_COUNT, 24);
    // Match the grounded profile and Y-up game space, preserving [sin, cos].
    let planar_length = entity.relative_position.x.hypot(entity.relative_position.z);
    let (bearing_sin, bearing_cos) = if planar_length > f32::EPSILON {
        (
            entity.relative_position.z / planar_length,
            entity.relative_position.x / planar_length,
        )
    } else {
        (0.0, 1.0)
    };
    let mut values = [0.0_f32; CANDIDATE_FEATURE_COUNT];
    values[CANDIDATE_FEATURE_BEARING_SIN_LANE] = bearing_sin;
    values[CANDIDATE_FEATURE_BEARING_COS_LANE] = bearing_cos;
    values[CANDIDATE_FEATURE_DISTANCE_LANE] = entity.distance / HEADLESS_VISION_RADIUS;
    values[CANDIDATE_FEATURE_RELATIVE_VELOCITY_X_LANE] = 0.0;
    values[CANDIDATE_FEATURE_RELATIVE_VELOCITY_Y_LANE] = 0.0;
    values[CANDIDATE_FEATURE_RELATIVE_VELOCITY_Z_LANE] = 0.0;
    for lane_offset in 0..CANDIDATE_FEATURE_AFFORDANCE_COUNT {
        let bit = AffordanceBits(1 << lane_offset);
        values[CANDIDATE_FEATURE_AFFORDANCE_START_LANE + lane_offset] =
            if entity.affordances.contains(bit) {
                1.0
            } else {
                0.0
            };
    }
    values[CANDIDATE_FEATURE_CONTACT_LANE] = if report.contact_entities.contains(&entity.id) {
        1.0
    } else {
        0.0
    };
    values[CANDIDATE_FEATURE_EVIDENCE_LANE] = 1.0;
    let features = CandidateFeatureVector(values);
    features.validate()?;
    Ok(features)
}

fn add(left: Vec3f, right: Vec3f) -> Vec3f {
    Vec3f::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn length(value: Vec3f) -> f32 {
    (value.x * value.x + value.y * value.y + value.z * value.z).sqrt()
}

#[cfg(test)]
mod founder_basis_tests {
    use super::*;
    use crate::HeadlessScenarioBuilder;
    use alife_core::{HomeostaticSnapshot, OrganismId, Tick};

    #[test]
    fn intrinsic_choices_have_learning_support_without_changing_object_evidence() {
        let mut world = HeadlessScenarioBuilder::new(60_011)
            .agent("observer", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(3.0, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        let frame = world
            .perception_frame_draft(
                OrganismId(1),
                Tick::ZERO,
                SensorProfile::GroundedTerrainVisionV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        for candidate in frame.candidates() {
            if candidate.target == ActionTarget::NONE {
                assert_eq!(
                    candidate.features.0[CandidateFeatureVector::INTRINSIC_ACTION_BASIS_LANE],
                    1.0
                );
            } else if let CandidateObservationRef::ObjectSlot(index) = candidate.observation {
                let slot = frame
                    .grounded_object_slots()
                    .iter()
                    .find(|s| s.slot_index == index)
                    .unwrap();
                assert_eq!(
                    candidate.features.0[CandidateFeatureVector::INTRINSIC_ACTION_BASIS_LANE],
                    slot.terrain[1]
                );
                assert_eq!(candidate.features.0[15], 0.0, "distant taste stays absent");
            }
        }
        assert!(frame
            .sensory()
            .channels
            .smell_chemistry
            .iter()
            .any(|v| *v > 0.0));
        let rest = frame
            .candidates()
            .iter()
            .find(|c| c.family == CandidateActionFamily::Rest)
            .unwrap();
        let vocal = frame
            .candidates()
            .iter()
            .find(|c| c.kind == ActionKind::Vocalize)
            .unwrap();
        let hold = frame
            .candidates()
            .iter()
            .find(|c| c.action_id == HeadlessActionIds::NO_POSTURE)
            .unwrap();
        assert_eq!(vocal.family, hold.family);
        assert_ne!(vocal.features, hold.features);
        // d(logit)/d(weight) = motor * feature is now nonzero for a nonzero
        // motor activation, so both offline and local action learning can act.
        assert_ne!(
            0.5 * rest.features.0[CandidateFeatureVector::INTRINSIC_ACTION_BASIS_LANE],
            0.0
        );
        let mut forged = frame.candidates().to_vec();
        forged[usize::from(rest.candidate_index)].features.0[22] = 1.0;
        assert_eq!(
            alife_core::PerceptionFrameDraft::new(
                frame.organism_id(),
                frame.tick(),
                frame.sensor_profile(),
                frame.sensory().clone(),
                frame.body(),
                *frame.homeostasis(),
                forged,
                frame.profile_provenance(),
                frame.grounded_object_slots().to_vec(),
            ),
            Err(ScaffoldContractError::InvalidPerceptionFrame),
            "the action descriptor cannot fabricate a different opportunity",
        );
    }
}

#[cfg(test)]
mod opportunity_coverage_tests {
    use super::*;
    use crate::{GroundedSensorExtractor, HeadlessScenarioBuilder, TrackedObjectRegistry};
    use alife_core::{OrganismId, Tick};
    fn fixture(count: usize, reverse: bool) -> GroundedSensingFrame {
        fixture_shape(count, reverse, false)
    }
    fn fixture_shape(count: usize, reverse: bool, ties: bool) -> GroundedSensingFrame {
        let mut builder =
            HeadlessScenarioBuilder::new(2313).agent("owner", OrganismId(1), Vec3f::ZERO);
        for i in 0..count {
            builder = builder.toy(
                &format!("o{i}"),
                Vec3f::new(0.5 + i as f32 * 0.4, 0.0, 0.0),
                true,
            );
        }
        let world = builder.build().unwrap();
        let mut snapshot = world
            .physical_observation_snapshot(OrganismId(1), Tick::ZERO)
            .unwrap();
        snapshot.visible.last_mut().unwrap().contact = true;
        if ties {
            for o in &mut snapshot.visible {
                o.position = Vec3f::new(1.0, 0.0, 0.0);
            }
        }
        if reverse {
            snapshot.visible.reverse();
        }
        GroundedSensorExtractor::extract(
            &snapshot,
            &mut TrackedObjectRegistry::new(2313, 16).unwrap(),
        )
        .unwrap()
    }
    fn targets(g: &GroundedSensingFrame, slots: &[u16]) -> Vec<u64> {
        slots
            .iter()
            .map(|&i| g.slots()[i as usize].tracked_object_id.raw())
            .collect()
    }
    #[test]
    fn maze_heading_has_only_three_exact_native_primitives_and_capacity_three() {
        let g=fixture(8,false);
        let contact=Some(g.transports()[7].transport_entity);
        let focal=Some(g.slots()[6].tracked_object_id);
        for phase in 0..6 {
            let (c,r)=enumerate_maze_heading2346(&g,SensorProfile::GroundedObjectSlotsV1,contact,focal,CandidateOpportunityCoveragePlan::new(phase),0).unwrap();
            let (_,old)=enumerate_grounded_candidates_with_coverage(&g,SensorProfile::GroundedObjectSlotsV1,contact,focal,true,CandidateOpportunityCoveragePlan::new(phase),0).unwrap();
            assert_eq!(r,old); // same contact/focus/rotating set and selector work
            assert_eq!(r.as_ref().unwrap().capacity(),3);
            assert!(c.len()<=32);
            for (id,sin,cos) in [(105,0.0,1.0),(604,1.0,0.0),(605,-1.0,0.0)] {
                let matches=c.iter().filter(|c|c.action_id.raw()==id).collect::<Vec<_>>();
                assert_eq!(matches.len(),1);let a=matches[0];
                assert_eq!(a.observation,CandidateObservationRef::None);assert_eq!(a.target,ActionTarget::NONE);
                let mut expected=[0.0;CANDIDATE_FEATURE_COUNT];expected[0]=sin;expected[1]=cos;expected[19]=1.0;
                let expected=CandidateFeatureVector(expected).with_intrinsic_action_basis(a.kind);
                assert_eq!(a.features.0.map(f32::to_bits),expected.0.map(f32::to_bits));
            }
            assert!(!c.iter().any(|a|matches!(a.action_id.raw(),601|602|603|606)));
        }
    }
    #[test]
    fn crowded_stable_roster_covers_every_object_without_overwriting_protection() {
        let g = fixture(8, false);
        let contact = Some(g.transports()[7].transport_entity);
        let focal = Some(g.slots()[6].tracked_object_id);
        for orientation in [false, true] {
            let cap = if orientation { 3 } else { 4 };
            let mut covered = BTreeSet::new();
            for phase in 0..6 {
                let (c, r) = enumerate_grounded_candidates_with_coverage(
                    &g,
                    SensorProfile::GroundedObjectSlotsV1,
                    contact,
                    focal,
                    orientation,
                    CandidateOpportunityCoveragePlan::new(phase),
                    0,
                )
                .unwrap();
                let r = r.unwrap();
                assert_eq!(r.capacity() as usize, cap);
                assert!(r.selected_slots().contains(&7));
                assert!(r.selected_slots().contains(&6));
                assert_eq!(r.selected_slots().len(), cap);
                assert_eq!(
                    r.selected_slots().iter().collect::<BTreeSet<_>>().len(),
                    cap
                );
                covered.extend(r.selected_slots().iter().copied());
                assert!(c.len() <= 32);
                for &slot in r.selected_slots() {
                    assert_eq!(
                        c.iter()
                            .filter(|x| x.observation == CandidateObservationRef::ObjectSlot(slot))
                            .count(),
                        6
                    );
                }
            }
            assert_eq!(covered, (0u16..8).collect());
        }
    }
    #[test]
    fn reorder_ties_and_disappearance_depend_only_on_current_observed_roster() {
        let g = fixture(8, false);
        let reversed = fixture(8, true);
        assert_eq!(g.slots(), reversed.slots());
        for phase in [0, 1, 7, u64::MAX] {
            let a = enumerate_grounded_candidates_with_coverage(
                &g,
                SensorProfile::GroundedObjectSlotsV1,
                None,
                None,
                false,
                CandidateOpportunityCoveragePlan::new(phase),
                0,
            )
            .unwrap();
            let b = enumerate_grounded_candidates_with_coverage(
                &reversed,
                SensorProfile::GroundedObjectSlotsV1,
                None,
                None,
                false,
                CandidateOpportunityCoveragePlan::new(phase),
                0,
            )
            .unwrap();
            assert_eq!(a, b);
            let smaller = fixture(5, false);
            let (_, r) = enumerate_grounded_candidates_with_coverage(
                &smaller,
                SensorProfile::GroundedObjectSlotsV1,
                Some(WorldEntityId(999)),
                Some(TrackedObjectId(999)),
                false,
                CandidateOpportunityCoveragePlan::new(phase),
                0,
            )
            .unwrap();
            let r = r.unwrap();
            assert_eq!(r.contact_slot(), None);
            assert_eq!(r.focal_slot(), None);
            assert!(r.selected_slots().iter().all(|&i| i < 5));
        }
        assert_eq!(targets(&g, &[0, 1, 2]).len(), 3);
    }
    #[test]
    fn uncrowded_is_legacy_exact_and_capacity_cannot_sacrifice_reservations() {
        let g = fixture(3, false);
        for phase in [0, u64::MAX] {
            let (c, r) = enumerate_grounded_candidates_with_coverage(
                &g,
                SensorProfile::GroundedObjectSlotsV1,
                None,
                None,
                false,
                CandidateOpportunityCoveragePlan::new(phase),
                0,
            )
            .unwrap();
            assert!(r.is_none());
            assert_eq!(
                c,
                enumerate_grounded_candidates(&g, SensorProfile::GroundedObjectSlotsV1, None)
                    .unwrap()
            );
        }
        let g = fixture(8, false);
        let contact = Some(g.transports()[7].transport_entity);
        let focal = Some(g.slots()[6].tracked_object_id);
        assert!(coverage_selection(&g, contact, focal, 0, 0).is_err());
        assert!(coverage_selection(&g, contact, focal, 1, 0).is_err());
        assert!(coverage_selection(&g, contact, focal, 2, 0).is_err());
        assert!(coverage_selection(&g, contact, focal, 3, 0).is_ok());
    }
    #[test]
    fn physical_distance_ties_are_stable_and_binding_authenticates_initial_phase() {
        let a = fixture_shape(8, false, true);
        let b = fixture_shape(8, true, true);
        for phase in 0..8 {
            let select = |g: &GroundedSensingFrame| {
                enumerate_grounded_candidates_with_coverage(
                    g,
                    SensorProfile::GroundedObjectSlotsV1,
                    None,
                    None,
                    false,
                    CandidateOpportunityCoveragePlan::new(phase),
                    0,
                )
                .unwrap()
            };
            assert_eq!(select(&a), select(&b));
        }
        let g = fixture(8, false);
        let plan = CandidateOpportunityCoveragePlan::new(3);
        let (c, r) = enumerate_grounded_candidates_with_coverage(
            &g,
            SensorProfile::GroundedObjectSlotsV1,
            None,
            None,
            false,
            plan,
            0,
        )
        .unwrap();
        let r = r.unwrap();
        let (sensory, body, slots, _) = g.clone().into_parts();
        let d = PerceptionFrameDraft::new(
            OrganismId(1),
            Tick::ZERO,
            SensorProfile::GroundedObjectSlotsV1,
            sensory,
            body,
            alife_core::HomeostaticSnapshot::baseline(Tick::ZERO),
            c,
            alife_core::SensorProfileProvenance::new(
                SensorProfile::GroundedObjectSlotsV1,
                alife_core::SensoryAbiVersion::CURRENT,
                Tick::ZERO,
            )
            .unwrap(),
            slots,
        )
        .unwrap()
        .with_opportunity_coverage(r.clone())
        .unwrap();
        let binding = GroundedCandidateBinding::new(
            g.clone(),
            SensorProfile::GroundedObjectSlotsV1,
            None,
            false,
        )
        .with_coverage(plan);
        let focus = Some(g.slots()[7].tracked_object_id);
        let (_, new) = binding
            .candidates_for_focus_with_coverage(&d, focus, r.work_units())
            .unwrap();
        let new = new.unwrap();
        assert_eq!(new.focal_slot(), Some(7));
        assert!(new.selected_slots().contains(&7));
        assert!(new.work_units() > r.work_units() * 2);
        assert!(binding.candidates_for_focus(&d, focus).is_err());
        assert!(binding
            .candidates_for_focus_with_coverage(&d, focus, 0)
            .is_err());
        let wrong =
            GroundedCandidateBinding::new(g, SensorProfile::GroundedObjectSlotsV1, None, false)
                .with_coverage(CandidateOpportunityCoveragePlan::new(4));
        assert!(wrong
            .candidates_for_focus_with_coverage(&d, focus, r.work_units())
            .is_err());
        assert!(GroundedCandidateBinding::new(
            fixture(3, false),
            SensorProfile::GroundedObjectSlotsV1,
            None,
            false
        )
        .with_coverage(plan)
        .coverage_plan()
        .is_none());
    }
    #[test]
    fn active_overlay_preserves_contact_focus_rotor_and_exact_baseline_work() {
        for reverse in [false, true] {
            let g = fixture_shape(8, reverse, true);
            for phase in 0..8 {
                let contact_index = g.slots().iter().position(|s| s.contact > 0.).unwrap();
                let focal_index = (contact_index + 1) % g.slots().len();
                let contact = Some(g.transports()[contact_index].transport_entity);
                let focal = Some(g.slots()[focal_index].tracked_object_id);
                let (_, base) = enumerate_grounded_candidates_with_coverage(&g, SensorProfile::GroundedObjectSlotsV1, contact, focal, false, CandidateOpportunityCoveragePlan::new(phase), 0).unwrap();
                let base = base.unwrap();
                let active = (0..8).find(|i| !base.selected_slots().contains(i)).unwrap();
                let overlay = active_opportunity_overlay(g.slots(), &base, Some(g.slots()[active as usize].tracked_object_id)).unwrap();
                assert!(overlay.admitted);
                assert_eq!(&overlay.final_selected_slots[..3], &base.selected_slots()[..3]);
                assert_eq!(overlay.final_selected_slots[3], active);
                assert_eq!(overlay.protected_rotation, Some(base.selected_slots()[2]));
                assert_eq!(overlay.final_selected_slots.iter().copied().collect::<BTreeSet<_>>().len(),4);
                assert!(overlay.work_units>0);
                let (_, baseline_again) = enumerate_grounded_candidates_with_coverage(&g, SensorProfile::GroundedObjectSlotsV1, contact, focal, false, CandidateOpportunityCoveragePlan::new(phase), 0).unwrap();
                assert_eq!(base,baseline_again.unwrap());
            }
        }
    }
    #[test]
    fn active_overlay_duplicate_missing_none_and_no_spare_leave_selection() {
        let g=fixture(8,false);
        for capacity in [0,1,3] {
            let selected=match capacity {0=>vec![],1=>vec![2],_=>vec![0,1,2]};
            let base=CandidateOpportunityCoverageV1::new(8,capacity, if capacity==3 {Some(0)} else {None},if capacity==3 {Some(1)} else {None},0,selected,50).unwrap();
            let r=active_opportunity_overlay(g.slots(),&base,Some(g.slots()[7].tracked_object_id)).unwrap();
            assert!(!r.admitted);assert_eq!(r.reason,ActiveApproachOpportunityReason::NoSpareCapacity);assert_eq!(r.final_selected_slots,base.selected_slots());
        }
        let base=CandidateOpportunityCoverageV1::new(8,4,Some(0),Some(0),0,vec![0,1,2,3],50).unwrap();
        let r=active_opportunity_overlay(g.slots(),&base,Some(g.slots()[7].tracked_object_id)).unwrap();
        assert!(r.admitted);assert_eq!(r.protected_rotation,Some(1));assert_eq!(r.final_selected_slots,vec![0,1,2,7]);
        let r=active_opportunity_overlay(g.slots(),&base,Some(g.slots()[0].tracked_object_id)).unwrap();assert_eq!(r.reason,ActiveApproachOpportunityReason::AlreadySelected);assert_eq!(r.final_selected_slots,base.selected_slots());
        let r=active_opportunity_overlay(g.slots(),&base,None).unwrap();assert_eq!(r.reason,ActiveApproachOpportunityReason::NoActive);assert_eq!(r.final_selected_slots,base.selected_slots());
        let r=active_opportunity_overlay(&g.slots()[..7],&base,Some(g.slots()[7].tracked_object_id)).unwrap();assert_eq!(r.reason,ActiveApproachOpportunityReason::Unobserved);assert_eq!(r.final_selected_slots,base.selected_slots());
    }
    #[test]
    fn active_binding_validates_initial_draft_and_bills_overlay_separately() {
        for count in [3,8] {
            let g=fixture(count,false);let plan=CandidateOpportunityCoveragePlan::new(3);
            let(c,initial)=enumerate_grounded_candidates_with_coverage(&g,SensorProfile::GroundedObjectSlotsV1,None,None,false,plan,0).unwrap();
            let(sensory,body,slots,_)=g.clone().into_parts();
            let mut draft=PerceptionFrameDraft::new(OrganismId(1),Tick::ZERO,SensorProfile::GroundedObjectSlotsV1,sensory,body,alife_core::HomeostaticSnapshot::baseline(Tick::ZERO),c,alife_core::SensorProfileProvenance::new(SensorProfile::GroundedObjectSlotsV1,alife_core::SensoryAbiVersion::CURRENT,Tick::ZERO).unwrap(),slots).unwrap();
            if let Some(r)=&initial {draft=draft.with_opportunity_coverage(r.clone()).unwrap();}
            let binding=GroundedCandidateBinding::new(g.clone(),SensorProfile::GroundedObjectSlotsV1,None,false).with_coverage(plan);
            assert!(!binding.active_approach_enabled());
            let work=initial.as_ref().map_or(0,|r|r.work_units());
            assert!(binding.candidates_for_focus_with_active_opportunity(&draft,None,work).is_err());
            let (old,oldreceipt)=if count==8 {binding.candidates_for_focus_with_coverage(&draft,None,work).unwrap()}else{(binding.candidates_for_focus(&draft,None).unwrap(),None)};
            let active=binding.clone().with_active_approach(None);assert!(active.active_approach_enabled());
            let(new,newreceipt,proof)=active.candidates_for_focus_with_active_opportunity(&draft,None,work).unwrap();
            assert_eq!(old,new);assert_eq!(oldreceipt,newreceipt);assert!(proof.work_units>0);
            let encoded=serde_json::to_vec(&proof).unwrap();assert_eq!(proof,serde_json::from_slice::<ActiveApproachOpportunityReceipt>(&encoded).unwrap());
            assert!(active.candidates_for_focus_with_active_opportunity(&draft,None,work+1).is_err());
            if count==8 {
                let base=newreceipt.unwrap();let idx=(0..8).find(|i|!base.selected_slots().contains(i)).unwrap();
                let with=binding.with_active_approach(Some(g.slots()[idx as usize].tracked_object_id));
                let(c,r,p)=with.candidates_for_focus_with_active_opportunity(&draft,None,work).unwrap();assert!(p.admitted);let r=r.unwrap();assert_eq!(r.work_units(),base.work_units());assert!(r.selected_slots().contains(&idx));assert!(c.len()<=32);
                draft.with_candidates_and_opportunity_coverage(c,r).unwrap();
            }
        }
    }

}

#[cfg(test)]
mod active_fixed_row_tests;

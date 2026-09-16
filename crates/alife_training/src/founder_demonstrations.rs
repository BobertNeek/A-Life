//! Offline teacher commands and their actual world/body consequences.

use alife_core::*;
use alife_world::{HeadlessScenarioBuilder, WorldOrganismRecord};
use serde::{Deserialize, Serialize};

use crate::TrainingError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FounderDemonstrationStep {
    pub observation: PerceptionFrameDraft,
    pub body_before: BiochemistryState,
    pub body_after: BiochemistryState,
    /// Supervision only. Never inserted into the sensory frame.
    pub teacher_candidate_index: u16,
    pub command: ActionCommand,
    pub execution: ReferenceActionExecution,
    pub world_before_digest: [u64; 4],
    pub world_after_digest: [u64; 4],
    pub position_before: Vec3f,
    pub position_after: Vec3f,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FounderDemonstration {
    pub seed: u64,
    pub body_phenotype: CreaturePhenotype,
    pub steps: Vec<FounderDemonstrationStep>,
}

/// A declared training-only demonstrator. It approaches one visible object,
/// inspects it within contact range, then samples it through ordinary ingestion.
/// The teacher has no choice between hidden nutritional contingencies.
pub fn record_founder_demonstration(
    seed: u64,
    target_position: Vec3f,
) -> Result<FounderDemonstration, TrainingError> {
    record_demonstration(seed, target_position, true)
}

/// Observation-grounded approach/sample curriculum without a hidden inspection
/// phase. Autonomous inspection is a separate, still-required behavior.
pub fn record_founder_sampling_demonstration(
    seed: u64,
    target_position: Vec3f,
) -> Result<FounderDemonstration, TrainingError> {
    record_demonstration(seed, target_position, false)
}

fn record_demonstration(
    seed: u64,
    target_position: Vec3f,
    inspect_first: bool,
) -> Result<FounderDemonstration, TrainingError> {
    target_position.validate()?;
    let organism = OrganismId(1);
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", organism, Vec3f::ZERO)
        .food("object", target_position, 1.0)
        .build()?;
    let entity = world.organism_entity_ids()[0].1;
    let genome = CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(
            FoundationId::N512_V1.raw(),
            1,
            FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
            BrainCapacityClass::N512_ID,
        )?,
    )?;
    let body_phenotype = genome.express()?;
    world.register_organism_record(WorldOrganismRecord::newborn(
        organism,
        entity,
        genome,
        body_phenotype.clone(),
        world.tick(),
    )?)?;
    let mut result = FounderDemonstration {
        seed,
        body_phenotype,
        steps: Vec::new(),
    };
    let mut inspected = !inspect_first;
    for _ in 0..32 {
        let body_before = world
            .organism_registry()
            .get(organism)
            .ok_or(ScaffoldContractError::InvalidId)?
            .biochemistry()
            .clone();
        let observation = world.perception_frame_draft(
            organism,
            world.tick(),
            SensorProfile::GroundedObjectSlotsV1,
            body_before.homeostasis,
        )?;
        let report = world.sensory_report(organism, world.tick())?;
        let target = report
            .visible_entities
            .first()
            .ok_or(ScaffoldContractError::MissingPhaseData)?
            .id;
        let family = if !report.contact_entities.contains(&target) {
            CandidateActionFamily::Approach
        } else if !inspected {
            CandidateActionFamily::Inspect
        } else {
            CandidateActionFamily::Ingest
        };
        let candidate = *observation
            .candidates()
            .iter()
            .find(|c| c.target.entity == Some(target) && c.family == family)
            .ok_or(ScaffoldContractError::InvalidActionDecision)?;
        let command = candidate.to_command(organism, Confidence::new(1.0)?)?;
        let world_before_digest = world.canonical_signature_digest()?.words;
        let position_before = world
            .entity(entity)
            .ok_or(ScaffoldContractError::InvalidId)?
            .position;
        let receipt = world.apply_registered_neural_command(
            &command,
            entity,
            Tick(world.tick().raw() + 1),
            None,
            false,
        )?;
        if !receipt.action_result.execution.succeeded {
            return Err(ScaffoldContractError::InvalidActionDecision.into());
        }
        world.advance_tick();
        let consumed =
            receipt.action_result.execution.physical.contact == PhysicalContactKind::Consumed;
        inspected |= family == CandidateActionFamily::Inspect;
        result.steps.push(FounderDemonstrationStep {
            observation,
            body_before,
            body_after: receipt.biology_after,
            teacher_candidate_index: candidate.candidate_index,
            command,
            execution: receipt.action_result.execution,
            world_before_digest,
            world_after_digest: world.canonical_signature_digest()?.words,
            position_before,
            position_after: world
                .entity(entity)
                .ok_or(ScaffoldContractError::InvalidId)?
                .position,
        });
        if consumed {
            return Ok(result);
        }
    }
    Err(ScaffoldContractError::MissingPhaseData.into())
}

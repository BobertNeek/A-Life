//! Production GPU active-battery runner over grounded headless challenge worlds.

use alife_core::{
    ActionKind, ActiveBatteryReceipt, ActiveChallengeKind, BrainCapacityClass, BrainGenome,
    CandidateActionFamily, ConsolidationIntent, CreatureGenome, DecisionSnapshot, DevelopmentState,
    ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId, FoundationWeightAsset, GenomeId,
    HomeostaticParameters, HomeostaticSnapshot, LanguageTokenId, LineageId, NeuralActionSelection,
    NormalizedScalar, OrganismId, PhenotypeCompiler, PhenotypeHash, PhysicalContactKind,
    PolicyBackend, PostActionOutcome, PreActionSnapshot, ScaffoldContractError, SensorProfile,
    SpeechActKind, SpeechMotorPayload, TeacherPerceptionChannel, Tick, UtteranceId,
    UtteranceSourceKind, Validate, Vec3f, WorldEntityId,
};
use alife_gpu_backend::{GpuBrainHandle, GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};
use alife_world::{
    HeadlessScenarioBuilder, HeadlessWorld, HeadlessWorldSignatureDigest, WorldObjectKind,
    HEADLESS_WORLD_SIGNATURE_SCHEMA_VERSION,
};

use crate::TrainingError;

const SCORE_ONE_Q16: u32 = 65_535;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveBatteryChallengeSpec {
    pub kind: ActiveChallengeKind,
    pub tick_budget: u32,
    pub world_object_count: u16,
    pub uses_grounded_sensing: bool,
    pub slm_enabled: bool,
}

impl ActiveBatteryChallengeSpec {
    pub fn all() -> Vec<Self> {
        ActiveChallengeKind::ALL
            .into_iter()
            .map(|kind| Self {
                kind,
                tick_budget: if kind == ActiveChallengeKind::PostSleepRetention {
                    8
                } else {
                    6
                },
                world_object_count: minimum_world_object_count(kind),
                uses_grounded_sensing: true,
                slm_enabled: false,
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveBatteryEvidence {
    pub receipt: ActiveBatteryReceipt,
    pub source_creature_genome_id: Option<GenomeId>,
    pub brain_genome_id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub lineage_id: Option<LineageId>,
    pub phenotype_hash: PhenotypeHash,
    pub foundation_id: u64,
    pub foundation_version: u32,
    pub compatibility_family_id: u64,
    pub challenge_worlds: u32,
    pub gpu_dispatches: u64,
    pub sealed_outcomes: u64,
    pub sleep_consolidations: u32,
    pub slm_enabled: bool,
    pub adapter_name: String,
    pub backend_api: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuReproductionIntentReceipt {
    pub initiator_organism_id: OrganismId,
    pub mate_organism_id: OrganismId,
    pub mate_entity_id: WorldEntityId,
    pub observed_ticks: u32,
    pub pre_action_world_digest: HeadlessWorldSignatureDigest,
    pub patch: ExperiencePatch,
}

impl Validate for GpuReproductionIntentReceipt {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.initiator_organism_id.validate()?;
        self.mate_organism_id.validate()?;
        self.mate_entity_id.validate()?;
        self.patch.validate_contract()?;
        if self.initiator_organism_id == self.mate_organism_id
            || self.observed_ticks == 0
            || self.pre_action_world_digest.schema_version
                != HEADLESS_WORLD_SIGNATURE_SCHEMA_VERSION
            || self.pre_action_world_digest.words == [0; 4]
            || self.patch.pre_action().organism_id != self.initiator_organism_id
            || self.patch.decision().policy_backend() != PolicyBackend::NeuralClosedLoopGpu
            || self.patch.decision().selected_action.kind != ActionKind::Interact
            || self.patch.decision().selected_action.target_entity != Some(self.mate_entity_id)
            || self.patch.decision().neural_evidence()?.action_family
                != CandidateActionFamily::Contact
            || !self.patch.outcome().success
            || self.patch.outcome().physical.contact != PhysicalContactKind::Touch
            || self.patch.outcome().physical.target_entity != Some(self.mate_entity_id)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

pub struct N2048ActiveBatteryRunner {
    session: GpuAuthoritativeSession,
    foundation: FoundationWeightAsset,
    capacity: BrainCapacityClass,
    adapter_name: String,
    backend_api: String,
}

impl N2048ActiveBatteryRunner {
    pub fn new_required() -> Result<Self, TrainingError> {
        let foundation =
            FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
        let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
        let hardware = backend.hardware_receipt();
        let adapter_name = hardware.adapter_name.clone();
        let backend_api = hardware.backend_api.clone();
        Ok(Self {
            session: GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Challenge),
            foundation,
            capacity: BrainCapacityClass::n2048(),
            adapter_name,
            backend_api,
        })
    }

    pub fn run_genetic_founder(
        &mut self,
        organism_id: OrganismId,
        genome_seed: u64,
    ) -> Result<ActiveBatteryEvidence, TrainingError> {
        if organism_id.raw() == 0 || genome_seed == 0 {
            return Err(ScaffoldContractError::InvalidId.into());
        }
        let genome = BrainGenome::scaffold(genome_seed, BrainCapacityClass::N2048_ID);
        self.run_brain_genome(
            organism_id,
            genome_seed,
            None,
            genome.clone(),
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0)?),
        )
    }

    /// Runs the exact expressed genetic brain from a validated creature genome
    /// through the production N2048 GPU challenge battery.
    pub fn run_creature_genome(
        &mut self,
        organism_id: OrganismId,
        creature_genome: &CreatureGenome,
    ) -> Result<ActiveBatteryEvidence, TrainingError> {
        if organism_id.raw() == 0 {
            return Err(ScaffoldContractError::InvalidId.into());
        }
        let (brain_genome, development) = self.express_compatible_creature(creature_genome)?;
        self.run_brain_genome(
            organism_id,
            creature_genome.conception_seed,
            Some(creature_genome.id),
            brain_genome,
            development,
        )
    }

    /// Observes the exact creature brain until its GPU policy selects a legal
    /// contact with the only available mate. Every observed decision is
    /// executed in the production headless world and returned to the GPU as a
    /// sealed causal outcome before the next tick.
    pub fn run_reproduction_intent(
        &mut self,
        initiator_organism_id: OrganismId,
        creature_genome: &CreatureGenome,
        mate_organism_id: OrganismId,
        max_ticks: u32,
    ) -> Result<GpuReproductionIntentReceipt, TrainingError> {
        let mut world = HeadlessScenarioBuilder::new(creature_genome.conception_seed)
            .agent("reproduction-initiator", initiator_organism_id, Vec3f::ZERO)
            .social_agent(
                "reproduction-mate",
                mate_organism_id,
                Vec3f::new(0.5, 0.0, 0.0),
                1.0,
            )
            .build()?;
        self.run_reproduction_intent_in_world(
            initiator_organism_id,
            creature_genome,
            mate_organism_id,
            &mut world,
            max_ticks,
        )
    }

    /// Runs reproduction intent against a caller-owned production world. The
    /// selected action mutates that world before its sealed outcome is returned
    /// to the GPU, allowing an authoritative runtime to commit both together.
    pub fn run_reproduction_intent_in_world(
        &mut self,
        initiator_organism_id: OrganismId,
        creature_genome: &CreatureGenome,
        mate_organism_id: OrganismId,
        world: &mut HeadlessWorld,
        max_ticks: u32,
    ) -> Result<GpuReproductionIntentReceipt, TrainingError> {
        self.run_reproduction_intent_in_world_internal(
            initiator_organism_id,
            creature_genome,
            Some(mate_organism_id),
            world,
            max_ticks,
        )
    }

    /// Lets the GPU network choose both the Contact action and its mate target
    /// from the caller-owned world. No parent pair is selected by the tool.
    pub fn run_creature_chosen_reproduction_intent_in_world(
        &mut self,
        initiator_organism_id: OrganismId,
        creature_genome: &CreatureGenome,
        world: &mut HeadlessWorld,
        max_ticks: u32,
    ) -> Result<GpuReproductionIntentReceipt, TrainingError> {
        self.run_reproduction_intent_in_world_internal(
            initiator_organism_id,
            creature_genome,
            None,
            world,
            max_ticks,
        )
    }

    fn run_reproduction_intent_in_world_internal(
        &mut self,
        initiator_organism_id: OrganismId,
        creature_genome: &CreatureGenome,
        expected_mate_organism_id: Option<OrganismId>,
        world: &mut HeadlessWorld,
        max_ticks: u32,
    ) -> Result<GpuReproductionIntentReceipt, TrainingError> {
        if initiator_organism_id.raw() == 0
            || max_ticks == 0
            || expected_mate_organism_id
                .is_some_and(|mate| mate.raw() == 0 || mate == initiator_organism_id)
        {
            return Err(ScaffoldContractError::InvalidId.into());
        }
        let (genome, mut development) = self.express_compatible_creature(creature_genome)?;
        let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &genome,
            &self.capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &self.foundation,
        )?;
        let handle = self
            .session
            .insert_brain(initiator_organism_id, phenotype)?;

        let attempt = (|| {
            let initiator_exists = world
                .organism_entity_ids()
                .into_iter()
                .any(|(organism, _)| organism == initiator_organism_id);
            if !initiator_exists {
                return Err(ScaffoldContractError::InvalidId);
            }
            if expected_mate_organism_id.is_some_and(|mate| {
                !world
                    .organism_entity_ids()
                    .into_iter()
                    .any(|(organism, _)| organism == mate)
            }) {
                return Err(ScaffoldContractError::InvalidId);
            }
            let mature_age = development.age_ticks.raw();
            let mut homeostasis = HomeostaticSnapshot::baseline(world.tick());
            homeostasis.drives.loneliness = 1.0;
            homeostasis.drives.curiosity = 0.8;
            homeostasis.drives.reproductive_drive = 1.0;
            homeostasis =
                HomeostaticSnapshot::new(world.tick(), homeostasis.drives, homeostasis.hormones)?;

            for step in 0..max_ticks {
                let pre_action_world_digest = world.canonical_signature_digest()?;
                development.age_ticks = Tick::new(mature_age.saturating_add(u64::from(step)));
                let frame = world.perception_frame(
                    initiator_organism_id,
                    world.tick(),
                    SensorProfile::GroundedObjectSlotsV1,
                    homeostasis,
                )?;
                let mut ticks = self.session.tick_batch(&[(handle, frame.clone())])?;
                let gpu_tick = ticks
                    .pop()
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let selection = NeuralActionSelection {
                    candidate_index: gpu_tick.selection.candidate_index,
                    logit: gpu_tick.selection.logit,
                    confidence: gpu_tick.selection.confidence,
                    active_tiles: gpu_tick.selection.active_tiles,
                    active_synapses: gpu_tick.selection.active_synapses,
                };
                let candidate = *frame
                    .candidates()
                    .get(usize::from(selection.candidate_index))
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let command = candidate.to_command(initiator_organism_id, selection.confidence)?;
                let sequence_id = ExperienceSequenceId(u64::from(step) + 1);
                let pre_action = PreActionSnapshot::from_neural_frame(
                    sequence_id,
                    handle.class_id(),
                    handle.phenotype_hash(),
                    genome.id,
                    genome.schema_version,
                    development.clone(),
                    frame.clone(),
                )?;
                let decision = DecisionSnapshot::from_neural_selection(
                    sequence_id,
                    handle.phenotype_hash(),
                    gpu_tick.dispatch_generation,
                    gpu_tick.active_activation_side,
                    &frame,
                    selection,
                    command,
                )?;
                let action_result =
                    world.apply_neural_command(&command, gpu_tick.speech_payload, false)?;
                let outcome_tick = Tick::new(frame.tick().raw().saturating_add(1));
                let mut outcome = PostActionOutcome::new(
                    initiator_organism_id,
                    sequence_id,
                    outcome_tick,
                    action_result.observation.success && action_result.execution.succeeded,
                    action_result.execution.physical,
                    action_result.observation.homeostatic_delta,
                    action_result.observation.reward_valence,
                    action_result.observation.frustration_delta,
                    action_result.observation.pain_delta,
                    action_result.observation.energy_delta,
                    action_result.observation.prediction_error,
                )?;
                outcome.contradiction_observed = action_result.observation.contradiction_observed
                    || !action_result.execution.succeeded;
                let patch = ExperiencePatchBuilder::new(sequence_id)
                    .record_pre_action(pre_action)?
                    .record_decision(decision)?
                    .record_outcome(outcome)?
                    .seal()?;
                self.session
                    .apply_sealed_outcome_batch(&[(handle, &patch)])?;

                let selected_mate =
                    patch
                        .decision()
                        .selected_action
                        .target_entity
                        .and_then(|entity_id| {
                            world.entity(entity_id).and_then(|entity| {
                                (entity.kind == WorldObjectKind::Agent)
                                    .then_some(entity.organism_id)
                                    .flatten()
                                    .map(|organism_id| (organism_id, entity_id))
                            })
                        });
                if let Some((mate_organism_id, mate_entity_id)) = selected_mate {
                    let matches_expected = expected_mate_organism_id
                        .map_or(true, |expected| expected == mate_organism_id);
                    let is_causal_contact = mate_organism_id != initiator_organism_id
                        && matches_expected
                        && patch.decision().selected_action.kind == ActionKind::Interact
                        && patch.decision().neural_evidence()?.action_family
                            == CandidateActionFamily::Contact
                        && patch.outcome().success
                        && patch.outcome().physical.contact == PhysicalContactKind::Touch
                        && patch.outcome().physical.target_entity == Some(mate_entity_id);
                    if is_causal_contact {
                        let receipt = GpuReproductionIntentReceipt {
                            initiator_organism_id,
                            mate_organism_id,
                            mate_entity_id,
                            observed_ticks: step + 1,
                            pre_action_world_digest,
                            patch,
                        };
                        receipt.validate_contract()?;
                        return Ok(receipt);
                    }
                }

                homeostasis = homeostasis.advance(
                    outcome_tick,
                    patch.outcome().homeostatic_delta,
                    HomeostaticParameters::reference(),
                )?;
                world.advance_tick();
            }
            Err(ScaffoldContractError::InvalidDecisionEvidence)
        })();
        let removal = self.session.remove_brain(handle);
        match (attempt, removal) {
            (Ok(receipt), Ok(())) => Ok(receipt),
            (Err(error), _) | (_, Err(error)) => Err(error.into()),
        }
    }

    fn express_compatible_creature(
        &self,
        creature_genome: &CreatureGenome,
    ) -> Result<(BrainGenome, DevelopmentState), TrainingError> {
        creature_genome.validate_contract()?;
        let manifest = self.foundation.manifest();
        if creature_genome.foundation.brain_class_id != self.capacity.id()
            || creature_genome.foundation.foundation_id != manifest.foundation_id().raw()
            || u32::from(creature_genome.foundation.version) != manifest.foundation_version().raw()
            || creature_genome.foundation.compatibility_family_id
                != manifest.compatibility_family_id().raw()
        {
            return Err(ScaffoldContractError::IncompatibleGeneticClass.into());
        }
        let expressed = creature_genome.express()?;
        let mature_tick = Tick::new(u64::from(expressed.development.maturation_duration_ticks));
        let development = expressed.development_state_at(mature_tick)?;
        Ok((expressed.brain_genome, development))
    }

    fn run_brain_genome(
        &mut self,
        organism_id: OrganismId,
        challenge_base_seed: u64,
        source_creature_genome_id: Option<GenomeId>,
        genome: BrainGenome,
        development: DevelopmentState,
    ) -> Result<ActiveBatteryEvidence, TrainingError> {
        let mut receipt = ActiveBatteryReceipt::empty(organism_id);
        let mut gpu_dispatches = 0_u64;
        let mut sealed_outcomes = 0_u64;
        let mut sleep_consolidations = 0_u32;
        let mut phenotype_hash = None;

        for (challenge_index, spec) in ActiveBatteryChallengeSpec::all().into_iter().enumerate() {
            let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
                &genome,
                &self.capacity,
                &development,
                SensorProfile::GroundedObjectSlotsV1,
                &self.foundation,
            )?;
            let compiled_hash = phenotype.phenotype_hash();
            match phenotype_hash {
                None => phenotype_hash = Some(compiled_hash),
                Some(expected) if expected == compiled_hash => {}
                Some(_) => return Err(ScaffoldContractError::PhenotypeCompile.into()),
            }
            let handle = self.session.insert_brain(organism_id, phenotype)?;
            let challenge_seed = challenge_base_seed
                .wrapping_add((challenge_index as u64 + 1).wrapping_mul(0x9E37_79B9));
            let mut world = build_challenge_world(spec.kind, challenge_seed, organism_id)?;
            prime_challenge_language(spec.kind, &mut world, organism_id)?;
            let score = run_challenge(
                &mut self.session,
                handle,
                &genome,
                development.clone(),
                &mut world,
                spec,
                &mut gpu_dispatches,
                &mut sealed_outcomes,
                &mut sleep_consolidations,
            )?;
            self.session.remove_brain(handle)?;
            receipt.record(spec.kind, score)?;
        }
        receipt.validate_contract()?;
        let phenotype_hash =
            phenotype_hash.ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let foundation_manifest = self.foundation.manifest();

        Ok(ActiveBatteryEvidence {
            receipt,
            source_creature_genome_id,
            brain_genome_id: genome.id,
            parent_genome_ids: genome.parent_genome_ids,
            lineage_id: genome.lineage_id,
            phenotype_hash,
            foundation_id: foundation_manifest.foundation_id().raw(),
            foundation_version: foundation_manifest.foundation_version().raw(),
            compatibility_family_id: foundation_manifest.compatibility_family_id().raw(),
            challenge_worlds: ActiveChallengeKind::ALL.len() as u32,
            gpu_dispatches,
            sealed_outcomes,
            sleep_consolidations,
            slm_enabled: false,
            adapter_name: self.adapter_name.clone(),
            backend_api: self.backend_api.clone(),
        })
    }
}

/// Independently recompiles the supplied creature against the shipped N2048
/// foundation and verifies that the GPU battery evidence names that phenotype.
pub fn verify_n2048_creature_evidence_phenotype(
    creature_genome: &CreatureGenome,
    evidence: &ActiveBatteryEvidence,
) -> Result<PhenotypeHash, TrainingError> {
    let expected = expected_n2048_creature_phenotype_hash(creature_genome)?;
    creature_genome.validate_contract()?;
    let foundation = FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
    let manifest = foundation.manifest();
    let expressed = creature_genome.express()?;
    if evidence.source_creature_genome_id != Some(creature_genome.id)
        || evidence.brain_genome_id != expressed.brain_genome.id
        || evidence.parent_genome_ids != creature_genome.parent_genome_ids
        || evidence.lineage_id != Some(creature_genome.lineage_id)
        || evidence.foundation_id != manifest.foundation_id().raw()
        || evidence.foundation_version != manifest.foundation_version().raw()
        || evidence.compatibility_family_id != manifest.compatibility_family_id().raw()
        || evidence.phenotype_hash != expected
    {
        return Err(ScaffoldContractError::PhenotypeCompile.into());
    }
    Ok(expected)
}

pub fn expected_n2048_creature_phenotype_hash(
    creature_genome: &CreatureGenome,
) -> Result<PhenotypeHash, TrainingError> {
    creature_genome.validate_contract()?;
    let foundation = FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
    let expressed = creature_genome.express()?;
    let development = expressed.development_state_at(Tick::new(u64::from(
        expressed.development.maturation_duration_ticks,
    )))?;
    Ok(PhenotypeCompiler::compile_from_foundation_asset(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &foundation,
    )?
    .phenotype_hash())
}

#[derive(Default)]
struct ChallengeScore {
    qualifying: u32,
    observations: u32,
    sleep_completed: bool,
}

#[allow(clippy::too_many_arguments)]
fn run_challenge(
    session: &mut GpuAuthoritativeSession,
    handle: GpuBrainHandle,
    genome: &BrainGenome,
    mut development: DevelopmentState,
    world: &mut HeadlessWorld,
    spec: ActiveBatteryChallengeSpec,
    gpu_dispatches: &mut u64,
    sealed_outcomes: &mut u64,
    sleep_consolidations: &mut u32,
) -> Result<u32, ScaffoldContractError> {
    let mut homeostasis = challenge_homeostasis(spec.kind, world.tick())?;
    let mut score = ChallengeScore::default();

    for step in 0..spec.tick_budget {
        alter_challenge_at_step(spec.kind, world, step, spec.tick_budget)?;
        development.age_ticks = world.tick();
        let frame = world.perception_frame(
            handle.organism_id(),
            world.tick(),
            SensorProfile::GroundedObjectSlotsV1,
            homeostasis,
        )?;
        let mut ticks = session.tick_batch(&[(handle, frame.clone())])?;
        let gpu_tick = ticks
            .pop()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        *gpu_dispatches = gpu_dispatches.saturating_add(1);
        let selection = NeuralActionSelection {
            candidate_index: gpu_tick.selection.candidate_index,
            logit: gpu_tick.selection.logit,
            confidence: gpu_tick.selection.confidence,
            active_tiles: gpu_tick.selection.active_tiles,
            active_synapses: gpu_tick.selection.active_synapses,
        };
        let candidate = *frame
            .candidates()
            .get(usize::from(selection.candidate_index))
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let command = candidate.to_command(handle.organism_id(), selection.confidence)?;
        let sequence_id = ExperienceSequenceId(u64::from(step) + 1);
        let pre_action = PreActionSnapshot::from_neural_frame(
            sequence_id,
            handle.class_id(),
            handle.phenotype_hash(),
            genome.id,
            genome.schema_version,
            development.clone(),
            frame.clone(),
        )?;
        let decision = DecisionSnapshot::from_neural_selection(
            sequence_id,
            handle.phenotype_hash(),
            gpu_tick.dispatch_generation,
            gpu_tick.active_activation_side,
            &frame,
            selection,
            command,
        )?;
        let prompted = frame
            .sensory()
            .language_context
            .heard_tokens
            .iter()
            .flatten()
            .any(|token| token.source_kind == UtteranceSourceKind::Player);
        let target_kind = command
            .target_entity
            .and_then(|entity| world.entity(entity))
            .map(|entity| entity.kind);
        let action_result =
            world.apply_neural_command(&command, gpu_tick.speech_payload, prompted)?;
        let outcome_tick = Tick::new(frame.tick().raw().saturating_add(1));
        let mut outcome = PostActionOutcome::new(
            handle.organism_id(),
            sequence_id,
            outcome_tick,
            action_result.observation.success && action_result.execution.succeeded,
            action_result.execution.physical,
            action_result.observation.homeostatic_delta,
            action_result.observation.reward_valence,
            action_result.observation.frustration_delta,
            action_result.observation.pain_delta,
            action_result.observation.energy_delta,
            action_result.observation.prediction_error,
        )?;
        outcome.contradiction_observed =
            action_result.observation.contradiction_observed || !action_result.execution.succeeded;
        let patch = ExperiencePatchBuilder::new(sequence_id)
            .record_pre_action(pre_action)?
            .record_decision(decision)?
            .record_outcome(outcome)?
            .seal()?;
        session.apply_sealed_outcome_batch(&[(handle, &patch)])?;
        *sealed_outcomes = sealed_outcomes.saturating_add(1);

        score.observations = score.observations.saturating_add(1);
        if challenge_tick_succeeded(
            spec.kind,
            step,
            spec.tick_budget,
            command.kind,
            candidate.family,
            target_kind,
            &action_result,
            score.sleep_completed,
        ) {
            score.qualifying = score.qualifying.saturating_add(1);
        }

        homeostasis = homeostasis.advance(
            outcome_tick,
            patch.outcome().homeostatic_delta,
            HomeostaticParameters::reference(),
        )?;
        world.advance_tick();

        if spec.kind == ActiveChallengeKind::PostSleepRetention && step + 1 == spec.tick_budget / 2
        {
            let replay = session.build_sleep_replay_batch(handle)?;
            let request = session.prepare_sleep_consolidation(
                handle,
                ConsolidationIntent { cycle_id: 1 },
                &replay,
            )?;
            let job = session.submit_sleep_consolidation(handle, &request, &replay)?;
            let staged = session
                .poll_sleep_consolidation(handle, job)?
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            session.commit_sleep_consolidation(handle, &request, &staged.staged)?;
            score.sleep_completed = true;
            *sleep_consolidations = sleep_consolidations.saturating_add(1);
        }
    }

    let qualifying =
        if spec.kind == ActiveChallengeKind::PostSleepRetention && !score.sleep_completed {
            0
        } else {
            score.qualifying
        };
    Ok(ratio_q16(qualifying, score.observations))
}

fn build_challenge_world(
    kind: ActiveChallengeKind,
    seed: u64,
    organism_id: OrganismId,
) -> Result<HeadlessWorld, ScaffoldContractError> {
    let base = HeadlessScenarioBuilder::new(seed).agent("candidate", organism_id, Vec3f::ZERO);
    let builder = match kind {
        ActiveChallengeKind::VisibleRewardNavigation => base
            .food("reward", Vec3f::new(2.0, 0.0, 0.0), 1.0)
            .obstacle("landmark", Vec3f::new(-2.0, 0.0, 0.0), 0.4),
        ActiveChallengeKind::BlockedRouteDetour => base
            .food("reward", Vec3f::new(3.0, 0.0, 0.0), 1.0)
            .obstacle("blocker", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .token("detour", Vec3f::new(0.0, 2.0, 0.0), 101),
        ActiveChallengeKind::DangerousShortVsSafeLong => base
            .food("safe-reward", Vec3f::new(3.0, 1.5, 0.0), 1.0)
            .hazard("short-hazard", Vec3f::new(1.0, 0.0, 0.0), 1.0),
        ActiveChallengeKind::RewardHazardReversal => base
            .food("reversal-reward", Vec3f::new(2.0, 0.0, 0.0), 1.0)
            .hazard("reversal-hazard", Vec3f::new(-2.0, 0.0, 0.0), 1.0),
        ActiveChallengeKind::DelayedChoice => base
            .food("delayed-reward", Vec3f::new(2.5, 0.0, 0.0), 1.0)
            .token("cue", Vec3f::new(0.5, 0.0, 0.0), 102),
        ActiveChallengeKind::UnfamiliarEdibility => base
            .food("novel-food", Vec3f::new(2.0, 0.5, 0.0), 1.0)
            .hazard("novel-distractor", Vec3f::new(2.0, -0.5, 0.0), 0.8),
        ActiveChallengeKind::PostSleepRetention => base
            .food("retention-reward", Vec3f::new(2.0, 0.0, 0.0), 1.0)
            .hazard("retention-hazard", Vec3f::new(-2.0, 0.0, 0.0), 0.8),
        ActiveChallengeKind::LayoutAppearanceGeneralization => base
            .food("shifted-reward", Vec3f::new(-2.5, 1.0, 0.0), 1.0)
            .obstacle("shifted-landmark", Vec3f::new(1.5, -1.0, 0.0), 0.5),
        ActiveChallengeKind::InjuryFatigueRecovery => base
            .food("recovery-food", Vec3f::new(2.0, 0.0, 0.0), 0.8)
            .hazard("recovery-hazard", Vec3f::new(-1.0, 0.0, 0.0), 0.6),
        ActiveChallengeKind::NameAddressedInstruction
        | ActiveChallengeKind::WordObjectGrounding
        | ActiveChallengeKind::ActionWordGrounding
        | ActiveChallengeKind::WhatWhyNarration => base
            .food("language-object", Vec3f::new(2.0, 0.0, 0.0), 0.8)
            .token("language-marker", Vec3f::new(-1.0, 0.0, 0.0), 103),
        ActiveChallengeKind::PeerTaughtAlias | ActiveChallengeKind::SlmDisabledDialectTransfer => {
            base.social_agent(
                "peer",
                OrganismId(organism_id.raw() + 1),
                Vec3f::new(0.5, 0.0, 0.0),
                0.5,
            )
            .food("peer-object", Vec3f::new(2.0, 0.0, 0.0), 0.8)
        }
    };
    builder.build()
}

fn prime_challenge_language(
    kind: ActiveChallengeKind,
    world: &mut HeadlessWorld,
    organism_id: OrganismId,
) -> Result<(), ScaffoldContractError> {
    match kind {
        ActiveChallengeKind::NameAddressedInstruction => {
            world.emit_player_tokens(
                Some(organism_id),
                Vec3f::new(0.25, 0.0, 0.0),
                vec![LanguageTokenId::new(193)?, LanguageTokenId::new(1)?],
            )?;
        }
        ActiveChallengeKind::WordObjectGrounding
        | ActiveChallengeKind::ActionWordGrounding
        | ActiveChallengeKind::WhatWhyNarration => {
            world.emit_teacher_tokens(
                Some(organism_id),
                Vec3f::new(0.25, 0.0, 0.0),
                vec![LanguageTokenId::new(41)?, LanguageTokenId::new(113)?],
                TeacherPerceptionChannel::Hearing,
            )?;
        }
        ActiveChallengeKind::PeerTaughtAlias | ActiveChallengeKind::SlmDisabledDialectTransfer => {
            world.emit_creature_utterance(
                UtteranceId::new(1)?,
                OrganismId(organism_id.raw() + 1),
                Some(organism_id),
                SpeechMotorPayload::try_new(
                    SpeechActKind::Declare,
                    vec![LanguageTokenId::new(129)?],
                    alife_core::Confidence::new(1.0)?,
                )?,
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn alter_challenge_at_step(
    kind: ActiveChallengeKind,
    world: &mut HeadlessWorld,
    step: u32,
    tick_budget: u32,
) -> Result<(), ScaffoldContractError> {
    if kind == ActiveChallengeKind::RewardHazardReversal && step == tick_budget / 2 {
        let reward = world
            .entity_id("reversal-reward")
            .ok_or(ScaffoldContractError::InvalidId)?;
        let hazard = world
            .entity_id("reversal-hazard")
            .ok_or(ScaffoldContractError::InvalidId)?;
        world.editor_move_object(reward, Vec3f::new(-2.0, 0.0, 0.0))?;
        world.editor_move_object(hazard, Vec3f::new(2.0, 0.0, 0.0))?;
    }
    if kind == ActiveChallengeKind::DelayedChoice && step == 1 {
        let cue = world
            .entity_id("cue")
            .ok_or(ScaffoldContractError::InvalidId)?;
        world.editor_remove_object(cue)?;
    }
    Ok(())
}

fn challenge_homeostasis(
    kind: ActiveChallengeKind,
    tick: Tick,
) -> Result<HomeostaticSnapshot, ScaffoldContractError> {
    let mut baseline = HomeostaticSnapshot::baseline(tick);
    if kind == ActiveChallengeKind::InjuryFatigueRecovery {
        baseline.drives.fatigue = 0.9;
        baseline.drives.pain = 0.7;
        baseline.drives.brain_atp = 0.25;
        baseline.hormones.sleep_pressure = 0.85;
    }
    HomeostaticSnapshot::new(tick, baseline.drives, baseline.hormones)
}

#[allow(clippy::too_many_arguments)]
fn challenge_tick_succeeded(
    kind: ActiveChallengeKind,
    step: u32,
    tick_budget: u32,
    action_kind: alife_core::ActionKind,
    action_family: alife_core::CandidateActionFamily,
    target_kind: Option<WorldObjectKind>,
    result: &alife_world::HeadlessActionResult,
    sleep_completed: bool,
) -> bool {
    let succeeded = result.execution.succeeded && result.observation.success;
    let safe = result.observation.pain_delta.raw() <= 0.01;
    let active = action_kind != alife_core::ActionKind::Idle;
    match kind {
        ActiveChallengeKind::VisibleRewardNavigation => {
            succeeded && target_kind == Some(WorldObjectKind::Food) && active
        }
        ActiveChallengeKind::BlockedRouteDetour => {
            succeeded && action_kind == alife_core::ActionKind::Move
        }
        ActiveChallengeKind::DangerousShortVsSafeLong => {
            succeeded
                && safe
                && (target_kind == Some(WorldObjectKind::Food)
                    || action_family == alife_core::CandidateActionFamily::Avoid)
        }
        ActiveChallengeKind::RewardHazardReversal => {
            step >= tick_budget / 2 && succeeded && safe && active
        }
        ActiveChallengeKind::DelayedChoice => step > 0 && succeeded && active,
        ActiveChallengeKind::UnfamiliarEdibility => succeeded && safe && active,
        ActiveChallengeKind::PostSleepRetention => sleep_completed && succeeded && active,
        ActiveChallengeKind::LayoutAppearanceGeneralization => succeeded && active,
        ActiveChallengeKind::InjuryFatigueRecovery => {
            succeeded && action_kind == alife_core::ActionKind::Rest
        }
        ActiveChallengeKind::NameAddressedInstruction
        | ActiveChallengeKind::WordObjectGrounding
        | ActiveChallengeKind::ActionWordGrounding => succeeded && active,
        ActiveChallengeKind::WhatWhyNarration => {
            succeeded
                && action_kind == alife_core::ActionKind::Vocalize
                && result.emitted_utterance.is_some()
        }
        ActiveChallengeKind::PeerTaughtAlias | ActiveChallengeKind::SlmDisabledDialectTransfer => {
            succeeded && active
        }
    }
}

const fn minimum_world_object_count(kind: ActiveChallengeKind) -> u16 {
    match kind {
        ActiveChallengeKind::BlockedRouteDetour
        | ActiveChallengeKind::PeerTaughtAlias
        | ActiveChallengeKind::SlmDisabledDialectTransfer => 4,
        _ => 3,
    }
}

fn ratio_q16(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    let scaled = (u64::from(numerator) * u64::from(SCORE_ONE_Q16) + u64::from(denominator / 2))
        / u64::from(denominator);
    u32::try_from(scaled)
        .unwrap_or(SCORE_ONE_Q16)
        .min(SCORE_ONE_Q16)
}

//! Authoritative restored-population reproduction for composite creature genomes.

use std::{collections::BTreeMap, collections::BTreeSet, path::Path};

use alife_core::{
    ActionKind, CandidateActionFamily, CreatureGenome, ExperiencePatch, FoundationWeightAsset,
    GenomeId, OrganismId, PhenotypeCompiler, PhenotypeHash, PhysicalContactKind, PolicyBackend,
    ScaffoldContractError, SensorProfile, Tick, Validate, Vec3f,
};
use alife_world::{
    CreatureLifetimeStateAsset, HabitatActor, HabitatAuthorityError, HabitatBreedingKind,
    HabitatBreedingReceipt, HabitatBreedingRequest, HabitatId, HeadlessWorld, PersistenceError,
    PortableAssetDigest, PortableSaveFile,
};

pub const MINIMUM_POST_RESTORE_TICKS: u32 = 128;

#[derive(Debug, thiserror::Error)]
pub enum CompositePopulationRuntimeError {
    #[error("portable population restore failed: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("habitat authority rejected reproduction: {0}")]
    Habitat(#[from] HabitatAuthorityError),
    #[error("composite population contract failed: {0}")]
    Contract(#[from] ScaffoldContractError),
    #[error("population has no resident {0:?}")]
    MissingResident(OrganismId),
    #[error("organism {0:?} is already present")]
    DuplicateOrganism(OrganismId),
    #[error("restored population must advance 128 ticks before reproduction")]
    InsufficientPostRestoreTicks,
    #[error("GPU reproduction intent is not causally bound to the restored world")]
    InvalidGpuReproductionIntent,
    #[error("GPU reproduction intent was already consumed")]
    ReplayedGpuReproductionIntent,
    #[error("restored population contains no creatures")]
    EmptyPopulation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositePopulationResident {
    pub organism_id: OrganismId,
    pub genome: CreatureGenome,
    pub foundation: FoundationWeightAsset,
    pub phenotype_hash: PhenotypeHash,
    pub lifetime_state: CreatureLifetimeStateAsset,
    pub habitat_id: HabitatId,
    pub generation: u32,
    pub restored_from_save: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifetimeInheritanceEvidence {
    pub memory_records: u32,
    pub lifetime_weights: u32,
    pub state_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositePopulationBirthReceipt {
    pub breeding: HabitatBreedingReceipt,
    pub child_organism_id: OrganismId,
    pub child_genome_id: GenomeId,
    pub child_generation: u32,
    pub parent_genome_ids: [GenomeId; 2],
    pub first_parent_lifetime: LifetimeInheritanceEvidence,
    pub second_parent_lifetime: LifetimeInheritanceEvidence,
    pub child_lifetime: LifetimeInheritanceEvidence,
    pub child_phenotype_hash: PhenotypeHash,
    pub post_restore_ticks: u32,
}

#[derive(Debug, Clone)]
pub struct CompositePopulationRuntime {
    world: HeadlessWorld,
    residents: BTreeMap<u64, CompositePopulationResident>,
    restored_tick: Tick,
    consumed_gpu_intents: BTreeSet<(u64, u64)>,
}

impl CompositePopulationRuntime {
    pub fn restore_from_file(
        save_path: impl AsRef<Path>,
        asset_root: impl AsRef<Path>,
    ) -> Result<Self, CompositePopulationRuntimeError> {
        let save = PortableSaveFile::from_json_file(save_path)?;
        save.validate_with_asset_root(asset_root.as_ref())?;
        let world = save.restore_headless_world()?;
        let world_organisms = world
            .organism_entity_ids()
            .into_iter()
            .map(|(organism_id, _)| organism_id.raw())
            .collect::<BTreeSet<_>>();
        let mut residents = BTreeMap::new();
        for creature in &save.creatures {
            if !world_organisms.contains(&creature.organism_id.raw()) {
                return Err(CompositePopulationRuntimeError::MissingResident(
                    creature.organism_id,
                ));
            }
            let genetic =
                save.load_composite_genetic_birth(creature.organism_id, asset_root.as_ref())?;
            let lifetime =
                save.load_creature_lifetime_state(creature.organism_id, asset_root.as_ref())?;
            let habitat_id = world
                .habitat_authority()
                .membership(creature.organism_id)
                .ok_or(CompositePopulationRuntimeError::MissingResident(
                    creature.organism_id,
                ))?
                .habitat_id;
            let resident = CompositePopulationResident {
                organism_id: creature.organism_id,
                genome: genetic.creature_genome,
                foundation: genetic.foundation,
                phenotype_hash: genetic.phenotype_hash,
                lifetime_state: lifetime,
                habitat_id,
                generation: 0,
                restored_from_save: true,
            };
            if residents
                .insert(creature.organism_id.raw(), resident)
                .is_some()
            {
                return Err(CompositePopulationRuntimeError::DuplicateOrganism(
                    creature.organism_id,
                ));
            }
        }
        if residents.is_empty() {
            return Err(CompositePopulationRuntimeError::EmptyPopulation);
        }
        let restored_tick = world.tick();
        Ok(Self {
            world,
            residents,
            restored_tick,
            consumed_gpu_intents: BTreeSet::new(),
        })
    }

    pub fn resident(&self, organism_id: OrganismId) -> Option<&CompositePopulationResident> {
        self.residents.get(&organism_id.raw())
    }

    pub fn residents(&self) -> impl Iterator<Item = &CompositePopulationResident> {
        self.residents.values()
    }

    pub fn world_snapshot(&self) -> HeadlessWorld {
        self.world.clone()
    }

    pub fn post_restore_ticks(&self) -> u32 {
        self.world
            .tick()
            .raw()
            .saturating_sub(self.restored_tick.raw())
            .min(u64::from(u32::MAX)) as u32
    }

    pub fn advance_ticks(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.world.advance_tick();
        }
    }

    pub fn apply_player_breed_command(
        &mut self,
        habitat_id: HabitatId,
        first_parent: OrganismId,
        second_parent: OrganismId,
        child_organism_id: OrganismId,
        conception_seed: u64,
    ) -> Result<CompositePopulationBirthReceipt, CompositePopulationRuntimeError> {
        self.apply_birth(
            HabitatBreedingRequest {
                habitat_id,
                first_parent,
                second_parent,
                kind: HabitatBreedingKind::Explicit,
                actor: HabitatActor::Player,
                tick: self.world.tick(),
            },
            child_organism_id,
            conception_seed,
            self.world.clone(),
        )
    }

    /// Commits a GPU-selected Contact/Interact that was executed against a
    /// clone returned by `world_snapshot`. The observed world and sealed patch
    /// must still bind to this exact restore generation and mate entity.
    pub fn apply_gpu_reproduction_intent(
        &mut self,
        habitat_id: HabitatId,
        observed_world: HeadlessWorld,
        patch: &ExperiencePatch,
        child_organism_id: OrganismId,
        conception_seed: u64,
    ) -> Result<CompositePopulationBirthReceipt, CompositePopulationRuntimeError> {
        patch.validate_contract()?;
        let initiator = patch.header().organism_id;
        let intent_key = (initiator.raw(), patch.header().sequence_id.0);
        if self.consumed_gpu_intents.contains(&intent_key) {
            return Err(CompositePopulationRuntimeError::ReplayedGpuReproductionIntent);
        }
        let target_entity = patch
            .decision()
            .selected_action
            .target_entity
            .ok_or(CompositePopulationRuntimeError::InvalidGpuReproductionIntent)?;
        let mate = self
            .world
            .entity(target_entity)
            .and_then(|entity| entity.organism_id)
            .ok_or(CompositePopulationRuntimeError::InvalidGpuReproductionIntent)?;
        let observed_target = observed_world
            .entity(target_entity)
            .ok_or(CompositePopulationRuntimeError::InvalidGpuReproductionIntent)?;
        let initiator_resident = self
            .resident(initiator)
            .ok_or(CompositePopulationRuntimeError::MissingResident(initiator))?;
        if observed_world.seed() != self.world.seed()
            || observed_world.tick().raw() < self.world.tick().raw()
            || patch.header().world_tick != observed_world.tick()
            || patch.pre_action().genome_id != initiator_resident.genome.id
            || patch.decision().policy_backend() != PolicyBackend::NeuralClosedLoopGpu
            || patch.decision().selected_action.kind != ActionKind::Interact
            || patch.decision().neural_evidence()?.action_family != CandidateActionFamily::Contact
            || !patch.outcome().success
            || patch.outcome().physical.contact != PhysicalContactKind::Touch
            || patch.outcome().physical.target_entity != Some(target_entity)
            || observed_target.organism_id != Some(mate)
            || observed_target.carried_by != Some(initiator)
        {
            return Err(CompositePopulationRuntimeError::InvalidGpuReproductionIntent);
        }
        let receipt = self.apply_birth(
            HabitatBreedingRequest {
                habitat_id,
                first_parent: initiator,
                second_parent: mate,
                kind: HabitatBreedingKind::CreatureChosen,
                actor: HabitatActor::Organism(initiator),
                tick: self.world.tick(),
            },
            child_organism_id,
            conception_seed,
            observed_world,
        )?;
        self.consumed_gpu_intents.insert(intent_key);
        Ok(receipt)
    }

    fn apply_birth(
        &mut self,
        request: HabitatBreedingRequest,
        child_organism_id: OrganismId,
        conception_seed: u64,
        mut next_world: HeadlessWorld,
    ) -> Result<CompositePopulationBirthReceipt, CompositePopulationRuntimeError> {
        if self.post_restore_ticks() < MINIMUM_POST_RESTORE_TICKS {
            return Err(CompositePopulationRuntimeError::InsufficientPostRestoreTicks);
        }
        child_organism_id.validate()?;
        if self.resident(child_organism_id).is_some() {
            return Err(CompositePopulationRuntimeError::DuplicateOrganism(
                child_organism_id,
            ));
        }
        let first = self.resident(request.first_parent).ok_or(
            CompositePopulationRuntimeError::MissingResident(request.first_parent),
        )?;
        let second = self.resident(request.second_parent).ok_or(
            CompositePopulationRuntimeError::MissingResident(request.second_parent),
        )?;
        if first.foundation.digest() != second.foundation.digest() {
            return Err(ScaffoldContractError::IncompatibleGeneticClass.into());
        }
        let breeding = self
            .world
            .habitat_authority()
            .authorize_breeding(request.clone())?;
        let child_genome =
            CreatureGenome::reproduce(&first.genome, &second.genome, conception_seed)?;
        let expressed = child_genome.express()?;
        let development = expressed.development_state_at(Tick::new(u64::from(
            expressed.development.maturation_duration_ticks,
        )))?;
        let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &expressed.brain_genome,
            &alife_core::BrainCapacityClass::production_for_id(
                child_genome.foundation.brain_class_id,
            )?,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &first.foundation,
        )?;
        let child_lifetime_state = CreatureLifetimeStateAsset {
            schema_version: 1,
            organism_id: child_organism_id,
            memory_records: Vec::new(),
            lifetime_weight_values: Vec::new(),
        };
        let child_generation = first.generation.max(second.generation).saturating_add(1);
        let first_lifetime = lifetime_evidence(&first.lifetime_state)?;
        let second_lifetime = lifetime_evidence(&second.lifetime_state)?;
        let child_lifetime = lifetime_evidence(&child_lifetime_state)?;
        let parent_genome_ids = [first.genome.id, second.genome.id];
        let child_position = self
            .world
            .organism_entity_ids()
            .into_iter()
            .find_map(|(organism, entity)| {
                (organism == request.first_parent)
                    .then(|| self.world.entity(entity).map(|object| object.position))
                    .flatten()
            })
            .unwrap_or(Vec3f::ZERO);
        next_world.spawn_social_agent(
            &format!("birth-{}", child_organism_id.raw()),
            child_organism_id,
            Vec3f::new(child_position.x + 0.25, child_position.y, child_position.z),
            0.5,
        )?;
        let mut authority = next_world.habitat_authority().clone();
        authority.register_creature(child_organism_id, request.habitat_id, request.tick)?;
        next_world.replace_habitat_authority(authority)?;

        let child_phenotype_hash = phenotype.phenotype_hash();
        let resident = CompositePopulationResident {
            organism_id: child_organism_id,
            genome: child_genome.clone(),
            foundation: first.foundation.clone(),
            phenotype_hash: child_phenotype_hash,
            lifetime_state: child_lifetime_state,
            habitat_id: request.habitat_id,
            generation: child_generation,
            restored_from_save: false,
        };
        self.world = next_world;
        self.residents.insert(child_organism_id.raw(), resident);

        Ok(CompositePopulationBirthReceipt {
            breeding,
            child_organism_id,
            child_genome_id: child_genome.id,
            child_generation,
            parent_genome_ids,
            first_parent_lifetime: first_lifetime,
            second_parent_lifetime: second_lifetime,
            child_lifetime,
            child_phenotype_hash,
            post_restore_ticks: self.post_restore_ticks(),
        })
    }
}

fn lifetime_evidence(
    state: &CreatureLifetimeStateAsset,
) -> Result<LifetimeInheritanceEvidence, PersistenceError> {
    let bytes = serde_json::to_vec(state)?;
    Ok(LifetimeInheritanceEvidence {
        memory_records: state.memory_records.len().min(u32::MAX as usize) as u32,
        lifetime_weights: state.lifetime_weight_values.len().min(u32::MAX as usize) as u32,
        state_digest: PortableAssetDigest::for_bytes(&bytes).0,
    })
}

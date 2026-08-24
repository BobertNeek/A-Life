//! Bounded sparse biochemical graph and typed neural coupling contracts.

use serde::{Deserialize, Serialize};

use crate::{
    BodyEventDelta, BodyState, DriveSnapshot, EndocrineProfile, EndocrineSnapshot,
    HomeostaticSnapshot, ScaffoldContractError, Tick, Validate,
};

pub const BIOCHEMICAL_GRAPH_SCHEMA_VERSION: u16 = 3;
pub const MAX_ACTIVE_CHEMICAL_SPECIES: usize = 32;
pub const MAX_ACTIVE_REACTIONS: usize = 128;
pub const MAX_ACTIVE_EMITTERS: usize = 64;
pub const MAX_ACTIVE_RECEPTORS: usize = 64;
pub const MAX_ACTIVE_NEUROEMITTERS: usize = 32;
pub const MAX_NEURAL_RECEPTOR_ACTIVATIONS: usize = 32;
pub const MAX_NEURAL_EMISSIONS: usize = 16;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChemicalSpeciesId(pub u16);

impl ChemicalSpeciesId {
    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.0 == 0 {
            Err(ScaffoldContractError::InvalidId)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChemicalSpeciesKind {
    Material,
    Regulatory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChemicalCompartment {
    Circulation,
    Neural,
    Gut,
    Organ(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChemicalSpecies {
    pub id: ChemicalSpeciesId,
    pub kind: ChemicalSpeciesKind,
    pub compartment: ChemicalCompartment,
    pub baseline: f32,
    pub decay_retention: f32,
    pub minimum: f32,
    pub maximum: f32,
}

impl Validate for ChemicalSpecies {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.id.validate()?;
        validate_finite_values(&[
            self.baseline,
            self.decay_retention,
            self.minimum,
            self.maximum,
        ])?;
        if !(0.0..=1.0).contains(&self.decay_retention)
            || self.minimum < 0.0
            || self.minimum >= self.maximum
            || !(self.minimum..=self.maximum).contains(&self.baseline)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StoichiometricTerm {
    pub species: ChemicalSpeciesId,
    pub amount: f32,
}

impl Validate for StoichiometricTerm {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.species.validate()?;
        validate_finite_values(&[self.amount])?;
        if self.amount <= 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SparseReaction {
    pub reactants: Vec<StoichiometricTerm>,
    pub products: Vec<StoichiometricTerm>,
    pub rate: f32,
    pub rate_control: Option<ChemicalSpeciesId>,
}

impl Validate for SparseReaction {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.reactants.is_empty()
            || self.reactants.len() > 2
            || self.products.is_empty()
            || self.products.len() > 2
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        validate_finite_values(&[self.rate])?;
        if !(0.0..=1.0).contains(&self.rate) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for term in self.reactants.iter().chain(self.products.iter()) {
            term.validate_contract()?;
        }
        if let Some(control) = self.rate_control {
            control.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiochemicalSourceLocus {
    Basal,
    EnergyDeficit,
    Damage,
    TemperatureStress,
    Nutrition,
    SocialContact,
    SleepRecovery,
    MatingOpportunity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmitterResponse {
    Analogue,
    Digital,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BiochemicalEmitter {
    pub source: BiochemicalSourceLocus,
    pub target: ChemicalSpeciesId,
    pub cadence_ticks: u32,
    pub threshold: f32,
    pub gain: f32,
    pub response: EmitterResponse,
    pub inverted: bool,
}

impl Validate for BiochemicalEmitter {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.target.validate()?;
        validate_finite_values(&[self.threshold, self.gain])?;
        if self.cadence_ticks == 0
            || !(0.0..=1.0).contains(&self.threshold)
            || !(-1.0..=1.0).contains(&self.gain)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriveChannel {
    Hunger,
    Fatigue,
    Fear,
    Pain,
    Loneliness,
    Curiosity,
    BrainAtp,
    TemperatureStress,
    Reproductive,
    Extension0,
    Extension1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndocrineChannel {
    Adrenaline,
    Cortisol,
    Dopamine,
    Oxytocin,
    Serotonin,
    Acetylcholine,
    LearningModulator,
    DevelopmentalHormone,
    SleepPressure,
    Extension0,
    Extension1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NeuralReceptorClass {
    InteroceptiveInput,
    RegionalExcitability,
    ProjectionGain,
    LocalThreshold,
    AttentionGate,
    PlasticityAppetitive,
    PlasticityAversive,
    StructuralGrowth,
    Sleep,
    Consolidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiochemicalTargetLocus {
    Drive(DriveChannel),
    Endocrine(EndocrineChannel),
    Neural(NeuralReceptorClass),
    Development(u8),
    Autonomic(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BiochemicalReceptor {
    pub source: ChemicalSpeciesId,
    pub target: BiochemicalTargetLocus,
    pub threshold: f32,
    pub gain: f32,
}

impl Validate for BiochemicalReceptor {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.source.validate()?;
        validate_finite_values(&[self.threshold, self.gain])?;
        if !(0.0..=1.0).contains(&self.threshold) || !(-2.0..=2.0).contains(&self.gain) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeuralEmissionClass {
    RegionalArousal,
    PredictionResidual,
    MotorCommitment,
    UnresolvedGap,
    SocialState,
    ExecutiveSustain,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Neuroemitter {
    pub source: NeuralEmissionClass,
    pub target: ChemicalSpeciesId,
    pub threshold: f32,
    pub gain: f32,
}

impl Validate for Neuroemitter {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.target.validate()?;
        validate_finite_values(&[self.threshold, self.gain])?;
        if !(0.0..=1.0).contains(&self.threshold) || !(-1.0..=1.0).contains(&self.gain) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NeuralEmission {
    pub class: NeuralEmissionClass,
    pub activity: f32,
    pub confidence: f32,
}

impl NeuralEmission {
    pub fn new(
        class: NeuralEmissionClass,
        activity: f32,
        confidence: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            class,
            activity,
            confidence,
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for NeuralEmission {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_unit_values(&[self.activity, self.confidence])
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeuralEmissionFrame {
    pub schema_version: u16,
    pub source_tick: Tick,
    pub graph_epoch: u64,
    pub emissions: Vec<NeuralEmission>,
}

impl NeuralEmissionFrame {
    pub fn new(
        source_tick: Tick,
        graph_epoch: u64,
        emissions: Vec<NeuralEmission>,
    ) -> Result<Self, ScaffoldContractError> {
        let frame = Self {
            schema_version: BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
            source_tick,
            graph_epoch,
            emissions,
        };
        frame.validate_contract()?;
        Ok(frame)
    }
}

impl Validate for NeuralEmissionFrame {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != BIOCHEMICAL_GRAPH_SCHEMA_VERSION
            || self.graph_epoch == 0
            || self.emissions.len() > MAX_NEURAL_EMISSIONS
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for emission in &self.emissions {
            emission.validate_contract()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NeuralReceptorActivation {
    pub class: NeuralReceptorClass,
    pub signal: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeuralReceptorFrame {
    pub schema_version: u16,
    pub source_chemistry_version: u16,
    pub source_tick: Tick,
    pub activations: Vec<NeuralReceptorActivation>,
}

impl NeuralReceptorFrame {
    pub fn activation_for(&self, class: NeuralReceptorClass) -> f32 {
        self.activations
            .iter()
            .find(|activation| activation.class == class)
            .map_or(0.0, |activation| activation.signal)
    }
}

impl Validate for NeuralReceptorFrame {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != BIOCHEMICAL_GRAPH_SCHEMA_VERSION
            || self.source_chemistry_version != BIOCHEMICAL_GRAPH_SCHEMA_VERSION
            || self.activations.len() > MAX_NEURAL_RECEPTOR_ACTIVATIONS
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let mut classes = self
            .activations
            .iter()
            .map(|activation| activation.class)
            .collect::<Vec<_>>();
        classes.sort();
        if classes.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        validate_unit_values(
            &self
                .activations
                .iter()
                .map(|activation| activation.signal)
                .collect::<Vec<_>>(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiochemicalPhenotype {
    schema_version: u16,
    species_budget: usize,
    reaction_budget: usize,
    species: Vec<ChemicalSpecies>,
    reactions: Vec<SparseReaction>,
    emitters: Vec<BiochemicalEmitter>,
    receptors: Vec<BiochemicalReceptor>,
    neuroemitters: Vec<Neuroemitter>,
}

impl BiochemicalPhenotype {
    pub fn species(&self) -> &[ChemicalSpecies] {
        &self.species
    }

    pub fn reactions(&self) -> &[SparseReaction] {
        &self.reactions
    }

    pub fn emitters(&self) -> &[BiochemicalEmitter] {
        &self.emitters
    }

    pub fn receptors(&self) -> &[BiochemicalReceptor] {
        &self.receptors
    }

    pub fn neuroemitters(&self) -> &[Neuroemitter] {
        &self.neuroemitters
    }

    pub const fn species_budget(&self) -> usize {
        self.species_budget
    }

    pub const fn reaction_budget(&self) -> usize {
        self.reaction_budget
    }

    pub(crate) fn with_reaction_rate(
        &self,
        reaction_index: usize,
        rate: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let mut value = self.clone();
        let reaction = value
            .reactions
            .get_mut(reaction_index)
            .ok_or(ScaffoldContractError::InvalidGeneticBounds)?;
        reaction.rate = rate;
        value.validate_contract()?;
        Ok(value)
    }

    pub(crate) fn early_mammal_reference(
        endocrine: EndocrineProfile,
        brain_atp_baseline: f32,
    ) -> Result<Self, ScaffoldContractError> {
        use ids::*;
        let drives = DriveSnapshot {
            brain_atp: brain_atp_baseline,
            ..DriveSnapshot::baseline()
        };
        let hormones = endocrine.baseline;
        let drive_decay = 1.0 - endocrine.parameters.drive_decay_per_update;
        let hormone_decay = 1.0 - endocrine.parameters.hormone_decay_per_update;
        let mut species = vec![
            regulatory(HUNGER, drives.hunger, drive_decay),
            regulatory(FATIGUE, drives.fatigue, drive_decay),
            regulatory(FEAR, drives.fear, drive_decay),
            regulatory(PAIN, drives.pain, drive_decay),
            regulatory(LONELINESS, drives.loneliness, drive_decay),
            regulatory(CURIOSITY, drives.curiosity, drive_decay),
            material(BRAIN_ATP, drives.brain_atp, drive_decay),
            regulatory(TEMPERATURE, drives.temperature_stress, drive_decay),
            regulatory(REPRODUCTIVE, drives.reproductive_drive, drive_decay),
            regulatory(ADRENALINE, hormones.adrenaline, hormone_decay),
            regulatory(CORTISOL, hormones.cortisol, hormone_decay),
            regulatory(DOPAMINE, hormones.dopamine, hormone_decay),
            regulatory(OXYTOCIN, hormones.oxytocin, hormone_decay),
            regulatory(SEROTONIN, hormones.serotonin, hormone_decay),
            regulatory(ACETYLCHOLINE, hormones.acetylcholine, hormone_decay),
            regulatory(LEARNING_SIGNAL, hormones.learning_modulator, hormone_decay),
            regulatory(
                DEVELOPMENT_SIGNAL,
                hormones.developmental_hormone,
                hormone_decay,
            ),
            regulatory(SLEEP_PRESSURE, hormones.sleep_pressure, hormone_decay),
            material(NUTRIENT, 0.0, 0.5),
        ];
        species.sort_by_key(|row| row.id);
        let reactions = vec![SparseReaction {
            reactants: vec![term(NUTRIENT, 1.0)],
            products: vec![term(BRAIN_ATP, 1.0)],
            rate: 0.5,
            rate_control: None,
        }];
        let emitters = vec![
            emitter(BiochemicalSourceLocus::EnergyDeficit, HUNGER, 0.18),
            emitter(BiochemicalSourceLocus::EnergyDeficit, FATIGUE, 0.12),
            emitter(BiochemicalSourceLocus::EnergyDeficit, BRAIN_ATP, -0.12),
            emitter(BiochemicalSourceLocus::Damage, PAIN, 0.80),
            emitter(BiochemicalSourceLocus::Damage, FEAR, 0.35),
            emitter(BiochemicalSourceLocus::Damage, ADRENALINE, 0.55),
            emitter(BiochemicalSourceLocus::Damage, CORTISOL, 0.65),
            emitter(BiochemicalSourceLocus::Nutrition, NUTRIENT, 0.80),
            emitter(BiochemicalSourceLocus::Nutrition, HUNGER, -0.30),
            emitter(BiochemicalSourceLocus::SocialContact, LONELINESS, -0.60),
            emitter(BiochemicalSourceLocus::SocialContact, OXYTOCIN, 0.45),
            emitter(BiochemicalSourceLocus::SleepRecovery, FATIGUE, -0.70),
            emitter(BiochemicalSourceLocus::SleepRecovery, PAIN, -0.45),
            emitter(BiochemicalSourceLocus::SleepRecovery, SLEEP_PRESSURE, -0.75),
            emitter(BiochemicalSourceLocus::SleepRecovery, BRAIN_ATP, 0.30),
            emitter(BiochemicalSourceLocus::TemperatureStress, TEMPERATURE, 0.30),
            emitter(BiochemicalSourceLocus::TemperatureStress, CORTISOL, 0.20),
            emitter(
                BiochemicalSourceLocus::MatingOpportunity,
                REPRODUCTIVE,
                0.85,
            ),
        ];
        let mut receptors = vec![
            drive_receptor(HUNGER, DriveChannel::Hunger),
            drive_receptor(FATIGUE, DriveChannel::Fatigue),
            drive_receptor(FEAR, DriveChannel::Fear),
            drive_receptor(PAIN, DriveChannel::Pain),
            drive_receptor(LONELINESS, DriveChannel::Loneliness),
            drive_receptor(CURIOSITY, DriveChannel::Curiosity),
            drive_receptor(BRAIN_ATP, DriveChannel::BrainAtp),
            drive_receptor(TEMPERATURE, DriveChannel::TemperatureStress),
            drive_receptor(REPRODUCTIVE, DriveChannel::Reproductive),
            endocrine_receptor(ADRENALINE, EndocrineChannel::Adrenaline),
            endocrine_receptor(CORTISOL, EndocrineChannel::Cortisol),
            endocrine_receptor(DOPAMINE, EndocrineChannel::Dopamine),
            endocrine_receptor(OXYTOCIN, EndocrineChannel::Oxytocin),
            endocrine_receptor(SEROTONIN, EndocrineChannel::Serotonin),
            endocrine_receptor(ACETYLCHOLINE, EndocrineChannel::Acetylcholine),
            endocrine_receptor(LEARNING_SIGNAL, EndocrineChannel::LearningModulator),
            endocrine_receptor(DEVELOPMENT_SIGNAL, EndocrineChannel::DevelopmentalHormone),
            endocrine_receptor(SLEEP_PRESSURE, EndocrineChannel::SleepPressure),
            neural_receptor(ADRENALINE, NeuralReceptorClass::RegionalExcitability, 1.0),
            neural_receptor(CORTISOL, NeuralReceptorClass::PlasticityAversive, 1.0),
            neural_receptor(DOPAMINE, NeuralReceptorClass::PlasticityAppetitive, 1.0),
            neural_receptor(ACETYLCHOLINE, NeuralReceptorClass::AttentionGate, 1.0),
            neural_receptor(SLEEP_PRESSURE, NeuralReceptorClass::Sleep, 1.0),
            neural_receptor(SEROTONIN, NeuralReceptorClass::Consolidation, 1.0),
        ];
        receptors.sort_by_key(|row| (target_order(row.target), row.source));
        let neuroemitters = vec![
            Neuroemitter {
                source: NeuralEmissionClass::RegionalArousal,
                target: ADRENALINE,
                threshold: 0.0,
                gain: 0.15,
            },
            Neuroemitter {
                source: NeuralEmissionClass::PredictionResidual,
                target: ADRENALINE,
                threshold: 0.0,
                gain: 0.25,
            },
            Neuroemitter {
                source: NeuralEmissionClass::MotorCommitment,
                target: ACETYLCHOLINE,
                threshold: 0.0,
                gain: 0.10,
            },
            Neuroemitter {
                source: NeuralEmissionClass::SocialState,
                target: OXYTOCIN,
                threshold: 0.0,
                gain: 0.20,
            },
            Neuroemitter {
                source: NeuralEmissionClass::ExecutiveSustain,
                target: ACETYLCHOLINE,
                threshold: 0.0,
                gain: 0.15,
            },
        ];
        let value = Self {
            schema_version: BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
            species_budget: MAX_ACTIVE_CHEMICAL_SPECIES,
            reaction_budget: MAX_ACTIVE_REACTIONS,
            species,
            reactions,
            emitters,
            receptors,
            neuroemitters,
        };
        value.validate_contract()?;
        Ok(value)
    }

    fn species_index(&self, id: ChemicalSpeciesId) -> Option<usize> {
        self.species.binary_search_by_key(&id, |row| row.id).ok()
    }
}

impl Validate for BiochemicalPhenotype {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != BIOCHEMICAL_GRAPH_SCHEMA_VERSION
            || self.species.is_empty()
            || self.species.len() > self.species_budget
            || self.species_budget > MAX_ACTIVE_CHEMICAL_SPECIES
            || self.reactions.len() > self.reaction_budget
            || self.reaction_budget > MAX_ACTIVE_REACTIONS
            || self.emitters.len() > MAX_ACTIVE_EMITTERS
            || self.receptors.len() > MAX_ACTIVE_RECEPTORS
            || self.neuroemitters.len() > MAX_ACTIVE_NEUROEMITTERS
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for row in &self.species {
            row.validate_contract()?;
        }
        if self.species.windows(2).any(|pair| pair[0].id >= pair[1].id) {
            return Err(ScaffoldContractError::InvalidId);
        }
        for reaction in &self.reactions {
            reaction.validate_contract()?;
            validate_reaction_species(self, reaction)?;
        }
        for emitter in &self.emitters {
            emitter.validate_contract()?;
            require_species(self, emitter.target)?;
        }
        for receptor in &self.receptors {
            receptor.validate_contract()?;
            require_species(self, receptor.source)?;
        }
        for neuroemitter in &self.neuroemitters {
            neuroemitter.validate_contract()?;
            require_species(self, neuroemitter.target)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BiochemicalWorkReceipt {
    pub species_updates: u32,
    pub reaction_evaluations: u32,
    pub emitter_evaluations: u32,
    pub receptor_evaluations: u32,
    pub neural_emitter_evaluations: u32,
}

impl BiochemicalWorkReceipt {
    pub const fn total(self) -> u64 {
        self.species_updates as u64
            + self.reaction_evaluations as u64
            + self.emitter_evaluations as u64
            + self.receptor_evaluations as u64
            + self.neural_emitter_evaluations as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BiochemicalGraphState {
    schema_version: u16,
    tick: Tick,
    active_species: u16,
    concentrations: [f32; MAX_ACTIVE_CHEMICAL_SPECIES],
}

impl BiochemicalGraphState {
    pub fn new(
        phenotype: &BiochemicalPhenotype,
        tick: Tick,
        developmental_expression: f32,
    ) -> Result<Self, ScaffoldContractError> {
        phenotype.validate_contract()?;
        validate_developmental_expression(developmental_expression)?;
        let mut concentrations = [0.0; MAX_ACTIVE_CHEMICAL_SPECIES];
        for (index, species) in phenotype.species.iter().enumerate() {
            concentrations[index] =
                species.minimum + (species.baseline - species.minimum) * developmental_expression;
        }
        Ok(Self {
            schema_version: BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
            tick,
            active_species: phenotype.species.len() as u16,
            concentrations,
        })
    }

    pub const fn tick(&self) -> Tick {
        self.tick
    }

    pub fn concentration(
        &self,
        phenotype: &BiochemicalPhenotype,
        species: ChemicalSpeciesId,
    ) -> Result<f32, ScaffoldContractError> {
        let index = phenotype
            .species_index(species)
            .ok_or(ScaffoldContractError::InvalidId)?;
        Ok(self.concentrations[index])
    }

    pub fn advance(
        &self,
        next_tick: Tick,
        body: BodyState,
        event: BodyEventDelta,
        neural: Option<&NeuralEmissionFrame>,
        phenotype: &BiochemicalPhenotype,
        developmental_expression: f32,
    ) -> Result<(Self, BiochemicalWorkReceipt), ScaffoldContractError> {
        self.validate_against(phenotype)?;
        validate_developmental_expression(developmental_expression)?;
        Tick::validate_monotonic(self.tick, next_tick)?;
        if let Some(frame) = neural {
            frame.validate_contract()?;
            if frame.source_tick != self.tick {
                return Err(ScaffoldContractError::NonMonotonicTick);
            }
        }
        let elapsed = next_tick.raw().saturating_sub(self.tick.raw()).min(64) as f32;
        let mut next = *self;
        next.tick = next_tick;
        for (index, species) in phenotype.species.iter().enumerate() {
            let expressed_baseline =
                species.minimum + (species.baseline - species.minimum) * developmental_expression;
            next.concentrations[index] = (expressed_baseline
                + (self.concentrations[index] - expressed_baseline)
                    * species.decay_retention.powf(elapsed))
            .clamp(species.minimum, species.maximum);
        }
        for emitter in &phenotype.emitters {
            if crossed_cadence(self.tick, next_tick, emitter.cadence_ticks) {
                let source = source_value(emitter.source, body, event);
                let source = if emitter.inverted {
                    1.0 - source
                } else {
                    source
                };
                let response = match emitter.response {
                    EmitterResponse::Analogue => (source - emitter.threshold).max(0.0),
                    EmitterResponse::Digital => {
                        if source >= emitter.threshold {
                            1.0
                        } else {
                            0.0
                        }
                    }
                };
                apply_delta(
                    phenotype,
                    &mut next,
                    emitter.target,
                    response * emitter.gain * developmental_expression,
                )?;
            }
        }
        let neural_evaluations = if let Some(frame) = neural {
            for neuroemitter in &phenotype.neuroemitters {
                let activity = frame
                    .emissions
                    .iter()
                    .filter(|emission| emission.class == neuroemitter.source)
                    .map(|emission| emission.activity * emission.confidence)
                    .fold(0.0, f32::max);
                let response = (activity - neuroemitter.threshold).max(0.0);
                apply_delta(
                    phenotype,
                    &mut next,
                    neuroemitter.target,
                    response * neuroemitter.gain * developmental_expression,
                )?;
            }
            frame.emissions.len() as u32
        } else {
            0
        };
        for reaction in &phenotype.reactions {
            apply_reaction(
                phenotype,
                &mut next,
                reaction,
                elapsed.max(1.0) * developmental_expression,
            )?;
        }
        next.validate_against(phenotype)?;
        Ok((
            next,
            BiochemicalWorkReceipt {
                species_updates: phenotype.species.len() as u32,
                reaction_evaluations: phenotype.reactions.len() as u32,
                emitter_evaluations: phenotype.emitters.len() as u32,
                receptor_evaluations: phenotype.receptors.len() as u32,
                neural_emitter_evaluations: neural_evaluations,
            },
        ))
    }

    pub fn derive_homeostasis(
        &self,
        phenotype: &BiochemicalPhenotype,
    ) -> Result<HomeostaticSnapshot, ScaffoldContractError> {
        self.validate_against(phenotype)?;
        let mut drives = DriveSnapshot {
            hunger: 0.0,
            fatigue: 0.0,
            fear: 0.0,
            pain: 0.0,
            loneliness: 0.0,
            curiosity: 0.0,
            brain_atp: 0.0,
            temperature_stress: 0.0,
            reproductive_drive: 0.0,
            extension: [0.0; crate::DRIVE_EXTENSION_SLOTS],
        };
        let mut hormones = EndocrineSnapshot {
            adrenaline: 0.0,
            cortisol: 0.0,
            dopamine: 0.0,
            oxytocin: 0.0,
            serotonin: 0.0,
            acetylcholine: 0.0,
            learning_modulator: 0.0,
            developmental_hormone: 0.0,
            sleep_pressure: 0.0,
            extension: [0.0; crate::ENDOCRINE_EXTENSION_SLOTS],
        };
        for receptor in &phenotype.receptors {
            let signal = receptor_signal(self, phenotype, *receptor)?;
            match receptor.target {
                BiochemicalTargetLocus::Drive(channel) => set_drive(&mut drives, channel, signal),
                BiochemicalTargetLocus::Endocrine(channel) => {
                    set_endocrine(&mut hormones, channel, signal)
                }
                _ => {}
            }
        }
        HomeostaticSnapshot::new(self.tick, drives, hormones)
    }

    pub fn neural_receptor_frame(
        &self,
        phenotype: &BiochemicalPhenotype,
    ) -> Result<NeuralReceptorFrame, ScaffoldContractError> {
        self.validate_against(phenotype)?;
        let mut activations = Vec::<NeuralReceptorActivation>::new();
        for receptor in &phenotype.receptors {
            let BiochemicalTargetLocus::Neural(class) = receptor.target else {
                continue;
            };
            let signal = receptor_signal(self, phenotype, *receptor)?;
            if let Some(existing) = activations.iter_mut().find(|row| row.class == class) {
                existing.signal = (existing.signal + signal).clamp(0.0, 1.0);
            } else {
                activations.push(NeuralReceptorActivation { class, signal });
            }
        }
        activations.sort_by_key(|row| row.class);
        let frame = NeuralReceptorFrame {
            schema_version: BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
            source_chemistry_version: phenotype.schema_version,
            source_tick: self.tick,
            activations,
        };
        frame.validate_contract()?;
        Ok(frame)
    }

    pub fn validate_against(
        &self,
        phenotype: &BiochemicalPhenotype,
    ) -> Result<(), ScaffoldContractError> {
        phenotype.validate_contract()?;
        if self.schema_version != BIOCHEMICAL_GRAPH_SCHEMA_VERSION
            || usize::from(self.active_species) != phenotype.species.len()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for (index, species) in phenotype.species.iter().enumerate() {
            let value = self.concentrations[index];
            if !value.is_finite() || !(species.minimum..=species.maximum).contains(&value) {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        Ok(())
    }
}

fn validate_developmental_expression(value: f32) -> Result<(), ScaffoldContractError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    Ok(())
}

impl Validate for BiochemicalGraphState {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != BIOCHEMICAL_GRAPH_SCHEMA_VERSION
            || self.active_species == 0
            || usize::from(self.active_species) > MAX_ACTIVE_CHEMICAL_SPECIES
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        validate_unit_values(&self.concentrations[..usize::from(self.active_species)])
    }
}

fn validate_reaction_species(
    phenotype: &BiochemicalPhenotype,
    reaction: &SparseReaction,
) -> Result<(), ScaffoldContractError> {
    for term in reaction.reactants.iter().chain(reaction.products.iter()) {
        require_species(phenotype, term.species)?;
    }
    if let Some(control) = reaction.rate_control {
        require_species(phenotype, control)?;
    }
    let material_reactants = reaction.reactants.iter().all(|term| {
        species(phenotype, term.species)
            .is_some_and(|row| row.kind == ChemicalSpeciesKind::Material)
    });
    let material_products = reaction.products.iter().all(|term| {
        species(phenotype, term.species)
            .is_some_and(|row| row.kind == ChemicalSpeciesKind::Material)
    });
    if material_reactants || material_products {
        let input = reaction
            .reactants
            .iter()
            .map(|term| term.amount)
            .sum::<f32>();
        let output = reaction
            .products
            .iter()
            .map(|term| term.amount)
            .sum::<f32>();
        if !material_reactants || !material_products || (input - output).abs() > 1.0e-6 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
    }
    Ok(())
}

fn apply_reaction(
    phenotype: &BiochemicalPhenotype,
    state: &mut BiochemicalGraphState,
    reaction: &SparseReaction,
    elapsed: f32,
) -> Result<(), ScaffoldContractError> {
    let available = reaction
        .reactants
        .iter()
        .map(|term| {
            state
                .concentration(phenotype, term.species)
                .map(|value| value / term.amount)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .fold(f32::INFINITY, f32::min);
    let control = reaction
        .rate_control
        .map(|id| state.concentration(phenotype, id))
        .transpose()?
        .unwrap_or(1.0);
    let amount = (reaction.rate * elapsed * control * available).max(0.0);
    for term in &reaction.reactants {
        apply_delta(phenotype, state, term.species, -amount * term.amount)?;
    }
    for term in &reaction.products {
        apply_delta(phenotype, state, term.species, amount * term.amount)?;
    }
    Ok(())
}

fn apply_delta(
    phenotype: &BiochemicalPhenotype,
    state: &mut BiochemicalGraphState,
    id: ChemicalSpeciesId,
    delta: f32,
) -> Result<(), ScaffoldContractError> {
    let index = phenotype
        .species_index(id)
        .ok_or(ScaffoldContractError::InvalidId)?;
    let row = phenotype.species[index];
    state.concentrations[index] =
        (state.concentrations[index] + delta).clamp(row.minimum, row.maximum);
    Ok(())
}

fn receptor_signal(
    state: &BiochemicalGraphState,
    phenotype: &BiochemicalPhenotype,
    receptor: BiochemicalReceptor,
) -> Result<f32, ScaffoldContractError> {
    Ok(
        ((state.concentration(phenotype, receptor.source)? - receptor.threshold) * receptor.gain)
            .clamp(0.0, 1.0),
    )
}

fn source_value(source: BiochemicalSourceLocus, body: BodyState, event: BodyEventDelta) -> f32 {
    match source {
        BiochemicalSourceLocus::Basal => 1.0,
        BiochemicalSourceLocus::EnergyDeficit => 1.0 - body.energy,
        BiochemicalSourceLocus::Damage => event.damage,
        BiochemicalSourceLocus::TemperatureStress => body.temperature_stress,
        BiochemicalSourceLocus::Nutrition => event.nutrition,
        BiochemicalSourceLocus::SocialContact => event.social_contact,
        BiochemicalSourceLocus::SleepRecovery => event.sleep_recovery,
        BiochemicalSourceLocus::MatingOpportunity => event.mating_opportunity,
    }
    .clamp(0.0, 1.0)
}

fn crossed_cadence(from: Tick, to: Tick, cadence: u32) -> bool {
    let cadence = u64::from(cadence);
    to.raw() / cadence > from.raw() / cadence
}

fn set_drive(drives: &mut DriveSnapshot, channel: DriveChannel, signal: f32) {
    match channel {
        DriveChannel::Hunger => drives.hunger = signal,
        DriveChannel::Fatigue => drives.fatigue = signal,
        DriveChannel::Fear => drives.fear = signal,
        DriveChannel::Pain => drives.pain = signal,
        DriveChannel::Loneliness => drives.loneliness = signal,
        DriveChannel::Curiosity => drives.curiosity = signal,
        DriveChannel::BrainAtp => drives.brain_atp = signal,
        DriveChannel::TemperatureStress => drives.temperature_stress = signal,
        DriveChannel::Reproductive => drives.reproductive_drive = signal,
        DriveChannel::Extension0 => drives.extension[0] = signal,
        DriveChannel::Extension1 => drives.extension[1] = signal,
    }
}

fn set_endocrine(hormones: &mut EndocrineSnapshot, channel: EndocrineChannel, signal: f32) {
    match channel {
        EndocrineChannel::Adrenaline => hormones.adrenaline = signal,
        EndocrineChannel::Cortisol => hormones.cortisol = signal,
        EndocrineChannel::Dopamine => hormones.dopamine = signal,
        EndocrineChannel::Oxytocin => hormones.oxytocin = signal,
        EndocrineChannel::Serotonin => hormones.serotonin = signal,
        EndocrineChannel::Acetylcholine => hormones.acetylcholine = signal,
        EndocrineChannel::LearningModulator => hormones.learning_modulator = signal,
        EndocrineChannel::DevelopmentalHormone => hormones.developmental_hormone = signal,
        EndocrineChannel::SleepPressure => hormones.sleep_pressure = signal,
        EndocrineChannel::Extension0 => hormones.extension[0] = signal,
        EndocrineChannel::Extension1 => hormones.extension[1] = signal,
    }
}

fn species(phenotype: &BiochemicalPhenotype, id: ChemicalSpeciesId) -> Option<&ChemicalSpecies> {
    phenotype
        .species_index(id)
        .map(|index| &phenotype.species[index])
}

fn require_species(
    phenotype: &BiochemicalPhenotype,
    id: ChemicalSpeciesId,
) -> Result<(), ScaffoldContractError> {
    species(phenotype, id)
        .map(|_| ())
        .ok_or(ScaffoldContractError::InvalidId)
}

fn validate_finite_values(values: &[f32]) -> Result<(), ScaffoldContractError> {
    if values.iter().any(|value| !value.is_finite()) {
        Err(ScaffoldContractError::NonFiniteFloat)
    } else {
        Ok(())
    }
}

fn validate_unit_values(values: &[f32]) -> Result<(), ScaffoldContractError> {
    validate_finite_values(values)?;
    if values.iter().any(|value| !(0.0..=1.0).contains(value)) {
        Err(ScaffoldContractError::ScalarOutOfRange)
    } else {
        Ok(())
    }
}

fn regulatory(id: ChemicalSpeciesId, baseline: f32, decay: f32) -> ChemicalSpecies {
    ChemicalSpecies {
        id,
        kind: ChemicalSpeciesKind::Regulatory,
        compartment: ChemicalCompartment::Circulation,
        baseline,
        decay_retention: decay.clamp(0.0, 1.0),
        minimum: 0.0,
        maximum: 1.0,
    }
}

fn material(id: ChemicalSpeciesId, baseline: f32, decay: f32) -> ChemicalSpecies {
    ChemicalSpecies {
        kind: ChemicalSpeciesKind::Material,
        ..regulatory(id, baseline, decay)
    }
}

const fn term(species: ChemicalSpeciesId, amount: f32) -> StoichiometricTerm {
    StoichiometricTerm { species, amount }
}

const fn emitter(
    source: BiochemicalSourceLocus,
    target: ChemicalSpeciesId,
    gain: f32,
) -> BiochemicalEmitter {
    BiochemicalEmitter {
        source,
        target,
        cadence_ticks: 1,
        threshold: 0.0,
        gain,
        response: EmitterResponse::Analogue,
        inverted: false,
    }
}

const fn drive_receptor(source: ChemicalSpeciesId, channel: DriveChannel) -> BiochemicalReceptor {
    BiochemicalReceptor {
        source,
        target: BiochemicalTargetLocus::Drive(channel),
        threshold: 0.0,
        gain: 1.0,
    }
}

const fn endocrine_receptor(
    source: ChemicalSpeciesId,
    channel: EndocrineChannel,
) -> BiochemicalReceptor {
    BiochemicalReceptor {
        source,
        target: BiochemicalTargetLocus::Endocrine(channel),
        threshold: 0.0,
        gain: 1.0,
    }
}

const fn neural_receptor(
    source: ChemicalSpeciesId,
    class: NeuralReceptorClass,
    gain: f32,
) -> BiochemicalReceptor {
    BiochemicalReceptor {
        source,
        target: BiochemicalTargetLocus::Neural(class),
        threshold: 0.0,
        gain,
    }
}

const fn target_order(target: BiochemicalTargetLocus) -> u8 {
    match target {
        BiochemicalTargetLocus::Drive(_) => 0,
        BiochemicalTargetLocus::Endocrine(_) => 1,
        BiochemicalTargetLocus::Neural(_) => 2,
        BiochemicalTargetLocus::Development(_) => 3,
        BiochemicalTargetLocus::Autonomic(_) => 4,
    }
}

mod ids {
    use super::ChemicalSpeciesId;

    pub const HUNGER: ChemicalSpeciesId = ChemicalSpeciesId(1);
    pub const FATIGUE: ChemicalSpeciesId = ChemicalSpeciesId(2);
    pub const FEAR: ChemicalSpeciesId = ChemicalSpeciesId(3);
    pub const PAIN: ChemicalSpeciesId = ChemicalSpeciesId(4);
    pub const LONELINESS: ChemicalSpeciesId = ChemicalSpeciesId(5);
    pub const CURIOSITY: ChemicalSpeciesId = ChemicalSpeciesId(6);
    pub const BRAIN_ATP: ChemicalSpeciesId = ChemicalSpeciesId(7);
    pub const TEMPERATURE: ChemicalSpeciesId = ChemicalSpeciesId(8);
    pub const REPRODUCTIVE: ChemicalSpeciesId = ChemicalSpeciesId(9);
    pub const ADRENALINE: ChemicalSpeciesId = ChemicalSpeciesId(10);
    pub const CORTISOL: ChemicalSpeciesId = ChemicalSpeciesId(11);
    pub const DOPAMINE: ChemicalSpeciesId = ChemicalSpeciesId(12);
    pub const OXYTOCIN: ChemicalSpeciesId = ChemicalSpeciesId(13);
    pub const SEROTONIN: ChemicalSpeciesId = ChemicalSpeciesId(14);
    pub const ACETYLCHOLINE: ChemicalSpeciesId = ChemicalSpeciesId(15);
    pub const LEARNING_SIGNAL: ChemicalSpeciesId = ChemicalSpeciesId(16);
    pub const DEVELOPMENT_SIGNAL: ChemicalSpeciesId = ChemicalSpeciesId(17);
    pub const SLEEP_PRESSURE: ChemicalSpeciesId = ChemicalSpeciesId(18);
    pub const NUTRIENT: ChemicalSpeciesId = ChemicalSpeciesId(19);
}

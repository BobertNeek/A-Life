//! Deterministic grounded N2048 foundation curriculum and promotion gates.

use alife_core::{
    BrainPhenotype, CandidateActionFamily, CandidateFeatureVector, CompiledSynapseKind,
    DecoderHeadKind, LobeKind, N2048FoundationLayoutV1, ScaffoldContractError,
};

use crate::{
    CandidateTrainingTarget, SpeechTrainingTarget, StageTrainableMask, TrainingSequence32,
    TrainingTick, TRAINING_SEQUENCE_TICKS,
};

const CURRICULUM_VERSION: u32 = 1;
const HELD_OUT_EPISODES: u32 = 256;
const ONE_SIDED_85_Z: f64 = 1.036_433_389_493_789_6;
pub const N2048_FOUNDATION_TRAINING_SEED: u64 = 0x4E32_3034_385F_5452;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoundationCurriculumStage {
    GroundedPerception,
    MovementAndEating,
    SurvivalRegulation,
    ContentNeutralMemory,
    IntegratedSurvival,
    SpeechMechanics,
    LiveVocabularyGrounding,
    SelfReporting,
    HeldOutGeneralization,
}

impl FoundationCurriculumStage {
    pub const ALL: [Self; 9] = [
        Self::GroundedPerception,
        Self::MovementAndEating,
        Self::SurvivalRegulation,
        Self::ContentNeutralMemory,
        Self::IntegratedSurvival,
        Self::SpeechMechanics,
        Self::LiveVocabularyGrounding,
        Self::SelfReporting,
        Self::HeldOutGeneralization,
    ];

    pub const fn ordinal(self) -> u16 {
        match self {
            Self::GroundedPerception => 1,
            Self::MovementAndEating => 2,
            Self::SurvivalRegulation => 3,
            Self::ContentNeutralMemory => 4,
            Self::IntegratedSurvival => 5,
            Self::SpeechMechanics => 6,
            Self::LiveVocabularyGrounding => 7,
            Self::SelfReporting => 8,
            Self::HeldOutGeneralization => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurriculumSplit {
    Training,
    HeldOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoundationStageSpec {
    stage: FoundationCurriculumStage,
    trainable_routes: &'static [u16],
    train_action_decoder: bool,
    train_speech_decoder: bool,
    train_memory_decoder: bool,
}

impl FoundationStageSpec {
    pub const fn ordinal(self) -> u16 {
        self.stage.ordinal()
    }

    pub const fn uses_privileged_semantic_labels(self) -> bool {
        false
    }

    pub const fn uses_slm_assistance(self) -> bool {
        false
    }

    pub const fn held_out_episode_count(self) -> u32 {
        HELD_OUT_EPISODES
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct N2048CurriculumV1;

impl N2048CurriculumV1 {
    pub const fn new() -> Self {
        Self
    }

    pub const fn version(self) -> u32 {
        CURRICULUM_VERSION
    }

    pub const fn stage(self, stage: FoundationCurriculumStage) -> FoundationStageSpec {
        let (routes, action, speech, memory): (&'static [u16], bool, bool, bool) = match stage {
            FoundationCurriculumStage::GroundedPerception => (&[0, 1, 2, 6], true, false, false),
            FoundationCurriculumStage::MovementAndEating => (&[6, 7], true, false, false),
            FoundationCurriculumStage::SurvivalRegulation => (&[3, 4, 5, 6], true, false, false),
            FoundationCurriculumStage::ContentNeutralMemory => (&[8, 9, 10, 11], true, false, true),
            FoundationCurriculumStage::IntegratedSurvival => {
                (&[0, 3, 4, 5, 6, 7, 8, 9, 10, 11], true, false, true)
            }
            FoundationCurriculumStage::SpeechMechanics => {
                (&[1, 6, 8, 9, 12, 13, 14, 15], true, true, false)
            }
            FoundationCurriculumStage::LiveVocabularyGrounding => {
                (&[0, 1, 8, 9, 12, 13, 14, 15], true, true, false)
            }
            FoundationCurriculumStage::SelfReporting => {
                (&[3, 4, 6, 7, 8, 9, 12, 13, 14, 15], true, true, false)
            }
            FoundationCurriculumStage::HeldOutGeneralization => (
                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
                true,
                true,
                true,
            ),
        };
        FoundationStageSpec {
            stage,
            trainable_routes: routes,
            train_action_decoder: action,
            train_speech_decoder: speech,
            train_memory_decoder: memory,
        }
    }

    pub fn stage_mask(
        self,
        phenotype: &BrainPhenotype,
        stage: FoundationCurriculumStage,
    ) -> Result<StageTrainableMask, ScaffoldContractError> {
        let spec = self.stage(stage);
        let indices = phenotype
            .synapses()
            .iter()
            .enumerate()
            .filter_map(|(index, synapse)| {
                let selected = match synapse.kind() {
                    CompiledSynapseKind::Recurrent => {
                        spec.trainable_routes.contains(&synapse.route_index())
                    }
                    CompiledSynapseKind::Decoder(coordinate) => match coordinate.head() {
                        DecoderHeadKind::ActionCandidate => spec.train_action_decoder,
                        DecoderHeadKind::SpeechPayload => spec.train_speech_decoder,
                        DecoderHeadKind::MemoryContext => spec.train_memory_decoder,
                    },
                };
                selected.then_some(index as u32)
            })
            .collect::<Vec<_>>();
        StageTrainableMask::from_synapse_indices(phenotype, &indices)
    }

    pub fn sequence(
        self,
        phenotype: &BrainPhenotype,
        stage: FoundationCurriculumStage,
        split: CurriculumSplit,
        seed: u64,
    ) -> Result<TrainingSequence32, ScaffoldContractError> {
        if phenotype.neuron_count() != 2_048 {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let split_seed = match split {
            CurriculumSplit::Training => seed ^ 0x5452_4149_4E00_0001,
            CurriculumSplit::HeldOut => seed ^ 0x4845_4C44_4F55_5401,
        };
        let mut rng = SplitMix64::new(split_seed ^ u64::from(stage.ordinal()));
        let mut ticks = Vec::with_capacity(TRAINING_SEQUENCE_TICKS);
        let mut delayed_sign = 1.0_f32;
        for tick_index in 0..TRAINING_SEQUENCE_TICKS {
            let sign = if rng.next_u64() & 1 == 0 { -1.0 } else { 1.0 };
            if tick_index % 4 == 0 {
                delayed_sign = sign;
            }
            let task_sign = if matches!(stage, FoundationCurriculumStage::SelfReporting) {
                1.0
            } else if matches!(stage, FoundationCurriculumStage::ContentNeutralMemory)
                && tick_index % 4 != 0
            {
                delayed_sign
            } else {
                sign
            };
            ticks.push(build_tick(
                phenotype, stage, tick_index, task_sign, sign, &mut rng,
            )?);
        }
        TrainingSequence32::try_new(ticks)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LanguageStageMetrics {
    pub paired_exposures: u32,
    pub grounding_accuracy: f32,
    pub false_grounding_rate: f32,
    pub literal_narration_agreement: f32,
    pub unseen_surface_transfer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageEvaluation {
    episodes: u32,
    successes: u32,
    mean_loss: f32,
    maximum_locked_stage_regression: f32,
    frozen_weights_bit_identical: bool,
    language: Option<LanguageStageMetrics>,
}

impl StageEvaluation {
    pub fn try_new(
        episodes: u32,
        successes: u32,
        mean_loss: f32,
        maximum_locked_stage_regression: f32,
        frozen_weights_bit_identical: bool,
        language: Option<LanguageStageMetrics>,
    ) -> Result<Self, ScaffoldContractError> {
        if episodes < HELD_OUT_EPISODES
            || successes > episodes
            || !mean_loss.is_finite()
            || mean_loss < 0.0
            || !maximum_locked_stage_regression.is_finite()
            || maximum_locked_stage_regression < 0.0
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(Self {
            episodes,
            successes,
            mean_loss,
            maximum_locked_stage_regression,
            frozen_weights_bit_identical,
            language,
        })
    }

    pub fn lower_confidence_bound(self) -> Result<f32, ScaffoldContractError> {
        wilson_lower_bound_85(self.successes, self.episodes)
    }

    pub const fn episodes(self) -> u32 {
        self.episodes
    }

    pub const fn successes(self) -> u32 {
        self.successes
    }

    pub const fn mean_loss(self) -> f32 {
        self.mean_loss
    }

    pub const fn maximum_locked_stage_regression(self) -> f32 {
        self.maximum_locked_stage_regression
    }

    pub const fn frozen_weights_bit_identical(self) -> bool {
        self.frozen_weights_bit_identical
    }

    pub const fn language(self) -> Option<LanguageStageMetrics> {
        self.language
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageGatePolicy {
    pub minimum_lower_confidence_success: f32,
    pub maximum_locked_stage_regression: f32,
}

impl Default for StageGatePolicy {
    fn default() -> Self {
        Self {
            minimum_lower_confidence_success: 0.90,
            maximum_locked_stage_regression: 0.02,
        }
    }
}

impl StageGatePolicy {
    pub fn validate(self, evaluation: &StageEvaluation) -> Result<(), ScaffoldContractError> {
        if evaluation.lower_confidence_bound()? < self.minimum_lower_confidence_success
            || evaluation.maximum_locked_stage_regression > self.maximum_locked_stage_regression
            || !evaluation.frozen_weights_bit_identical
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        if let Some(language) = evaluation.language {
            if language.paired_exposures < 32
                || language.grounding_accuracy < 0.80
                || language.false_grounding_rate >= 0.05
                || language.literal_narration_agreement < 0.90
                || !language.unseen_surface_transfer
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
        Ok(())
    }
}

pub fn wilson_lower_bound_85(successes: u32, episodes: u32) -> Result<f32, ScaffoldContractError> {
    if episodes == 0 || successes > episodes {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    let n = f64::from(episodes);
    let p = f64::from(successes) / n;
    let z2 = ONE_SIDED_85_Z * ONE_SIDED_85_Z;
    let center = p + z2 / (2.0 * n);
    let spread = ONE_SIDED_85_Z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt();
    Ok(((center - spread) / (1.0 + z2 / n)) as f32)
}

fn build_tick(
    phenotype: &BrainPhenotype,
    stage: FoundationCurriculumStage,
    tick_index: usize,
    action_sign: f32,
    content_sign: f32,
    rng: &mut SplitMix64,
) -> Result<TrainingTick, ScaffoldContractError> {
    let neurons = phenotype.neuron_count() as usize;
    let mut inputs = vec![0.0; neurons];
    let mut targets = vec![0.0; neurons];
    let mut target_weights = vec![0.0; neurons];
    let (input_lobe, target_lobe, family) = task_shape(stage, tick_index);
    stimulate_lobe(phenotype, input_lobe, content_sign, rng, &mut inputs)?;
    if matches!(
        stage,
        FoundationCurriculumStage::MovementAndEating
            | FoundationCurriculumStage::SurvivalRegulation
            | FoundationCurriculumStage::SpeechMechanics
    ) {
        clamp_candidate_motor_sources(phenotype, family, &mut inputs)?;
    }
    if matches!(stage, FoundationCurriculumStage::SpeechMechanics) {
        clamp_speech_source_inputs(phenotype, content_sign, &mut inputs)?;
    }
    supervise_lobe(
        phenotype,
        target_lobe,
        content_sign * 0.75,
        rng,
        &mut targets,
        &mut target_weights,
    )?;
    supervise_candidate_motor_sources(phenotype, family, 0.75, &mut targets, &mut target_weights)?;
    if is_language_stage(stage) {
        supervise_speech_source_neurons(
            phenotype,
            content_sign * 0.75,
            &mut targets,
            &mut target_weights,
        )?;
    }
    let mut features = [0.0; alife_core::CANDIDATE_FEATURE_COUNT];
    for (lane, feature) in features.iter_mut().enumerate() {
        let noise = (rng.next_unit() as f32 - 0.5) * 0.08;
        let lane_scale = 0.72 + (lane % 6) as f32 * 0.04;
        *feature = action_sign * (lane_scale + noise).clamp(0.0, 1.0);
    }
    let candidate = CandidateTrainingTarget::try_new(
        family,
        CandidateFeatureVector(features),
        action_sign * 0.8,
        1.0,
    )?;
    let tick = TrainingTick::try_new(inputs, targets, target_weights, Some(candidate))?;
    if is_language_stage(stage) {
        tick.with_speech_target(SpeechTrainingTarget::try_new(
            (rng.next_u64() % 18) as u16,
            content_sign * 0.8,
            1.0,
        )?)
    } else {
        Ok(tick)
    }
}

fn speech_source_range(
    phenotype: &BrainPhenotype,
) -> Result<std::ops::Range<u32>, ScaffoldContractError> {
    let start = phenotype.candidate_decoder().motor_start()
        + alife_core::SpeechDecoderLayoutV1::MOTOR_SOURCE_OFFSET;
    let end = start + u32::from(alife_core::SpeechDecoderLayoutV1::INPUT_WIDTH);
    if end > phenotype.neuron_count() {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    Ok(start..end)
}

fn clamp_speech_source_inputs(
    phenotype: &BrainPhenotype,
    value: f32,
    inputs: &mut [f32],
) -> Result<(), ScaffoldContractError> {
    for neuron in speech_source_range(phenotype)? {
        inputs[neuron as usize] = value;
    }
    Ok(())
}

fn supervise_speech_source_neurons(
    phenotype: &BrainPhenotype,
    value: f32,
    targets: &mut [f32],
    weights: &mut [f32],
) -> Result<(), ScaffoldContractError> {
    for neuron in speech_source_range(phenotype)? {
        targets[neuron as usize] = value;
        weights[neuron as usize] = 1.0;
    }
    Ok(())
}

fn clamp_candidate_motor_sources(
    phenotype: &BrainPhenotype,
    family: CandidateActionFamily,
    inputs: &mut [f32],
) -> Result<(), ScaffoldContractError> {
    let start = phenotype.candidate_decoder().motor_start()
        + u32::from(family.raw())
            * u32::from(N2048FoundationLayoutV1::CANDIDATE_MOTOR_UNITS_PER_FAMILY);
    let end = start + u32::from(N2048FoundationLayoutV1::CANDIDATE_MOTOR_UNITS_PER_FAMILY);
    if end > phenotype.neuron_count() {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    for neuron in start..end {
        inputs[neuron as usize] = 0.75;
    }
    Ok(())
}

fn supervise_candidate_motor_sources(
    phenotype: &BrainPhenotype,
    family: CandidateActionFamily,
    value: f32,
    targets: &mut [f32],
    weights: &mut [f32],
) -> Result<(), ScaffoldContractError> {
    let start = phenotype.candidate_decoder().motor_start()
        + u32::from(family.raw())
            * u32::from(N2048FoundationLayoutV1::CANDIDATE_MOTOR_UNITS_PER_FAMILY);
    let end = start + u32::from(N2048FoundationLayoutV1::CANDIDATE_MOTOR_UNITS_PER_FAMILY);
    if end > phenotype.neuron_count() {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    for neuron in start..end {
        targets[neuron as usize] = value;
        weights[neuron as usize] = 1.0;
    }
    Ok(())
}

fn task_shape(
    stage: FoundationCurriculumStage,
    tick: usize,
) -> (LobeKind, LobeKind, CandidateActionFamily) {
    match stage {
        FoundationCurriculumStage::GroundedPerception => (
            if tick.is_multiple_of(3) {
                LobeKind::GlyphVision
            } else {
                LobeKind::SensoryGrounding
            },
            LobeKind::CoreAssociation,
            CandidateActionFamily::Inspect,
        ),
        FoundationCurriculumStage::MovementAndEating => (
            LobeKind::CoreAssociation,
            LobeKind::MotorArbitration,
            [
                CandidateActionFamily::Approach,
                CandidateActionFamily::Contact,
                CandidateActionFamily::Ingest,
                CandidateActionFamily::Rest,
            ][tick % 4],
        ),
        FoundationCurriculumStage::SurvivalRegulation => (
            LobeKind::MetabolicDrive,
            LobeKind::HomeostaticRegulation,
            [
                CandidateActionFamily::Avoid,
                CandidateActionFamily::Ingest,
                CandidateActionFamily::Rest,
            ][tick % 3],
        ),
        FoundationCurriculumStage::ContentNeutralMemory => (
            if tick.is_multiple_of(4) {
                LobeKind::CoreAssociation
            } else {
                LobeKind::WorkingMemory
            },
            LobeKind::EpisodicMemory,
            CandidateActionFamily::Inspect,
        ),
        FoundationCurriculumStage::IntegratedSurvival => (
            LobeKind::SensoryGrounding,
            LobeKind::MotorArbitration,
            [
                CandidateActionFamily::Approach,
                CandidateActionFamily::Avoid,
                CandidateActionFamily::Ingest,
                CandidateActionFamily::Rest,
            ][tick % 4],
        ),
        FoundationCurriculumStage::SpeechMechanics => (
            LobeKind::AuditorySpeech,
            LobeKind::LexiconConcept,
            CandidateActionFamily::Other,
        ),
        FoundationCurriculumStage::LiveVocabularyGrounding => (
            if tick.is_multiple_of(2) {
                LobeKind::AuditorySpeech
            } else {
                LobeKind::SensoryGrounding
            },
            LobeKind::LexiconConcept,
            CandidateActionFamily::Inspect,
        ),
        FoundationCurriculumStage::SelfReporting => (
            if tick.is_multiple_of(2) {
                LobeKind::MetabolicDrive
            } else {
                LobeKind::CoreAssociation
            },
            LobeKind::LexiconConcept,
            CandidateActionFamily::Other,
        ),
        FoundationCurriculumStage::HeldOutGeneralization => (
            [
                LobeKind::SensoryGrounding,
                LobeKind::AuditorySpeech,
                LobeKind::GlyphVision,
                LobeKind::MetabolicDrive,
            ][tick % 4],
            [
                LobeKind::CoreAssociation,
                LobeKind::WorkingMemory,
                LobeKind::LexiconConcept,
                LobeKind::MotorArbitration,
            ][tick % 4],
            [
                CandidateActionFamily::Approach,
                CandidateActionFamily::Avoid,
                CandidateActionFamily::Ingest,
                CandidateActionFamily::Other,
            ][tick % 4],
        ),
    }
}

fn stimulate_lobe(
    phenotype: &BrainPhenotype,
    lobe: LobeKind,
    sign: f32,
    rng: &mut SplitMix64,
    values: &mut [f32],
) -> Result<(), ScaffoldContractError> {
    let region = phenotype
        .lobe_layout()
        .region(lobe)
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    for lane in 0..8_u32 {
        let surface_offset = if matches!(lobe, LobeKind::AuditorySpeech | LobeKind::GlyphVision) {
            (rng.next_u64() as u32 % 16) * 8
        } else {
            0
        };
        let ordinal = (surface_offset + lane) % region.len;
        let amplitude_noise = (rng.next_unit() as f32 - 0.5) * 0.04;
        values[(region.start + ordinal) as usize] =
            sign * (0.72 + lane as f32 * 0.02 + amplitude_noise);
    }
    Ok(())
}

fn supervise_lobe(
    phenotype: &BrainPhenotype,
    lobe: LobeKind,
    value: f32,
    rng: &mut SplitMix64,
    targets: &mut [f32],
    weights: &mut [f32],
) -> Result<(), ScaffoldContractError> {
    let region = phenotype
        .lobe_layout()
        .region(lobe)
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    for lane in 0..8_u32 {
        let ordinal = lane % region.len;
        let index = (region.start + ordinal) as usize;
        targets[index] = value + (rng.next_unit() as f32 - 0.5) * 0.01;
        weights[index] = 1.0;
    }
    Ok(())
}

struct SplitMix64(u64);

const fn is_language_stage(stage: FoundationCurriculumStage) -> bool {
    matches!(
        stage,
        FoundationCurriculumStage::SpeechMechanics
            | FoundationCurriculumStage::LiveVocabularyGrounding
            | FoundationCurriculumStage::SelfReporting
            | FoundationCurriculumStage::HeldOutGeneralization
    )
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64))
    }
}

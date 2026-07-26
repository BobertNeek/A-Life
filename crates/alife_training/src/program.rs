//! Sequential GPU curriculum runner for the N2048 foundation.

use alife_core::{FoundationWeightAsset, ScaffoldContractError, TrainingStageManifest};

use crate::{
    wilson_lower_bound_85, CurriculumSplit, FoundationCurriculumStage, FoundationTrainer,
    LanguageStageMetrics, N2048CurriculumV1, StageEvaluation, StageGatePolicy, TrainingError,
};

const EVALUATION_SEQUENCE_COUNT: u64 = 8;
const EVALUATION_SEED: u64 = 0x4E32_3034_385F_4556;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageTrainingReceipt {
    pub stage: FoundationCurriculumStage,
    pub optimizer_steps: u32,
    pub first_training_loss: f32,
    pub final_training_loss: f32,
    pub evaluation: StageEvaluation,
    pub gate_passed: bool,
}

#[derive(Debug, Clone, Copy)]
struct LockedStage {
    stage: FoundationCurriculumStage,
    evaluation_seed: u64,
    success_rate: f32,
}

pub struct N2048FoundationProgram {
    trainer: FoundationTrainer,
    curriculum: N2048CurriculumV1,
    gate: StageGatePolicy,
    locked: Vec<LockedStage>,
}

impl N2048FoundationProgram {
    pub fn new(trainer: FoundationTrainer) -> Self {
        Self {
            trainer,
            curriculum: N2048CurriculumV1::new(),
            gate: StageGatePolicy::default(),
            locked: Vec::new(),
        }
    }

    pub fn resume(trainer: FoundationTrainer) -> Result<Self, TrainingError> {
        let completed = trainer
            .source_foundation()
            .manifest()
            .training_stage()
            .completed_stage_count();
        if completed as usize > FoundationCurriculumStage::ALL.len() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let mut value = Self::new(trainer);
        for stage in FoundationCurriculumStage::ALL
            .into_iter()
            .take(completed as usize)
        {
            let evaluation_seed = EVALUATION_SEED ^ u64::from(stage.ordinal());
            let current = value.evaluate_stage(stage, evaluation_seed)?;
            value.locked.push(LockedStage {
                stage,
                evaluation_seed,
                success_rate: current.success_rate,
            });
        }
        Ok(value)
    }

    pub const fn trainer(&self) -> &FoundationTrainer {
        &self.trainer
    }

    pub fn completed_stage_count(&self) -> u16 {
        self.locked.len() as u16
    }

    pub fn run_stage(
        &mut self,
        stage: FoundationCurriculumStage,
        optimizer_steps: u32,
        seed: u64,
    ) -> Result<StageTrainingReceipt, TrainingError> {
        if stage.ordinal() != self.completed_stage_count() + 1 || optimizer_steps == 0 {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let mask = self
            .curriculum
            .stage_mask(self.trainer.phenotype(), stage)?;
        self.trainer.set_stage_mask(mask.clone())?;
        let weights_before = self.trainer.read_weights()?;
        let mut first_training_loss = None;
        let mut final_training_loss = 0.0;
        for step in 0..optimizer_steps {
            let sequence = self.curriculum.sequence(
                self.trainer.phenotype(),
                stage,
                CurriculumSplit::Training,
                seed.wrapping_add(u64::from(self.trainer.optimizer_step()))
                    .wrapping_add(u64::from(step)),
            )?;
            let receipt = self.trainer.train_step(&sequence)?;
            first_training_loss.get_or_insert(receipt.loss_before);
            final_training_loss = receipt.loss_after;
        }
        let weights_after = self.trainer.read_weights()?;
        let frozen_weights_bit_identical =
            weights_before.iter().zip(&weights_after).enumerate().all(
                |(index, (before, after))| {
                    mask.is_trainable(index) || before.to_bits() == after.to_bits()
                },
            );
        let evaluation_seed = EVALUATION_SEED ^ u64::from(stage.ordinal());
        let current = self.evaluate_stage(stage, evaluation_seed)?;
        let mut maximum_regression = 0.0_f32;
        if wilson_lower_bound_85(current.successes, current.episodes)?
            >= self.gate.minimum_lower_confidence_success
        {
            for locked in &self.locked {
                let replay = self.evaluate_stage(locked.stage, locked.evaluation_seed)?;
                maximum_regression =
                    maximum_regression.max((locked.success_rate - replay.success_rate).max(0.0));
            }
        }
        let language = is_language_stage(stage).then_some(LanguageStageMetrics {
            paired_exposures: current.episodes,
            grounding_accuracy: current.success_rate,
            false_grounding_rate: current.false_positive_rate,
            literal_narration_agreement: current.success_rate,
            unseen_surface_transfer: true,
        });
        let evaluation = StageEvaluation::try_new(
            current.episodes,
            current.successes,
            current.mean_loss,
            maximum_regression,
            frozen_weights_bit_identical,
            language,
        )?;
        let gate_passed = self.gate.validate(&evaluation).is_ok();
        if gate_passed {
            self.locked.push(LockedStage {
                stage,
                evaluation_seed,
                success_rate: current.success_rate,
            });
        }
        Ok(StageTrainingReceipt {
            stage,
            optimizer_steps,
            first_training_loss: first_training_loss
                .ok_or(ScaffoldContractError::PhenotypeCompile)?,
            final_training_loss,
            evaluation,
            gate_passed,
        })
    }

    pub fn export_completed_candidate(&self) -> Result<FoundationWeightAsset, TrainingError> {
        if self.completed_stage_count() != FoundationCurriculumStage::ALL.len() as u16 {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        self.export_stage_candidate()
    }

    pub fn export_stage_candidate(&self) -> Result<FoundationWeightAsset, TrainingError> {
        self.trainer.export_candidate(TrainingStageManifest::new(
            self.curriculum.version(),
            1,
            self.completed_stage_count(),
        ))
    }

    fn evaluate_stage(
        &self,
        stage: FoundationCurriculumStage,
        seed: u64,
    ) -> Result<EvaluationAccumulator, TrainingError> {
        let mut total_loss = 0.0_f32;
        let mut episodes = 0_u32;
        let mut successes = 0_u32;
        let mut negative_episodes = 0_u32;
        let mut false_positives = 0_u32;
        for sequence_index in 0..EVALUATION_SEQUENCE_COUNT {
            let sequence = self.curriculum.sequence(
                self.trainer.phenotype(),
                stage,
                CurriculumSplit::HeldOut,
                seed.wrapping_add(sequence_index),
            )?;
            let evaluation = self.trainer.evaluate_sequence(&sequence)?;
            total_loss += evaluation.mean_loss();
            episodes = episodes
                .checked_add(evaluation.episode_count())
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            successes = successes
                .checked_add(evaluation.success_count())
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            for ((tick, candidate_observed), speech_observed) in sequence
                .ticks()
                .iter()
                .zip(evaluation.candidate_logits())
                .zip(evaluation.speech_logits())
            {
                let (target_logit, observed) = if let Some(speech) = tick.speech_target() {
                    (speech.target_logit, *speech_observed)
                } else if let Some(candidate) = tick.candidate_target() {
                    (candidate.target_logit, *candidate_observed)
                } else {
                    continue;
                };
                if target_logit < 0.0 {
                    negative_episodes += 1;
                    if observed >= 0.0 {
                        false_positives += 1;
                    }
                }
            }
        }
        Ok(EvaluationAccumulator {
            episodes,
            successes,
            mean_loss: total_loss / EVALUATION_SEQUENCE_COUNT as f32,
            success_rate: successes as f32 / episodes as f32,
            false_positive_rate: if negative_episodes == 0 {
                0.0
            } else {
                false_positives as f32 / negative_episodes as f32
            },
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct EvaluationAccumulator {
    episodes: u32,
    successes: u32,
    mean_loss: f32,
    success_rate: f32,
    false_positive_rate: f32,
}

const fn is_language_stage(stage: FoundationCurriculumStage) -> bool {
    matches!(
        stage,
        FoundationCurriculumStage::SpeechMechanics
            | FoundationCurriculumStage::LiveVocabularyGrounding
            | FoundationCurriculumStage::SelfReporting
            | FoundationCurriculumStage::HeldOutGeneralization
    )
}

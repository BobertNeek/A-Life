//! Self-contained experimental training state. This is never an accepted
//! lineage receipt or gameplay save, and its auxiliary head is training-only.
use super::*;
use crate::{FoundationTrainerCheckpoint, FoundationTrainingHandoffV1};
use alife_core::{PhenotypeCompiler, PhenotypeCompilerInputs, TrainingStageManifest};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

pub type ArtifactResult<T> = Result<T, Box<dyn std::error::Error>>;

pub fn identity(value: &impl Serialize) -> Result<[u8; 32], TrainingError> {
    Ok(Sha256::digest(serde_json::to_vec(value).map_err(|_| invalid())?).into())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredictiveRun {
    pub seed: u64,
    pub dataset_sha256: [u8; 32],
    pub epochs: usize,
    pub head_learning_rate: f32,
    pub inputs: PredictiveInputs,
    /// Preserve evaluation membership across disposable continuation datasets.
    pub split_assignments: BTreeMap<String, PredictiveSplit>,
}

impl PredictiveRun {
    pub fn from_dataset(
        seed: u64,
        data: &PredictiveDataset,
        dataset_sha256: [u8; 32],
        epochs: usize,
        head_learning_rate: f32,
        inputs: PredictiveInputs,
    ) -> Result<Self, TrainingError> {
        let mut split_assignments = BTreeMap::new();
        for e in &data.episodes {
            let content = identity(&(&e.replay, &e.observations, &e.actions, &e.motor_values))?;
            let mut keys = vec![
                format!("group:{}", e.split_group),
                format!("content:{}", hex(&content)),
            ];
            if let Some(evidence) = &e.evidence {
                keys.push(format!(
                    "native-source:{}",
                    hex(&evidence.source_patch_sha256)
                ));
            }
            for key in keys {
                if split_assignments
                    .insert(key, e.split)
                    .is_some_and(|s| s != e.split)
                {
                    return Err(invalid());
                }
            }
        }
        Ok(Self {
            seed,
            dataset_sha256,
            epochs,
            head_learning_rate,
            inputs,
            split_assignments,
        })
    }
}
fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn validate_split_history<'a>(
    runs: impl IntoIterator<Item = &'a PredictiveRun>,
) -> Result<(), TrainingError> {
    let mut assignments = BTreeMap::new();
    for run in runs {
        if run.split_assignments.is_empty()
            || run.split_assignments.len() > 16384
            || run
                .split_assignments
                .keys()
                .any(|k| k.is_empty() || k.len() > 320)
        {
            return Err(invalid());
        }
        for (key, &split) in &run.split_assignments {
            if assignments.insert(key, split).is_some_and(|s| s != split) {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentalPredictivePayload {
    pub schema_version: u16,
    pub disposable_only: bool,
    /// Immutable original native source, not replaced by learned weights.
    pub source: FoundationTrainingHandoffV1,
    pub source_sha256: [u8; 32],
    pub actor: FoundationTrainerCheckpoint,
    pub head: PredictiveHead,
    pub runs: Vec<PredictiveRun>,
    pub parent_state_sha256: Option<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentalPredictiveState {
    pub payload: ExperimentalPredictivePayload,
    pub payload_sha256: [u8; 32],
}

impl ExperimentalPredictiveState {
    pub fn capture(
        trainer: &CpuFoundationTrainer,
        head: &PredictiveHead,
        source: FoundationTrainingHandoffV1,
        run: PredictiveRun,
        parent: Option<&Self>,
    ) -> Result<Self, TrainingError> {
        let mut runs = parent.map_or_else(Vec::new, |s| s.payload.runs.clone());
        runs.push(run);
        validate_split_history(&runs)?;
        let payload = ExperimentalPredictivePayload {
            schema_version: 1,
            disposable_only: true,
            source_sha256: identity(&source)?,
            source,
            actor: trainer.checkpoint(),
            head: head.clone(),
            runs,
            parent_state_sha256: parent.map(|s| s.payload_sha256),
        };
        let state = Self {
            payload_sha256: identity(&payload)?,
            payload,
        };
        state.restore()?;
        if let Some(previous) = parent {
            previous.restore()?;
            if previous.payload.source_sha256 != state.payload.source_sha256
                || previous.payload.actor.optimizer_step > state.payload.actor.optimizer_step
            {
                return Err(invalid());
            }
        }
        Ok(state)
    }

    /// Restore exact native weights, moments, integer ages, mask and head.
    pub fn restore(&self) -> Result<(CpuFoundationTrainer, PredictiveHead), TrainingError> {
        let p = &self.payload;
        if p.schema_version != 1
            || !p.disposable_only
            || identity(p)? != self.payload_sha256
            || identity(&p.source)? != p.source_sha256
            || p.runs.is_empty()
            || p.runs.len() > 256
            || p.runs.iter().any(|r| {
                r.epochs == 0
                    || r.epochs > 256
                    || !r.head_learning_rate.is_finite()
                    || r.head_learning_rate <= 0.
            })
        {
            return Err(invalid());
        }
        let source = p.source.clone().admit()?;
        validate_split_history(&p.runs)?;
        let trainer = CpuFoundationTrainer::from_checkpoint(
            source.phenotype,
            source.foundation,
            p.actor.clone(),
        )?;
        let h = &p.head;
        h.validate_shape(trainer.phenotype())?;
        Ok((trainer, h.clone()))
    }

    pub fn validate_next_run(&self, run: &PredictiveRun) -> Result<(), TrainingError> {
        self.restore()?;
        validate_split_history(self.payload.runs.iter().chain(std::iter::once(run)))
    }

    /// Native fresh-cohort fine-tuning route. Rebind an experimental founder to
    /// the actual exported genetic weights while retaining Adam history. The
    /// auxiliary head is deliberately dropped. No accepted source is changed.
    pub fn into_disposable_founder(
        &self,
    ) -> Result<(CpuFoundationTrainer, PhenotypeCompilerInputs), TrainingError> {
        let (mut trainer, _) = self.restore()?;
        let asset = trainer.export_candidate(TrainingStageManifest::new(1, 1, 1))?;
        let inputs = &self.payload.source.compiler_inputs;
        let (phenotype, next_inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
            inputs.genome().clone(),
            inputs.development().clone(),
            asset.clone(),
        )?;
        trainer.rebind_for_next_cohort(phenotype, asset)?;
        Ok((trainer, next_inputs))
    }

    /// Publish a new local file without overwriting any source or previous run.
    /// Complete pending bytes are recoverable after interruption; both names
    /// contain the same checksummed envelope when publication has completed.
    pub fn write_new(&self, path: &Path) -> ArtifactResult<()> {
        self.restore()?;
        if path.exists() {
            return Err("experimental state output already exists".into());
        }
        let name = path
            .file_name()
            .ok_or("state output needs a filename")?
            .to_string_lossy();
        let pending = path.with_file_name(format!(".{name}.pending"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)?;
        file.write_all(&serde_json::to_vec(self)?)?;
        file.sync_all()?;
        fs::hard_link(&pending, path)?;
        fs::remove_file(pending)?;
        Ok(())
    }

    pub fn read(path: &Path) -> ArtifactResult<Self> {
        if fs::metadata(path)?.len() > 128 * 1024 * 1024 {
            return Err("experimental state exceeds 128 MiB".into());
        }
        let state: Self = serde_json::from_slice(&fs::read(path)?)?;
        state.restore()?;
        Ok(state)
    }
}

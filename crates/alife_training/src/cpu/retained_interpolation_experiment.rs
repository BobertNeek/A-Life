//! Disposable, unqualified two-parent interpolation. No ancestry is asserted and
//! no candidate Adam/critic or resumable Current checkpoint is manufactured.
//!
//! The retained projection is extracted from the hash-pinned PC Current's own
//! manifest-bound assets. This does not qualify full native recovery or dispatch.
use super::{CpuHostError, CpuRetainedNeuralInheritance};
use crate::{native_receipt::sha256, CurrentTrainingCheckpointV1};
use alife_core::{
    validate_weight_only_policy_successor, BrainCapacityClass, FoundationWeightAsset,
    N2048LivePolicyTransition, PhenotypeCompilerInputs,
};
use alife_gpu_backend::GpuLiveTopologyCheckpointV1;
use serde::{Deserialize, Serialize};
mod source_projection;
pub use source_projection::*;
mod cpu_archive;
pub use cpu_archive::*;
mod archive_encoding;

type Result<T> = std::result::Result<T, CpuHostError>;

/// Original artifact bytes, including both full optimizer histories and any
/// private life. They remain unchanged, rather than reserialized as new parents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterpolationExperimentInputs {
    #[serde(with = "archive_encoding::blob")]
    pub pc_current: Vec<u8>,
    pub expected_pc_current_sha256: String,
    pub cloud_parent: InterpolationCloudParent,
    pub cloud_fraction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InterpolationCloudParent {
    Current {
        #[serde(with = "archive_encoding::blob")]
        bytes: Vec<u8>,
        expected_sha256: String,
    },
    NativeCpuArchive {
        archive: NativeCpuParentArchive,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterpolationExperimentReceipt {
    pub schema: u16,
    pub kind: String,
    pub qualification: String,
    pub optimizer_semantics: String,
    pub pc_current_sha256: String,
    pub cloud_parent_kind: String,
    /// Current byte SHA256 or the explicitly distinct native CPU file-tree pin.
    pub cloud_parent_sha256: String,
    pub cloud_native_cpu_binding: Option<NativeCpuArchiveBinding>,
    pub pc_projection_sha256: String,
    pub pc_projection_binding: RetainedProjectionBinding,
    pub canonical_coordinates: usize,
    pub acquired_coordinates: usize,
    pub changed_genetic_coordinates: usize,
}

/// An inference-only view, not a FoundationTrainingHandoff or accepted Current.
/// Both parent archives contain their original Adam, but the candidate has none.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedInterpolationExperiment {
    inputs: InterpolationExperimentInputs,
    compiler_inputs: PhenotypeCompilerInputs,
    retained: CpuRetainedNeuralInheritance,
    receipt: InterpolationExperimentReceipt,
}

impl RetainedInterpolationExperiment {
    pub fn inputs(&self) -> &InterpolationExperimentInputs {
        &self.inputs
    }
    pub fn compiler_inputs(&self) -> &PhenotypeCompilerInputs {
        &self.compiler_inputs
    }
    pub fn retained(&self) -> &CpuRetainedNeuralInheritance {
        &self.retained
    }
    pub fn receipt(&self) -> &InterpolationExperimentReceipt {
        &self.receipt
    }
    /// Cold requalification checks all candidate fields against both exact
    /// archived parents. Merely decoding this experiment confers no authority.
    pub fn validate(&self) -> Result<()> {
        let expected = prepare_retained_interpolation_experiment(self.inputs.clone())?;
        if serde_json::to_vec(self)? != serde_json::to_vec(&expected)? {
            return Err(CpuHostError::State(
                "interpolation candidate or receipt changed",
            ));
        }
        Ok(())
    }
}

/// Synthetic-fixture preparation only. The native Current codec checks the
/// archived heads and metadata. The PC projection is derived from its selected
/// private assets; full native recovery, gameplay and skills remain unqualified.
pub fn prepare_retained_interpolation_experiment(
    inputs: InterpolationExperimentInputs,
) -> Result<RetainedInterpolationExperiment> {
    let bad = |message| CpuHostError::State(message);
    if !inputs.cloud_fraction.is_finite() || !(0.0..=1.0).contains(&inputs.cloud_fraction) {
        return Err(bad("interpolation fraction must be finite and in [0,1]"));
    }
    if sha256(&inputs.pc_current) != inputs.expected_pc_current_sha256 {
        return Err(bad("parent artifact differs from its explicit SHA256 pin"));
    }
    let pc = CurrentTrainingCheckpointV1::decode(&inputs.pc_current)
        .map_err(|_| bad("PC Current codec admission failed"))?;
    let (cloud_handoff, cloud_kind, cloud_pin, cloud_native_cpu_binding) =
        match &inputs.cloud_parent {
            InterpolationCloudParent::Current {
                bytes,
                expected_sha256,
            } => {
                if sha256(bytes) != *expected_sha256 {
                    return Err(bad("cloud parent differs from its explicit SHA256 pin"));
                }
                let cloud = CurrentTrainingCheckpointV1::decode(bytes)
                    .map_err(|_| bad("cloud Current codec admission failed"))?;
                (cloud.handoff, "current", expected_sha256.clone(), None)
            }
            InterpolationCloudParent::NativeCpuArchive { archive } => {
                let (handoff, binding) =
                    cpu_archive::admit_native_cpu_archive(archive).map_err(|e| {
                        CpuHostError::Preparation(format!("native CPU parent admission: {e}"))
                    })?;
                (
                    handoff,
                    "native_cpu_archive",
                    binding.tree_sha256.clone(),
                    Some(binding),
                )
            }
        };
    let a = pc.handoff.clone().admit()?;
    let b = cloud_handoff.admit()?;
    validate_weight_only_policy_successor(&a.phenotype, &b.phenotype)?;
    if a.actor.stage_mask != b.actor.stage_mask
        || a.foundation
            .manifest()
            .training_stage()
            .curriculum_version()
            != b.foundation
                .manifest()
                .training_stage()
                .curriculum_version()
        || a.foundation
            .manifest()
            .training_stage()
            .evaluation_version()
            != b.foundation
                .manifest()
                .training_stage()
                .evaluation_version()
    {
        return Err(bad(
            "parent stage masks or curriculum/evaluation contracts differ",
        ));
    }
    let (source, source_binding) =
        source_projection::extract(&pc, &inputs.expected_pc_current_sha256)?;
    let source_map = source.canonical_to_live(&a.phenotype)?;
    let mut weights = a.actor.weights.clone();
    let mut changed = 0;
    for (i, weight) in weights.iter_mut().enumerate() {
        let other = b.actor.weights[i];
        if !a.actor.stage_mask.is_trainable(i) {
            if weight.to_bits() != other.to_bits() {
                return Err(bad("parents disagree on a frozen coordinate"));
            }
        } else if weight.to_bits() != other.to_bits() {
            let before = weight.to_bits();
            *weight = match inputs.cloud_fraction {
                0.0 => *weight,
                1.0 => other,
                x => ((1.0 - x) * f64::from(*weight) + x * f64::from(other)) as f32,
            };
            changed += usize::from(before != weight.to_bits());
        }
    }
    let asset = FoundationWeightAsset::from_trained_weights(
        &a.phenotype,
        weights,
        a.foundation.manifest().training_stage(),
    )?;
    // This is an experimental overlay sequence, not an offline optimizer step
    // or a claim that either parent descended from the other.
    let transition = N2048LivePolicyTransition::prepare(
        &pc.handoff.compiler_inputs,
        pc.handoff.policy_version,
        asset,
    )?;
    let next = transition.next_phenotype();
    let topology = GpuLiveTopologyCheckpointV1::recompile_for_phenotype(
        next,
        source.topology.v11_checkpoint.clone(),
    )?;
    // Only canonical genetic weights and their derived identities may change.
    // Added edges never become actor/Adam coordinates; offsets cannot shift.
    let mut allowed = source.topology.clone();
    allowed.phenotype_hash = next.phenotype_hash();
    for (canonical, &live) in source_map.iter().enumerate() {
        allowed.genetic_weight_bits[live] = next.synapses()[canonical].genetic_weight().to_bits();
    }
    allowed.canonical_digest = allowed.recompute_canonical_digest()?;
    if serde_json::to_vec(&allowed)? != serde_json::to_vec(&topology)? {
        return Err(bad(
            "weight-only transition changed live coordinates or acquired state",
        ));
    }
    let mut retained = source.clone();
    retained.topology = topology;
    for bank in [&mut retained.lifetime, &mut retained.fast] {
        bank.phenotype_hash = next.phenotype_hash();
        bank.canonical_digest = bank.recompute_canonical_digest()?;
    }
    if retained.canonical_to_live(next)? != source_map {
        return Err(bad(
            "canonical-to-live permutation changed during interpolation",
        ));
    }
    next.validate_against(&BrainCapacityClass::n2048())?;
    let receipt = InterpolationExperimentReceipt {
        schema: 3,
        kind: "inference-only-two-parent-interpolation-pc-retained-overlay".into(),
        qualification: "synthetic-fixture-qualified-source-projection-binding; full-native-recovery/runtime/skills-unqualified; no-common-ancestry-claim".into(),
        optimizer_semantics: "unresolved; no-candidate-actor-or-critic; both-parent-Adam-archives-unchanged; not-resumable".into(),
        pc_current_sha256: sha256(&inputs.pc_current),
        cloud_parent_kind:cloud_kind.into(),
        cloud_parent_sha256:cloud_pin,
        cloud_native_cpu_binding,
        pc_projection_sha256: sha256(&serde_json::to_vec(&source)?),
        pc_projection_binding: source_binding,
        canonical_coordinates: source_map.len(),
        acquired_coordinates: retained.topology.total_synapse_count as usize - source_map.len(),
        changed_genetic_coordinates: changed,
    };
    Ok(RetainedInterpolationExperiment {
        inputs,
        compiler_inputs: transition.next_inputs().clone(),
        retained,
        receipt,
    })
}

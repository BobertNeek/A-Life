//! Explicit optimizer-transfer fork operations. No exact-continuation or
//! accepted-source receipt is manufactured; the experiment retains both parents.
use super::*;
use crate::{CurrentTrainingCheckpointV1, PpoValueHeadCheckpoint};
use alife_core::*;
use alife_gpu_backend::GpuLiveTopologyCheckpointV1;
use alife_world::persistence::*;
use retained_interpolation_experiment::RetainedInterpolationExperiment;

/// Initial native heads and a separate retained-neural newborn projection.
/// The donor's moments describe the donor trajectory, not the interpolated genes.
pub struct PcDonorRetainedFork {
    pub trainer: CpuFoundationTrainer,
    pub value: CpuValueHead,
    pub retained: CpuRetainedNeuralInheritance,
    pub donor_actor: FoundationTrainerCheckpoint,
    pub donor_value: PpoValueHeadCheckpoint,
}

pub fn initialize_pc_donor_retained_fork(
    experiment: &RetainedInterpolationExperiment,
) -> CpuContinuationResult<PcDonorRetainedFork> {
    experiment.validate()?;
    let pc = CurrentTrainingCheckpointV1::decode(&experiment.inputs().pc_current)?;
    let donor = pc.handoff.admit()?;
    let inputs = experiment.compiler_inputs();
    let p = PhenotypeCompiler::compile_validated(inputs, &BrainCapacityClass::n2048())?;
    validate_weight_only_policy_successor(&donor.phenotype, &p)?;
    let asset = inputs.n2048_candidate_asset().ok_or_else(invalid)?.clone();
    let donor_actor = donor.actor;
    let donor_value = donor.value;
    let mut fork_actor = donor_actor.clone();
    fork_actor.phenotype_hash = p.phenotype_hash();
    fork_actor.source_foundation_digest = asset.digest();
    fork_actor.weights = asset.weights().to_vec();
    let trainer = CpuFoundationTrainer::from_checkpoint(p.clone(), asset, fork_actor)?;
    let value = CpuValueHead::from_checkpoint(donor_value.clone())?;
    let retained = experiment.retained().with_reset_somatic_banks(&p)?;
    Ok(PcDonorRetainedFork {
        trainer,
        value,
        retained,
        donor_actor,
        donor_value,
    })
}

impl CpuRetainedNeuralInheritance {
    /// A reversible newborn recipe: both physical banks start at positive zero.
    /// Selectors, generations, predictor and acquired topology remain exact.
    pub fn with_reset_somatic_banks(&self, p: &BrainPhenotype) -> Result<Self, CpuHostError> {
        self.canonical_to_live(p)?;
        let mut next = self.clone();
        for bank in [&mut next.lifetime, &mut next.fast] {
            bank.bank_0_bits.fill(0);
            bank.bank_1_bits.fill(0);
            bank.canonical_digest = bank.recompute_canonical_digest()?;
        }
        next.canonical_to_live(p)?;
        Ok(next)
    }

    /// Keep the acquired layout, every somatic bit and all metadata; change only
    /// canonical genetic values and the identities derived from those values.
    pub fn rebind_weight_only(
        &self,
        old: &BrainPhenotype,
        next: &BrainPhenotype,
    ) -> Result<Self, CpuHostError> {
        validate_weight_only_policy_successor(old, next)?;
        let map = self.canonical_to_live(old)?;
        let mut result = self.clone();
        let rebuilt = GpuLiveTopologyCheckpointV1::recompile_for_phenotype(
            next,
            self.topology.v11_checkpoint.clone(),
        )?;
        let mut allowed = self.topology.clone();
        allowed.phenotype_hash = next.phenotype_hash();
        for (canonical, &live) in map.iter().enumerate() {
            allowed.genetic_weight_bits[live] =
                next.synapses()[canonical].genetic_weight().to_bits();
        }
        allowed.canonical_digest = allowed.recompute_canonical_digest()?;
        if serde_json::to_vec(&rebuilt)? != serde_json::to_vec(&allowed)? {
            return Err(CpuHostError::State(
                "fork rebind changed acquired slots or non-genetic state",
            ));
        }
        result.topology = rebuilt;
        for bank in [&mut result.lifetime, &mut result.fast] {
            bank.phenotype_hash = next.phenotype_hash();
            bank.canonical_digest = bank.recompute_canonical_digest()?;
        }
        if result.canonical_to_live(next)? != map {
            return Err(CpuHostError::State(
                "fork rebind renumbered actor/Adam coordinates",
            ));
        }
        Ok(result)
    }
}

impl CpuCanonicalHost {
    /// New-cohort inheritance from actual current CPU state. CPU stores one
    /// active canonical fast/lifetime vector, so both exported physical banks
    /// receive that vector. This is not an exact GPU ping-pong-bank continuation.
    /// The complete collector retains its original two-bank inheritance evidence
    /// and all personal memory/activation/eligibility separately and unchanged.
    pub fn export_retained_neural_for_newborn(
        &self,
        founder: &CpuFoundationTrainer,
        organism: OrganismId,
    ) -> Result<CpuRetainedNeuralInheritance, CpuHostError> {
        let checkpoint = self.checkpoint(founder)?;
        let resident = checkpoint
            .residents
            .iter()
            .find(|r| r.neural.organism_id == organism)
            .ok_or(CpuHostError::State("retained fork export organism missing"))?;
        let p = founder.phenotype();
        let topology = GpuLiveTopologyCheckpointV1::recompile_for_phenotype(
            p,
            resident.neural_topology.clone(),
        )?;
        let bank = |layer_raw| -> Result<PortableDualWeightBankV1, CpuHostError> {
            let mut b = PortableDualWeightBankV1 {
                schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
                layer_raw,
                phenotype_hash: p.phenotype_hash(),
                synapse_count: topology.total_synapse_count,
                active_generation: resident.neural.weight_generation,
                active_bank: 0,
                bank_0_bits: vec![0; topology.total_synapse_count as usize],
                bank_1_bits: vec![0; topology.total_synapse_count as usize],
                canonical_digest: [0; 4],
            };
            b.canonical_digest = b.recompute_canonical_digest()?;
            Ok(b)
        };
        let mut result = CpuRetainedNeuralInheritance {
            lifetime: bank(GPU_BRAIN_WEIGHT_LAYER_LIFETIME)?,
            fast: bank(GPU_BRAIN_WEIGHT_LAYER_FAST)?,
            topology,
            predictor: resident.predictor.clone(),
        };
        let map = result.canonical_to_live(p)?;
        for (values, b) in [
            (&resident.neural.lifetime, &mut result.lifetime),
            (&resident.neural.fast, &mut result.fast),
        ] {
            for (&value, &live) in values.iter().zip(&map) {
                b.bank_0_bits[live] = value.to_bits();
                b.bank_1_bits[live] = value.to_bits();
            }
            b.canonical_digest = b.recompute_canonical_digest()?;
        }
        result.canonical_to_live(p)?;
        Ok(result)
    }
}

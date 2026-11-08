//! Disposable admission/continuation tests, no world or training lives.
#[cfg(test)]
#[allow(dead_code)]
#[path = "support/motor_balance_assay.rs"]
mod assay;
#[cfg(test)]
use alife_core::*;
#[cfg(test)]
use alife_gpu_backend::{GpuClassBucketPlan, GpuPhenotypeUpload, GpuV11CausalState};
#[cfg(test)]
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask};
#[cfg(test)]
fn compiled(bounded: bool) -> (BrainPhenotype, FoundationWeightAsset) {
    let g = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    let p = if bounded {
        PhenotypeCompiler::compile_n2048_bounded_founder(
            g.clone(),
            d.clone(),
            SensorProfile::GroundedObjectSlotsV1,
        )
    } else {
        PhenotypeCompiler::compile_n2048_experimental_graph(
            g.clone(),
            d.clone(),
            SensorProfile::GroundedObjectSlotsV1,
        )
    }
    .unwrap()
    .0;
    let a = FoundationWeightAsset::from_phenotype_for_genetic_birth(&p).unwrap();
    let p = if bounded {
        PhenotypeCompiler::compile_n2048_bounded_founder_candidate(g, d, a.clone())
    } else {
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(g, d, a.clone())
    }
    .unwrap()
    .0;
    (p, a)
}
fn main() {
    println!("Run focused tests; this binary does not launch a world.");
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fresh_branches_and_direct_gpu_upload_are_identity_bound() {
        let (old, _) = compiled(false);
        let (new, _) = compiled(true);
        let a = GpuV11CausalState::for_phenotype(&old).unwrap();
        let b = GpuV11CausalState::for_phenotype(&new).unwrap();
        assert_eq!(a.dendritic_branches().branches().len(), 64);
        for (x, y) in a
            .dendritic_branches()
            .branches()
            .iter()
            .zip(b.dendritic_branches().branches())
        {
            assert_eq!(x.target, y.target);
            assert_eq!(x.inputs, y.inputs);
            assert_eq!((x.output_gain, x.threshold), (6., -1.));
            assert_eq!((y.output_gain, y.threshold), (0.25, 0.1));
        }
        let old_upload = GpuPhenotypeUpload::try_from(&old).unwrap();
        let new_upload = GpuPhenotypeUpload::try_from(&new).unwrap();
        old_upload.validate_against(&old).unwrap();
        new_upload.validate_against(&new).unwrap();
        assert!(old_upload.validate_against(&new).is_err());
        assert!(new_upload.validate_against(&old).is_err());
        // Planning is CPU-side validation, not a GPU dispatch or parity claim.
        GpuClassBucketPlan::for_phenotype(&old).unwrap();
        GpuClassBucketPlan::for_phenotype(&new).unwrap();
    }
    #[test]
    fn acquired_branch_checkpoint_survives_and_continues_without_fresh_reset() {
        let (p, asset) = compiled(true);
        let (old, _) = compiled(false);
        let mut causal = GpuV11CausalState::for_phenotype(&p).unwrap();
        let mut branches = causal.dendritic_branches().branches().to_vec();
        branches[0] = DendriticBranch::new(
            branches[0].target,
            0.31,
            0.73,
            vec![DendriticInputRef::new(256, 0.42).unwrap()],
        )
        .unwrap();
        causal
            .set_dendritic_branches(DendriticBranchSet::new(branches).unwrap())
            .unwrap();
        let cp = causal.checkpoint();
        let restored = GpuV11CausalState::restore(
            serde_json::from_slice(&serde_json::to_vec(&cp).unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(cp, restored.checkpoint());
        let base = [0.1, 0.1, 0., 0., 0.1, 0.45, 0.75, 0., 0.1, 0.1, 0.];
        let mut q = assay::sequence(
            &p,
            base,
            base,
            "nutritious_contact",
            restored.dendritic_branches(),
            false,
        );
        q.ticks.truncate(8);
        q.burn_in_ticks = 0;
        let mask = StageTrainableMask::from_synapse_indices(
            &p,
            &(0..p.synapses().len() as u32).collect::<Vec<_>>(),
        )
        .unwrap();
        let trainer =
            CpuFoundationTrainer::new(p.clone(), asset, mask, AdamWConfig::default()).unwrap();
        let full = trainer.evaluate_replay(&q).unwrap();
        let mut prefix = q.clone();
        prefix.ticks.truncate(4);
        let first = trainer.evaluate_replay(&prefix).unwrap();
        let mut initial = q.initial.clone();
        initial.activations = first.final_activations[3].clone();
        initial.activity_ema = first.final_activity_ema[3].clone();
        initial.metabolic_load = first.final_metabolic_load[3].clone();
        let state = CpuOrganismState::new(&p, OrganismId::new(1).unwrap(), initial).unwrap();
        let checkpoint = state.checkpoint().unwrap();
        let restored = CpuOrganismState::from_checkpoint(
            &p,
            serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(checkpoint, restored.checkpoint().unwrap());
        assert!(CpuOrganismState::from_checkpoint(&old, checkpoint).is_err());
        let mut suffix = q.clone();
        suffix.initial = restored.neural_state().clone();
        suffix.ticks = q.ticks[4..].to_vec();
        let last = trainer.evaluate_replay(&suffix).unwrap();
        assert_eq!(full.final_activations[4..], last.final_activations);
        assert_eq!(full.final_activity_ema[4..], last.final_activity_ema);
        assert_eq!(full.final_metabolic_load[4..], last.final_metabolic_load);
        assert_eq!(full.candidate_logits[4..], last.candidate_logits);
        assert_ne!(
            restored.neural_state().dendrites,
            GpuV11CausalState::for_phenotype(&p)
                .unwrap()
                .dendritic_branches()
                .clone()
        );
    }
}

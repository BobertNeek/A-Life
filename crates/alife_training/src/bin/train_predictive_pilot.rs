//! Disposable offline trial. Source handoff/dataset are read-only, and neither
//! accepted lineage nor gameplay saves nor trained checkpoints are written.
use alife_core::CompiledSynapseKind;
use alife_training::{
    cpu::{predictive::*, CpuFoundationTrainer},
    FoundationTrainingHandoffV1, StageTrainableMask,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs,
    time::Instant,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn read(path: &str) -> Result<Vec<u8>> {
    if fs::metadata(path)?.len() > 64 * 1024 * 1024 {
        return Err("pilot input exceeds 64 MiB".into());
    }
    Ok(fs::read(path)?)
}

fn main() -> Result<()> {
    let mut args = BTreeMap::new();
    let mut iter = std::env::args().skip(1);
    while let Some(key) = iter.next() {
        if key == "--help" || key == "-h" {
            println!("train_predictive_pilot --handoff NATIVE_HANDOFF.json --dataset DATASET.json --seed SEED [--epochs 4] [--head-lr 0.03]\n\nOpt-in native CPU recurrent BPTT/Adam plus a disposable nonlinear action-conditioned sensor/body head (SGD). Requires explicit train/validation groups and complete categorical motor bundles. Reports MSE/timing as JSON; writes no files, checkpoints or accepted lineage. Dataset provenance is caller-owned. This is not evidence of gameplay gains or a throughput improvement.");
            return Ok(());
        }
        if !matches!(
            key.as_str(),
            "--handoff" | "--dataset" | "--seed" | "--epochs" | "--head-lr"
        ) {
            return Err(format!("unknown argument {key}").into());
        }
        let value = iter.next().ok_or("missing argument value")?;
        if args.insert(key, value).is_some() {
            return Err("duplicate argument".into());
        }
    }
    let source = read(&args.remove("--handoff").ok_or("--handoff is required")?)?;
    let dataset_bytes = read(&args.remove("--dataset").ok_or("--dataset is required")?)?;
    let seed: u64 = args.remove("--seed").ok_or("--seed is required")?.parse()?;
    let epochs: usize = args
        .remove("--epochs")
        .unwrap_or_else(|| "4".into())
        .parse()?;
    let head_lr: f32 = args
        .remove("--head-lr")
        .unwrap_or_else(|| "0.03".into())
        .parse()?;
    if !(1..=256).contains(&epochs) || !head_lr.is_finite() || head_lr <= 0. {
        return Err("require 1..256 epochs and finite positive head learning rate".into());
    }
    let admitted = serde_json::from_slice::<FoundationTrainingHandoffV1>(&source)?.admit()?;
    let mut trainer = CpuFoundationTrainer::from_checkpoint(
        admitted.phenotype,
        admitted.foundation,
        admitted.actor,
    )?;
    let data: PredictiveDataset = serde_json::from_slice(&dataset_bytes)?;
    data.validate_for(trainer.phenotype())?;
    // Respect every existing frozen coordinate and both native Adam moments.
    // Freeze decoder coordinates explicitly: zero gradients alone would still
    // permit weight decay or an old Adam momentum to move them.
    let checkpoint = trainer.checkpoint();
    let selected = trainer
        .phenotype()
        .synapses()
        .iter()
        .enumerate()
        .filter(|(i, s)| {
            checkpoint.stage_mask.is_trainable(*i) && s.kind() == CompiledSynapseKind::Recurrent
        })
        .map(|(i, _)| i as u32)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err("source has no trainable recurrent coordinates".into());
    }
    trainer.set_stage_mask(StageTrainableMask::from_synapse_indices(
        trainer.phenotype(),
        &selected,
    )?)?;
    let neurons = trainer
        .phenotype()
        .synapses()
        .iter()
        .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
        .map(|s| s.target() as usize)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(32)
        .collect();
    let mut head =
        PredictiveHead::new(trainer.phenotype(), data.schema.clone(), neurons, 16, seed)?;
    let before_train = head.evaluate(&trainer, &data, PredictiveSplit::Train)?;
    let before_validation = head.evaluate(&trainer, &data, PredictiveSplit::Validation)?;
    let start_step = trainer.optimizer_step();
    let start = Instant::now();
    for _ in 0..epochs {
        for (index, e) in data.episodes.iter().enumerate() {
            if e.split == PredictiveSplit::Train {
                head.train_step(&mut trainer, &data, index, head_lr)?;
            }
        }
    }
    let training_seconds = start.elapsed().as_secs_f64();
    let test = if data
        .episodes
        .iter()
        .any(|e| e.split == PredictiveSplit::Test)
    {
        Some(head.evaluate(&trainer, &data, PredictiveSplit::Test)?)
    } else {
        None
    };
    println!(
        "{}",
        serde_json::json!({
            "experiment": "action-conditioned-sensor-body-pilot-v1", "backend": trainer.hardware_receipt(),
            "source_description": data.source_description,
            "dataset_sha256": format!("{:x}", Sha256::digest(&dataset_bytes)),
            "handoff_sha256": format!("{:x}", Sha256::digest(&source)),
            "neurons": trainer.phenotype().neuron_count(), "synapses": trainer.weights().len(),
            "trainable_recurrent_coordinates": selected.len(), "epochs": epochs, "seed": seed,
            "head_learning_rate": head_lr, "training_seconds": training_seconds,
            "optimizer_step_before": start_step, "optimizer_step_after": trainer.optimizer_step(),
            "before_train": before_train, "after_train": head.evaluate(&trainer, &data, PredictiveSplit::Train)?,
            "before_validation": before_validation, "after_validation": head.evaluate(&trainer, &data, PredictiveSplit::Validation)?,
            "after_test": test, "head_exported": false, "checkpoint_written": false,
            "throughput_improvement_measured": false, "gameplay_gain_measured": false
        })
    );
    Ok(())
}

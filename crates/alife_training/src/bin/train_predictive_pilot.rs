//! Disposable offline trial. Source handoff/dataset are read-only, and neither
//! accepted lineage nor gameplay saves are written. Experimental state export
//! is explicit, new-file-only and never an accepted native continuation receipt.
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
    path::Path,
    time::Instant,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn read(path: &str) -> Result<Vec<u8>> {
    if fs::metadata(path)?.len() > 64 * 1024 * 1024 {
        return Err("pilot input exceeds 64 MiB".into());
    }
    if path.ends_with(".zst") {
        use std::io::Read;
        let decoder = zstd::stream::read::Decoder::new(fs::File::open(path)?)?;
        let mut bytes = Vec::new();
        decoder
            .take(512 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 512 * 1024 * 1024 {
            return Err("expanded pilot input exceeds 512 MiB".into());
        }
        Ok(bytes)
    } else {
        Ok(fs::read(path)?)
    }
}

fn main() -> Result<()> {
    let mut args = BTreeMap::new();
    let mut iter = std::env::args().skip(1);
    while let Some(key) = iter.next() {
        if key == "--help" || key == "-h" {
            println!("train_predictive_pilot (--handoff NATIVE_HANDOFF.json | --resume EXPERIMENTAL_STATE.json) --dataset DATASET.json --seed SEED [--epochs 4] [--head-lr 0.03] [--inputs full|no_action|no_recurrent] [--output NEW_EXPERIMENTAL_STATE.json]\n\nOpt-in native CPU recurrent BPTT/Adam plus a disposable nonlinear sensor/body prediction head (SGD). Reports heldout MSE and persistence comparison. Optional checksummed experimental state retains original source, topology, config, weights, moments, integer ages and training-only head, and can be resumed with a compatible dataset. Source files and existing output files are never overwritten. No accepted lineage or gameplay save is written. Loss changes do not establish behavioral or throughput gains.");
            return Ok(());
        }
        if !matches!(
            key.as_str(),
            "--handoff"
                | "--resume"
                | "--dataset"
                | "--seed"
                | "--epochs"
                | "--head-lr"
                | "--output"
                | "--inputs"
        ) {
            return Err(format!("unknown argument {key}").into());
        }
        let value = iter.next().ok_or("missing argument value")?;
        if args.insert(key, value).is_some() {
            return Err("duplicate argument".into());
        }
    }
    let handoff = args.remove("--handoff");
    let resume = args.remove("--resume");
    if handoff.is_some() == resume.is_some() {
        return Err("choose exactly one of --handoff or --resume".into());
    }
    let output = args.remove("--output");
    if output.as_ref().is_some_and(|p| Path::new(p).exists()) {
        return Err("experimental output must be a new file".into());
    }
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
    let inputs = match args.remove("--inputs").as_deref().unwrap_or("full") {
        "full" => PredictiveInputs::Full,
        "no_action" => PredictiveInputs::NoAction,
        "no_recurrent" => PredictiveInputs::NoRecurrent,
        _ => return Err("invalid --inputs".into()),
    };
    let parent = resume
        .as_ref()
        .map(|p| artifact::ExperimentalPredictiveState::read(Path::new(p)))
        .transpose()?;
    let source = match &parent {
        Some(p) => p.payload.source.clone(),
        None => serde_json::from_slice::<FoundationTrainingHandoffV1>(&read(
            handoff.as_ref().unwrap(),
        )?)?,
    };
    let (mut trainer, restored_head) = if let Some(p) = &parent {
        let (t, h) = p.restore()?;
        (t, Some(h))
    } else {
        let admitted = source.clone().admit()?;
        (
            CpuFoundationTrainer::from_checkpoint(
                admitted.phenotype,
                admitted.foundation,
                admitted.actor,
            )?,
            None,
        )
    };
    let data: PredictiveDataset = serde_json::from_slice(&dataset_bytes)?;
    data.validate_for(trainer.phenotype())?;
    let run = artifact::PredictiveRun::from_dataset(
        seed,
        &data,
        Sha256::digest(&dataset_bytes).into(),
        epochs,
        head_lr,
        inputs,
    )?;
    if let Some(previous) = &parent {
        previous.validate_next_run(&run)?;
    }
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
    let mut head = match restored_head {
        Some(h) => h,
        None => PredictiveHead::new(trainer.phenotype(), data.schema.clone(), neurons, 16, seed)?,
    };
    let before_train =
        head.evaluate_with_inputs(&trainer, &data, PredictiveSplit::Train, inputs)?;
    let before_validation =
        head.evaluate_with_inputs(&trainer, &data, PredictiveSplit::Validation, inputs)?;
    let start_step = trainer.optimizer_step();
    let start = Instant::now();
    for _ in 0..epochs {
        head.train_epoch(&mut trainer, &data, head_lr, inputs)?;
    }
    let training_seconds = start.elapsed().as_secs_f64();
    let test = if data
        .episodes
        .iter()
        .any(|e| e.split == PredictiveSplit::Test)
    {
        Some(head.evaluate_with_inputs(&trainer, &data, PredictiveSplit::Test, inputs)?)
    } else {
        None
    };
    let state_identity = if let Some(path) = &output {
        let state = artifact::ExperimentalPredictiveState::capture(
            &trainer,
            &head,
            source.clone(),
            run,
            parent.as_ref(),
        )?;
        state.write_new(Path::new(path))?;
        let restored = artifact::ExperimentalPredictiveState::read(Path::new(path))?;
        let (restored_trainer, restored_head) = restored.restore()?;
        if restored_trainer.checkpoint() != trainer.checkpoint() || restored_head != head {
            return Err("experimental state did not restore exactly".into());
        }
        let (disposable, _) = restored.into_disposable_founder()?;
        let a = trainer.checkpoint();
        let b = disposable.checkpoint();
        if a.weights != b.weights
            || a.first_moment != b.first_moment
            || a.second_moment != b.second_moment
            || a.update_ages != b.update_ages
            || a.optimizer_step != b.optimizer_step
        {
            return Err("disposable native rebind lost training state".into());
        }
        Some(restored.payload_sha256)
    } else {
        None
    };
    println!(
        "{}",
        serde_json::json!({
            "experiment": "action-conditioned-sensor-body-pilot-v1", "backend": trainer.hardware_receipt(),
            "source_description": data.source_description,
            "dataset_sha256": format!("{:x}", Sha256::digest(&dataset_bytes)),
            "source_identity_sha256": artifact::identity(&source)?,
            "neurons": trainer.phenotype().neuron_count(), "synapses": trainer.weights().len(),
            "trainable_recurrent_coordinates": selected.len(), "epochs": epochs, "seed": seed,
            "head_learning_rate": head_lr, "training_seconds": training_seconds,
            "inputs": inputs,
            "head_objective": head.objective_name(),
            "persistence_metric": "raw_observation_mse",
            "optimizer_step_before": start_step, "optimizer_step_after": trainer.optimizer_step(),
            "before_train": before_train, "after_train": head.evaluate_with_inputs(&trainer, &data, PredictiveSplit::Train, inputs)?,
            "before_validation": before_validation, "after_validation": head.evaluate_with_inputs(&trainer, &data, PredictiveSplit::Validation, inputs)?,
            "after_test": test, "persistence_train": persistence_metrics(&data,PredictiveSplit::Train)?,
            "persistence_validation": persistence_metrics(&data,PredictiveSplit::Validation)?,
            "persistence_test": if data.episodes.iter().any(|e|e.split==PredictiveSplit::Test) { Some(persistence_metrics(&data,PredictiveSplit::Test)?) } else {None},
            "head_exported_to_experiment_only": output.is_some(), "experimental_state": output,
            "experimental_state_payload_sha256": state_identity, "accepted_checkpoint_written": false,
            "throughput_improvement_measured": false, "gameplay_gain_measured": false
        })
    );
    Ok(())
}

//! Bounded diagnostic of saved assessment inputs; never a replacement battery.
//! Software Vulkan is used only by the offline trainer, never by a live backend.
use alife_core::*;
use alife_game_app::*;
use alife_gpu_backend::GpuPhenotypeUpload;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, TrainingSequence};
use alife_world::SavedBodyRoundingCompatibilityV1;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{error::Error, fs, path::Path};
#[path = "audit_retained_bridge/recovery_probe.rs"]
mod recovery_probe;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn scope<T>(proof: &SavedBodyRoundingCompatibilityV1, f: impl FnOnce() -> Result<T>) -> Result<T> {
    // A typed record already admitted during JSON decode can restore without
    // invoking the rounding exception again. The fixed proof remains scoped;
    // requiring a second use would reject that ordinary successful validation.
    let (result, _used) = alife_world::with_saved_body_rounding_compatibility(proof.clone(), f)?;
    result
}
fn write(path: impl AsRef<Path>, v: &impl serde::Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec(v)?)?;
    Ok(())
}
fn words<T: bytemuck::Pod>(v: &[T]) -> Vec<u32> {
    bytemuck::cast_slice(v).to_vec()
}
fn evaluation(v: &alife_training::TrainingReplayEvaluation) -> Value {
    json!({"candidate_logits":v.candidate_logits,"final_activations":v.final_activations,
           "final_activity_ema":v.final_activity_ema,"final_metabolic_load":v.final_metabolic_load})
}

fn main() -> Result<()> {
    let a = std::env::args().skip(1).collect::<Vec<_>>();
    if a.len() == 2 && a[0] == "--probe-quarter-recovery" {
        return recovery_probe::run(Path::new(&a[1]));
    }
    if !(a.len() == 4
        || a.len() == 5 && matches!(a[4].as_str(), "--offline-wgsl" | "--first-frame-only"))
    {
        return Err(
            "CURRENT EXPECTED_SHA BATTERY_DIRECTORY NEW_OUTPUT [--offline-wgsl|--first-frame-only]"
                .into(),
        );
    }
    let offline_requested = a.get(4).is_some_and(|flag| flag == "--offline-wgsl");
    let capture_limit = if a.get(4).is_some_and(|flag| flag == "--first-frame-only") {
        1
    } else {
        6
    };
    if offline_requested && !cfg!(feature = "gpu-tests") {
        return Err("--offline-wgsl requires the explicit gpu-tests build feature".into());
    }
    let source_path = Path::new(&a[0]);
    let bytes = fs::read(source_path)?;
    if format!("{:x}", Sha256::digest(&bytes)) != a[1] {
        return Err("source SHA mismatch".into());
    }
    let source_proposal = propose_current_body_rounding_compatibility(source_path, &a[1])?;
    let source = read_current_training_checkpoint_with_body_rounding(
        source_path,
        &a[1],
        &source_proposal.proof,
    )?;
    let admitted = source.handoff.clone().admit()?;
    let actor = admitted.actor.clone();
    let mut founder = CpuFoundationTrainer::from_checkpoint(
        admitted.phenotype,
        admitted.foundation.clone(),
        admitted.actor,
    )?;
    let original_actor = serde_json::to_vec(&founder.checkpoint())?;
    let output = Path::new(&a[3]);
    if output.exists() {
        return Err("diagnostic output already exists; preserve and inspect".into());
    }
    fs::create_dir(output)?;
    let qualification: Value =
        serde_json::from_slice(&fs::read(Path::new(&a[2]).join("qualification.json"))?)?;
    let proof: SavedBodyRoundingCompatibilityV1 =
        serde_json::from_value(qualification["body_compatibility"]["explicit_proof"].clone())?;

    #[cfg(feature = "gpu-tests")]
    let mut gpu = if offline_requested {
        let session = alife_runtime::GpuAuthoritativeSession::new_software_vulkan_training_test()?;
        if !session.backend().is_software_training_test() {
            return Err("expected explicit software offline fixture".into());
        }
        let mut t = alife_training::FoundationTrainer::from_session(
            session,
            founder.phenotype().clone(),
            admitted.foundation,
            actor.stage_mask.clone(),
            actor.config,
        )?;
        t.restore_checkpoint(&actor)?;
        write(
            output.join("offline-wgsl-hardware-receipt.json"),
            t.hardware_receipt(),
        )?;
        Some(t)
    } else {
        None
    };

    let p = founder.phenotype().clone();
    let upload = GpuPhenotypeUpload::try_from(&p)?;
    write(
        output.join("native-plan.json"),
        &json!({
            "phenotype": p,
            "canonical_to_packed":upload.canonical_to_local_synapse,
            "target_offsets":upload.target_offsets,"source_indices":upload.source_indices,"route_indices":upload.route_indices,
            "genetic":upload.genetic_weights,"alpha":upload.alpha,
            "decoder_plan_words":words(&upload.decoder_plans),"decoder_family_words":words(&upload.decoder_families),
            "decoder_weight_index_words":words(&upload.decoder_weight_indices),"memory_weight_indices":upload.memory_weight_indices,
            "decoder_metadata_words":words(&upload.decoder_eligibility_metadata),
            "actor":actor,"value":source.handoff.value,
            "source_sha256":a[1],"policy":source.handoff.policy_version,
            "sampling_protocol":"GPU-training conditional stochastic temperature 1; gameplay exploration is a separate protocol"
        }),
    )?;
    let mut first_sequence = None;
    for condition in ["held", "feeding_aged"] {
        let trial = Path::new(&a[2]).join(format!("{condition}-202611185878"));
        let cp: CpuCanonicalCheckpoint = scope(&proof, || {
            Ok(serde_json::from_slice(&fs::read(
                trial.join("initial-checkpoint.json"),
            )?)?)
        })?;
        let world = scope(&proof, || Ok(cp.world.restore_headless_world()?))?;
        let record = world
            .organism_registry()
            .iter()
            .next()
            .ok_or("no organism")?;
        let genome = &record.phenotype().brain_genome;
        let development = foundation_construction_development(
            genome,
            &BrainCapacityClass::n2048(),
            &record.phenotype().development_state_at(Tick::ZERO)?,
        )?;
        let (body_p, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome.clone(),
            development,
            source
                .handoff
                .compiler_inputs
                .n2048_candidate_asset()
                .unwrap()
                .clone(),
        )?;
        founder.bind_equivalent_assessment_body(&body_p)?;
        let retained = cp.residents[0]
            .inherited_neural_source
            .as_ref()
            .ok_or("retained state absent")?;
        let native_dendrites = alife_gpu_backend::GpuV11CausalState::for_phenotype(&p)?
            .checkpoint()
            .dendritic_branches;
        write(
            output.join(format!("{condition}-mapping.json")),
            &json!({"adapter_map":retained.canonical_to_live(&p)?,"source":retained,"retained_dendrites_match_native_initializer":retained.topology.v11_checkpoint.dendritic_branches == native_dendrites,"initial_neural":cp.residents[0].neural,"initial_world":cp.world.world}),
        )?;
        let mut host = scope(&proof, || {
            Ok(CpuCanonicalHost::from_checkpoint(cp, &founder)?)
        })?;
        let original_trace: Value =
            serde_json::from_slice(&fs::read(trial.join("care-trace.json"))?)?;
        let mut records = Vec::new();
        for _ in 0..4096 {
            let before = host.checkpoint(&founder)?;
            let step = host.step(&founder, 1.)?;
            for row in step.decisions {
                let q = TrainingSequence {
                    phenotype_hash: p.phenotype_hash(),
                    initial: row.initial.clone(),
                    ticks: vec![row.replay_tick.clone()],
                    burn_in_ticks: 0,
                    memory_candidate_gain: p
                        .candidate_decoder()
                        .memory_channel()
                        .map_or(0., |m| m.max_candidate_gain()),
                };
                if first_sequence.is_none() {
                    first_sequence = Some(q.clone());
                }
                let mut evaluations = Vec::new();
                for variant in [
                    "full",
                    "acquired_edge_ablated",
                    "native_canonical_topology",
                    "all_dendrites_and_acquired_edges_ablated",
                    "zero_lifetime",
                    "zero_fast",
                    "zero_somatic",
                ] {
                    let mut q = q.clone();
                    if matches!(
                        variant,
                        "acquired_edge_ablated"
                            | "native_canonical_topology"
                            | "all_dendrites_and_acquired_edges_ablated"
                    ) {
                        q.ticks[0].structural_synapses.clear();
                    }
                    if variant == "native_canonical_topology" {
                        q.initial.dendrites = native_dendrites.clone();
                    }
                    if variant == "all_dendrites_and_acquired_edges_ablated" {
                        q.initial.dendrites = Default::default();
                    }
                    if matches!(variant, "zero_lifetime" | "zero_fast" | "zero_somatic") {
                        let state = &before.residents[0].neural;
                        q.ticks[0].effective_weight_offsets = p
                            .synapses()
                            .iter()
                            .enumerate()
                            .map(|(index, synapse)| match variant {
                                "zero_lifetime" => synapse.alpha() * state.fast[index],
                                "zero_fast" => state.lifetime[index],
                                _ => 0.,
                            })
                            .collect();
                    }
                    let cpu = founder.evaluate_replay(&q)?;
                    let frame = row.patch.pre_action().perception();
                    let sample = sample_cpu_joint_action(
                        &p,
                        frame,
                        &cpu.candidate_logits[0],
                        before.sampling_seed,
                        row.sampling_counter.ok_or("sampling counter absent")?,
                        1.,
                    )?;
                    #[allow(unused_mut)]
                    let mut record = json!({"variant":variant,"sequence":q,"cpu":evaluation(&cpu),"cpu_sample":sample});
                    #[cfg(feature = "gpu-tests")]
                    if let Some(gpu) = gpu.as_mut() {
                        let wgsl = gpu.evaluate_replay(&q)?;
                        let wgsl_sample = sample_cpu_joint_action(
                            &p,
                            frame,
                            &wgsl.candidate_logits[0],
                            before.sampling_seed,
                            row.sampling_counter.unwrap(),
                            1.,
                        )?;
                        record["offline_wgsl"] = evaluation(&wgsl);
                        record["offline_wgsl_sample"] = serde_json::to_value(wgsl_sample)?;
                    }
                    evaluations.push(record);
                }
                let old = &original_trace["rows"][records.len()];
                let after = host.checkpoint(&founder)?;
                let before_world = scope(&proof, || Ok(before.world.restore_headless_world()?))?;
                let record = before_world
                    .organism_registry()
                    .get(row.organism_id)
                    .ok_or("diagnostic owner absent")?;
                let receptors = record
                    .biochemistry()
                    .neural_receptor_frame(record.phenotype())?;
                let effects = NeuralReceptorEffects::from_frame(
                    &receptors,
                    &NeuralReceptorPhenotype::compile(&p)?,
                )?;
                records.push(json!({"row":row,"evaluations":evaluations,"sampling_seed":before.sampling_seed,"native_receptor_effects":effects,"before_neural":before.residents[0].neural,"after_neural":after.residents[0].neural,"before_world":before.world.world,"after_world":after.world.world,"original_trace":old}));
            }
            if records.len() >= capture_limit {
                break;
            }
        }
        if records.len() != capture_limit {
            return Err("bounded diagnostic did not capture the requested decisions".into());
        }
        write(output.join(format!("{condition}-rows.json")), &records)?;
    }
    // Qualification input deliberately excites every decoder lane and family.
    // These artificial rows qualify arithmetic only, never creature performance.
    let mut q = first_sequence.ok_or("missing diagnostic sequence")?;
    q.ticks[0].candidates = (0..8)
        .map(|raw| {
            Ok(alife_training::TrainingReplayCandidate {
                family: CandidateActionFamily::try_from_raw(raw)?,
                decoder_inputs: std::array::from_fn(|lane| {
                    ((lane * 7 + raw as usize * 3) % 23) as f32 / 19. - 0.4
                }),
                innate_bias: 0.07 * raw as f32,
            })
        })
        .collect::<std::result::Result<Vec<_>, ScaffoldContractError>>()?;
    q.initial.activations = (0..p.neuron_count())
        .map(|i| 0.03 + (i % 11) as f32 * 0.007)
        .collect();
    #[allow(unused_mut)]
    let mut synthetic = json!({"scope":"arithmetic qualification only; artificial inputs are not scored trials","sequence":q,"cpu":evaluation(&founder.evaluate_replay(&q)?)});
    #[cfg(feature = "gpu-tests")]
    if let Some(gpu) = gpu.as_mut() {
        synthetic["offline_wgsl"] = evaluation(&gpu.evaluate_replay(&q)?);
    }
    write(output.join("nonzero-all-families.json"), &synthetic)?;
    if fs::read(source_path)? != bytes
        || serde_json::to_vec(&founder.checkpoint())? != original_actor
    {
        return Err("diagnostic mutated source or actor optimizer".into());
    }
    #[cfg(feature = "gpu-tests")]
    if let Some(gpu) = gpu.as_ref() {
        if gpu.checkpoint()? != actor {
            return Err("offline WGSL read-only evaluation mutated actor".into());
        }
    }
    write(
        output.join("completion.json"),
        &json!({"source_unchanged":true,"actor_unchanged":true,"value_unchanged":true,"conditions":2,"decisions_each":capture_limit,"new_training_allocations":0,"full_battery_rerun":false,"gpu_live_dispatch":false,"hardware_live_parity":false}),
    )?;
    println!("{}", output.display());
    Ok(())
}

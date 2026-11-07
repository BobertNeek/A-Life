//! Bounded native CPU learning assay. All lives are fresh and disposable.
//! This never reads retained lives or changes accepted/default founder assets.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask, TrainingSequence};
use alife_world::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path, time::Instant};

#[path = "support/care_candidate.rs"]
mod care_candidate;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Spec {
    seed: u64,
    preparation_ticks: usize,
    initial_energy: f32,
    baseline_ticks: usize,
    life_ticks: usize,
    max_a_consumptions: usize,
    nutrition: f32,
    a_harm: f32,
    regrow_ticks: u32,
    minimum_health: f32,
    maximum_injury: f32,
    sleep_policy: String,
}
impl Spec {
    fn predeclared(seed: u64) -> Self {
        Self {
            seed,
            preparation_ticks: 600,
            initial_energy: 0.4,
            baseline_ticks: 24,
            life_ticks: 256,
            max_a_consumptions: 3,
            nutrition: 0.05,
            a_harm: 0.35,
            regrow_ticks: 16,
            minimum_health: 0.70,
            maximum_injury: 0.25,
            sleep_policy: "Request RecoveryProtocol once immediately after the first actual A consumption, regardless of its measured value. Never request without exposure.".into(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Arm {
    name: String,
    action_gain: f32,
    memory_reader: bool,
    memory_gain: f32,
    family_valence_reader: bool,
    filename: String,
    sha256: String,
}
#[derive(Serialize, Deserialize)]
struct Manifest {
    schema: String,
    source_commit: String,
    executable_sha256: String,
    archived_v6_sha256: String,
    experiment: String,
    retrieval_source_sha256: String,
    spec: Spec,
    arms: Vec<Arm>,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn create(path: &Path) -> Result<fs::File> {
    Ok(fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?)
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut file = create(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
fn write_fresh_diagnostic(path: &Path, value: &impl Serialize) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    let compressed = zstd::stream::encode_all(bytes.as_slice(), 3)?;
    let mut file = create(path)?;
    file.write_all(&compressed)?;
    file.sync_all()?;
    Ok(digest(&bytes))
}
fn append(file: &mut fs::File, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *file, value)?;
    file.write_all(b"\n")?;
    file.sync_data()?;
    Ok(())
}
fn candidate(seed: u64, gain: f32) -> Result<FoundationWeightAsset> {
    candidate_with_memory(seed, gain, false, 0.5)
}
fn candidate_with_memory(
    seed: u64,
    gain: f32,
    reader: bool,
    memory_gain: f32,
) -> Result<FoundationWeightAsset> {
    candidate_with_readouts(seed, gain, reader, memory_gain, false)
}
fn candidate_with_readouts(
    seed: u64,
    gain: f32,
    reader: bool,
    memory_gain: f32,
    family_valence_reader: bool,
) -> Result<FoundationWeightAsset> {
    if family_valence_reader && (!reader || memory_gain != 12.) {
        return Err("the family-value factor requires the frozen target reader and cap 12".into());
    }
    let (_, p, source) = if memory_gain == 12. {
        care_candidate::source_memory_gain(seed)?
    } else if memory_gain == 0.5 {
        care_candidate::source(seed, true)?
    } else {
        return Err("unsupported memory gain".into());
    };
    let (mut weights, _) = care_candidate::whole_care(&p)?;
    for (synapse, weight) in p.synapses().iter().zip(&mut weights) {
        if matches!(synapse.kind(), CompiledSynapseKind::Decoder(c)
            if c.head() == DecoderHeadKind::ActionCandidate)
        {
            *weight *= gain;
        }
    }
    if reader {
        let mut memory_counts = std::collections::BTreeMap::new();
        for synapse in p.synapses() {
            if let CompiledSynapseKind::Decoder(c) = synapse.kind() {
                if c.head() == DecoderHeadKind::MemoryContext {
                    *memory_counts
                        .entry((c.family().raw(), c.input_lane()))
                        .or_insert(0usize) += 1;
                }
            }
        }
        for (i, synapse) in p.synapses().iter().enumerate() {
            let CompiledSynapseKind::Decoder(c) = synapse.kind() else {
                continue;
            };
            if c.head() != DecoderHeadKind::MemoryContext {
                continue;
            }
            let total = match (c.family(), c.input_lane()) {
                (CandidateActionFamily::Ingest, 25) => -16.,
                (CandidateActionFamily::Ingest, 26) => -64.,
                (CandidateActionFamily::Ingest, 32) if family_valence_reader => 32.,
                (CandidateActionFamily::Approach | CandidateActionFamily::Contact, 25) => -8.,
                (CandidateActionFamily::Approach | CandidateActionFamily::Contact, 26) => -32.,
                (CandidateActionFamily::Avoid, 25) => 16.,
                (CandidateActionFamily::Avoid, 26) => 128.,
                _ => 0.,
            };
            let copies = memory_counts[&(c.family().raw(), c.input_lane())];
            weights[i] = total / copies as f32;
        }
    }
    Ok(FoundationWeightAsset::from_trained_weights(
        &p,
        weights,
        source.manifest().training_stage(),
    )?)
}
// Bind the executable to the actual compiled retrieval source, not a caller's
// phase-on/off label. Both factor worktrees have identical assay source.
fn retrieval_source_sha256() -> String {
    let sources: [(&str, &[u8]); 4] = [
        (
            "memory.rs",
            include_bytes!("../../alife_core/src/memory.rs"),
        ),
        (
            "candidate_index.rs",
            include_bytes!("../../alife_core/src/memory/candidate_index.rs"),
        ),
        (
            "candidate_recall.rs",
            include_bytes!("../../alife_core/src/memory/candidate_recall.rs"),
        ),
        (
            "sidecar.rs",
            include_bytes!("../../alife_core/src/memory/sidecar.rs"),
        ),
    ];
    let mut hash = Sha256::new();
    for (name, contents) in sources {
        hash.update(name.as_bytes());
        hash.update([0]);
        hash.update((contents.len() as u64).to_le_bytes());
        hash.update(contents);
    }
    format!("{:x}", hash.finalize())
}
type ArmSpec = (&'static str, f32, bool, f32, bool);
fn arm_specs(experiment: &str) -> Result<Vec<ArmSpec>> {
    Ok(match experiment {
        "original" => vec![
            ("original-v6", 1., false, 0.5, false),
            ("action-gain-quarter", 0.25, false, 0.5, false),
        ],
        "memory" => vec![
            ("reader-off-cap-half", 0.25, false, 0.5, false),
            ("reader-on-cap-half", 0.25, true, 0.5, false),
            ("reader-off-cap-twelve", 0.25, false, 12., false),
            ("reader-on-cap-twelve", 0.25, true, 12., false),
        ],
        "family-value" => vec![
            ("family-value-off", 0.25, true, 12., false),
            ("family-value-on", 0.25, true, 12., true),
        ],
        _ => return Err("unknown predeclared factor design".into()),
    })
}
fn prepare(root: &Path, seed: u64, commit: &str, archive: &Path, experiment: &str) -> Result<()> {
    if seed == 0 || commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("a nonzero fixed seed and full source commit are required".into());
    }
    let original = candidate(2026110901, 1.)?.encode_canonical()?;
    let archive_bytes = fs::read(archive)?;
    if original != archive_bytes {
        return Err("shared v6 construction is not byte-identical to the archived v6".into());
    }
    let specs = arm_specs(experiment)?;
    fs::create_dir(root)?;
    let mut arms = Vec::new();
    for (name, gain, reader, memory_gain, family_valence_reader) in specs {
        let bytes =
            candidate_with_readouts(seed, gain, reader, memory_gain, family_valence_reader)?
                .encode_canonical()?;
        let filename = format!("{name}.alife-foundation");
        let mut file = create(&root.join(&filename))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        arms.push(Arm {
            name: name.into(),
            action_gain: gain,
            memory_reader: reader,
            memory_gain,
            family_valence_reader,
            filename,
            sha256: digest(&bytes),
        });
    }
    write_json(
        &root.join("manifest.json"),
        &Manifest {
            schema: "fresh-native-learning-pilot-v2".into(),
            source_commit: commit.into(),
            executable_sha256: digest(&fs::read(std::env::current_exe()?)?),
            archived_v6_sha256: digest(&archive_bytes),
            experiment: experiment.into(),
            retrieval_source_sha256: retrieval_source_sha256(),
            spec: Spec::predeclared(seed),
            arms,
        },
    )
}
fn load_manifest(root: &Path) -> Result<Manifest> {
    let manifest: Manifest = serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
    let expected = arm_specs(&manifest.experiment)?;
    if manifest.schema != "fresh-native-learning-pilot-v2"
        || manifest.spec != Spec::predeclared(manifest.spec.seed)
        || manifest.executable_sha256 != digest(&fs::read(std::env::current_exe()?)?)
        || manifest.retrieval_source_sha256 != retrieval_source_sha256()
        || manifest.arms.len() != expected.len()
    {
        return Err("predeclared manifest, retrieval source or frozen executable mismatch".into());
    }
    for (arm, (name, action_gain, memory_reader, memory_gain, family_valence_reader)) in
        manifest.arms.iter().zip(expected)
    {
        if arm.name != name
            || arm.action_gain != action_gain
            || arm.memory_reader != memory_reader
            || arm.memory_gain != memory_gain
            || arm.family_valence_reader != family_valence_reader
            || arm.filename != format!("{name}.alife-foundation")
        {
            return Err("candidate factor design differs from predeclaration".into());
        }
        let bytes = fs::read(root.join(&arm.filename))?;
        if digest(&bytes) != arm.sha256
            || bytes
                != candidate_with_readouts(
                    manifest.spec.seed,
                    arm.action_gain,
                    arm.memory_reader,
                    arm.memory_gain,
                    arm.family_valence_reader,
                )?
                .encode_canonical()?
        {
            return Err("frozen candidate does not match its predeclared construction".into());
        }
    }
    Ok(manifest)
}

struct Life {
    host: CpuCanonicalHost,
    trainer: CpuFoundationTrainer,
    own: OrganismId,
    targets: Vec<WorldEntityId>,
}
fn make_life(
    root: &Path,
    manifest: &Manifest,
    arm: &Arm,
    scene: &str,
    episode: Option<Episode>,
) -> Result<Life> {
    let spec = &manifest.spec;
    let seed = episode.map_or(spec.seed, |e| e.seed);
    let (config, _, _) = care_candidate::source(seed, true)?;
    let asset = FoundationWeightAsset::decode_canonical(&fs::read(root.join(&arm.filename))?)?;
    let mut game = if arm.memory_gain == 12. {
        create_canonical_new_game_with_n2048_memory_gain_candidate(&config, &asset)?
    } else {
        create_canonical_new_game_with_n2048_experimental_candidate(&config, &asset)?
    };
    let original = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let capacity = BrainCapacityClass::n2048();
    let development = foundation_construction_development(
        &original.phenotype().brain_genome,
        &capacity,
        &original.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    let (p, _) = if arm.memory_gain == 12. {
        PhenotypeCompiler::compile_n2048_memory_gain_foundation_candidate(
            original.phenotype().brain_genome.clone(),
            development,
            asset.clone(),
        )?
    } else {
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            original.phenotype().brain_genome.clone(),
            development,
            asset.clone(),
        )?
    };
    let trainer = CpuFoundationTrainer::new(
        p.clone(),
        asset,
        StageTrainableMask::recurrent_only(&p)?,
        AdamWConfig::default(),
    )?;
    let own = original.organism_id();
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", own, Vec3f::ZERO)
        .terrain_zone(
            1,
            "assay",
            TerrainZoneKind::Meadow,
            Vec3f::ZERO,
            12.,
            0.,
            0.,
        )
        .build()?;
    let count = if scene == "learning" { 2 } else { 1 };
    let mut targets = Vec::new();
    // Identical geometry, nutrition, chemistry and material. Appearance and
    // object identity distinguish the two resources, never a hidden harm label.
    let common = GroundedPhysicalProperties {
        velocity: Vec3f::ZERO,
        color: [0.9, 0.2, 0.2],
        material: [0.35, 0.15, 0.65],
        shape: [0.2, 0.8, 0.2],
        chemical: [if scene == "bitter" { -0.85 } else { 0.85 }, 0.1, 0.2],
        surface_temperature: 0.1,
        terrain: [0.5, 0.25],
    };
    for i in 0..count {
        let position = episode_position(scene, i, episode);
        let id = world.editor_spawn_object(WorldEditorSpawnSpec {
            label: if i == 0 { "A" } else { "B" }.into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position,
            nutrition: if scene == "learning" {
                spec.nutrition
            } else {
                0.6
            },
            hazard_pain: if scene == "learning" && i == 0 {
                spec.a_harm
            } else {
                0.
            },
            radius: 0.5,
            token_id: None,
        })?;
        let mut physical = common;
        if i == 1 {
            physical.color = [0.2, 0.2, 0.9];
            physical.shape = [0.8, 0.2, 0.8];
        }
        world.set_grounded_physical_properties(id, physical)?;
        if scene == "learning" {
            world.track_resource_lifecycle(id, EcologyZoneId(1), spec.regrow_ticks, 512)?;
        }
        targets.push(id);
    }
    let mut chemistry = *original.biochemistry();
    chemistry.body.set_energy(spec.initial_energy)?;
    world.register_organism_record(WorldOrganismRecord::new(
        own,
        world.entity_id("learner").unwrap(),
        original.genome().clone(),
        original.phenotype().clone(),
        chemistry,
        original.birth_tick(),
    )?)?;
    game.world = world;
    let sampling_seed = (seed ^ (seed >> 32)) as u32;
    let (host, _) =
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, sampling_seed, |world| {
            for _ in 0..spec.preparation_ticks {
                world.try_advance_tick()?;
            }
            Ok::<_, ScaffoldContractError>(())
        })?;
    Ok(Life {
        host,
        trainer,
        own,
        targets,
    })
}
fn offsets(p: &BrainPhenotype, neural: &CpuOrganismCheckpoint) -> Vec<f32> {
    p.synapses()
        .iter()
        .enumerate()
        .map(|(i, s)| neural.lifetime[i] + s.alpha() * neural.fast[i])
        .collect()
}
fn bank_stats(values: &[f32], old: &[f32]) -> Value {
    json!({"nonzero":values.iter().filter(|v| **v != 0.).count(),
        "changed":values.iter().zip(old).filter(|(a,b)| a.to_bits()!=b.to_bits()).count(),
        "l1":values.iter().map(|v| f64::from(v.abs())).sum::<f64>(),
        "max_abs":values.iter().map(|v| v.abs()).fold(0.,f32::max),
        "max_delta":values.iter().zip(old).map(|(a,b)|(a-b).abs()).fold(0.,f32::max)})
}
fn replay(
    trainer: &CpuFoundationTrainer,
    row: &CpuCollectedDecision,
    weights: &[f32],
) -> Result<Vec<f32>> {
    let mut tick = row.replay_tick.clone();
    tick.effective_weight_offsets = weights.to_vec();
    Ok(trainer
        .evaluate_replay(&TrainingSequence {
            phenotype_hash: trainer.phenotype().phenotype_hash(),
            initial: row.initial.clone(),
            ticks: vec![tick],
            burn_in_ticks: 0,
            memory_candidate_gain: trainer
                .phenotype()
                .candidate_decoder()
                .memory_channel()
                .map_or(0., |m| m.max_candidate_gain()),
        })?
        .candidate_logits
        .remove(0))
}
// This detached diagnostic changes only explicit episodic decoder inputs.
// It never writes the organism, a memory bank, a receptor, or an action.
fn replay_without_episodic_context(
    trainer: &CpuFoundationTrainer,
    row: &CpuCollectedDecision,
) -> Result<Vec<f32>> {
    let mut detached = row.clone();
    for candidate in &mut detached.replay_tick.candidates {
        candidate.decoder_inputs[24..36].fill(0.);
    }
    replay(
        trainer,
        &detached,
        &row.replay_tick.effective_weight_offsets,
    )
}
// Only the Ingest family's acquired valence lane is removed. Target fear/pain,
// all other families, weights, physiology and sampling state remain unchanged.
fn replay_without_ingest_family_valence(
    trainer: &CpuFoundationTrainer,
    row: &CpuCollectedDecision,
) -> Result<Vec<f32>> {
    let mut detached = row.clone();
    for candidate in &mut detached.replay_tick.candidates {
        if candidate.family == CandidateActionFamily::Ingest {
            candidate.decoder_inputs[32] = 0.;
        }
    }
    replay(
        trainer,
        &detached,
        &row.replay_tick.effective_weight_offsets,
    )
}
fn margins(row: &CpuCollectedDecision, logits: &[f32], targets: &[WorldEntityId]) -> Value {
    let candidates = row.patch.pre_action().perception().candidates();
    let best = |target, family| {
        candidates
            .iter()
            .zip(logits)
            .filter(|(c, _)| c.target.entity == Some(target) && c.family == family)
            .map(|(_, l)| *l)
            .max_by(f32::total_cmp)
    };
    let a = best(targets[0], CandidateActionFamily::Ingest);
    let b = targets
        .get(1)
        .and_then(|&id| best(id, CandidateActionFamily::Ingest));
    json!({"ingest_a":a,"ingest_b":b,"ingest_a_minus_b":a.zip(b).map(|(a,b)|a-b),
        "approach_a":best(targets[0],CandidateActionFamily::Approach),
        "avoid_a":best(targets[0],CandidateActionFamily::Avoid)})
}
fn consumed(row: &CpuCollectedDecision, target: WorldEntityId) -> bool {
    let outcome = row.patch.outcome();
    let matches = |physical: &PhysicalActionOutcome| {
        physical.target_entity == Some(target) && physical.contact == PhysicalContactKind::Consumed
    };
    matches(&outcome.physical)
        || outcome
            .joint
            .as_ref()
            .is_some_and(|joint| joint.channel_outcomes.iter().any(|c| matches(&c.physical)))
}
fn cold_check(host: &CpuCanonicalHost, trainer: &CpuFoundationTrainer) -> Result<Value> {
    let checkpoint = host.checkpoint(trainer)?;
    let bytes = serde_json::to_vec(&checkpoint)?;
    let mut cold = CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes)?, trainer)?;
    assert_eq!(bytes, serde_json::to_vec(&cold.checkpoint(trainer)?)?);
    let mut hot = host.snapshot();
    let hot_step = hot.step(trainer, 1.);
    let cold_step = cold.step(trainer, 1.);
    let next = match (hot_step, cold_step) {
        (Ok(h), Ok(c)) => {
            assert_eq!(
                serde_json::to_vec(&h.decisions)?,
                serde_json::to_vec(&c.decisions)?
            );
            json!({"completed":true,"decisions":h.decisions.len()})
        }
        (Err(h), Err(c)) => {
            assert_eq!(h.to_string(), c.to_string());
            json!({"completed":false,"identical_atomic_error":h.to_string()})
        }
        _ => return Err("hot and cold next-step results diverged".into()),
    };
    assert_eq!(
        serde_json::to_vec(&hot.checkpoint(trainer)?)?,
        serde_json::to_vec(&cold.checkpoint(trainer)?)?
    );
    Ok(json!({"checkpoint_roundtrip_exact":true,"next_step":next,"next_checkpoint_exact":true}))
}

fn run(
    root: &Path,
    manifest: &Manifest,
    arm: &Arm,
    scene: &str,
    episode: Option<Episode>,
) -> Result<Value> {
    let started = Instant::now();
    let Life {
        mut host,
        trainer,
        own,
        targets,
    } = make_life(root, manifest, arm, scene, episode)?;
    let prefix = format!("{}.{}", arm.name, scene);
    let mut journal = create(&root.join(format!("{prefix}.jsonl")))?;
    let founder_before = serde_json::to_vec(&trainer.checkpoint())?;
    let initial = host.checkpoint(&trainer)?;
    let initial_offsets = offsets(trainer.phenotype(), &initial.residents[0].neural);
    let mut previous = initial.clone();
    let mut anchor: Option<CpuCollectedDecision> = None;
    let mut last_row: Option<CpuCollectedDecision> = None;
    let mut counts = vec![0usize; targets.len()];
    let mut negative_a_meals = 0;
    let mut phase_meals_checked = 0;
    let mut all_consumed_known = true;
    let mut negative_a_experienced_valence_meals = 0;
    let mut request_tick = None;
    let mut consolidation_count = 0;
    let mut post_sleep_decisions = 0;
    let mut decisions = 0;
    let mut steps = 0;
    let mut opportunities_a = 0;
    let mut contact_opportunities_a = 0;
    let mut paired_ingest_opportunities = 0;
    let mut paired_contact_opportunities = 0;
    let mut captured_first_post_harm_a_opportunity = false;
    let mut stopped = "tick_budget".to_owned();
    let learning = scene == "learning";
    let limit = if learning {
        manifest.spec.life_ticks
    } else {
        manifest.spec.baseline_ticks
    };
    append(
        &mut journal,
        &json!({"event":"start","scene":scene,"arm":arm,"spec":manifest.spec,
        "backend":"native-cpu","motor_channels":trainer.phenotype().candidate_decoder().factorized_motor_channels(trainer.phenotype())?,"episode":episode,"construction_seed":manifest.spec.seed,"experiment":manifest.experiment,"retrieval_source_sha256":manifest.retrieval_source_sha256,"fresh_disposable":true,"initial_biology":host.world().organism_registry().get(own).unwrap().biochemistry(),
        "target_objects":targets.iter().map(|id|WorldObjectSaveState::from(host.world().entity(*id).unwrap().clone())).collect::<Vec<_>>(),
        "no_adam_updates":true,"no_learning_off_comparator":true}),
    )?;
    for index in 0..limit {
        let before_record = host.world().organism_registry().get(own).unwrap();
        let before_receptors = before_record
            .biochemistry()
            .neural_receptor_frame(before_record.phenotype())?;
        let step = match host.step(&trainer, 1.) {
            Ok(step) => step,
            Err(error) => {
                stopped = format!("host_error: {error}");
                break;
            }
        };
        steps += 1;
        consolidation_count += step.sleep_consolidations;
        let current = host.checkpoint(&trainer)?;
        let resident = &current.residents[0];
        let old = &previous.residents[0].neural;
        let current_offsets = offsets(trainer.phenotype(), &resident.neural);
        let record = host.world().organism_registry().get(own).unwrap();
        let mut first_a_consumption = false;
        for row in &step.decisions {
            decisions += 1;
            if request_tick.is_some() && consolidation_count > 0 {
                post_sleep_decisions += 1;
            }
            let frame = row.patch.pre_action().perception();
            let outcome = row.patch.outcome();
            let physiology = outcome
                .measured_physiology
                .as_ref()
                .ok_or("measured physiology missing")?;
            let credit = OutcomeCreditPacket::from_sealed_patch(&row.patch)?
                .with_biochemical_receptors(&before_receptors)?;
            let parameters = record.phenotype().brain_genome.plasticity_parameters();
            let profile = parameters.action_candidate_credit_profile().map_or_else(
                || parameters.receptor_profile(),
                |profile| profile.receptor_profile(),
            );
            assert_eq!(
                profile.project(&credit.modulator().frame())?.to_bits(),
                row.biological_reward.to_bits()
            );
            let logits = replay(&trainer, row, &row.replay_tick.effective_weight_offsets)?;
            let has_ingest = |id| {
                frame.candidates().iter().any(|c| {
                    c.family == CandidateActionFamily::Ingest && c.target.entity == Some(id)
                })
            };
            opportunities_a += usize::from(has_ingest(targets[0]));
            paired_ingest_opportunities +=
                usize::from(targets.len() == 2 && has_ingest(targets[0]) && has_ingest(targets[1]));
            let has_contact_ingest = |id| {
                frame.candidates().iter().any(|c| {
                    c.family == CandidateActionFamily::Ingest
                    && c.target.entity == Some(id)
                    // GroundedObjectSlotV1::candidate_features puts contact in lane 18.
                    && c.features.0[18] > 0.
                })
            };
            contact_opportunities_a += usize::from(has_contact_ingest(targets[0]));
            paired_contact_opportunities += usize::from(
                targets.len() == 2
                    && has_contact_ingest(targets[0])
                    && has_contact_ingest(targets[1]),
            );
            let mut meals = Vec::new();
            for (i, &target) in targets.iter().enumerate() {
                if consumed(row, target) {
                    counts[i] += 1;
                    meals.push(i);
                    if episode.is_some() {
                        phase_meals_checked += 1;
                        let decision = serde_json::to_value(row.patch.decision())?;
                        let records = serde_json::to_value(&resident.memory.records)?;
                        all_consumed_known &=
                            consumed_phase_matches(&decision, &records, target.raw());
                    }
                    if i == 0 {
                        negative_a_meals += usize::from(
                            row.biological_reward < 0. && physiology.aversive_harm() > 0.,
                        );
                        negative_a_experienced_valence_meals += usize::from(
                            outcome.experienced_valence().raw() < 0.
                                && physiology.aversive_harm() > 0.,
                        );
                        first_a_consumption = counts[0] == 1;
                    }
                }
            }
            if anchor.is_none() {
                write_fresh_diagnostic(
                    &root.join(format!("{prefix}.anchor.fresh-diagnostic.json.zst")),
                    row,
                )?;
                anchor = Some(row.clone());
            }
            let first_reencounter = learning
                && counts[0] > 0
                && !first_a_consumption
                && has_contact_ingest(targets[0])
                && !captured_first_post_harm_a_opportunity;
            captured_first_post_harm_a_opportunity |= first_reencounter;
            let inspect = first_reencounter
                || index == 0
                || index % 32 == 0
                || !meals.is_empty()
                || (request_tick.is_some() && post_sleep_decisions == 1);
            let diagnostic = if inspect {
                let origin = replay(&trainer, row, &initial_offsets)?;
                let after = replay(&trainer, row, &current_offsets)?;
                let without_episodic = replay_without_episodic_context(&trainer, row)?;
                let without_family = replay_without_ingest_family_valence(&trainer, row)?;
                let before_choice = sample_cpu_joint_action(
                    trainer.phenotype(),
                    frame,
                    &logits,
                    current.sampling_seed,
                    row.sampling_counter.unwrap(),
                    1.,
                )?;
                assert_eq!(
                    serde_json::to_vec(&before_choice)?,
                    serde_json::to_vec(&row.action)?
                );
                let frozen = json!({"schema":"fresh-detached-offset-assay-v1","source_row":row,
                    "initial_offset_bank":initial_offsets,"post_update_offset_bank":current_offsets,
                    "post_neural_bank":resident.neural,"candidate_sha256":arm.sha256,
                    "all_other_sequence_fields_identical":true});
                let provenance = write_fresh_diagnostic(
                    &root.join(format!("{prefix}.tick-{index}.fresh-diagnostic.json.zst")),
                    &frozen,
                )?;
                Some(json!({"uncompressed_provenance_sha256":provenance,
                    "initial_bank":margins(row,&origin,&targets),"actual_pre_update":margins(row,&logits,&targets),
                    "post_update_bank":margins(row,&after,&targets),
                    "without_explicit_episodic_context":margins(row,&without_episodic,&targets),
                    "without_ingest_family_valence_context":margins(row,&without_family,&targets),
                    "without_ingest_family_valence_choice_same_counter":sample_cpu_joint_action(trainer.phenotype(),frame,&without_family,current.sampling_seed,row.sampling_counter.unwrap(),1.)?,
                    "without_explicit_episodic_choice_same_counter":sample_cpu_joint_action(trainer.phenotype(),frame,&without_episodic,current.sampling_seed,row.sampling_counter.unwrap(),1.)?,
                    "post_update_choice_same_counter":sample_cpu_joint_action(trainer.phenotype(),frame,&after,current.sampling_seed,row.sampling_counter.unwrap(),1.)?,
                    "interpretation":"Detached counterfactuals only. Offset swaps hold actual physiology, recurrent initial state, gates, memory inputs and structural context fixed. The episodic comparison zeroes only decoder input lanes 24 through 35. The family-valence comparison zeroes only Ingest lane 32. Both retain actual offsets and every other sequence field."}))
            } else {
                None
            };
            let watched = trainer.phenotype().synapses().iter().enumerate().filter(|(_,s)|
                matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.input_lane()==15 && [CandidateActionFamily::Ingest,CandidateActionFamily::Approach].contains(&c.family()))
                && s.genetic_weight()!=0.).map(|(i,s)| {
                    let receptor=&trainer.phenotype().plasticity_receptors()[s.receptor_index() as usize];
                    json!({"index":i,"kind":s.kind(),"source":s.source(),"genetic":s.genetic_weight(),"alpha":s.alpha(),
                        "learning_rate":receptor.learning_rate(),"normalization_rate":receptor.normalization_rate(),
                        "projected_credit":receptor.receptor_profile().project(&credit.modulator().frame()).unwrap(),
                        "eligibility":resident.neural.eligibility[i],"fast":resident.neural.fast[i],"lifetime":resident.neural.lifetime[i],
                        "effective":s.genetic_weight()+current_offsets[i],"effective_update":current_offsets[i]-row.replay_tick.effective_weight_offsets[i]})
                }).collect::<Vec<_>>();
            append(
                &mut journal,
                &json!({"event":"decision","index":index,"tick":step.tick_before,"sampling_counter":row.sampling_counter,
                "action":row.action,"selected_command":row.patch.decision().selected_action,
                "candidates":frame.candidates(),"logits":logits,
                "memory_and_cognitive_inputs":row.replay_tick.candidates.iter().map(|c|&c.decoder_inputs[24..]).collect::<Vec<_>>(),
                "outcome":outcome,"biological_reward":row.biological_reward,"experienced_valence":outcome.experienced_valence(),
                "homeostatic_improvement":physiology.homeostatic_improvement(),"aversive_harm":physiology.aversive_harm(),
                "credit_modulator":credit.modulator(),"consumed_target_indices":meals,"target_margins":margins(row,&logits,&targets),
                "watched_readouts":watched,"detached_assay":diagnostic}),
            )?;
            last_row = Some(row.clone());
        }
        append(
            &mut journal,
            &json!({"event":"boundary","index":index,"tick":step.tick_after,
            "sleep":resident.sleep,"sleeping_organisms":step.sleeping_organisms,"consolidations":step.sleep_consolidations,
            "fast":bank_stats(&resident.neural.fast,&old.fast),"lifetime":bank_stats(&resident.neural.lifetime,&old.lifetime),
            "eligibility":bank_stats(&resident.neural.eligibility,&old.eligibility),"memory_count":resident.memory.records.len(),
            "body":record.biochemistry().body,"homeostasis":record.biochemistry().homeostasis,
            "position":host.world().entity(record.world_entity_id()).unwrap().position,
            "targets":targets.iter().map(|id|WorldObjectSaveState::from(host.world().entity(*id).unwrap().clone())).collect::<Vec<_>>(),
            "ecology":host.world().ecology_metrics()}),
        )?;
        if learning && first_a_consumption && request_tick.is_none() {
            host.request_recovery_sleep(own)?;
            request_tick = Some(host.world().tick());
            append(
                &mut journal,
                &json!({"event":"requested_recovery_sleep","tick":request_tick,"reason":"Predeclared first A-consumption boundary, independent of measured value."}),
            )?;
        }
        previous = current;
        let biology = host.world().organism_registry().get(own).unwrap();
        if !biology.lifecycle().is_alive() {
            stopped = "organism_not_alive".into();
            break;
        }
        if learning
            && (biology.biochemistry().body.health < manifest.spec.minimum_health
                || biology.biochemistry().body.injury > manifest.spec.maximum_injury)
        {
            stopped = "vitality_safeguard".into();
            break;
        }
        if learning && counts[0] >= manifest.spec.max_a_consumptions {
            stopped = "harmful_consumption_cap".into();
            break;
        }
        if index % 32 == 0 {
            eprintln!(
                "{} {} tick={} A={} B={:?} sleep={:?}",
                arm.name,
                scene,
                index,
                counts[0],
                counts.get(1),
                previous.residents[0].sleep.state().phase
            );
        }
    }
    assert_eq!(founder_before, serde_json::to_vec(&trainer.checkpoint())?);
    let final_checkpoint = host.checkpoint(&trainer)?;
    let final_offsets = offsets(trainer.phenotype(), &final_checkpoint.residents[0].neural);
    let anchor_assay = if let Some(anchor) = &anchor {
        let initial_logits = replay(&trainer, anchor, &initial_offsets)?;
        let final_logits = replay(&trainer, anchor, &final_offsets)?;
        Some(
            json!({"initial":margins(anchor,&initial_logits,&targets),"final":margins(anchor,&final_logits,&targets),
            "fixed_genuine_first_row":true}),
        )
    } else {
        None
    };
    let diagnostic_digest = write_fresh_diagnostic(
        &root.join(format!("{prefix}.final.fresh-diagnostic.json.zst")),
        &json!({"fresh_generated_only":true,"initial_offsets":initial_offsets,"final_offsets":final_offsets,
            "anchor_row":anchor,"last_row":last_row,"final_checkpoint":final_checkpoint}),
    )?;
    let cold = cold_check(&host, &trainer)?;
    let record = host.world().organism_registry().get(own).unwrap();
    let summary = json!({"schema":"fresh-native-life-result-v1","complete":true,"backend":"native-cpu","scene":scene,"arm":arm,
        "source_commit":manifest.source_commit,"experiment":manifest.experiment,"retrieval_source_sha256":manifest.retrieval_source_sha256,"seed":episode.map_or(manifest.spec.seed, |e|e.seed),"episode":episode,"construction_seed":manifest.spec.seed,"phase_provenance":{"meals_checked":phase_meals_checked,"all_consumed_known":all_consumed_known},"steps":steps,"decisions":decisions,"stop_reason":stopped,
        "consumptions":counts,"negative_a_meals":negative_a_meals,
        "negative_a_meals_definition":"Native biological_reward below zero with measured aversive harm above zero.",
        "negative_a_experienced_valence_meals":negative_a_experienced_valence_meals,
        "a_ingest_opportunities":opportunities_a,"paired_ingest_opportunities":paired_ingest_opportunities,
        "a_contact_ingest_opportunities":contact_opportunities_a,"paired_contact_ingest_opportunities":paired_contact_opportunities,
        "requested_sleep_tick":request_tick,"sleep_consolidations":consolidation_count,"post_sleep_decisions":post_sleep_decisions,
        "final_body":record.biochemistry().body,"final_homeostasis":record.biochemistry().homeostasis,
        "final_position":host.world().entity(record.world_entity_id()).unwrap().position,"ecology":host.world().ecology_metrics(),
        "anchor_assay":anchor_assay,"optimizer_exact":true,"cold_restore":cold,
        "diagnostic_uncompressed_sha256":diagnostic_digest,"elapsed_seconds":started.elapsed().as_secs_f64(),
        "learning_claim":"None automatically. Exposure, persistent physiology, native selections and detached bank effects require joint interpretation.",
        "reversal_tested":false,"unrelated_acquired_retention_tested":false});
    write_json(&root.join(format!("{prefix}.summary.json")), &summary)?;
    eprintln!(
        "{} {} complete: steps={} meals={:?} stop={}",
        arm.name, scene, steps, counts, stopped
    );
    Ok(summary)
}
const CONSTRUCTION_SEED: u64 = 2026110901;
const TRANSFER_ASSETS: [&str; 2] = [
    "ba006d31a7ceb1f75d22704893494871652b6bc0200ef1a0f31039bab11b1e6a",
    "6edcd6dd1dc5bfedf8f7b73bf605bb480e36c0ba022a316840a6f0bd87629632",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Episode {
    seed: u64,
    mirror_z: bool,
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransferPlan {
    construction_seed: u64,
    episodes: Vec<Episode>,
    asset_sha256: [String; 2],
    max_lives: usize,
}
impl TransferPlan {
    fn fixed() -> Self {
        Self {
            construction_seed: CONSTRUCTION_SEED,
            episodes: (0..4)
                .map(|i| Episode {
                    seed: 2026111201 + i,
                    mirror_z: i % 2 == 1,
                })
                .collect(),
            asset_sha256: TRANSFER_ASSETS.map(str::to_owned),
            max_lives: 8,
        }
    }
    fn validate(&self) -> Result<()> {
        if *self != Self::fixed() {
            return Err("undeclared transfer assignment or asset".into());
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransferManifest {
    schema: String,
    source_commit: String,
    executable_sha256: String,
    retrieval_source_sha256: String,
    plan: TransferPlan,
    spec: Spec,
}
fn episode_position(scene: &str, target_index: usize, episode: Option<Episode>) -> Vec3f {
    if scene == "learning" {
        let z = if target_index == 0 { -0.35 } else { 0.35 };
        Vec3f::new(
            0.35,
            0.,
            if episode.is_some_and(|e| e.mirror_z) {
                -z
            } else {
                z
            },
        )
    } else {
        Vec3f::new(if scene == "far" { 2.5 } else { 0.25 }, 0., 0.)
    }
}
// World IDs and perceptual tracked IDs are distinct namespaces. Bind them only
// through the actual consumed command and its sealed episodic key.
fn consumed_phase_matches(decision: &Value, records: &Value, world_target: u64) -> bool {
    if decision["selected_action"]["target_entity"].as_u64() != Some(world_target)
        || decision["selected_action"]["action_id"].as_u64() != Some(210)
    {
        return false;
    }
    let Some(tracked) = decision["episodic_key"]["query"]["tracked_object_id"]
        .as_u64()
        .filter(|x| *x != 0)
    else {
        return false;
    };
    let Some(sequence) = decision["sequence_id"].as_u64().filter(|x| *x != 0) else {
        return false;
    };
    records.as_array().is_some_and(|records| {
        records.iter().any(|r| {
            r["tracked_object_id_raw"].as_u64() == Some(tracked)
                && r["sealed_sequence_id_raw"].as_u64() == Some(sequence)
                && r["family_raw"].as_u64() == Some(6)
                && r["action_id_raw"].as_u64() == Some(210)
                && r["outcome_contact"].as_str() == Some("Consumed")
        })
    })
}
fn prepare_transfer(root: &Path, commit: &str) -> Result<()> {
    if commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("full integration commit required".into());
    }
    let plan = TransferPlan::fixed();
    let v6 = candidate(CONSTRUCTION_SEED, 1.)?.encode_canonical()?;
    if digest(&v6) != "94a70c709661d883d6f146dec773fe120f5f53a8e25949c8c38cf232d0d6b11f" {
        return Err("archived v6 byte identity failed".into());
    }
    let mut frozen = Vec::new();
    for (i, (_, gain, reader, cap, family)) in arm_specs("family-value")?.into_iter().enumerate() {
        let bytes = candidate_with_readouts(CONSTRUCTION_SEED, gain, reader, cap, family)?
            .encode_canonical()?;
        if digest(&bytes) != TRANSFER_ASSETS[i] {
            return Err("frozen transfer genetic bytes changed".into());
        }
        frozen.push(bytes);
    }
    fs::create_dir(root)?;
    let exe = digest(&fs::read(std::env::current_exe()?)?);
    for episode in &plan.episodes {
        let dir = root.join(format!("episode-{}", episode.seed));
        fs::create_dir(&dir)?;
        let mut arms = Vec::new();
        for (i, (name, gain, reader, cap, family)) in
            arm_specs("family-value")?.into_iter().enumerate()
        {
            let filename = format!("{name}.alife-foundation");
            create(&dir.join(&filename))?.write_all(&frozen[i])?;
            arms.push(Arm {
                name: name.into(),
                action_gain: gain,
                memory_reader: reader,
                memory_gain: cap,
                family_valence_reader: family,
                filename,
                sha256: TRANSFER_ASSETS[i].into(),
            });
        }
        write_json(
            &dir.join("manifest.json"),
            &Manifest {
                schema: "fresh-native-learning-pilot-v2".into(),
                source_commit: commit.into(),
                executable_sha256: exe.clone(),
                archived_v6_sha256: digest(&v6),
                experiment: "family-value".into(),
                retrieval_source_sha256: retrieval_source_sha256(),
                spec: Spec::predeclared(CONSTRUCTION_SEED),
                arms,
            },
        )?;
    }
    write_json(
        &root.join("manifest.json"),
        &TransferManifest {
            schema: "fresh-native-transfer-v1".into(),
            source_commit: commit.into(),
            executable_sha256: exe,
            retrieval_source_sha256: retrieval_source_sha256(),
            plan,
            spec: Spec::predeclared(CONSTRUCTION_SEED),
        },
    )
}
fn transfer_one(root: &Path, seed: u64, arm_name: &str) -> Result<()> {
    let manifest: TransferManifest =
        serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
    manifest.plan.validate()?;
    if manifest.schema != "fresh-native-transfer-v1"
        || manifest.spec != Spec::predeclared(CONSTRUCTION_SEED)
        || manifest.executable_sha256 != digest(&fs::read(std::env::current_exe()?)?)
        || manifest.retrieval_source_sha256 != retrieval_source_sha256()
    {
        return Err("transfer source, executable or budget mismatch".into());
    }
    let episode = *manifest
        .plan
        .episodes
        .iter()
        .find(|e| e.seed == seed)
        .ok_or("seed is outside the fixed transfer matrix")?;
    let dir = root.join(format!("episode-{seed}"));
    let life_manifest = load_manifest(&dir)?;
    let arm = life_manifest
        .arms
        .iter()
        .find(|a| a.name == arm_name)
        .ok_or("arm is outside the fixed transfer matrix")?;
    let index = usize::from(arm.family_valence_reader);
    if arm.sha256 != TRANSFER_ASSETS[index] || life_manifest.source_commit != manifest.source_commit
    {
        return Err("transfer asset or source binding mismatch".into());
    }
    // A started life cannot be retried, even if its summary never completed.
    write_json(
        &root.join(format!("started-{seed}-{arm_name}.json")),
        &json!({"episode":episode,"arm":arm_name,"replacement_forbidden":true}),
    )?;
    let result = run(&dir, &life_manifest, arm, "learning", Some(episode))?;
    if result["stop_reason"]
        .as_str()
        .is_none_or(|s| s.starts_with("host_error"))
    {
        return Err("native life failed; preserve partial evidence and do not replace it".into());
    }
    let mut all = Vec::new();
    for e in &manifest.plan.episodes {
        for (name, _, _, _, _) in arm_specs("family-value")? {
            let path = root.join(format!("episode-{}/{}.learning.summary.json", e.seed, name));
            if path.is_file() {
                all.push(serde_json::from_slice::<Value>(&fs::read(path)?)?);
            }
        }
    }
    if all.len() == 8 {
        write_json(
            &root.join("transfer.complete.json"),
            &json!({"complete":true,"results":all}),
        )?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let mode = args.get(1).ok_or("mode: prepare, baseline or pilot")?;
    let root = Path::new(args.get(2).ok_or("fresh output directory required")?);
    if mode == "prepare-transfer" {
        return prepare_transfer(
            root,
            args.get(3).ok_or("integration source commit required")?,
        );
    }
    if mode == "transfer-one" {
        return transfer_one(
            root,
            args.get(3).ok_or("episode seed required")?.parse()?,
            args.get(4).ok_or("arm required")?,
        );
    }
    if matches!(
        mode.as_str(),
        "prepare" | "prepare-memory" | "prepare-family"
    ) {
        return prepare(
            root,
            args.get(3).ok_or("seed required")?.parse()?,
            args.get(4).ok_or("source commit required")?,
            Path::new(args.get(5).ok_or("archived v6 candidate required")?),
            match mode.as_str() {
                "prepare-memory" => "memory",
                "prepare-family" => "family-value",
                _ => "original",
            },
        );
    }
    let manifest = load_manifest(root)?;
    let scenes = match mode.as_str() {
        "baseline" => vec!["positive", "bitter", "far"],
        "pilot" => vec!["learning"],
        _ => return Err("unknown mode".into()),
    };
    if mode == "pilot" && !root.join("baseline.complete.json").is_file() {
        return Err("complete and inspect the fixed baseline before the pilot".into());
    }
    let mut results = Vec::new();
    for arm in &manifest.arms {
        for scene in &scenes {
            results.push(run(root, &manifest, arm, scene, None)?);
        }
    }
    write_json(
        &root.join(format!("{mode}.complete.json")),
        &json!({"results":results,"automatic_improvement_claim":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quarter_gain_changes_only_action_candidate_genes() {
        let seed = 2026110901;
        let (_, p, _) = care_candidate::source(seed, true).unwrap();
        let original = candidate(seed, 1.).unwrap();
        let quarter = candidate(seed, 0.25).unwrap();
        let mut changed = 0;
        for (s, (a, b)) in p
            .synapses()
            .iter()
            .zip(original.weights().iter().zip(quarter.weights()))
        {
            if matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate)
            {
                assert_eq!((a * 0.25).to_bits(), b.to_bits());
                changed += usize::from(a.to_bits() != b.to_bits());
            } else {
                assert_eq!(a.to_bits(), b.to_bits());
            }
            if *a != 0. {
                assert_eq!(a.is_sign_negative(), b.is_sign_negative());
            }
        }
        assert!(changed > 0);
        assert_eq!(
            FoundationWeightAsset::decode_canonical(&quarter.encode_canonical().unwrap()).unwrap(),
            quarter
        );
    }
    #[test]
    fn memory_factor_changes_only_declared_coefficients_and_plan() {
        let seed = 2026110901;
        let (_, old, _) = care_candidate::source(seed, true).unwrap();
        let (_, new, _) = care_candidate::source_memory_gain(seed).unwrap();
        assert_eq!(old.synapses(), new.synapses());
        assert_eq!(old.plasticity_receptors(), new.plasticity_receptors());
        for reader in [false, true] {
            let small = candidate_with_memory(seed, 0.25, reader, 0.5).unwrap();
            let large = candidate_with_memory(seed, 0.25, reader, 12.).unwrap();
            assert_eq!(small.weights(), large.weights());
            let off = candidate(seed, 0.25).unwrap();
            for (i, synapse) in old.synapses().iter().enumerate() {
                match synapse.kind() {
                    CompiledSynapseKind::Decoder(c)
                        if c.head() == DecoderHeadKind::MemoryContext =>
                    {
                        if !reader {
                            assert_eq!(small.weights()[i], 0.);
                        }
                    }
                    _ => assert_eq!(small.weights()[i].to_bits(), off.weights()[i].to_bits()),
                }
            }
        }
        let reader = candidate_with_memory(seed, 0.25, true, 12.).unwrap();
        for (family, lane, total) in [
            (CandidateActionFamily::Ingest, 25, -16.),
            (CandidateActionFamily::Ingest, 26, -64.),
            (CandidateActionFamily::Approach, 25, -8.),
            (CandidateActionFamily::Approach, 26, -32.),
            (CandidateActionFamily::Contact, 25, -8.),
            (CandidateActionFamily::Contact, 26, -32.),
            (CandidateActionFamily::Avoid, 25, 16.),
            (CandidateActionFamily::Avoid, 26, 128.),
        ] {
            let sum:f32=old.synapses().iter().zip(reader.weights()).filter_map(|(s,w)|
                matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::MemoryContext
                    && c.family()==family && c.input_lane()==lane).then_some(*w)).sum();
            assert!((sum - total).abs() < 0.0001);
        }
    }
    #[test]
    fn family_value_factor_changes_only_ingest_valence_coordinates() {
        let seed = 2026110901;
        let (_, p, _) = care_candidate::source_memory_gain(seed).unwrap();
        let off = candidate_with_memory(seed, 0.25, true, 12.).unwrap();
        let on = candidate_with_readouts(seed, 0.25, true, 12., true).unwrap();
        let mut count = 0;
        let mut sum = 0.;
        for (i, synapse) in p.synapses().iter().enumerate() {
            if matches!(synapse.kind(), CompiledSynapseKind::Decoder(c)
                if c.head() == DecoderHeadKind::MemoryContext
                    && c.family() == CandidateActionFamily::Ingest && c.input_lane() == 32)
            {
                assert_eq!(off.weights()[i], 0.);
                assert!(on.weights()[i] > 0.);
                count += 1;
                sum += on.weights()[i];
            } else {
                assert_eq!(off.weights()[i].to_bits(), on.weights()[i].to_bits());
            }
        }
        assert_eq!(count, 40);
        assert!((sum - 32.).abs() < 0.0001);
        assert_eq!(
            FoundationWeightAsset::decode_canonical(&on.encode_canonical().unwrap()).unwrap(),
            on
        );
        assert!(candidate_with_readouts(seed, 0.25, false, 12., true).is_err());
        assert!(candidate_with_readouts(seed, 0.25, true, 0.5, true).is_err());
        assert_eq!(arm_specs("family-value").unwrap().len(), 2);
        assert!(arm_specs("undeclared").is_err());
        assert_eq!(retrieval_source_sha256().len(), 64);
    }
    #[test]
    fn predeclared_budget_is_bounded_and_sleep_is_exposure_triggered() {
        let spec = Spec::predeclared(2026110901);
        assert!(spec.life_ticks <= 256 && spec.max_a_consumptions <= 3);
        assert_eq!(spec.regrow_ticks, 16);
        assert!(spec.sleep_policy.contains("regardless"));
    }

    #[test]
    fn transfer_default_layout_and_mirror_change_only_z() {
        let normal = Some(Episode {
            seed: CONSTRUCTION_SEED,
            mirror_z: false,
        });
        let mirror = Some(Episode {
            seed: 2026111202,
            mirror_z: true,
        });
        for scene in ["learning", "positive", "bitter", "far"] {
            for i in 0..2 {
                let default = episode_position(scene, i, None);
                assert_eq!(default, episode_position(scene, i, normal));
                let changed = episode_position(scene, i, mirror);
                assert_eq!((default.x, default.y), (changed.x, changed.y));
                assert_eq!(
                    changed.z,
                    if scene == "learning" {
                        -default.z
                    } else {
                        default.z
                    }
                );
            }
        }
        assert!(episode_position("learning", 0, normal).z < 0.);
        assert!(episode_position("learning", 0, mirror).z > 0.);
        assert_eq!(Spec::predeclared(CONSTRUCTION_SEED).max_a_consumptions, 3);
    }
    #[test]
    fn transfer_plan_rejects_replacement_seeds_or_genes() {
        TransferPlan::fixed().validate().unwrap();
        let mut changed = TransferPlan::fixed();
        changed.episodes[1] = changed.episodes[0];
        assert!(changed.validate().is_err());
        let mut changed = TransferPlan::fixed();
        changed.asset_sha256[0] = "other".into();
        assert!(changed.validate().is_err());
        assert!(serde_json::from_value::<Episode>(
            json!({"seed":2026111201_u64,"mirror_z":false,"hazard":0})
        )
        .is_err());
    }
    #[test]
    fn transfer_consumption_binds_world_id_to_sealed_tracked_id() {
        let decision = json!({"sequence_id":17,"selected_action":{"target_entity":2,"action_id":210},"episodic_key":{"query":{"tracked_object_id":999}}});
        let records = json!([{"sealed_sequence_id_raw":17,"tracked_object_id_raw":999,"family_raw":6,"action_id_raw":210,"outcome_contact":"Consumed"}]);
        assert!(consumed_phase_matches(&decision, &records, 2));
        assert!(!consumed_phase_matches(&decision, &records, 999));
        let preparation = json!([{"sealed_sequence_id_raw":17,"tracked_object_id_raw":999,"family_raw":6,"action_id_raw":210,"outcome_contact":"Touch"}]);
        assert!(!consumed_phase_matches(&decision, &preparation, 2));
        assert!(!consumed_phase_matches(&decision, &json!([]), 2));
        let mut stale = records.clone();
        stale[0]["sealed_sequence_id_raw"] = json!(16);
        assert!(!consumed_phase_matches(&decision, &stale, 2));
        let mut missing = records.clone();
        missing[0]
            .as_object_mut()
            .unwrap()
            .remove("sealed_sequence_id_raw");
        assert!(!consumed_phase_matches(&decision, &missing, 2));
        let mut no_sequence = decision.clone();
        no_sequence.as_object_mut().unwrap().remove("sequence_id");
        assert!(!consumed_phase_matches(&no_sequence, &records, 2));
        no_sequence["sequence_id"] = json!(0);
        assert!(!consumed_phase_matches(&no_sequence, &records, 2));
    }
    #[test]
    fn transfer_fixed_assets_admit_all_declared_bodies_without_gene_changes() {
        for (i, (_, gain, reader, cap, family)) in
            arm_specs("family-value").unwrap().into_iter().enumerate()
        {
            let asset =
                candidate_with_readouts(CONSTRUCTION_SEED, gain, reader, cap, family).unwrap();
            let bytes = asset.encode_canonical().unwrap();
            assert_eq!(digest(&bytes), TRANSFER_ASSETS[i]);
            for episode in TransferPlan::fixed().episodes {
                let (config, _, _) = care_candidate::source(episode.seed, true).unwrap();
                let game =
                    create_canonical_new_game_with_n2048_memory_gain_candidate(&config, &asset)
                        .expect(
                            "fixed genetic asset must admit held-out body; do not regenerate genes",
                        );
                let record = game.world.organism_registry().iter().next().unwrap();
                let development = foundation_construction_development(
                    &record.phenotype().brain_genome,
                    &BrainCapacityClass::n2048(),
                    &record.phenotype().development_state_at(Tick::ZERO).unwrap(),
                )
                .unwrap();
                let (compiled, _) =
                    PhenotypeCompiler::compile_n2048_memory_gain_foundation_candidate(
                        record.phenotype().brain_genome.clone(),
                        development,
                        asset.clone(),
                    )
                    .unwrap();
                assert_eq!(compiled.synapses().len(), asset.weights().len());
                assert_eq!(asset.encode_canonical().unwrap(), bytes);
            }
        }
    }
}

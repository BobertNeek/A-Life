//! Disposable exploratory direct-weight probe. CPU only; never reads a life.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask, TrainingSequence};
use alife_world::*;
use serde_json::json;
use std::error::Error;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[path = "support/care_candidate.rs"]
mod care_candidate;
use care_candidate::{source, whole_care, whole_care_with_contact};

struct TrialSpec<'a> {
    seed: u64,
    arm: &'a str,
    gain: f32,
    scene: &'a str,
    ticks: usize,
    receipt_root: &'a str,
    write_asset: bool,
    initial_energy: f32,
    preparation_ticks: usize,
    target_hazard_pain: f32,
}
fn trial(spec: TrialSpec<'_>) -> Result<serde_json::Value> {
    let TrialSpec {
        seed,
        arm,
        gain,
        scene,
        ticks,
        receipt_root,
        write_asset,
        initial_energy,
        preparation_ticks,
        target_hazard_pain,
    } = spec;
    let experimental = arm == "zero-excitatory" || arm.starts_with("whole-care");
    let (config, p, source) = source(seed, experimental)?;
    let mut weights = p
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    let mut edits = Vec::new();
    if arm == "zero-excitatory" {
        weights.fill(0.);
        for (i, s) in p.synapses().iter().enumerate() {
            if s.kind() == CompiledSynapseKind::Recurrent
                && p.projections()[s.route_index() as usize].projection_type()
                    == ProjectionType::LateralInhibition
            {
                weights[i] = -0.03;
            }
        }
    } else if arm.starts_with("whole-care") {
        let (whole, circuits) = if arm == "whole-care-v5" {
            whole_care_with_contact(&p, -5.)?
        } else {
            whole_care(&p)?
        };
        weights = whole;
        edits = circuits;
        if arm == "whole-care-no-need" || arm == "whole-care-no-tonic" {
            let role = if arm.ends_with("no-need") {
                "need"
            } else {
                "tonic"
            };
            for circuit in &edits {
                if circuit["role"] == role {
                    for i in circuit["bridge_synapses"].as_array().unwrap() {
                        weights[i.as_u64().unwrap() as usize] = 0.;
                    }
                }
            }
        }
        if arm == "whole-care-no-reference" {
            for circuit in &edits {
                if circuit["role"] == "reference" {
                    let reference = circuit["reference_motor"].as_u64().unwrap() as u32;
                    for (i, s) in p.synapses().iter().enumerate() {
                        if matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate)
                            && s.source() == reference
                        {
                            weights[i] = 0.;
                        }
                    }
                }
            }
        }
    } else if gain != 0. {
        for (i, s) in p.synapses().iter().enumerate() {
            if matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Ingest && c.input_lane()==15)
            {
                weights[i] += gain;
                edits.push(json!({"index":i,"source":s.source(),"before":s.genetic_weight(),"after":weights[i]}));
            }
        }
    }
    let asset = if arm == "untouched" {
        source.clone()
    } else {
        FoundationWeightAsset::from_trained_weights(
            &p,
            weights,
            source.manifest().training_stage(),
        )?
    };
    if write_asset {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(format!("{receipt_root}/{seed}.{arm}.alife-foundation"))?;
        file.write_all(&asset.encode_canonical()?)?;
        file.sync_all()?;
    }
    let candidate_digest = asset.digest();
    let changed_weights = p
        .synapses()
        .iter()
        .zip(asset.weights())
        .filter(|(s, w)| s.genetic_weight().to_bits() != w.to_bits())
        .count();
    let mut game = if experimental {
        create_canonical_new_game_with_n2048_experimental_candidate(&config, &asset)?
    } else {
        create_canonical_new_game_with_n2048_candidate(&config, &asset)?
    };
    let old = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let cap = BrainCapacityClass::n2048();
    let development = foundation_construction_development(
        &old.phenotype().brain_genome,
        &cap,
        &old.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    let (p, _) = if experimental {
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            old.phenotype().brain_genome.clone(),
            development,
            asset.clone(),
        )?
    } else {
        PhenotypeCompiler::compile_n2048_foundation_candidate(
            old.phenotype().brain_genome.clone(),
            development,
            asset.clone(),
        )?
    };
    let mask = StageTrainableMask::recurrent_only(&p)?;
    let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default())?;
    let before = serde_json::to_vec(&trainer.checkpoint())?;
    let own = old.organism_id();
    let distance = if scene.starts_with("far-") || scene == "obstacle-food" {
        2.5
    } else {
        0.25
    };
    let target_position = if scene.ends_with("-left") {
        Vec3f::new(0., 0., distance)
    } else if scene.ends_with("-back") {
        Vec3f::new(-distance, 0., 0.)
    } else {
        Vec3f::new(distance, 0., 0.)
    };
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", own, Vec3f::ZERO)
        .build()?;
    if scene == "obstacle-food" {
        world.editor_spawn_object(WorldEditorSpawnSpec {
            label: "obstacle".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(1.25, 0., 0.),
            nutrition: 0.,
            hazard_pain: 0.,
            radius: 0.35,
            token_id: None,
        })?;
    }
    let object = if scene == "toy" {
        world.spawn_toy("target", target_position, true)?
    } else {
        world.editor_spawn_object(WorldEditorSpawnSpec {
            label: "target".into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: target_position,
            nutrition: 0.6,
            hazard_pain: target_hazard_pain,
            radius: 0.5,
            token_id: None,
        })?
    };
    let mut physical = world.entity(object).unwrap().grounded_physical;
    physical.chemical[0] = if scene.starts_with("bitter-food") {
        -0.85
    } else if scene == "toy" {
        0.
    } else {
        0.85
    };
    world.set_grounded_physical_properties(object, physical)?;
    let mut chemistry = *old.biochemistry();
    chemistry.body.set_energy(initial_energy)?;
    world.register_organism_record(WorldOrganismRecord::new(
        own,
        world.entity_id("learner").unwrap(),
        old.genome().clone(),
        old.phenotype().clone(),
        chemistry,
        old.birth_tick(),
    )?)?;
    game.world = world;
    let sampling_seed = (seed ^ (seed >> 32)) as u32;
    let mut host = if preparation_ticks == 0 {
        CpuCanonicalHost::new_cohort(game, &trainer, sampling_seed)?
    } else {
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, sampling_seed, |world| {
            for _ in 0..preparation_ticks {
                world.try_advance_tick()?;
            }
            Ok::<_, ScaffoldContractError>(())
        })?
        .0
    };
    let initial_biology = *host
        .world()
        .organism_registry()
        .get(own)
        .unwrap()
        .biochemistry();
    let mut rows = Vec::new();
    for tick in 0..ticks {
        let step = host.step(&trainer, 1.)?;
        for row in step.decisions {
            let eval = trainer.evaluate_replay(&TrainingSequence {
                phenotype_hash: trainer.phenotype().phenotype_hash(),
                initial: row.initial.clone(),
                ticks: vec![row.replay_tick.clone()],
                burn_in_ticks: 0,
                memory_candidate_gain: trainer
                    .phenotype()
                    .candidate_decoder()
                    .memory_channel()
                    .map_or(0., |m| m.max_candidate_gain()),
            })?;
            let frame = row.patch.pre_action().perception();
            if tick == 0 && scene != "toy" {
                let expected = if scene.starts_with("bitter-food") {
                    -0.85
                } else {
                    0.85
                };
                assert!(frame
                    .candidates()
                    .iter()
                    .filter(|c| c.target.entity == Some(object))
                    .all(|c| (c.features.0[15] - expected).abs() < 1e-5));
            }
            let motors=trainer.phenotype().synapses().iter().filter_map(|s|match s.kind() {CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Ingest && c.input_lane()==15=>Some(json!({"source":s.source(),"activation":eval.final_activations[0][s.source() as usize],"weight":s.genetic_weight()})),_=>None}).collect::<Vec<_>>();
            // Detached neural-input sensitivity, not a world action or a learning update.
            // Innate biases stay fixed so this isolates the encoded recurrent path.
            let hunger_probe = if tick == 0 {
                let mut changed = row.replay_tick.clone();
                for a in trainer
                    .phenotype()
                    .sensor_encoder()
                    .assignments()
                    .iter()
                    .filter(|a| {
                        a.source_group() == SensorEncoderSourceGroup::Homeostasis
                            && a.source_index() == 0
                    })
                {
                    changed.encoded_inputs[a.target_neuron() as usize] += 0.1;
                }
                let response = trainer.evaluate_replay(&TrainingSequence {
                    phenotype_hash: trainer.phenotype().phenotype_hash(),
                    initial: row.initial.clone(),
                    ticks: vec![changed],
                    burn_in_ticks: 0,
                    memory_candidate_gain: trainer
                        .phenotype()
                        .candidate_decoder()
                        .memory_channel()
                        .map_or(0., |m| m.max_candidate_gain()),
                })?;
                Some(
                    frame
                        .candidates()
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c.family == CandidateActionFamily::Ingest)
                        .map(|(i, _)| response.candidate_logits[0][i] - eval.candidate_logits[0][i])
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };
            let mut debug_neurons = std::collections::BTreeSet::new();
            let mut bridge_ids = std::collections::BTreeSet::new();
            for circuit in &edits {
                for key in ["source_neuron", "motor_neuron", "reference_motor"] {
                    if let Some(n) = circuit[key].as_u64() {
                        debug_neurons.insert(n as u32);
                    }
                }
                if let Some(indices) = circuit["bridge_synapses"].as_array() {
                    for i in indices {
                        let i = i.as_u64().unwrap() as usize;
                        bridge_ids.insert(i);
                        debug_neurons.insert(trainer.phenotype().synapses()[i].target());
                    }
                }
            }
            let debug_activations = debug_neurons
                .into_iter()
                .map(|n| (n, eval.final_activations[0][n as usize]))
                .collect::<std::collections::BTreeMap<_, _>>();
            let effective_bridges=bridge_ids.into_iter().map(|i|json!({"index":i,"genetic":trainer.phenotype().synapses()[i].genetic_weight(),"offset":row.replay_tick.effective_weight_offsets[i],"effective":trainer.phenotype().synapses()[i].genetic_weight()+row.replay_tick.effective_weight_offsets[i]})).collect::<Vec<_>>();
            let hunger_three_frames = if tick == 0 {
                let frozen = vec![row.replay_tick.clone(); 3];
                let mut changed = frozen.clone();
                for t in &mut changed {
                    for a in trainer
                        .phenotype()
                        .sensor_encoder()
                        .assignments()
                        .iter()
                        .filter(|a| {
                            a.source_group() == SensorEncoderSourceGroup::Homeostasis
                                && a.source_index() == 0
                        })
                    {
                        t.encoded_inputs[a.target_neuron() as usize] += 0.1;
                    }
                }
                let sequence = |ticks| TrainingSequence {
                    phenotype_hash: trainer.phenotype().phenotype_hash(),
                    initial: row.initial.clone(),
                    ticks,
                    burn_in_ticks: 0,
                    memory_candidate_gain: trainer
                        .phenotype()
                        .candidate_decoder()
                        .memory_channel()
                        .map_or(0., |m| m.max_candidate_gain()),
                };
                let baseline = trainer.evaluate_replay(&sequence(frozen))?;
                let response = trainer.evaluate_replay(&sequence(changed))?;
                Some(
                    frame
                        .candidates()
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c.family == CandidateActionFamily::Ingest)
                        .map(|(i, _)| {
                            response.candidate_logits[2][i] - baseline.candidate_logits[2][i]
                        })
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };
            let outcome = row.patch.outcome();
            rows.push(json!({"tick":tick,"counter":row.sampling_counter,"action":row.action,"candidates":frame.candidates(),"logits":eval.candidate_logits[0],"ingest_taste_motors":motors,"encoded_hunger_plus_0_1_logit_delta":hunger_probe,"encoded_hunger_3_frozen_frames_delta":hunger_three_frames,"constructed_neuron_activations":debug_activations,"effective_bridges":effective_bridges,"enabled_routes":row.replay_tick.enabled_routes,"outcome":outcome,"reward":row.biological_reward}));
        }
    }
    assert_eq!(before, serde_json::to_vec(&trainer.checkpoint())?);
    let ck = host.checkpoint(&trainer)?;
    let bytes = serde_json::to_vec(&ck)?;
    let mut cold = CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes)?, &trainer)?;
    assert_eq!(bytes, serde_json::to_vec(&cold.checkpoint(&trainer)?)?);
    let hot = host.snapshot().step(&trainer, 1.)?;
    let fresh = cold.step(&trainer, 1.)?;
    assert_eq!(
        serde_json::to_vec(&hot.decisions)?,
        serde_json::to_vec(&fresh.decisions)?
    );
    let mut hot_host = host.snapshot();
    hot_host.step(&trainer, 1.)?;
    assert_eq!(
        serde_json::to_vec(&hot_host.checkpoint(&trainer)?)?,
        serde_json::to_vec(&cold.checkpoint(&trainer)?)?
    );
    let r = host.world().organism_registry().get(own).unwrap();
    Ok(
        json!({"seed":seed,"arm":arm,"scene":scene,"edits":edits,"source_digest":source.digest(),"candidate_digest":candidate_digest,"changed_weights":changed_weights,"experimental_graph":experimental,"active_tiles":trainer.phenotype().budgets().global.active_tiles,"microstep_count":trainer.phenotype().microstep_count(),"total_synapses":trainer.phenotype().synapses().len(),"preparation_ticks":preparation_ticks,"target_hazard_pain":target_hazard_pain,"initial_biology":initial_biology,"optimizer_unchanged":true,"cold_next_step_exact":true,"target_active":!host.world().entity(object).unwrap().consumed,"final_position":host.world().entity(host.world().entity_id("learner").unwrap()).unwrap().position,"final_target_carrier":host.world().entity(object).unwrap().carried_by,"final_body":r.biochemistry().body,"final_homeostasis":r.biochemistry().homeostasis,"rows":rows}),
    )
}
fn graph_report(p: &BrainPhenotype) -> serde_json::Value {
    use std::collections::{BTreeSet, VecDeque};
    let mut rows = Vec::new();
    for a in p.sensor_encoder().assignments() {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::new();
        seen.insert(a.target_neuron());
        queue.push_back((a.target_neuron(), 0usize));
        let mut family_depth = std::collections::BTreeMap::new();
        while let Some((n, depth)) = queue.pop_front() {
            for s in p.synapses().iter().filter(|s| s.source() == n) {
                match s.kind() {
                    CompiledSynapseKind::Recurrent => {
                        if seen.insert(s.target()) {
                            queue.push_back((s.target(), depth + 1));
                        }
                    }
                    CompiledSynapseKind::Decoder(c)
                        if c.head() == DecoderHeadKind::ActionCandidate =>
                    {
                        family_depth
                            .entry(format!("{:?}", c.family()))
                            .or_insert(depth);
                    }
                    _ => {}
                }
            }
        }
        rows.push(json!({"group":a.source_group(),"lane":a.source_index(),"neuron":a.target_neuron(),"outgoing_recurrent":p.synapses().iter().filter(|s|s.source()==a.target_neuron() && s.kind()==CompiledSynapseKind::Recurrent).count(),"reachable_neurons":seen.len(),"shortest_recurrent_depth_to_family":family_depth}));
    }
    json!({"structural_reachability_only":true,"all_directed_hops":true,"sensor_assignments":rows})
}
fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let path = args.get(1).ok_or("output JSON required")?;
    if matches!(
        args.get(2).map(String::as_str),
        Some("graph" | "graph-experimental")
    ) {
        let (_, p, _) = source(2026110701, args[2] == "graph-experimental")?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        use std::io::Write;
        output.write_all(&serde_json::to_vec_pretty(&graph_report(&p))?)?;
        return Ok(());
    }
    let ticks: usize = args.get(2).map_or(Ok(12), |s| s.parse())?;
    let gain: f32 = args.get(3).map_or(Ok(-8.), |s| s.parse())?;
    let mode = args.get(4).map_or("surgery", String::as_str);
    let seeds: usize = args.get(5).map_or(Ok(8), |s| s.parse())?;
    let seed_start: u64 = args.get(6).map_or(Ok(2026110701), |s| s.parse())?;
    let scenes = args
        .get(7)
        .map_or("food,bitter-food,toy,far-food", String::as_str)
        .split(',')
        .collect::<Vec<_>>();
    let initial_energy: f32 = args.get(8).map_or(Ok(0.4), |s| s.parse())?;
    let preparation_ticks: usize = args.get(9).map_or(Ok(0), |s| s.parse())?;
    let target_hazard_pain: f32 = args.get(10).map_or(Ok(0.), |s| s.parse())?;
    if !target_hazard_pain.is_finite() || !(0.0..=1.).contains(&target_hazard_pain) {
        return Err("target hazard pain must be finite0..1".into());
    }
    if preparation_ticks > 1200 {
        return Err("preparation ticks must be0..1200".into());
    }
    if seed_start.checked_add(seeds as u64).is_none()
        || scenes.is_empty()
        || scenes.len() > 10
        || scenes.iter().any(|s| {
            ![
                "food",
                "bitter-food",
                "toy",
                "far-food",
                "food-left",
                "bitter-food-left",
                "food-back",
                "far-food-left",
                "far-food-back",
                "obstacle-food",
            ]
            .contains(s)
        })
        || !initial_energy.is_finite()
        || !(0.1..=1.).contains(&initial_energy)
    {
        return Err("invalid bounded seed, scene or energy".into());
    }
    if !(1..=120).contains(&ticks)
        || !(1..=32).contains(&seeds)
        || !gain.is_finite()
        || gain.abs() > 32.
    {
        return Err("bounded disposable probe: ticks1..120,seeds1..32,gain finite abs<=32".into());
    }
    let arms = match mode {
        "surgery" => vec![
            ("untouched", 0.),
            ("taste-readout", gain),
            ("wrong-sign", -gain),
        ],
        "validate" => vec![("untouched", 0.), ("whole-care", 0.)],
        "contact" => vec![("whole-care-v5", 0.), ("whole-care", 0.)],
        "whole" => vec![
            ("untouched", 0.),
            ("zero-excitatory", 0.),
            ("whole-care", 0.),
            ("whole-care-no-need", 0.),
            ("whole-care-no-tonic", 0.),
            ("whole-care-no-reference", 0.),
        ],
        _ => return Err("mode must be surgery, whole, validate or contact".into()),
    };
    // Reserve a new output directory before any simulation. Never overwrite a prior receipt.
    std::fs::create_dir(path)?;
    let mut trial_files = Vec::new();
    for seed in (seed_start..).take(seeds) {
        for (scene_index, &scene) in scenes.iter().enumerate() {
            for &(arm, gain) in &arms {
                let r = trial(TrialSpec {
                    seed,
                    arm,
                    gain,
                    scene,
                    ticks,
                    receipt_root: path,
                    write_asset: scene_index == 0,
                    initial_energy,
                    preparation_ticks,
                    target_hazard_pain,
                })?;
                use std::io::Write;
                let mut receipt = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(format!("{path}/{seed}.{scene}.{arm}.json"))?;
                receipt.write_all(&serde_json::to_vec(&r)?)?;
                receipt.sync_all()?;
                eprintln!(
                    "{seed} {scene} {arm} energy={} active={}",
                    r["final_body"]["energy"], r["target_active"]
                );
                trial_files.push(format!("{seed}.{scene}.{arm}.json"));
            }
        }
    }
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(format!("{path}/receipt.pending.json"))?;
    serde_json::to_writer(
        &mut output,
        &json!({"receipt_format":"journal-v2","complete":true,"backend":"native-cpu","experimental":true,"promoted":false,"initial_energy":initial_energy,"seed_start":seed_start,"seed_count":seeds,"arms":arms.iter().map(|(arm,_)|*arm).collect::<Vec<_>>(),"scenes":scenes,"preparation_ticks":preparation_ticks,"target_hazard_pain":target_hazard_pain,"newborn_homeostasis_unchanged":preparation_ticks==0,"gain":gain,"ticks":ticks,"mode":mode,"trial_files":trial_files}),
    )?;
    output.sync_all()?;
    // Publish only after every per-trial receipt and the compact index are durable.
    // hard_link fails rather than replacing an existing completed receipt.
    std::fs::hard_link(
        format!("{path}/receipt.pending.json"),
        format!("{path}/receipt.json"),
    )?;
    std::fs::remove_file(format!("{path}/receipt.pending.json"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_weight_builder_preserves_signs_and_separates_need_axes() {
        let (_, p, source) = source(2026110711, true).unwrap();
        let (weights, circuits) = whole_care(&p).unwrap();
        assert_eq!(weights.len(), 32768);
        let (old, _) = whole_care_with_contact(&p, -5.).unwrap();
        let changes = p
            .synapses()
            .iter()
            .zip(weights.iter().zip(&old))
            .filter(|(_, (a, b))| a.to_bits() != b.to_bits())
            .collect::<Vec<_>>();
        assert_eq!(changes.len(), 2);
        for (s, _) in changes {
            assert!(matches!(s.kind(), CompiledSynapseKind::Decoder(c)
                if c.head() == DecoderHeadKind::ActionCandidate
                && c.family() == CandidateActionFamily::Approach && c.input_lane() == 18));
        }
        for (s, w) in p.synapses().iter().zip(&weights) {
            assert!(w.is_finite());
            if s.kind() == CompiledSynapseKind::Recurrent {
                match p.projections()[s.route_index() as usize].projection_type() {
                    ProjectionType::LateralInhibition => assert!(*w < 0.),
                    ProjectionType::Homeostatic | ProjectionType::MotorProposal => {
                        assert!(*w >= 0.)
                    }
                    _ => {}
                }
            }
        }
        let mut axes = std::collections::BTreeMap::new();
        for c in circuits.iter().filter(|c| c["role"] == "need") {
            let lane = c["homeostatic_lane"].as_u64().unwrap();
            let indices = c["bridge_synapses"].as_array().unwrap();
            let reserve = p.synapses()[indices[0].as_u64().unwrap() as usize].target();
            let temporal = p.synapses()[indices[1].as_u64().unwrap() as usize].target();
            if let Some(old) = axes.insert(lane, (reserve, temporal)) {
                assert_eq!(old, (reserve, temporal));
            }
        }
        let values = axes.values().collect::<Vec<_>>();
        for (i, a) in values.iter().enumerate() {
            for b in &values[i + 1..] {
                assert_ne!(a.0, b.0);
                assert_ne!(a.1, b.1);
            }
        }
        FoundationWeightAsset::from_trained_weights(
            &p,
            weights,
            source.manifest().training_stage(),
        )
        .unwrap();
    }
}

//! Disposable exploratory direct-weight probe. CPU only; never reads a life.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask, TrainingSequence};
use alife_world::*;
use serde_json::json;
use std::error::Error;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn source(
    seed: u64,
    experimental: bool,
) -> Result<(
    CanonicalNewGameConfig,
    BrainPhenotype,
    FoundationWeightAsset,
)> {
    let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    let asset = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile)?;
    let game = create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
    let r = game.world.organism_registry().iter().next().unwrap();
    let capacity = BrainCapacityClass::n2048();
    let genome = r.phenotype().brain_genome.clone();
    let development = foundation_construction_development(
        &genome,
        &capacity,
        &r.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    if experimental {
        let (p, _) = PhenotypeCompiler::compile_n2048_experimental_graph(
            genome.clone(),
            development.clone(),
            config.sensor_profile,
        )?;
        let asset = FoundationWeightAsset::from_trained_weights(
            &p,
            p.synapses().iter().map(|s| s.genetic_weight()).collect(),
            TrainingStageManifest::bootstrap(),
        )?;
        let (p, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            genome,
            development,
            asset.clone(),
        )?;
        return Ok((config, p, asset));
    }
    let p = PhenotypeCompiler::compile(&genome, &capacity, &development, config.sensor_profile)?;
    let asset = FoundationWeightAsset::from_trained_weights(
        &p,
        p.synapses().iter().map(|s| s.genetic_weight()).collect(),
        asset.manifest().training_stage(),
    )?;
    let (p, _) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset.clone())?;
    Ok((config, p, asset))
}
// Whole-weight revision 4. Feedback-informed: explicit signed tonic/need circuits
// and a native reference motor population; never an external action policy.
fn whole_care(p: &BrainPhenotype) -> Result<(Vec<f32>, Vec<serde_json::Value>)> {
    use std::collections::BTreeSet;
    let mut w = vec![0.; p.synapses().len()];
    let mut circuits = Vec::new();
    let mut incoming = vec![0usize; p.neuron_count() as usize];
    let mut outgoing = vec![Vec::new(); p.neuron_count() as usize];
    for (i, s) in p.synapses().iter().enumerate() {
        if s.kind() == CompiledSynapseKind::Recurrent {
            incoming[s.target() as usize] += 1;
            outgoing[s.source() as usize].push(i);
        }
    }
    for (i, s) in p.synapses().iter().enumerate() {
        if s.kind() != CompiledSynapseKind::Recurrent {
            continue;
        }
        let route = &p.projections()[s.route_index() as usize];
        if route.projection_type() == ProjectionType::LateralInhibition {
            w[i] = -0.03;
        } else if [
            LobeKind::WorkingContextExecutive,
            LobeKind::MemoryInterface,
            LobeKind::MultimodalAssociation,
        ]
        .contains(&route.source_lobe())
            || [
                LobeKind::WorkingContextExecutive,
                LobeKind::MemoryInterface,
                LobeKind::MultimodalAssociation,
            ]
            .contains(&route.target_lobe())
        {
            w[i] = 0.12 / incoming[s.target() as usize] as f32;
        }
    }
    let intero = p
        .lobe_layout()
        .region(LobeKind::InteroceptiveMotivational)
        .unwrap();
    let reserve = p.lobe_layout().region(LobeKind::FlexibleReserve).unwrap();
    let tonic_source = (intero.start..intero.start + intero.len)
        .rev()
        .find(|n| {
            !p.sensor_encoder()
                .assignments()
                .iter()
                .any(|a| a.target_neuron() == *n)
        })
        .ok_or("no unsensed tonic neuron")?;
    let tonic_reserve = reserve.start + 15;
    let tonic_first = *outgoing[tonic_source as usize]
        .iter()
        .find(|&&i| p.synapses()[i].route_index() == 2 && p.synapses()[i].target() == tonic_reserve)
        .ok_or("tonic input route absent")?;
    let mut family_motors = std::collections::BTreeMap::new();
    for (family, lanes) in [
        (CandidateActionFamily::Ingest, vec![0]),
        (CandidateActionFamily::Approach, vec![0]),
        (CandidateActionFamily::Rest, vec![1]),
        (CandidateActionFamily::Avoid, vec![3, 2]),
        (CandidateActionFamily::Inspect, vec![5]),
        (CandidateActionFamily::Contact, vec![5]),
    ] {
        let motors = p
            .synapses()
            .iter()
            .filter_map(|s| match s.kind() {
                CompiledSynapseKind::Decoder(c)
                    if c.head() == DecoderHeadKind::ActionCandidate && c.family() == family =>
                {
                    Some(s.source())
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let reference = *motors.last().unwrap();
        let mut selected = BTreeSet::new();
        for lane in lanes {
            let input = p
                .sensor_encoder()
                .assignments()
                .iter()
                .find(|a| {
                    a.source_group() == SensorEncoderSourceGroup::Homeostasis
                        && a.source_index() == lane
                })
                .ok_or("essential homeostatic port absent")?
                .target_neuron();
            let mut found = None;
            'paths: for &first in &outgoing[input as usize] {
                let first_edge = &p.synapses()[first];
                if first_edge.route_index() != 2 || first_edge.target() == tonic_reserve {
                    continue;
                }
                for &second in &outgoing[first_edge.target() as usize] {
                    let second_edge = &p.synapses()[second];
                    if second_edge.route_index() != 3 {
                        continue;
                    }
                    let Some(&tonic_second) = outgoing[tonic_reserve as usize].iter().find(|&&i| {
                        p.synapses()[i].route_index() == 3
                            && p.synapses()[i].target() == second_edge.target()
                    }) else {
                        continue;
                    };
                    for &third in &outgoing[second_edge.target() as usize] {
                        let target = p.synapses()[third].target();
                        if p.synapses()[third].route_index() == 5
                            && motors.contains(&target)
                            && target != reference
                        {
                            found = Some((first, second, third, tonic_second, target));
                            break 'paths;
                        }
                    }
                }
            }
            let (first, second, third, tonic_second, motor) =
                found.ok_or("compatible tonic and need circuit absent")?;
            w[first] = 2.;
            w[second] = 2.;
            w[third] = 4.;
            w[tonic_first] = 8.;
            w[tonic_second] = -4.;
            selected.insert(motor);
            circuits.push(json!({"role":"need","family":family,"homeostatic_lane":lane,"source_neuron":input,"motor_neuron":motor,"bridge_synapses":[first,second],"output_synapse":third}));
            circuits.push(json!({"role":"tonic","family":family,"source_neuron":tonic_source,"motor_neuron":motor,"bridge_synapses":[tonic_first,tonic_second],"output_synapse":third}));
        }
        for (i, synapse) in p.synapses().iter().enumerate() {
            let CompiledSynapseKind::Decoder(c) = synapse.kind() else {
                continue;
            };
            if c.head() != DecoderHeadKind::ActionCandidate || c.family() != family {
                continue;
            }
            let coefficient = match (family, c.input_lane()) {
                (CandidateActionFamily::Ingest, 15) => 12.,
                (CandidateActionFamily::Ingest, 2) => -6.,
                (CandidateActionFamily::Approach, 15) => 4.,
                (CandidateActionFamily::Approach, 2) => -2.,
                (CandidateActionFamily::Approach, 18) => -5.,
                (CandidateActionFamily::Rest, 23) => 2.,
                (CandidateActionFamily::Avoid, 15) => -10.,
                (CandidateActionFamily::Inspect, 6 | 7 | 8) => 0.5,
                (CandidateActionFamily::Contact, 15) => 2.,
                _ => 0.,
            };
            if selected.contains(&synapse.source()) {
                w[i] = coefficient;
            }
            if synapse.source() == reference {
                w[i] = -5. * selected.len() as f32 * coefficient;
            }
        }
        circuits.push(json!({"role":"reference","family":family,"reference_motor":reference,"signal_motors":selected}));
        family_motors.insert(family.raw(), reference);
    }
    Ok((w, circuits))
}
fn trial(
    seed: u64,
    arm: &str,
    gain: f32,
    scene: &str,
    ticks: usize,
    receipt_root: &str,
) -> Result<serde_json::Value> {
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
        let (whole, circuits) = whole_care(&p)?;
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
    if scene == "food" {
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
    let side = 1.;
    let distance = if scene == "far-food" { 2.5 } else { 0.25 };
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", own, Vec3f::ZERO)
        .build()?;
    let object = if scene == "toy" {
        world.spawn_toy("target", Vec3f::new(side * distance, 0., 0.), true)?
    } else {
        world.editor_spawn_object(WorldEditorSpawnSpec {
            label: "target".into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: Vec3f::new(side * distance, 0., 0.),
            nutrition: 0.6,
            hazard_pain: 0.,
            radius: 0.5,
            token_id: None,
        })?
    };
    let mut physical = world.entity(object).unwrap().grounded_physical;
    physical.chemical[0] = if scene == "bitter-food" {
        -0.85
    } else if scene == "toy" {
        0.
    } else {
        0.85
    };
    world.set_grounded_physical_properties(object, physical)?;
    let mut chemistry = *old.biochemistry();
    chemistry.body.set_energy(0.4)?;
    world.register_organism_record(WorldOrganismRecord::new(
        own,
        world.entity_id("learner").unwrap(),
        old.genome().clone(),
        old.phenotype().clone(),
        chemistry,
        old.birth_tick(),
    )?)?;
    game.world = world;
    let mut host = CpuCanonicalHost::new_cohort(game, &trainer, (seed ^ (seed >> 32)) as u32)?;
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
                let expected = if scene == "bitter-food" { -0.85 } else { 0.85 };
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
            let outcome = row.patch.outcome();
            rows.push(json!({"tick":tick,"counter":row.sampling_counter,"action":row.action,"candidates":frame.candidates(),"logits":eval.candidate_logits[0],"ingest_taste_motors":motors,"encoded_hunger_plus_0_1_logit_delta":hunger_probe,"outcome":outcome,"reward":row.biological_reward}));
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
        json!({"seed":seed,"arm":arm,"scene":scene,"edits":edits,"source_digest":source.digest(),"candidate_digest":candidate_digest,"changed_weights":changed_weights,"experimental_graph":experimental,"active_tiles":trainer.phenotype().budgets().global.active_tiles,"microstep_count":trainer.phenotype().microstep_count(),"total_synapses":trainer.phenotype().synapses().len(),"optimizer_unchanged":true,"cold_next_step_exact":true,"target_active":!host.world().entity(object).unwrap().consumed,"final_body":r.biochemistry().body,"final_homeostasis":r.biochemistry().homeostasis,"rows":rows}),
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
        "whole" => vec![
            ("untouched", 0.),
            ("zero-excitatory", 0.),
            ("whole-care", 0.),
            ("whole-care-no-need", 0.),
            ("whole-care-no-tonic", 0.),
            ("whole-care-no-reference", 0.),
        ],
        _ => return Err("mode must be surgery or whole".into()),
    };
    // Reserve a new output directory before any simulation. Never overwrite a prior receipt.
    std::fs::create_dir(path)?;
    let mut results = Vec::new();
    for seed in (2026110701..).take(seeds) {
        for scene in ["food", "bitter-food", "toy", "far-food"] {
            for &(arm, gain) in &arms {
                let r = trial(seed, arm, gain, scene, ticks, path)?;
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
                results.push(r);
            }
        }
    }
    use std::io::Write;
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(format!("{path}/receipt.json"))?;
    output.write_all(&serde_json::to_vec(&json!({"backend":"native-cpu","experimental":true,"promoted":false,"initial_energy":0.4,"newborn_homeostasis_unchanged":true,"gain":gain,"ticks":ticks,"mode":mode,"results":results}))?)?;
    output.sync_all()?;
    Ok(())
}

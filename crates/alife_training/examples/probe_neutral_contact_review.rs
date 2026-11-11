//! Opt-in neutral-contact recipe comparison. CPU only; never reads a saved life.
//! No production/default change. The failure witness is a qualification limit.
#[path = "support/bold_v2_surgery.rs"]
mod bold_v2_surgery;

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
    source_with_anchored(seed, experimental, false)
}

fn source_with_anchored(
    seed: u64,
    experimental: bool,
    anchored_v2: bool,
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
        let (reference, _) = PhenotypeCompiler::compile_n2048_experimental_graph(
            genome.clone(),
            development.clone(),
            config.sensor_profile,
        )?;
        config.sensor_profile = SensorProfile::GroundedTerrainVisionV1;
        let (p, _) = if anchored_v2 {
            PhenotypeCompiler::compile_n2048_anchored_v2_graph(
                genome.clone(),
                development.clone(),
                config.sensor_profile,
            )?
        } else {
            PhenotypeCompiler::compile_n2048_experimental_graph(
                genome.clone(),
                development.clone(),
                config.sensor_profile,
            )?
        };
        assert_eq!(reference.synapses().len(), p.synapses().len());
        assert!(reference
            .synapses()
            .iter()
            .zip(p.synapses())
            .all(|(a, b)| a.source() == b.source()
                && a.target() == b.target()
                && a.route_index() == b.route_index()
                && a.kind() == b.kind()));
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
// Whole-weight revision 6. Feedback-informed: explicit signed tonic/need circuits
// and a native reference motor population; never an external action policy.
fn whole_care(p: &BrainPhenotype) -> Result<(Vec<f32>, Vec<serde_json::Value>)> {
    whole_care_with_contact(p, -2.)
}
fn whole_care_with_contact(
    p: &BrainPhenotype,
    contact_coefficient: f32,
) -> Result<(Vec<f32>, Vec<serde_json::Value>)> {
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
    let mut need_axes = std::collections::BTreeMap::<u16, (u32, u32)>::new();
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
                if first_edge.route_index() != 2
                    || first_edge.target() == tonic_reserve
                    || need_axes
                        .iter()
                        .any(|(other, (r, _))| *other != lane && *r == first_edge.target())
                    || need_axes
                        .get(&lane)
                        .is_some_and(|(r, _)| *r != first_edge.target())
                {
                    continue;
                }
                for &second in &outgoing[first_edge.target() as usize] {
                    let second_edge = &p.synapses()[second];
                    if second_edge.route_index() != 3
                        || need_axes
                            .iter()
                            .any(|(other, (_, t))| *other != lane && *t == second_edge.target())
                        || need_axes
                            .get(&lane)
                            .is_some_and(|(_, t)| *t != second_edge.target())
                    {
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
            need_axes.insert(
                lane,
                (p.synapses()[first].target(), p.synapses()[second].target()),
            );
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
                (CandidateActionFamily::Approach, 18) => contact_coefficient,
                (CandidateActionFamily::Rest, 23) => 2.,
                (CandidateActionFamily::Avoid, 15) => -10.,
                (CandidateActionFamily::Inspect, 6..=8) => 0.5,
                (CandidateActionFamily::Contact, 15) => 2.,
                _ => 0.,
            };
            if selected.contains(&synapse.source()) {
                w[i] = coefficient;
            }
            if synapse.source() == reference {
                let reference_gain = match family {
                    CandidateActionFamily::Approach | CandidateActionFamily::Inspect => 5.,
                    CandidateActionFamily::Rest => -1.,
                    _ => -5.,
                };
                if coefficient != 0. {
                    w[i] = reference_gain * selected.len() as f32 * coefficient;
                }
            }
        }
        circuits.push(json!({"role":"reference","family":family,"reference_motor":reference,"signal_motors":selected}));
    }
    Ok((w, circuits))
}
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
    let (config, p, source) = source(2026111701, experimental)?;
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
        let (whole, mut circuits) = if arm == "whole-care-v5" {
            whole_care_with_contact(&p, -5.)?
        } else {
            whole_care(&p)?
        };
        if arm == "whole-care-bold-v1" || arm == "whole-care-contact-only" {
            let (mut bold, mut bold_edits) = bold_v2_surgery::build_bold_v1(&p, &whole)?;
            if arm == "whole-care-contact-only" {
                let (full, _) = bold_v2_surgery::build_bold_v2(&p, &whole)?;
                let mut count = 0;
                for (i, synapse) in p.synapses().iter().enumerate() {
                    if matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Ingest && c.input_lane()==18)
                        && bold[i].to_bits() != full[i].to_bits()
                    {
                        bold_edits.push(json!({"index":i,"before":bold[i],"after":full[i],"reason":"exact-pinned-contact-only"}));
                        bold[i] = full[i];
                        count += 1;
                    }
                }
                assert_eq!(count, 2);
            }
            circuits.extend(bold_edits);
            weights = bold;
            edits = circuits;
        } else if arm == "whole-care-bold-v2" || arm == "whole-care-bold-v2-anchored" {
            let (bold, bold_edits) = bold_v2_surgery::build_bold_v2(&p, &whole)?;
            circuits.extend(bold_edits);
            weights = bold;
            edits = circuits;
        } else {
            weights = whole;
            edits = circuits;
        }
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
    let canonical_asset_bytes = asset.encode_canonical()?;
    let stage = asset.manifest().training_stage();
    let trainer = CpuFoundationTrainer::new(
        p,
        FoundationWeightAsset::decode_canonical(&canonical_asset_bytes)?,
        mask,
        AdamWConfig::default(),
    )?;
    assert_eq!(
        canonical_asset_bytes,
        trainer.export_candidate(stage)?.encode_canonical()?
    );
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
    let mut world = HeadlessScenarioBuilder::new(2026111701)
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
    let nonfood = scene.contains("nonfood");
    let bitter = scene.contains("bitter");
    let object = world.editor_spawn_object(WorldEditorSpawnSpec {
        label: "target".into(),
        kind: if nonfood {
            WorldObjectKind::ActivityToy
        } else {
            WorldObjectKind::Food
        },
        organism_id: None,
        position: target_position,
        nutrition: if nonfood { 0. } else { 0.6 },
        hazard_pain: 0.,
        radius: 0.5,
        token_id: None,
    })?;
    // Identical observable physical scene across edible/nonedible controls.
    let physical = GroundedPhysicalProperties {
        velocity: Vec3f::ZERO,
        color: [0.4, 0.6, 0.3],
        material: [0.5, 0.3, 0.2],
        shape: [0.4, 0.5, 0.3],
        chemical: [if bitter { -0.85 } else { 0. }, 0., 0.],
        surface_temperature: 0.,
        terrain: [0.5, 0.5],
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
            let targets = frame
                .candidates()
                .iter()
                .filter(|c| c.target.entity == Some(object))
                .collect::<Vec<_>>();
            if tick == 0 {
                assert!(!targets.is_empty());
            }
            for c in &targets {
                let contact = c.features.0[18] > 0.;
                let expected = if contact && bitter {
                    c.features.0[15]
                } else {
                    0.
                };
                assert!(c.features.0[15] <= 0. && c.features.0[15] >= -1.0);
                assert!(
                    (c.features.0[15] - expected).abs() < 1e-5,
                    "taste must be contact-gated: scene={scene} tick={tick} contact={} taste={}",
                    c.features.0[18],
                    c.features.0[15]
                );
                if tick == 0 {
                    assert_eq!(contact, !scene.starts_with("far-"));
                }
            }
            let encoded_hunger = trainer
                .phenotype()
                .sensor_encoder()
                .assignments()
                .iter()
                .filter(|a| {
                    a.source_group() == SensorEncoderSourceGroup::Homeostasis
                        && a.source_index() == 0
                })
                .map(|a| row.replay_tick.encoded_inputs[a.target_neuron() as usize])
                .collect::<Vec<_>>();
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
            rows.push(json!({"tick":tick,"body_after":host.world().organism_registry().get(own).unwrap().biochemistry().body,"position_after":host.world().entity(host.world().entity_id("learner").unwrap()).unwrap().position,"encoded_hunger":encoded_hunger,"counter":row.sampling_counter,"action":row.action,"candidates":frame.candidates(),"logits":eval.candidate_logits[0],"ingest_taste_motors":motors,"encoded_hunger_plus_0_1_logit_delta":hunger_probe,"encoded_hunger_3_frozen_frames_delta":hunger_three_frames,"constructed_neuron_activations":debug_activations,"effective_bridges":effective_bridges,"enabled_routes":row.replay_tick.enabled_routes,"outcome":outcome,"reward":row.biological_reward}));
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
        json!({"founder_seed":2026111701u64,"sensor_profile":config.sensor_profile,"candidate_reexport_exact":true,"seed":seed,"arm":arm,"scene":scene,"edits":edits,"source_digest":source.digest(),"candidate_digest":candidate_digest,"changed_weights":changed_weights,"experimental_graph":experimental,"active_tiles":trainer.phenotype().budgets().global.active_tiles,"microstep_count":trainer.phenotype().microstep_count(),"total_synapses":trainer.phenotype().synapses().len(),"preparation_ticks":preparation_ticks,"target_hazard_pain":target_hazard_pain,"initial_biology":initial_biology,"optimizer_unchanged":true,"cold_next_step_exact":true,"target_active":!host.world().entity(object).unwrap().consumed,"final_position":host.world().entity(host.world().entity_id("learner").unwrap()).unwrap().position,"final_target_carrier":host.world().entity(object).unwrap().carried_by,"final_body":r.biochemistry().body,"final_homeostasis":r.biochemistry().homeostasis,"rows":rows}),
    )
}
fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let path = args.get(1).ok_or("output JSON required")?;
    let ticks: usize = args.get(2).map_or(Ok(48), |s| s.parse())?;
    let gain: f32 = args.get(3).map_or(Ok(0.), |s| s.parse())?;
    let mode = args.get(4).map_or("neutral-matrix", String::as_str);
    let seeds: usize = args.get(5).map_or(Ok(8), |s| s.parse())?;
    let seed_start: u64 = args.get(6).map_or(Ok(2026111901), |s| s.parse())?;
    let scenes = args
        .get(7)
        .map_or("neutral-food,bitter-food,neutral-nonfood,bitter-nonfood,far-neutral-food,far-bitter-food,far-neutral-nonfood,far-bitter-nonfood", String::as_str)
        .split(',')
        .collect::<Vec<_>>();
    let initial_energy: f32 = args.get(8).map_or(Ok(0.18), |s| s.parse())?;
    let preparation_ticks: usize = args.get(9).map_or(Ok(1200), |s| s.parse())?;
    let target_hazard_pain: f32 = args.get(10).map_or(Ok(0.), |s| s.parse())?;
    if target_hazard_pain != 0.0 {
        return Err("this comparison fixes hazard pain at zero".into());
    }
    if gain != 0.0 {
        return Err("the reserved gain argument must be zero for these fixed recipes".into());
    }
    if preparation_ticks > 12000 {
        return Err("preparation ticks must be0..12000".into());
    }
    if seed_start.checked_add(seeds as u64).is_none()
        || scenes.is_empty()
        || scenes.len() > 12
        || scenes.iter().any(|s| {
            ![
                "neutral-nonfood",
                "bitter-nonfood",
                "far-neutral-nonfood",
                "far-bitter-nonfood",
                "far-bitter-food",
                "bitter-food",
                "neutral-food",
                "far-neutral-food",
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
        "neutral-matrix" => vec![
            ("whole-care-bold-v1", 0.),
            ("whole-care-contact-only", 0.),
            ("whole-care-bold-v2", 0.),
        ],
        _ => return Err("this opt-in experiment supports only neutral-matrix mode".into()),
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
mod review_tests {
    use super::*;

    #[test]
    fn exact_contact_delta_and_roundtrip() {
        let (_, p, source) = source(2026111701, true).unwrap();
        let (whole, _) = whole_care(&p).unwrap();
        let (baseline, _) = bold_v2_surgery::build_bold_v1(&p, &whole).unwrap();
        let (full, _) = bold_v2_surgery::build_bold_v2(&p, &whole).unwrap();
        let mut contact = baseline.clone();
        let mut changed = Vec::new();
        for (i, synapse) in p.synapses().iter().enumerate() {
            if matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Ingest && c.input_lane()==18)
                && baseline[i].to_bits() != full[i].to_bits()
            {
                changed.push((i, full[i]));
                contact[i] = full[i];
            }
        }
        assert_eq!(changed, vec![(27168, 8.0), (27183, -42.0)]);
        use sha2::{Digest, Sha256};
        for (weights, expected) in [
            (
                baseline.clone(),
                "850198921b689947aa4d244093be9b149a9ad2b74272a1a2133f957147fe8dfb",
            ),
            (
                contact.clone(),
                "2b3b019ff7f09fb40391b4ff56bfea698d3419b26b983406f3ada7522b7c030b",
            ),
            (
                full,
                "45d34e4d7a76bc69fff51f73d11a421d2547dc9748160d80cca2a17949bcd0b4",
            ),
        ] {
            let asset = FoundationWeightAsset::from_trained_weights(
                &p,
                weights,
                source.manifest().training_stage(),
            )
            .unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(asset.encode_canonical().unwrap())),
                expected
            );
        }

        assert_eq!(
            baseline
                .iter()
                .zip(&contact)
                .filter(|(a, b)| a.to_bits() != b.to_bits())
                .count(),
            2
        );
        let asset = FoundationWeightAsset::from_trained_weights(
            &p,
            contact,
            source.manifest().training_stage(),
        )
        .unwrap();
        let bytes = asset.encode_canonical().unwrap();
        assert_eq!(
            bytes,
            FoundationWeightAsset::decode_canonical(&bytes)
                .unwrap()
                .encode_canonical()
                .unwrap()
        );
    }

    #[test]
    fn neutral_intake_and_nonfood_failure_witness() {
        let run = |arm, scene| {
            trial(TrialSpec {
                seed: 2026111901,
                arm,
                gain: 0.0,
                scene,
                ticks: 48,
                receipt_root: "",
                write_asset: false,
                initial_energy: 0.18,
                preparation_ticks: 1200,
                target_hazard_pain: 0.0,
            })
            .unwrap()
        };
        let base_food = run("whole-care-bold-v1", "neutral-food");
        let contact_food = run("whole-care-contact-only", "neutral-food");
        assert_eq!(base_food["target_active"], true);
        assert_eq!(contact_food["target_active"], false);
        let base_toy = run("whole-care-bold-v1", "neutral-nonfood");
        let contact_toy = run("whole-care-contact-only", "neutral-nonfood");
        let failed_ingests = |r: &serde_json::Value| {
            r["rows"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| {
                    row["outcome"]["success"] == false
                        && row["action"]["motor_candidates"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter_map(|i| i.as_u64())
                            .any(|i| row["candidates"][i as usize]["family"] == "Ingest")
                })
                .count()
        };
        assert_eq!(contact_toy["target_active"], true);
        assert!(failed_ingests(&contact_toy) > failed_ingests(&base_toy));
        assert!(
            failed_ingests(&contact_toy) > 1,
            "the observed qualification limit must stay visible"
        );
    }
}

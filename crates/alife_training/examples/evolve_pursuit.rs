//! Native CPU behavioral assay for experimental weight-space evolution.
//! Moving Food is a pursuit surrogate, not live prey ecology. No saved life is read.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask};
use alife_world::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{BufRead, Write};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const FOUNDER_SEED: u64 = 2026111701;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    params: [f32; 6],
    seed_start: u64,
    seeds: usize,
    ticks: usize,
    scenes: Vec<String>,
    #[serde(default)]
    trace: bool,
    #[serde(default)]
    memory_aversion: f32,
    export: Option<String>,
}

#[derive(Serialize)]
struct Trial {
    seed: u64,
    scene: String,
    consumed: bool,
    caught_tick: Option<usize>,
    consumed_tick: Option<usize>,
    min_distance: f32,
    final_distance: f32,
    initial_energy: f32,
    final_energy: f32,
    injury: f32,
    decisions: usize,
    failed_ingests: usize,
    actions: std::collections::BTreeMap<String, usize>,
    trace: Vec<serde_json::Value>,
    target_motion: Vec<serde_json::Value>,
}

fn config(asset: &FoundationWeightAsset) -> Result<CanonicalNewGameConfig> {
    let mut c = CanonicalNewGameConfig::phase3(FOUNDER_SEED, 1)?;
    c.brain_class = BrainScaleTier::Standard2048;
    c.sensor_profile = asset.manifest().sensor_profile();
    Ok(c)
}

fn phenotype(asset: &FoundationWeightAsset) -> Result<BrainPhenotype> {
    let game = create_canonical_new_game_with_n2048_experimental_candidate(&config(asset)?, asset)?;
    let r = game.world.organism_registry().iter().next().unwrap();
    let dev = foundation_construction_development(
        &r.phenotype().brain_genome,
        &BrainCapacityClass::n2048(),
        &r.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    Ok(
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            r.phenotype().brain_genome.clone(),
            dev,
            asset.clone(),
        )?
        .0,
    )
}

// Six coherent decoder perturbations. Each is a real genetic weight edit; the
// evaluator supplies no host action bonuses, targets, or reward shaping.
fn candidate(
    base: &FoundationWeightAsset,
    p: &BrainPhenotype,
    x: [f32; 6],
) -> Result<FoundationWeightAsset> {
    if x.iter().any(|v| !v.is_finite() || v.abs() > 64.) {
        return Err("parameters must be finite and within +/-64".into());
    }
    let mut w = base.weights().to_vec();
    for (i, s) in p.synapses().iter().enumerate() {
        let CompiledSynapseKind::Decoder(c) = s.kind() else {
            continue;
        };
        if c.head() != DecoderHeadKind::ActionCandidate {
            continue;
        }
        let coefficient = match (c.family(), c.input_lane()) {
            (CandidateActionFamily::Approach, 6..=14) => x[0],
            (CandidateActionFamily::Approach, 2) => x[1],
            (CandidateActionFamily::Approach, 18) => x[2],
            (CandidateActionFamily::Ingest, 18) => x[3],
            (CandidateActionFamily::Ingest, 2) => x[4],
            (CandidateActionFamily::Inspect, 6..=14) => x[5],
            _ => 0.,
        };
        let selected_source = match (c.family(), c.input_lane()) {
            (CandidateActionFamily::Approach, 18) => 1791,
            (CandidateActionFamily::Approach, _) => 1776,
            (CandidateActionFamily::Ingest, _) => 1839,
            (CandidateActionFamily::Inspect, _) => 1760,
            _ => continue,
        };
        if s.source() == selected_source {
            w[i] += coefficient;
        }
    }
    Ok(FoundationWeightAsset::from_trained_weights(
        p,
        w,
        base.manifest().training_stage(),
    )?)
}

fn distance(a: Vec3f, b: Vec3f) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

fn memory_candidate(
    base: FoundationWeightAsset,
    p: &BrainPhenotype,
    gain: f32,
) -> Result<FoundationWeightAsset> {
    if !gain.is_finite() || !(0.0..=64.0).contains(&gain) {
        return Err("memory aversion must be finite in 0..64".into());
    }
    if gain == 0.0 {
        return Ok(base);
    }
    let mut weights = base.weights().to_vec();
    for (lane, sign) in [(32, 1.0), (34, -1.0)] {
        let indices = p.synapses().iter().enumerate().filter_map(|(i,s)|
            matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::MemoryContext
                && c.family()==CandidateActionFamily::Ingest && c.input_lane()==lane).then_some(i)
        ).collect::<Vec<_>>();
        if indices.is_empty() {
            return Err("memory readout lane missing".into());
        }
        let delta = sign * gain / indices.len() as f32;
        for i in indices {
            weights[i] += delta;
        }
    }
    Ok(FoundationWeightAsset::from_trained_weights(
        p,
        weights,
        base.manifest().training_stage(),
    )?)
}

fn trial(
    asset: &FoundationWeightAsset,
    trainer: &CpuFoundationTrainer,
    seed: u64,
    scene: &str,
    ticks: usize,
    trace: bool,
) -> Result<Trial> {
    let mut game =
        create_canonical_new_game_with_n2048_experimental_candidate(&config(asset)?, asset)?;
    let old = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let own = old.organism_id();
    let mirror = if seed.is_multiple_of(2) { 1. } else { -1. };
    let initial = match scene {
        "contact" | "toy" => Vec3f::new(0.25, 0., 0.),
        "crossing" | "frozen-crossing" => Vec3f::new(2.5, 0., -mirror),
        _ => Vec3f::new(2.5, 0., 0.),
    };
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("hunter", own, Vec3f::ZERO)
        .build()?;
    let target = world.editor_spawn_object(WorldEditorSpawnSpec {
        label: "target".into(),
        kind: if scene == "toy" {
            WorldObjectKind::ActivityToy
        } else {
            WorldObjectKind::Food
        },
        organism_id: None,
        position: initial,
        nutrition: if scene == "toy" { 0. } else { 0.6 },
        hazard_pain: 0.,
        radius: 0.3,
        token_id: None,
    })?;
    // Food and toy appearance are matched. Taste is initially unknown at range.
    world.set_grounded_physical_properties(
        target,
        GroundedPhysicalProperties {
            velocity: Vec3f::ZERO,
            color: [0.4, 0.6, 0.3],
            material: [0.5, 0.3, 0.2],
            shape: [0.4, 0.5, 0.3],
            chemical: [0., 0., 0.],
            surface_temperature: 0.,
            terrain: [0.5, 0.5],
        },
    )?;
    if scene == "obstacle" {
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
    let mut biology = *old.biochemistry();
    biology.body.set_energy(0.25)?;
    world.register_organism_record(WorldOrganismRecord::new(
        own,
        world.entity_id("hunter").unwrap(),
        old.genome().clone(),
        old.phenotype().clone(),
        biology,
        old.birth_tick(),
    )?)?;
    game.world = world;
    let (mut host, _) = CpuCanonicalHost::new_prepared_cohort(game, trainer, seed as u32, |w| {
        for _ in 0..600 {
            w.try_advance_tick()?;
        }
        Ok::<_, ScaffoldContractError>(())
    })?;
    let hunter = host.world().entity_id("hunter").unwrap();
    let body = host
        .world()
        .organism_registry()
        .get(own)
        .unwrap()
        .biochemistry()
        .body;
    let mut result = Trial {
        seed,
        scene: scene.into(),
        consumed: false,
        caught_tick: None,
        consumed_tick: None,
        min_distance: distance(Vec3f::ZERO, initial),
        final_distance: distance(Vec3f::ZERO, initial),
        initial_energy: body.energy,
        final_energy: body.energy,
        injury: 0.,
        decisions: 0,
        failed_ingests: 0,
        actions: Default::default(),
        trace: Vec::new(),
        target_motion: Vec::new(),
    };
    for tick in 0..ticks {
        let object = host.world().entity(target).unwrap();
        if !object.consumed && object.carried_by.is_none() {
            let t = tick as f32;
            let next = match scene {
                "receding" => Vec3f::new(initial.x + 0.012 * t, 0., 0.),
                "crossing" => Vec3f::new(initial.x, 0., initial.z + mirror * 0.020 * t),
                "zigzag" => Vec3f::new(initial.x + 0.008 * t, 0., mirror * 0.4 * (t * 0.08).sin()),
                _ => object.position,
            };
            let previous = object.position;
            host.world_mut().editor_move_object(target, next)?;
            let actual = host.world().entity(target).unwrap().position;
            if trace {
                result
                    .target_motion
                    .push(json!({"world_tick":host.world().tick().raw(),
                    "previous":previous,"current":actual,"translated":distance(previous,actual)}));
            }
        }
        let step = host.step(trainer, 1.)?;
        for row in step.decisions {
            result.decisions += 1;
            let frame = row.patch.pre_action().perception();
            for &index in row.action.motor_candidates.iter().flatten() {
                if let Some(c) = frame.candidates().get(index as usize) {
                    *result.actions.entry(format!("{:?}", c.family)).or_default() += 1;
                    if c.family == CandidateActionFamily::Ingest && !row.patch.outcome().success {
                        result.failed_ingests += 1;
                    }
                }
            }
            if trace {
                result.trace.push(json!({"tick":tick,"action":row.action,"candidates":frame.candidates(),
                    "reward":row.biological_reward,"outcome":row.patch.outcome(),
                    "motor_activations": ([1760,1775,1776,1791,1824,1839].map(|i| row.final_features[i])),
                    "position":host.world().entity(hunter).unwrap().position,
                    "target":host.world().entity(target).unwrap().position}));
            }
        }
        let object = host.world().entity(target).unwrap();
        result.final_distance = distance(
            host.world().entity(hunter).unwrap().position,
            object.position,
        );
        result.min_distance = result.min_distance.min(result.final_distance);
        if object.carried_by == Some(own) && result.caught_tick.is_none() {
            result.caught_tick = Some(tick);
        }
        if object.consumed {
            result.consumed = true;
            result.consumed_tick = Some(tick);
            break;
        }
    }
    let body = host
        .world()
        .organism_registry()
        .get(own)
        .unwrap()
        .biochemistry()
        .body;
    result.final_energy = body.energy;
    result.injury = body.injury;
    Ok(result)
}

fn run(base: &FoundationWeightAsset, p: &BrainPhenotype, r: Request) -> Result<serde_json::Value> {
    if r.seeds == 0
        || r.seeds > 100
        || r.ticks == 0
        || r.ticks > 2000
        || r.scenes.is_empty()
        || r.seed_start.checked_add(r.seeds as u64).is_none()
        || r.scenes.iter().any(|s| {
            ![
                "contact",
                "stationary",
                "receding",
                "crossing",
                "frozen-crossing",
                "zigzag",
                "obstacle",
                "toy",
            ]
            .contains(&s.as_str())
        })
    {
        return Err("invalid trial dimensions or scene".into());
    }
    let start = std::time::Instant::now();
    let asset = memory_candidate(candidate(base, p, r.params)?, p, r.memory_aversion)?;
    let compiled = phenotype(&asset)?;
    assert!(compiled
        .synapses()
        .iter()
        .zip(asset.weights())
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
    let mask = StageTrainableMask::recurrent_only(&compiled)?;
    let trainer = CpuFoundationTrainer::new(compiled, asset.clone(), mask, AdamWConfig::default())?;
    assert_eq!(trainer.weights(), asset.weights());
    let mut trials = Vec::new();
    for seed in r.seed_start..r.seed_start + r.seeds as u64 {
        for scene in &r.scenes {
            trials.push(trial(&asset, &trainer, seed, scene, r.ticks, r.trace)?);
        }
    }
    if let Some(path) = r.export {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        f.write_all(&asset.encode_canonical()?)?;
    }
    Ok(
        json!({"backend":"native-cpu", "base_digest":base.digest(),"candidate_digest":asset.digest(),
        "assay_version":2,"params":r.params,"memory_aversion":r.memory_aversion,"ticks":r.ticks,"seconds":start.elapsed().as_secs_f64(),"trials":trials}),
    )
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: evolve_pursuit <experimental-foundation>; JSON requests on stdin")?;
    let base = FoundationWeightAsset::decode_canonical(&std::fs::read(path)?)?;
    let p = phenotype(&base)?;
    eprintln!(
        "ready: {:?}, {} neurons, {} synapses",
        p.sensor_profile(),
        p.neuron_count(),
        p.synapses().len()
    );
    if std::env::args().nth(2).as_deref() == Some("--inspect") {
        println!(
            "{}",
            json!({"weights":base.weights(),"synapses":p.synapses().iter().map(|s|
            json!({"source":s.source(),"target":s.target(),"kind":s.kind(),"route":s.route_index()})
        ).collect::<Vec<_>>()})
        );
        return Ok(());
    }
    for line in std::io::stdin().lock().lines() {
        let response = serde_json::from_str::<Request>(&line?)
            .map_err(Into::into)
            .and_then(|r| run(&base, &p, r));
        match response {
            Ok(value) => println!("{}", value),
            Err(error) => println!("{}", json!({"error":error.to_string()})),
        }
        std::io::stdout().flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> FoundationWeightAsset {
        FoundationWeightAsset::decode_canonical(include_bytes!(
            "../../../assets/foundations/experimental-anchored-v2-research.alife-foundation"
        ))
        .unwrap()
    }

    #[test]
    fn every_search_direction_changes_the_intended_compiled_weights() {
        let base = base();
        let p = phenotype(&base).unwrap();
        assert_eq!(
            candidate(&base, &p, [0.; 6])
                .unwrap()
                .encode_canonical()
                .unwrap(),
            base.encode_canonical().unwrap()
        );
        for (gene, expected) in [9, 1, 1, 1, 1, 9].into_iter().enumerate() {
            let mut x = [0.; 6];
            x[gene] = 0.25;
            let edited = candidate(&base, &p, x).unwrap();
            assert_eq!(
                base.weights()
                    .iter()
                    .zip(edited.weights())
                    .filter(|(a, b)| a.to_bits() != b.to_bits())
                    .count(),
                expected
            );
            let compiled = phenotype(&edited).unwrap();
            assert!(compiled
                .synapses()
                .iter()
                .zip(edited.weights())
                .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
        }
    }

    #[test]
    fn memory_variant_changes_only_ingest_consequence_readouts() {
        let base = base();
        let p = phenotype(&base).unwrap();
        let edited = memory_candidate(base.clone(), &p, 16.).unwrap();
        let mut changed = 0;
        for ((s, a), b) in p
            .synapses()
            .iter()
            .zip(base.weights())
            .zip(edited.weights())
        {
            if a.to_bits() != b.to_bits() {
                changed += 1;
                assert!(
                    matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::MemoryContext
                    && c.family()==CandidateActionFamily::Ingest && matches!(c.input_lane(),32|34))
                );
            }
        }
        assert_eq!(changed, 80);
        let compiled = phenotype(&edited).unwrap();
        assert!(compiled
            .synapses()
            .iter()
            .zip(edited.weights())
            .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
    }

    #[test]
    fn edible_and_nonfood_controls_have_identical_initial_visible_features() {
        let base = base();
        let p = phenotype(&base).unwrap();
        let mask = StageTrainableMask::recurrent_only(&p).unwrap();
        let trainer =
            CpuFoundationTrainer::new(p, base.clone(), mask, AdamWConfig::default()).unwrap();
        let food = trial(&base, &trainer, 2026111300, "contact", 1, true).unwrap();
        let toy = trial(&base, &trainer, 2026111300, "toy", 1, true).unwrap();
        assert_eq!(food.trace[0]["candidates"], toy.trace[0]["candidates"]);
        for c in food.trace[0]["candidates"].as_array().unwrap() {
            if !c["target"]["entity"].is_null() {
                assert_eq!(c["features"][15], 0.0);
            }
        }
    }
}

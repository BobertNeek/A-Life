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
    #[serde(default)]
    variation: Variation,
    #[serde(default = "unit_scales")]
    readout_scale: [f32; 2],
    #[serde(default)]
    bearing_gain: f32,
    #[serde(default = "one_head")]
    motor_heads: u8,
    #[serde(default)]
    hold_readout: [f32; 2],
    #[serde(default)]
    weight_edits: Vec<(usize, f32)>,
}

fn one_head() -> u8 {
    1
}

fn unit_scales() -> [f32; 2] {
    [1., 1.]
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct Variation {
    angle: f32,
    speed: f32,
    appearance: u8,
    distance_scale: f32,
    initial_energy: f32,
    terrain: [f32; 2],
    neural_preparation: bool,
    continue_after_consumption: bool,
}
impl Default for Variation {
    fn default() -> Self {
        Self {
            angle: 0.,
            speed: 1.,
            appearance: 0,
            distance_scale: 1.,
            initial_energy: 0.25,
            terrain: [0.5, 0.5],
            neural_preparation: false,
            continue_after_consumption: false,
        }
    }
}
impl Variation {
    fn rotate(self, p: Vec3f) -> Vec3f {
        let (s, c) = self.angle.sin_cos();
        Vec3f::new(c * p.x - s * p.z, p.y, s * p.x + c * p.z)
    }
    fn appearance(self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        match self.appearance {
            1 => ([0.1, 0.1, 0.1], [0.2, 0.1, 0.1], [0.2, 0.1, 0.2]),
            2 => ([0.9, 0.2, 0.7], [0.8, 0.7, 0.1], [0.1, 0.8, 0.9]),
            3 => ([0.2, 0.3, 0.9], [0.1, 0.9, 0.6], [0.8, 0.2, 0.1]),
            _ => ([0.4, 0.6, 0.3], [0.5, 0.3, 0.2], [0.4, 0.5, 0.3]),
        }
    }
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
    initial_world_tick: u64,
    initial_drives: DriveSnapshot,
    final_drives: DriveSnapshot,
    sleeping_steps: usize,
    world_steps: usize,
    injury: f32,
    alive: bool,
    decisions: usize,
    preparation_decisions: usize,
    preparation_microsteps: std::collections::BTreeMap<u32, usize>,
    microstep_counts: std::collections::BTreeMap<u32, usize>,
    failed_ingests: usize,
    no_manipulations: usize,
    contact_interactions: usize,
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

fn construction_context(asset: &FoundationWeightAsset) -> Result<(BrainGenome, DevelopmentState)> {
    let game = create_canonical_new_game_with_n2048_experimental_candidate(&config(asset)?, asset)?;
    let r = game.world.organism_registry().iter().next().unwrap();
    let dev = foundation_construction_development(
        &r.phenotype().brain_genome,
        &BrainCapacityClass::n2048(),
        &r.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    Ok((r.phenotype().brain_genome.clone(), dev))
}

fn phenotype(asset: &FoundationWeightAsset) -> Result<BrainPhenotype> {
    let (genome, dev) = construction_context(asset)?;
    Ok(
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            genome,
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

fn scale_readouts(
    base: FoundationWeightAsset,
    p: &BrainPhenotype,
    scale: [f32; 2],
    bearing: f32,
) -> Result<FoundationWeightAsset> {
    if scale
        .iter()
        .any(|v| !v.is_finite() || !(0.0..=4.0).contains(v))
        || !bearing.is_finite()
        || bearing.abs() > 64.
    {
        return Err("readout scales must be in 0..4 and bearing gain within +/-64".into());
    }
    if scale == [1., 1.] && bearing == 0. {
        return Ok(base);
    }
    let mut w = base.weights().to_vec();
    for (i, s) in p.synapses().iter().enumerate() {
        if let CompiledSynapseKind::Decoder(c) = s.kind() {
            if c.head() == DecoderHeadKind::ActionCandidate {
                match c.family() {
                    CandidateActionFamily::Approach => w[i] *= scale[0],
                    CandidateActionFamily::Ingest => w[i] *= scale[1],
                    _ => {}
                }
                if c.family() == CandidateActionFamily::Approach
                    && c.input_lane() == 1
                    && s.source() == 1791
                {
                    w[i] += bearing;
                }
            }
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

fn edit_weights(
    base: FoundationWeightAsset,
    p: &BrainPhenotype,
    edits: &[(usize, f32)],
) -> Result<FoundationWeightAsset> {
    if edits.is_empty() {
        return Ok(base);
    }
    let mut weights = base.weights().to_vec();
    for &(index, delta) in edits {
        if !delta.is_finite() {
            return Err("weight delta must be finite".into());
        }
        *weights.get_mut(index).ok_or("weight index out of range")? += delta;
    }
    Ok(FoundationWeightAsset::from_trained_weights(
        p,
        weights,
        base.manifest().training_stage(),
    )?)
}

// Existing NO_MANIPULATION uses Contact lane23=1, lane18=0. Object Contact
// candidates share these lanes; measure both effects, with no host action gate.
fn hold_candidate(
    base: FoundationWeightAsset,
    p: &BrainPhenotype,
    delta: [f32; 2],
) -> Result<FoundationWeightAsset> {
    if delta.iter().any(|v| !v.is_finite() || v.abs() > 64.) {
        return Err("hold readout deltas must be finite within +/-64".into());
    }
    if delta == [0.; 2] {
        return Ok(base);
    }
    let mut weights = base.weights().to_vec();
    for (i, s) in p.synapses().iter().enumerate() {
        if let CompiledSynapseKind::Decoder(c) = s.kind() {
            if c.head() == DecoderHeadKind::ActionCandidate
                && c.family() == CandidateActionFamily::Contact
                && s.source() == 1823
            {
                match c.input_lane() {
                    23 => weights[i] += delta[0],
                    18 => weights[i] += delta[1],
                    _ => {}
                }
            }
        }
    }
    Ok(FoundationWeightAsset::from_trained_weights(
        p,
        weights,
        base.manifest().training_stage(),
    )?)
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

// Fresh founder chromosome experiment. All prior weights and coordinates survive;
// no acquired life is admitted or rewritten. Exported variants also need their genome.
fn prepare_founder(
    asset: FoundationWeightAsset,
    heads: u8,
) -> Result<(FoundationWeightAsset, BrainPhenotype, CanonicalNewGame)> {
    let old = phenotype(&asset)?;
    let mut game =
        create_canonical_new_game_with_n2048_experimental_candidate(&config(&asset)?, &asset)?;
    if heads == 1 {
        return Ok((asset, old, game));
    }
    if !(2..=4).contains(&heads) {
        return Err("motor heads must be 1..4".into());
    }
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let mut genome = record.genome().clone();
    for architecture in [
        &mut genome.brain.construction.maternal_architecture,
        &mut genome.brain.construction.paternal_architecture,
    ] {
        let mut wire = serde_json::to_value(*architecture)?;
        wire["motor_head_count"] = json!(heads);
        *architecture = serde_json::from_value(wire)?;
    }
    // Express the same inherited body and edited construction genes before rebinding.
    genome.n2048_foundation_candidate = None;
    let expressed = genome.express()?;
    let development = foundation_construction_development(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &expressed.development_state_at(Tick::ZERO)?,
    )?;
    let (fresh, _) = PhenotypeCompiler::compile_n2048_anchored_v2_graph(
        expressed.brain_genome.clone(),
        development.clone(),
        asset.manifest().sensor_profile(),
    )?;
    if old.synapses().len() != fresh.synapses().len()
        || old.synapses().iter().zip(fresh.synapses()).any(|(a, b)| {
            a.source() != b.source()
                || a.target() != b.target()
                || a.kind() != b.kind()
                || a.route_index() != b.route_index()
        })
    {
        return Err("motor chromosome edit changed weight coordinates".into());
    }
    let rebound = FoundationWeightAsset::from_trained_weights(
        &fresh,
        asset.weights().to_vec(),
        asset.manifest().training_stage(),
    )?;
    genome = genome.with_n2048_foundation_candidate(rebound.clone())?;
    let expressed = genome.express()?;
    let (compiled, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        expressed.brain_genome.clone(),
        development,
        rebound.clone(),
    )?;
    game.world
        .replace_organism_record_exact(WorldOrganismRecord::new(
            record.organism_id(),
            record.world_entity_id(),
            genome.clone(),
            expressed,
            *record.biochemistry(),
            record.birth_tick(),
        )?)?;
    game.creatures[0].weights.genetic_fixed_digest =
        PortableAssetDigest::for_bytes(&serde_json::to_vec(&genome)?).0;
    Ok((rebound, compiled, game))
}

// Fixture placement is a teleport, not a physical motion interval. Keep the
// World editor's displacement semantics for real target motion below, but do
// not leave an offstage parking displacement in stationary sensory input.
fn place_stationary_fixture(
    world: &mut HeadlessWorld,
    id: WorldEntityId,
    position: Vec3f,
) -> Result<()> {
    world.editor_move_object(id, position)?;
    let mut properties = world.entity(id).ok_or("missing fixture")?.grounded_physical;
    properties.velocity = Vec3f::ZERO;
    world.set_grounded_physical_properties(id, properties)?;
    Ok(())
}

fn trial(
    template: &CanonicalNewGame,
    trainer: &CpuFoundationTrainer,
    seed: u64,
    scene: &str,
    ticks: usize,
    trace: bool,
    variation: Variation,
) -> Result<Trial> {
    let mut game = template.clone();
    let old = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let own = old.organism_id();
    let mirror = if seed.is_multiple_of(2) { 1. } else { -1. };
    let mut initial_local = match scene {
        "contact" | "toy" | "hazard" | "delayed-hazard" => Vec3f::new(0.25, 0., 0.),
        "toy-then-food" | "wait-then-food" => Vec3f::new(100., 0., 0.),
        "crossing" | "frozen-crossing" => Vec3f::new(2.5, 0., -mirror),
        _ => Vec3f::new(2.5, 0., 0.),
    };
    if !matches!(
        scene,
        "contact" | "toy" | "hazard" | "delayed-hazard" | "toy-then-food" | "wait-then-food"
    ) {
        initial_local.x *= variation.distance_scale;
        initial_local.z *= variation.distance_scale;
    }
    let initial = variation.rotate(initial_local);
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("hunter", own, Vec3f::ZERO)
        .build()?;
    let target = world.editor_spawn_object(WorldEditorSpawnSpec {
        label: "target".into(),
        kind: match scene {
            "toy" => WorldObjectKind::ActivityToy,
            "hazard" | "delayed-hazard" => WorldObjectKind::Hazard,
            _ => WorldObjectKind::Food,
        },
        organism_id: None,
        position: if matches!(scene, "hazard" | "delayed-hazard") {
            Vec3f::new(100., 0., 0.)
        } else {
            initial
        },
        nutrition: if matches!(scene, "toy" | "hazard" | "delayed-hazard") {
            0.
        } else {
            0.6
        },
        hazard_pain: if matches!(scene, "hazard" | "delayed-hazard") {
            0.05
        } else {
            0.
        },
        radius: 0.3,
        token_id: None,
    })?;
    // Food and toy appearance are matched. Taste is initially unknown at range.
    let (color, material, shape) = variation.appearance();
    world.set_grounded_physical_properties(
        target,
        GroundedPhysicalProperties {
            velocity: Vec3f::ZERO,
            color,
            material,
            shape,
            chemical: [0., 0., 0.],
            surface_temperature: 0.,
            terrain: variation.terrain,
        },
    )?;
    let mut distraction_id = None;
    if matches!(scene, "choice" | "toy-then-food" | "hazard-choice") {
        let distraction = world.editor_spawn_object(WorldEditorSpawnSpec {
            label: "nonfood".into(),
            kind: if scene == "hazard-choice" {
                WorldObjectKind::Hazard
            } else {
                WorldObjectKind::ActivityToy
            },
            organism_id: None,
            position: if scene == "hazard-choice" {
                Vec3f::new(100., 0., 0.)
            } else {
                variation.rotate(Vec3f::new(0.25, 0., 0.))
            },
            nutrition: 0.,
            hazard_pain: if scene == "hazard-choice" { 0.05 } else { 0. },
            radius: 0.3,
            token_id: None,
        })?;
        distraction_id = Some(distraction);
        world.set_grounded_physical_properties(
            distraction,
            GroundedPhysicalProperties {
                velocity: Vec3f::ZERO,
                color,
                material,
                shape,
                chemical: [0.; 3],
                surface_temperature: 0.,
                terrain: variation.terrain,
            },
        )?;
    }
    let obstacle_id = if matches!(scene, "obstacle" | "partial-obstacle" | "moving-obstacle") {
        Some(world.editor_spawn_object(WorldEditorSpawnSpec {
            label: "obstacle".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: variation.rotate(Vec3f::new(
                1.25 * variation.distance_scale,
                0.,
                if scene == "partial-obstacle" {
                    0.45 * mirror * variation.distance_scale
                } else {
                    0.
                },
            )),
            nutrition: 0.,
            hazard_pain: 0.,
            radius: 0.35,
            token_id: None,
        })?)
    } else {
        None
    };
    // Replace biology-only preparation with the same number of real host steps,
    // without exposing the experimental objects during those steps.
    let mut parked = Vec::new();
    if variation.neural_preparation {
        for id in [Some(target), distraction_id, obstacle_id]
            .into_iter()
            .flatten()
        {
            parked.push((id, world.entity(id).unwrap().position));
            place_stationary_fixture(
                &mut world,
                id,
                Vec3f::new(100. + parked.len() as f32 * 10., 0., 0.),
            )?;
        }
    }
    let mut biology = *old.biochemistry();
    biology.body.set_energy(variation.initial_energy)?;
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
        if !variation.neural_preparation {
            for _ in 0..600 {
                w.try_advance_tick()?;
            }
        }
        Ok::<_, ScaffoldContractError>(())
    })?;
    let mut preparation_decisions = 0;
    let mut preparation_microsteps = std::collections::BTreeMap::new();
    if variation.neural_preparation {
        for _ in 0..600 {
            let step = host.step(trainer, 1.)?;
            for row in step.decisions {
                preparation_decisions += 1;
                *preparation_microsteps
                    .entry(row.replay_tick.microstep_count)
                    .or_default() += 1;
                if row
                    .patch
                    .pre_action()
                    .perception()
                    .candidates()
                    .iter()
                    .any(|c| c.target.entity.is_some())
                {
                    return Err("neural preparation unexpectedly observed a target".into());
                }
            }
        }
        for (id, position) in parked {
            place_stationary_fixture(host.world_mut(), id, position)?;
        }
    }
    // Begin hazard exposure with the first neural decision, after ordinary
    // maturation. Otherwise the unattended preparation itself kills the newborn.
    if scene == "hazard" {
        place_stationary_fixture(host.world_mut(), target, initial)?;
    } else if scene == "hazard-choice" {
        place_stationary_fixture(
            host.world_mut(),
            distraction_id.unwrap(),
            variation.rotate(Vec3f::new(0.25, 0., 0.)),
        )?;
    }
    let hunter = host.world().entity_id("hunter").unwrap();
    let initial_distance = distance(
        host.world().entity(hunter).unwrap().position,
        host.world().entity(target).unwrap().position,
    );
    let body = host
        .world()
        .organism_registry()
        .get(own)
        .unwrap()
        .biochemistry()
        .body;
    let initial_drives = host
        .world()
        .organism_registry()
        .get(own)
        .unwrap()
        .biochemistry()
        .homeostasis
        .drives;
    let mut result = Trial {
        seed,
        scene: scene.into(),
        consumed: false,
        caught_tick: None,
        consumed_tick: None,
        min_distance: initial_distance,
        final_distance: initial_distance,
        initial_energy: body.energy,
        final_energy: body.energy,
        initial_world_tick: host.world().tick().raw(),
        initial_drives,
        final_drives: initial_drives,
        sleeping_steps: 0,
        world_steps: 0,
        injury: 0.,
        alive: true,
        decisions: 0,
        preparation_decisions,
        preparation_microsteps,
        microstep_counts: Default::default(),
        failed_ingests: 0,
        no_manipulations: 0,
        contact_interactions: 0,
        actions: Default::default(),
        trace: Vec::new(),
        target_motion: Vec::new(),
    };
    for tick in 0..ticks {
        if tick == 64 && matches!(scene, "toy-then-food" | "wait-then-food" | "delayed-hazard") {
            if let Some(id) = distraction_id.take() {
                host.world_mut().editor_remove_object(id)?;
            }
            let pose = host.world().entity(hunter).unwrap().position;
            let offset = variation.rotate(Vec3f::new(0.25, 0., 0.));
            place_stationary_fixture(
                host.world_mut(),
                target,
                Vec3f::new(pose.x + offset.x, pose.y, pose.z + offset.z),
            )?;
        }
        let object = host.world().entity(target).unwrap();
        if !object.consumed && object.carried_by.is_none() {
            let t = tick as f32 * variation.speed;
            let next = match scene {
                "receding" => variation.rotate(Vec3f::new(initial_local.x + 0.012 * t, 0., 0.)),
                "crossing" | "moving-obstacle" => variation.rotate(Vec3f::new(
                    initial_local.x,
                    0.,
                    initial_local.z + mirror * 0.020 * t,
                )),
                "zigzag" => variation.rotate(Vec3f::new(
                    initial_local.x + 0.008 * t,
                    0.,
                    mirror * 0.4 * (t * 0.08).sin(),
                )),
                "orbiting" => variation.rotate(Vec3f::new(
                    initial_local.x - 0.7 + 0.7 * (t * 0.028).cos(),
                    0.,
                    mirror * 0.7 * (t * 0.028).sin(),
                )),
                "fleeing" => {
                    let hunter_position = host.world().entity(hunter).unwrap().position;
                    let d = distance(object.position, hunter_position).max(0.001);
                    let stride = 0.012 * variation.speed;
                    Vec3f::new(
                        object.position.x + (object.position.x - hunter_position.x) * stride / d,
                        object.position.y,
                        object.position.z + (object.position.z - hunter_position.z) * stride / d,
                    )
                }
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
        let objects_before = trace.then(|| [Some(target), distraction_id].into_iter().flatten().map(|id| {
            let o = host.world().entity(id).unwrap();
            json!({"id":id,"position":o.position,"carried_by":o.carried_by,"activity_hold":o.activity_hold,"consumed":o.consumed})
        }).collect::<Vec<_>>());
        let pain_before = trace.then(|| {
            host.world()
                .organism_registry()
                .get(own)
                .unwrap()
                .biochemistry()
                .homeostasis
                .drives
                .pain
        });
        let step = host.step(trainer, 1.)?;
        result.world_steps = tick + 1;
        result.sleeping_steps += step.sleeping_organisms;
        for row in step.decisions {
            result.decisions += 1;
            *result
                .microstep_counts
                .entry(row.replay_tick.microstep_count)
                .or_default() += 1;
            let frame = row.patch.pre_action().perception();
            for &index in row.action.motor_candidates.iter().flatten() {
                if let Some(c) = frame.candidates().get(index as usize) {
                    *result.actions.entry(format!("{:?}", c.family)).or_default() += 1;
                    if c.action_id == HeadlessActionIds::NO_MANIPULATION {
                        result.no_manipulations += 1;
                    } else if c.family == CandidateActionFamily::Contact {
                        result.contact_interactions += 1;
                    }
                    if c.family == CandidateActionFamily::Ingest && !row.patch.outcome().success {
                        result.failed_ingests += 1;
                    }
                }
            }
            if trace {
                let replay = trainer.evaluate_replay(&alife_training::TrainingSequence {
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
                result.trace.push(json!({"tick":tick,"action":row.action,"candidates":frame.candidates(),
                    "reward":row.biological_reward,"outcome":row.patch.outcome(),
                    "logits":replay.candidate_logits[0],"decoder_candidates":row.replay_tick.candidates,
                    "motor_activations": ([1760,1775,1776,1791,1824,1839].map(|i| row.final_features[i])),
                    "hold_motor_activation":row.final_features[1823],
                    "inspect_motor_activations":&row.final_features[1760..1776],
                    "action_motor_activations":&row.final_features[1728..1856],
                    "diagnostic_recurrent_weights":trainer.phenotype().synapses().iter().enumerate()
                        .filter(|(_,s)| s.kind()==CompiledSynapseKind::Recurrent && matches!(s.target(),1761|1809|1810))
                        .map(|(i,s)| json!({"index":i,"source":s.source(),"target":s.target(),
                            "genetic":s.genetic_weight(),"offset":row.replay_tick.effective_weight_offsets[i],
                            "effective":s.genetic_weight()+row.replay_tick.effective_weight_offsets[i]})).collect::<Vec<_>>(),
                    // One complete local replay boundary permits an independent
                    // microstep audit without dumping a full state every tick.
                    "replay_snapshot":(tick==32).then(|| json!({"initial":&row.initial,"tick":&row.replay_tick,
                        "expected_final":&replay.final_activations[0],"retained_final":&row.final_features})),
                    "need_relay_activations":&row.final_features[896..912],
                    "objects_before":objects_before,
                    "objects_after":([Some(target),distraction_id].into_iter().flatten().map(|id| {
                        let o=host.world().entity(id).unwrap();
                        json!({"id":id,"position":o.position,"carried_by":o.carried_by,"activity_hold":o.activity_hold,"consumed":o.consumed})
                    }).collect::<Vec<_>>()),
                    "pain_before":pain_before,
                    "pain_after":host.world().organism_registry().get(own).unwrap().biochemistry().homeostasis.drives.pain,
                    "position":host.world().entity(hunter).unwrap().position,
                    "steering":host.world().entity(hunter).unwrap().local_steering,
                    "body_yaw":host.world().entity(hunter).unwrap().body_yaw,
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
            result.consumed_tick.get_or_insert(tick);
            if !variation.continue_after_consumption {
                break;
            }
        }
        result.alive = host
            .world()
            .organism_registry()
            .get(own)
            .unwrap()
            .lifecycle()
            .is_alive();
        if !result.alive {
            break;
        }
    }
    result.alive = host
        .world()
        .organism_registry()
        .get(own)
        .unwrap()
        .lifecycle()
        .is_alive();
    let body = host
        .world()
        .organism_registry()
        .get(own)
        .unwrap()
        .biochemistry()
        .body;
    result.final_energy = body.energy;
    result.final_drives = host
        .world()
        .organism_registry()
        .get(own)
        .unwrap()
        .biochemistry()
        .homeostasis
        .drives;
    result.injury = body.injury;
    Ok(result)
}

fn run(base: &FoundationWeightAsset, p: &BrainPhenotype, r: Request) -> Result<serde_json::Value> {
    if !r.variation.angle.is_finite()
        || !r.variation.speed.is_finite()
        || !r.variation.distance_scale.is_finite()
        || !(0.5..=2.0).contains(&r.variation.distance_scale)
        || !r.variation.initial_energy.is_finite()
        || !(0.0..=1.0).contains(&r.variation.initial_energy)
        || r.variation
            .terrain
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        || !(0.0..=4.0).contains(&r.variation.speed)
        || r.variation.appearance > 3
        || r.seeds == 0
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
                "partial-obstacle",
                "moving-obstacle",
                "orbiting",
                "fleeing",
                "choice",
                "toy-then-food",
                "wait-then-food",
                "hazard",
                "delayed-hazard",
                "hazard-choice",
            ]
            .contains(&s.as_str())
        })
    {
        return Err("invalid trial dimensions or scene".into());
    }
    let start = std::time::Instant::now();
    let asset = scale_readouts(
        memory_candidate(candidate(base, p, r.params)?, p, r.memory_aversion)?,
        p,
        r.readout_scale,
        r.bearing_gain,
    )?;
    let asset = hold_candidate(asset, p, r.hold_readout)?;
    let asset = edit_weights(asset, p, &r.weight_edits)?;
    let (asset, compiled, template) = prepare_founder(asset, r.motor_heads)?;
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
            trials.push(trial(
                &template,
                &trainer,
                seed,
                scene,
                r.ticks,
                r.trace,
                r.variation,
            )?);
        }
    }
    if let Some(path) = r.export {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        f.write_all(&asset.encode_canonical()?)?;
        if r.motor_heads != 1 {
            let record = template.world.organism_registry().iter().next().unwrap();
            let mut packet = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(format!("{path}.genome.json"))?;
            packet.write_all(&serde_json::to_vec(record.genome())?)?;
        }
    }
    Ok(
        json!({"backend":"native-cpu", "memory_cap":trainer.phenotype().candidate_decoder().memory_channel().map(|m|m.max_candidate_gain()), "base_digest":base.digest(),"candidate_digest":asset.digest(),
        "assay_version":17,"fixture_placement":"stationary-teleport-v1","weight_edits":r.weight_edits,"hold_readout":r.hold_readout,"motor_heads":r.motor_heads,"variation":r.variation,"readout_scale":r.readout_scale,"bearing_gain":r.bearing_gain,"params":r.params,"memory_aversion":r.memory_aversion,"ticks":r.ticks,"seconds":start.elapsed().as_secs_f64(),"trials":trials}),
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
            json!({"construction_context":construction_context(&base)?,"lobe_layout":p.lobe_layout(),"sensor_encoder":p.sensor_encoder(),"neuron_dynamics":p.neuron_dynamics(),
            "projection_cadences":p.projections().iter().map(|r|r.update_cadence().raw()).collect::<Vec<_>>(),
            "microstep_count":p.microstep_count(),"weights":base.weights(),"synapses":p.synapses().iter().map(|s|
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
    fn sparse_weight_edits_roundtrip_without_changing_other_coordinates() {
        let base = base();
        let p = phenotype(&base).unwrap();
        assert_eq!(
            edit_weights(base.clone(), &p, &[])
                .unwrap()
                .encode_canonical()
                .unwrap(),
            base.encode_canonical().unwrap()
        );
        let edited = edit_weights(base.clone(), &p, &[(9552, 4.)]).unwrap();
        let restored =
            FoundationWeightAsset::decode_canonical(&edited.encode_canonical().unwrap()).unwrap();
        for (i, (before, after)) in base.weights().iter().zip(restored.weights()).enumerate() {
            assert_eq!(
                after.to_bits(),
                (if i == 9552 { before + 4. } else { *before }).to_bits()
            );
        }
        let compiled = phenotype(&restored).unwrap();
        assert!(compiled
            .synapses()
            .iter()
            .zip(restored.weights())
            .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
        assert!(edit_weights(base.clone(), &p, &[(usize::MAX, 1.)]).is_err());
        assert!(edit_weights(base.clone(), &p, &[(9552, f32::NAN)]).is_err());
        assert!(edit_weights(base, &p, &[(9552, f32::MAX), (9552, f32::MAX)]).is_err());
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
    fn readout_scaling_preserves_recurrent_speech_and_memory_weights() {
        let base = base();
        let p = phenotype(&base).unwrap();
        let edited = scale_readouts(base.clone(), &p, [0.5, 0.25], 8.).unwrap();
        let mut changes = 0;
        for ((syn, a), b) in p
            .synapses()
            .iter()
            .zip(base.weights())
            .zip(edited.weights())
        {
            if a.to_bits() != b.to_bits() {
                changes += 1;
                assert!(matches!(syn.kind(),CompiledSynapseKind::Decoder(c) if
                    c.head()==DecoderHeadKind::ActionCandidate && matches!(c.family(),CandidateActionFamily::Approach|CandidateActionFamily::Ingest)));
            }
        }
        assert!(changes > 0);
        assert!(phenotype(&edited)
            .unwrap()
            .synapses()
            .iter()
            .zip(edited.weights())
            .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
    }

    #[test]
    fn independent_manipulation_preserves_weights_and_reaches_native_sampler() {
        let original = base();
        let (asset, p, game) = prepare_founder(original.clone(), 2).unwrap();
        assert_eq!(asset.weights(), original.weights());
        assert_eq!(
            p.candidate_decoder().factorized_motor_channels(&p).unwrap(),
            vec![MotorChannel::Locomotion, MotorChannel::Manipulation]
        );
        let mask = StageTrainableMask::recurrent_only(&p).unwrap();
        let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
        let result = trial(
            &game,
            &trainer,
            2026112400,
            "contact",
            1,
            true,
            Variation::default(),
        )
        .unwrap();
        let masks = result.trace[0]["action"]["motor_masks"].as_array().unwrap();
        assert!(masks[0].as_u64().unwrap() > 0);
        assert!(masks[2].as_u64().unwrap() > 0);
        assert_eq!(masks[1], 0);
    }

    #[test]
    fn postmeal_continuation_keeps_the_first_meal_and_identical_prefix() {
        let parent = FoundationWeightAsset::decode_canonical(include_bytes!(
            "../../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation"
        ))
        .unwrap();
        let (asset, p, game) = prepare_founder(parent, 2).unwrap();
        let mask = StageTrainableMask::recurrent_only(&p).unwrap();
        let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
        let first_meal = trial(
            &game,
            &trainer,
            2026116100,
            "contact",
            64,
            true,
            Variation::default(),
        )
        .unwrap();
        assert!(first_meal.consumed && first_meal.world_steps < 64);
        let continued = trial(
            &game,
            &trainer,
            2026116100,
            "contact",
            64,
            true,
            Variation {
                continue_after_consumption: true,
                ..Variation::default()
            },
        )
        .unwrap();
        assert!(continued.consumed && continued.alive);
        assert_eq!(continued.world_steps, 64);
        assert_eq!(continued.consumed_tick, first_meal.consumed_tick);
        assert_eq!(
            &continued.trace[..first_meal.trace.len()],
            first_meal.trace.as_slice()
        );
        assert!(continued.trace.len() > first_meal.trace.len());
        assert!(continued.trace[first_meal.trace.len()..]
            .iter()
            .all(|row| row["objects_before"][0]["consumed"] == true));
    }

    #[test]
    fn hazard_exposure_begins_after_preparation_and_has_real_consequences() {
        let (asset, p, game) = prepare_founder(base(), 1).unwrap();
        let mask = StageTrainableMask::recurrent_only(&p).unwrap();
        let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
        let result = trial(
            &game,
            &trainer,
            2026112800,
            "hazard",
            3,
            false,
            Variation::default(),
        )
        .unwrap();
        assert!(result.decisions > 0);
        assert!(result.injury > 0.);
        assert!(!result.consumed);
    }

    #[test]
    fn stationary_fixture_placement_clears_teleport_velocity_only() {
        let mut world = HeadlessScenarioBuilder::new(2026116400).build().unwrap();
        let id = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "secondary hazard".into(),
                kind: WorldObjectKind::Hazard,
                organism_id: None,
                position: Vec3f::new(100., 0., 0.),
                nutrition: 0.,
                hazard_pain: 0.05,
                radius: 0.3,
                token_id: None,
            })
            .unwrap();
        let original = world.entity(id).unwrap().grounded_physical;
        place_stationary_fixture(&mut world, id, Vec3f::new(0.25, 0., 0.)).unwrap();
        for _ in 0..2 {
            world.try_advance_tick().unwrap();
            assert_eq!(world.entity(id).unwrap().position, Vec3f::new(0.25, 0., 0.));
            assert_eq!(world.entity(id).unwrap().grounded_physical, original);
        }
        // Real editor motion keeps its existing measured displacement. The
        // helper is used at placement boundaries, never as a per-tick reset.
        world
            .editor_move_object(id, Vec3f::new(0.5, 0., 0.))
            .unwrap();
        assert_eq!(
            world.entity(id).unwrap().grounded_physical.velocity,
            Vec3f::new(0.25, 0., 0.)
        );
    }

    #[test]
    fn secondary_hazard_and_warm_restoration_start_without_phantom_motion() {
        let (asset, p, game) = prepare_founder(base(), 2).unwrap();
        let mask = StageTrainableMask::recurrent_only(&p).unwrap();
        let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
        for (scene, warm) in [
            ("hazard-choice", false),
            ("hazard-choice", true),
            ("choice", true),
            ("obstacle", true),
        ] {
            let result = trial(
                &game,
                &trainer,
                2026116401,
                scene,
                1,
                true,
                Variation {
                    neural_preparation: warm,
                    initial_energy: 0.8,
                    ..Variation::default()
                },
            )
            .unwrap();
            let mut secondary_seen = false;
            for candidate in result.trace[0]["candidates"].as_array().unwrap() {
                if candidate["target"]["entity"] == 3 {
                    secondary_seen = true;
                    for lane in 3..=5 {
                        assert_eq!(candidate["features"][lane], 0.0, "{scene} warm={warm}");
                    }
                }
            }
            assert!(secondary_seen, "{scene} warm={warm}");
        }
    }

    #[test]
    fn neural_preparation_uses_real_empty_host_steps_at_the_same_world_age() {
        let (asset, p, game) = prepare_founder(base(), 2).unwrap();
        let mask = StageTrainableMask::recurrent_only(&p).unwrap();
        let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
        let cold = trial(
            &game,
            &trainer,
            2026115200,
            "hazard",
            2,
            false,
            Variation::default(),
        )
        .unwrap();
        let warm = trial(
            &game,
            &trainer,
            2026115200,
            "hazard",
            2,
            false,
            Variation {
                neural_preparation: true,
                ..Variation::default()
            },
        )
        .unwrap();
        assert_eq!(cold.preparation_decisions, 0);
        assert_eq!(warm.initial_world_tick, cold.initial_world_tick);
        assert!(warm.preparation_decisions > 0);
        assert_eq!(
            warm.preparation_microsteps.values().sum::<usize>(),
            warm.preparation_decisions
        );
        assert_eq!(
            warm.microstep_counts.values().sum::<usize>(),
            warm.decisions
        );
        assert_eq!(warm.initial_drives.pain, 0.0);
        assert!(warm.injury > 0.0);
    }

    #[test]
    fn delayed_hazard_allows_neural_activity_before_real_pain_arrives() {
        let original = base();
        let before_arrival = run(
            &original,
            &phenotype(&original).unwrap(),
            serde_json::from_value(json!({
                "params": [0., 0., 0., 0., 0., 0.], "motor_heads": 2,
                "seed_start": 2026114900_u64, "seeds": 1,
                "scenes": ["delayed-hazard"], "ticks": 64
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(
            before_arrival["trials"][0]["min_distance"]
                .as_f64()
                .unwrap()
                > 90.0
        );
        assert_eq!(before_arrival["trials"][0]["injury"], 0.0);
        let (asset, p, game) = prepare_founder(original, 2).unwrap();
        let mask = StageTrainableMask::recurrent_only(&p).unwrap();
        let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
        let result = trial(
            &game,
            &trainer,
            2026114900,
            "delayed-hazard",
            72,
            true,
            Variation::default(),
        )
        .unwrap();
        assert!(result.trace.iter().any(|r| r["tick"] == 64));
        for row in result
            .trace
            .iter()
            .filter(|r| r["tick"].as_u64().unwrap() < 64)
        {
            assert_eq!(row["pain_before"], 0.0);
            assert_eq!(row["pain_after"], 0.0);
            assert_eq!(row["objects_before"][0]["position"]["x"], 100.0);
        }
        assert!(result
            .trace
            .iter()
            .any(|r| r["tick"].as_u64().unwrap() >= 64 && r["pain_after"].as_f64().unwrap() > 0.0));
        assert!(result.injury > 0.0);
        assert!(!result.consumed);
    }

    #[test]
    fn delayed_food_trace_drops_the_removed_toy_without_changing_behavior() {
        let (asset, p, game) = prepare_founder(base(), 2).unwrap();
        let mask = StageTrainableMask::recurrent_only(&p).unwrap();
        let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
        let traced = trial(
            &game,
            &trainer,
            2026113900,
            "toy-then-food",
            72,
            true,
            Variation::default(),
        )
        .unwrap();
        assert!(traced.trace.iter().any(|r| r["tick"] == 64));
        for row in &traced.trace {
            let count = if row["tick"].as_u64().unwrap() < 64 {
                2
            } else {
                1
            };
            assert_eq!(row["objects_before"].as_array().unwrap().len(), count);
            assert_eq!(row["objects_after"].as_array().unwrap().len(), count);
        }
        let untraced = trial(
            &game,
            &trainer,
            2026113900,
            "toy-then-food",
            72,
            false,
            Variation::default(),
        )
        .unwrap();
        let mut traced = serde_json::to_value(traced).unwrap();
        let mut untraced = serde_json::to_value(untraced).unwrap();
        for value in [&mut traced, &mut untraced] {
            value.as_object_mut().unwrap().remove("trace");
            value.as_object_mut().unwrap().remove("target_motion");
        }
        assert_eq!(traced, untraced);
    }

    #[test]
    fn zero_speed_control_has_no_actual_target_motion_after_rotation() {
        let base = base();
        let p = phenotype(&base).unwrap();
        let mask = StageTrainableMask::recurrent_only(&p).unwrap();
        let trainer =
            CpuFoundationTrainer::new(p, base.clone(), mask, AdamWConfig::default()).unwrap();
        let template = create_canonical_new_game_with_n2048_experimental_candidate(
            &config(&base).unwrap(),
            &base,
        )
        .unwrap();
        for distance_scale in [1., 1.4] {
            let result = trial(
                &template,
                &trainer,
                2026111300,
                "receding",
                3,
                true,
                Variation {
                    angle: std::f32::consts::FRAC_PI_2,
                    speed: 0.,
                    appearance: 1,
                    distance_scale,
                    initial_energy: 0.25,
                    terrain: [0.5, 0.5],
                    neural_preparation: false,
                    continue_after_consumption: false,
                },
            )
            .unwrap();
            assert_eq!(result.target_motion.len(), 3);
            assert!(result.target_motion.iter().all(|r| r["translated"] == 0.0));
            assert!(
                (result.target_motion[0]["current"]["z"].as_f64().unwrap()
                    - f64::from(2.5 * distance_scale))
                .abs()
                    < 1e-6
            );
        }
    }

    #[test]
    fn edible_and_nonfood_controls_have_identical_initial_visible_features() {
        let base = base();
        let p = phenotype(&base).unwrap();
        let mask = StageTrainableMask::recurrent_only(&p).unwrap();
        let trainer =
            CpuFoundationTrainer::new(p, base.clone(), mask, AdamWConfig::default()).unwrap();
        let template = create_canonical_new_game_with_n2048_experimental_candidate(
            &config(&base).unwrap(),
            &base,
        )
        .unwrap();
        for terrain in [[0.5, 0.5], [0.25, 0.0], [0.0, 1.0]] {
            let variation = Variation {
                terrain,
                ..Variation::default()
            };
            let food = trial(
                &template, &trainer, 2026111300, "contact", 1, true, variation,
            )
            .unwrap();
            let toy = trial(&template, &trainer, 2026111300, "toy", 1, true, variation).unwrap();
            assert_eq!(food.trace[0]["candidates"], toy.trace[0]["candidates"]);
            for c in food.trace[0]["candidates"].as_array().unwrap() {
                if !c["target"]["entity"].is_null() {
                    assert_eq!(c["features"][15], 0.0);
                    assert_eq!(c["features"][22], terrain[0]);
                    assert_eq!(c["features"][23], terrain[1]);
                }
            }
        }
    }
}

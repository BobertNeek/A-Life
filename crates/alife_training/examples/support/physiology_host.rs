//! Disposable newborn physiology fixtures. No retained creature can enter here.
//! Chemical concentrations, never encoder values or action logits, are varied.
use super::{assay, dedicated, readout};
use alife_core::*;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask};
use alife_world::*;
use serde_json::{json, Value};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Arm {
    pub bounded: bool,
    pub dedicated_b: bool,
    pub tonic_bridges: bool,
}
pub fn arms() -> Vec<Arm> {
    [false, true]
        .into_iter()
        .flat_map(|bounded| {
            [false, true].into_iter().flat_map(move |dedicated_b| {
                [false, true].into_iter().map(move |tonic_bridges| Arm {
                    bounded,
                    dedicated_b,
                    tonic_bridges,
                })
            })
        })
        .collect()
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub enum Scene {
    Neutral,
    Food,
    Bitter,
    Object,
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Case {
    pub name: &'static str,
    pub scene: Scene,
    pub hunger: f32,
    pub fatigue: f32,
    pub curiosity: f32,
}
pub const CASES: [Case; 10] = [
    Case {
        name: "neutral_comfortable",
        scene: Scene::Neutral,
        hunger: 0.1,
        fatigue: 0.1,
        curiosity: 0.1,
    },
    Case {
        name: "neutral_fatigued",
        scene: Scene::Neutral,
        hunger: 0.1,
        fatigue: 0.65,
        curiosity: 0.1,
    },
    Case {
        name: "food_comfortable",
        scene: Scene::Food,
        hunger: 0.1,
        fatigue: 0.1,
        curiosity: 0.1,
    },
    Case {
        name: "food_hungry",
        scene: Scene::Food,
        hunger: 0.65,
        fatigue: 0.1,
        curiosity: 0.1,
    },
    Case {
        name: "bitter_comfortable",
        scene: Scene::Bitter,
        hunger: 0.1,
        fatigue: 0.1,
        curiosity: 0.1,
    },
    Case {
        name: "bitter_hungry",
        scene: Scene::Bitter,
        hunger: 0.65,
        fatigue: 0.1,
        curiosity: 0.1,
    },
    Case {
        name: "object_comfortable",
        scene: Scene::Object,
        hunger: 0.1,
        fatigue: 0.1,
        curiosity: 0.1,
    },
    Case {
        name: "object_curious",
        scene: Scene::Object,
        hunger: 0.1,
        fatigue: 0.1,
        curiosity: 0.65,
    },
    Case {
        name: "food_fatigued",
        scene: Scene::Food,
        hunger: 0.1,
        fatigue: 0.65,
        curiosity: 0.1,
    },
    Case {
        name: "bitter_fatigued",
        scene: Scene::Bitter,
        hunger: 0.1,
        fatigue: 0.65,
        curiosity: 0.1,
    },
];
/// Resolve each unique inherited biochemical receptor, then set its source
/// concentration in a newborn-only serde fixture and rederive cached drives.
/// This intentionally does not assert concentration == perceived drive.
pub fn chemistry(
    p: &CreaturePhenotype,
    before: BiochemistryState,
    case: Case,
    atp: f32,
) -> Result<BiochemistryState> {
    if before.tick != Tick::ZERO
        || before.development.age_ticks != Tick::ZERO
        || before.body.sleeping
    {
        return Err("only newborn chemistry fixtures are allowed".into());
    }
    let mut wire = serde_json::to_value(before)?;
    let mut assignments = vec![
        (BiochemicalDriveChannel::Hunger, case.hunger),
        (BiochemicalDriveChannel::Fatigue, case.fatigue),
        (BiochemicalDriveChannel::Fear, 0.),
        (BiochemicalDriveChannel::Pain, 0.),
        (BiochemicalDriveChannel::Loneliness, 0.1),
        (BiochemicalDriveChannel::Curiosity, case.curiosity),
        (BiochemicalDriveChannel::BrainAtp, atp),
        (BiochemicalDriveChannel::TemperatureStress, 0.),
    ];
    // Bound boredom is optional; reproductive readiness and unbound lanes are
    // never coerced into low/high distress. All hormones stay inherited.
    if p.chemistry
        .biochemical
        .receptors()
        .iter()
        .any(|r| r.target == BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Extension0))
    {
        assignments.push((BiochemicalDriveChannel::Extension0, 0.1));
    }
    for (channel, value) in assignments {
        let rows = p
            .chemistry
            .biochemical
            .receptors()
            .iter()
            .filter(|r| r.target == BiochemicalTargetLocus::Drive(channel))
            .collect::<Vec<_>>();
        if rows.len() != 1 || rows[0].digital || rows[0].gain <= 0. {
            return Err("fixture requires one positive analog inherited drive receptor".into());
        }
        let index = p
            .chemistry
            .biochemical
            .species()
            .iter()
            .position(|s| s.id == rows[0].source)
            .ok_or("drive source species missing")?;
        let species = p.chemistry.biochemical.species()[index];
        if !value.is_finite() || !(species.minimum..=species.maximum).contains(&value) {
            return Err("concentration outside inherited species bounds".into());
        }
        *wire
            .get_mut("graph_state")
            .and_then(|g| g.get_mut("concentrations"))
            .and_then(Value::as_array_mut)
            .and_then(|a| a.get_mut(index))
            .ok_or("newborn chemistry fixture schema changed")? = json!(value);
    }
    let mut next: BiochemistryState = serde_json::from_value(wire)?;
    next.body.set_energy(0.8)?;
    next.homeostasis = next
        .graph_state()
        .derive_homeostasis(&p.chemistry.biochemical)?;
    next.validate_against(p)?;
    assert_eq!(
        next.homeostasis,
        next.graph_state()
            .derive_homeostasis(&p.chemistry.biochemical)?
    );
    Ok(next)
}
pub fn tonic_factor(
    p: &BrainPhenotype,
    before: &[f32],
    enabled: bool,
) -> Result<(Vec<f32>, Value)> {
    let mut weights = before.to_vec();
    let mut rows = vec![];
    for (index, target) in [
        (7841, 896),
        (7842, 897),
        (7844, 899),
        (7845, 900),
        (7847, 902),
    ] {
        let s = p.synapses().get(index).ok_or("tonic edge missing")?;
        if (s.kind(), s.route_index(), s.source(), s.target())
            != (CompiledSynapseKind::Recurrent, 3, 1967, target)
            || before[index] != -4.
        {
            return Err("tonic coordinate or prior mismatch".into());
        }
        if !enabled {
            weights[index] = 0.;
        }
        rows.push(json!({"index":index,"source":1967,"target":target,"before":-4.,"after":weights[index]}));
    }
    let root = &p.synapses()[7679];
    if (
        root.kind(),
        root.route_index(),
        root.source(),
        root.target(),
    ) != (CompiledSynapseKind::Recurrent, 2, 383, 1967)
        || before[7679] != 8.
    {
        return Err("tonic root mismatch".into());
    }
    assert_eq!(
        weights
            .iter()
            .zip(before)
            .filter(|(a, b)| a.to_bits() != b.to_bits())
            .count(),
        if enabled { 0 } else { 5 }
    );
    Ok((
        weights,
        json!({"enabled":enabled,"root_edge_unchanged":7679,"root_weight":8.,"bridge_rows":rows,"source383_is_not_a_constant":true}),
    ))
}
pub struct Fixture {
    pub host: CpuCanonicalHost,
    pub trainer: CpuFoundationTrainer,
    pub organism: OrganismId,
    pub recipe: Value,
}
pub fn fixture(seed: u64, arm: Arm, case: Case) -> Result<Fixture> {
    let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    let builtin = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile)?;
    let original = create_canonical_new_game_with_n2048_candidate(&config, &builtin)?;
    let record = original
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("no founder")?;
    let genome = record.phenotype().brain_genome.clone();
    let development = alife_runtime::canonical_preparation::foundation_construction_development(
        &genome,
        &BrainCapacityClass::n2048(),
        &record.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    let p = if arm.bounded {
        PhenotypeCompiler::compile_n2048_bounded_founder(
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
    }
    .0;
    let (weights, circuits) = assay::frozen::whole_care(&p)?;
    let (weights, _) = readout::counterbalance(&p, &weights, &circuits, 0., true)?;
    let (weights, refs) = dedicated::full_reference_calibration(&p, &weights, &circuits, true)?;
    let factor = if arm.dedicated_b {
        dedicated::Factor::Dedicated
    } else {
        dedicated::Factor::Baseline
    };
    let (weights, b) = dedicated::build(&p, &weights, factor)?;
    let (weights, tonic) = tonic_factor(&p, &weights, arm.tonic_bridges)?;
    let asset = FoundationWeightAsset::from_trained_weights(
        &p,
        weights,
        TrainingStageManifest::bootstrap(),
    )?;
    let p = if arm.bounded {
        PhenotypeCompiler::compile_n2048_bounded_founder_candidate(
            genome,
            development,
            asset.clone(),
        )?
    } else {
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            genome,
            development,
            asset.clone(),
        )?
    }
    .0;
    asset.validate_against(&p)?;
    let mut game = create_canonical_new_game_with_n2048_experimental_candidate(&config, &asset)?;
    let old = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("no candidate founder")?
        .clone();
    let organism = old.organism_id();
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", organism, Vec3f::ZERO)
        .build()?;
    let target = match case.scene {
        Scene::Neutral => None,
        Scene::Object => Some(world.spawn_toy("target", Vec3f::new(2.5, 0., 0.), true)?),
        Scene::Food | Scene::Bitter => Some(world.editor_spawn_object(WorldEditorSpawnSpec {
            label: "target".into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: Vec3f::new(0.25, 0., 0.),
            nutrition: 0.6,
            hazard_pain: 0.,
            radius: 0.5,
            token_id: None,
        })?),
    };
    if let Some(id) = target {
        let mut physical = world
            .entity(id)
            .ok_or("target disappeared")?
            .grounded_physical;
        physical.chemical[0] = match case.scene {
            Scene::Food => 0.85,
            Scene::Bitter => -0.85,
            _ => 0.,
        };
        world.set_grounded_physical_properties(id, physical)?;
    }
    let biology = chemistry(old.phenotype(), *old.biochemistry(), case, 0.75)?;
    world.register_organism_record(WorldOrganismRecord::new(
        organism,
        world.entity_id("learner").ok_or("learner missing")?,
        old.genome().clone(),
        old.phenotype().clone(),
        biology,
        Tick::ZERO,
    )?)?;
    game.world = world;
    let mask = StageTrainableMask::recurrent_only(&p)?;
    let trainer =
        CpuFoundationTrainer::new(p.clone(), asset.clone(), mask, AdamWConfig::default())?;
    let host = CpuCanonicalHost::new_cohort(game, &trainer, (seed ^ (seed >> 32)) as u32)?;
    Ok(Fixture {
        host,
        trainer,
        organism,
        recipe: json!({"arm":arm,"case":case,"seed":seed,"asset_digest":asset.digest(),"manifest":asset.manifest(),"phenotype_hash":p.phenotype_hash(),"full_reference_common":refs,"dedicated_b":b,"tonic":tonic,"contact_prior":"B1 retained; B2 not silently adopted","target_entity":target,"fresh_chemistry":biology,"learned_state_imported":false}),
    })
}

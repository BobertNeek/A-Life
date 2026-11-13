//! Matched mixed toy→food lives on the exact exported pursuit-memory16 parent.
//! No action forcing, target motion, teleport, neural reset or fabricated reward.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{AdamWConfig, StageTrainableMask, cpu::*};
use alife_world::*;
use serde::Deserialize;
use serde_json::json;
use std::io::{BufRead, Write};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    seed: u64,
    ticks: usize,
    leadin: usize,
    arm: String,
    gain: f32,
    reverse_appearance: bool,
    alias: bool,
}
fn config(asset: &FoundationWeightAsset) -> Result<CanonicalNewGameConfig> {
    let mut c = CanonicalNewGameConfig::phase3(2026111701, 1)?;
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
fn spawn(
    world: &mut HeadlessWorld,
    label: &str,
    food: bool,
    position: Vec3f,
    appearance: bool,
) -> Result<WorldEntityId> {
    let id = world.editor_spawn_object(WorldEditorSpawnSpec {
        label: label.into(),
        kind: if food {
            WorldObjectKind::Food
        } else {
            WorldObjectKind::ActivityToy
        },
        organism_id: None,
        position,
        nutrition: if food { 0.6 } else { 0. },
        hazard_pain: 0.,
        radius: 0.3,
        token_id: None,
    })?;
    world.set_grounded_physical_properties(
        id,
        GroundedPhysicalProperties {
            velocity: Vec3f::ZERO,
            color: if appearance {
                [0.75, 0.25, 0.75]
            } else {
                [0.2, 0.7, 0.1]
            },
            material: [0.2; 3],
            shape: if appearance {
                [0.25, 0.75, 0.25]
            } else {
                [0.7, 0.2, 0.4]
            },
            chemical: [0.; 3],
            surface_temperature: 0.,
            terrain: [0.5; 2],
        },
    )?;
    Ok(id)
}
fn run(asset: &FoundationWeightAsset, request: Request) -> Result<serde_json::Value> {
    if !["off", "active", "zero", "mask"].contains(&request.arm.as_str())
        || !(1..=512).contains(&request.ticks)
        || request.leadin > 2400
    {
        return Err("invalid bounded assay request".into());
    }
    verify_terminal_parent2135(asset)?;
    let p = phenotype(asset)?;
    let mask = StageTrainableMask::recurrent_only(&p)?;
    let trainer = CpuFoundationTrainer::new(p, asset.clone(), mask, AdamWConfig::default())?;
    assert_eq!(trainer.weights(), asset.weights());
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
    let mut world = HeadlessScenarioBuilder::new(request.seed)
        .agent("hunter", own, Vec3f::ZERO)
        .build()?;
    let toy_appearance = request.reverse_appearance;
    let _toy = spawn(
        &mut world,
        "toy",
        false,
        Vec3f::new(0.25, 0., 0.),
        toy_appearance,
    )?;
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
    let (mut host, _) =
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, request.seed as u32, |w| {
            for _ in 0..request.leadin {
                w.try_advance_tick()?;
            }
            Ok::<_, ScaffoldContractError>(())
        })?;
    if request.arm != "off" {
        let mut c = TerminalContextConfig2135::new(if request.arm == "zero" {
            0.
        } else {
            request.gain
        })?;
        c.mask_new_context = request.arm == "mask";
        host.enable_terminal_context2135(c, &trainer)?;
    }
    let first_hunger = host
        .world()
        .organism_registry()
        .get(own)
        .unwrap()
        .biochemistry()
        .homeostasis
        .drives
        .hunger;
    let mut hunger_min = first_hunger;
    let mut hunger_max = first_hunger;
    let mut food = Vec::new();
    let mut meals = 0;
    let mut failed_ingest_bundles = 0;
    let mut decisions = 0;
    let mut context_rows = 0;
    let mut sourced_rows = 0;
    let mut deltas = 0;
    let mut work = 0u64;
    let mut meal_hunger_deltas = Vec::new();
    let mut capacity_rejected = 0;
    for tick in 0..request.ticks {
        // Prespecified environmental arrivals; never contingent on the selected action.
        if [64, 128, 192].contains(&tick) {
            let j = food.len();
            let positions = [
                Vec3f::new(2.5, 0., 1.),
                Vec3f::new(0., 0., 2.5),
                Vec3f::new(-2.5, 0., 1.),
            ];
            food.push(spawn(
                host.world_mut(),
                &format!("food-{j}"),
                true,
                positions[j],
                if request.alias {
                    toy_appearance
                } else {
                    !toy_appearance
                },
            )?);
        }
        let step = host.step(&trainer, 1.)?;
        for row in step.decisions {
            decisions += 1;
            let frame = row.patch.pre_action().perception();
            let h = frame.homeostasis().drives.hunger;
            hunger_min = hunger_min.min(h);
            hunger_max = hunger_max.max(h);
            if row.action.motor_candidates.iter().flatten().any(|&i| {
                frame.candidates()[usize::from(i)].family == CandidateActionFamily::Ingest
            }) && !row.patch.outcome().success
            {
                failed_ingest_bundles += 1;
            }
            if row.patch.outcome().physical.contact == PhysicalContactKind::Consumed {
                meals += 1;
                meal_hunger_deltas.push(row.patch.outcome().homeostatic_delta.drives.hunger);
            }
            if let Some(r) = row.terminal_context2135 {
                context_rows += 1;
                work += r.reserved_work_units;
                capacity_rejected += usize::from(r.capacity_rejected);
                sourced_rows += r
                    .candidates
                    .iter()
                    .filter(|c| c.consumed_source_count > 0)
                    .count();
                deltas += r.candidates.iter().filter(|c| c.delta != 0.).count();
            }
        }
    }
    let cp = host.checkpoint(&trainer)?;
    let bytes = serde_json::to_vec(&cp)?;
    let restored = CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes)?, &trainer)?;
    assert_eq!(bytes, serde_json::to_vec(&restored.checkpoint(&trainer)?)?);
    assert_eq!(trainer.weights(), asset.weights());
    let consumed_food = food
        .iter()
        .filter(|id| host.world().entity(**id).unwrap().consumed)
        .count();
    Ok(
        json!({"schema":2135,"parent_sha256":"8f84afd6ac2ec575a8df61d18a0a18ac17296615a9ef7af7d22df416b3dd0ad7","seed":request.seed,"arm":request.arm,"gain":request.gain,"ticks":request.ticks,"leadin":request.leadin,"reverse_appearance":request.reverse_appearance,"alias":request.alias,"first_hunger":first_hunger,"hunger_min":hunger_min,"hunger_max":hunger_max,"meals":meals,"consumed_food":consumed_food,"spawned_food":food.len(),"meal_hunger_deltas":meal_hunger_deltas,"failed_ingest_containing_bundles":failed_ingest_bundles,"decisions":decisions,"context_rows":context_rows,"sourced_candidates":sourced_rows,"nonzero_deltas":deltas,"reserved_work_units":work,"capacity_rejected":capacity_rejected,"typed_cold_exact":true,"parent_weights_exact":true,"head_fixed_in_life":true,"long_ppo_calls":0}),
    )
}
fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or("exact pursuit-memory16 asset required")?;
    let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(path)?)?;
    verify_terminal_parent2135(&asset)?;
    for line in std::io::stdin().lock().lines() {
        let r = serde_json::from_str::<Request>(&line?)
            .map_err(Into::into)
            .and_then(|q| run(&asset, q));
        match r {
            Ok(v) => println!("{v}"),
            Err(e) => println!("{}", json!({"error":e.to_string()})),
        };
        std::io::stdout().flush()?;
    }
    Ok(())
}

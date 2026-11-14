//! Matched mixed toy→food lives on an exact exported two-head asset+genome pair.
//! No action forcing, target motion, teleport, neural reset or fabricated reward.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask};
use alife_world::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
#[path = "support/terminal_evidence2135.rs"]
mod evidence;
#[path = "support/terminal_trace2135.rs"]
mod trace;
use evidence::LocalEvidence;
use serde_json::json;
use std::io::{BufRead, Write};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Request {
    parent: String,
    evidence_id: String,
    seed: u64,
    ticks: usize,
    leadin: usize,
    arm: String,
    gain: f32,
    reverse_appearance: bool,
    alias: bool,
    #[serde(default)]
    trace: bool,
    /// Environmental diet control; absent retains the original food schedule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    food_nutrition: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    decision_target: Option<usize>,
}
fn config(asset: &FoundationWeightAsset) -> Result<CanonicalNewGameConfig> {
    let mut c = CanonicalNewGameConfig::phase3(2026111701, 1)?;
    c.brain_class = BrainScaleTier::Standard2048;
    c.sensor_profile = asset.manifest().sensor_profile();
    Ok(c)
}
// Load the exported construction genome directly. No new chromosome mutation,
// weight rebinding or reconstruction of a different founder is performed.
fn prepare_parent(
    asset: &FoundationWeightAsset,
    genome: &CreatureGenome,
) -> Result<(BrainPhenotype, CanonicalNewGame)> {
    let embedded = genome
        .n2048_foundation_candidate
        .as_ref()
        .ok_or("parent genome lacks foundation")?;
    if embedded.encode_canonical()? != asset.encode_canonical()? {
        return Err("genome/asset pair mismatch".into());
    }
    let mut game =
        create_canonical_new_game_with_n2048_experimental_candidate(&config(asset)?, asset)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let expressed = genome.express()?;
    let dev = foundation_construction_development(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &expressed.development_state_at(Tick::ZERO)?,
    )?;
    let (compiled, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        expressed.brain_genome.clone(),
        dev,
        asset.clone(),
    )?;
    TerminalContextConfig2135::for_parent(asset, genome, &compiled, 0.0)?;
    if !compiled
        .synapses()
        .iter()
        .zip(asset.weights())
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits())
    {
        return Err("compiled parent weights changed".into());
    }
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
        PortableAssetDigest::for_bytes(&serde_json::to_vec(genome)?).0;
    Ok((compiled, game))
}
fn spawn(
    world: &mut HeadlessWorld,
    label: &str,
    food: bool,
    position: Vec3f,
    appearance: bool,
    nutrition: f32,
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
        nutrition: if food { nutrition } else { 0. },
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
fn run(
    asset: &FoundationWeightAsset,
    genome: &CreatureGenome,
    request: &Request,
    evidence: &mut LocalEvidence,
) -> Result<serde_json::Value> {
    if request.parent.is_empty()
        || request.parent.len() > 120
        || !["off", "active", "zero", "mask"].contains(&request.arm.as_str())
        || !(1..=512).contains(&request.ticks)
        || request.leadin > 2400
        || request
            .decision_target
            .is_some_and(|n| n == 0 || n > request.ticks)
        || request
            .food_nutrition
            .is_some_and(|n| !n.is_finite() || !(0.0..=0.6).contains(&n))
    {
        return Err("invalid bounded assay request".into());
    }
    let (p, mut game) = prepare_parent(asset, genome)?;
    let mask = StageTrainableMask::recurrent_only(&p)?;
    let trainer = CpuFoundationTrainer::new(p, asset.clone(), mask, AdamWConfig::default())?;
    assert_eq!(trainer.weights(), asset.weights());
    let founder_bytes = serde_json::to_vec(&trainer.checkpoint())?;
    evidence.save("FOUNDER-OPTIMIZER-BIRTH.json", &founder_bytes)?;
    evidence.save("PROVENANCE.json",&serde_json::to_vec_pretty(&json!({
        "schema":2135,"revision":"v6-local-physical-trace","parent_sha256":format!("{:x}",Sha256::digest(asset.encode_canonical()?)),
        "parent_native_digest":asset.digest(),"parent_manifest":asset.manifest(),
        "construction_genome_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(genome)?)),
        "motor_head_count":trainer.phenotype().cognitive_architecture().motor_head_count(),
        "motor_channels":trainer.phenotype().candidate_decoder().factorized_motor_channels(trainer.phenotype())?,
        "compiler_inputs_digest":trainer.phenotype().compiler_inputs_digest(),
        "founder_seed":2026111701u64,"genome_and_development":"preserved in full native CP world record",
        "compiled_phenotype_hash":trainer.phenotype().phenotype_hash(),
        "source_hashes":{"assay":format!("{:x}",Sha256::digest(include_bytes!("terminal_context2135.rs"))),
            "evidence":format!("{:x}",Sha256::digest(include_bytes!("support/terminal_evidence2135.rs"))),
            "head":format!("{:x}",Sha256::digest(include_bytes!("../src/cpu/terminal_head.rs"))),
            "channel_state":format!("{:x}",Sha256::digest(include_bytes!("../src/cpu/terminal_observation.rs"))),
            "channel_proof":format!("{:x}",Sha256::digest(include_bytes!("../../alife_core/src/memory/channel_terminal.rs"))),
            "trace":format!("{:x}",Sha256::digest(include_bytes!("support/terminal_trace2135.rs")))},
        "local_only":true,"automatic_retry":false,"compile_commit":option_env!("ALIFE_ASSAY_GIT_COMMIT")
    }))?)?;
    evidence.save(
        "PARENT-CONSTRUCTION-GENOME.json",
        &serde_json::to_vec(genome)?,
    )?;
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
    let toy = spawn(
        &mut world,
        "toy",
        false,
        Vec3f::new(0.25, 0., 0.),
        toy_appearance,
        0.0,
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
        let mut c = TerminalContextConfig2135::for_parent(
            asset,
            genome,
            trainer.phenotype(),
            if request.arm == "zero" {
                0.
            } else {
                request.gain
            },
        )?;
        c.mask_new_context = request.arm == "mask";
        host.enable_terminal_context2135(c, &trainer)?;
    }
    let birth = host.checkpoint(&trainer)?;
    let birth_bytes = serde_json::to_vec(&birth)?;
    evidence.remember_valid(birth_bytes.clone());
    evidence.save("PREPARED-BIRTH.cp.json", &birth_bytes)?;
    evidence.save(
        "RECIPE.json",
        &serde_json::to_vec_pretty(&birth.terminal_context2135)?,
    )?;
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
    let mut insufficient_atp = 0;
    let mut unfunded_admission_checks = 0u64;
    let mut first_informative_saved = false;
    let mut elapsed_host_intervals = 0;
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
                request.food_nutrition.unwrap_or(0.6),
            )?);
        }
        // Retain the complete valid pre-step state after any declared arrival.
        // This is evaluator I/O, not a creature input or additional decision.
        evidence.remember_valid(serde_json::to_vec(&host.checkpoint(&trainer)?)?);
        let step = host.step(&trainer, 1.)?;
        // Capture immediately after a committed step, before event serialization.
        let committed = host.checkpoint(&trainer)?;
        let committed_bytes = serde_json::to_vec(&committed)?;
        evidence.remember_valid(committed_bytes.clone());
        if request.trace {
            trace::save(
                evidence,
                tick,
                &step,
                &committed,
                &committed_bytes,
                toy,
                &food,
            )?;
        }
        for row in step.decisions {
            decisions += 1;
            let consumed = row.patch.outcome().joint.as_ref().is_some_and(|j| {
                j.channel_outcomes.iter().any(|c| {
                    c.channel == MotorChannel::Manipulation
                        && c.physical.contact == PhysicalContactKind::Consumed
                })
            });
            let informative = row.terminal_context2135.as_ref().is_some_and(|r| {
                r.candidates.iter().any(|c| {
                    c.consumed_source_count > 0 && c.desirability.is_some_and(|v| v != 0.0)
                })
            });
            if consumed || (informative && !first_informative_saved) {
                let name = format!("EVENT-{tick:04}-{decisions:04}");
                evidence.save(&format!("{name}.cp.json"), &committed_bytes)?;
                evidence.save(&format!("{name}.json"),&serde_json::to_vec(&json!({
                    "consumed":consumed,"first_informative_context":informative && !first_informative_saved,
                    "host_interval":tick,"decision":&row,"checkpoint":format!("{name}.cp.json")
                }))?)?;
                if informative {
                    first_informative_saved = true;
                }
            }
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
            if consumed {
                meals += 1;
                meal_hunger_deltas.push(row.patch.outcome().homeostatic_delta.drives.hunger);
            }
            if let Some(r) = row.terminal_context2135 {
                context_rows += 1;
                insufficient_atp += usize::from(matches!(
                    r.admission,
                    TerminalAdmission2135::InsufficientAtp { .. }
                ));
                unfunded_admission_checks += u64::from(r.admission_check_unfunded_atp_q16);
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
        elapsed_host_intervals += 1;
        if request
            .decision_target
            .is_some_and(|target| decisions >= target)
        {
            break;
        }
    }
    let cp = host.checkpoint(&trainer)?;
    let bytes = serde_json::to_vec(&cp)?;
    evidence.remember_valid(bytes.clone());
    evidence.save("FINAL.cp.json", &bytes)?;
    evidence.save(
        "FINAL-RECIPE.json",
        &serde_json::to_vec_pretty(&cp.terminal_context2135)?,
    )?;
    let final_founder = serde_json::to_vec(&trainer.checkpoint())?;
    evidence.save("FOUNDER-OPTIMIZER-FINAL.json", &final_founder)?;
    assert_eq!(founder_bytes, final_founder);
    let restored = CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes)?, &trainer)?;
    assert_eq!(bytes, serde_json::to_vec(&restored.checkpoint(&trainer)?)?);
    assert_eq!(trainer.weights(), asset.weights());
    let consumed_food = food
        .iter()
        .filter(|id| host.world().entity(**id).unwrap().consumed)
        .count();
    Ok(
        json!({"schema":2135,"parent_sha256":format!("{:x}",Sha256::digest(asset.encode_canonical()?)),"construction_genome_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(genome)?)),"motor_head_count":2,"seed":request.seed,"arm":request.arm,"gain":request.gain,"ticks":request.ticks,"elapsed_host_intervals":elapsed_host_intervals,"decision_target":request.decision_target,"leadin":request.leadin,"reverse_appearance":request.reverse_appearance,"alias":request.alias,"food_nutrition":request.food_nutrition.unwrap_or(0.6),"first_hunger":first_hunger,"hunger_min":hunger_min,"hunger_max":hunger_max,"meals":meals,"consumed_food":consumed_food,"spawned_food":food.len(),"meal_hunger_deltas":meal_hunger_deltas,"failed_ingest_containing_bundles":failed_ingest_bundles,"decisions":decisions,"context_rows":context_rows,"sourced_candidates":sourced_rows,"nonzero_deltas":deltas,"reserved_work_units":work,"capacity_rejected":capacity_rejected,"insufficient_atp":insufficient_atp,"unfunded_admission_checks_atp_q16":unfunded_admission_checks,"typed_cold_exact":true,"parent_weights_exact":true,"head_fixed_in_life":true,"long_ppo_calls":0}),
    )
}
fn execute_request(
    asset: &FoundationWeightAsset,
    genome: &CreatureGenome,
    root: &Path,
    request: &Request,
) -> Result<serde_json::Value> {
    // Existing directories reject before any simulation, never resume/retry implicitly.
    let mut evidence = LocalEvidence::begin(root, &request.evidence_id)?;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || -> Result<serde_json::Value> {
            evidence.save("REQUEST.json", &serde_json::to_vec_pretty(request)?)?;
            let summary = run(asset, genome, request, &mut evidence)?;
            evidence.save(
                "COMPLETED.json",
                &serde_json::to_vec_pretty(&json!({
                    "completed":true,"summary":&summary,"final_checkpoint":"FINAL.cp.json",
                    "founder_optimizer":"FOUNDER-OPTIMIZER-FINAL.json","automatic_retry":false
                }))?,
            )?;
            Ok(summary)
        },
    ));
    let failure = match outcome {
        Ok(Ok(summary)) => return Ok(summary),
        Ok(Err(error)) => error.to_string(),
        Err(_) => "assay panic; inspect retained native state; no automatic retry".into(),
    };
    let retention = evidence.retain_error_checkpoint();
    let receipt = json!({"completed":false,"error":failure,"automatic_retry":false,
        "last_valid_checkpoint_saved":retention.as_ref().is_ok_and(|saved| *saved),
        "checkpoint_retention_error":retention.as_ref().err().map(ToString::to_string),
        "last_valid_checkpoint":if retention.as_ref().is_ok_and(|saved| *saved){Some("ERROR-LAST-VALID.cp.json")}else{None}});
    let receipt_error = evidence
        .save("ERROR.json", &serde_json::to_vec_pretty(&receipt)?)
        .err();
    Err(format!("{failure}; LOCAL evidence: {}; retention: {retention:?}; error receipt write: {receipt_error:?}",evidence.directory().display()).into())
}
fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or("exact parent foundation required")?;
    let genome_path = std::env::args()
        .nth(2)
        .ok_or("exact exported construction genome required")?;
    let root = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "terminal-context2135-LOCAL".into());
    let asset_bytes = std::fs::read(path)?;
    let genome_bytes = std::fs::read(genome_path)?;
    let asset = FoundationWeightAsset::decode_canonical(&asset_bytes)?;
    let genome: CreatureGenome = serde_json::from_slice(&genome_bytes)?;
    for line in std::io::stdin().lock().lines() {
        let r = serde_json::from_str::<Request>(&line?)
            .map_err(Into::into)
            .and_then(|q| execute_request(&asset, &genome, Path::new(&root), &q));
        match r {
            Ok(v) => println!("{v}"),
            Err(e) => println!("{}", json!({"error":e.to_string(),"automatic_retry":false})),
        };
        std::io::stdout().flush()?;
    }
    Ok(())
}

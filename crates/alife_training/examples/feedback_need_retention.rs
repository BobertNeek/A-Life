//! LOCAL-only Contact20 need assay after 600 prospectively fixed native host steps.
//! Diagnostic interventions never enter the host or teach an action.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask, TrainingSequence};
use alife_world::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
#[path = "support/terminal_evidence2135.rs"]
mod evidence;
use evidence::LocalEvidence;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn bytes<T: serde::Serialize>(x: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(x)?)
}
fn max_delta(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0., f32::max)
}
fn norm(x: &[f32]) -> f64 {
    x.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>().sqrt()
}
fn physiology(h: &CpuCanonicalHost, id: OrganismId) -> Value {
    let r = h.world().organism_registry().get(id).unwrap();
    json!({"biology_tick":r.biochemistry().tick,"homeostasis":r.biochemistry().homeostasis,
        "body":r.biochemistry().body,"sleep_phase":r.sleep_phase(),"alive":r.lifecycle().is_alive()})
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
    // Verify that carrying the canonical newborn biology changes no biology.
    assert_eq!(
        bytes(record.biochemistry())?,
        bytes(&BiochemistryState::new_with_age(
            &expressed,
            record.birth_tick(),
            Tick::ZERO
        )?)?
    );
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

fn setup(
    asset: &FoundationWeightAsset,
    genome: &CreatureGenome,
    seed: u64,
) -> Result<(CpuCanonicalHost, CpuFoundationTrainer, OrganismId)> {
    let (p, mut game) = prepare_parent(asset, genome)?;
    assert_eq!(p.cognitive_architecture().motor_head_count(), 2);
    let mask = StageTrainableMask::recurrent_only(&p)?;
    let trainer = CpuFoundationTrainer::new(p, asset.clone(), mask, AdamWConfig::default())?;
    let old = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let id = old.organism_id();
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("subject", id, Vec3f::ZERO)
        .build()?;
    world.register_organism_record(WorldOrganismRecord::new(
        id,
        world.entity_id("subject").unwrap(),
        old.genome().clone(),
        old.phenotype().clone(),
        *old.biochemistry(),
        old.birth_tick(),
    )?)?;
    game.world = world;
    Ok((
        CpuCanonicalHost::new_cohort(game, &trainer, seed as u32)?,
        trainer,
        id,
    ))
}
fn run(
    asset: &FoundationWeightAsset,
    genome: &CreatureGenome,
    seed: u64,
    ticks: usize,
    e: &mut LocalEvidence,
) -> Result<Value> {
    let (mut base, trainer, id) = setup(asset, genome, seed)?;
    e.save("FOUNDER-ADAM-BIRTH.json", &bytes(&trainer.checkpoint())?)?;
    let founder = bytes(&trainer.checkpoint())?;
    let p = trainer.phenotype();
    let edges = p
        .synapses()
        .iter()
        .map(|s| match s.kind() {
            CompiledSynapseKind::Recurrent => json!([
                s.source(),
                s.target(),
                s.route_index(),
                0,
                0,
                0,
                0,
                s.alpha()
            ]),
            CompiledSynapseKind::Decoder(c) => json!([
                s.source(),
                s.target(),
                s.route_index(),
                c.head().raw(),
                c.family().raw(),
                c.input_lane(),
                0,
                s.alpha()
            ]),
        })
        .collect::<Vec<_>>();
    e.save("GRAPH-LOCAL.json",&bytes(&json!({"weights":trainer.weights(),"edges":edges,
        "dynamics":p.neuron_dynamics(),"cadence":p.projections().iter().map(|r|r.update_cadence().raw()).collect::<Vec<_>>(),
        "biases":p.candidate_decoder().families().iter().map(|f|(f.family().raw(),f.bias())).collect::<Vec<_>>(),
        "microsteps":p.microstep_count()}))?)?;
    e.save("GENOME.json", &bytes(genome)?)?;
    // Prospectively fixed duration. No need threshold changes its endpoint.
    const LEADIN: usize = 600;
    e.save("LEADIN-BIRTH.cp.json", &bytes(&base.checkpoint(&trainer)?)?)?;
    let leadin_start = physiology(&base, id);
    let body_id = base
        .world()
        .organism_registry()
        .get(id)
        .unwrap()
        .world_entity_id();
    let start_position = base.world().entity(body_id).unwrap().position;
    let mut previous_position = start_position;
    let mut total_movement = 0.0_f64;
    let mut leadin_sleeping = 0;
    let mut leadin_consolidations = 0;
    let mut leadin_decisions = 0;
    let mut leadin_actions = std::collections::BTreeMap::<String, usize>::new();
    let mut leadin_trace = Vec::new();
    let mut leadin_twin = base.snapshot();
    for i in 0..LEADIN {
        let s = base.step(&trainer, 1.)?;
        leadin_twin.step(&trainer, 1.)?;
        e.save(&format!("LEADIN-{i:04}.rows.json"), &bytes(&s.decisions)?)?;
        leadin_sleeping += s.sleeping_organisms;
        leadin_consolidations += s.sleep_consolidations;
        leadin_decisions += s.decisions.len();
        for row in &s.decisions {
            let family = format!(
                "{:?}",
                row.replay_tick.candidates[row.action.representative as usize].family
            );
            *leadin_actions.entry(family).or_default() += 1;
        }
        let position = base.world().entity(body_id).unwrap().position;
        total_movement += f64::from(
            (position.x - previous_position.x).powi(2)
                + (position.y - previous_position.y).powi(2)
                + (position.z - previous_position.z).powi(2),
        )
        .sqrt();
        previous_position = position;
        leadin_trace.push(
            json!({"tick":s.tick_after,"physiology":physiology(&base,id),
            "sleeping":s.sleeping_organisms,"consolidations":s.sleep_consolidations,
            "decisions":s.decisions.len(),"movement_total":total_movement}),
        );
        if (i + 1).is_multiple_of(60) {
            let cp = bytes(&base.checkpoint(&trainer)?)?;
            assert_eq!(cp, bytes(&leadin_twin.checkpoint(&trainer)?)?);
            e.save(&format!("LEADIN-{:04}.cp.json", i + 1), &cp)?;
            e.remember_valid(cp);
            let bio = base
                .world()
                .organism_registry()
                .get(id)
                .unwrap()
                .biochemistry();
            eprintln!("leadin tick={} hunger={:.6} loneliness={:.6} energy={:.6} maturity={:.6} movement={:.6}",
                s.tick_after.raw(),bio.homeostasis.drives.hunger,bio.homeostasis.drives.loneliness,
                bio.body.energy,bio.development.maturation,total_movement);
        }
    }
    e.save("LEADIN-TRACE.json", &bytes(&leadin_trace)?)?;
    let need_boundary = physiology(&base, id);
    let actual_bio = base
        .world()
        .organism_registry()
        .get(id)
        .unwrap()
        .biochemistry();
    let qualified = actual_bio.homeostasis.drives.loneliness > 1.0e-4;
    let leadin_summary = json!({"host_steps":LEADIN,"start":leadin_start,"end":need_boundary,
        "decisions":leadin_decisions,"sleeping_steps":leadin_sleeping,"consolidations":leadin_consolidations,
        "actions":leadin_actions,"total_movement":total_movement,
        "maturation":actual_bio.development.maturation,"biochemical_expression":actual_bio.development.biochemical_expression,
        "loneliness_measurable_threshold":1.0e-4,"loneliness_qualified":qualified,
        "observer_noninterference_full_cp_every_60":true});
    e.save("LEADIN-SUMMARY.json", &bytes(&leadin_summary)?)?;
    // One ordinary fresh food target, introduced after the fixed endpoint.
    // Spawning a new target does not move the body or any existing object.
    let food = base.world_mut().editor_spawn_object(WorldEditorSpawnSpec {
        label: "fresh-food-after-fixed-leadin".into(),
        kind: WorldObjectKind::Food,
        organism_id: None,
        position: Vec3f::new(
            previous_position.x + 2.5,
            previous_position.y,
            previous_position.z + if seed.is_multiple_of(2) { 0.5 } else { -0.5 },
        ),
        nutrition: 0.6,
        hazard_pain: 0.,
        radius: 0.3,
        token_id: None,
    })?;
    let a_before = physiology(&base, id);
    let a = base.step(&trainer, 1.)?;
    if a.decisions.len() != 1 {
        let cp = bytes(&base.checkpoint(&trainer)?)?;
        e.save("FAILED-A-QUALIFICATION.cp.json", &cp)?;
        e.remember_valid(cp);
        return Ok(
            json!({"seed":seed,"qualified_A":false,"reason":"fixed lead-in endpoint had no awake A decision",
            "leadin_summary":leadin_summary,"actual_A_decisions":a.decisions.len()}),
        );
    }
    e.save("A.rows.json", &bytes(&a.decisions)?)?;
    let branch = base.checkpoint(&trainer)?;
    let branch_bytes = bytes(&branch)?;
    e.save("AFTER-A.cp.json", &branch_bytes)?;
    e.remember_valid(branch_bytes.clone());
    let cold = CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&branch_bytes)?, &trainer)?;
    assert_eq!(branch_bytes, bytes(&cold.checkpoint(&trainer)?)?);
    let mut none_pre = Vec::new();
    let mut summaries = Vec::new();
    for (name, cue) in [
        ("None", None),
        ("Praise", Some(PlayerInteractionV1::Praise)),
        ("Scold", Some(PlayerInteractionV1::Scold)),
        ("Pet", Some(PlayerInteractionV1::Pet)),
        ("Slap", Some(PlayerInteractionV1::Slap)),
    ] {
        let mut h = base.snapshot();
        assert_eq!(branch_bytes, bytes(&h.checkpoint(&trainer)?)?);
        let queue_tick = h.world().tick();
        let source = h
            .world()
            .entity(
                h.world()
                    .organism_registry()
                    .get(id)
                    .unwrap()
                    .world_entity_id(),
            )
            .unwrap()
            .position;
        let queued = cue
            .map(|c| h.world_mut().queue_player_interaction_v1(id, source, c))
            .transpose()?;
        // Full parallel native continuation has no checkpoint observer/replay diagnostic.
        let mut unobserved = h.snapshot();
        let mut trace = Vec::new();
        let mut diagnostics = Vec::new();
        let mut rewards = 0.;
        let mut sleeping = 0;
        let mut consolidations = 0;
        for j in 0..ticks {
            let pre = h.checkpoint(&trainer)?;
            let pre_bytes = bytes(&pre)?;
            e.remember_valid(pre_bytes.clone());
            e.save(&format!("{name}-{j:03}-PRE.cp.json"), &pre_bytes)?;
            let before = physiology(&h, id);
            assert!(pre.terminal_context2135.is_none());
            let s = h.step(&trainer, 1.)?;
            unobserved.step(&trainer, 1.)?;
            let post = h.checkpoint(&trainer)?;
            let post_bytes = bytes(&post)?;
            assert_eq!(post_bytes, bytes(&unobserved.checkpoint(&trainer)?)?);
            e.save(&format!("{name}-{j:03}-POST.cp.json"), &post_bytes)?;
            e.remember_valid(post_bytes.clone());
            e.save(&format!("{name}-{j:03}.rows.json"), &bytes(&s.decisions)?)?;
            sleeping += s.sleeping_organisms;
            consolidations += s.sleep_consolidations;
            if name == "None" {
                none_pre.push(pre.clone());
            }
            for row in &s.decisions {
                let q = TrainingSequence {
                    phenotype_hash: trainer.phenotype().phenotype_hash(),
                    initial: row.initial.clone(),
                    ticks: vec![row.replay_tick.clone()],
                    burn_in_ticks: 0,
                    memory_candidate_gain: trainer
                        .phenotype()
                        .candidate_decoder()
                        .memory_channel()
                        .map_or(0., |m| m.max_candidate_gain()),
                };
                assert_eq!(row.initial, pre.residents[0].neural.neural);
                assert_eq!(row.patch.pre_action().tick, s.tick_before);
                let native = trainer.evaluate_replay(&q)?;
                assert_eq!(native.final_activations[0], row.final_features);
                assert_eq!(
                    native.final_activity_ema[0],
                    post.residents[0].neural.neural.activity_ema
                );
                assert_eq!(
                    native.final_metabolic_load[0],
                    post.residents[0].neural.neural.metabolic_load
                );
                let frame = row.patch.pre_action().perception();
                let sampled = sample_cpu_joint_action(
                    trainer.phenotype(),
                    frame,
                    &native.candidate_logits[0],
                    seed as u32,
                    row.sampling_counter.unwrap(),
                    1.,
                )?;
                assert_eq!(sampled, row.action);
                let arm = &pre.residents[0].neural;
                let control = &none_pre[j].residents[0].neural;
                assert_eq!(pre.world.world.tick, none_pre[j].world.world.tick);
                let mut ablated = q.clone();
                let mut component = Vec::new();
                for (i, syn) in trainer.phenotype().synapses().iter().enumerate() {
                    let delta = syn.alpha() * (arm.fast[i] - control.fast[i]);
                    component.push(delta);
                    // Preserve the actual branch's lifetime and every other coordinate.
                    assert_eq!(
                        q.ticks[0].effective_weight_offsets[i].to_bits(),
                        (arm.lifetime[i] + syn.alpha() * arm.fast[i]).to_bits()
                    );
                    ablated.ticks[0].effective_weight_offsets[i] -= delta;
                }
                let intervention = trainer.evaluate_replay(&ablated)?;
                let changed = sample_cpu_joint_action(
                    trainer.phenotype(),
                    frame,
                    &intervention.candidate_logits[0],
                    seed as u32,
                    row.sampling_counter.unwrap(),
                    1.,
                )?;
                let action_changed = changed.representative != row.action.representative
                    || changed.motor_candidates != row.action.motor_candidates;
                diagnostics.push(json!({"interval":j,"world_tick":s.tick_before,"sampling_counter":row.sampling_counter,
                    "none_last_decision_tick":control.last_decision_tick,"arm_last_decision_tick":arm.last_decision_tick,
                    "fast_component_l2":norm(&component),"fast_component_nonzero":component.iter().filter(|&&x|x!=0.).count(),
                    "activation_max_abs":max_delta(&native.final_activations[0],&intervention.final_activations[0]),
                    "logit_max_abs":max_delta(&native.candidate_logits[0],&intervention.candidate_logits[0]),
                    "resampled_action_changed":action_changed,"native_joint_log_probability":row.action.old_joint_log_probability,
                    "ablated_resampled_joint_log_probability":changed.old_joint_log_probability,
                    "none_sleep_phase":none_pre[j].residents[0].sleep.state().phase,
                    "arm_sleep_phase":pre.residents[0].sleep.state().phase,
                    "microsteps":row.replay_tick.microstep_count,
                    "frame_digest":frame.frame_digest(),"sequence_id":row.patch.pre_action().sequence_id,
                    "same_state_intervention_only":true}));
                e.save(&format!("{name}-{j:03}-DIAGNOSTIC.json"),&bytes(&json!({"native":q,"ablated":ablated,
                    "native_logits":native.candidate_logits[0],"ablated_logits":intervention.candidate_logits[0],
                    "native_final":native.final_activations[0],"ablated_final":intervention.final_activations[0]}))?)?;
                rewards += row.biological_reward;
                trace.push(json!({"interval":j,"tick_before":s.tick_before,"tick_after":s.tick_after,
                    "before":before,"after":physiology(&h,id),"reward":row.biological_reward,
                    "pending_cue_before":pre.world.world.pending_player_care.contains_key(&id.raw()),
                    "pending_cue_after":post.world.world.pending_player_care.contains_key(&id.raw()),
                    "outcome":row.patch.outcome(),"microsteps":row.replay_tick.microstep_count,
                    "sensory":frame.sensory(),
                    "representative":row.action.representative,"motor_candidates":row.action.motor_candidates,
                    "selected_family":row.replay_tick.candidates[row.action.representative as usize].family,
                    "brain_work":row.brain_work,"input_l2":norm(&row.replay_tick.encoded_inputs),
                    "staged_and_updated_eligibility_l2":norm(&post.residents[0].neural.eligibility),
                    "fast_l2":norm(&post.residents[0].neural.fast),"lifetime_l2":norm(&post.residents[0].neural.lifetime)}));
            }
            // Observer and interventions must not alter any byte of the actual continuation.
            assert_eq!(post_bytes, bytes(&h.checkpoint(&trainer)?)?);
        }
        let final_bytes = bytes(&h.checkpoint(&trainer)?)?;
        let restored =
            CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&final_bytes)?, &trainer)?;
        assert_eq!(final_bytes, bytes(&restored.checkpoint(&trainer)?)?);
        summaries.push(json!({"arm":name,"queue_tick":queue_tick,"queued":queued,
            "reward_sum":rewards,"sleeping_steps":sleeping,"consolidations":consolidations,
            "food_consumed":h.world().entity(food).unwrap().consumed,"final":physiology(&h,id),
            "trace":trace,"diagnostics":diagnostics,"cold_exact":true,"observer_noninterference_full_cp":true}));
    }
    e.save("FOUNDER-ADAM-FINAL.json", &bytes(&trainer.checkpoint())?)?;
    assert_eq!(founder, bytes(&trainer.checkpoint())?);
    Ok(
        json!({"seed":seed,"ticks":ticks,"leadin":600,"leadin_summary":leadin_summary,"qualified_A":true,"biology_variant":"none; exact parent biology",
        "a_tick":a.tick_before,"a_before":a_before,"a_reward":a.decisions[0].biological_reward,
        "a_action":a.decisions[0].action,"a_family":a.decisions[0].replay_tick.candidates[a.decisions[0].action.representative as usize].family,
        "motor_head_count":2,"terminal_context":"Off","founder_adam_unchanged":true,"arms":summaries}),
    )
}
fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 7 {
        return Err(
            "usage: feedback_need_retention PARENT GENOME LOCAL_ROOT UNIQUE_ID SEED TICKS".into(),
        );
    }
    let ab = std::fs::read(&args[1])?;
    let gb = std::fs::read(&args[2])?;
    if format!("{:x}", Sha256::digest(&ab))
        != "d83a995e802e41b8e17799b0871f75d9ede35f95850049777323112e636654d1"
        || format!("{:x}", Sha256::digest(&gb))
            != "557da394c045d0b4353009c842825677aa7505f636d6252b20f67bfe0c3a5189"
    {
        return Err("exact Contact20 asset/genome hashes required".into());
    }
    let asset = FoundationWeightAsset::decode_canonical(&ab)?;
    let genome: CreatureGenome = serde_json::from_slice(&gb)?;
    let seed = args[5].parse::<u64>()?;
    let ticks = args[6].parse::<usize>()?;
    if !(1..=64).contains(&ticks) {
        return Err("ticks must be 1..64".into());
    }
    let mut e = LocalEvidence::begin(Path::new(&args[3]), &args[4])?;
    e.save("PROVENANCE.json",&bytes(&json!({"source_commit":option_env!("ALIFE_ASSAY_GIT_COMMIT"),
        "source_sha256":format!("{:x}",Sha256::digest(include_bytes!("feedback_need_retention.rs"))),
        "binary_sha256":format!("{:x}",Sha256::digest(std::fs::read(std::env::current_exe()?)?)),
        "asset_sha256":format!("{:x}",Sha256::digest(&ab)),"genome_sha256":format!("{:x}",Sha256::digest(&gb)),
        "seed":seed,"ticks":ticks,"backend":"native-cpu","raw_evidence_local_only":true}))?)?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&asset, &genome, seed, ticks, &mut e)
    }));
    let result = match result {
        Ok(result) => result,
        Err(_) => Err(
            "assay assertion/panic; inspect retained native evidence; no automatic retry".into(),
        ),
    };
    match result {
        Ok(summary) => {
            e.save("SUMMARY.json", &serde_json::to_vec_pretty(&summary)?)?;
            println!(
                "{}",
                json!({"completed":true,"seed":seed,"ticks":ticks,"evidence":e.directory()})
            );
            Ok(())
        }
        Err(error) => {
            let retained = e.retain_error_checkpoint();
            e.save("ERROR.json",&bytes(&json!({"error":error.to_string(),"retained":format!("{retained:?}"),"automatic_retry":false}))?)?;
            Err(error)
        }
    }
}

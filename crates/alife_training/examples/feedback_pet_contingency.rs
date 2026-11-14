//! LOCAL-only natural meal/Pet contingency pilot from an actual fixed-600 boundary.
//! No action forcing, state injection, reward replay, or neural reset.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, TrainingSequence};
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
fn hash(x: &[u8]) -> String {
    format!("{:x}", Sha256::digest(x))
}
fn norm(xs: &[f32]) -> f64 {
    xs.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>().sqrt()
}
fn diff(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0., f32::max)
}
fn physiology(h: &CpuCanonicalHost, id: OrganismId) -> Value {
    let r = h.world().organism_registry().get(id).unwrap();
    json!({"tick":h.world().tick(),"biology_tick":r.biochemistry().tick,"homeostasis":r.biochemistry().homeostasis,
 "body":r.biochemistry().body,"development":r.biochemistry().development,"alive":r.lifecycle().is_alive(),"sleep_phase":r.sleep_phase()})
}

fn trainer(
    asset: FoundationWeightAsset,
    genome: &CreatureGenome,
    adam: &[u8],
) -> Result<CpuFoundationTrainer> {
    let expressed = genome.express()?;
    let dev = foundation_construction_development(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &expressed.development_state_at(Tick::ZERO)?,
    )?;
    let (p, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        expressed.brain_genome,
        dev,
        asset.clone(),
    )?;
    assert_eq!(p.cognitive_architecture().motor_head_count(), 2);
    Ok(CpuFoundationTrainer::from_checkpoint(
        p,
        asset,
        serde_json::from_slice(adam)?,
    )?)
}
fn spawn(
    h: &mut CpuCanonicalHost,
    id: OrganismId,
    label: &str,
    far: bool,
) -> Result<WorldEntityId> {
    let p = h
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
    Ok(h.world_mut().editor_spawn_object(WorldEditorSpawnSpec {
        label: label.into(),
        kind: WorldObjectKind::Food,
        organism_id: None,
        position: Vec3f::new(p.x + if far { 2.5 } else { 1.5 }, p.y, p.z + 0.5),
        nutrition: 0.6,
        hazard_pain: 0.,
        radius: 0.3,
        token_id: None,
    })?)
}
fn pet(h: &mut CpuCanonicalHost, id: OrganismId) -> Result<()> {
    let p = h
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
    let result = h
        .world_mut()
        .queue_player_interaction_v1(id, p, PlayerInteractionV1::Pet)?;
    assert_eq!(result, PlayerInteractionQueueOutcomeV1::Queued);
    Ok(())
}

fn run(source: &Path, e: &mut LocalEvidence) -> Result<Value> {
    let asset_path=Path::new("docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation");
    let asset_bytes = std::fs::read(asset_path)?;
    let genome_bytes = std::fs::read(format!("{}.genome.json", asset_path.display()))?;
    assert_eq!(
        hash(&asset_bytes),
        "d83a995e802e41b8e17799b0871f75d9ede35f95850049777323112e636654d1"
    );
    assert_eq!(
        hash(&genome_bytes),
        "557da394c045d0b4353009c842825677aa7505f636d6252b20f67bfe0c3a5189"
    );
    let genome: CreatureGenome = serde_json::from_slice(&genome_bytes)?;
    let adam = std::fs::read(source.join("FOUNDER-ADAM-BIRTH.json"))?;
    let t = trainer(
        FoundationWeightAsset::decode_canonical(&asset_bytes)?,
        &genome,
        &adam,
    )?;
    assert_eq!(adam, bytes(&t.checkpoint())?);
    let source_bytes = std::fs::read(source.join("LEADIN-0600.cp.json"))?;
    let cp: CpuCanonicalCheckpoint = serde_json::from_slice(&source_bytes)?;
    assert_eq!(cp.world.world.tick, Tick(600));
    assert!(cp.terminal_context2135.is_none());
    let seed = cp.sampling_seed;
    let id = cp.residents[0].neural.organism_id;
    let birth = CpuCanonicalHost::from_checkpoint(cp, &t)?;
    assert_eq!(source_bytes, bytes(&birth.checkpoint(&t)?)?);
    e.save("COMMON-600.cp.json", &source_bytes)?;
    e.remember_valid(source_bytes.clone());
    e.save("FOUNDER-ADAM-BIRTH.json", &adam)?;
    e.save(
        "GRAPH-LOCAL.json",
        &std::fs::read(source.join("GRAPH-LOCAL.json"))?,
    )?;
    e.save("PROVENANCE.json",&bytes(&json!({"source_commit":option_env!("ALIFE_ASSAY_GIT_COMMIT"),
 "source_sha256":hash(include_bytes!("feedback_pet_contingency.rs")),"binary_sha256":hash(&std::fs::read(std::env::current_exe()?)?),
 "source_boundary_sha256":hash(&source_bytes),"source_adam_sha256":hash(&adam),"seed":seed,
 "asset_sha256":hash(&asset_bytes),"genome_sha256":hash(&genome_bytes),"backend":"native-cpu",
 "source_life":"actual fixed-600 boundary before any target or player cue","terminal_context":"Off"}))?)?;
    // Prospectively fixed: exposures [600,690), quiet [690,714), fresh target [714,738).
    // Three presentations at offsets 0,24,48; one Pet per target consumed within its 24-step block.
    const STEPS: usize = 138;
    const PROBE: usize = 114;
    let mut none_probe = Vec::new();
    let mut summaries = Vec::new();
    let mut contingent_queue_ticks = Vec::new();
    for arm in ["None", "PetContingent", "PetDelayedYoked"] {
        let mut h = birth.snapshot();
        let mut twin = h.snapshot();
        let start = physiology(&h, id);
        let mut trace = Vec::new();
        let mut pulses = Vec::new();
        let mut meals = Vec::new();
        let mut targets = Vec::new();
        let mut probe_target = None;
        let mut sleeping = 0;
        let mut consolidations = 0;
        let mut queue_ticks = Vec::new();
        let mut probe_diagnostics = Vec::new();
        let mut movement = 0_f64;
        let mut previous = h
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
        let yoked = contingent_queue_ticks
            .iter()
            .map(|tick| tick + 6)
            .collect::<Vec<u64>>();
        for j in 0..STEPS {
            if [0, 24, 48].contains(&j) || j == PROBE {
                let label = format!("fresh-food-{j}");
                let target = spawn(&mut h, id, &label, j == PROBE)?;
                assert_eq!(target, spawn(&mut twin, id, &label, j == PROBE)?);
                if j == PROBE {
                    probe_target = Some(target);
                } else {
                    targets.push(target);
                }
            }
            if arm == "PetDelayedYoked" && yoked.contains(&h.world().tick().raw()) {
                pet(&mut h, id)?;
                pet(&mut twin, id)?;
                queue_ticks.push(h.world().tick().raw());
                pulses.push(json!({"queue_tick":h.world().tick(),"source":"six ticks after matched contingent schedule; actual external Pet", "leaves_A_unattributed":true}));
            }
            let pre = h.checkpoint(&t)?;
            let pre_bytes = bytes(&pre)?;
            e.remember_valid(pre_bytes.clone());
            e.save(&format!("{arm}-{j:03}-PRE.cp.json"), &pre_bytes)?;
            let before = physiology(&h, id);
            let consumed_before = targets
                .iter()
                .map(|&target| h.world().entity(target).unwrap().consumed)
                .collect::<Vec<_>>();
            let s = h.step(&t, 1.)?;
            twin.step(&t, 1.)?;
            assert_eq!(s.tick_before, Tick(600 + j as u64));
            assert_eq!(s.tick_after, Tick(601 + j as u64));
            sleeping += s.sleeping_organisms;
            consolidations += s.sleep_consolidations;
            for (k, &target) in targets.iter().enumerate() {
                if !consumed_before[k] && h.world().entity(target).unwrap().consumed {
                    let eligible = j >= k * 24 && j < (k + 1) * 24;
                    meals.push(json!({"target_ordinal":k,"outcome_tick":s.tick_after,"in_presentation_window":eligible}));
                    if arm == "PetContingent" && eligible {
                        pet(&mut h, id)?;
                        pet(&mut twin, id)?;
                        queue_ticks.push(s.tick_after.raw());
                        pulses.push(json!({"queue_tick":s.tick_after,"observed_consumption_outcome_tick":s.tick_after,
       "target_ordinal":k,"source":"ordinary external Pet after actual target consumption", "leaves_A_unattributed":true}));
                    }
                }
            }
            let post = h.checkpoint(&t)?;
            let post_bytes = bytes(&post)?;
            assert_eq!(post_bytes, bytes(&twin.checkpoint(&t)?)?);
            e.save(&format!("{arm}-{j:03}-POST.cp.json"), &post_bytes)?;
            e.remember_valid(post_bytes.clone());
            e.save(&format!("{arm}-{j:03}.rows.json"), &bytes(&s.decisions)?)?;
            let current = h
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
            movement += f64::from(
                (current.x - previous.x).powi(2)
                    + (current.z - previous.z).powi(2)
                    + (current.y - previous.y).powi(2),
            )
            .sqrt();
            previous = current;
            if arm == "None" && j >= PROBE {
                none_probe.push(pre.clone());
            }
            if j >= PROBE {
                assert!(pre.world.world.pending_player_care.is_empty());
                assert!(post.world.world.pending_player_care.is_empty());
                for row in &s.decisions {
                    let p = t.phenotype();
                    let q = TrainingSequence {
                        phenotype_hash: p.phenotype_hash(),
                        initial: row.initial.clone(),
                        ticks: vec![row.replay_tick.clone()],
                        burn_in_ticks: 0,
                        memory_candidate_gain: p
                            .candidate_decoder()
                            .memory_channel()
                            .map_or(0., |m| m.max_candidate_gain()),
                    };
                    let native = t.evaluate_replay(&q)?;
                    assert_eq!(native.final_activations[0], row.final_features);
                    let actual = sample_cpu_joint_action(
                        p,
                        row.patch.pre_action().perception(),
                        &native.candidate_logits[0],
                        seed,
                        row.sampling_counter.unwrap(),
                        1.,
                    )?;
                    assert_eq!(actual, row.action);
                    let control = &none_probe[j - PROBE].residents[0].neural;
                    let own = &pre.residents[0].neural;
                    assert_eq!(none_probe[j - PROBE].world.world.tick, pre.world.world.tick);
                    let mut ablated = q.clone();
                    let mut component = Vec::new();
                    for (i, syn) in p.synapses().iter().enumerate() {
                        assert_eq!(
                            q.ticks[0].effective_weight_offsets[i].to_bits(),
                            (own.lifetime[i] + syn.alpha() * own.fast[i]).to_bits()
                        );
                        let delta = syn.alpha() * (own.fast[i] - control.fast[i]);
                        component.push(delta);
                        ablated.ticks[0].effective_weight_offsets[i] -= delta;
                    }
                    let intervention = t.evaluate_replay(&ablated)?;
                    let changed = sample_cpu_joint_action(
                        p,
                        row.patch.pre_action().perception(),
                        &intervention.candidate_logits[0],
                        seed,
                        row.sampling_counter.unwrap(),
                        1.,
                    )?;
                    probe_diagnostics.push(json!({"interval":j,"world_tick":s.tick_before,"sampling_counter":row.sampling_counter,
      "none_last_decision_tick":control.last_decision_tick,"arm_last_decision_tick":own.last_decision_tick,
      "fast_component_l2":norm(&component),"activation_max_abs":diff(&native.final_activations[0],&intervention.final_activations[0]),
      "logit_max_abs":diff(&native.candidate_logits[0],&intervention.candidate_logits[0]),
      "resampled_action_changed":changed.representative!=actual.representative || changed.motor_candidates!=actual.motor_candidates,
      "none_sleep_phase":none_probe[j-PROBE].residents[0].sleep.state().phase,"arm_sleep_phase":pre.residents[0].sleep.state().phase,
      "microsteps":row.replay_tick.microstep_count,"same_state_only":true}));
                    e.save(&format!("{arm}-{j:03}-DIAGNOSTIC.json"),&bytes(&json!({"native":q,"ablated":ablated,"native_logits":native.candidate_logits[0],
       "ablated_logits":intervention.candidate_logits[0],"native_final":native.final_activations[0],"ablated_final":intervention.final_activations[0]}))?)?;
                }
            }
            trace.push(json!({"interval":j,"tick_before":s.tick_before,"tick_after":s.tick_after,
     "phase":if j<90{"exposure"}else if j<PROBE{"quiet"}else{"fresh_no_feedback_probe"},
     "before":before,"after":physiology(&h,id),"movement_total":movement,"sleeping":s.sleeping_organisms,"consolidations":s.sleep_consolidations,
     "pending_cue_before":!pre.world.world.pending_player_care.is_empty(),"pending_cue_after":!post.world.world.pending_player_care.is_empty(),
     "rows":s.decisions.iter().map(|r|json!({"action":r.action,"family":r.replay_tick.candidates[r.action.representative as usize].family,
      "reward":r.biological_reward,"work":r.brain_work,"outcome":r.patch.outcome()})).collect::<Vec<_>>()}));
            assert_eq!(post_bytes, bytes(&h.checkpoint(&t)?)?);
            if (j + 1).is_multiple_of(24) {
                eprintln!(
                    "arm={arm} interval={} tick={} doses={} meals={}",
                    j + 1,
                    s.tick_after.raw(),
                    queue_ticks.len(),
                    meals.len()
                );
            }
        }
        if arm == "PetContingent" {
            contingent_queue_ticks = queue_ticks.clone();
        }
        if arm == "PetDelayedYoked" {
            assert_eq!(queue_ticks, yoked);
        }
        assert!(queue_ticks.iter().all(|&tick| tick < 690));
        let final_bytes = bytes(&h.checkpoint(&t)?)?;
        let cold = CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&final_bytes)?, &t)?;
        assert_eq!(final_bytes, bytes(&cold.checkpoint(&t)?)?);
        summaries.push(json!({"arm":arm,"start":start,"final":physiology(&h,id),"trace":trace,"pulses":pulses,"queue_ticks":queue_ticks,
   "meal_events":meals,"exposure_targets_consumed":targets.iter().filter(|&&target|h.world().entity(target).unwrap().consumed).count(),
   "probe_target_consumed":h.world().entity(probe_target.unwrap()).unwrap().consumed,
   "sleeping_steps":sleeping,"consolidations":consolidations,"movement_total":movement,"probe_diagnostics":probe_diagnostics,
   "observer_full_cp_exact":true,"cold_exact":true}));
    }
    assert_eq!(adam, bytes(&t.checkpoint())?);
    e.save("FOUNDER-ADAM-FINAL.json", &bytes(&t.checkpoint())?)?;
    Ok(
        json!({"seed":seed,"start_tick":600,"exposure_steps":90,"quiet_steps":24,"probe_steps":24,"presentations":3,
 "yoked_delay_ticks":6,"signal":"Pet only; other three signals remain separately measured in fixed600 single-pulse assay",
 "terminal_context":"Off","founder_adam_unchanged":true,"arms":summaries}),
    )
}
fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        return Err(
            "usage: feedback_pet_contingency SOURCE_FIXED600_RUN LOCAL_ROOT UNIQUE_ID".into(),
        );
    }
    let mut e = LocalEvidence::begin(Path::new(&args[2]), &args[3])?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(Path::new(&args[1]), &mut e)
    }));
    match result.unwrap_or_else(|_| {
        Err("assay assertion/panic; inspect local state; no automatic retry".into())
    }) {
        Ok(s) => {
            e.save("SUMMARY.json", &serde_json::to_vec_pretty(&s)?)?;
            println!("{}", json!({"completed":true,"evidence":e.directory()}));
            Ok(())
        }
        Err(err) => {
            let retained = e.retain_error_checkpoint();
            e.save("ERROR.json",&bytes(&json!({"error":err.to_string(),"retained":format!("{retained:?}"),"automatic_retry":false}))?)?;
            Err(err)
        }
    }
}

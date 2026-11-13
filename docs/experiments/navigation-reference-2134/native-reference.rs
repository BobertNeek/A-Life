//! Prospective genuine-own-feedback physical comparison2092. Source only until qualified.
use alife_core::*;
use alife_training::cpu::current_self2000::*;
use alife_training::cpu::joint_head2056::*;
use alife_training::cpu::joint_replay2056::*;
use alife_training::cpu::joint_runtime2056::*;
use alife_training::cpu::position_output2034::*;
use alife_training::cpu::position_readout2018::*;
use alife_training::cpu::positional_context2013::*;
use alife_training::cpu::retained_context1992::*;
use alife_training::cpu::selected_bearing_asset_1955::*;
use alife_training::cpu::*;
use alife_training::cpu::{
    contextual_joint2076::*, own_event2076::Model, own_event_queue2086::Queue, own_joint2086::*,
};
use alife_training::{AdamWConfig, StageTrainableMask};
use alife_world::*;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
type R<T> = Result<T, Box<dyn std::error::Error>>;
#[derive(Default, Serialize)]
struct Observer {
    microsteps: usize,
    retained: Vec<RetainedScore1992>,
    current_self: Vec<CurrentSelfScore2000>,
    logits_bits: Vec<u32>,
    endpoints: Vec<Vec<f32>>,
    metabolism: Vec<Vec<f32>>,
    ema: Vec<Vec<f32>>,
    position_states: Vec<Vec<PositionState2013>>,
    position_work: Vec<PositionWork2013>,
    readout: Vec<PositionScore2018>,
    output: Vec<PositionOutputScore2029>,
    joint: Vec<JointDecisionReceipt2056>,
    own_joint: Vec<OwnJointDecision2086>,
}
impl NativeReplayObserver for Observer {
    fn own_joint2086(&mut self, r: &OwnJointDecision2086) {
        self.own_joint.push(r.clone());
    }
    fn joint_policy2056(&mut self, r: &JointDecisionReceipt2056) {
        self.joint.push(r.clone());
    }
    fn retained_context1992(&mut self, r: &RetainedScore1992) {
        self.retained.push(r.clone());
    }
    fn current_self2000(&mut self, r: &CurrentSelfScore2000) {
        self.current_self.push(r.clone());
    }
    fn decision_logits(&mut self, v: &[f32]) {
        self.logits_bits.extend(v.iter().map(|x| x.to_bits()));
    }
    fn position_readout2018(
        &mut self,
        _: usize,
        _: usize,
        _: &PositionReadoutFrame2018,
        s: &PositionScore2018,
    ) {
        self.readout.push(s.clone());
    }
    fn position_output2034(
        &mut self,
        _: usize,
        _: usize,
        _: &PositionOutputFrame2029,
        s: &PositionOutputScore2029,
    ) {
        self.output.push(s.clone());
    }
    fn wants_target(&self, _: usize) -> bool {
        false
    }
    fn cell(&mut self, _: NativeReplayCellObservation<'_>) {}
    fn microstep(&mut self, _: usize, _: usize, a: &[f32], m: &[f32], e: &[f32]) {
        self.microsteps += 1;
        self.endpoints.push(a.to_vec());
        self.metabolism.push(m.to_vec());
        self.ema.push(e.to_vec())
    }
    fn positional_context2013(&mut self, _: usize, w: &PositionWork2013, s: &[PositionState2013]) {
        self.position_work.push(w.clone());
        self.position_states.push(s.to_vec());
    }
}
fn save<T: Serialize>(p: &Path, x: &T) -> R<()> {
    codec::save(p, &serde_json::to_vec(x)?)
}
mod codec;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    scene_id: u64,
    case: String,
    condition: String,
    mirror: i8,
    seed_pair: u8,
    arm: String,
    world_seed: u64,
    sampling_seed: u32,
    steps: usize,
    confidence_decay: f32,
    inner_gains: [f32; 2],
    motion_multipliers: [f32; 2],
    motion_coefficients: [f32; 2],
    policy_profile: String,
    scalar_output_amplitudes: [f32; 2],
    joint_coefficients: Option<[f32; 3]>,
    joint_amplitude: Option<f32>,
    scalar_output_active: bool,
    old_r_coefficients: String,
    current_self_coefficient: f32,
    wall_half_span: f32,
    target_kind: String,
    target_chemistry: [f32; 3],
    target_color: [f32; 3],
    target_shape: [f32; 3],
    target_material: [f32; 3],
    target_nutrition: f32,
    target_hazard_pain: f32,
    starting_energy: f32,
    angular_couplings: [f32; 2],
    own_model_parameters: [f64; 5],
    initial_body_yaw: f32,
}
fn spawn(w: &mut HeadlessWorld, p: &Plan) -> R<()> {
    if p.target_kind == "Absent" {
        return Ok(());
    }
    let kind = match p.target_kind.as_str() {
        "Food" => WorldObjectKind::Food,
        "Ball" => WorldObjectKind::Ball,
        _ => return Err("target kind".into()),
    };
    let id = w.editor_spawn_object(WorldEditorSpawnSpec {
        label: "target".into(),
        kind,
        organism_id: None,
        position: Vec3f::new(2.2, 0., 0.6 * f32::from(p.mirror)),
        nutrition: p.target_nutrition,
        hazard_pain: p.target_hazard_pain,
        radius: 0.5,
        token_id: None,
    })?;
    let mut ph = w.entity(id).unwrap().grounded_physical;
    ph.color = p.target_color;
    ph.shape = p.target_shape;
    ph.material = p.target_material;
    ph.chemical = p.target_chemistry;
    ph.velocity = Vec3f::ZERO;
    w.set_grounded_physical_properties(id, ph)?;
    Ok(())
}
fn own(w: &HeadlessWorld, id: OrganismId) -> R<serde_json::Value> {
    let s = w.physical_observation_snapshot(id, w.tick())?;
    let p = s.observer_pose.translation;
    let v = s.observer_velocity.linear;
    let entity = w
        .object_snapshots()
        .into_iter()
        .find(|o| o.organism_id == Some(id))
        .ok_or("own body")?;
    Ok(
        serde_json::json!({"tick":w.tick().raw(),"own_position":[p.x,p.y,p.z],"legacy_velocity":[v.x,v.y,v.z],"body_yaw":entity.body_yaw,"head_yaw":entity.head_yaw}),
    )
}
fn head() -> R<RetainedHead1992> {
    Ok(RetainedHead1992::fresh_zero(32., AdamWConfig::default())?)
}
fn validate_plan(plan: &Plan) -> R<()> {
    let novel = plan.condition == "ClearSweetNovel";
    let color = if novel {
        [0.2, 0.7, 0.1]
    } else {
        [0.75, 0.25, 0.75]
    };
    let shape = if novel {
        [0.7, 0.2, 0.4]
    } else {
        [0.25, 0.75, 0.25]
    };
    if plan.target_color != color
        || plan.target_shape != shape
        || plan.target_material != [0.3, 0.7, 0.2]
    {
        return Err("appearance recipe".into());
    }
    let (coefficients, amplitude, coupling) = match plan.arm.as_str() {
        "TranslationOnly" => ([1., 0., 0.], 1024., [0.; 2]),
        "CoupledHigh" => ([1., 1., 1.], 1024., [1.; 2]),
        "UncoupledHigh" => ([1., 1., 1.], 1024., [0.; 2]),
        "ZeroPotential" => ([1., 1., 1.], 0., [0.; 2]),
        _ => return Err("arm".into()),
    };
    if ![-1, 1].contains(&plan.mirror)
        || plan.inner_gains != [1.; 2]
        || plan.confidence_decay.to_bits() != 0.98f32.to_bits()
        || plan.old_r_coefficients != "zero_preserved"
        || plan.current_self_coefficient != 0.
    {
        return Err("fixed context recipe".into());
    }
    if plan.joint_amplitude != Some(amplitude)
        || plan.joint_coefficients != Some(coefficients)
        || plan.scalar_output_amplitudes != [1.; 2]
        || plan.scalar_output_active
        || plan.policy_profile != "OwnFeedback2086"
        || plan.angular_couplings != coupling
        || plan.own_model_parameters != [0., 1., 1., 1., 1.]
        || !plan.initial_body_yaw.is_finite()
    {
        return Err("policy recipe".into());
    }
    if plan.motion_coefficients != [1. / 32.; 2] || plan.motion_multipliers != [1.; 2] {
        return Err("motion units".into());
    }
    Ok(())
}
fn run() -> R<()> {
    let a = std::env::args().collect::<Vec<_>>();
    let plan: Plan = serde_json::from_slice(&fs::read(a.get(1).ok_or("plan")?)?)?;
    let arm = &plan.arm;
    validate_plan(&plan)?;
    let out = Path::new(a.get(2).ok_or("fresh output path")?);
    fs::create_dir(out)?;
    let seed = plan.world_seed;
    save(&out.join("START.json"), &plan)?;
    let d = produce_current_self_orientation2000()?;
    let mut birth = CanonicalNewGameConfig::phase3(seed, 1)?;
    birth.brain_class = BrainScaleTier::Standard2048;
    birth.founder_seed_base = 2026111560;
    birth.sensor_profile = SensorProfile::GroundedTerrainObjectPortsV2;
    let mut game = create_canonical_new_game_with_current_self_orientation2000(&birth, d.asset())?;
    let f = CpuFoundationTrainer::new(
        d.phenotype().clone(),
        d.asset().clone(),
        StageTrainableMask::recurrent_only(d.phenotype())?,
        AdamWConfig::default(),
    )?;
    let original = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("founder")?
        .clone();
    let owner = original.organism_id();
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("self", owner, Vec3f::ZERO)
        .build()?;
    let center = (0.75f32 / 2.2) * (0.6 * f32::from(plan.mirror));
    let half = plan.wall_half_span;
    let walls = if half == 0. {
        vec![]
    } else {
        vec![
            [0.65, center - half, 0.85, center - 0.08, 0., 2.],
            [0.65, center + 0.08, 0.85, center + half, 0., 2.],
        ]
    };
    world.enable_terrain_for_new_game(
        WorldTerrain::new(
            TerrainData {
                width: 17,
                depth: 17,
                origin_x: -4.,
                origin_z: -4.,
                spacing: 0.5,
                heights: vec![0.; 289],
                obstacles: walls,
                water_level: None,
            },
            LocomotionLimits::default(),
        )?,
        Vec3f::ZERO,
    )?;
    spawn(&mut world, &plan)?;
    let mut chemistry = *original.biochemistry();
    chemistry.body.set_energy(plan.starting_energy)?;
    let body = world.entity_id("self").unwrap();
    world.register_organism_record(WorldOrganismRecord::new(
        owner,
        body,
        original.genome().clone(),
        original.phenotype().clone(),
        chemistry,
        original.birth_tick(),
    )?)?;
    // Fresh physical fixture only: no host/neural checkpoint exists yet.
    if plan.initial_body_yaw != 0. {
        let mut snapshot = PortableSaveFile::from_headless_world(
            "fresh-own-yaw2092",
            &world,
            RuntimeConfig::deterministic_default(seed, BrainScaleTier::Standard2048),
            AssetManifest::empty(),
            game.creatures.clone(),
        )?;
        let original_snapshot = snapshot.clone();
        snapshot
            .world
            .objects
            .iter_mut()
            .find(|o| o.id == body)
            .ok_or("physical body absent")?
            .body_yaw = plan.initial_body_yaw;
        let restored = snapshot.restore_headless_world()?;
        let mut roundtrip = snapshot.clone();
        roundtrip.replace_headless_world_snapshot(&restored)?;
        assert_eq!(
            serde_json::to_vec(&roundtrip.world)?,
            serde_json::to_vec(&snapshot.world)?
        );
        let mut reverted = snapshot.clone();
        reverted
            .world
            .objects
            .iter_mut()
            .find(|o| o.id == body)
            .unwrap()
            .body_yaw = 0.;
        assert_eq!(
            serde_json::to_vec(&reverted)?,
            serde_json::to_vec(&original_snapshot)?
        );
        save(
            &out.join("FRESH-YAW-FIXTURE.json"),
            &serde_json::json!({"body_yaw":plan.initial_body_yaw,"only_declared_physical_yaw_changed":true,"no_setup_action":true}),
        )?;
        world = restored;
    }
    game.world = world;
    let self_entity = game.world.entity(body).ok_or("self")?;
    if self_entity.position != Vec3f::ZERO
        || self_entity.body_yaw.to_bits() != plan.initial_body_yaw.to_bits()
        || self_entity.head_yaw.to_bits() != 0
        || self_entity.grounded_physical.velocity != Vec3f::ZERO
        || game.world.tick() != Tick::ZERO
    {
        return Err("not at-rest birth".into());
    }
    let config = CpuNavigationConfigV1::own_feedback2086(&f)?;
    let mut sh = CurrentSelfHead2000::fresh_from_forward(&f)?;
    sh.coefficient = 0.;
    let mut pm = PositionModel2013::fresh(AdamWConfig::default())?;
    pm.weights[4..].copy_from_slice(&plan.motion_coefficients);
    let ph = PositionHead2018::fresh(plan.inner_gains, AdamWConfig::default())?;
    let output =
        PositionOutputAdam2029::fresh(plan.scalar_output_amplitudes, AdamWConfig::default())?;
    let mut host = RetainedCanonicalHost1992::new_own_feedback2086(
        game,
        &f,
        plan.sampling_seed,
        config.clone(),
        head()?,
        sh,
        pm,
        ph,
        output,
        JointHead2056::fresh(
            plan.joint_coefficients.unwrap(),
            plan.joint_amplitude.unwrap(),
            AdamWConfig::default(),
        )?,
        plan.confidence_decay,
        Model::default(),
        AngularCoupling2076::fresh(plan.angular_couplings, AdamWConfig::default())?,
    )?;
    let founder = serde_json::to_vec(&f.checkpoint())?;
    let birthcp = host.checkpoint(&f)?;
    save(&out.join("birth.json"), &birthcp)?;
    save(&out.join("founder.json"), &f.checkpoint())?;
    let fixed = |cp: &RetainedHostCheckpoint1992| -> R<Vec<u8>> {
        let r = &cp.retained;
        if r.position_output2034.is_none()
            || r.position_readout2018.is_none()
            || r.joint2056.is_none()
        {
            return Err("model presence".into());
        }
        let own = r.own2086.as_ref().ok_or("own model absent")?;
        let models = own
            .residents
            .iter()
            .map(|(id, c)| {
                Ok((
                    *id,
                    Queue::restore(c.clone())
                        .map_err(std::io::Error::other)?
                        .model()
                        .cold_bits()
                        .map_err(std::io::Error::other)?,
                ))
            })
            .collect::<R<Vec<_>>>()?;
        Ok(serde_json::to_vec(&(
            &r.head,
            &r.current_self2000,
            &r.position2013.as_ref().ok_or("P absent")?.model,
            &r.position_readout2018,
            &r.position_output2034,
            &r.joint2056,
            &own.coupling,
            models,
        ))?)
    };
    let heads = fixed(&birthcp)?;
    save(
        &out.join("LOGIT-SEMANTICS.json"),
        &serde_json::json!({"ordinary_logits":"BASE R/S; pair potential is separate","joint":true,"own_feedback":true,"old_scalar_heads_dormant":true}),
    )?;
    let (mut decisions, mut microsteps, mut captures, mut raw_deliveries, mut slept) =
        (0, 0, 0, 0, 0);
    for k in 0..plan.steps {
        let dir = out.join(format!("step-{k}"));
        fs::create_dir(&dir)?;
        let before = host.checkpoint(&f)?;
        save(&dir.join("before.json"), &before)?;
        save(&dir.join("own-before.json"), &own(host.world(), owner)?)?;
        save(&dir.join("START.json"), &serde_json::json!({"step":k}))?;
        let mut observer = Observer::default();
        let mut staged = host.clone();
        let native_started = std::time::Instant::now();
        let result = staged.step_observed(&f, 1., Some(&mut observer));
        let native_micros = native_started.elapsed().as_micros();
        save(&dir.join("observer.json"), &observer)?;
        if let Ok(step) = &result {
            save(&dir.join("decisions.json"), &step.decisions)?;
            save(
                &dir.join("canonical-decisions.json"),
                &step.canonical.decisions,
            )?;
            save(
                &dir.join("raw-completed-step.json"),
                &serde_json::json!({"before":step.canonical.tick_before.raw(),"after":step.canonical.tick_after.raw(),"sleeping":step.canonical.sleeping_organisms,"decisions":step.decisions.len()}),
            )?;
        }
        let after = staged.checkpoint(&f)?;
        save(&dir.join("after.json"), &after)?;
        save(&dir.join("own-after.json"), &own(staged.world(), owner)?)?;
        let step = match result {
            Ok(s) => s,
            Err(e) => {
                save(
                    &dir.join("FAILED.json"),
                    &serde_json::json!({"error":e.to_string(),"native_microsteps":observer.microsteps,"whole_host_rollback_exact":serde_json::to_vec(&before)?==serde_json::to_vec(&after)?,"no_retry":true}),
                )?;
                return Err(e.into());
            }
        };
        decisions += step.decisions.len();
        microsteps += observer.microsteps;
        slept += step.canonical.sleeping_organisms;
        for row in &step.decisions {
            row.validate(&config)?;
            staged.validate_decision(row)?;
            let rr = row.input_receipt.retained1992.as_ref().unwrap();
            assert_eq!(rr.scores, observer.retained);
            assert_eq!(
                rr.current_self2000.as_ref().unwrap().scores,
                observer.current_self
            );
            assert_eq!(
                row.input_receipt
                    .preserved_motor_candidate_logits_bits
                    .as_ref()
                    .unwrap(),
                &observer.logits_bits
            );
            let p = rr.position2013.as_ref().unwrap();
            raw_deliveries += usize::from(p.raw.newly_delivered.is_some());
            captures += usize::from(
                p.raw.newly_delivered.is_some()
                    && row.canonical_row.replay_tick.microstep_count > 0,
            );
            assert_eq!(p.after, *p.states.last().unwrap());
            assert_eq!(p.states, observer.position_states[0]);
            assert_eq!(p.work.dispatches, observer.position_work[0].transitions);
            assert_eq!(
                p.work.active_updates,
                observer.position_work[0].active_updates
            );
            let reference = if p.work.dispatches > 0 {
                p.raw.current_tick
            } else {
                p.raw.previous_tick
            };
            {
                let j = rr.own2086.as_ref().ok_or("own receipt absent")?;
                assert!(rr.joint2056.is_none() && observer.joint.is_empty());
                assert_eq!(j.coupling.weights, plan.angular_couplings);
                assert!(rr.position_readout2018.is_none() && rr.position_output2034.is_none());
                assert!(observer.readout.is_empty() && observer.output.is_empty());
                assert_eq!(observer.own_joint, vec![j.decision.clone()]);
                assert_eq!(j.head.coefficients, plan.joint_coefficients.unwrap());
                assert_eq!(j.head.amplitude, plan.joint_amplitude.unwrap());
                assert_eq!(
                    j.decision.source_state,
                    [p.after.x, p.after.z, p.after.occupancy, p.after.confidence]
                );
                assert_eq!(
                    j.decision.frame.joint.processed_position_reference_tick,
                    reference
                );
                let base = rr
                    .current_self2000
                    .as_ref()
                    .unwrap()
                    .scores
                    .iter()
                    .map(|s| s.final_score)
                    .collect::<Vec<_>>();
                assert_eq!(
                    base.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    observer.logits_bits
                );
                assert_eq!(
                    decision_from_source2086(
                        &j.head,
                        &j.coupling,
                        &j.decision.frame,
                        j.decision.source_state,
                        &base
                    )?,
                    j.decision
                );
                let selected = j.selection.as_ref().ok_or("joint selection absent")?;
                assert_eq!(selected.action, row.canonical_row.action);
                assert_eq!(selected.action.policy_schema, Some(2086));
                assert_eq!(
                    sample_own_joint2086(
                        &base,
                        &j.decision,
                        plan.sampling_seed,
                        selected.sampling_counter
                    )?,
                    *selected
                );
                let mut bad = row.clone();
                bad.input_receipt
                    .retained1992
                    .as_mut()
                    .unwrap()
                    .own2086
                    .as_mut()
                    .unwrap()
                    .decision
                    .scores[0]
                    .value += 0.01;
                assert!(bad.validate(&config).is_err());
                let mut bad = row.clone();
                bad.input_receipt
                    .retained1992
                    .as_mut()
                    .unwrap()
                    .own2086
                    .as_mut()
                    .unwrap()
                    .process
                    .work
                    .extra_angular_primitives += 1;
                assert!(bad.validate(&config).is_err());
            }
            let mut bad = row.clone();
            bad.input_receipt
                .retained1992
                .as_mut()
                .unwrap()
                .position2013
                .as_mut()
                .unwrap()
                .states
                .last_mut()
                .unwrap()
                .x += 0.01;
            assert!(bad.validate(&config).is_err());
            if p.raw.newly_delivered.is_some() {
                let mut bad = row.clone();
                bad.input_receipt
                    .retained1992
                    .as_mut()
                    .unwrap()
                    .position2013
                    .as_mut()
                    .unwrap()
                    .raw
                    .newly_delivered
                    .as_mut()
                    .unwrap()
                    .range01 += 0.01;
                assert!(bad.validate(&config).is_err());
            }
        }
        if let Some(last) = observer.endpoints.last() {
            let v = serde_json::to_value(&after)?;
            let n = &v["host"]["residents"][0]["neural"]["neural"];
            for (key, actual) in [
                ("activations", last),
                ("metabolic_load", observer.metabolism.last().unwrap()),
                ("activity_ema", observer.ema.last().unwrap()),
            ] {
                let expected: Vec<f32> = serde_json::from_value(n[key].clone())?;
                assert_eq!(
                    expected.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                    actual.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                );
            }
        }
        let raw = serde_json::to_vec(&after)?;
        let cold_started = std::time::Instant::now();
        let restored = RetainedCanonicalHost1992::from_checkpoint(
            serde_json::from_slice(&codec::decode_exact_frame1941(&fs::read(
                dir.join("after.json.zst"),
            )?)?)?,
            &f,
            &config,
        )?;
        assert_eq!(raw, serde_json::to_vec(&restored.checkpoint(&f)?)?);
        let cold_micros = cold_started.elapsed().as_micros();
        assert_eq!(founder, serde_json::to_vec(&f.checkpoint())?);
        assert_eq!(heads, fixed(&after)?);
        let feedback = after.retained.own2086.as_ref().ok_or("own state absent")?;
        let summaries=feedback.residents.iter().map(|(id,c)| {let q=Queue::restore(c.clone()).map_err(std::io::Error::other)?;Ok(serde_json::json!({"owner":id,"received":q.received(),"drained":q.drained(),"pending":q.pending().len(),"occupied":q.state().occupied,"stride":q.state().stride,"latest":q.latest()}))}).collect::<R<Vec<_>>>()?;
        save(&dir.join("OWN-REFERENCES.json"), &summaries)?;
        save(
            &dir.join("CLOSED.json"),
            &serde_json::json!({"decisions":step.decisions.len(),"microsteps":observer.microsteps,"sleeping":step.canonical.sleeping_organisms,"native_microseconds":native_micros,"cold_microseconds":cold_micros,"typed_cold_exact":true,"all_model_adam_exact":true}),
        )?;
        host = staged;
    }
    let fact = serde_json::json!({"arm":arm,"host_intervals":plan.steps,"committed_decisions":decisions,"committed_neural_microsteps":microsteps,"genuine_position_captures":captures,"raw_position_deliveries":raw_deliveries,"sleeping_intervals":slept,"typed_cold_exact":true,"all_model_adam_exact":true,"rollback_attempts":0,"forced_setup_commands":0});
    save(&out.join("CLOSED.json"), &fact)?;
    println!("{fact}");
    Ok(())
}

fn main() {
    let result = std::panic::catch_unwind(run);
    let error = match result {
        Ok(Ok(())) => return,
        Ok(Err(e)) => e.to_string(),
        Err(_) => "validation panic; inspect all retained attempted records; no retry".into(),
    };
    if let Some(out) = std::env::args().nth(2) {
        let p = Path::new(&out);
        if p.is_dir() {
            let _ = save(
                &p.join("FAILED.json"),
                &serde_json::json!({"error":error,"automatic_retry":false,"outcome":"inspect last saved after/decision records before any recovery"}),
            );
        }
    }
    eprintln!("{error}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plan(arm: &str) -> Plan {
        let (coeff, amp, k) = match arm {
            "CoupledHigh" => ([1., 1., 1.], 1024., [1., 1.]),
            "UncoupledHigh" => ([1., 1., 1.], 1024., [0., 0.]),
            "TranslationOnly" => ([1., 0., 0.], 1024., [0., 0.]),
            _ => ([1., 1., 1.], 0., [0., 0.]),
        };
        serde_json::from_value(serde_json::json!({"scene_id":1,"case":"test","condition":"ClearSweet","mirror":1,"seed_pair":0,"arm":arm,"world_seed":1,"sampling_seed":1,"steps":96,"confidence_decay":0.98,"inner_gains":[1,1],"motion_multipliers":[1,1],"motion_coefficients":[0.03125,0.03125],"policy_profile":"OwnFeedback2086","scalar_output_amplitudes":[1,1],"joint_coefficients":coeff,"joint_amplitude":amp,"scalar_output_active":false,"old_r_coefficients":"zero_preserved","current_self_coefficient":0,"wall_half_span":0,"target_kind":"Food","target_chemistry":[0.85,0,0],"target_color":[0.75,0.25,0.75],"target_shape":[0.25,0.75,0.25],"target_material":[0.3,0.7,0.2],"target_nutrition":0.6,"target_hazard_pain":0,"starting_energy":0.4,"angular_couplings":k,"own_model_parameters":[0,1,1,1,1],"initial_body_yaw":0})).unwrap()
    }
    #[test]
    fn four_exact_recipes() {
        for arm in [
            "CoupledHigh",
            "UncoupledHigh",
            "TranslationOnly",
            "ZeroPotential",
        ] {
            validate_plan(&plan(arm)).unwrap();
        }
    }
    #[test]
    fn coupling_and_model_mutations_reject() {
        let mut p = plan("UncoupledHigh");
        p.angular_couplings = [1., 1.];
        assert!(validate_plan(&p).is_err());
        p.angular_couplings = [0., 0.];
        p.own_model_parameters[0] = 0.1;
        assert!(validate_plan(&p).is_err());
    }
    #[test]
    fn declared_finite_rotation_preserves_other_recipe() {
        for yaw in [-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4] {
            let mut p = plan("CoupledHigh");
            p.initial_body_yaw = yaw;
            validate_plan(&p).unwrap();
        }
        let mut p = plan("CoupledHigh");
        p.initial_body_yaw = f32::NAN;
        assert!(validate_plan(&p).is_err());
    }
    #[test]
    fn plan_unknown_fields_reject() {
        let mut p = serde_json::to_value(plan("CoupledHigh")).unwrap();
        p["hidden_target"] = serde_json::json!(1);
        assert!(serde_json::from_value::<Plan>(p).is_err());
    }
}

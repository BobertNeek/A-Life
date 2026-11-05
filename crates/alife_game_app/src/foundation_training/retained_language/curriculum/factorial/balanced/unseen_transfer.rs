//! Frozen V18 actors on predeclared unseen worlds; no additional training.
use super::*;

pub(super) const SEEDS: [u64; 6] = [
    4_104_491, 4_209_927, 4_308_851, 4_401_393, 4_507_581, 4_609_807,
];
pub(super) const FRESH_SEEDS: [u64; 6] = [
    6_104_531, 6_210_019, 6_309_173, 6_401_879, 6_508_117, 6_610_327,
];
pub(super) const REPLAY_SEEDS: [u64; 6] = [
    8_104_543, 8_210_021, 8_309_181, 8_401_903, 8_508_131, 8_610_343,
];
pub(super) const EARLY_STOP_SEEDS: [u64; 6] = [
    10_104_579, 10_210_057, 10_309_219, 10_401_957, 10_508_179, 10_610_399,
];
pub(super) fn prepared_early_stop_case(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    founder: u64,
    case: usize,
) -> Result<(CpuCanonicalHost, Scene)> {
    prepared_instance(
        actor,
        asset,
        founder,
        case,
        EARLY_STOP_SEEDS[case],
        853 + 31 * case,
    )
}
pub(super) const CARE_SEEDS: [u64; 6] = [
    12_104_621, 12_210_103, 12_309_269, 12_402_009, 12_508_237, 12_610_457,
];
pub(super) fn prepared_care_case(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    founder: u64,
    case: usize,
) -> Result<(CpuCanonicalHost, Scene)> {
    prepared_instance(
        actor,
        asset,
        founder,
        case,
        CARE_SEEDS[case],
        1103 + 37 * case,
    )
}
pub(super) fn prepared_replay_case(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    founder: u64,
    case: usize,
) -> Result<(CpuCanonicalHost, Scene)> {
    prepared_instance(
        actor,
        asset,
        founder,
        case,
        REPLAY_SEEDS[case],
        617 + 29 * case,
    )
}
// Food angle, toy angle, food distance, toy distance, food nutrition, food radius.
const INSTANCES: [[f32; 6]; 6] = [
    [0.36, -0.79, 0.37, 0.71, 0.85, 0.105],
    [-0.61, 0.77, 0.47, 0.75, 1.10, 0.135],
    [0.51, -0.78, 0.65, 0.79, 0.95, 0.115],
    [-0.68, 0.73, 0.73, 0.82, 1.15, 0.130],
    [0.57, -0.81, 0.55, 0.74, 0.90, 0.110],
    [-0.43, 0.80, 0.60, 0.81, 1.05, 0.125],
];
fn padding(case: usize) -> usize {
    173 + 19 * case
}
pub(super) fn prepared_case(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    founder: u64,
    case: usize,
) -> Result<(CpuCanonicalHost, Scene)> {
    prepared_instance(actor, asset, founder, case, SEEDS[case], padding(case))
}
pub(super) fn prepared_fresh_case(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    founder: u64,
    case: usize,
) -> Result<(CpuCanonicalHost, Scene)> {
    prepared_instance(
        actor,
        asset,
        founder,
        case,
        FRESH_SEEDS[case],
        379 + 23 * case,
    )
}
pub(super) fn prepared_frozen_case(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    founder: u64,
    index: usize,
) -> Result<(CpuCanonicalHost, Scene)> {
    let c = super::frozen_causal::CASES[index];
    prepared_instance(actor, asset, founder, c.fixture, c.seed, c.padding)
}
fn prepared_instance(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    founder: u64,
    case: usize,
    seed: u64,
    identity_padding: usize,
) -> Result<(CpuCanonicalHost, Scene)> {
    let [_, _, _, _, nutrition, radius] = INSTANCES[case];
    Ok(CpuCanonicalHost::new_prepared_cohort(
        game(seed, founder, asset)?,
        actor,
        seed as u32,
        |w| {
            let s = arrange(w, 1, true)?;
            remap_with_food(
                w,
                s,
                identity_padding,
                case.is_multiple_of(2),
                nutrition,
                radius,
            )
        },
    )?)
}
pub(super) fn place_case(w: &mut HeadlessWorld, s: Scene, case: usize) -> Result<()> {
    let body = w
        .entity(w.organism_entity_ids()[0].1)
        .ok_or("no body")?
        .clone();
    let [fa, ta, fd, td, _, _] = INSTANCES[case];
    for (id, angle, distance) in [(s.food, fa, fd), (s.toy, ta, td)] {
        if w.entity(id).is_some_and(|e| e.carried_by.is_some()) {
            return Err("cannot relocate held transfer object".into());
        }
        w.editor_move_object(
            id,
            Vec3f::new(
                body.position.x + distance * (body.body_yaw + angle).cos(),
                body.position.y,
                body.position.z + distance * (body.body_yaw + angle).sin(),
            ),
        )?;
    }
    Ok(())
}
fn plan() -> AssessmentPlan {
    AssessmentPlan {
        recovery_sleep: true,
        collect_utterance_timing: true,
        factorial_grounding: true,
        paired_request_focus: true,
        linear_readout: true,
        balanced_grounding_v1: false,
        balanced_grounding_v2: true,
        event_confirmed_speech_v1: true,
        unseen_report_transfer_v1: true,
        physical_before_speech_v1: false,
        captured_failure_replay_v1: false,
        replay_early_stop_v1: false,
        care_retention_v1: false,
        frozen_causal_v1: false,
    }
}
pub fn assess_unseen_grounding(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    if c.llamacpp_translation || !c.experimental_word_bridge || c.decisions != 24 {
        return Err("frozen transfer requires scripted public V3 actors and decisions24".into());
    }
    let old: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    if old["kind"] != "experimental-report-choice-noun-margin-v18"
        || old["config"]["seed"] != c.seed
    {
        return Err("frozen transfer requires its matching measured V18 source".into());
    }
    fs::create_dir(output)?;
    let started = Instant::now();
    let mut arms = vec![];
    for arm in [Arm::Untreated, Arm::Grounded, Arm::Shuffled, Arm::NoSpeech] {
        let recorded = old["arms"]
            .as_array()
            .ok_or("missing V18 arms")?
            .iter()
            .find(|a| a["arm"] == json!(arm))
            .ok_or("missing source arm")?;
        let recovery = read_recovery(&source.join(format!("recovery-{arm:?}")))?;
        if recovery.inputs.experimental_language_input()
            != Some(ExperimentalLanguageInputV1::LinearWordReadoutV3)
        {
            return Err("not a retained public V3 actor".into());
        }
        let phenotype =
            PhenotypeCompiler::compile_validated(&recovery.inputs, &BrainCapacityClass::n2048())?;
        let asset = FoundationWeightAsset::decode_canonical(&recovery.foundation)?;
        let actor =
            CpuFoundationTrainer::from_checkpoint(phenotype, asset.clone(), recovery.actor)?;
        let before = identity(&actor.checkpoint())?;
        let private = identity(
            &CpuCanonicalHost::from_checkpoint(recovery.host, &actor)?.checkpoint(&actor)?,
        )?;
        if recorded["actor_sha256"] != before
            || recorded["continuity"]["native_private_and_optimizer_hashes"]["before_host"]
                != private
        {
            return Err("frozen source actor or private state differs from measured V18".into());
        }
        eprintln!("unseen frozen {arm:?}: native assessment and cold repeat");
        let cold = exact_reload_plan(
            &output.join(format!("recovery-{arm:?}")),
            &actor,
            &asset,
            RecoveredAssessment {
                config: c,
                arm,
                curriculum: Some(plan()),
            },
            recovery.next_gradient,
        )?;
        if identity(&actor.checkpoint())? != before {
            return Err("frozen actor changed during transfer assessment".into());
        }
        arms.push(json!({"arm":arm,"common_source_actor_sha256":recorded["common_source_actor_sha256"],"source_actor_sha256":before,"source_private_sha256":private,"source_step":actor.optimizer_step(),"actor_step":actor.optimizer_step(),"actor_sha256":before,"collections":[],"updates":[],"evaluation":cold["native_private_and_optimizer_hashes"]["assessment"]["evaluation"],"continuity":cold,"source_history_retained":true,"actor_weights_moments_integer_ages_config_unchanged":true}));
    }
    let instances = INSTANCES.iter().enumerate().map(|(case, values)|json!({"case":case,"world_seed":SEEDS[case],"identity_padding":padding(case),"toy_spawned_first":case.is_multiple_of(2),"food_angle_toy_angle_food_distance_toy_distance_nutrition_radius":values})).collect::<Vec<_>>();
    let result = json!({"kind":"experimental-unseen-frozen-grounding-transfer-v19","execution_domain":"native-cpu","config":c,"source_report_sha256":identity(&old)?,"held_out_world_seeds":SEEDS,"instances":instances,"independent_seed_count":SEEDS.len(),"independent_trained_founders":1,"pre_post_are_paired_not_independent":true,"teacher_and_player_cue_ticks":INSTRUCTION_TICKS,"evaluation_ticks":EVALUATION_TICKS,"training_updates":0,"arms":arms,"elapsed_seconds":started.elapsed().as_secs_f64(),"semantic_prior_enabled":false,"teacher":"absent; frozen actors retain scripted-teacher history","limitations":["Six new environment/sampling seeds and new physical instances; one retained trained founder, not six independent training replicates.","Pre/post induced RecoveryProtocol sleep is paired, not independent; no spontaneous sleep or awake-delay causal control.","Familiar token combinations only; literal first-phrase naming is separate from timely physical grounding and silence.","Natural preparation and cooldown retained; no forced evaluation action or fabricated transition/reward.","No real SLM request, Luna 401 retry, source-state mutation, live/default-brain change or promotion."]});
    write_json(&output.join("results.json"), &result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_instances_remain_visible_edible_and_nourishing() {
        let founder = 847501;
        let asset = initial_n2048_care_asset(founder).unwrap();
        let actor = new_actor_profile(founder, &asset, true).unwrap();
        for variant in 0..5 {
            for case in 0..SEEDS.len() {
                let (mut h, s) = if variant == 4 {
                    prepared_care_case(&actor, &asset, founder, case).unwrap()
                } else if variant == 3 {
                    prepared_early_stop_case(&actor, &asset, founder, case).unwrap()
                } else if variant == 2 {
                    prepared_replay_case(&actor, &asset, founder, case).unwrap()
                } else if variant == 1 {
                    prepared_fresh_case(&actor, &asset, founder, case).unwrap()
                } else {
                    prepared_case(&actor, &asset, founder, case).unwrap()
                };
                place_case(h.world_mut(), s, case).unwrap();
                assert!(s.food.raw() > 170 && s.toy.raw() > 170);
                let id = h.world().organism_entity_ids()[0].0;
                let homeostasis = h
                    .world()
                    .organism_registry()
                    .get(id)
                    .unwrap()
                    .biochemistry()
                    .homeostasis;
                let tick = h.world().tick();
                let frame = h
                    .world_mut()
                    .perception_frame(id, tick, actor.phenotype().sensor_profile(), homeostasis)
                    .unwrap();
                assert!(
                    [s.food, s.toy, s.distractor]
                        .into_iter()
                        .all(|t| visible_target(&frame, t)),
                    "transfer visibility {case}"
                );
                assert!(
                    !super::super::tests::eat_control(&mut h, &actor, s)
                        .unwrap()
                        .0
                );
                let (consumed, gain) = super::super::tests::eat_control(&mut h, &actor, s)
                    .unwrap_or_else(|| panic!("held food occluded in transfer case {case}"));
                assert!(consumed && gain > 0., "transfer fixture {case}");
            }
        }
    }
    #[test]
    fn old_disposable_plan_json_roundtrips_without_new_fields() {
        let old = json!({"recovery_sleep":true,"collect_utterance_timing":true,"factorial_grounding":true,"paired_request_focus":true,"linear_readout":true,"balanced_grounding_v2":true,"event_confirmed_speech_v1":true});
        let restored: AssessmentPlan = serde_json::from_value(old.clone()).unwrap();
        assert!(!restored.unseen_report_transfer_v1);
        assert_eq!(serde_json::to_value(restored).unwrap(), old);
        assert!(
            serde_json::from_value::<AssessmentPlan>(serde_json::to_value(plan()).unwrap())
                .unwrap()
                .unseen_report_transfer_v1
        );
    }
}

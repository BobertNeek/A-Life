//! CPU care episodes reuse the native lesson world, prior and outcome observer.
use crate::foundation_training::{
    care_meal_completed, close_foundation_navigation_gate_in_world,
    configure_foundation_scenario_with_food_need, expose_foundation_teacher_cue,
    food_outcome::{ownership, FoodOutcomeObservation, FoodOwnershipSnapshot},
    grounded_navigation_completed, heard_foundation_teacher_cue, outcome_consumed,
    relocate_unheld_foundation_food, teacher_outcome_blocked, teacher_outcome_moved,
    write_foundation_lesson_prior_readiness, FoundationScenarioSetup,
};
use crate::gpu_live_runtime::semantic_prior::RuntimeSemanticPrior;
use crate::{FoundationFoodNeedCondition, FoundationFoodOutcome, FoundationTeacherLesson};
use alife_core::{
    ExperienceSequenceId, PerceptionFrameDraft, ScaffoldContractError, SensorProfile, Validate,
};
use alife_training::cpu::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};
mod admission;
mod evaluation;
mod round;
pub use evaluation::*;
pub use round::*;
type Result<T> = CpuContinuationResult<T>;

pub const CPU_CARE_FRESHNESS_UNKNOWN_CLAIM_SCOPE: &str = "historical freshness unknown; disclosed exclusions only; no claim of unseen-seed generalization or synergy";
const UNKNOWN_FRESHNESS_CONTEXT_FIELDS: [&str; 3] =
    ["known_exclusions", "seed_admission", "claim_scope"];

fn add_unknown_freshness_context(value: &mut Value, known_exclusions: bool) {
    if known_exclusions {
        value["known_exclusions"] = json!(true);
        value["seed_admission"] =
            json!({"policy":"known-exclusions","historical_freshness":"unknown"});
        value["claim_scope"] = json!(CPU_CARE_FRESHNESS_UNKNOWN_CLAIM_SCOPE);
    }
}

pub struct CpuCareRequest {
    pub previous: Option<PathBuf>,
    pub output: PathBuf,
    pub seed: u64,
    pub decisions: usize,
    pub temperature: f32,
    pub lesson: FoundationTeacherLesson,
    pub food_need_condition: FoundationFoodNeedCondition,
    pub seed_history: Option<PathBuf>,
    pub known_exclusions: bool,
    /// Optional bulky debug recording; the in-memory training rows are unchanged.
    pub record_diagnostic_replay: bool,
    /// Development fixtures only; this never imports a retained policy.
    pub fixture: bool,
}

pub fn reconcile_cpu_care_attempt(lane: &std::path::Path) -> Result<Value> {
    admission::reconcile(lane)
}

pub fn validate_cpu_care_output(output: &std::path::Path) -> Result<Value> {
    let receipt = crate::foundation_training_cycle::read_native_cpu_care_evidence(
        &output.join("cpu-cycle.json"),
    )?
    .1;
    admission::validate_care_output(output, &receipt)?;
    Ok(receipt)
}

pub(crate) fn trained_care_meal_completed(
    summary: &FoundationFoodOutcome,
    train_rows: usize,
    meal_rows: &[usize],
) -> bool {
    meal_rows.iter().any(|row| *row < train_rows)
        && care_meal_completed(Some(summary), summary.organism, summary.target)
}

fn prepare_care_episode(request: CpuCareRequest) -> Result<(CpuContinuationConfig, CareEpisode)> {
    if !matches!(
        request.lesson,
        FoundationTeacherLesson::EatHeldFood
            | FoundationTeacherLesson::EatReachableFood
            | FoundationTeacherLesson::Feeding
            | FoundationTeacherLesson::ObstacleNavigation
            | FoundationTeacherLesson::RelocatedFood
    ) || !(1..=128).contains(&request.decisions)
        || (!request.fixture && request.seed_history.is_none())
        || request.fixture && (request.known_exclusions || request.seed_history.is_some())
    {
        return Err("CPU care requires an explicit supported care lesson, 1..=128 decisions and exact seed history (or an unreserved development fixture)".into());
    }
    request
        .food_need_condition
        .validate_lesson(Some(request.lesson))?;
    let config = CpuContinuationConfig {
        previous: request.previous.clone(),
        output: request.output.clone(),
        seed: request.seed,
        ticks: request.decisions + 1,
        temperature: request.temperature,
        fixture: request.fixture,
    };
    let episode = CareEpisode {
        request,
        scenario: None,
        prior: None,
        summary: None,
        ownership_before: None,
        traces: Vec::new(),
        meal_rows: Vec::new(),
        navigation_rows: Vec::new(),
        relocation_row: None,
        relocation_target: None,
        cue_frames: 0,
        generic_consumed: 0,
        reservation: None,
        collection_binding: None,
        explicit_deterministic_prior: false,
        assessment: false,
    };
    Ok((config, episode))
}

pub fn run_cpu_care(request: CpuCareRequest) -> Result<Value> {
    let (config, mut episode) = prepare_care_episode(request)?;
    let result = run_cpu_continuation(config, &mut episode);
    if let Some(reservation) = episode.reservation.as_mut() {
        match &result {
            Ok(receipt) => reservation.accept(receipt)?,
            Err(error) => reservation.fail(&error.to_string())?,
        }
    }
    result
}

struct CareEpisode {
    request: CpuCareRequest,
    scenario: Option<FoundationScenarioSetup>,
    prior: Option<RuntimeSemanticPrior>,
    summary: Option<FoundationFoodOutcome>,
    ownership_before: Option<FoodOwnershipSnapshot>,
    traces: Vec<Value>,
    meal_rows: Vec<usize>,
    navigation_rows: Vec<(bool, bool, bool)>,
    relocation_row: Option<usize>,
    relocation_target: Option<alife_core::Vec3f>,
    cue_frames: usize,
    generic_consumed: usize,
    reservation: Option<admission::CareSeedReservation>,
    collection_binding: Option<(CpuCollectorAssignmentV1, [u8; 32])>,
    explicit_deterministic_prior: bool,
    assessment: bool,
}

impl CareEpisode {
    fn private_life_archive(
        &self,
        host: &CpuCanonicalHost,
        output: &std::path::Path,
    ) -> Result<(String, Value)> {
        let policy_sha = format!(
            "{:x}",
            Sha256::digest(std::fs::read(
                output.join("collection-policy.alife-foundation")
            )?)
        );
        let mut lives = BTreeMap::new();
        if let Some(prior) = self.prior.as_ref() {
            for record in host.world().organism_registry().iter() {
                lives.insert(
                    record.organism_id().raw(),
                    prior.snapshot(record.organism_id().raw())?,
                );
            }
        }
        let archive = json!({"schema":1,"collection_policy_sha256":policy_sha,
            "provider_identity":self.prior.as_ref().map(|p| &p.metrics.provider_identity),
            "world_tick":host.world().tick(),"lives":lives,
            "restore_contract":"restore host checkpoint under collection policy and restore_life for the same organism before further dispatch"});
        Ok((policy_sha, archive))
    }
}

impl CpuEpisodeHooks for CareEpisode {
    fn preserve_collection_recovery(
        &mut self,
        host: &CpuCanonicalHost,
        _founder: &CpuFoundationTrainer,
        output: &std::path::Path,
    ) -> Result<()> {
        let (_, archive) = self.private_life_archive(host, output)?;
        admission::write_atomic(&output.join("care-prior-lives.json"), &archive).map_err(
            |error| {
                format!(
                    "care prior recovery archive at {} could not be sealed: {error}",
                    output.display()
                )
            },
        )?;
        if self.assessment {
            let rows = self
                .traces
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    let mut row = row.clone();
                    row["loss_row"] = json!(false);
                    row["evaluation_row"] = json!(i < self.request.decisions);
                    row["bootstrap_row"] = json!(i >= self.request.decisions);
                    row["teacher_heard"] = json!(false);
                    row
                })
                .collect::<Vec<_>>();
            admission::write_atomic(
                &output.join("care-trace.json"),
                &json!({
                    "schema":1,"backend":"native-cpu","lesson":self.request.lesson,
                    "complete":false,"recovery_prefix":true,"world_tick":host.world().tick(),"rows":rows
                }),
            )?;
        }
        Ok(())
    }
    fn decision_budget(&self) -> Option<usize> {
        Some(self.request.decisions)
    }
    fn fixture_asset(&self, seed: u64) -> Result<Option<alife_core::FoundationWeightAsset>> {
        Ok(Some(crate::initial_n2048_care_asset(seed)?))
    }
    fn admit(
        &mut self,
        config: &CpuContinuationConfig,
        _source: Option<&Value>,
        _hashes: &BTreeMap<String, String>,
    ) -> Result<Value> {
        if config.fixture {
            return Ok(
                json!({"allocation":"development-fixture-only","historical_freshness":"unknown"}),
            );
        }
        if self.assessment {
            let provenance =
                crate::foundation_training_cycle::admit_native_cpu_care_assessment_seed(
                    config
                        .previous
                        .as_deref()
                        .ok_or("assessment source missing")?,
                    self.request
                        .seed_history
                        .as_deref()
                        .ok_or("assessment history missing")?,
                    &config.output,
                    config.seed,
                    self.request.known_exclusions,
                )?;
            return Ok(json!({"allocation":"assessment-no-training",
                "seed_admission":provenance["seed_admission"],"source_sha256":provenance["source_sha256"]}));
        }
        let reservation = match &self.collection_binding {
            Some((assignment, plan)) => admission::CareSeedReservation::reserve_collection(
                config,
                &self.request,
                assignment,
                *plan,
            )?,
            None => admission::CareSeedReservation::reserve(config, &self.request)?,
        };
        let provenance = reservation.provenance["seed_admission"].clone();
        self.reservation = Some(reservation);
        Ok(provenance)
    }
    fn validate_policy(&mut self, founder: &CpuFoundationTrainer) -> Result<()> {
        if founder.phenotype().sensor_profile() != SensorProfile::GroundedTerrainVisionV1 {
            return Err("CPU care requires the retained grounded natural-senses policy; sensor conversion is not continuation".into());
        }
        self.prior = if self.assessment {
            None
        } else if self.explicit_deterministic_prior {
            Some(RuntimeSemanticPrior::deterministic_for_training(
                self.request.seed,
            ))
        } else {
            RuntimeSemanticPrior::from_environment(self.request.seed, true)?
        };
        if matches!(
            self.request.lesson,
            FoundationTeacherLesson::EatHeldFood | FoundationTeacherLesson::EatReachableFood
        ) {
            if let Some(prior) = self.prior.as_mut() {
                prior.require_rich_information();
            }
        }
        Ok(())
    }
    fn prepare_world(&mut self, world: &mut alife_world::HeadlessWorld) -> Result<()> {
        let mut scenario = configure_foundation_scenario_with_food_need(
            world,
            self.request.seed,
            Some(self.request.lesson),
            None,
            true,
            self.request.food_need_condition,
        )?;
        if self.assessment {
            evaluation::ensure_no_teacher(world)?;
            scenario.silent_request = true;
            scenario.teacher_cue_tokens.clear();
        }
        let learner = world
            .organism_entity_ids()
            .first()
            .ok_or("care learner missing")?
            .0;
        if self.request.lesson == FoundationTeacherLesson::RelocatedFood {
            let body = world.organism_entity_ids()[0].1;
            let origin = world.entity(body).ok_or("learner body missing")?.position;
            let start = world.entity(scenario.food).ok_or("food missing")?.position;
            // One sideways relocation, not continuous pursuit or an action hint.
            let side = if self.request.seed & 1 == 0 {
                1.0
            } else {
                -1.0
            };
            self.relocation_target = Some(alife_core::Vec3f::new(
                origin.x - side * (start.z - origin.z),
                start.y,
                origin.z + side * (start.x - origin.x),
            ));
        }
        self.summary = Some(FoundationFoodOutcome::new(learner, scenario.food));
        self.scenario = Some(scenario);
        Ok(())
    }
    fn before_step(
        &mut self,
        host: &mut CpuCanonicalHost,
        founder: &CpuFoundationTrainer,
    ) -> Result<()> {
        let scenario = self.scenario.as_ref().ok_or("care scene is not prepared")?;
        close_foundation_navigation_gate_in_world(host.world_mut(), scenario)?;
        if self.relocation_row.is_none()
            && host
                .world()
                .tick()
                .raw()
                .saturating_sub(scenario.start_tick)
                >= 2
        {
            if let Some(position) = self.relocation_target {
                if relocate_unheld_foundation_food(host.world_mut(), scenario.food, position)? {
                    self.relocation_row = Some(self.traces.len());
                }
            }
        }
        if self.assessment {
            evaluation::ensure_no_teacher(host.world())?;
        } else {
            expose_foundation_teacher_cue(host.world_mut(), scenario)?;
        }
        if matches!(
            self.request.lesson,
            FoundationTeacherLesson::EatHeldFood | FoundationTeacherLesson::EatReachableFood
        ) {
            if let Some(prior) = self.prior.as_mut() {
                for (draft, sequence) in host.semantic_prior_inputs(founder)? {
                    prior.prime_with_receipt(draft, sequence)?;
                }
            }
        }
        self.ownership_before = ownership(host.world(), scenario.food);
        Ok(())
    }
    fn prepare_draft(
        &mut self,
        draft: PerceptionFrameDraft,
        sequence: ExperienceSequenceId,
    ) -> std::result::Result<PerceptionFrameDraft, ScaffoldContractError> {
        match self.prior.as_mut() {
            Some(prior) => prior.prepare(draft, sequence),
            None => Ok(draft),
        }
    }
    fn after_step(
        &mut self,
        host: &CpuCanonicalHost,
        founder: &CpuFoundationTrainer,
        step: &CpuCanonicalStep,
    ) -> Result<()> {
        if self.assessment {
            evaluation::ensure_no_teacher(host.world())?;
        }
        let scenario = self.scenario.as_ref().ok_or("care scene missing")?;
        let summary = self
            .summary
            .as_mut()
            .ok_or("care outcome identity missing")?;
        for row in &step.decisions {
            row.patch.validate_contract()?;
            let frame = row.patch.pre_action().perception();
            if self.assessment && evaluation::teacher_heard(frame) {
                return Err("assessment heard a teacher; retain private recovery without qualifying results".into());
            }
            let evidence = row.patch.decision().neural_evidence()?;
            if row.organism_id != summary.organism
                || frame.organism_id() != row.organism_id
                || row.patch.decision().policy_backend()
                    != alife_core::PolicyBackend::NeuralClosedLoopCpu
                || evidence.frame_digest != frame.frame_digest()
            {
                return Err("care replay has inconsistent CPU learner identity".into());
            }
            if let Some(prior) = self.prior.as_mut() {
                prior.record_encoded_decision_input(
                    frame,
                    row.patch.header().sequence_id.raw(),
                    evidence.dispatch_generation,
                    founder.phenotype(),
                );
            }
            self.cue_frames += usize::from(heard_foundation_teacher_cue(
                frame,
                &scenario.teacher_cue_tokens,
            ));
            self.generic_consumed += usize::from(outcome_consumed(row.patch.outcome()));
            let observation = FoodOutcomeObservation {
                organism: summary.organism,
                target: summary.target,
                decision_tick: step.tick_before,
                outcome_tick: step.tick_after,
                before: self.ownership_before,
                after: ownership(host.world(), summary.target),
            };
            let before = summary.nourished_meal_events;
            let before_acquisitions = summary.ordinary_acquisition_events;
            let before_consumptions = summary.consumption_events;
            summary.observe(observation, &row.patch);
            let index = self.traces.len();
            if summary.nourished_meal_events > before {
                self.meal_rows.push(index);
            }
            let mut trace = json!({"row":index,"organism":row.organism_id,"sequence":row.patch.header().sequence_id,
                "decision_tick":step.tick_before,"outcome_tick":step.tick_after,
                "frame_digest":frame.frame_digest(),"dispatch_generation":evidence.dispatch_generation,
                "action":row.patch.decision().selected_action,"bundle":row.patch.selected_bundle(),
                "outcome":row.patch.outcome(),"ppo_action":row.action,
                "nourished_meal":summary.nourished_meal_events>before});
            if self.assessment {
                trace["acquisition_event"] =
                    json!(summary.ordinary_acquisition_events > before_acquisitions);
                trace["consumption_event"] =
                    json!(summary.consumption_events > before_consumptions);
            }
            if self.request.lesson == FoundationTeacherLesson::RelocatedFood {
                trace["food_relocated"] = json!(self.relocation_row.is_some());
                trace["food_position"] = json!(host
                    .world()
                    .entity(summary.target)
                    .map(|food| food.position));
            }
            if self.request.lesson == FoundationTeacherLesson::ObstacleNavigation {
                let visible = frame
                    .candidates()
                    .iter()
                    .any(|candidate| candidate.target.entity == Some(summary.target));
                let blocked = teacher_outcome_blocked(row.patch.outcome());
                let moved = teacher_outcome_moved(row.patch.outcome());
                self.navigation_rows.push((visible, blocked, moved));
                trace["food_visible"] = json!(visible);
                trace["navigation_blocked"] = json!(blocked);
                trace["navigation_moved"] = json!(moved);
            }
            if let Some((gate, position)) = scenario.closing_gate {
                trace["navigation_gate_closed"] = json!(host
                    .world()
                    .entity(gate)
                    .is_some_and(|object| object.position == position));
            }
            observation.decorate_trace(&mut trace);
            self.traces.push(trace);
        }
        Ok(())
    }
    fn finish_collection(
        &mut self,
        host: &CpuCanonicalHost,
        _founder: &CpuFoundationTrainer,
        rows: &BTreeMap<u64, Vec<CpuCollectedDecision>>,
        train_rows: usize,
        output: &std::path::Path,
    ) -> Result<CpuEpisodeReceipt> {
        let scenario = self.scenario.as_ref().ok_or("care setup missing")?;
        let summary = self.summary.as_ref().ok_or("care outcome missing")?;
        let trained_meals = self.meal_rows.iter().filter(|i| **i < train_rows).count();
        for (i, trace) in self.traces.iter_mut().enumerate() {
            trace["loss_row"] = json!(!self.assessment && i < train_rows);
            if self.assessment {
                trace["evaluation_row"] = json!(i < train_rows);
                trace["bootstrap_row"] = json!(i >= train_rows);
                trace["teacher_heard"] = json!(false);
            }
        }
        let (policy_sha, private_lives) = self.private_life_archive(host, output)?;
        let write = |name: &str, value: &Value| -> Result<()> {
            Ok(std::fs::write(
                output.join(name),
                serde_json::to_vec(value)?,
            )?)
        };
        if self.request.record_diagnostic_replay {
            write(
                "care-replay.json",
                &json!({"schema":1,"backend":"native-cpu","collection_policy_sha256":policy_sha,"rows":rows}),
            )?;
        }
        let trace = json!({"schema":1,"backend":"native-cpu","lesson":self.request.lesson,"rows":self.traces});
        if self.assessment {
            admission::write_atomic(&output.join("care-trace.json"), &trace)?;
        } else {
            write("care-trace.json", &trace)?;
        }
        admission::write_atomic(&output.join("care-prior-lives.json"), &private_lives)?;
        if let Some(reservation) = self.reservation.as_ref() {
            // Full history/provenance is retained once outside recursive native
            // receipts. Only policy/freshness belongs in seed_admission.
            write("care-seed-admission.json", &reservation.provenance)?;
        }
        write_foundation_lesson_prior_readiness(
            self.prior.as_ref().map(|p| &p.metrics),
            output,
            Some(self.request.lesson),
        )?;
        let meal_completed = trained_care_meal_completed(summary, train_rows, &self.meal_rows);
        let lesson_completed = if self.request.lesson == FoundationTeacherLesson::ObstacleNavigation
        {
            grounded_navigation_completed(
                meal_completed,
                self.navigation_rows.iter().take(train_rows).copied(),
            )
        } else if self.request.lesson == FoundationTeacherLesson::RelocatedFood {
            meal_completed
                && self.relocation_row.is_some_and(|relocated| {
                    self.meal_rows
                        .iter()
                        .any(|row| *row >= relocated && *row < train_rows)
                })
        } else {
            meal_completed
        };
        let mut metadata = json!({"schema":1,"kind":"care-first-cpu-episode","lesson":self.request.lesson,
            "episode_body":"new-prepared-cohort","persistent_learning":"native-founder-and-full-actor-value-optimizer",
            "raw_replay_recorded":self.request.record_diagnostic_replay,
            "setup_ticks_are_training_rows":false,"setup_tick":scenario.start_tick,
            "food_need_initialization":scenario.food_need_initialization,
            "navigation_gate":scenario.closing_gate,
            "food_relocation_target":self.relocation_target,"food_relocation_row":self.relocation_row,
            "held_food_setup":scenario.held_food_setup,"reachable_food_setup":scenario.reachable_food_setup,
            "teacher_cue_tokens":scenario.teacher_cue_tokens,"teacher_cue_frames":self.cue_frames,
            "food_outcome":summary,"generic_consumed_events":self.generic_consumed,
            "captured_nourished_meals":summary.nourished_meal_events,"trained_nourished_meals":trained_meals,
            "bootstrap_nourished_meals":summary.nourished_meal_events-trained_meals,
            "lesson_completed":lesson_completed,
            "curriculum_progression":"none; explicit supported care schedule only",
            "semantic_prior":self.prior.as_ref().map(|p| &p.metrics),
            "prior_failure_policy":"log_and_continue_without_prior",
            "critic_optimizer":alife_training::PpoConfig::default().value_optimizer});
        if let Some((_, plan)) = &self.collection_binding {
            metadata["care_round_plan_sha256"] = json!(plan);
            metadata["comparison_role"] = json!("serial-collection-parity-control");
        }
        if self.assessment {
            metadata["kind"] = json!("care-cpu-assessment");
            metadata["teacher_present"] = json!(false);
            metadata["teacher_cue_frames"] = json!(0);
            metadata["semantic_prior_enabled"] = json!(false);
            metadata["prior_failure_policy"] = json!("disabled-independent-of-environment");
            metadata["evaluated_decisions"] = json!(train_rows);
            metadata["evaluated_nourished_meals"] = json!(trained_meals);
            metadata["evaluated_ordinary_acquisitions"] = json!(self
                .traces
                .iter()
                .take(train_rows)
                .filter(|r| r["acquisition_event"] == true)
                .count());
            metadata["evaluated_consumptions"] = json!(self
                .traces
                .iter()
                .take(train_rows)
                .filter(|r| r["consumption_event"] == true)
                .count());
            metadata["trained_nourished_meals"] = json!(0);
            metadata["offline_actor_optimizer_updates"] = json!(0);
            metadata["offline_value_optimizer_updates"] = json!(0);
            metadata["persistent_learning"] = json!("fresh-private-life-with-ordinary-biology-waking-learning-and-sleep; frozen-founder-and-full-optimizers");
        }
        add_unknown_freshness_context(&mut metadata, self.request.known_exclusions);
        write("care-episode.json", &metadata)?;
        let private_path = cpu_private_checkpoint_path(output)?;
        let private_name = private_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid private checkpoint path")?;
        let mut artifacts = vec![
            "care-trace.json",
            "care-prior-lives.json",
            "care-episode.json",
            private_name,
            "collection-policy.alife-foundation",
            "collection-compiler-inputs.json",
            "collection-actor-checkpoint.json",
            "initial-value-checkpoint.json",
        ];
        if self.request.record_diagnostic_replay {
            artifacts.push("care-replay.json");
        }
        if self.reservation.is_some() {
            artifacts.push("care-seed-admission.json");
        }
        if output.join("semantic-prior-readiness.json").is_file() {
            artifacts.push("semantic-prior-readiness.json");
        }
        Ok(CpuEpisodeReceipt {
            metadata,
            artifacts: artifacts.into_iter().map(str::to_owned).collect(),
        })
    }
}

#[cfg(test)]
mod collect_only_tests {
    use super::*;

    #[test]
    fn cpu_need_condition_prepares_before_capture_without_changing_lesson_cues() {
        for lesson in [
            FoundationTeacherLesson::Feeding,
            FoundationTeacherLesson::RelocatedFood,
        ] {
            let (_, mut episode) = prepare_care_episode(CpuCareRequest {
                previous: None,
                output: PathBuf::from("unused-setup-only"),
                seed: 42,
                decisions: 3,
                temperature: 1.0,
                lesson,
                food_need_condition: FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
                seed_history: None,
                known_exclusions: false,
                record_diagnostic_replay: false,
                fixture: true,
            })
            .unwrap();
            let asset = crate::initial_n2048_care_asset(42).unwrap();
            let mut config = alife_world::CanonicalNewGameConfig::phase3(42, 1).unwrap();
            config.brain_class = alife_core::BrainScaleTier::Standard2048;
            config.sensor_profile = asset.manifest().sensor_profile();
            let mut world =
                alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)
                    .unwrap()
                    .world;
            episode.prepare_world(&mut world).unwrap();
            let scene = episode.scenario.as_ref().unwrap();
            let init = scene.food_need_initialization.as_ref().unwrap();
            assert_eq!(
                init.condition,
                FoundationFoodNeedCondition::OrdinaryHealthyAgingV1
            );
            assert_eq!(init.prepared.tick, scene.start_tick);
            assert!(scene.start_tick > 0);
            assert!(scene.teacher_cue_tokens.is_empty());
            assert!(episode.traces.is_empty() && episode.meal_rows.is_empty());
            assert!(episode.relocation_row.is_none());
            assert_eq!(
                episode.relocation_target.is_some(),
                lesson == FoundationTeacherLesson::RelocatedFood
            );
        }
    }

    #[test]
    fn interrupted_care_collection_retains_matching_host_and_private_prior_lives() {
        use alife_core::{BrainCapacityClass, PhenotypeCompiler, PhenotypeCompilerInputs};
        use alife_training::{FoundationTrainerCheckpoint, PpoValueHeadCheckpoint};
        use std::path::Path;

        struct InterruptedCare {
            episode: CareEpisode,
            ticks: usize,
            started_tick: Option<u64>,
            failure_tick: Option<u64>,
            recovery: Option<(FoundationTrainerCheckpoint, CpuCanonicalCheckpoint)>,
        }
        impl CpuEpisodeHooks for InterruptedCare {
            fn decision_budget(&self) -> Option<usize> {
                self.episode.decision_budget()
            }
            fn fixture_asset(
                &self,
                seed: u64,
            ) -> Result<Option<alife_core::FoundationWeightAsset>> {
                self.episode.fixture_asset(seed)
            }
            fn admit(
                &mut self,
                config: &CpuContinuationConfig,
                source: Option<&Value>,
                hashes: &BTreeMap<String, String>,
            ) -> Result<Value> {
                self.episode.admit(config, source, hashes)
            }
            fn validate_policy(&mut self, founder: &CpuFoundationTrainer) -> Result<()> {
                self.episode.validate_policy(founder)
            }
            fn prepare_world(&mut self, world: &mut alife_world::HeadlessWorld) -> Result<()> {
                self.episode.prepare_world(world)
            }
            fn before_step(
                &mut self,
                host: &mut CpuCanonicalHost,
                founder: &CpuFoundationTrainer,
            ) -> Result<()> {
                self.started_tick.get_or_insert(host.world().tick().raw());
                if self.ticks == 2 {
                    self.failure_tick = Some(host.world().tick().raw());
                    return Err("synthetic care interruption after two native ticks".into());
                }
                self.episode.before_step(host, founder)
            }
            fn prepare_draft(
                &mut self,
                draft: PerceptionFrameDraft,
                sequence: ExperienceSequenceId,
            ) -> std::result::Result<PerceptionFrameDraft, ScaffoldContractError> {
                self.episode.prepare_draft(draft, sequence)
            }
            fn after_step(
                &mut self,
                host: &CpuCanonicalHost,
                founder: &CpuFoundationTrainer,
                step: &CpuCanonicalStep,
            ) -> Result<()> {
                self.episode.after_step(host, founder, step)?;
                self.ticks += 1;
                Ok(())
            }
            fn finish_collection(
                &mut self,
                host: &CpuCanonicalHost,
                founder: &CpuFoundationTrainer,
                rows: &BTreeMap<u64, Vec<CpuCollectedDecision>>,
                train_rows: usize,
                output: &Path,
            ) -> Result<CpuEpisodeReceipt> {
                self.episode
                    .finish_collection(host, founder, rows, train_rows, output)
            }
            fn preserve_collection_recovery(
                &mut self,
                host: &CpuCanonicalHost,
                founder: &CpuFoundationTrainer,
                output: &Path,
            ) -> Result<()> {
                self.recovery = Some((founder.checkpoint(), host.checkpoint(founder)?));
                self.episode
                    .preserve_collection_recovery(host, founder, output)
            }
        }

        struct TemporaryOutput(PathBuf);
        impl Drop for TemporaryOutput {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let output = TemporaryOutput(std::env::temp_dir().join(format!(
            "alife-interrupted-care-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let root = &output.0;
        let seed = 0x93_42;
        let (config, mut episode) = prepare_care_episode(CpuCareRequest {
            previous: None,
            output: root.clone(),
            seed,
            decisions: 4,
            temperature: 1.,
            lesson: FoundationTeacherLesson::EatHeldFood,
            food_need_condition: crate::FoundationFoodNeedCondition::default(),
            seed_history: None,
            known_exclusions: false,
            record_diagnostic_replay: false,
            fixture: true,
        })
        .unwrap();
        // This constructor ignores ambient provider settings and shared caches.
        episode.explicit_deterministic_prior = true;
        let mut interrupted = InterruptedCare {
            episode,
            ticks: 0,
            started_tick: None,
            failure_tick: None,
            recovery: None,
        };
        let error = match collect_cpu_episode(config, &mut interrupted) {
            Ok(_) => panic!("interrupted care unexpectedly finalized a collector"),
            Err(error) => error,
        };
        assert_eq!(
            error.to_string(),
            "synthetic care interruption after two native ticks"
        );
        assert_eq!(interrupted.ticks, 2);
        let tick = interrupted.failure_tick.unwrap();
        assert_eq!(tick, interrupted.started_tick.unwrap() + 2);
        assert!(interrupted.episode.reservation.is_none());

        let actor: FoundationTrainerCheckpoint = serde_json::from_slice(
            &std::fs::read(root.join("collection-actor-checkpoint.json")).unwrap(),
        )
        .unwrap();
        let value: PpoValueHeadCheckpoint = serde_json::from_slice(
            &std::fs::read(root.join("initial-value-checkpoint.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(actor.optimizer_step, 0);
        assert_eq!(value.optimizer_step, 0);
        assert!(actor.update_ages.iter().all(|age| *age == 0));
        assert!(actor
            .first_moment
            .iter()
            .chain(&actor.second_moment)
            .all(|m| *m == 0.));
        assert!(value.parameters.iter().all(|parameter| *parameter == 0.));
        let checkpoint: CpuCanonicalCheckpoint = read_cpu_private_checkpoint(root).unwrap();
        assert!(checkpoint.sampling_counter > 0);
        assert!(checkpoint.residents.iter().any(|resident| resident
            .neural
            .eligibility
            .iter()
            .any(|trace| *trace != 0.)));
        let (recovery_actor, recovery_host) = interrupted.recovery.as_ref().unwrap();
        assert_eq!(recovery_actor, &actor);
        assert_eq!(
            serde_json::to_vec(recovery_host).unwrap(),
            serde_json::to_vec(&checkpoint).unwrap()
        );
        let inputs: PhenotypeCompilerInputs = serde_json::from_slice(
            &std::fs::read(root.join("collection-compiler-inputs.json")).unwrap(),
        )
        .unwrap();
        let phenotype =
            PhenotypeCompiler::compile_validated(&inputs, &BrainCapacityClass::n2048()).unwrap();
        let founder = CpuFoundationTrainer::from_checkpoint(
            phenotype,
            inputs.n2048_candidate_asset().unwrap().clone(),
            actor,
        )
        .unwrap();
        let host = CpuCanonicalHost::from_checkpoint(checkpoint, &founder).unwrap();
        assert_eq!(host.world().tick().raw(), tick);
        let archive: Value =
            serde_json::from_slice(&std::fs::read(root.join("care-prior-lives.json")).unwrap())
                .unwrap();
        assert_eq!(archive["world_tick"], tick);
        assert_eq!(
            archive["collection_policy_sha256"],
            format!(
                "{:x}",
                Sha256::digest(
                    std::fs::read(root.join("collection-policy.alife-foundation")).unwrap()
                )
            )
        );
        let prior = interrupted.episode.prior.as_ref().unwrap();
        assert_eq!(
            archive["provider_identity"],
            prior.metrics.provider_identity
        );
        assert_eq!(archive["lives"].as_object().unwrap().len(), 1);
        let id = host
            .world()
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .organism_id()
            .raw();
        let saved_life: Vec<u8> =
            serde_json::from_value(archive["lives"][id.to_string()].clone()).unwrap();
        assert_eq!(Some(saved_life.clone()), prior.snapshot(id).unwrap());
        let mut restored_prior = RuntimeSemanticPrior::deterministic_for_training(seed);
        restored_prior.restore_life(id, tick, &saved_life).unwrap();
        assert_eq!(restored_prior.snapshot(id).unwrap(), Some(saved_life));
        for name in [
            "collector-shard.json",
            "collector-commit.json",
            "cpu-cycle.json",
            "training-handoff.json",
            "trained.alife-foundation",
            "care-episode.json",
        ] {
            assert!(
                !root.join(name).exists(),
                "interrupted collection published {name}"
            );
        }
        assert!(read_cpu_collector_output(root).is_err());
    }

    #[test]
    fn care_collection_keeps_private_learning_and_seals_without_offline_updates() {
        // Give the fixture its own environment; never initialize a live prior
        // provider or race another test's provider/cache configuration.
        const CHILD: &str = "ALIFE_TEST_COLLECT_ONLY_CHILD";
        if std::env::var(CHILD).as_deref() != Ok("1") {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "foundation_cpu_care::collect_only_tests::care_collection_keeps_private_learning_and_seals_without_offline_updates", "--nocapture"])
                .env(CHILD, "1").env("ALIFE_SLM_PRIOR", "on")
                .env("ALIFE_SLM_PRIOR_BACKEND", "deterministic")
                .output().unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
            return;
        }
        let root = std::env::temp_dir().join(format!(
            "alife-care-collect-only-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let seed = 0x93_41;
        let (config, mut episode) = prepare_care_episode(CpuCareRequest {
            previous: None,
            output: root.clone(),
            seed,
            decisions: 3,
            temperature: 1.,
            lesson: FoundationTeacherLesson::EatHeldFood,
            food_need_condition: crate::FoundationFoodNeedCondition::default(),
            seed_history: None,
            known_exclusions: false,
            record_diagnostic_replay: true,
            fixture: true,
        })
        .unwrap();
        let collected = collect_cpu_episode(config, &mut episode).unwrap();
        assert!(episode.reservation.is_none());
        assert_eq!(collected.actor_checkpoint().optimizer_step, 0);
        assert_eq!(collected.value_checkpoint().optimizer_step, 0);
        let checkpoint = collected.host_checkpoint().unwrap();
        assert_eq!(checkpoint.sampling_counter, 4);
        assert_eq!(
            collected.decisions().values().map(Vec::len).sum::<usize>(),
            4
        );
        assert!(checkpoint.residents[0]
            .neural
            .eligibility
            .iter()
            .any(|v| *v != 0.));
        let prior: Value =
            serde_json::from_slice(&std::fs::read(root.join("care-prior-lives.json")).unwrap())
                .unwrap();
        assert!(prior["provider_identity"].as_str().is_some());
        assert_eq!(prior["lives"].as_object().unwrap().len(), 1);
        let lesson = collected.episode_metadata().unwrap()["lesson"]
            .as_str()
            .unwrap()
            .to_owned();
        collected
            .seal(alife_training::cpu::CpuCollectorAssignmentV1 {
                collector_run_id: "synthetic-care".into(),
                backend: alife_training::FoundationTrainingBackendV1::NativeCpu,
                producer_revision: "synthetic-collect-only-fixture".into(),
                world_seed: seed,
                sampling_seed: seed as u32,
                sampling_counter_start: 0,
                sampling_counter_end: checkpoint.sampling_counter,
                lesson,
                episode_config_sha256: [42; 32],
            })
            .unwrap();
        let shard = read_cpu_collector_output(&root).unwrap();
        assert_eq!(shard.trajectories.len(), 1);
        assert_eq!(shard.trajectories[0].decisions.len(), 4);
        let after: CpuCanonicalCheckpoint = read_cpu_private_checkpoint(&root).unwrap();
        assert_eq!(
            serde_json::to_vec(&checkpoint).unwrap(),
            serde_json::to_vec(&after).unwrap()
        );
        for name in [
            "cpu-cycle.json",
            "training-handoff.json",
            "trained.alife-foundation",
        ] {
            assert!(!root.join(name).exists());
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

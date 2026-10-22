//! CPU care episodes reuse the native lesson world, prior and outcome observer.
use crate::foundation_training::{
    care_meal_completed, close_foundation_navigation_gate_in_world, configure_foundation_scenario,
    expose_foundation_teacher_cue,
    food_outcome::{ownership, FoodOutcomeObservation, FoodOwnershipSnapshot},
    grounded_navigation_completed, heard_foundation_teacher_cue, outcome_consumed,
    relocate_unheld_foundation_food, teacher_outcome_blocked, teacher_outcome_moved,
    write_foundation_lesson_prior_readiness, FoundationScenarioSetup,
};
use crate::gpu_live_runtime::semantic_prior::RuntimeSemanticPrior;
use crate::{FoundationFoodOutcome, FoundationTeacherLesson};
use alife_core::{
    ExperienceSequenceId, PerceptionFrameDraft, ScaffoldContractError, SensorProfile, Validate,
};
use alife_training::cpu::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};
mod admission;
type Result<T> = CpuContinuationResult<T>;

pub struct CpuCareRequest {
    pub previous: Option<PathBuf>,
    pub output: PathBuf,
    pub seed: u64,
    pub decisions: usize,
    pub temperature: f32,
    pub lesson: FoundationTeacherLesson,
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

pub fn run_cpu_care(request: CpuCareRequest) -> Result<Value> {
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
    let config = CpuContinuationConfig {
        previous: request.previous.clone(),
        output: request.output.clone(),
        seed: request.seed,
        ticks: request.decisions + 1,
        temperature: request.temperature,
        fixture: request.fixture,
    };
    let mut episode = CareEpisode {
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
    };
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
}

impl CpuEpisodeHooks for CareEpisode {
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
        let reservation = admission::CareSeedReservation::reserve(config, &self.request)?;
        let provenance = reservation.provenance["seed_admission"].clone();
        self.reservation = Some(reservation);
        Ok(provenance)
    }
    fn validate_policy(&mut self, founder: &CpuFoundationTrainer) -> Result<()> {
        if founder.phenotype().sensor_profile() != SensorProfile::GroundedTerrainVisionV1 {
            return Err("CPU care requires the retained grounded natural-senses policy; sensor conversion is not continuation".into());
        }
        self.prior = RuntimeSemanticPrior::from_environment(self.request.seed, true)?;
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
        let scenario = configure_foundation_scenario(
            world,
            self.request.seed,
            Some(self.request.lesson),
            None,
            true,
        )?;
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
        expose_foundation_teacher_cue(host.world_mut(), scenario)?;
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
        let scenario = self.scenario.as_ref().ok_or("care scene missing")?;
        let summary = self
            .summary
            .as_mut()
            .ok_or("care outcome identity missing")?;
        for row in &step.decisions {
            row.patch.validate_contract()?;
            let frame = row.patch.pre_action().perception();
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
            trace["loss_row"] = json!(i < train_rows);
        }
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
        write(
            "care-trace.json",
            &json!({"schema":1,"backend":"native-cpu","lesson":self.request.lesson,"rows":self.traces}),
        )?;
        write(
            "care-prior-lives.json",
            &json!({"schema":1,"collection_policy_sha256":policy_sha,
            "provider_identity":self.prior.as_ref().map(|p| &p.metrics.provider_identity),"world_tick":host.world().tick(),"lives":lives,
            "restore_contract":"restore host checkpoint under collection policy and restore_life for the same organism before further dispatch"}),
        )?;
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
        let metadata = json!({"schema":1,"kind":"care-first-cpu-episode","lesson":self.request.lesson,
            "episode_body":"new-prepared-cohort","persistent_learning":"native-founder-and-full-actor-value-optimizer",
            "raw_replay_recorded":self.request.record_diagnostic_replay,
            "setup_ticks_are_training_rows":false,"setup_tick":scenario.start_tick,
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
        write("care-episode.json", &metadata)?;
        let mut artifacts = vec![
            "care-trace.json",
            "care-prior-lives.json",
            "care-episode.json",
            "collector-checkpoint.json",
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

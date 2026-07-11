//! v0 test fixture layer: deterministic headless behavior scenarios for P18.
//!
//! These fixtures sit above the P17 headless harness. They define named world
//! layouts, creature starting state, scripted proposal timelines, and broad
//! assertions over sealed patches, memory, topology, sleep, language, and
//! social context. They intentionally avoid P19-style exact golden traces.

use alife_core::{
    ActionId, ActionKind, ActionProposal, ActionTarget, BrainScaleTier, BrainTickInput,
    BrainTickStatus, ConceptCell, Confidence, CuriosityBias, DurationTicks, ExperiencePatch,
    HomeostaticSnapshot, Intensity, MemoryExpectancySnapshot, MemoryRecord, NormalizedScalar,
    OrganismId, PhysicalContactKind, ReferenceActionFailure, ScaffoldContractError,
    SleepConsolidationReport, SleepPhase, TeacherPerceptionChannel, Tick, Validate, Vec3f,
    WorldEntityId,
};

use crate::{
    HeadlessActionIds, HeadlessBrainHarness, HeadlessScenarioBuilder, HeadlessWorld,
    WorldObjectKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScenarioName {
    FoodSeeking,
    PoisonPainAvoidance,
    ObstacleFrustration,
    FatigueSleep,
    CuriosityContradiction,
    WordTokenGrounding,
    SimpleSocialTrustFear,
    TeacherPerceptionEvent,
}

impl ScenarioName {
    pub const ALL: [Self; 8] = [
        Self::FoodSeeking,
        Self::PoisonPainAvoidance,
        Self::ObstacleFrustration,
        Self::FatigueSleep,
        Self::CuriosityContradiction,
        Self::WordTokenGrounding,
        Self::SimpleSocialTrustFear,
        Self::TeacherPerceptionEvent,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FoodSeeking => "food-seeking",
            Self::PoisonPainAvoidance => "poison/pain avoidance",
            Self::ObstacleFrustration => "obstacle frustration",
            Self::FatigueSleep => "fatigue/sleep",
            Self::CuriosityContradiction => "curiosity from contradiction",
            Self::WordTokenGrounding => "word-token grounding",
            Self::SimpleSocialTrustFear => "simple social trust/fear",
            Self::TeacherPerceptionEvent => "teacher perception event",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedDirection {
    Any,
    Positive,
    Negative,
    NonNegative,
    NonPositive,
    Zero,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioCreatureConfig {
    pub organism_id: OrganismId,
    pub brain_tier: BrainScaleTier,
    pub genome_seed: u64,
    pub initial_homeostasis: HomeostaticSnapshot,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioObjectSpec {
    pub label: &'static str,
    pub kind: WorldObjectKind,
    pub organism_id: Option<OrganismId>,
    pub position: Vec3f,
    pub nutrition: f32,
    pub hazard_pain: f32,
    pub radius: f32,
    pub token_id: Option<u32>,
    pub social_affinity: f32,
    pub teacher_channel: Option<TeacherPerceptionChannel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioSensoryFrame {
    pub tick: Tick,
    pub expected_visible_labels: Vec<&'static str>,
    pub expected_heard_tokens: Vec<u32>,
    pub expected_teacher_channel: Option<TeacherPerceptionChannel>,
    pub expected_social_agents: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioStep {
    pub tick: Tick,
    pub proposals: Vec<ActionProposal>,
    pub expected_behavior: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioExpectedBehavior {
    pub broad_behavior: &'static str,
    pub run_sleep_consolidation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedPatchFields {
    pub patch_index: usize,
    pub expected_status: BrainTickStatus,
    pub expected_action_kind: ActionKind,
    pub expected_action_id: Option<ActionId>,
    pub expected_target_label: Option<&'static str>,
    pub expected_success: bool,
    pub expected_contact: Option<PhysicalContactKind>,
    pub reward: ExpectedDirection,
    pub hunger_delta: ExpectedDirection,
    pub fear_delta: ExpectedDirection,
    pub pain_delta: ExpectedDirection,
    pub cortisol_delta: ExpectedDirection,
    pub frustration: ExpectedDirection,
    pub energy: ExpectedDirection,
    pub prediction_error: ExpectedDirection,
    pub contradiction: bool,
    pub requires_food_salience_bias: bool,
    pub requires_word_token: Option<u32>,
    pub requires_teacher_channel: Option<TeacherPerceptionChannel>,
    pub requires_social_context: bool,
    pub requires_no_hidden_vectors: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedMemoryChange {
    pub min_records: usize,
    pub require_danger_bias: bool,
    pub require_safety_bias: bool,
    pub require_social_trust_bias: bool,
    pub require_social_fear_bias: bool,
    pub require_curiosity_bias: bool,
    pub require_bias_only_recall: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedTopologyChange {
    pub min_concepts: usize,
    pub min_edges: usize,
    pub min_simplexes: usize,
    pub min_gaps: usize,
    pub require_curiosity_bias: bool,
    pub require_word_binding: Option<u32>,
    pub require_social_binding: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioExpectations {
    pub behavior: ScenarioExpectedBehavior,
    pub patch: ExpectedPatchFields,
    pub memory: ExpectedMemoryChange,
    pub topology: ExpectedTopologyChange,
}

#[derive(Debug, Clone)]
pub struct ScenarioFixture {
    pub name: ScenarioName,
    pub seed: u64,
    pub creature: ScenarioCreatureConfig,
    pub object_layout: Vec<ScenarioObjectSpec>,
    pub sensory_timeline: Vec<ScenarioSensoryFrame>,
    pub steps: Vec<ScenarioStep>,
    pub expectations: ScenarioExpectations,
    pub world: HeadlessWorld,
}

impl ScenarioFixture {
    pub fn named(name: ScenarioName) -> Result<Self, ScaffoldContractError> {
        Self::with_seed(name, default_seed(name))
    }

    pub fn with_seed(name: ScenarioName, seed: u64) -> Result<Self, ScaffoldContractError> {
        match name {
            ScenarioName::FoodSeeking => food_seeking(seed),
            ScenarioName::PoisonPainAvoidance => poison_pain(seed),
            ScenarioName::ObstacleFrustration => obstacle_frustration(seed),
            ScenarioName::FatigueSleep => fatigue_sleep(seed),
            ScenarioName::CuriosityContradiction => curiosity_contradiction(seed),
            ScenarioName::WordTokenGrounding => word_token_grounding(seed),
            ScenarioName::SimpleSocialTrustFear => social_trust_fear(seed),
            ScenarioName::TeacherPerceptionEvent => teacher_perception_event(seed),
        }
    }

    pub fn run(&self) -> Result<ScenarioRun, ScaffoldContractError> {
        run_scenario(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioPatchSummary {
    pub tick: u64,
    pub outcome_tick: u64,
    pub selected_action_id: u32,
    pub selected_action_kind: ActionKind,
    pub target_entity: Option<u64>,
    pub success: bool,
    pub contact: PhysicalContactKind,
    pub reward_milli: i32,
    pub pain_milli: u16,
    pub frustration_milli: u16,
    pub prediction_error_milli: u16,
    pub contradiction: bool,
    pub memory_danger_milli: u16,
    pub memory_salience_milli: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioSummary {
    pub name: ScenarioName,
    pub seed: u64,
    pub patch_summaries: Vec<ScenarioPatchSummary>,
    pub memory_record_count: usize,
    pub topology_concept_count: usize,
    pub topology_gap_count: usize,
    pub sleep_phase: SleepPhase,
    pub sleep_cycle_count: u32,
    pub world_signature: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ScenarioRun {
    pub name: ScenarioName,
    pub seed: u64,
    pub ticks: Vec<crate::HeadlessBrainTick>,
    pub statuses: Vec<BrainTickStatus>,
    pub failures: Vec<Option<ReferenceActionFailure>>,
    pub patches: Vec<ExperiencePatch>,
    pub patch_summaries: Vec<ScenarioPatchSummary>,
    pub memory_records: Vec<MemoryRecord>,
    pub memory_record_count: usize,
    pub topology_concepts: Vec<ConceptCell>,
    pub topology_concept_count: usize,
    pub topology_edge_count: usize,
    pub topology_simplex_count: usize,
    pub topology_gap_ids: Vec<alife_core::UnresolvedGapId>,
    pub curiosity_biases: Vec<CuriosityBias>,
    pub sleep_report: Option<SleepConsolidationReport>,
    pub sleep_phase: SleepPhase,
    pub sleep_transition_observed: bool,
    pub sleep_cycle_count: u32,
    pub pending_structural_edit_count: usize,
    pub final_homeostasis: HomeostaticSnapshot,
    pub world_signature: Vec<String>,
    pub summary: ScenarioSummary,
    pub fixture_expectations: ScenarioExpectations,
}

impl ScenarioRun {
    pub fn first_patch(&self) -> &ExperiencePatch {
        self.patch_at(0)
    }

    pub fn patch_at(&self, index: usize) -> &ExperiencePatch {
        self.patches
            .get(index)
            .unwrap_or_else(|| panic!("missing scenario patch at index {index}"))
    }
}

pub struct ScenarioAssertions;

impl ScenarioAssertions {
    #[track_caller]
    pub fn assert_fixture_is_complete(fixture: &ScenarioFixture) {
        assert!(!fixture.name.as_str().is_empty());
        assert!(!fixture.object_layout.is_empty());
        assert!(!fixture.sensory_timeline.is_empty());
        assert!(!fixture.steps.is_empty());
        assert!(fixture
            .object_layout
            .iter()
            .any(|object| object.organism_id == Some(fixture.creature.organism_id)));
        assert!(!fixture.expectations.behavior.broad_behavior.is_empty());
        for step in &fixture.steps {
            assert!(!step.proposals.is_empty());
            assert!(!step.expected_behavior.is_empty());
        }
    }

    #[track_caller]
    pub fn assert_run_matches_expectations(fixture: &ScenarioFixture, run: &ScenarioRun) {
        assert_eq!(run.name, fixture.name);
        assert_eq!(run.seed, fixture.seed);
        assert!(!run.patches.is_empty());
        for patch in &run.patches {
            Self::assert_causal_patch_fields(patch);
            Self::assert_selected_action_came_from_arbitration(patch);
        }

        let expected = &fixture.expectations;
        let patch = run.patch_at(expected.patch.patch_index);
        assert_eq!(
            run.statuses[expected.patch.patch_index],
            expected.patch.expected_status
        );
        assert_eq!(
            patch.decision().selected_action.kind,
            expected.patch.expected_action_kind
        );
        if let Some(action_id) = expected.patch.expected_action_id {
            assert_eq!(patch.decision().selected_action.action_id, action_id);
        }
        if let Some(label) = expected.patch.expected_target_label {
            let target = fixture
                .world
                .entity_id(label)
                .unwrap_or_else(|| panic!("missing expected target label {label}"));
            assert!(
                patch.decision().selected_action.target_entity == Some(target)
                    || patch.outcome().physical.target_entity == Some(target),
                "expected target label {label} to appear in decision or outcome"
            );
        }
        assert_eq!(patch.outcome().success, expected.patch.expected_success);
        if let Some(contact) = expected.patch.expected_contact {
            assert_eq!(patch.outcome().physical.contact, contact);
        }
        Self::assert_direction(
            "reward",
            patch.outcome().reward_valence.raw(),
            expected.patch.reward,
        );
        Self::assert_direction(
            "hunger delta",
            patch.outcome().homeostatic_delta.drives.hunger,
            expected.patch.hunger_delta,
        );
        Self::assert_direction(
            "fear delta",
            patch.outcome().homeostatic_delta.drives.fear,
            expected.patch.fear_delta,
        );
        Self::assert_direction(
            "pain delta",
            patch.outcome().pain_delta.raw(),
            expected.patch.pain_delta,
        );
        Self::assert_direction(
            "cortisol delta",
            patch.outcome().homeostatic_delta.hormones.cortisol,
            expected.patch.cortisol_delta,
        );
        Self::assert_direction(
            "frustration",
            patch.outcome().frustration_delta.raw(),
            expected.patch.frustration,
        );
        Self::assert_direction(
            "energy",
            patch.outcome().energy_delta.raw(),
            expected.patch.energy,
        );
        Self::assert_direction(
            "prediction error",
            patch.outcome().prediction_error.raw(),
            expected.patch.prediction_error,
        );
        assert_eq!(
            patch.outcome().contradiction_observed,
            expected.patch.contradiction
        );
        if expected.patch.requires_food_salience_bias {
            let selected = patch.decision().selected_action.action_id;
            let selected_proposal = patch
                .decision()
                .heuristic_evidence()
                .expect("scenario patches use heuristic baseline decisions")
                .proposals
                .iter()
                .find(|proposal| proposal.action_id == selected)
                .expect("selected proposal exists");
            assert!(patch.pre_action().homeostasis().drives.hunger > 0.5);
            assert!(patch.pre_action().sensory().channels.visual_affordance[0] > 0.0);
            assert!(selected_proposal.salience.raw() > 0.0);
            assert!(selected_proposal
                .score_bias
                .is_some_and(|bias| bias.score_delta > 0.0));
        }
        if let Some(token_id) = expected.patch.requires_word_token {
            assert!(patch
                .pre_action()
                .sensory()
                .language_context
                .heard_tokens
                .iter()
                .flatten()
                .any(|token| token.token_id == token_id));
        }
        if let Some(channel) = expected.patch.requires_teacher_channel {
            assert_eq!(
                patch
                    .pre_action()
                    .sensory()
                    .language_context
                    .teacher_channel_marker,
                Some(channel)
            );
            assert!(patch
                .pre_action()
                .sensory()
                .language_context
                .heard_tokens
                .iter()
                .flatten()
                .any(|token| token.teacher_channel == Some(channel)));
            assert!(patch.decision().selected_action.teacher_lesson.is_none());
        }
        if expected.patch.requires_social_context {
            assert!(patch
                .pre_action()
                .sensory()
                .social_context
                .nearest_agents
                .iter()
                .flatten()
                .next()
                .is_some());
        }
        if expected.patch.requires_no_hidden_vectors {
            assert!(patch.pre_action().sensory().semantic_context.is_none());
            assert!(patch.pre_action().sensory().gaussian_context.is_none());
        }

        Self::assert_memory_expectations(&expected.memory, run);
        Self::assert_topology_expectations(&expected.topology, run);
        if expected.behavior.run_sleep_consolidation {
            assert!(run.sleep_transition_observed);
            assert!(run.sleep_report.is_some());
            assert!(run.sleep_cycle_count >= 1);
            assert!(run.pending_structural_edit_count >= 1);
        }
    }

    #[track_caller]
    pub fn assert_causal_patch_fields(patch: &ExperiencePatch) {
        patch.validate_contract().unwrap();
        assert_eq!(
            patch.phase_sequence(),
            [
                alife_core::ExperiencePatchPhase::PreActionSnapshot,
                alife_core::ExperiencePatchPhase::DecisionSnapshot,
                alife_core::ExperiencePatchPhase::PostActionOutcome,
                alife_core::ExperiencePatchPhase::Sealed,
            ]
        );
        assert_eq!(patch.pre_action().sequence_id, patch.decision().sequence_id);
        assert_eq!(patch.pre_action().sequence_id, patch.outcome().sequence_id);
        assert!(patch.decision().decision_tick.raw() >= patch.pre_action().tick.raw());
        assert!(patch.outcome().outcome_tick.raw() >= patch.decision().decision_tick.raw());
    }

    #[track_caller]
    pub fn assert_selected_action_came_from_arbitration(patch: &ExperiencePatch) {
        let selected_id = patch.decision().selected_action.action_id;
        let evidence = patch
            .decision()
            .heuristic_evidence()
            .expect("scenario patches use heuristic baseline decisions");
        assert!(
            evidence
                .proposals
                .iter()
                .any(|proposal| proposal.action_id == selected_id)
                || evidence.status == alife_core::ActionDecisionStatus::FallbackSelected,
            "selected action must come from supplied proposals or the configured fallback"
        );
        assert_eq!(
            evidence.arbitration_trace.wta_result.selected_action_id,
            Some(selected_id)
        );
    }

    #[track_caller]
    pub fn assert_memory_expectancy_snapshot_is_bias_only(snapshot: &MemoryExpectancySnapshot) {
        let MemoryExpectancySnapshot {
            expected_valence,
            predicted_drive_delta,
            affordance_bias,
            danger_bias,
            safety_bias,
            salience_hint,
        } = *snapshot;
        alife_core::SignedValence::new(expected_valence.raw()).unwrap();
        predicted_drive_delta.validate_contract().unwrap();
        NormalizedScalar::new(affordance_bias.raw()).unwrap();
        NormalizedScalar::new(danger_bias.raw()).unwrap();
        NormalizedScalar::new(safety_bias.raw()).unwrap();
        NormalizedScalar::new(salience_hint.raw()).unwrap();
    }

    #[track_caller]
    pub fn assert_direction(label: &str, value: f32, direction: ExpectedDirection) {
        alife_core::validate_finite(value).unwrap();
        match direction {
            ExpectedDirection::Any => {}
            ExpectedDirection::Positive => assert!(value > 0.0, "{label} expected positive"),
            ExpectedDirection::Negative => assert!(value < 0.0, "{label} expected negative"),
            ExpectedDirection::NonNegative => assert!(value >= 0.0, "{label} expected >= 0"),
            ExpectedDirection::NonPositive => assert!(value <= 0.0, "{label} expected <= 0"),
            ExpectedDirection::Zero => assert_eq!(value, 0.0, "{label} expected zero"),
        }
    }

    #[track_caller]
    fn assert_memory_expectations(expected: &ExpectedMemoryChange, run: &ScenarioRun) {
        assert!(run.memory_record_count >= expected.min_records);
        if expected.require_danger_bias {
            assert!(run
                .memory_records
                .iter()
                .any(|record| record.danger_bias.raw() > 0.0));
        }
        if expected.require_safety_bias {
            assert!(run
                .memory_records
                .iter()
                .any(|record| record.safety_bias.raw() > 0.0));
        }
        if expected.require_social_trust_bias {
            assert!(run
                .memory_records
                .iter()
                .any(|record| record.social_trust_bias.raw() > 0.0));
        }
        if expected.require_social_fear_bias {
            assert!(run
                .memory_records
                .iter()
                .any(|record| record.social_fear_bias.raw() > 0.0));
        }
        if expected.require_curiosity_bias {
            assert!(run
                .memory_records
                .iter()
                .any(|record| record.curiosity_bias.raw() > 0.0));
        }
        if expected.require_bias_only_recall {
            assert!(run.patches.iter().any(|patch| {
                let memory = &patch
                    .pre_action()
                    .heuristic_evidence()
                    .expect("scenario patches use heuristic baseline evidence")
                    .memory_expectancy;
                memory.danger_bias.raw() > 0.0 || memory.salience_hint.raw() > 0.0
            }));
            for patch in &run.patches {
                let memory = &patch
                    .pre_action()
                    .heuristic_evidence()
                    .expect("scenario patches use heuristic baseline evidence")
                    .memory_expectancy;
                Self::assert_memory_expectancy_snapshot_is_bias_only(memory);
            }
        }
    }

    #[track_caller]
    fn assert_topology_expectations(expected: &ExpectedTopologyChange, run: &ScenarioRun) {
        assert!(run.topology_concept_count >= expected.min_concepts);
        assert!(run.topology_edge_count >= expected.min_edges);
        assert!(run.topology_simplex_count >= expected.min_simplexes);
        assert!(run.topology_gap_ids.len() >= expected.min_gaps);
        if expected.require_curiosity_bias {
            assert!(run
                .curiosity_biases
                .iter()
                .any(|bias| bias.curiosity_voltage.raw() > 0.0));
        }
        if let Some(token_id) = expected.require_word_binding {
            assert!(run
                .topology_concepts
                .iter()
                .any(|concept| concept.bindings.words.contains(&token_id)));
        }
        if expected.require_social_binding {
            assert!(run
                .topology_concepts
                .iter()
                .any(|concept| !concept.bindings.agents.is_empty()));
        }
    }
}

fn run_scenario(fixture: &ScenarioFixture) -> Result<ScenarioRun, ScaffoldContractError> {
    let mut harness = HeadlessBrainHarness::new(fixture.world.clone());
    let mut mind = alife_core::CreatureMind::scaffold(
        fixture.creature.organism_id,
        fixture.creature.brain_tier,
        fixture.creature.genome_seed,
        Tick::ZERO,
    )?;
    *mind.homeostasis_mut() = fixture.creature.initial_homeostasis;
    mind.homeostasis().validate_contract()?;

    let mut ticks = Vec::with_capacity(fixture.steps.len());
    let mut sleep_transition_observed = false;
    for step in &fixture.steps {
        let input = BrainTickInput::new(step.tick, step.proposals.clone())
            .with_pack_experience(true)
            .with_action_duration(DurationTicks::new(1));
        let tick = harness.tick_mind(&mut mind, input);
        sleep_transition_observed |= tick.sleep_transition.is_some();
        ticks.push(tick);
    }

    let sleep_report = if fixture.expectations.behavior.run_sleep_consolidation
        && mind.sleep_state().phase != SleepPhase::Awake
    {
        Some(mind.run_sleep_consolidation(Tick::new(mind.current_tick().raw().saturating_add(1)))?)
    } else {
        None
    };

    let patches = harness.telemetry().sealed_patches.clone();
    let patch_summaries = patches
        .iter()
        .map(ScenarioPatchSummary::from_patch)
        .collect::<Vec<_>>();
    let memory_records = mind
        .memory_bank()
        .records_chronological()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    let topology_concepts = mind.topological_map().concepts().to_vec();
    let topology_gap_ids = mind
        .topological_map()
        .unresolved_gaps()
        .iter()
        .map(|gap| gap.id)
        .collect::<Vec<_>>();
    let curiosity_biases = mind.topological_map().curiosity_biases();
    let world_signature = harness.world().stable_signature();
    let statuses = ticks
        .iter()
        .map(|tick| tick.brain.status)
        .collect::<Vec<_>>();
    let failures = ticks
        .iter()
        .map(|tick| {
            tick.action_result
                .as_ref()
                .and_then(|result| result.execution.failure)
        })
        .collect::<Vec<_>>();
    let sleep_phase = mind.sleep_state().phase;
    let sleep_cycle_count = mind.development_state().sleep_cycle_count;
    let pending_structural_edit_count = mind.pending_structural_edits().len();
    let final_homeostasis = *mind.homeostasis();
    let summary = ScenarioSummary {
        name: fixture.name,
        seed: fixture.seed,
        patch_summaries: patch_summaries.clone(),
        memory_record_count: memory_records.len(),
        topology_concept_count: topology_concepts.len(),
        topology_gap_count: topology_gap_ids.len(),
        sleep_phase,
        sleep_cycle_count,
        world_signature: world_signature.clone(),
    };

    Ok(ScenarioRun {
        name: fixture.name,
        seed: fixture.seed,
        ticks,
        statuses,
        failures,
        patches,
        patch_summaries,
        memory_record_count: memory_records.len(),
        memory_records,
        topology_concept_count: topology_concepts.len(),
        topology_edge_count: mind.topological_map().edges().len(),
        topology_simplex_count: mind.topological_map().simplexes().len(),
        topology_concepts,
        topology_gap_ids,
        curiosity_biases,
        sleep_report,
        sleep_phase,
        sleep_transition_observed,
        sleep_cycle_count,
        pending_structural_edit_count,
        final_homeostasis,
        world_signature,
        summary,
        fixture_expectations: fixture.expectations.clone(),
    })
}

impl ScenarioPatchSummary {
    fn from_patch(patch: &ExperiencePatch) -> Self {
        let memory = &patch
            .pre_action()
            .heuristic_evidence()
            .expect("scenario patches use heuristic baseline evidence")
            .memory_expectancy;
        Self {
            tick: patch.pre_action().tick.raw(),
            outcome_tick: patch.outcome().outcome_tick.raw(),
            selected_action_id: patch.decision().selected_action.action_id.raw(),
            selected_action_kind: patch.decision().selected_action.kind,
            target_entity: patch
                .decision()
                .selected_action
                .target_entity
                .map(WorldEntityId::raw),
            success: patch.outcome().success,
            contact: patch.outcome().physical.contact,
            reward_milli: milli_signed(patch.outcome().reward_valence.raw()),
            pain_milli: milli_unit(patch.outcome().pain_delta.raw()),
            frustration_milli: milli_unit(patch.outcome().frustration_delta.raw()),
            prediction_error_milli: milli_unit(patch.outcome().prediction_error.raw()),
            contradiction: patch.outcome().contradiction_observed,
            memory_danger_milli: milli_unit(memory.danger_bias.raw()),
            memory_salience_milli: milli_unit(memory.salience_hint.raw()),
        }
    }
}

fn food_seeking(seed: u64) -> Result<ScenarioFixture, ScaffoldContractError> {
    let organism = OrganismId(1801);
    let layout = vec![
        agent("agent", organism, pos(0.0, 0.0)),
        food("berry", pos(0.5, 0.0), 0.65),
    ];
    let world = world_from_layout(seed, &layout)?;
    let steps = vec![ScenarioStep {
        tick: Tick::ZERO,
        proposals: vec![
            proposal(
                &world,
                HeadlessActionIds::EAT,
                ActionKind::Interact,
                Some("berry"),
                None,
                0.56,
                0.95,
            )?,
            proposal(
                &world,
                ActionKind::Inspect.canonical_id(),
                ActionKind::Inspect,
                Some("berry"),
                None,
                0.34,
                0.55,
            )?,
        ],
        expected_behavior: "hungry creature selects edible target and eats it",
    }];
    fixture(
        ScenarioName::FoodSeeking,
        seed,
        organism,
        homeostasis(seed, |state| {
            state.drives.hunger = 0.86;
            state.drives.brain_atp = 0.45;
        })?,
        layout,
        world,
        sensory(vec!["berry"], vec![], None, 0),
        steps,
        expectations(
            "hunger raises food salience; eating improves reward and energy",
            false,
            ExpectedPatchFields {
                patch_index: 0,
                expected_status: BrainTickStatus::Normal,
                expected_action_kind: ActionKind::Interact,
                expected_action_id: Some(HeadlessActionIds::EAT),
                expected_target_label: Some("berry"),
                expected_success: true,
                expected_contact: Some(PhysicalContactKind::Consumed),
                reward: ExpectedDirection::Positive,
                hunger_delta: ExpectedDirection::Negative,
                fear_delta: ExpectedDirection::Zero,
                pain_delta: ExpectedDirection::Zero,
                cortisol_delta: ExpectedDirection::Zero,
                frustration: ExpectedDirection::Zero,
                energy: ExpectedDirection::Positive,
                prediction_error: ExpectedDirection::Positive,
                contradiction: false,
                requires_food_salience_bias: true,
                requires_word_token: None,
                requires_teacher_channel: None,
                requires_social_context: false,
                requires_no_hidden_vectors: true,
            },
            ExpectedMemoryChange {
                min_records: 1,
                require_danger_bias: false,
                require_safety_bias: true,
                require_social_trust_bias: false,
                require_social_fear_bias: false,
                require_curiosity_bias: true,
                require_bias_only_recall: false,
            },
            ExpectedTopologyChange {
                min_concepts: 2,
                min_edges: 1,
                min_simplexes: 1,
                min_gaps: 0,
                require_curiosity_bias: false,
                require_word_binding: None,
                require_social_binding: false,
            },
        ),
    )
}

fn poison_pain(seed: u64) -> Result<ScenarioFixture, ScaffoldContractError> {
    let organism = OrganismId(1802);
    let layout = vec![
        agent("agent", organism, pos(0.0, 0.0)),
        hazard("poison", pos(1.0, 0.0), 0.8),
    ];
    let world = world_from_layout(seed, &layout)?;
    let steps = vec![
        ScenarioStep {
            tick: Tick::ZERO,
            proposals: vec![proposal(
                &world,
                HeadlessActionIds::APPROACH,
                ActionKind::Move,
                Some("poison"),
                None,
                0.8,
                0.85,
            )?],
            expected_behavior: "first encounter contacts the harmful object",
        },
        ScenarioStep {
            tick: Tick::new(1),
            proposals: vec![
                proposal(
                    &world,
                    HeadlessActionIds::FLEE,
                    ActionKind::Move,
                    Some("poison"),
                    None,
                    0.62,
                    0.9,
                )?,
                proposal(
                    &world,
                    HeadlessActionIds::APPROACH,
                    ActionKind::Move,
                    Some("poison"),
                    None,
                    0.50,
                    0.8,
                )?,
            ],
            expected_behavior: "repeat encounter carries danger expectancy and selects avoidance",
        },
    ];
    fixture(
        ScenarioName::PoisonPainAvoidance,
        seed,
        organism,
        homeostasis(seed, |state| {
            state.drives.fear = 0.18;
            state.drives.curiosity = 0.5;
        })?,
        layout,
        world,
        sensory(vec!["poison"], vec![], None, 0),
        steps,
        expectations(
            "harmful object creates pain/fear and later danger expectancy",
            false,
            ExpectedPatchFields {
                patch_index: 0,
                expected_status: BrainTickStatus::Normal,
                expected_action_kind: ActionKind::Move,
                expected_action_id: Some(HeadlessActionIds::APPROACH),
                expected_target_label: Some("poison"),
                expected_success: true,
                expected_contact: Some(PhysicalContactKind::Collision),
                reward: ExpectedDirection::Negative,
                hunger_delta: ExpectedDirection::Zero,
                fear_delta: ExpectedDirection::Positive,
                pain_delta: ExpectedDirection::Positive,
                cortisol_delta: ExpectedDirection::Positive,
                frustration: ExpectedDirection::Positive,
                energy: ExpectedDirection::Negative,
                prediction_error: ExpectedDirection::Positive,
                contradiction: true,
                requires_food_salience_bias: false,
                requires_word_token: None,
                requires_teacher_channel: None,
                requires_social_context: false,
                requires_no_hidden_vectors: true,
            },
            ExpectedMemoryChange {
                min_records: 2,
                require_danger_bias: true,
                require_safety_bias: false,
                require_social_trust_bias: false,
                require_social_fear_bias: false,
                require_curiosity_bias: true,
                require_bias_only_recall: true,
            },
            ExpectedTopologyChange {
                min_concepts: 1,
                min_edges: 1,
                min_simplexes: 2,
                min_gaps: 1,
                require_curiosity_bias: true,
                require_word_binding: None,
                require_social_binding: false,
            },
        ),
    )
}

fn obstacle_frustration(seed: u64) -> Result<ScenarioFixture, ScaffoldContractError> {
    let organism = OrganismId(1803);
    let layout = vec![
        agent("agent", organism, pos(0.0, 0.0)),
        obstacle("wall", pos(1.0, 0.0), 0.8),
    ];
    let world = world_from_layout(seed, &layout)?;
    let steps = vec![ScenarioStep {
        tick: Tick::ZERO,
        proposals: vec![proposal(
            &world,
            ActionKind::Move.canonical_id(),
            ActionKind::Move,
            None,
            Some(pos(1.0, 0.0)),
            0.82,
            0.75,
        )?],
        expected_behavior: "blocked target produces failure and frustration",
    }];
    fixture(
        ScenarioName::ObstacleFrustration,
        seed,
        organism,
        homeostasis(seed, |state| {
            state.drives.curiosity = 0.6;
        })?,
        layout,
        world,
        sensory(vec!["wall"], vec![], None, 0),
        steps,
        expectations(
            "blocked movement records frustration and an unresolved topology gap",
            false,
            ExpectedPatchFields {
                patch_index: 0,
                expected_status: BrainTickStatus::RecoverableActionFailure,
                expected_action_kind: ActionKind::Move,
                expected_action_id: Some(ActionKind::Move.canonical_id()),
                expected_target_label: Some("wall"),
                expected_success: false,
                expected_contact: Some(PhysicalContactKind::Blocked),
                reward: ExpectedDirection::Negative,
                hunger_delta: ExpectedDirection::Zero,
                fear_delta: ExpectedDirection::Zero,
                pain_delta: ExpectedDirection::Positive,
                cortisol_delta: ExpectedDirection::Positive,
                frustration: ExpectedDirection::Positive,
                energy: ExpectedDirection::Negative,
                prediction_error: ExpectedDirection::Positive,
                contradiction: true,
                requires_food_salience_bias: false,
                requires_word_token: None,
                requires_teacher_channel: None,
                requires_social_context: false,
                requires_no_hidden_vectors: true,
            },
            ExpectedMemoryChange {
                min_records: 1,
                require_danger_bias: true,
                require_safety_bias: false,
                require_social_trust_bias: false,
                require_social_fear_bias: false,
                require_curiosity_bias: true,
                require_bias_only_recall: false,
            },
            ExpectedTopologyChange {
                min_concepts: 2,
                min_edges: 1,
                min_simplexes: 1,
                min_gaps: 1,
                require_curiosity_bias: true,
                require_word_binding: None,
                require_social_binding: false,
            },
        ),
    )
}

fn fatigue_sleep(seed: u64) -> Result<ScenarioFixture, ScaffoldContractError> {
    let organism = OrganismId(1804);
    let layout = vec![agent("agent", organism, pos(0.0, 0.0))];
    let world = world_from_layout(seed, &layout)?;
    let steps = vec![ScenarioStep {
        tick: Tick::ZERO,
        proposals: vec![proposal(
            &world,
            ActionKind::Rest.canonical_id(),
            ActionKind::Rest,
            None,
            None,
            0.88,
            0.8,
        )?],
        expected_behavior: "fatigued creature selects rest and enters forced sleep hook",
    }];
    fixture(
        ScenarioName::FatigueSleep,
        seed,
        organism,
        homeostasis(seed, |state| {
            state.drives.fatigue = 0.95;
            state.drives.brain_atp = 0.32;
            state.hormones.sleep_pressure = 0.9;
        })?,
        layout,
        world,
        sensory(vec![], vec![], None, 0),
        steps,
        expectations(
            "fatigue selects rest; P16 sleep consolidation stages structural edits",
            true,
            ExpectedPatchFields {
                patch_index: 0,
                expected_status: BrainTickStatus::Normal,
                expected_action_kind: ActionKind::Rest,
                expected_action_id: Some(ActionKind::Rest.canonical_id()),
                expected_target_label: None,
                expected_success: true,
                expected_contact: Some(PhysicalContactKind::None),
                reward: ExpectedDirection::Positive,
                hunger_delta: ExpectedDirection::Zero,
                fear_delta: ExpectedDirection::Zero,
                pain_delta: ExpectedDirection::Zero,
                cortisol_delta: ExpectedDirection::Zero,
                frustration: ExpectedDirection::Zero,
                energy: ExpectedDirection::Positive,
                prediction_error: ExpectedDirection::Positive,
                contradiction: false,
                requires_food_salience_bias: false,
                requires_word_token: None,
                requires_teacher_channel: None,
                requires_social_context: false,
                requires_no_hidden_vectors: true,
            },
            ExpectedMemoryChange {
                min_records: 1,
                require_danger_bias: false,
                require_safety_bias: true,
                require_social_trust_bias: false,
                require_social_fear_bias: false,
                require_curiosity_bias: true,
                require_bias_only_recall: false,
            },
            ExpectedTopologyChange {
                min_concepts: 1,
                min_edges: 1,
                min_simplexes: 1,
                min_gaps: 0,
                require_curiosity_bias: false,
                require_word_binding: None,
                require_social_binding: false,
            },
        ),
    )
}

fn curiosity_contradiction(seed: u64) -> Result<ScenarioFixture, ScaffoldContractError> {
    let organism = OrganismId(1805);
    let layout = vec![
        agent("agent", organism, pos(0.0, 0.0)),
        obstacle("sealed_box", pos(0.4, 0.0), 0.3),
    ];
    let world = world_from_layout(seed, &layout)?;
    let steps = vec![ScenarioStep {
        tick: Tick::ZERO,
        proposals: vec![
            proposal(
                &world,
                HeadlessActionIds::EAT,
                ActionKind::Interact,
                Some("sealed_box"),
                None,
                0.76,
                0.95,
            )?,
            proposal(
                &world,
                ActionKind::Inspect.canonical_id(),
                ActionKind::Inspect,
                Some("sealed_box"),
                None,
                0.58,
                0.9,
            )?,
        ],
        expected_behavior: "expected reward mismatch opens a curiosity gap through arbitration",
    }];
    fixture(
        ScenarioName::CuriosityContradiction,
        seed,
        organism,
        homeostasis(seed, |state| {
            state.drives.curiosity = 0.88;
        })?,
        layout,
        world,
        sensory(vec!["sealed_box"], vec![], None, 0),
        steps,
        expectations(
            "contradiction raises curiosity voltage without issuing an action shortcut",
            false,
            ExpectedPatchFields {
                patch_index: 0,
                expected_status: BrainTickStatus::RecoverableActionFailure,
                expected_action_kind: ActionKind::Interact,
                expected_action_id: Some(HeadlessActionIds::EAT),
                expected_target_label: Some("sealed_box"),
                expected_success: false,
                expected_contact: Some(PhysicalContactKind::Blocked),
                reward: ExpectedDirection::Negative,
                hunger_delta: ExpectedDirection::Zero,
                fear_delta: ExpectedDirection::Zero,
                pain_delta: ExpectedDirection::Zero,
                cortisol_delta: ExpectedDirection::Positive,
                frustration: ExpectedDirection::Positive,
                energy: ExpectedDirection::Negative,
                prediction_error: ExpectedDirection::Positive,
                contradiction: true,
                requires_food_salience_bias: false,
                requires_word_token: None,
                requires_teacher_channel: None,
                requires_social_context: false,
                requires_no_hidden_vectors: true,
            },
            ExpectedMemoryChange {
                min_records: 1,
                require_danger_bias: true,
                require_safety_bias: false,
                require_social_trust_bias: false,
                require_social_fear_bias: false,
                require_curiosity_bias: true,
                require_bias_only_recall: false,
            },
            ExpectedTopologyChange {
                min_concepts: 2,
                min_edges: 1,
                min_simplexes: 1,
                min_gaps: 1,
                require_curiosity_bias: true,
                require_word_binding: None,
                require_social_binding: false,
            },
        ),
    )
}

fn word_token_grounding(seed: u64) -> Result<ScenarioFixture, ScaffoldContractError> {
    let organism = OrganismId(1806);
    let layout = vec![
        agent("agent", organism, pos(0.0, 0.0)),
        food("berry", pos(1.0, 0.0), 0.5),
        token("word_food", pos(0.8, 0.0), 41),
    ];
    let world = world_from_layout(seed, &layout)?;
    let steps = vec![ScenarioStep {
        tick: Tick::ZERO,
        proposals: vec![proposal(
            &world,
            ActionKind::Inspect.canonical_id(),
            ActionKind::Inspect,
            Some("word_food"),
            None,
            0.74,
            0.85,
        )?],
        expected_behavior: "word token is heard/seen and bound to a topology concept",
    }];
    fixture(
        ScenarioName::WordTokenGrounding,
        seed,
        organism,
        homeostasis(seed, |state| {
            state.drives.curiosity = 0.58;
        })?,
        layout,
        world,
        sensory(vec!["berry", "word_food"], vec![41], None, 0),
        steps,
        expectations(
            "word inputs enter through language sensory context only",
            false,
            ExpectedPatchFields {
                patch_index: 0,
                expected_status: BrainTickStatus::Normal,
                expected_action_kind: ActionKind::Inspect,
                expected_action_id: Some(ActionKind::Inspect.canonical_id()),
                expected_target_label: Some("word_food"),
                expected_success: true,
                expected_contact: Some(PhysicalContactKind::Touch),
                reward: ExpectedDirection::Positive,
                hunger_delta: ExpectedDirection::Zero,
                fear_delta: ExpectedDirection::Zero,
                pain_delta: ExpectedDirection::Zero,
                cortisol_delta: ExpectedDirection::Zero,
                frustration: ExpectedDirection::Zero,
                energy: ExpectedDirection::Negative,
                prediction_error: ExpectedDirection::Positive,
                contradiction: false,
                requires_food_salience_bias: false,
                requires_word_token: Some(41),
                requires_teacher_channel: None,
                requires_social_context: false,
                requires_no_hidden_vectors: true,
            },
            ExpectedMemoryChange {
                min_records: 1,
                require_danger_bias: false,
                require_safety_bias: true,
                require_social_trust_bias: false,
                require_social_fear_bias: false,
                require_curiosity_bias: true,
                require_bias_only_recall: false,
            },
            ExpectedTopologyChange {
                min_concepts: 2,
                min_edges: 1,
                min_simplexes: 1,
                min_gaps: 0,
                require_curiosity_bias: false,
                require_word_binding: Some(41),
                require_social_binding: false,
            },
        ),
    )
}

fn social_trust_fear(seed: u64) -> Result<ScenarioFixture, ScaffoldContractError> {
    let organism = OrganismId(1807);
    let layout = vec![
        agent("agent", organism, pos(0.0, 0.0)),
        social_agent("friend", OrganismId(2801), pos(0.8, 0.0), 0.7),
        social_agent("threat", OrganismId(2802), pos(0.0, 0.9), -0.8),
    ];
    let world = world_from_layout(seed, &layout)?;
    let steps = vec![ScenarioStep {
        tick: Tick::ZERO,
        proposals: vec![proposal(
            &world,
            HeadlessActionIds::APPROACH,
            ActionKind::Move,
            Some("friend"),
            None,
            0.72,
            0.82,
        )?],
        expected_behavior: "social proximity contributes trust/fear bias but not motor commands",
    }];
    fixture(
        ScenarioName::SimpleSocialTrustFear,
        seed,
        organism,
        homeostasis(seed, |state| {
            state.drives.loneliness = 0.72;
        })?,
        layout,
        world,
        sensory(vec!["friend", "threat"], vec![], None, 2),
        steps,
        expectations(
            "nearby agents provide social trust/fear signals through context",
            false,
            ExpectedPatchFields {
                patch_index: 0,
                expected_status: BrainTickStatus::Normal,
                expected_action_kind: ActionKind::Move,
                expected_action_id: Some(HeadlessActionIds::APPROACH),
                expected_target_label: Some("friend"),
                expected_success: true,
                expected_contact: Some(PhysicalContactKind::Moved),
                reward: ExpectedDirection::Zero,
                hunger_delta: ExpectedDirection::Zero,
                fear_delta: ExpectedDirection::Zero,
                pain_delta: ExpectedDirection::Zero,
                cortisol_delta: ExpectedDirection::Zero,
                frustration: ExpectedDirection::Zero,
                energy: ExpectedDirection::Negative,
                prediction_error: ExpectedDirection::Positive,
                contradiction: false,
                requires_food_salience_bias: false,
                requires_word_token: None,
                requires_teacher_channel: None,
                requires_social_context: true,
                requires_no_hidden_vectors: true,
            },
            ExpectedMemoryChange {
                min_records: 1,
                require_danger_bias: false,
                require_safety_bias: true,
                require_social_trust_bias: true,
                require_social_fear_bias: true,
                require_curiosity_bias: true,
                require_bias_only_recall: false,
            },
            ExpectedTopologyChange {
                min_concepts: 2,
                min_edges: 1,
                min_simplexes: 1,
                min_gaps: 0,
                require_curiosity_bias: false,
                require_word_binding: None,
                require_social_binding: true,
            },
        ),
    )
}

fn teacher_perception_event(seed: u64) -> Result<ScenarioFixture, ScaffoldContractError> {
    let organism = OrganismId(1808);
    let layout = vec![
        agent("agent", organism, pos(0.0, 0.0)),
        teacher_token(
            "teacher_word",
            pos(0.7, 0.0),
            77,
            TeacherPerceptionChannel::Hearing,
        ),
    ];
    let world = world_from_layout(seed, &layout)?;
    let steps = vec![ScenarioStep {
        tick: Tick::ZERO,
        proposals: vec![proposal(
            &world,
            ActionKind::Inspect.canonical_id(),
            ActionKind::Inspect,
            Some("teacher_word"),
            None,
            0.73,
            0.88,
        )?],
        expected_behavior: "teacher label appears only as a heard perceptual token",
    }];
    fixture(
        ScenarioName::TeacherPerceptionEvent,
        seed,
        organism,
        homeostasis(seed, |state| {
            state.drives.curiosity = 0.62;
        })?,
        layout,
        world,
        sensory(
            vec!["teacher_word"],
            vec![77],
            Some(TeacherPerceptionChannel::Hearing),
            0,
        ),
        steps,
        expectations(
            "teacher event is sensory/modulatory and does not attach lesson action metadata",
            false,
            ExpectedPatchFields {
                patch_index: 0,
                expected_status: BrainTickStatus::Normal,
                expected_action_kind: ActionKind::Inspect,
                expected_action_id: Some(ActionKind::Inspect.canonical_id()),
                expected_target_label: Some("teacher_word"),
                expected_success: true,
                expected_contact: Some(PhysicalContactKind::Touch),
                reward: ExpectedDirection::Positive,
                hunger_delta: ExpectedDirection::Zero,
                fear_delta: ExpectedDirection::Zero,
                pain_delta: ExpectedDirection::Zero,
                cortisol_delta: ExpectedDirection::Zero,
                frustration: ExpectedDirection::Zero,
                energy: ExpectedDirection::Negative,
                prediction_error: ExpectedDirection::Positive,
                contradiction: false,
                requires_food_salience_bias: false,
                requires_word_token: Some(77),
                requires_teacher_channel: Some(TeacherPerceptionChannel::Hearing),
                requires_social_context: false,
                requires_no_hidden_vectors: true,
            },
            ExpectedMemoryChange {
                min_records: 1,
                require_danger_bias: false,
                require_safety_bias: true,
                require_social_trust_bias: false,
                require_social_fear_bias: false,
                require_curiosity_bias: true,
                require_bias_only_recall: false,
            },
            ExpectedTopologyChange {
                min_concepts: 2,
                min_edges: 1,
                min_simplexes: 1,
                min_gaps: 0,
                require_curiosity_bias: false,
                require_word_binding: Some(77),
                require_social_binding: false,
            },
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn fixture(
    name: ScenarioName,
    seed: u64,
    organism_id: OrganismId,
    initial_homeostasis: HomeostaticSnapshot,
    object_layout: Vec<ScenarioObjectSpec>,
    world: HeadlessWorld,
    sensory_timeline: Vec<ScenarioSensoryFrame>,
    steps: Vec<ScenarioStep>,
    expectations: ScenarioExpectations,
) -> Result<ScenarioFixture, ScaffoldContractError> {
    let creature = ScenarioCreatureConfig {
        organism_id,
        brain_tier: BrainScaleTier::Nano512,
        genome_seed: seed ^ 0x5C3A_011Fu64,
        initial_homeostasis,
    };
    creature.initial_homeostasis.validate_contract()?;
    Ok(ScenarioFixture {
        name,
        seed,
        creature,
        object_layout,
        sensory_timeline,
        steps,
        expectations,
        world,
    })
}

fn expectations(
    broad_behavior: &'static str,
    run_sleep_consolidation: bool,
    patch: ExpectedPatchFields,
    memory: ExpectedMemoryChange,
    topology: ExpectedTopologyChange,
) -> ScenarioExpectations {
    ScenarioExpectations {
        behavior: ScenarioExpectedBehavior {
            broad_behavior,
            run_sleep_consolidation,
        },
        patch,
        memory,
        topology,
    }
}

fn world_from_layout(
    seed: u64,
    layout: &[ScenarioObjectSpec],
) -> Result<HeadlessWorld, ScaffoldContractError> {
    let mut builder = HeadlessScenarioBuilder::new(seed);
    for object in layout {
        builder = match object.kind {
            WorldObjectKind::Agent => {
                let organism_id = object.organism_id.ok_or(ScaffoldContractError::InvalidId)?;
                if object.social_affinity == 0.0 {
                    builder.agent(object.label, organism_id, object.position)
                } else {
                    builder.social_agent(
                        object.label,
                        organism_id,
                        object.position,
                        object.social_affinity,
                    )
                }
            }
            WorldObjectKind::Food => builder.food(object.label, object.position, object.nutrition),
            WorldObjectKind::Hazard => {
                builder.hazard(object.label, object.position, object.hazard_pain)
            }
            WorldObjectKind::Obstacle => {
                builder.obstacle(object.label, object.position, object.radius)
            }
            WorldObjectKind::Token => {
                let token_id = object.token_id.ok_or(ScaffoldContractError::InvalidId)?;
                if let Some(channel) = object.teacher_channel {
                    builder.teacher_token(object.label, object.position, token_id, channel)
                } else {
                    builder.token(object.label, object.position, token_id)
                }
            }
        };
    }
    builder.build()
}

fn proposal(
    world: &HeadlessWorld,
    action_id: ActionId,
    kind: ActionKind,
    target_label: Option<&str>,
    target_position: Option<Vec3f>,
    score: f32,
    salience: f32,
) -> Result<ActionProposal, ScaffoldContractError> {
    let target = match target_label {
        Some(label) => Some(
            world
                .entity_id(label)
                .ok_or(ScaffoldContractError::InvalidId)?,
        ),
        None => None,
    };
    ActionProposal::new(
        action_id,
        kind,
        score,
        Confidence::new(0.9)?,
        None,
        0b111,
        ActionTarget::new(target, target_position),
        NormalizedScalar::new(salience)?,
    )?
    .with_intensity(Intensity::new(1.0)?)
    .tap_ok()
}

fn homeostasis(
    _seed: u64,
    configure: impl FnOnce(&mut HomeostaticSnapshot),
) -> Result<HomeostaticSnapshot, ScaffoldContractError> {
    let mut state = HomeostaticSnapshot::baseline(Tick::ZERO);
    configure(&mut state);
    state.validate_contract()?;
    Ok(state)
}

fn sensory(
    visible: Vec<&'static str>,
    tokens: Vec<u32>,
    teacher_channel: Option<TeacherPerceptionChannel>,
    social_agents: usize,
) -> Vec<ScenarioSensoryFrame> {
    vec![ScenarioSensoryFrame {
        tick: Tick::ZERO,
        expected_visible_labels: visible,
        expected_heard_tokens: tokens,
        expected_teacher_channel: teacher_channel,
        expected_social_agents: social_agents,
    }]
}

fn agent(label: &'static str, organism_id: OrganismId, position: Vec3f) -> ScenarioObjectSpec {
    object(label, WorldObjectKind::Agent, position)
        .with_organism(organism_id)
        .with_social_affinity(0.0)
}

fn social_agent(
    label: &'static str,
    organism_id: OrganismId,
    position: Vec3f,
    affinity: f32,
) -> ScenarioObjectSpec {
    object(label, WorldObjectKind::Agent, position)
        .with_organism(organism_id)
        .with_social_affinity(affinity)
}

fn food(label: &'static str, position: Vec3f, nutrition: f32) -> ScenarioObjectSpec {
    object(label, WorldObjectKind::Food, position).with_nutrition(nutrition)
}

fn hazard(label: &'static str, position: Vec3f, pain: f32) -> ScenarioObjectSpec {
    object(label, WorldObjectKind::Hazard, position).with_hazard_pain(pain)
}

fn obstacle(label: &'static str, position: Vec3f, radius: f32) -> ScenarioObjectSpec {
    object(label, WorldObjectKind::Obstacle, position).with_radius(radius)
}

fn token(label: &'static str, position: Vec3f, token_id: u32) -> ScenarioObjectSpec {
    object(label, WorldObjectKind::Token, position).with_token(token_id, None)
}

fn teacher_token(
    label: &'static str,
    position: Vec3f,
    token_id: u32,
    channel: TeacherPerceptionChannel,
) -> ScenarioObjectSpec {
    object(label, WorldObjectKind::Token, position).with_token(token_id, Some(channel))
}

fn object(label: &'static str, kind: WorldObjectKind, position: Vec3f) -> ScenarioObjectSpec {
    ScenarioObjectSpec {
        label,
        kind,
        organism_id: None,
        position,
        nutrition: 0.0,
        hazard_pain: 0.0,
        radius: 0.75,
        token_id: None,
        social_affinity: 0.0,
        teacher_channel: None,
    }
}

impl ScenarioObjectSpec {
    const fn with_organism(mut self, organism_id: OrganismId) -> Self {
        self.organism_id = Some(organism_id);
        self
    }

    fn with_nutrition(mut self, nutrition: f32) -> Self {
        self.nutrition = nutrition.clamp(0.0, 1.0);
        self
    }

    fn with_hazard_pain(mut self, pain: f32) -> Self {
        self.hazard_pain = pain.clamp(0.0, 1.0);
        self
    }

    fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.1);
        self
    }

    fn with_token(
        mut self,
        token_id: u32,
        teacher_channel: Option<TeacherPerceptionChannel>,
    ) -> Self {
        self.token_id = Some(token_id);
        self.teacher_channel = teacher_channel;
        self
    }

    fn with_social_affinity(mut self, affinity: f32) -> Self {
        self.social_affinity = affinity.clamp(-1.0, 1.0);
        self
    }
}

fn default_seed(name: ScenarioName) -> u64 {
    match name {
        ScenarioName::FoodSeeking => 18_001,
        ScenarioName::PoisonPainAvoidance => 18_002,
        ScenarioName::ObstacleFrustration => 18_003,
        ScenarioName::FatigueSleep => 18_004,
        ScenarioName::CuriosityContradiction => 18_005,
        ScenarioName::WordTokenGrounding => 18_006,
        ScenarioName::SimpleSocialTrustFear => 18_007,
        ScenarioName::TeacherPerceptionEvent => 18_008,
    }
}

fn pos(x: f32, y: f32) -> Vec3f {
    Vec3f::new(x, y, 0.0)
}

fn milli_unit(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn milli_signed(value: f32) -> i32 {
    (value.clamp(-1.0, 1.0) * 1000.0).round() as i32
}

trait TapOk: Sized {
    fn tap_ok(self) -> Result<Self, ScaffoldContractError>;
}

impl<T> TapOk for T {
    fn tap_ok(self) -> Result<Self, ScaffoldContractError> {
        Ok(self)
    }
}

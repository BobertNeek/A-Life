use alife_core::{
    ActionCommand, ActionId, ActionKind, ActionProposal, ActionTarget, BrainScaleTier,
    BrainTickInput, BrainTickStatus, Confidence, CreatureMind, DurationTicks, EndocrineDelta,
    HomeostaticDelta, Intensity, NormalizedScalar, OrganismId, PhysicalActionOutcome,
    PhysicalContactKind, ReferenceActionExecution, ReferenceActionExecutor, ReferenceActionFailure,
    ReferenceOutcomeObservation, ReferenceOutcomeObserver, ReferenceOutcomeRequest,
    ReferenceSensoryAdapter, ReferenceSensoryRequest, ScaffoldContractError, SensoryChannels,
    SensorySnapshot, SignedValence, Tick, Validate, Vec3f, WorldEntityId,
};

fn organism() -> OrganismId {
    OrganismId(77)
}

fn target() -> WorldEntityId {
    WorldEntityId(9)
}

fn proposal(action_id: u32, kind: ActionKind, score: f32) -> ActionProposal {
    ActionProposal::new(
        ActionId::new(action_id).unwrap(),
        kind,
        score,
        Confidence::new(0.8).unwrap(),
        None,
        0b101,
        ActionTarget::new(Some(target()), Some(Vec3f::new(0.0, 0.0, 1.0))),
        NormalizedScalar::new(0.6).unwrap(),
    )
    .unwrap()
    .with_intensity(Intensity::new(0.7).unwrap())
}

fn low_proposal() -> ActionProposal {
    proposal(700, ActionKind::Move, 0.01)
}

fn normal_proposals() -> Vec<ActionProposal> {
    vec![
        proposal(
            ActionKind::Move.canonical_id().raw(),
            ActionKind::Move,
            0.35,
        ),
        proposal(
            ActionKind::Interact.canonical_id().raw(),
            ActionKind::Interact,
            0.75,
        ),
    ]
}

fn recovery_proposals() -> Vec<ActionProposal> {
    vec![
        proposal(
            ActionKind::Interact.canonical_id().raw(),
            ActionKind::Interact,
            0.55,
        ),
        proposal(
            ActionKind::Rest.canonical_id().raw(),
            ActionKind::Rest,
            0.35,
        ),
    ]
}

fn mind() -> CreatureMind {
    CreatureMind::scaffold(organism(), BrainScaleTier::Nano512, 42, Tick::ZERO).unwrap()
}

#[derive(Clone)]
struct FixtureSensory {
    cue: f32,
}

impl FixtureSensory {
    fn new(cue: f32) -> Self {
        Self { cue }
    }
}

impl ReferenceSensoryAdapter for FixtureSensory {
    fn gather_sensory(
        &mut self,
        request: ReferenceSensoryRequest,
    ) -> Result<SensorySnapshot, ScaffoldContractError> {
        let mut visual = [0.0; alife_core::SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT];
        visual[0] = self.cue;
        visual[1] = 1.0 - self.cue;
        let channels = SensoryChannels::try_from_groups(
            visual,
            [0.0; alife_core::SENSORY_AUDITORY_CHANNEL_COUNT],
            [0.0; alife_core::SENSORY_SMELL_CHANNEL_COUNT],
            [0.0; alife_core::SENSORY_TACTILE_CHANNEL_COUNT],
            NormalizedScalar::new(0.0).unwrap(),
            NormalizedScalar::new(self.cue).unwrap(),
            Default::default(),
        )?;

        SensorySnapshot::new(
            request.organism_id,
            request.tick,
            request.body_pose.translation,
            channels,
            Default::default(),
        )
    }
}

#[derive(Clone)]
struct FixtureExecutor {
    fail: bool,
    attempts: usize,
}

impl FixtureExecutor {
    fn success() -> Self {
        Self {
            fail: false,
            attempts: 0,
        }
    }

    fn missing_affordance() -> Self {
        Self {
            fail: true,
            attempts: 0,
        }
    }
}

impl ReferenceActionExecutor for FixtureExecutor {
    fn execute_action(
        &mut self,
        command: &ActionCommand,
    ) -> Result<ReferenceActionExecution, ScaffoldContractError> {
        self.attempts += 1;
        let physical = PhysicalActionOutcome {
            contact: if self.fail {
                PhysicalContactKind::Blocked
            } else {
                PhysicalContactKind::Touch
            },
            target_entity: command.target_entity,
            displacement: if self.fail {
                Vec3f::ZERO
            } else {
                Vec3f::new(0.0, 0.0, 0.25)
            },
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        };
        if self.fail {
            ReferenceActionExecution::failed(ReferenceActionFailure::MissingAffordance, physical)
        } else {
            ReferenceActionExecution::succeeded(physical)
        }
    }
}

#[derive(Clone)]
struct FixtureOutcome {
    reward: f32,
    contradiction: bool,
    invalid_learning_value: bool,
}

impl FixtureOutcome {
    fn reward(reward: f32) -> Self {
        Self {
            reward,
            contradiction: false,
            invalid_learning_value: false,
        }
    }

    fn contradiction() -> Self {
        Self {
            reward: -0.75,
            contradiction: true,
            invalid_learning_value: false,
        }
    }

    fn invalid() -> Self {
        Self {
            reward: f32::NAN,
            contradiction: false,
            invalid_learning_value: true,
        }
    }
}

impl ReferenceOutcomeObserver for FixtureOutcome {
    fn observe_outcome(
        &mut self,
        request: ReferenceOutcomeRequest<'_>,
    ) -> Result<ReferenceOutcomeObservation, ScaffoldContractError> {
        let failed = !request.execution.succeeded;
        let pain = if failed || self.contradiction {
            0.55
        } else {
            0.0
        };
        let reward = if self.invalid_learning_value {
            f32::NAN
        } else if failed {
            -0.8
        } else {
            self.reward
        };
        ReferenceOutcomeObservation::new(
            !failed,
            HomeostaticDelta {
                drives: alife_core::DriveDelta {
                    pain,
                    brain_atp: -0.05,
                    curiosity: if self.contradiction { 0.25 } else { 0.0 },
                    ..alife_core::DriveDelta::zero()
                },
                hormones: EndocrineDelta {
                    dopamine: if reward > 0.0 { 0.1 } else { -0.05 },
                    cortisol: if failed { 0.2 } else { 0.0 },
                    ..EndocrineDelta::zero()
                },
            },
            SignedValence::new(reward)?,
            NormalizedScalar::new(if failed { 0.7 } else { 0.05 })?,
            NormalizedScalar::new(pain)?,
            SignedValence::new(-0.05)?,
            NormalizedScalar::new(if self.contradiction || failed {
                0.9
            } else {
                0.1
            })?,
        )
        .map(|mut observation| {
            observation.contradiction_observed = self.contradiction || failed;
            observation
        })
    }
}

fn tick_input(tick: u64, proposals: Vec<ActionProposal>) -> BrainTickInput {
    BrainTickInput::new(Tick::new(tick), proposals)
        .with_pack_experience(true)
        .with_action_duration(DurationTicks::new(2))
}

#[test]
fn full_normal_tick_seals_patch_updates_subsystems_and_increments_tick() {
    let mut mind = mind();
    let mut sensory = FixtureSensory::new(0.8);
    let mut executor = FixtureExecutor::success();
    let mut outcome = FixtureOutcome::reward(0.65);

    let output = mind.tick(
        tick_input(0, normal_proposals()),
        &mut sensory,
        &mut executor,
        &mut outcome,
    );

    assert_eq!(output.status, BrainTickStatus::Normal);
    assert_eq!(
        output.selected_action.unwrap().kind,
        ActionKind::Interact,
        "P09 arbitration remains the action-selection path"
    );
    assert!(output.experience_patch.is_some());
    assert!(output.packed_record.is_some());
    assert_eq!(output.memory_update.unwrap(), alife_core::MemoryId(1));
    assert!(output
        .topology_update
        .as_ref()
        .unwrap()
        .simplex_id
        .is_valid());
    assert_eq!(mind.memory_bank().len(), 1);
    assert!(!mind.topological_map().concepts().is_empty());
    assert_eq!(mind.current_tick(), Tick::new(1));
    assert_eq!(mind.homeostasis().tick, Tick::new(1));
    assert!(mind.homeostasis().drives.brain_atp < 0.75);
    assert_eq!(executor.attempts, 1);
    assert!(output.neural_report.active_tiles <= mind.brain_class().max_active_microtiles);
}

#[test]
fn failed_action_path_is_bounded_sealed_and_biases_away_next_tick() {
    let mut mind = mind();
    let mut sensory = FixtureSensory::new(0.9);
    let mut executor = FixtureExecutor::missing_affordance();
    let mut outcome = FixtureOutcome::contradiction();

    let failed = mind.tick(
        tick_input(
            0,
            vec![proposal(
                ActionKind::Interact.canonical_id().raw(),
                ActionKind::Interact,
                0.8,
            )],
        ),
        &mut sensory,
        &mut executor,
        &mut outcome,
    );

    assert_eq!(failed.status, BrainTickStatus::RecoverableActionFailure);
    assert_eq!(
        executor.attempts, 1,
        "P15 must not retry infinitely in one tick"
    );
    assert!(
        failed
            .experience_patch
            .as_ref()
            .unwrap()
            .outcome()
            .contradiction_observed
    );
    assert!(!failed.topology_update.as_ref().unwrap().gap_ids.is_empty());
    assert_eq!(mind.memory_bank().len(), 1);
    assert_eq!(failed.diagnostics.recoverable_action_failures, 1);

    let mut executor = FixtureExecutor::success();
    let mut outcome = FixtureOutcome::reward(0.1);
    let next = mind.tick(
        tick_input(1, recovery_proposals()),
        &mut sensory,
        &mut executor,
        &mut outcome,
    );

    assert_ne!(
        next.selected_action.unwrap().kind,
        ActionKind::Interact,
        "recent failed action state may discourage a repeat, but memory itself does not replay actions"
    );
}

#[test]
fn terminal_invalid_state_rejects_without_learning_or_tick_increment() {
    let mut mind = mind();
    mind.homeostasis_mut().drives.hunger = f32::NAN;
    let mut sensory = FixtureSensory::new(0.8);
    let mut executor = FixtureExecutor::success();
    let mut outcome = FixtureOutcome::reward(0.5);

    let output = mind.tick(
        tick_input(0, normal_proposals()),
        &mut sensory,
        &mut executor,
        &mut outcome,
    );

    assert_eq!(output.status, BrainTickStatus::TerminalInvalidState);
    assert!(output.selected_action.is_none());
    assert!(output.experience_patch.is_none());
    assert_eq!(mind.memory_bank().len(), 0);
    assert!(mind.topological_map().concepts().is_empty());
    assert_eq!(mind.current_tick(), Tick::ZERO);
}

#[test]
fn neutral_fallback_selects_safe_idle_patch_when_no_proposal_passes() {
    let mut mind = mind();
    let mut sensory = FixtureSensory::new(0.2);
    let mut executor = FixtureExecutor::success();
    let mut outcome = FixtureOutcome::reward(0.0);

    let output = mind.tick(
        tick_input(0, vec![low_proposal()]),
        &mut sensory,
        &mut executor,
        &mut outcome,
    );

    assert_eq!(output.status, BrainTickStatus::SafeIdle);
    assert_eq!(output.selected_action.unwrap().kind, ActionKind::Inspect);
    assert!(output.experience_patch.is_some());
    assert_eq!(mind.memory_bank().len(), 1);
}

#[test]
fn invalid_outcome_cannot_update_memory_topology_or_learning() {
    let mut mind = mind();
    let mut sensory = FixtureSensory::new(0.8);
    let mut executor = FixtureExecutor::success();
    let mut outcome = FixtureOutcome::invalid();

    let output = mind.tick(
        tick_input(0, normal_proposals()),
        &mut sensory,
        &mut executor,
        &mut outcome,
    );

    assert_eq!(output.status, BrainTickStatus::TerminalInvalidState);
    assert!(output.experience_patch.is_none());
    assert_eq!(mind.memory_bank().len(), 0);
    assert!(mind.topological_map().concepts().is_empty());
    assert_eq!(output.diagnostics.learning_updates, 0);
    assert_eq!(mind.current_tick(), Tick::ZERO);
}

#[test]
fn topology_contradiction_output_remains_bias_only() {
    let mut mind = mind();
    let mut sensory = FixtureSensory::new(0.9);
    let mut executor = FixtureExecutor::missing_affordance();
    let mut outcome = FixtureOutcome::contradiction();

    let output = mind.tick(
        tick_input(0, normal_proposals()),
        &mut sensory,
        &mut executor,
        &mut outcome,
    );

    assert_eq!(output.status, BrainTickStatus::RecoverableActionFailure);
    assert!(!mind.topological_map().curiosity_biases().is_empty());
    fn curiosity_is_not_action(_: Vec<alife_core::CuriosityBias>) {}
    curiosity_is_not_action(mind.topological_map().curiosity_biases());
}

#[test]
fn endocrine_update_after_outcome_remains_bounded() {
    let mut mind = mind();
    let mut sensory = FixtureSensory::new(0.8);
    let mut executor = FixtureExecutor::success();
    let mut outcome = FixtureOutcome::reward(0.5);

    let output = mind.tick(
        tick_input(0, normal_proposals()),
        &mut sensory,
        &mut executor,
        &mut outcome,
    );

    assert_eq!(output.status, BrainTickStatus::Normal);
    assert!(mind.homeostasis().drives.brain_atp < 0.75);
    assert!(mind.homeostasis().hormones.dopamine > 0.5);
    assert!(mind.homeostasis().validate_contract().is_ok());
}

#[test]
fn same_seed_replay_is_deterministic_across_multiple_ticks() {
    let mut first = mind();
    let mut second = mind();
    let mut first_sensory = FixtureSensory::new(0.7);
    let mut second_sensory = FixtureSensory::new(0.7);
    let mut first_executor = FixtureExecutor::success();
    let mut second_executor = FixtureExecutor::success();
    let mut first_outcome = FixtureOutcome::reward(0.4);
    let mut second_outcome = FixtureOutcome::reward(0.4);

    for tick in 0..3 {
        let a = first.tick(
            tick_input(tick, normal_proposals()),
            &mut first_sensory,
            &mut first_executor,
            &mut first_outcome,
        );
        let b = second.tick(
            tick_input(tick, normal_proposals()),
            &mut second_sensory,
            &mut second_executor,
            &mut second_outcome,
        );

        assert_eq!(a.status, b.status);
        assert_eq!(a.selected_action, b.selected_action);
        assert_eq!(
            a.experience_patch
                .as_ref()
                .map(|patch| patch.header().sequence_id),
            b.experience_patch
                .as_ref()
                .map(|patch| patch.header().sequence_id)
        );
        assert_eq!(first.memory_bank().len(), second.memory_bank().len());
        assert_eq!(
            first.topological_map().concepts().len(),
            second.topological_map().concepts().len()
        );
    }
}

#[test]
fn core_reference_brain_contract_stays_engine_independent() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("crate manifest should be readable");
    for forbidden in [
        concat!("be", "vy"),
        concat!("av", "ian"),
        concat!("wg", "pu"),
    ] {
        assert!(
            !manifest.to_ascii_lowercase().contains(forbidden),
            "alife_core manifest must not depend on {forbidden}"
        );
    }

    let source = include_str!("../src/reference_brain.rs");
    for forbidden in [
        concat!("be", "vy"),
        concat!("av", "ian"),
        concat!("wg", "pu"),
        concat!("Render", "Device"),
        concat!("Render", "Queue"),
        concat!("Ent", "ity"),
    ] {
        assert!(
            !source.contains(forbidden),
            "reference brain loop must not embed engine type {forbidden}"
        );
    }
}

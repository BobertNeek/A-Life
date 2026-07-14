use std::{fs, path::PathBuf};

use alife_core::{
    ActionKind, ExperiencePatchBuilder, ExperiencePatchPhase, ExperienceSequenceId,
    ScaffoldContractError, SchemaVersions, SleepPhase, Validate, WorldEntityId,
};
use alife_world::{
    ActionLegality, ActionLegalityChecker, HeadlessWorldCommand, ScenarioFixture, ScenarioName,
    ScenarioRun,
};
use serde::{Deserialize, Serialize};

const GOLDEN_TRACE_SCHEMA: &str = "alife.p19.golden_trace.v1";
const GOLDEN_TRACE_SCHEMA_VERSION: u16 = 1;
const STOCHASTIC_SEED_FIELD: &str = "scenario.seed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GoldenTrace {
    schema: String,
    schema_version: u16,
    tolerances: GoldenTolerances,
    scenario: GoldenScenario,
    patches: Vec<GoldenPatch>,
    state: GoldenState,
    neural_ticks: Vec<GoldenNeuralTick>,
    world_signature: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GoldenTolerances {
    milliscale_absolute: u16,
    stochastic_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GoldenScenario {
    key: String,
    label: String,
    seed: u64,
    experience_schema_version: u16,
    action_abi_version: u16,
    sensory_abi_version: u16,
    chemistry_schema_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GoldenPatch {
    index: usize,
    phase_sequence: Vec<String>,
    tick: u64,
    outcome_tick: u64,
    status: String,
    failure: Option<String>,
    selected_action_id: u32,
    selected_action_kind: String,
    target_entity: Option<u64>,
    success: bool,
    contact: String,
    reward_milli: i32,
    pain_milli: u16,
    frustration_milli: u16,
    prediction_error_milli: u16,
    contradiction: bool,
    memory_danger_milli: u16,
    memory_salience_milli: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GoldenState {
    patch_count: usize,
    memory_record_count: usize,
    topology_concept_count: usize,
    topology_edge_count: usize,
    topology_simplex_count: usize,
    topology_gap_count: usize,
    curiosity_bias_count: usize,
    sleep_phase: String,
    sleep_cycle_count: u32,
    pending_structural_edit_count: usize,
    final_drive_milli: Vec<u16>,
    final_hormone_milli: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GoldenNeuralTick {
    index: usize,
    active_tiles: u32,
    active_synapses: u32,
    mask_skipped_tiles: u32,
    range_rejections: u32,
}

#[test]
fn p18_scenarios_match_versioned_golden_trace_fixtures() {
    for name in ScenarioName::ALL {
        let fixture = ScenarioFixture::named(name).unwrap();
        let run = fixture.run().unwrap();
        let actual = GoldenTrace::from_run(&run);
        if update_golden_traces_enabled() {
            write_golden_trace(scenario_key(name), &actual);
            continue;
        }
        let expected = read_golden_trace(scenario_key(name));
        assert_trace_matches(&expected, &actual);
    }
}

#[test]
fn same_seed_replay_produces_identical_golden_trace_summary() {
    for name in ScenarioName::ALL {
        let seed = 0xA11F_EED0 + scenario_index(name) as u64;
        let first = ScenarioFixture::with_seed(name, seed)
            .unwrap()
            .run()
            .unwrap();
        let second = ScenarioFixture::with_seed(name, seed)
            .unwrap()
            .run()
            .unwrap();

        assert_eq!(
            GoldenTrace::from_run(&first),
            GoldenTrace::from_run(&second),
            "same-seed replay drifted for {}",
            scenario_key(name)
        );
    }
}

#[test]
fn different_seed_replay_changes_only_declared_stochastic_trace_fields() {
    for name in ScenarioName::ALL {
        let default = ScenarioFixture::named(name).unwrap().run().unwrap();
        let alternate = ScenarioFixture::with_seed(name, default.seed + 10_000)
            .unwrap()
            .run()
            .unwrap();

        let mut default_trace = GoldenTrace::from_run(&default);
        let mut alternate_trace = GoldenTrace::from_run(&alternate);
        default_trace.clear_stochastic_fields();
        alternate_trace.clear_stochastic_fields();

        assert_eq!(
            default_trace,
            alternate_trace,
            "non-stochastic fields changed for {}",
            scenario_key(name)
        );
    }
}

#[test]
fn bounded_randomized_scenario_loops_preserve_learning_invariants() {
    let mut rng = DeterministicTestRng::new(0x19_0019_0019);
    for case_index in 0..24 {
        let name = ScenarioName::ALL[(rng.next_u32() as usize) % ScenarioName::ALL.len()];
        let seed = 40_000 + rng.next_u32() as u64;
        let fixture = ScenarioFixture::with_seed(name, seed).unwrap();
        let run = fixture.run().unwrap();

        assert!(
            !run.patches.is_empty(),
            "case {case_index}: no sealed patches"
        );
        assert_homeostasis_is_bounded(&run, case_index);
        assert_monotonic_patch_ticks(&run, case_index);
        assert_valid_ids_and_actions(&fixture, &run, case_index);
        assert_sealed_patch_consumers_only_observe_sealed_patches(&run, case_index);
    }
}

#[test]
fn partial_experience_builders_reject_snapshot_update_inputs_before_seal() {
    let sequence = ExperienceSequenceId::new(1).unwrap();
    let error = ExperiencePatchBuilder::new(sequence).seal().unwrap_err();
    assert_eq!(error, ScaffoldContractError::MissingPhaseData);
}

fn read_golden_trace(key: &str) -> GoldenTrace {
    let path = golden_fixture_path(key);
    let json = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read golden trace {}: {error}", path.display()));
    serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("failed to parse golden trace {}: {error}", path.display()))
}

fn write_golden_trace(key: &str, trace: &GoldenTrace) {
    let path = golden_fixture_path(key);
    fs::create_dir_all(path.parent().expect("golden trace fixture directory"))
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", path.display()));
    let json = serde_json::to_string_pretty(trace).expect("golden trace serializes");
    fs::write(&path, format!("{json}\n"))
        .unwrap_or_else(|error| panic!("failed to write golden trace {}: {error}", path.display()));
}

fn update_golden_traces_enabled() -> bool {
    std::env::var_os("ALIFE_UPDATE_GOLDEN_TRACES").is_some()
}

fn golden_fixture_path(key: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("golden_traces")
        .join(format!("{key}.json"))
}

fn assert_trace_matches(expected: &GoldenTrace, actual: &GoldenTrace) {
    assert_eq!(expected.schema, GOLDEN_TRACE_SCHEMA);
    assert_eq!(expected.schema_version, GOLDEN_TRACE_SCHEMA_VERSION);
    assert_eq!(expected.tolerances.milliscale_absolute, 0);
    assert_eq!(
        expected.tolerances.stochastic_fields,
        vec![STOCHASTIC_SEED_FIELD.to_string()]
    );
    assert_eq!(
        expected,
        actual,
        "{}",
        golden_trace_diagnostic(expected, actual)
    );
}

fn golden_trace_diagnostic(expected: &GoldenTrace, actual: &GoldenTrace) -> String {
    let mut lines = vec![format!(
        "golden trace mismatch for scenario '{}'",
        actual.scenario.key
    )];
    if expected.scenario != actual.scenario {
        lines.push(format!(
            "scenario header differs: expected {:?}, actual {:?}",
            expected.scenario, actual.scenario
        ));
    }
    if expected.state != actual.state {
        lines.push(format!(
            "state summary differs: expected {:?}, actual {:?}",
            expected.state, actual.state
        ));
    }
    if expected.world_signature != actual.world_signature {
        lines.push(format!(
            "world signature differs: expected {:?}, actual {:?}",
            expected.world_signature, actual.world_signature
        ));
    }
    if expected.neural_ticks != actual.neural_ticks {
        lines.push(first_index_mismatch(
            "neural tick",
            &expected.neural_ticks,
            &actual.neural_ticks,
        ));
    }
    if expected.patches != actual.patches {
        lines.push(first_index_mismatch(
            "patch",
            &expected.patches,
            &actual.patches,
        ));
    }
    lines.join("\n")
}

fn first_index_mismatch<T: std::fmt::Debug + PartialEq>(
    label: &str,
    expected: &[T],
    actual: &[T],
) -> String {
    let max = expected.len().max(actual.len());
    for index in 0..max {
        match (expected.get(index), actual.get(index)) {
            (Some(left), Some(right)) if left != right => {
                return format!("{label} {index} differs: expected {left:?}, actual {right:?}");
            }
            (Some(left), None) => {
                return format!("{label} {index} missing in actual: expected {left:?}");
            }
            (None, Some(right)) => {
                return format!("{label} {index} unexpected in actual: {right:?}");
            }
            _ => {}
        }
    }
    format!("{label} vectors differ")
}

fn assert_homeostasis_is_bounded(run: &ScenarioRun, case_index: usize) {
    for value in run.final_homeostasis.drives.to_array() {
        assert!(
            value.is_finite() && (0.0..=1.0).contains(&value),
            "case {case_index}: invalid final drive {value}"
        );
    }
    for value in run.final_homeostasis.hormones.to_array() {
        assert!(
            value.is_finite() && (0.0..=1.0).contains(&value),
            "case {case_index}: invalid final hormone {value}"
        );
    }
    run.final_homeostasis.validate_contract().unwrap();
}

fn assert_monotonic_patch_ticks(run: &ScenarioRun, case_index: usize) {
    let mut last_tick = None;
    for patch in &run.patches {
        let tick = patch.pre_action().tick.raw();
        let outcome_tick = patch.outcome().outcome_tick.raw();
        assert!(
            outcome_tick >= tick,
            "case {case_index}: outcome tick precedes pre-action tick"
        );
        if let Some(last_tick) = last_tick {
            assert!(
                tick >= last_tick,
                "case {case_index}: patch ticks are not monotonic"
            );
        }
        last_tick = Some(tick);
    }
}

fn assert_valid_ids_and_actions(fixture: &ScenarioFixture, run: &ScenarioRun, case_index: usize) {
    assert!(
        fixture.creature.organism_id.raw() > 0,
        "case {case_index}: invalid organism id"
    );
    for patch in &run.patches {
        patch
            .decision()
            .selected_action
            .validate_contract()
            .unwrap();
        if let Some(target) = patch.decision().selected_action.target_entity {
            assert!(target.raw() > 0, "case {case_index}: invalid target id");
        }
    }

    let invalid_target =
        HeadlessWorldCommand::eat(fixture.creature.organism_id, WorldEntityId(999_999)).unwrap();
    assert_eq!(
        fixture.world.check_action(&invalid_target),
        ActionLegality::ImpossibleTarget,
        "case {case_index}: invalid target was not rejected"
    );
}

fn assert_sealed_patch_consumers_only_observe_sealed_patches(run: &ScenarioRun, case_index: usize) {
    for patch in &run.patches {
        assert_eq!(
            patch.header().phase,
            ExperiencePatchPhase::Sealed,
            "case {case_index}: unsealed patch reached scenario run"
        );
        patch.validate_contract().unwrap();
        assert_eq!(
            patch.phase_sequence().last(),
            Some(&ExperiencePatchPhase::Sealed)
        );
    }
    assert_eq!(
        run.memory_record_count,
        run.patches.len(),
        "case {case_index}: memory did not consume one sealed record per patch"
    );
    assert!(
        run.topology_simplex_count >= run.patches.len(),
        "case {case_index}: topology did not bind sealed patches into simplexes"
    );
}

impl GoldenTrace {
    fn from_run(run: &ScenarioRun) -> Self {
        Self {
            schema: GOLDEN_TRACE_SCHEMA.to_string(),
            schema_version: GOLDEN_TRACE_SCHEMA_VERSION,
            tolerances: GoldenTolerances {
                milliscale_absolute: 0,
                stochastic_fields: vec![STOCHASTIC_SEED_FIELD.to_string()],
            },
            scenario: GoldenScenario {
                key: scenario_key(run.name).to_string(),
                label: run.name.as_str().to_string(),
                seed: run.seed,
                experience_schema_version: SchemaVersions::CURRENT.experience.raw(),
                action_abi_version: SchemaVersions::CURRENT.action_abi.raw(),
                sensory_abi_version: SchemaVersions::CURRENT.sensory_abi.raw(),
                chemistry_schema_version: SchemaVersions::CURRENT.chemistry.raw(),
            },
            patches: run
                .patches
                .iter()
                .enumerate()
                .map(|(index, patch)| {
                    let memory_expectancy = &patch
                        .pre_action()
                        .heuristic_evidence()
                        .expect("scenario patches use heuristic baseline evidence")
                        .memory_expectancy;
                    GoldenPatch {
                        index,
                        phase_sequence: patch
                            .phase_sequence()
                            .into_iter()
                            .map(|phase| format!("{phase:?}"))
                            .collect(),
                        tick: patch.pre_action().tick.raw(),
                        outcome_tick: patch.outcome().outcome_tick.raw(),
                        status: format!("{:?}", run.statuses[index]),
                        failure: run.failures[index].map(|failure| format!("{failure:?}")),
                        selected_action_id: patch.decision().selected_action.action_id.raw(),
                        selected_action_kind: action_kind_name(
                            patch.decision().selected_action.kind,
                        )
                        .to_string(),
                        target_entity: patch
                            .decision()
                            .selected_action
                            .target_entity
                            .map(WorldEntityId::raw),
                        success: patch.outcome().success,
                        contact: format!("{:?}", patch.outcome().physical.contact),
                        reward_milli: milli_signed(patch.outcome().reward_valence.raw()),
                        pain_milli: milli_unit(patch.outcome().pain_delta.raw()),
                        frustration_milli: milli_unit(patch.outcome().frustration_delta.raw()),
                        prediction_error_milli: milli_unit(patch.outcome().prediction_error.raw()),
                        contradiction: patch.outcome().contradiction_observed,
                        memory_danger_milli: milli_unit(memory_expectancy.danger_bias.raw()),
                        memory_salience_milli: milli_unit(memory_expectancy.salience_hint.raw()),
                    }
                })
                .collect(),
            state: GoldenState {
                patch_count: run.patches.len(),
                memory_record_count: run.memory_record_count,
                topology_concept_count: run.topology_concept_count,
                topology_edge_count: run.topology_edge_count,
                topology_simplex_count: run.topology_simplex_count,
                topology_gap_count: run.topology_gap_ids.len(),
                curiosity_bias_count: run.curiosity_biases.len(),
                sleep_phase: sleep_phase_name(run.sleep_phase).to_string(),
                sleep_cycle_count: run.sleep_cycle_count,
                pending_structural_edit_count: run.pending_structural_edit_count,
                final_drive_milli: run
                    .final_homeostasis
                    .drives
                    .to_array()
                    .into_iter()
                    .map(milli_unit)
                    .collect(),
                final_hormone_milli: run
                    .final_homeostasis
                    .hormones
                    .to_array()
                    .into_iter()
                    .map(milli_unit)
                    .collect(),
            },
            neural_ticks: run
                .ticks
                .iter()
                .enumerate()
                .map(|(index, tick)| GoldenNeuralTick {
                    index,
                    active_tiles: tick.brain.neural_report.active_tiles,
                    active_synapses: tick.brain.neural_report.active_synapses,
                    mask_skipped_tiles: tick.brain.neural_report.mask_skipped_tiles,
                    range_rejections: tick.brain.neural_report.range_rejections,
                })
                .collect(),
            world_signature: run.world_signature.clone(),
        }
    }

    fn clear_stochastic_fields(&mut self) {
        self.scenario.seed = 0;
    }
}

fn scenario_key(name: ScenarioName) -> &'static str {
    match name {
        ScenarioName::FoodSeeking => "food-seeking",
        ScenarioName::PoisonPainAvoidance => "poison-pain-avoidance",
        ScenarioName::ObstacleFrustration => "obstacle-frustration",
        ScenarioName::FatigueSleep => "fatigue-sleep",
        ScenarioName::CuriosityContradiction => "curiosity-contradiction",
        ScenarioName::WordTokenGrounding => "word-token-grounding",
        ScenarioName::SimpleSocialTrustFear => "simple-social-trust-fear",
        ScenarioName::TeacherPerceptionEvent => "teacher-perception-event",
    }
}

fn scenario_index(name: ScenarioName) -> usize {
    ScenarioName::ALL
        .iter()
        .position(|candidate| *candidate == name)
        .expect("scenario in ALL")
}

fn action_kind_name(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::Idle => "Idle",
        ActionKind::Move => "Move",
        ActionKind::Interact => "Interact",
        ActionKind::Rest => "Rest",
        ActionKind::Inspect => "Inspect",
        ActionKind::Hold => "Hold",
        ActionKind::Vocalize => "Vocalize",
        ActionKind::Write => "Write",
        ActionKind::Gesture => "Gesture",
    }
}

fn sleep_phase_name(phase: SleepPhase) -> &'static str {
    match phase {
        SleepPhase::Awake => "Awake",
        SleepPhase::EnteringSleep => "EnteringSleep",
        SleepPhase::Consolidating => "Consolidating",
        SleepPhase::Waking => "Waking",
        SleepPhase::ForcedRecoverySleep => "ForcedRecoverySleep",
    }
}

fn milli_unit(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn milli_signed(value: f32) -> i32 {
    (value.clamp(-1.0, 1.0) * 1000.0).round() as i32
}

#[derive(Debug, Clone, Copy)]
struct DeterministicTestRng {
    state: u64,
}

impl DeterministicTestRng {
    const fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0xC0DE_0019_A11F_EE19,
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }
}

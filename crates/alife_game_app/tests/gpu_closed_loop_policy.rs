use alife_core::{PolicyBackend, ScaffoldContractError};
use alife_game_app::{
    run_gpu_closed_loop_smoke_with_factory, AppShellLaunchConfig, BrainPolicyRuntime,
    GameAppShellError, GraphicalBrainPolicyMode, GraphicalPlaygroundLaunchConfig, LiveBrainLoop,
    ProductionRequiredGpuFactory, RequiredGpuFactory,
};
use alife_gpu_backend::GpuClosedLoopBackend;

const FIXTURE_ROOT: &str = "../alife_world/tests/fixtures/p34";

struct UnavailableGpuFactory;

impl RequiredGpuFactory for UnavailableGpuFactory {
    fn new_required(&self) -> Result<GpuClosedLoopBackend, ScaffoldContractError> {
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    }
}

#[test]
fn graphical_default_is_explicitly_gpu_required() {
    let launch = GraphicalPlaygroundLaunchConfig::interactive(FIXTURE_ROOT);

    assert_eq!(launch.brain_policy, PolicyBackend::NeuralClosedLoopGpu);
    assert!(launch.brain_policy.requires_gpu());
}

#[test]
fn graphical_policy_cli_accepts_exactly_the_two_documented_labels() {
    assert_eq!(
        GraphicalBrainPolicyMode::parse("gpu-required").unwrap(),
        GraphicalBrainPolicyMode::GpuRequired
    );
    assert_eq!(
        GraphicalBrainPolicyMode::parse("heuristic-baseline").unwrap(),
        GraphicalBrainPolicyMode::HeuristicBaseline
    );
    assert_eq!(
        GraphicalBrainPolicyMode::GpuRequired.label(),
        "gpu-required"
    );
    assert_eq!(
        GraphicalBrainPolicyMode::HeuristicBaseline.label(),
        "heuristic-baseline"
    );

    for rejected in [
        "",
        "gpu",
        "cpu-reference",
        "static-cpu-shadow-guarded",
        "static-plastic-cpu-shadow-guarded",
        "full-cpu-shadow-guarded",
        "auto-with-cpu-fallback",
        "GPU-REQUIRED",
        "heuristic_baseline",
    ] {
        assert!(
            GraphicalBrainPolicyMode::parse(rejected).is_err(),
            "obsolete or noncanonical policy label was accepted: {rejected}"
        );
    }
}

#[test]
fn requires_gpu_is_derived_only_from_explicit_policy_intent() {
    assert!(PolicyBackend::NeuralClosedLoopGpu.requires_gpu());
    assert!(!PolicyBackend::HeuristicBaseline.requires_gpu());
}

#[test]
fn explicit_heuristic_runtime_ticks_only_when_deliberately_selected() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(FIXTURE_ROOT);
    let loop_ = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut runtime = BrainPolicyRuntime::Heuristic(Box::new(loop_));

    assert_eq!(runtime.policy(), PolicyBackend::HeuristicBaseline);
    let summary = runtime.tick().unwrap();
    assert!(summary.patch_sealed);
}

#[test]
fn unavailable_gpu_returns_typed_error_without_selecting_heuristic_baseline() {
    let launch = GraphicalPlaygroundLaunchConfig::interactive(FIXTURE_ROOT);
    let result = run_gpu_closed_loop_smoke_with_factory(launch, &UnavailableGpuFactory);

    assert!(matches!(
        result,
        Err(GameAppShellError::NeuralBackendUnavailable { .. })
    ));
}

#[test]
fn production_gpu_factory_builds_neural_runtime_and_seals_a_tick() {
    let launch = GraphicalPlaygroundLaunchConfig::interactive(FIXTURE_ROOT);
    let mut runtime =
        run_gpu_closed_loop_smoke_with_factory(launch, &ProductionRequiredGpuFactory).unwrap();

    assert_eq!(runtime.policy(), PolicyBackend::NeuralClosedLoopGpu);
    let summary = runtime.tick().unwrap();
    assert!(summary.patch_sealed);
}

#[test]
fn neural_unavailability_has_a_typed_error_and_no_policy_fallback_branch() {
    fn is_neural_unavailable(error: &GameAppShellError) -> bool {
        matches!(error, GameAppShellError::NeuralBackendUnavailable { .. })
    }

    let policy_source = include_str!("../src/brain_policy.rs");
    let app_error_source = include_str!("../src/app_shell.rs");
    assert!(app_error_source.contains("NeuralBackendUnavailable"));
    assert!(policy_source.contains("NeuralBackendUnavailable"));
    for forbidden in [
        "AutoWithCpuFallback",
        "fallback_to_cpu",
        "fallback_to_heuristic",
        "unwrap_or(PolicyBackend::HeuristicBaseline)",
        "unwrap_or_else(PolicyBackend::HeuristicBaseline)",
    ] {
        assert!(
            !policy_source.contains(forbidden),
            "GPU-required policy contains a fallback branch: {forbidden}"
        );
    }

    let _typed_error_predicate: fn(&GameAppShellError) -> bool = is_neural_unavailable;
}

#[test]
fn gpu_live_runtime_keeps_handle_authority_private_and_reconciles_deaths_before_reuse() {
    let source = include_str!("../src/gpu_live_runtime.rs");

    assert!(source.contains("struct GpuLiveBrainRuntime"));
    assert!(source.contains("handles: BTreeMap<u64, GpuBrainHandle>"));
    assert!(!source.contains("pub handles:"));
    assert!(!source.contains("pub backend:"));
    assert!(source.contains("reconcile_population"));
    assert!(source.contains("organism_id.raw()"));

    let retire = source
        .find("remove_brain(handle)")
        .expect("despawn must retire the generation-checked GPU capability");
    let forget = source[retire..]
        .find("handles.remove")
        .map(|offset| retire + offset)
        .expect("retired organism must be removed from the private handle map");
    assert!(retire < forget);

    assert!(source.contains("#[cfg(test)]"));
    assert!(source.contains("handle_for"));
    assert!(source.contains("test_tick_retired_handle"));
    assert!(source.contains("organism_despawn_retires_its_gpu_handle_before_slot_reuse"));
}

#[test]
fn gpu_live_runtime_does_not_call_the_heuristic_symbolic_controller() {
    let source = include_str!("../src/gpu_live_runtime.rs");

    for forbidden in [
        "CreatureMind",
        "tick_with_proposals",
        "tick_with_proposals_detailed",
        "current_context_proposals",
        "current_context_proposals_with_scores",
    ] {
        assert!(
            !source.contains(forbidden),
            "GPU live runtime still calls forbidden controller path {forbidden}"
        );
    }
}

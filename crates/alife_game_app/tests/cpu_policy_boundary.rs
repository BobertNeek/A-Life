use alife_core::PolicyBackend;
use alife_game_app::{validate_app_shell_config, AppShellLaunchConfig};
#[test]
fn gameplay_shell_rejects_cpu_policy_before_file_or_device_access() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root("/nonexistent-cpu-game-fixture")
        .with_brain_policy(PolicyBackend::NeuralClosedLoopCpu);
    let error = validate_app_shell_config(&launch).unwrap_err();
    assert!(format!("{error}").contains("offline"));
}

#![cfg(feature = "foundation-training")]

use std::{process::Command, time::SystemTime};

#[test]
fn build_info_is_compile_time_read_only_and_separate_from_resident_contract() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "alife-native-build-info-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_train_n2048_care"))
            .args(args)
            .current_dir(&directory)
            .env("VK_ICD_FILENAMES", directory.join("absent-driver.json"))
            .env("PROFILE", "runtime-decoy-profile")
            .env("OPT_LEVEL", "runtime-decoy-opt-level")
            .env("TARGET", "runtime-decoy-target")
            .env("ALIFE_GAME_APP_CARGO_PROFILE", "runtime-decoy-embedded-key")
            .output()
            .unwrap()
    };
    let output = run(&["--build-info"]);
    assert!(output.status.success(), "{:?}", output);
    let info: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(info["kind"], "native-training-build-info-v1");
    assert_eq!(info["schema"], 1);
    assert_eq!(info["package"], "alife_game_app");
    assert_eq!(
        info["cargo_reported_profile"],
        env!("ALIFE_GAME_APP_CARGO_PROFILE")
    );
    assert_eq!(
        info["cargo_reported_opt_level"],
        env!("ALIFE_GAME_APP_CARGO_OPT_LEVEL")
    );
    assert_eq!(
        info["cargo_reported_target"],
        env!("ALIFE_GAME_APP_CARGO_TARGET")
    );
    assert!(info["debug_assertions"].is_boolean());
    assert_eq!(
        info["selected_package_feature_flags"]["foundation-training"],
        true
    );
    assert_eq!(info["selected_package_feature_flags"]["gpu-runtime"], true);

    let rejected = run(&["--build-info", "unused"]);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("unexpected build-info argument"));

    let output = run(&["--resident-contract"]);
    assert!(output.status.success());
    let contract: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(contract["kind"], "native-resident-serial-v1");
    assert_eq!(contract["schema"], 1);
    assert_eq!(contract["same_individual_ppo"], true);
    assert!(contract.get("cargo_reported_profile").is_none());
    assert!(std::fs::read_dir(&directory).unwrap().next().is_none());
    std::fs::remove_dir(directory).unwrap();
}

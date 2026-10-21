#![cfg(feature = "foundation-training")]

use std::{ffi::OsString, path::PathBuf, process::Command, time::SystemTime};

fn rejected(lesson: &str, flags: &[&str], expected: &str) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let absent =
        std::env::temp_dir().join(format!("alife-center-cli-{}-{nonce}", std::process::id()));
    let output: PathBuf = absent.join("output");
    assert!(!absent.exists());
    let mut args = vec![
        OsString::from("--resume-cycle"),
        absent.as_os_str().into(),
        output.as_os_str().into(),
        "16".into(),
        "--lesson".into(),
        lesson.into(),
    ];
    args.extend(flags.iter().map(OsString::from));
    let result = Command::new(env!("CARGO_BIN_EXE_train_n2048_care"))
        .args(args)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains(expected),
        "unexpected CLI rejection: {stderr}"
    );
    // All requests use an absent source, so even a rejected-flag regression
    // cannot reach GPU creation. No failed request may create an output tree.
    assert!(!absent.exists());
}

#[test]
fn centering_cli_rejects_duplicate_and_incompatible_requests_before_gpu_work() {
    rejected(
        "grab_food",
        &["--center-actor-advantages", "--center-actor-advantages"],
        "duplicate actor advantage centering",
    );
    rejected(
        "eat_held_food",
        &["--center-actor-advantages"],
        "actor advantage centering requires",
    );
    rejected(
        "grab_food",
        &[
            "--center-actor-advantages",
            "--acquisition-rehearsal-epochs",
            "1",
        ],
        "actor advantage centering requires",
    );
    rejected(
        "grab_food",
        &[
            "--center-actor-advantages",
            "--archived-success-manifest",
            "absent.json",
            "--archived-success-epochs",
            "1",
        ],
        "actor advantage centering requires",
    );
}

#[test]
fn centering_cli_is_explicitly_admitted_then_requires_a_sealed_source() {
    rejected(
        "grab_food",
        &["--center-actor-advantages", "--sampling-temperature", "32"],
        "actor advantage centering requires a sealed GrabFood continuation",
    );
}

#[test]
fn centering_cli_rejects_mixed_continuation_sources_before_gpu_work() {
    for marker in ["adaptation.json", "warmup.json"] {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let source = std::env::temp_dir().join(format!(
            "alife-center-mixed-cli-{}-{nonce}",
            std::process::id()
        ));
        let output = source.join("output");
        std::fs::create_dir(&source).unwrap();
        // Invalid dummy receipts also keep the test CPU-only if admission regresses.
        std::fs::write(source.join("cycle.json"), b"{}").unwrap();
        std::fs::write(source.join(marker), b"{}").unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_train_n2048_care"))
            .arg("--resume-cycle")
            .arg(&source)
            .arg(&output)
            .args(["16", "--lesson", "grab_food", "--center-actor-advantages"])
            .output()
            .unwrap();
        assert!(!result.status.success());
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains("ambiguous training continuation"),
            "unexpected rejection for {marker}: {stderr}"
        );
        assert!(!output.exists());
        std::fs::remove_file(source.join(marker)).unwrap();
        std::fs::remove_file(source.join("cycle.json")).unwrap();
        std::fs::remove_dir(source).unwrap();
    }
}

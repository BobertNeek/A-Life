#![cfg(feature = "cpu-training")]
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "alife-cpu-care-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn text(p: &Path) -> &str {
    p.to_str().unwrap()
}
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_train_cpu_care"))
        .env("VK_ICD_FILENAMES", "/no-vulkan-icd.json")
        .env("ALIFE_SLM_PRIOR_BACKEND", "deterministic")
        .env_remove("ALIFE_SLM_PRIOR")
        .args(args)
        .output()
        .unwrap()
}
fn success(args: &[&str]) -> Value {
    let result = run(args);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
fn load(p: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn sha(p: impl AsRef<Path>) -> String {
    format!("{:x}", Sha256::digest(fs::read(p).unwrap()))
}
fn pins(source: &Path) -> Value {
    Value::Object(
        [
            "cpu-cycle.json",
            "trained.alife-foundation",
            "actor-checkpoint.json",
            "value-checkpoint.json",
        ]
        .into_iter()
        .map(|n| (n.to_owned(), json!(sha(source.join(n)))))
        .collect(),
    )
}
fn history(path: &Path, source: &Path, lane: &Path, consumed: &[u64]) {
    let ledger = lane.join(".grab-food-seed-reservations.json");
    let imports = if ledger.exists() {
        vec![json!({"path":ledger,"sha256":sha(&ledger)})]
    } else {
        vec![]
    };
    fs::write(path, serde_json::to_vec_pretty(&json!({"schema":1,"kind":"mixed-care-seed-history", "complete":true,
        "evidence":"Explicit synthetic CPU fixture history; no retained real package or historical freshness is qualified",
        "source_sha256":pins(source), "reserved_seed_ranges":[], "consumed_seed_ranges":consumed.iter().map(|s| vec![*s,*s]).collect::<Vec<_>>(),
        "assessment_seeds":[],"imports":imports})).unwrap()).unwrap();
}
fn snapshot(root: &Path) -> Value {
    let mut files = serde_json::Map::new();
    fn walk(root: &Path, path: &Path, files: &mut serde_json::Map<String, Value>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            if path.is_dir() {
                files.insert(name, Value::Null);
                walk(root, &path, files);
            } else {
                files.insert(name, json!(sha(path)));
            }
        }
    }
    walk(root, root, &mut files);
    Value::Object(files)
}

#[test]
fn care_fixtures_prepare_native_lessons_and_capture_exact_decision_bootstrap_dose() {
    let fixture = Fixture::new();
    for (index, lesson) in ["eat_held_food", "eat_reachable_food", "feeding"]
        .into_iter()
        .enumerate()
    {
        let output = fixture.0.join(lesson);
        let seed = (891001 + index).to_string();
        let receipt = success(&[
            "--fixture",
            "--output",
            text(&output),
            "--seed",
            &seed,
            "--lesson",
            lesson,
            "--decisions",
            "2",
        ]);
        assert_eq!(receipt["backend"], "native-cpu");
        success(&["--validate-output", text(&output)]);
        assert_eq!(receipt["fixture"], true);
        assert_eq!(receipt["objective_state_reset"], false);
        assert_eq!(receipt["training_ticks"], 2);
        assert_eq!(receipt["captured_replay_rows"], 3);
        assert_eq!(receipt["bootstrap_replay_rows"], 1);
        assert_eq!(receipt["episode"]["lesson"], lesson);
        assert_eq!(receipt["episode"]["episode_body"], "new-prepared-cohort");
        assert_eq!(receipt["terminal_gpu_individual_restore_proven"], false);
        let episode = &receipt["episode"];
        let traces = load(output.join("care-trace.json"));
        let rows = traces["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.iter().filter(|r| r["loss_row"] == true).count(), 2);
        assert_eq!(
            episode["trained_nourished_meals"].as_u64().unwrap() as usize,
            rows.iter()
                .filter(|r| r["loss_row"] == true && r["nourished_meal"] == true)
                .count()
        );
        assert_eq!(episode["semantic_prior"]["decision_frames"], 3);
        assert_eq!(episode["semantic_prior"]["backend"], "deterministic");
        assert_eq!(receipt["collection_start_tick"], episode["setup_tick"]);
        for row in rows {
            assert!(
                row["decision_tick"].as_u64().unwrap() >= episode["setup_tick"].as_u64().unwrap()
            );
            assert_eq!(
                row["outcome_tick"].as_u64(),
                row["decision_tick"].as_u64().map(|t| t + 1)
            );
        }
        if lesson == "feeding" {
            assert_eq!(episode["teacher_cue_tokens"], json!([]));
            assert_eq!(episode["teacher_cue_frames"], 0);
        } else {
            assert_eq!(episode["teacher_cue_tokens"], json!([8, 1]));
            assert!(episode["teacher_cue_frames"].as_u64().unwrap() > 0);
            let setup = &episode[if lesson == "eat_held_food" {
                "held_food_setup"
            } else {
                "reachable_food_setup"
            }];
            assert!(setup["initial_hunger"].as_f64().unwrap() >= 0.12);
            assert!(setup["initial_energy"].as_f64().unwrap() <= 0.9);
            assert!(episode["setup_tick"].as_u64().unwrap() > 0);
        }
        for (name, hash) in receipt["episode_artifact_sha256"].as_object().unwrap() {
            assert_eq!(hash, &json!(sha(output.join(name))));
        }
        let replay = load(output.join("care-replay.json"));
        for rows in replay["rows"].as_object().unwrap().values() {
            for row in rows.as_array().unwrap() {
                let row: alife_training::cpu::CpuCollectedDecision =
                    serde_json::from_value(row.clone()).unwrap();
                assert_eq!(
                    row.patch.decision().policy_backend(),
                    alife_core::PolicyBackend::NeuralClosedLoopCpu
                );
                row.action.validate(1.).unwrap();
            }
        }
        eprintln!("CPU care fixture: lesson={lesson}, decisions={}, rows={}, collection_s={}, update_s={}", receipt["canonical_valid_decisions"], receipt["training_rows"],receipt["collection_seconds"],receipt["update_seconds"]);
    }
    let rejected = fixture.0.join("unsupported");
    let result = run(&[
        "--fixture",
        "--output",
        text(&rejected),
        "--seed",
        "891099",
        "--lesson",
        "obstacle_navigation",
        "--decisions",
        "2",
    ]);
    assert!(!result.status.success());
    assert!(!rejected.exists());
}

#[test]
fn care_continuation_preserves_full_native_state_and_seed_source_isolation() {
    let fixture = Fixture::new();
    let source = fixture.0.join("source");
    let first = success(&[
        "--fixture",
        "--output",
        text(&source),
        "--seed",
        "893001",
        "--lesson",
        "eat_held_food",
        "--decisions",
        "2",
    ]);
    let before = snapshot(&source);
    let lane = fixture.0.join("cpu-lane");
    fs::create_dir(&lane).unwrap();
    let history_path = fixture.0.join("history.json");
    history(&history_path, &source, &lane, &[893001]);
    let output = lane.join("episode-0");
    let second = success(&[
        "--previous",
        text(&source),
        "--output",
        text(&output),
        "--seed-history",
        text(&history_path),
        "--seed",
        "893002",
        "--lesson",
        "eat_reachable_food",
        "--decisions",
        "2",
    ]);
    assert_eq!(before, snapshot(&source));
    assert_eq!(
        load(output.join("collection-actor-checkpoint.json")),
        load(source.join("actor-checkpoint.json"))
    );
    assert_eq!(
        load(output.join("initial-value-checkpoint.json")),
        load(source.join("value-checkpoint.json"))
    );
    assert_eq!(second["source_policy_version"], first["policy_version"]);
    assert_eq!(
        second["initial_actor_optimizer_step"],
        first["actor_optimizer_step"]
    );
    assert_eq!(
        second["initial_value_optimizer_step"],
        first["value_optimizer_step"]
    );
    assert_eq!(second["fixture"], true);
    assert!(second["seed_provenance"]
        .get("native_source_receipt")
        .is_none());
    assert_eq!(
        second["seed_provenance"]["native_source_receipt_ref"],
        json!(sha(source.join("cpu-cycle.json")))
    );
    let blob = output
        .join("source-lineage")
        .join(format!("{}.json", sha(source.join("cpu-cycle.json"))));
    assert_eq!(
        fs::read(blob).unwrap(),
        fs::read(source.join("cpu-cycle.json")).unwrap()
    );
    assert_eq!(
        second["seed_admission"]["historical_freshness"],
        "operator-attested-complete"
    );
    success(&["--validate-output", text(&output)]);
    let ledger_path = lane.join(".grab-food-seed-reservations.json");
    let ledger = load(&ledger_path);
    assert_eq!(ledger["cpu_care_attempts"][0]["status"], "accepted");
    assert!(!lane.join(".eat-held-food-owner.lock").exists());
    // A pending seal is admitted only against its immutable reservation fields.
    let mut pending = ledger.clone();
    pending["cpu_care_attempts"][0]["status"] = json!("pending");
    pending["cpu_care_attempts"][0]["lesson"] = json!("feeding");
    fs::write(&ledger_path, serde_json::to_vec(&pending).unwrap()).unwrap();
    assert!(!run(&["--reconcile-pending", text(&lane)]).status.success());
    assert_eq!(
        load(&ledger_path)["cpu_care_attempts"][0]["status"],
        "pending"
    );
    pending["cpu_care_attempts"][0]["lesson"] = json!("eat_reachable_food");
    fs::write(&ledger_path, serde_json::to_vec(&pending).unwrap()).unwrap();
    assert_eq!(
        success(&["--reconcile-pending", text(&lane)])["status"],
        "accepted"
    );
    // Reproduce the legacy native reader failure after seal, then reconcile the
    // exact complete optimizer/archive bytes without redispatching the episode.
    let models_before = [
        "actor-checkpoint.json",
        "value-checkpoint.json",
        "trained.alife-foundation",
        "training-handoff.json",
    ]
    .map(|name| (name, fs::read(output.join(name)).unwrap()));
    let mut legacy = load(output.join("cpu-cycle.json"));
    let mut deep = json!(0);
    for _ in 0..128 {
        deep = json!({"legacy":deep});
    }
    legacy["legacy_depth_regression"] = deep;
    let legacy_bytes = serde_json::to_vec_pretty(&legacy).unwrap();
    assert!(serde_json::from_slice::<Value>(&legacy_bytes).is_err());
    fs::write(output.join("cpu-cycle.json"), &legacy_bytes).unwrap();
    let mut reservation = load(&ledger_path);
    reservation["cpu_care_attempts"][0]["status"] = json!("pending");
    reservation["cpu_care_attempts"][0]
        .as_object_mut()
        .unwrap()
        .remove("native_output_sha256");
    reservation["cpu_care_attempts"][0]
        .as_object_mut()
        .unwrap()
        .remove("result");
    fs::write(&ledger_path, serde_json::to_vec(&reservation).unwrap()).unwrap();
    let before_reconcile = snapshot(&output);
    assert_eq!(
        success(&["--reconcile-pending", text(&lane)])["status"],
        "accepted"
    );
    assert_eq!(before_reconcile, snapshot(&output));
    assert!(run(&["--validate-output", text(&output)]).status.success());
    for (name, bytes) in models_before {
        assert_eq!(bytes, fs::read(output.join(name)).unwrap());
    }
    // Full private archive loss or a changed retained moment invalidates a seal.
    for name in [
        "collector-checkpoint.json",
        "collection-actor-checkpoint.json",
        "source-native/actor-checkpoint.json",
    ] {
        let path = output.join(name);
        let original = fs::read(&path).unwrap();
        fs::write(&path, b"{}").unwrap();
        assert!(
            !run(&["--validate-output", text(&output)]).status.success(),
            "{name}"
        );
        fs::write(path, original).unwrap();
    }
    let mut interrupted = load(&ledger_path);
    interrupted["training_seed_ranges"]
        .as_array_mut()
        .unwrap()
        .push(json!([893099, 893099]));
    interrupted["cpu_care_attempts"]
        .as_array_mut()
        .unwrap()
        .push(json!({"status":"pending", "seed":893099,
        "output":lane.join("interrupted"), "source_sha256":pins(&output)}));
    fs::write(&ledger_path, serde_json::to_vec(&interrupted).unwrap()).unwrap();
    let reconciled = success(&["--reconcile-pending", text(&lane)]);
    assert_eq!(reconciled["status"], "failed");
    assert_eq!(reconciled["dispatch"], false);
    assert!(!lane.join("interrupted").exists());
    assert_eq!(
        load(&ledger_path)["training_seed_ranges"],
        json!([[893002, 893002], [893099, 893099]])
    );
    let rejected = lane.join("stale-source");
    history(&history_path, &source, &lane, &[893001, 893002]);
    let args = [
        "--previous",
        text(&source),
        "--output",
        text(&rejected),
        "--seed-history",
        text(&history_path),
        "--seed",
        "893003",
        "--lesson",
        "feeding",
        "--decisions",
        "2",
    ];
    let result = run(&args);
    assert!(!result.status.success());
    assert!(!rejected.exists());
    assert!(String::from_utf8_lossy(&result.stderr).contains("latest accepted"));
    let blocked = source.join("nested/new-output");
    let result = run(&[
        "--previous",
        text(&source),
        "--output",
        text(&blocked),
        "--seed-history",
        text(&history_path),
        "--seed",
        "893004",
        "--lesson",
        "feeding",
        "--decisions",
        "2",
    ]);
    assert!(!result.status.success());
    assert!(!source.join("nested").exists());
    assert_eq!(before, snapshot(&source));
    let existing = fixture.0.join("existing");
    fs::create_dir(&existing).unwrap();
    fs::write(existing.join("keep"), b"owner").unwrap();
    let result = run(&[
        "--previous",
        text(&source),
        "--output",
        text(&existing),
        "--seed-history",
        text(&history_path),
        "--seed",
        "893004",
        "--lesson",
        "feeding",
        "--decisions",
        "2",
    ]);
    assert!(!result.status.success());
    assert_eq!(fs::read(existing.join("keep")).unwrap(), b"owner");
    // Incomplete history remains explicitly unknown across a real CPU successor.
    history(&history_path, &output, &lane, &[893001, 893002, 893099]);
    let evidence = fixture.0.join("immutable-recovery-evidence.json");
    fs::copy(&history_path, &evidence).unwrap();
    let mut known = load(&history_path);
    known["complete"] = json!(false);
    known["seed_admission"] = json!({"policy":"known-exclusions","historical_freshness":"unknown"});
    known["missing_history"] =
        json!("Synthetic fixture with explicitly incomplete historical evidence");
    known["recovery_reconciliation"] = json!({"accepted_checkpoint_authoritative":true,
        "known_inflight_attempts_reconciled_or_excluded":true,"no_known_detached_later_accepted_chain":true,
        "explanation":"Fixture pending attempt reconciled; latest CPU founder is authoritative",
        "evidence":[{"path":evidence,"sha256":sha(&evidence)}],
        "prior_ledger_runs_sha256":format!("{:x}", Sha256::digest(b"[]"))});
    fs::write(&history_path, serde_json::to_vec(&known).unwrap()).unwrap();
    let third_output = lane.join("episode-1");
    let third = success(&[
        "--previous",
        text(&output),
        "--output",
        text(&third_output),
        "--seed-history",
        text(&history_path),
        "--known-exclusions",
        "--seed",
        "893003",
        "--lesson",
        "feeding",
        "--decisions",
        "2",
    ]);
    assert_eq!(third["seed_admission"]["historical_freshness"], "unknown");
    assert_eq!(third["seed_admission"]["policy"], "known-exclusions");
    let lineage = alife_training::native_receipt::Lineage::load(&third, &third_output).unwrap();
    assert_eq!(lineage.frames.len(), 3);
    assert_eq!(lineage.frames[0], third);
    assert_eq!(lineage.frames[1], legacy);
    assert_eq!(lineage.frames[2], first);
    for predecessor in [&output, &source] {
        let blob = third_output
            .join("source-lineage")
            .join(format!("{}.json", sha(predecessor.join("cpu-cycle.json"))));
        assert_eq!(
            fs::read(blob).unwrap(),
            fs::read(predecessor.join("cpu-cycle.json")).unwrap()
        );
    }
    success(&["--validate-output", text(&third_output)]);
}

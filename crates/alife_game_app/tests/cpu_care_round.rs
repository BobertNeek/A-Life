#![cfg(feature = "cpu-training")]
//! Real care collection over disposable native CPU fixtures. These are transport,
//! recovery and optimizer-boundary tests, not evidence of acquired competence.

use alife_core::{ActionCommand, PhysicalContactKind, PolicyBackend, PostActionOutcome, Validate};
use alife_training::cpu::{read_cpu_collector_output, read_cpu_cooperative_round};
use alife_training::PpoJointAction;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const SOURCE_SEED: u64 = 9_201_100;
const HELD_SEED: u64 = SOURCE_SEED + 1;
const REACHABLE_SEED: u64 = SOURCE_SEED + 2;
const EXCLUDED_SEED: u64 = SOURCE_SEED + 99;
const LEDGER: &str = ".grab-food-seed-reservations.json";
const COLLECTORS: [(&str, &str, u64); 2] = [
    ("00-held", "eat_held_food", HELD_SEED),
    ("01-reachable", "eat_reachable_food", REACHABLE_SEED),
];

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "alife-cpu-care-round-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("source-process")).unwrap();
        fs::create_dir(path.join("round-process")).unwrap();
        Self(path)
    }
    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}
fn write_json(path: impl AsRef<Path>, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn load(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn sha(path: impl AsRef<Path>) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
fn tree(root: &Path) -> BTreeMap<PathBuf, String> {
    fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, String>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(path.strip_prefix(root).unwrap().to_owned(), sha(&path));
            }
        }
    }
    let mut files = BTreeMap::new();
    if root.exists() {
        visit(root, root, &mut files);
    }
    files
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
        .map(|name| (name.into(), json!(sha(source.join(name)))))
        .collect(),
    )
}
fn successful(output: Output) -> Value {
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn round_cli(fixture: &Fixture, args: &[&str], off: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_train_cpu_care_round"));
    command
        .current_dir(fixture.join("round-process"))
        .env("VK_ICD_FILENAMES", "/no-vulkan-icd.json")
        .env("ALIFE_SLM_PRIOR_BACKEND", "local")
        .env(
            "ALIFE_SLM_PRIOR_MODEL",
            fixture.join("absent-hostile-model.gguf"),
        )
        .env("ALIFE_SLM_PRIOR_MODEL_SHA256", "0".repeat(64));
    if off {
        command.env("ALIFE_SLM_PRIOR", "off");
    } else {
        command.env_remove("ALIFE_SLM_PRIOR");
    }
    command.args(args).output().unwrap()
}
fn assert_no_shared_cache(fixture: &Fixture) {
    assert!(!fixture.join("round-process/target/slm-prior").exists());
    assert!(!fixture.join("absent-hostile-model.gguf").exists());
}

struct NativeFixture {
    fixture: Fixture,
    source: PathBuf,
    source_receipt: Value,
    history: PathBuf,
    imported_ledger: PathBuf,
}
impl NativeFixture {
    fn new() -> Self {
        let fixture = Fixture::new();
        let source = fixture.join("explicit-synthetic-source");
        let source_receipt = successful(
            Command::new(env!("CARGO_BIN_EXE_train_cpu_care"))
                .current_dir(fixture.join("source-process"))
                .env("VK_ICD_FILENAMES", "/no-vulkan-icd.json")
                .env("ALIFE_SLM_PRIOR_BACKEND", "deterministic")
                .env_remove("ALIFE_SLM_PRIOR")
                .args([
                    "--fixture",
                    "--output",
                    text(&source),
                    "--seed",
                    &SOURCE_SEED.to_string(),
                    "--lesson",
                    "eat_held_food",
                    "--decisions",
                    "2",
                ])
                .output()
                .unwrap(),
        );
        let imported_ledger = fixture.join("original-imported-ledger.json");
        write_json(
            &imported_ledger,
            &json!({"schema":1,"kind":"grab-food-seed-ledger","cpu_care_schema":1,
                "training_seed_ranges":[[EXCLUDED_SEED,EXCLUDED_SEED]],
                "assessment_seeds":[],"allocations":[],"cpu_care_attempts":[]}),
        );
        let history = fixture.join("original-history.json");
        write_json(
            &history,
            &json!({"schema":1,"kind":"mixed-care-seed-history","complete":true,
                "evidence":"Explicit synthetic CPU fixture history; no accepted real lineage or historical freshness is qualified",
                "source_sha256":pins(&source),"reserved_seed_ranges":[],
                "consumed_seed_ranges":[[SOURCE_SEED,SOURCE_SEED]],"assessment_seeds":[],
                "imports":[{"path":imported_ledger,"sha256":sha(&imported_ledger)}]}),
        );
        Self {
            fixture,
            source,
            source_receipt,
            history,
            imported_ledger,
        }
    }
    fn spec(&self, output: &Path) -> Value {
        json!({"schema_version":1,"previous":self.source,"seed_history":self.history,
        "output":output,"known_exclusions":false,"temperature":1.0,"prior":"deterministic_v1",
        "window_ticks":2,"burn_in":1,"effective_batch_size":4,"record_diagnostic_replay":false,
        "collectors":[
            {"collector_run_id":"01-reachable","seed":REACHABLE_SEED,"lesson":"eat_reachable_food","decisions":2},
            {"collector_run_id":"00-held","seed":HELD_SEED,"lesson":"eat_held_food","decisions":2}
        ]})
    }
    fn prepare(&self, label: &str, spec: &Value) -> PathBuf {
        let spec_path = self.fixture.join(&format!("{label}-spec.json"));
        let plan_path = self.fixture.join(&format!("{label}-sealed-plan.json"));
        write_json(&spec_path, spec);
        successful(round_cli(
            &self.fixture,
            &[
                "plan",
                "--spec",
                text(&spec_path),
                "--output",
                text(&plan_path),
            ],
            false,
        ));
        plan_path
    }
    fn protected(&self) -> (BTreeMap<PathBuf, String>, Vec<u8>, Vec<u8>) {
        (
            tree(&self.source),
            fs::read(&self.history).unwrap(),
            fs::read(&self.imported_ledger).unwrap(),
        )
    }
    fn assert_protected(&self, before: &(BTreeMap<PathBuf, String>, Vec<u8>, Vec<u8>)) {
        assert_eq!(&self.protected(), before);
        assert_no_shared_cache(&self.fixture);
    }
}

fn assert_not_dispatched(root: &Path) {
    for (run, _, _) in COLLECTORS {
        assert!(!root.join("collectors").join(run).exists());
    }
    assert!(!root.join("collectors").join(LEDGER).exists());
    assert!(!root.join("learner").exists());
}

fn assert_same_summary(first: &Value, second: &Value) {
    for key in [
        "source_policy_version",
        "policy_version",
        "actor_optimizer_step",
        "value_optimizer_step",
        "training_rows",
        "windows",
        "completed_epochs",
        "stopped_for_kl",
        "comparison_role",
        "promotion",
    ] {
        assert_eq!(first[key], second[key], "{key}");
    }
}

fn assert_history_snapshots(root: &Path, source: &Path, ledger: &Value) {
    let mut prefix_ledger = ledger.clone();
    prefix_ledger["cpu_care_attempts"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    prefix_ledger["training_seed_ranges"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    let mut histories = 0;
    let mut found_collected_prefix = false;
    for entry in fs::read_dir(root.join("histories")).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_file() {
            continue;
        }
        let document = load(&path);
        if document["kind"] != "mixed-care-seed-history" {
            continue;
        }
        histories += 1;
        assert_eq!(document["source_sha256"], pins(source));
        for imported in document["imports"].as_array().unwrap() {
            let imported_path = Path::new(imported["path"].as_str().unwrap());
            let imported_path = if imported_path.is_absolute() {
                imported_path.to_owned()
            } else {
                path.parent().unwrap().join(imported_path)
            };
            assert_eq!(imported["sha256"], sha(&imported_path));
            let evidence = load(imported_path);
            if evidence["kind"] == "grab-food-seed-ledger"
                && evidence["cpu_care_attempts"]
                    .as_array()
                    .is_some_and(|attempts| attempts.len() == 1)
            {
                assert_eq!(evidence, prefix_ledger);
                found_collected_prefix = true;
            }
        }
    }
    assert!(histories >= 2);
    assert!(
        found_collected_prefix,
        "second collector lacks exact immutable ledger import"
    );
}
fn assert_collection(
    fixture: &NativeFixture,
    directory: &Path,
    run: &str,
    lesson: &str,
    seed: u64,
) {
    for name in [
        "cpu-cycle.json",
        "trained.alife-foundation",
        "training-handoff.json",
        "actor-checkpoint.json",
        "value-checkpoint.json",
        "care-replay.json",
    ] {
        assert!(!directory.join(name).exists(), "collector published {name}");
    }
    for (collected, original) in [
        ("collection-actor-checkpoint.json", "actor-checkpoint.json"),
        ("initial-value-checkpoint.json", "value-checkpoint.json"),
    ] {
        assert_eq!(
            fs::read(directory.join(collected)).unwrap(),
            fs::read(fixture.source.join(original)).unwrap(),
            "collector changed full frozen state: {collected}"
        );
    }
    let collection = load(directory.join("collection-metadata.json"));
    assert_eq!(collection["offline_actor_optimizer_updates"], 0);
    assert_eq!(collection["offline_value_optimizer_updates"], 0);
    let shard = read_cpu_collector_output(directory).unwrap();
    assert_eq!(shard.assignment.collector_run_id, run);
    assert_eq!(shard.assignment.world_seed, seed);
    assert_eq!(shard.assignment.lesson, lesson);
    assert_eq!(shard.trajectories.len(), 1);
    assert_eq!(shard.trajectories[0].decisions.len(), 3);
    let episode = load(directory.join("care-episode.json"));
    assert_eq!(episode["lesson"], lesson);
    assert_eq!(episode["raw_replay_recorded"], false);
    assert_eq!(episode["teacher_cue_tokens"], json!([8, 1]));
    assert_eq!(episode["semantic_prior"]["backend"], "deterministic");
    assert_eq!(
        episode["semantic_prior"]["model"],
        alife_semantic::DETERMINISTIC_PRIOR_ID
    );
    assert_eq!(episode["semantic_prior"]["decision_frames"], 3);
    let trace = load(directory.join("care-trace.json"));
    let rows = trace["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows.iter().filter(|row| row["loss_row"] == true).count(), 2);
    assert_eq!(rows[2]["loss_row"], false);
    let target = episode["food_outcome"]["target"].clone();
    let organism = episode["food_outcome"]["organism"].clone();
    assert_eq!(
        rows[0]["lesson_target_ownership_before_decision"]["consumed"],
        false
    );
    assert_eq!(
        rows[0]["lesson_target_ownership_before_decision"]["owner"],
        if lesson == "eat_held_food" {
            organism.clone()
        } else {
            Value::Null
        }
    );
    let mut acquisitions = 0usize;
    let mut consumptions = 0usize;
    for (index, (trace, decision)) in rows
        .iter()
        .zip(&shard.trajectories[0].decisions)
        .enumerate()
    {
        decision.patch.validate_contract().unwrap();
        assert_eq!(
            decision.patch.decision().policy_backend(),
            PolicyBackend::NeuralClosedLoopCpu
        );
        assert_eq!(trace["lesson_target"], target);
        assert_eq!(
            serde_json::from_value::<ActionCommand>(trace["action"].clone()).unwrap(),
            decision.patch.decision().selected_action
        );
        assert_eq!(
            &serde_json::from_value::<PostActionOutcome>(trace["outcome"].clone()).unwrap(),
            decision.patch.outcome()
        );
        assert_eq!(
            serde_json::from_value::<PpoJointAction>(trace["ppo_action"].clone()).unwrap(),
            decision.action
        );
        assert_eq!(trace["loss_row"], index < 2);
        let before = &trace["lesson_target_ownership_before_decision"];
        let after = &trace["lesson_target_ownership_after_outcome"];
        let outcome = decision.patch.outcome();
        let contacts =
            std::iter::once(&outcome.physical).chain(outcome.joint.iter().flat_map(|joint| {
                joint
                    .channel_outcomes
                    .iter()
                    .map(|channel| &channel.physical)
            }));
        let physical = contacts.collect::<Vec<_>>();
        if before["owner"].is_null()
            && before["consumed"] == false
            && after["owner"] == organism
            && after["consumed"] == false
        {
            acquisitions += 1;
            assert!(physical
                .iter()
                .any(|contact| contact.contact == PhysicalContactKind::Touch
                    && serde_json::to_value(contact.target_entity).unwrap() == target));
        }
        if after["consumed"] == true
            && physical.iter().any(|contact| {
                contact.contact == PhysicalContactKind::Consumed
                    && serde_json::to_value(contact.target_entity).unwrap() == target
            })
        {
            consumptions += 1;
        }
        if trace["nourished_meal"] == true {
            assert_eq!(before["owner"], organism);
            assert_eq!(after["consumed"], true);
            assert!(outcome.measured_physiology.is_some());
        }
    }
    assert_eq!(
        episode["food_outcome"]["ordinary_acquisition_events"],
        acquisitions
    );
    assert_eq!(episode["food_outcome"]["consumption_events"], consumptions);
    assert_eq!(
        episode["trained_nourished_meals"],
        rows[..2]
            .iter()
            .filter(|row| row["nourished_meal"] == true)
            .count()
    );
    assert_eq!(
        episode["captured_nourished_meals"],
        rows.iter()
            .filter(|row| row["nourished_meal"] == true)
            .count()
    );
}

#[test]
fn serial_real_care_round_preserves_frozen_collectors_and_reuses_their_completed_lives() {
    let fixture = NativeFixture::new();
    let actor = load(fixture.source.join("actor-checkpoint.json"));
    let value = load(fixture.source.join("value-checkpoint.json"));
    assert!(actor["optimizer_step"].as_u64().unwrap() > 0);
    assert!(value["optimizer_step"].as_u64().unwrap() > 0);
    assert!(actor["update_ages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|age| age.as_u64().unwrap() > 0));
    for field in ["first_moment", "second_moment"] {
        assert!(
            actor[field]
                .as_array()
                .unwrap()
                .iter()
                .any(|moment| moment.as_f64().unwrap() != 0.),
            "fixture lacks nonzero actor {field}"
        );
    }
    let feature_parameters = value["feature_count"].as_u64().unwrap() as usize + 1;
    assert!(
        value["parameters"].as_array().unwrap()[feature_parameters..]
            .iter()
            .any(|moment| moment.as_f64().unwrap() != 0.)
    );
    let output = fixture.fixture.join("round");
    let before = fixture.protected();
    let plan_path = fixture.prepare("two-lessons", &fixture.spec(&output));
    assert_not_dispatched(&output);
    fixture.assert_protected(&before);
    let result = successful(round_cli(
        &fixture.fixture,
        &["run", "--plan", text(&plan_path)],
        true,
    ));
    assert_eq!(
        result["comparison_role"],
        "serial-collection-parity-control"
    );
    assert_eq!(result["promotion"], "none");
    assert_eq!(result["training_rows"], 4);
    assert_eq!(result["windows"], 2);
    assert_eq!(
        result["source_policy_version"],
        fixture.source_receipt["policy_version"]
    );
    assert_eq!(load(output.join("care-round-plan.json")), load(&plan_path));
    for (run, lesson, seed) in COLLECTORS {
        assert_collection(
            &fixture,
            &output.join("collectors").join(run),
            run,
            lesson,
            seed,
        );
    }
    let ledger_path = output.join("collectors").join(LEDGER);
    let ledger = load(&ledger_path);
    let attempts = ledger["cpu_care_attempts"].as_array().unwrap();
    assert_eq!(attempts.len(), 2);
    for (attempt, (run, _, seed)) in attempts.iter().zip(COLLECTORS) {
        assert_eq!(attempt["status"], "collected");
        assert_eq!(attempt["seed"], seed);
        assert_eq!(attempt["source_sha256"], pins(&fixture.source));
        assert_eq!(attempt["collection"]["assignment"]["collector_run_id"], run);
        assert_eq!(
            attempt["result"]["source_policy_version"],
            fixture.source_receipt["policy_version"]
        );
        assert_eq!(
            attempt["result"]["actor_optimizer_step"],
            fixture.source_receipt["actor_optimizer_step"]
        );
        assert_eq!(
            attempt["result"]["value_optimizer_step"],
            fixture.source_receipt["value_optimizer_step"]
        );
        assert!(attempt.get("native_output_sha256").is_none());
    }
    assert_eq!(
        ledger["training_seed_ranges"],
        json!([[HELD_SEED, HELD_SEED], [REACHABLE_SEED, REACHABLE_SEED]])
    );
    assert_history_snapshots(&output, &fixture.source, &ledger);
    let learned = read_cpu_cooperative_round(&output.join("learner")).unwrap();
    assert_eq!(
        learned.source_handoff.actor,
        serde_json::from_value(load(fixture.source.join("actor-checkpoint.json"))).unwrap()
    );
    assert_eq!(
        learned.source_handoff.value,
        serde_json::from_value(load(fixture.source.join("value-checkpoint.json"))).unwrap()
    );
    let actor_delta =
        learned.output_handoff.actor.optimizer_step - learned.source_handoff.actor.optimizer_step;
    let value_delta =
        learned.output_handoff.value.optimizer_step - learned.source_handoff.value.optimizer_step;
    assert!(actor_delta > 0);
    assert_eq!(actor_delta, value_delta);
    assert_eq!(
        result["actor_optimizer_step"],
        learned.output_handoff.actor.optimizer_step
    );
    assert_eq!(
        result["value_optimizer_step"],
        learned.output_handoff.value.optimizer_step
    );
    assert_eq!(
        result["policy_version"],
        learned.output_handoff.policy_version
    );
    assert_eq!(result["completed_epochs"], learned.update.completed_epochs);
    assert_eq!(result["stopped_for_kl"], learned.update.stopped_for_kl);
    fixture.assert_protected(&before);

    let completed = tree(&output);
    let retried = successful(round_cli(
        &fixture.fixture,
        &["run", "--plan", text(&plan_path)],
        false,
    ));
    assert_same_summary(&retried, &result);
    let inspect = round_cli(
        &fixture.fixture,
        &["inspect", "--result", text(&output.join("learner"))],
        true,
    );
    assert!(inspect.stdout.len() < 4096);
    assert!(!String::from_utf8_lossy(&inspect.stdout).contains("first_moment"));
    let inspected = successful(inspect);
    assert_same_summary(&inspected, &result);
    assert_eq!(tree(&output), completed);

    // Preserve the complete first learner output, then exercise reuse of the
    // already consumed, sealed collector lives without redispatch or reseeding.
    let saved_learner = fixture.fixture.join("preserved-first-learner");
    fs::rename(output.join("learner"), &saved_learner).unwrap();
    let saved_before = tree(&saved_learner);
    let collector_before = tree(&output.join("collectors"));
    let reused = successful(round_cli(
        &fixture.fixture,
        &["run", "--plan", text(&plan_path)],
        true,
    ));
    assert_same_summary(&reused, &result);
    assert_eq!(tree(&output.join("collectors")), collector_before);
    assert_eq!(tree(&saved_learner), saved_before);
    assert_eq!(
        read_cpu_cooperative_round(&output.join("learner"))
            .unwrap()
            .payload_sha256,
        learned.payload_sha256
    );
    fixture.assert_protected(&before);

    // Preserve the complete collector elsewhere, then represent a consumed,
    // failed attempt retaining only its unsealed private recovery prefix.
    let held = output.join("collectors/00-held");
    let saved_collector = fixture.fixture.join("preserved-complete-held-collector");
    fs::rename(&held, &saved_collector).unwrap();
    let saved_collector_before = tree(&saved_collector);
    fs::create_dir(&held).unwrap();
    fs::copy(
        saved_collector.join("collector-checkpoint.json"),
        held.join("collector-checkpoint.json"),
    )
    .unwrap();
    fs::write(
        held.join("interrupted-life.txt"),
        b"synthetic failed private recovery prefix",
    )
    .unwrap();
    let mut failed_ledger = load(&ledger_path);
    failed_ledger["cpu_care_attempts"][0]["status"] = json!("failed");
    failed_ledger["cpu_care_attempts"][0]["result"] =
        json!({"error":"synthetic interrupted unsealed fixture","seed_consumed":true});
    write_json(&ledger_path, &failed_ledger);
    let failed_before = tree(&output);
    assert!(
        !round_cli(&fixture.fixture, &["run", "--plan", text(&plan_path)], true)
            .status
            .success()
    );
    assert_eq!(tree(&output), failed_before);
    assert_eq!(tree(&saved_collector), saved_collector_before);
    assert_eq!(tree(&saved_learner), saved_before);
    fixture.assert_protected(&before);
}

#[test]
fn planning_and_dispatch_reject_excluded_duplicate_unsupported_and_changed_inputs() {
    let fixture = NativeFixture::new();
    let before = fixture.protected();
    for name in [
        "duplicate-seed",
        "consumed-source-seed",
        "imported-exclusion",
        "unsupported-prior",
        "fixture-flag",
    ] {
        let output = fixture.fixture.join(name);
        let mut spec = fixture.spec(&output);
        match name {
            "duplicate-seed" => spec["collectors"][0]["seed"] = json!(HELD_SEED),
            "consumed-source-seed" => spec["collectors"][0]["seed"] = json!(SOURCE_SEED),
            "imported-exclusion" => spec["collectors"][0]["seed"] = json!(EXCLUDED_SEED),
            "unsupported-prior" => spec["prior"] = json!("local"),
            "fixture-flag" => spec["fixture"] = json!(true),
            _ => unreachable!(),
        }
        let spec_path = fixture.fixture.join(&format!("{name}-spec.json"));
        let plan_path = fixture.fixture.join(&format!("{name}-plan.json"));
        write_json(&spec_path, &spec);
        assert!(
            !round_cli(
                &fixture.fixture,
                &[
                    "plan",
                    "--spec",
                    text(&spec_path),
                    "--output",
                    text(&plan_path)
                ],
                true
            )
            .status
            .success(),
            "{name}"
        );
        assert!(!plan_path.exists());
        assert_not_dispatched(&output);
        fixture.assert_protected(&before);
    }
    let output = fixture.fixture.join("stale-inputs");
    let plan_path = fixture.prepare("stale-inputs", &fixture.spec(&output));
    for path in [
        fixture.source.join("phase.txt"),
        fixture.history.clone(),
        fixture.imported_ledger.clone(),
    ] {
        let original = fs::read(&path).unwrap();
        let mut changed = original.clone();
        changed.push(b'\n');
        fs::write(&path, &changed).unwrap();
        assert!(
            !round_cli(
                &fixture.fixture,
                &["run", "--plan", text(&plan_path)],
                false
            )
            .status
            .success(),
            "{}",
            path.display()
        );
        assert_eq!(fs::read(&path).unwrap(), changed);
        assert_not_dispatched(&output);
        fs::write(path, original).unwrap();
        fixture.assert_protected(&before);
    }
    let mut tampered = load(&plan_path);
    tampered["payload_sha256"][0] = json!(255 - tampered["payload_sha256"][0].as_u64().unwrap());
    write_json(&plan_path, &tampered);
    assert!(
        !round_cli(&fixture.fixture, &["run", "--plan", text(&plan_path)], true)
            .status
            .success()
    );
    assert_not_dispatched(&output);
    fixture.assert_protected(&before);
}

#[test]
fn an_existing_unsealed_private_life_is_never_reset_or_redispatched() {
    let fixture = NativeFixture::new();
    let output = fixture.fixture.join("interrupted-round");
    let plan_path = fixture.prepare("interrupted", &fixture.spec(&output));
    fs::create_dir(&output).unwrap();
    fs::copy(&plan_path, output.join("care-round-plan.json")).unwrap();
    let owner = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(output.join(".care-round-owner.lock"))
        .unwrap();
    let locked_before = tree(&output);
    owner.try_lock().unwrap();
    let protected = fixture.protected();
    let locked = round_cli(&fixture.fixture, &["run", "--plan", text(&plan_path)], true);
    assert!(!locked.status.success());
    assert!(String::from_utf8_lossy(&locked.stderr).contains("owner"));
    assert_not_dispatched(&output);
    // Windows byte-range locks also refuse reads through another file handle.
    drop(owner);
    assert_eq!(tree(&output), locked_before);
    fixture.assert_protected(&protected);

    let interrupted = output.join("collectors/00-held");
    fs::create_dir_all(&interrupted).unwrap();
    // Use actual native private checkpoint bytes, not an invented archive shape.
    fs::copy(
        fixture.source.join("collector-checkpoint.json"),
        interrupted.join("collector-checkpoint.json"),
    )
    .unwrap();
    fs::write(
        interrupted.join("interrupted-life.txt"),
        b"synthetic interrupted unsealed collector; retain private recovery",
    )
    .unwrap();
    let private_before = tree(&interrupted);
    let protected = fixture.protected();
    assert!(
        !round_cli(&fixture.fixture, &["run", "--plan", text(&plan_path)], true)
            .status
            .success()
    );
    assert_eq!(tree(&interrupted), private_before);
    assert!(!output.join("collectors/01-reachable").exists());
    assert!(!output.join("collectors").join(LEDGER).exists());
    assert!(!output.join("learner").exists());
    assert!(!interrupted.join("collector-commit.json").exists());
    fixture.assert_protected(&protected);
}

#[test]
fn two_real_care_rounds_continue_exact_founders_and_run_matched_heldout_assessment() {
    let fixture = NativeFixture::new();
    let output = fixture.fixture.join("matched-comparison");
    let recipe = fixture.fixture.join("comparison-spec.json");
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/compare_cpu_care_rounds.py")
        .canonicalize()
        .unwrap();
    let round_binary = Path::new(env!("CARGO_BIN_EXE_train_cpu_care_round"))
        .canonicalize()
        .unwrap();
    let care_binary = Path::new(env!("CARGO_BIN_EXE_train_cpu_care"))
        .canonicalize()
        .unwrap();
    let trained = [HELD_SEED, REACHABLE_SEED, SOURCE_SEED + 3, SOURCE_SEED + 4];
    write_json(
        &recipe,
        &json!({"schema":1,"previous":fixture.source.canonicalize().unwrap(),
        "seed_history":fixture.history.canonicalize().unwrap(),"output":output,
        "care_binary":care_binary,"round_binary":round_binary,"temperature":1.0,
        "rounds":[
            [{"collector_run_id":"00-held","seed":trained[0],"lesson":"eat_held_food","decisions":1},
             {"collector_run_id":"01-reachable","seed":trained[1],"lesson":"eat_reachable_food","decisions":1}],
            [{"collector_run_id":"00-held","seed":trained[2],"lesson":"eat_held_food","decisions":1},
             {"collector_run_id":"01-reachable","seed":trained[3],"lesson":"eat_reachable_food","decisions":1}]],
        "assessments":[{"seed":SOURCE_SEED+20,"lesson":"obstacle_navigation","decisions":2},
                       {"seed":SOURCE_SEED+21,"lesson":"relocated_food","decisions":2}]}),
    );
    let protected = fixture.protected();
    let result = Command::new("python")
        .args(["-B", text(&script), "--spec", text(&recipe), "--run"])
        .env("ALIFE_SLM_PRIOR_BACKEND", "local")
        .env("ALIFE_SLM_PRIOR", "on")
        .current_dir(fixture.fixture.join("round-process"))
        .output()
        .unwrap();
    successful(result);
    let report = load(output.join("comparison-report.json"));
    assert_eq!(report["complete"], true);
    assert_eq!(report["budget_matched"], true);
    assert_eq!(report["promotion"], "none");
    assert_eq!(report["heldout_pairs"].as_array().unwrap().len(), 2);
    for arm in ["cooperative", "serial"] {
        assert_eq!(report["training"][arm]["training_rows"], 4);
        assert_eq!(report["training"][arm]["optimizer_updates"]["actor"], 8);
        assert_eq!(report["training"][arm]["optimizer_updates"]["value"], 8);
        for pair in report["heldout_pairs"].as_array().unwrap() {
            let assessment = &pair[arm]["result"];
            assert_eq!(assessment["teacher_present"], false);
            assert_eq!(assessment["semantic_prior_enabled"], false);
            assert_eq!(assessment["offline_actor_optimizer_updates"], 0);
            assert_eq!(assessment["offline_value_optimizer_updates"], 0);
        }
    }
    let first = output.join("cooperative/founder-0");
    let second = output.join("cooperative/founder-1");
    let round1 = read_cpu_cooperative_round(&output.join("cooperative/round-0/learner")).unwrap();
    let round2 = read_cpu_cooperative_round(&output.join("cooperative/round-1/learner")).unwrap();
    assert_eq!(
        serde_json::to_vec(&round1.output_handoff).unwrap(),
        serde_json::to_vec(&round2.source_handoff).unwrap()
    );
    assert!(
        round2.output_handoff.actor.optimizer_step > round1.output_handoff.actor.optimizer_step
    );
    for collector in ["00-held", "01-reachable"] {
        let private = output
            .join("cooperative/round-1/collectors")
            .join(collector);
        assert_eq!(
            fs::read(private.join("collection-actor-checkpoint.json")).unwrap(),
            fs::read(first.join("actor-checkpoint.json")).unwrap()
        );
        assert_eq!(
            fs::read(private.join("initial-value-checkpoint.json")).unwrap(),
            fs::read(first.join("value-checkpoint.json")).unwrap()
        );
        assert_eq!(
            fs::read(private.join("collection-policy.alife-foundation")).unwrap(),
            fs::read(first.join("trained.alife-foundation")).unwrap()
        );
    }
    let first_before = tree(&first);
    successful(round_cli(
        &fixture.fixture,
        &[
            "export-source",
            "--source",
            text(&fixture.source),
            "--result",
            text(&output.join("cooperative/round-0/learner")),
            "--output",
            text(&first),
        ],
        true,
    ));
    assert_eq!(tree(&first), first_before);
    fixture.assert_protected(&protected);

    // The ancestry alone must exclude every member, even if an operator's new
    // history omits the just-disclosed collector seeds.
    let mut history = load(&fixture.history);
    history["source_sha256"] = pins(&second);
    history["assessment_seeds"] = json!([SOURCE_SEED + 20, SOURCE_SEED + 21, SOURCE_SEED + 22]);
    let history_path = fixture.fixture.join("continued-history.json");
    write_json(&history_path, &history);
    let source_before = tree(&second);
    for seed in [trained.as_slice(), &[SOURCE_SEED + 20]].concat() {
        let rejected = fixture.fixture.join(&format!("rejected-seed-{seed}"));
        let mut spec = fixture.spec(&rejected);
        spec["previous"] = json!(second);
        spec["seed_history"] = json!(history_path);
        spec["collectors"] = json!([{"collector_run_id":"retry","seed":seed,"lesson":"eat_held_food","decisions":1}]);
        let spec_path = fixture.fixture.join(&format!("rejected-{seed}.json"));
        let plan_path = fixture.fixture.join(&format!("rejected-{seed}-plan.json"));
        write_json(&spec_path, &spec);
        assert!(!round_cli(
            &fixture.fixture,
            &[
                "plan",
                "--spec",
                text(&spec_path),
                "--output",
                text(&plan_path)
            ],
            true
        )
        .status
        .success());
        assert!(!rejected.exists() && !plan_path.exists());
    }
    let prior_handoff = second.join("cooperative-source-handoff.json");
    let original = fs::read(&prior_handoff).unwrap();
    let mut changed = original.clone();
    changed.push(b'\n');
    fs::write(&prior_handoff, &changed).unwrap();
    let eval_path = fixture.fixture.join("continued-evaluation-spec.json");
    let eval_output = fixture.fixture.join("continued-evaluation");
    write_json(
        &eval_path,
        &json!({"previous":second,"seed_history":history_path,"output":eval_output,
        "seed":SOURCE_SEED+22,"decisions":1,"temperature":1.0,"lesson":"eat_reachable_food"}),
    );
    assert!(!round_cli(
        &fixture.fixture,
        &["evaluate", "--spec", text(&eval_path)],
        true
    )
    .status
    .success());
    assert!(!eval_output.exists());
    fs::write(&prior_handoff, original).unwrap();
    assert_eq!(tree(&second), source_before);
    // Retire paths without destroying any archived private life. Admission of
    // the next fresh life must use its own portable proofs and ancestry.
    fs::rename(&first, fixture.fixture.join("preserved-first-founder")).unwrap();
    fs::rename(
        output.join("cooperative/round-0"),
        fixture.fixture.join("preserved-first-round"),
    )
    .unwrap();
    fs::rename(
        &fixture.source,
        fixture.fixture.join("preserved-initial-source"),
    )
    .unwrap();
    let assessment = successful(round_cli(
        &fixture.fixture,
        &["evaluate", "--spec", text(&eval_path)],
        true,
    ));
    assert_eq!(assessment["offline_actor_optimizer_updates"], 0);
    assert_eq!(assessment["actor_checkpoint_unchanged"], true);
    assert_eq!(assessment["teacher_present"], false);
    assert_eq!(tree(&second), source_before);
    let serial_output = fixture
        .fixture
        .join("ordinary-child-of-cooperative-founder");
    let child = successful(
        Command::new(&care_binary)
            .args([
                "--previous",
                text(&second),
                "--output",
                text(&serial_output),
                "--seed-history",
                text(&history_path),
                "--seed",
                &(SOURCE_SEED + 23).to_string(),
                "--lesson",
                "eat_reachable_food",
                "--decisions",
                "1",
            ])
            .env("ALIFE_SLM_PRIOR_BACKEND", "deterministic")
            .env("ALIFE_SLM_PRIOR", "on")
            .current_dir(fixture.fixture.join("source-process"))
            .output()
            .unwrap(),
    );
    assert_eq!(
        child["initial_actor_optimizer_step"],
        round2.output_handoff.actor.optimizer_step
    );
    assert_eq!(
        child["initial_value_optimizer_step"],
        round2.output_handoff.value.optimizer_step
    );
    assert_eq!(child["actor_optimizer_updates"], 2);
    assert_eq!(child["value_optimizer_updates"], 2);
    successful(
        Command::new(&care_binary)
            .args(["--validate-output", text(&serial_output)])
            .output()
            .unwrap(),
    );
    assert_eq!(tree(&second), source_before);
}

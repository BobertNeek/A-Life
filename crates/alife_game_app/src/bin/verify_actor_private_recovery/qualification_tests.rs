//! Isolated public fixtures only; no owner recovery inputs are accessed.
use super::*;
use std::{
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
};

#[test]
#[ignore = "process-wide barrier lifecycle: run in isolation with --test-threads=1"]
fn scope_ownership_cleanup_and_joined_workers() -> Result<()> {
    use load_only::{Operation, Scope};
    for kind in [
        Operation::Decision,
        Operation::NeuralForward,
        Operation::WorldAdvance,
        Operation::WorldAction,
        Operation::Credit,
        Operation::ActorAdam,
        Operation::CriticAdam,
    ] {
        assert!(load_only::permitted(kind));
    }
    let scope = Scope::enter()?;
    assert!(Scope::enter().is_err());
    let worker = std::thread::spawn(|| {
        assert!(Scope::enter().is_err());
        for kind in [
            Operation::Decision,
            Operation::NeuralForward,
            Operation::WorldAdvance,
            Operation::WorldAction,
            Operation::Credit,
            Operation::ActorAdam,
            Operation::CriticAdam,
        ] {
            assert!(!load_only::permitted(kind));
        }
    });
    // Quiescence is a caller obligation. Join before snapshot and drop.
    worker.join().unwrap();
    assert_eq!(scope.attempts(), [1; 7]);
    drop(scope);
    let early_error = || -> Result<()> {
        let scope = Scope::enter()?;
        assert_eq!(scope.attempts(), [0; 7]);
        assert!(!load_only::permitted(Operation::Credit));
        Err("intentional fixture error".into())
    };
    assert!(early_error().is_err());
    let scope = Scope::enter()?;
    assert_eq!(scope.attempts(), [0; 7]);
    drop(scope);
    assert!(std::panic::catch_unwind(|| {
        let scope = Scope::enter().unwrap();
        assert_eq!(scope.attempts(), [0; 7]);
        assert!(!load_only::permitted(Operation::ActorAdam));
        panic!("intentional scope-unwind fixture");
    })
    .is_err());
    let scope = Scope::enter()?;
    assert_eq!(scope.attempts(), [0; 7]);
    drop(scope);
    assert!(load_only::permitted(Operation::Decision));
    Ok(())
}

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

#[test]
#[ignore = "one public fixture decision and detached reconstruction; process-wide scope, run in isolation"]
fn native_witnessed_credit_refuses_without_changing_continuation() -> Result<()> {
    use alife_core::*;
    use alife_training::cpu::{CpuDecisionBinding, CpuOrganismState};
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| -> std::result::Result<(), String> {
            let run = || -> Result<()> {
                let (p, asset, cp, private, _) = super::tests::fixture()?;
                let actor = admit_actor(&p, &asset, cp)?;
                let mut host = CpuCanonicalHost::from_checkpoint(private.clone(), &actor)?;
                let record = host
                    .world()
                    .organism_registry()
                    .iter()
                    .next()
                    .ok_or("no public founder")?
                    .clone();
                let receptors = record
                    .biochemistry()
                    .neural_receptor_frame(record.phenotype())?;
                let effects = NeuralReceptorEffects::from_frame(
                    &receptors,
                    &NeuralReceptorPhenotype::compile(&p)?,
                )?;
                let rows = host.step(&actor, 1.0)?.decisions;
                assert_eq!(rows.len(), 1);
                let row = &rows[0];
                let patch = &row.patch;
                let credit = OutcomeCreditPacket::from_sealed_patch(patch)?;
                let mut state =
                    CpuOrganismState::from_checkpoint(&p, private.residents[0].neural.clone())?;
                let output = state.advance(&actor, row.replay_tick.clone(), &effects)?;
                assert_eq!(output.initial, row.initial);
                assert_eq!(output.replay_tick, row.replay_tick);
                let bundle = patch.selected_bundle().ok_or("native bundle missing")?;
                let speech = bundle
                    .channels
                    .iter()
                    .any(|c| c.channel == MotorChannel::Vocal && !c.payload.values.is_empty());
                state.stage_selection(
                    &p,
                    &row.action,
                    CpuDecisionBinding {
                        sequence_id: credit.sequence_id(),
                        frame_digest: credit.frame_digest(),
                        action_id: credit.selected_action(),
                        candidate_feature_digest: credit.candidate_feature_digest(),
                        dispatch_generation: credit.dispatch_generation(),
                        active_activation_side: credit.active_activation_side(),
                        joint_selection: JointActionSelectionV1::new(
                            row.action.motor_candidates.map(|i| i.map_or(0, |i| i + 1)),
                            bundle,
                        )?,
                    },
                    speech,
                )?;
                let development = &patch.pre_action().development_state;
                let parameters = record.phenotype().brain_genome.plasticity_parameters();
                let mut control = state.clone();
                control.apply_credit(&actor, patch, &receptors, development, parameters)?;
                let expected = control.checkpoint()?;
                assert_eq!(expected, host.checkpoint(&actor)?.residents[0].neural);
                let before = serde_json::to_vec(state.neural_state())?;
                let actor_before = serde_json::to_vec(&actor.checkpoint())?;
                let scope = load_only::Scope::enter()?;
                assert!(state
                    .apply_credit(&actor, patch, &receptors, development, parameters)
                    .is_err());
                assert_eq!(scope.attempts(), [0, 0, 0, 0, 1, 0, 0]);
                assert_eq!(serde_json::to_vec(state.neural_state())?, before);
                assert!(state.checkpoint().is_err());
                assert_eq!(serde_json::to_vec(&actor.checkpoint())?, actor_before);
                drop(scope);
                // Successful exact continuation also checks retained pending credit,
                // eligibility, fast/lifetime weights, generation and timing state.
                state.apply_credit(&actor, patch, &receptors, development, parameters)?;
                assert_eq!(state.checkpoint()?, expected);
                Ok(())
            };
            run().map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "credit fixture panicked")?
        .map_err(|s| -> Box<dyn std::error::Error + Send + Sync> { s.into() })
}

struct Fixture {
    root: PathBuf,
    data: PathBuf,
    manifest: serde_json::Value,
}
impl Fixture {
    fn new() -> Result<Self> {
        let (p, asset, actor, host, inputs) = super::tests::fixture()?;
        assert_eq!(p.phenotype_hash(), actor.phenotype_hash);
        let root = std::env::temp_dir().join(format!(
            "alife-load-only-public-fixture-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root)?;
        let data = root.join("data");
        fs::create_dir(&data)?;
        let original = serde_json::to_vec(
            &serde_json::json!({"kind":"disposable-retained-language-pilot-v1","inputs":inputs,"foundation":asset.encode_canonical()?,"actor":actor,"host":host,"next_gradient":[],"assessment":null}),
        )?;
        let checksum = sha(&original).into_bytes();
        let actor_bytes = serde_json::to_vec(&actor)?;
        let host_bytes = serde_json::to_vec(&host)?;
        let mut files = serde_json::Map::new();
        for (id, path, bytes, requires, native) in [
            ("a", "recovery.json", original, vec![], None),
            ("b", "checksum.txt", checksum, vec![], None),
            (
                "c",
                "actor.json",
                actor_bytes,
                vec!["a", "b"],
                Some(identity(&actor)?),
            ),
            (
                "d",
                "private.json",
                host_bytes,
                vec!["a", "b", "c"],
                Some(identity(&host)?),
            ),
        ] {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(data.join(path))?
                .write_all(&bytes)?;
            files.insert(id.into(),serde_json::json!({"root":"data","path":path,"bytes":bytes.len(),"sha256":sha(&bytes),"requires":requires,"native_typed_identity_sha256":native}));
        }
        let manifest = serde_json::json!({"schema":1,"roots":{"data":data},"files":files,
            "profiles":{"four_arm_snapshots":{"files":["a","b","c","d"]},"original_snapshot":{"files":["a","b"]}},
            "original":{"recovery_file":"a","checksum_file":"b","native_actor_identity_sha256":identity(&actor)?,"native_private_identity_sha256":identity(&host)?},
            "actors":{"Untouched":{"actor_file":"c","updates":[]}},"latest_private_end_files":{},"known_positive_private_start":"d","critic":{"present":false}});
        // These are new public fixture files. Leave them available for failure
        // inspection; never copy, edit or remove an owner checkpoint.
        Ok(Self {
            root,
            data,
            manifest,
        })
    }
    fn request(&self, name: &str, m: &serde_json::Value, profile: &str) -> Result<Request> {
        let bytes = serde_json::to_vec(m)?;
        let path = self.root.join(name);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?
            .write_all(&bytes)?;
        Ok(Request {
            manifest: path,
            expected_sha256: sha(&bytes),
            profile: profile.into(),
            require_joint: false,
        })
    }
    fn hashes(&self) -> Result<BTreeMap<PathBuf, String>> {
        fs::read_dir(&self.data)?
            .map(|e| {
                let p = e?.path();
                Ok((p.clone(), sha(&fs::read(p)?)))
            })
            .collect()
    }
}

#[test]
#[ignore = "filesystem native admission: public fixture, process-wide scope, run with --test-threads=1"]
fn full_pipeline_rejects_digest_escape_binding_and_changed_input() -> Result<()> {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| -> std::result::Result<(), String> {
            let run = || -> Result<()> {
                let fixture = Fixture::new()?;
                let before = fixture.hashes()?;
                let valid = inspect_files(fixture.request(
                    "valid.json",
                    &fixture.manifest,
                    "four_arm_snapshots",
                )?)?;
                assert_eq!(valid.actor_admissions.len(), 2);
                assert_eq!(valid.private_admissions.len(), 2);
                assert_eq!(valid.input_files_hashed_before_after, 5);
                assert!(valid.input_hashes_unchanged);
                assert!(valid.forbidden_operation_attempts.values().all(|n| *n == 0));
                assert_eq!(fixture.hashes()?, before);

                let mut wrong =
                    fixture.request("bad-digest.json", &fixture.manifest, "four_arm_snapshots")?;
                wrong.expected_sha256 = "0".repeat(64);
                assert!(inspect_files(wrong)
                    .unwrap_err()
                    .to_string()
                    .contains("manifest SHA256 mismatch"));
                assert_eq!(fixture.hashes()?, before);

                let mut malformed = fixture.manifest.clone();
                malformed["files"]["d"]["requires"] = serde_json::json!(["a", "b"]);
                assert!(inspect_files(fixture.request(
                    "bad-binding.json",
                    &malformed,
                    "four_arm_snapshots"
                )?)
                .unwrap_err()
                .to_string()
                .contains("mismatched actor/asset dependencies"));
                assert_eq!(fixture.hashes()?, before);

                #[cfg(unix)]
                {
                    let outside = fixture.root.join("outside.txt");
                    fs::write(&outside, b"public fixture only")?;
                    let escape = fixture.data.join("escape.json");
                    std::os::unix::fs::symlink(&outside, &escape)?;
                    let mut escaped = fixture.manifest.clone();
                    escaped["files"]["a"]["path"] = serde_json::json!("escape.json");
                    assert!(inspect_files(fixture.request(
                        "escape.json",
                        &escaped,
                        "original_snapshot"
                    )?)
                    .unwrap_err()
                    .to_string()
                    .contains("escaped its declared root"));
                    // The new symlink itself is test setup; original files stay exact.
                    for (path, digest) in &before {
                        assert_eq!(sha(&fs::read(path)?), *digest);
                    }
                }
                let changed = inspect_files_with_after_admission(
                    fixture.request("changed.json", &fixture.manifest, "original_snapshot")?,
                    || {
                        fs::OpenOptions::new()
                            .append(true)
                            .open(fixture.data.join("checksum.txt"))?
                            .write_all(b"changed by fixture between reads")?;
                        Ok(())
                    },
                );
                assert!(changed
                    .unwrap_err()
                    .to_string()
                    .contains("input size/SHA256 mismatch"));
                for (path, digest) in &before {
                    if path.file_name().unwrap() != "checksum.txt" {
                        assert_eq!(sha(&fs::read(path)?), *digest);
                    }
                }
                // Every early-return path releases the process owner without a
                // critic fallback or checkpoint rewrite by the verifier.
                let scope = load_only::Scope::enter()?;
                assert_eq!(scope.attempts(), [0; 7]);
                drop(scope);
                Ok(())
            };
            run().map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "filesystem fixture panicked")?
        .map_err(|s| -> Box<dyn std::error::Error + Send + Sync> { s.into() })
}

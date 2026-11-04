use super::*;
use alife_training::{CurrentLifeCheckpointV1, FoundationTrainingBackendV1};

struct Fixture {
    root: PathBuf,
    source: PathBuf,
    current: PathBuf,
    history: PathBuf,
    founder_seed: u64,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "alife-current-cpu-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let source = root.join("synthetic-native-source");
        let (config, mut episode) = prepare_care_episode(CpuCareRequest {
            previous: None,
            output: source.clone(),
            seed: 0xa510,
            decisions: 2,
            temperature: 1.,
            lesson: FoundationTeacherLesson::EatHeldFood,
            food_need_condition: Default::default(),
            seed_history: None,
            known_exclusions: false,
            record_diagnostic_replay: false,
            fixture: true,
        })
        .unwrap();
        episode.explicit_deterministic_prior = true;
        run_cpu_continuation(config, &mut episode).unwrap();
        let admitted = read_cpu_founder_source(&source).unwrap();
        let founder_seed = admitted.receipt["founder_seed_base"].as_u64().unwrap();
        // Ordinary device-free private save. This is a fresh-cohort Current
        // fixture, not a synthetic claim of a saved GPU individual's identity.
        let world = alife_world::HeadlessScenarioBuilder::new(0xa510)
            .build()
            .unwrap();
        let save = alife_world::PortableSaveFile::from_headless_world(
            "public-disposable-current",
            &world,
            alife_world::RuntimeConfig::deterministic_default(
                world.seed(),
                alife_core::BrainScaleTier::Nano512,
            ),
            alife_world::AssetManifest::empty(),
            vec![],
        )
        .unwrap();
        let private = root.join("fixture-private");
        fs::create_dir(&private).unwrap();
        let assets = private.join("world-assets");
        fs::create_dir(&assets).unwrap();
        crate::GpuDurableSaveManifest::publish_snapshot(private.join("world.json"), &assets, &save)
            .unwrap();
        let (durable, loaded) =
            crate::GpuDurableSaveManifest::open_loaded(private.join("world.json"), &assets)
                .unwrap();
        let current = root.join("current.alife-checkpoint");
        let checkpoint = CurrentTrainingCheckpointV1 {
            schema_version: 1,
            seed: 0xa510,
            founder_seed_base: founder_seed,
            seeds: CheckpointSeedsV1 {
                historical_freshness: "unknown".into(),
                excluded_ranges: vec![[0xa510, 0xa510], [0xa520, 0xa525]],
                pending_range: Some([0xa520, 0xa525]),
            },
            handoff: admitted.handoff,
            life: Some(CurrentLifeCheckpointV1 {
                world_json: serde_json::to_string(&loaded.save).unwrap(),
                sleep_journal_json: serde_json::to_string(
                    &durable.load_sleep_transaction_journal(&loaded).unwrap(),
                )
                .unwrap(),
                assets_base64: Default::default(),
            }),
            same_individual: None,
            terminal_disposition: None,
            training_attempt_origin: None,
        };
        fs::write(&current, checkpoint.encode().unwrap()).unwrap();
        crate::read_current_training_checkpoint(&current).unwrap();
        let history = root.join("current-assessment-history.json");
        write(
            &history,
            &CpuCurrentAssessmentSeeds {
                schema_version: 1,
                kind: "current-cpu-assessment-seeds".into(),
                source_current_sha256: file_hash(&current).unwrap(),
                historical_freshness: "unknown".into(),
                excluded_ranges: checkpoint.seeds.excluded_ranges,
                assessment_seeds: vec![0xa511, 0xa512, 0xa513],
            },
        )
        .unwrap();
        Self {
            root,
            source,
            current,
            history,
            founder_seed,
        }
    }
    fn request(
        &self,
        seed: u64,
        lesson: FoundationTeacherLesson,
        condition: FoundationFoodNeedCondition,
    ) -> CpuCurrentEvaluationRequest {
        CpuCurrentEvaluationRequest {
            current: self.current.clone(),
            expected_current_sha256: file_hash(&self.current).unwrap(),
            seed_history: self.history.clone(),
            output: self.root.join(format!("current-assessment-{seed}")),
            founder_seed: self.founder_seed,
            seed,
            decisions: 4,
            temperature: 1.,
            lesson,
            food_need_condition: condition,
            known_exclusions: true,
        }
    }
    fn native_history(&self) -> PathBuf {
        let pins = [
            "cpu-cycle.json",
            "trained.alife-foundation",
            "actor-checkpoint.json",
            "value-checkpoint.json",
        ]
        .into_iter()
        .map(|n| (n, file_hash(&self.source.join(n)).unwrap()))
        .collect::<BTreeMap<_, _>>();
        let path = self.root.join("synthetic-native-history.json");
        write(
            &path,
            &json!({"schema":1,"kind":"mixed-care-seed-history","complete":true,
            "evidence":"Disposable public fixture only; no real historical freshness claim",
            "source_sha256":pins,"consumed_seed_ranges":[[0xa510,0xa510]],"reserved_seed_ranges":[],
            "assessment_seeds":[0xa511,0xa512,0xa513],"imports":[]}),
        )
        .unwrap();
        path
    }
}

#[test]
fn current_adapter_preserves_real_fixture_heads_private_state_and_matches_native_controls() {
    let f = Fixture::new();
    let original = fs::read(&f.current).unwrap();
    let source = inventory(&f.source).unwrap();
    let before = crate::read_current_training_checkpoint(&f.current).unwrap();
    assert!(before.handoff.actor.optimizer_step > 0);
    let native_history = f.native_history();
    for (seed, lesson, condition) in [
        (
            0xa511,
            FoundationTeacherLesson::EatHeldFood,
            FoundationFoodNeedCondition::NewbornV1,
        ),
        (
            0xa512,
            FoundationTeacherLesson::Feeding,
            FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
        ),
        (
            0xa513,
            FoundationTeacherLesson::RelocatedFood,
            FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
        ),
    ] {
        let request = f.request(seed, lesson, condition);
        let result = evaluate_cpu_current(request.clone()).unwrap();
        assert_eq!(result["same_individual_restore"], false);
        assert_eq!(result["offline_optimizer_constructed"], false);
        let mut full_request = request.clone();
        full_request.output = f.root.join(format!("current-full-{seed}"));
        let full_result = evaluate_cpu_current_full(full_request.clone()).unwrap();
        assert_eq!(full_result["offline_optimizer_constructed"], true);
        assert_eq!(full_result["critic_constructed"], true);
        assert_eq!(
            verify_cpu_current_parity(&request.output, &full_request.output).unwrap()
                ["parity_verified"],
            true
        );
        assert!(verify_cpu_current_parity(&request.output, &request.output).is_err());
        let full_proof: Value = read_json(&full_request.output.join(FULL_PROOF)).unwrap();
        assert_eq!(
            full_proof["source_actor_sha256"],
            hash_value(&before.handoff.actor).unwrap()
        );
        assert_eq!(
            full_proof["source_value_sha256"],
            hash_value(&before.handoff.value).unwrap()
        );
        assert_eq!(
            full_proof["rebound_actor_before_sha256"],
            full_proof["rebound_actor_after_sha256"]
        );
        assert_eq!(full_proof["actor_optimizer_updates"], 0);
        assert_eq!(full_proof["value_optimizer_updates"], 0);
        let full_inventory = inventory(&full_request.output).unwrap();
        assert!(evaluate_cpu_current_full(full_request.clone()).is_err());
        assert_eq!(inventory(&full_request.output).unwrap(), full_inventory);

        let full = f.root.join(format!("native-full-{seed}"));
        evaluate_cpu_care(CpuCareEvaluationRequest {
            previous: f.source.clone(),
            output: full.clone(),
            seed_history: native_history.clone(),
            seed,
            decisions: 4,
            temperature: 1.,
            lesson,
            food_need_condition: condition,
            known_exclusions: false,
        })
        .unwrap();
        assert_eq!(
            fs::read(full.join("care-trace.json")).unwrap(),
            fs::read(request.output.join("care-trace.json")).unwrap()
        );
        assert_eq!(
            read_cpu_private_checkpoint_bytes(&full, u64::MAX).unwrap(),
            read_cpu_private_checkpoint_bytes(&request.output, u64::MAX).unwrap()
        );
        let bare = f.root.join(format!("native-bare-{seed}"));
        evaluate_cpu_founder(CpuFounderEvaluationRequest {
            foundation: f.source.join("trained.alife-foundation"),
            expected_foundation_sha256: file_hash(&f.source.join("trained.alife-foundation"))
                .unwrap(),
            seed_anchor: f.source.clone(),
            seed_history: native_history.clone(),
            output: bare.clone(),
            seed,
            decisions: 4,
            temperature: 1.,
            lesson,
            food_need_condition: condition,
            known_exclusions: false,
        })
        .unwrap();
        assert_eq!(
            compare_cpu_fresh_assessments(&request.output, &bare).unwrap()["matched"],
            true
        );
        assert_eq!(fs::read(&f.current).unwrap(), original);
        assert_eq!(inventory(&f.source).unwrap(), source);
        let retained = inventory(&request.output).unwrap();
        assert!(evaluate_cpu_current(request.clone()).is_err());
        assert_eq!(inventory(&request.output).unwrap(), retained);
        verify_cpu_current_evaluation(&request.output).unwrap();
    }
    let after = crate::read_current_training_checkpoint(&f.current).unwrap();
    assert_eq!(
        serde_json::to_vec(&before.handoff).unwrap(),
        serde_json::to_vec(&after.handoff).unwrap()
    );
    assert_eq!(before.life, after.life);
    assert_eq!(before.seeds, after.seeds);
    let out = f.root.join("current-assessment-42257");
    fs::write(out.join("care-trace.json"), b"tampered disposable trace").unwrap();
    assert!(verify_cpu_current_evaluation(&out).is_err());
}

#[test]
fn current_adapter_rejects_bad_source_dropped_exclusions_collisions_and_incomplete_prefix() {
    let f = Fixture::new();
    let original = fs::read(&f.current).unwrap();
    let request = f.request(0xa511, FoundationTeacherLesson::Feeding, Default::default());
    let mut bad = request.clone();
    bad.expected_current_sha256 = "0".repeat(64);
    assert!(evaluate_cpu_current(bad).is_err());
    assert!(!request.output.exists());
    let current = crate::read_current_training_checkpoint(&f.current).unwrap();
    let history: CpuCurrentAssessmentSeeds = read_json(&f.history).unwrap();
    let mut wrong = history.clone();
    wrong.excluded_ranges = vec![[0xa510, 0xa510]];
    assert!(admit_seeds(&request, &current, &wrong).is_err());
    let mut wrong = history.clone();
    wrong.assessment_seeds.push(0xa520);
    assert!(admit_seeds(&request, &current, &wrong).is_err());
    let mut wrong = history.clone();
    wrong.assessment_seeds.push(0xa511);
    assert!(admit_seeds(&request, &current, &wrong).is_err());
    let mut wrong = request.clone();
    wrong.known_exclusions = false;
    assert!(admit_seeds(&wrong, &current, &history).is_err());
    let mut wrong = request.clone();
    wrong.decisions = 129;
    assert!(evaluate_cpu_current(wrong).is_err());
    // Typed same-individual admission may not be bypassed by clearing its tag.
    let mut wrong = current.clone();
    wrong.handoff.continuation = alife_training::FoundationContinuationKindV1::SameIndividual;
    assert!(wrong.encode().is_err());
    fs::create_dir(&request.output).unwrap();
    fs::write(request.output.join("preserved-prefix"), b"keep").unwrap();
    assert!(evaluate_cpu_current(request.clone()).is_err());
    assert_eq!(
        fs::read(request.output.join("preserved-prefix")).unwrap(),
        b"keep"
    );
    assert_eq!(fs::read(&f.current).unwrap(), original);
    // Explicitly wrong origin is not a route to a native CPU training receipt.
    assert_eq!(
        current.handoff.origin,
        FoundationTrainingBackendV1::NativeCpu
    );
}

#[test]
fn synthetic_current_gpu_origin_is_retained_without_claiming_gpu_execution() {
    let f = Fixture::new();
    let mut checkpoint = crate::read_current_training_checkpoint(&f.current).unwrap();
    // Disposable typed fixture only: this exercises an origin tag, not a GPU run.
    checkpoint.handoff.origin = FoundationTrainingBackendV1::NativeGpu;
    fs::write(&f.current, checkpoint.encode().unwrap()).unwrap();
    let mut history: CpuCurrentAssessmentSeeds = read_json(&f.history).unwrap();
    history.source_current_sha256 = file_hash(&f.current).unwrap();
    fs::write(&f.history, serde_json::to_vec(&history).unwrap()).unwrap();
    let request = f.request(0xa511, FoundationTeacherLesson::Feeding, Default::default());
    let source = admit_current(&request).unwrap();
    assert_eq!(source.proof["origin"], "NativeGpu");
    assert_eq!(source.proof["source_continuation"], "FreshCohort");
    assert_eq!(source.proof["private_life_transferred"], false);
    assert!(!request.output.exists());
}

#[test]
fn current_full_state_parity_needs_no_native_directory_and_rejects_changed_case_or_proof() {
    let f = Fixture::new();
    let original = fs::read(&f.current).unwrap();
    // Disposable fixture only: the self-contained Current is now the sole
    // policy/Adam/private source; no fake CPU source receipt is constructed.
    fs::remove_dir_all(&f.source).unwrap();
    let bare = f.request(
        0xa511,
        FoundationTeacherLesson::Feeding,
        FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
    );
    let mut full = bare.clone();
    full.output = f.root.join("standalone-full");
    evaluate_cpu_current(bare.clone()).unwrap();
    evaluate_cpu_current_full(full.clone()).unwrap();
    assert_eq!(
        verify_cpu_current_parity(&bare.output, &full.output).unwrap()["parity_verified"],
        true
    );
    let mut other = f.request(
        0xa512,
        FoundationTeacherLesson::Feeding,
        FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
    );
    other.output = f.root.join("different-full-case");
    evaluate_cpu_current_full(other.clone()).unwrap();
    assert!(verify_cpu_current_parity(&bare.output, &other.output).is_err());
    fs::write(
        full.output.join(FULL_PROOF),
        b"changed disposable full-state proof",
    )
    .unwrap();
    assert!(verify_cpu_current_evaluation(&full.output).is_err());
    assert_eq!(fs::read(&f.current).unwrap(), original);
}

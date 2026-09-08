//! One acquired-life retention experiment. No inherited weights are trained here.
use super::*;
use alife_core::*;
use alife_gpu_backend::GpuRuntimeProfile;
use alife_world::GpuBrainSaveState;

const ORGANISM: OrganismId = OrganismId(1);

fn save(root: &Path, name: &str, value: &impl serde::Serialize) {
    write_capture(&root.join(name), &serde_json::to_value(value).unwrap());
}

fn retain(
    runtime: &mut GpuLiveBrainRuntime,
    root: &Path,
    assets: &Path,
    label: &str,
) -> Option<PortableSaveFile> {
    // Publish to a fresh path. The live runtime keeps its own durable boundary.
    match runtime
        .capture_portable_checkpoint()
        .and_then(|checkpoint| {
            GpuDurableSaveManifest::publish_snapshot(
                root.join(format!("{label}.json")),
                assets,
                &checkpoint,
            )?;
            Ok(checkpoint)
        }) {
        Ok(checkpoint) => Some(checkpoint),
        Err(error) => {
            save(
                root,
                &format!("{label}-capture-error.json"),
                &format!("{error:?}"),
            );
            None
        }
    }
}

fn brain(save: &PortableSaveFile) -> &GpuBrainSaveState {
    save.creatures
        .iter()
        .find(|c| c.organism_id == ORGANISM)
        .and_then(|c| c.gpu_brain.as_ref())
        .expect("complete organism checkpoint")
}

fn decision_key(patch: &ExperiencePatch) -> serde_json::Value {
    let decision = serde_json::to_value(patch.decision()).unwrap();
    let neural = patch.decision().neural_evidence().unwrap();
    serde_json::json!({
        "family":neural.action_family, "candidate_index":neural.candidate_index,
        "logit_bits":neural.logit.to_bits(), "global_action":decision["selected_action"],
        "motor_bundle":decision["selected_bundle"],
    })
}

#[derive(Default, serde::Serialize)]
struct AwakePhase {
    progressed: u32,
    no_progress: u32,
    both_available: u32,
    nutritious: u32,
    harmful: u32,
    negative_harmful_feedback: u32,
    decisions: Vec<serde_json::Value>,
    error: Option<String>,
}

impl AwakePhase {
    fn preference_passed(&self) -> bool {
        self.error.is_none()
            && self.both_available == 8
            && self.nutritious >= 3
            && self.nutritious > self.harmful
    }
}

fn observe_awake(
    runtime: &mut GpuLiveBrainRuntime,
    phenotype: &BrainPhenotype,
    root: &Path,
    label: &str,
    tick_cap: u32,
    first_expected: Option<&serde_json::Value>,
) -> AwakePhase {
    let started = Instant::now();
    let mut phase = AwakePhase::default();
    while phase.progressed < tick_cap && started.elapsed().as_secs() < 60 {
        let before = runtime.world_snapshot();
        let record = before.organism_registry().get(ORGANISM).unwrap();
        if !record.lifecycle().is_alive() {
            phase.error = Some("organism died".into());
            break;
        }
        let receptors = record
            .biochemistry()
            .neural_receptor_frame(record.phenotype())
            .unwrap();
        let sensed = before.sensory_report(ORGANISM, before.tick()).unwrap();
        let available = [WorldEntityId(2), WorldEntityId(3)].map(|food| {
            !before.entity(food).unwrap().consumed
                && sensed.visible_entities.iter().any(|e| e.id == food)
        });
        let fast_before = runtime.active_fast_weights_for_test(ORGANISM).unwrap();
        match runtime.tick_outcome() {
            Ok(GpuLiveTickOutcome::NoProgress(_)) => {
                phase.no_progress += 1;
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            Ok(GpuLiveTickOutcome::Progressed(summaries)) => {
                phase.progressed += 1;
                if !summaries
                    .iter()
                    .any(|s| s.organism_id == ORGANISM && s.patch_sealed)
                {
                    phase.error = Some("expected awake sealed action".into());
                    break;
                }
            }
            Err(error) => {
                phase.error = Some(format!("{error:?}"));
                break;
            }
        }
        let patch = runtime.sealed_patches().last().unwrap().clone();
        let evidence = patch.decision().neural_evidence().unwrap();
        let credit = OutcomeCreditPacket::from_sealed_patch(&patch)
            .unwrap()
            .with_biochemical_receptors(&receptors)
            .unwrap();
        let lanes = *credit.modulator().frame().lanes();
        let receptor = phenotype.synapses().iter().find_map(|s| match s.kind() {
            CompiledSynapseKind::Decoder(c)
                if c.head() == DecoderHeadKind::ActionCandidate
                    && c.family() == evidence.action_family =>
            {
                Some(&phenotype.plasticity_receptors()[usize::from(s.receptor_index())])
            }
            _ => None,
        });
        let factor = receptor.map(|r| {
            let weights = *r.receptor_profile().weights();
            weights.iter().zip(lanes).map(|(w, l)| w * l).sum::<f32>()
                / weights.iter().map(|w| w.abs()).sum::<f32>()
        });
        let both = available.iter().all(|v| *v);
        phase.both_available += u32::from(both);
        let consumed = patch.outcome().physical.contact == PhysicalContactKind::Consumed;
        let target = patch.outcome().physical.target_entity;
        phase.nutritious += u32::from(both && consumed && target == Some(WorldEntityId(2)));
        phase.harmful += u32::from(both && consumed && target == Some(WorldEntityId(3)));
        phase.negative_harmful_feedback += u32::from(
            consumed
                && target == Some(WorldEntityId(3))
                && patch.outcome().pain_delta.raw() > 0.0
                && factor.is_some_and(|f| f < 0.0),
        );
        let key = decision_key(&patch);
        phase.decisions.push(key.clone());
        save(
            root,
            &format!("{label}-tick-{:02}.json", phase.progressed),
            &serde_json::json!({
                "world_tick":runtime.world_snapshot().tick(), "availability":available,
                "no_progress":phase.no_progress, "patch":patch, "decision_key":key,
                "credit_lanes":lanes, "third_factor":factor,
                "fast_before":fast_before.iter().map(|w|w.to_bits()).collect::<Vec<_>>(),
                "fast_after":runtime.active_fast_weights_for_test(ORGANISM).unwrap().iter().map(|w|w.to_bits()).collect::<Vec<_>>(),
                "learning":runtime.last_learning_receipts().iter().map(|r| serde_json::json!({
                    "sequence":r.sequence_id,"dispatch":r.dispatch_generation,
                    "input_fast":r.input_fast_generation,"output_fast":r.output_fast_generation,
                    "eligibility":r.output_eligibility_generation,"replay":r.replay_journal_generation,
                    "changed":r.fast_weights_changed,"max_abs_delta":r.max_abs_delta,
                })).collect::<Vec<_>>(),
                "memory_updates":runtime.last_memory_update_receipts(),
            }),
        );
        if evidence.phenotype_hash != phenotype.phenotype_hash() {
            phase.error = Some("GPU phenotype identity changed".into());
            break;
        }
        if phase.progressed == 1 && first_expected.is_some_and(|expected| *expected != key) {
            phase.error =
                Some("first restored decision differs from uninterrupted reference".into());
            break;
        }
    }
    if phase.progressed != tick_cap && phase.error.is_none() {
        phase.error = Some("awake phase wall bound exhausted".into());
    }
    save(root, &format!("{label}-observations.json"), &phase);
    phase
}

#[test]
#[ignore = "one retained c=.05 asset, new acquired life, exact restore and one sleep cycle"]
fn scaled_choice_checkpoint_sleep_retention_once() {
    let bytes = std::fs::read(std::env::var("ALIFE_RETENTION_CANDIDATE").unwrap()).unwrap();
    let asset = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    assert_eq!(
        asset.digest().bytes(),
        &[
            162, 205, 184, 109, 162, 15, 136, 4, 200, 120, 232, 83, 105, 194, 153, 231, 150, 181,
            202, 200, 230, 97, 59, 164, 174, 244, 52, 65, 167, 166, 238, 30,
        ]
    );
    let configured = Nano512ActionCreditCandidateV2::new(
        &asset,
        ActionCandidateCreditProfileV1::SignedChoiceReadouts,
    )
    .unwrap();
    assert_eq!(
        configured.asset().unwrap().encode_canonical().unwrap(),
        bytes
    );
    let (phenotype, _) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&configured).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/choice-retention-{}",
        std::process::id(),
    ));
    assert!(!root.exists(), "preserve all earlier evidence");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("candidate.alife-foundation"), bytes).unwrap();
    save(&root, "configured-candidate.json", &configured);
    save(
        &root,
        "bounds.json",
        &serde_json::json!({
            "life":"new measured acquired life; not recovered 32416 state",
            "candidate_digest":asset.digest(), "optimizer_steps":0, "outer_seconds":360,
            "acquisition_ticks":8, "acquisition_seconds":60, "reference_ticks":1,
            "post_load_awake_ticks":8, "post_load_seconds":60,
            "sleep_progressed_steps":96, "sleep_seconds":120,
            "post_wake_awake_ticks":8, "post_wake_seconds":60,
            "no_progress":"counted separately; wall bound applies",
            "sleep_trigger":"public request_recovery_sleep, one cycle",
            "automatic_retry":false, "default_promoted":false,
        }),
    );
    let (mut runtime, assets) =
        super::super::founder_consequence_tests::paired_food_runtime_with_action_credit(
            &asset,
            true,
            &root.join("life"),
            true,
            Some(&configured),
        );
    let acquisition = observe_awake(&mut runtime, &phenotype, &root, "acquisition", 8, None);
    let learned = retain(&mut runtime, &root, &assets, "learned");
    let acquisition_pass =
        acquisition.preference_passed() && acquisition.negative_harmful_feedback > 0;
    save(
        &root,
        "acquisition-gate.json",
        &serde_json::json!({
            "pass":acquisition_pass, "complete_checkpoint":learned.is_some(), "observations":acquisition,
        }),
    );
    assert!(
        acquisition_pass && learned.is_some(),
        "acquisition or learned checkpoint failed"
    );
    let learned = learned.unwrap();
    let source_brain = brain(&learned).clone();
    assert!(
        source_brain.pending_eligibility.is_none()
            && source_brain.pending_experience_transaction.is_none()
    );
    let handle = *runtime.handles.get(&ORGANISM.raw()).unwrap();
    let source_snapshot = runtime
        .backend
        .snapshot_brain(handle, learned.world.tick)
        .unwrap();
    let reference = observe_awake(&mut runtime, &phenotype, &root, "reference", 1, None);
    let reference_checkpoint = retain(&mut runtime, &root, &assets, "after-reference");
    assert!(
        reference.error.is_none()
            && reference.decisions.len() == 1
            && reference_checkpoint.is_some()
    );
    drop(runtime);

    // Sleep may advance the live manifest. Preserve the named learned boundary.
    let live_path = root.join("resume-live.json");
    GpuDurableSaveManifest::publish_snapshot(&live_path, &assets, &learned).unwrap();
    let (manifest, loaded) = GpuDurableSaveManifest::open_loaded(&live_path, &assets).unwrap();
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    save(&root, "restore-hardware.json", &backend.hardware_receipt());
    let mut restored = match GpuLiveBrainRuntime::restore_loaded_save(
        backend,
        manifest,
        loaded,
        learned.deterministic_seed,
        BrainScaleTier::Nano512,
    ) {
        Ok(runtime) => runtime,
        Err(error) => {
            save(
                &root,
                "exact-restore-gate.json",
                &serde_json::json!({"pass":false,"error":format!("{error:?}")}),
            );
            panic!("production restore failed; learned checkpoint retained");
        }
    };
    let recaptured = retain(&mut restored, &root, &assets, "restored-before-advance")
        .expect("retain restored boundary before equality checks");
    let handle = *restored.handles.get(&ORGANISM.raw()).unwrap();
    let restored_snapshot = restored
        .backend
        .snapshot_brain(handle, learned.world.tick)
        .unwrap();
    let store = GpuCheckpointAssetStore::new(&assets).unwrap();
    let old_cognition = store
        .read_exact_cognitive_state(
            &learned.assets,
            source_brain.exact_cognitive_state.as_ref().unwrap(),
        )
        .unwrap();
    let new_cognition = store
        .read_exact_cognitive_state(
            &recaptured.assets,
            brain(&recaptured).exact_cognitive_state.as_ref().unwrap(),
        )
        .unwrap();
    let exact_pass = learned.world == recaptured.world
        && source_brain == *brain(&recaptured)
        && source_snapshot == restored_snapshot
        && old_cognition == new_cognition;
    save(
        &root,
        "exact-restore-gate.json",
        &serde_json::json!({
            "pass":exact_pass, "world_equal":learned.world == recaptured.world,
            "full_brain_save_state_equal":source_brain == *brain(&recaptured),
            "mutable_gpu_snapshot_equal":source_snapshot == restored_snapshot,
            "exact_cognitive_state_equal":old_cognition == new_cognition,
            "source_snapshot_digest":source_snapshot.canonical_digest(),
            "restored_snapshot_digest":restored_snapshot.canonical_digest(),
            "source_brain":source_brain, "restored_brain":brain(&recaptured),
        }),
    );
    assert!(
        exact_pass,
        "exact restore mismatch; both complete boundaries retained"
    );

    let post_load = observe_awake(
        &mut restored,
        &phenotype,
        &root,
        "post-load",
        8,
        reference.decisions.first(),
    );
    let before_sleep = retain(&mut restored, &root, &assets, "before-sleep");
    let reference_pass = post_load.decisions.first() == reference.decisions.first();
    save(
        &root,
        "reference-decision-gate.json",
        &serde_json::json!({
            "pass":reference_pass,"uninterrupted":reference.decisions.first(),"restored":post_load.decisions.first(),
        }),
    );
    save(
        &root,
        "post-load-preference-gate.json",
        &serde_json::json!({
            "pass":post_load.preference_passed(), "observations":post_load,
            "complete_checkpoint":before_sleep.is_some(),
        }),
    );
    assert!(
        reference_pass && post_load.preference_passed() && before_sleep.is_some(),
        "post-load gate failed"
    );

    let pre_sleep = restored.sleep_state_for_test(ORGANISM).unwrap();
    let transition = restored.request_recovery_sleep(ORGANISM).unwrap();
    save(
        &root,
        "sleep-request.json",
        &serde_json::json!({"before":pre_sleep,"transition":transition}),
    );
    let started = Instant::now();
    let mut progressed = 0;
    let mut polls = 0;
    let mut saw_submitted = false;
    let mut saw_completed = false;
    let mut saw_committed = false;
    let mut saw_waking = false;
    let mut compactions = Vec::new();
    let mut observations = Vec::new();
    let mut prior_state = None;
    let mut sleep_error = None;
    while progressed < 96 && started.elapsed().as_secs() < 120 {
        match restored.tick_outcome() {
            Ok(GpuLiveTickOutcome::Progressed(_)) => progressed += 1,
            Ok(GpuLiveTickOutcome::NoProgress(_)) => polls += 1,
            Err(error) => {
                sleep_error = Some(format!("{error:?}"));
                break;
            }
        }
        let state = restored.sleep_state_for_test(ORGANISM).unwrap();
        for receipt in restored.last_memory_compaction_receipts() {
            let row = serde_json::to_value(receipt).unwrap();
            if !compactions.contains(&row) {
                compactions.push(row);
            }
        }
        saw_submitted |= matches!(state.consolidation, ConsolidationState::Submitted { .. });
        saw_completed |= matches!(state.consolidation, ConsolidationState::Completed { .. });
        saw_committed |= matches!(state.consolidation, ConsolidationState::Committed { .. });
        saw_waking |= state.phase == SleepPhase::Waking;
        if prior_state != Some(state) {
            let row = serde_json::json!({"world_tick":restored.world_snapshot().tick(),
                "progressed":progressed,"no_progress":polls,"state":state,"compactions":compactions});
            save(
                &root,
                &format!("sleep-observation-{:03}.json", observations.len()),
                &row,
            );
            observations.push(row);
            prior_state = Some(state);
        }
        if state.phase == SleepPhase::Awake
            && state.last_consolidated_cycle_id > pre_sleep.last_consolidated_cycle_id
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let after_sleep = restored.sleep_state_for_test(ORGANISM).unwrap();
    let compaction = restored.memory_compaction_checkpoint(ORGANISM);
    let sleep_pass = sleep_error.is_none()
        && saw_submitted
        && saw_completed
        && saw_committed
        && saw_waking
        && after_sleep.phase == SleepPhase::Awake
        && after_sleep.last_consolidated_cycle_id == pre_sleep.last_consolidated_cycle_id + 1
        && !compactions.is_empty()
        && compaction.is_some_and(|c| {
            c.last_committed_cycle_id == Some(after_sleep.last_consolidated_cycle_id)
        });
    let sleep_checkpoint = retain(&mut restored, &root, &assets, "after-sleep");
    save(
        &root,
        "sleep-gate.json",
        &serde_json::json!({
            "pass":sleep_pass,"progressed":progressed,"no_progress":polls,"error":sleep_error,
            "before":pre_sleep,"after":after_sleep,"compactions":compactions,"compaction_checkpoint":compaction,
            "saw_submitted":saw_submitted,"saw_completed":saw_completed,"saw_committed":saw_committed,
            "saw_waking":saw_waking,"complete_checkpoint":sleep_checkpoint.is_some(),
        }),
    );
    assert!(
        sleep_pass && sleep_checkpoint.is_some(),
        "sleep failed; partial receipts and prior checkpoints retained"
    );
    let post_wake = observe_awake(&mut restored, &phenotype, &root, "post-wake", 8, None);
    let final_checkpoint = retain(&mut restored, &root, &assets, "final");
    save(
        &root,
        "post-wake-preference-gate.json",
        &serde_json::json!({
            "pass":post_wake.preference_passed(),"observations":post_wake,"complete_checkpoint":final_checkpoint.is_some(),
        }),
    );
    assert!(
        post_wake.preference_passed() && final_checkpoint.is_some(),
        "post-wake preference failed"
    );
    save(
        &root,
        "completion.json",
        &serde_json::json!({
            "acquisition":"PASS","exact_restore":"PASS","reference_decision":"PASS",
            "post_load_preference":"PASS","sleep_completion":"PASS","post_wake_preference":"PASS",
            "candidate_digest":asset.digest(),"life":"new acquired life","default_promoted":false,
        }),
    );
}

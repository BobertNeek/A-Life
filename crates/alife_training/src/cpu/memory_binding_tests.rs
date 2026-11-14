use super::*;
use crate::*;
#[path = "../../tests/support/terminal_parent2135.rs"]
#[allow(dead_code)]
mod parent;

fn config(host: &CpuCanonicalHost, coefficient: f32) -> FrozenMemoryReadoutConstructionV1 {
    FrozenMemoryReadoutConstructionV1 {
        schema_version: 1,
        readout_abi_version: 1,
        work_policy_version: 1,
        parent_genome: host
            .world()
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .genome()
            .clone(),
        source_family: CandidateActionFamily::Ingest,
        destination_family: CandidateActionFamily::Approach,
        coefficient,
    }
}
fn save(name: &str, data: &impl serde::Serialize) {
    if let Some(root) = std::env::var_os("ALIFE_BINDING_QUALIFY_OUTPUT") {
        use std::io::Write;
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        let ignore = root.join(".gitignore");
        if !ignore.exists() {
            std::fs::write(ignore, b"*\n").unwrap();
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(name))
            .unwrap();
        file.write_all(&serde_json::to_vec(data).unwrap()).unwrap();
        file.sync_all().unwrap();
    }
}
#[test]
fn memory_readout_native_cold_and_frozen_construction_are_exact() {
    let (mut host, trainer) = parent::contact20_fixture(None);
    let adam = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    let gene = config(&host, 4.);
    host.enable_frozen_memory_readout(gene, &trainer).unwrap();
    let birth = host.checkpoint(&trainer).unwrap();
    assert_eq!(birth.schema_version, 5);
    save("COLD-BIRTH.cp.json", &birth);
    save("COLD-ADAM.json", &trainer.checkpoint());
    let mut cold = CpuCanonicalHost::from_checkpoint(
        serde_json::from_slice(&serde_json::to_vec(&birth).unwrap()).unwrap(),
        &trainer,
    )
    .unwrap();
    let mut collected = Vec::new();
    for index in 0..2 {
        let row = host.step(&trainer, 1.).unwrap();
        let twin = cold.step(&trainer, 1.).unwrap();
        assert_eq!(
            serde_json::to_vec(&row.decisions).unwrap(),
            serde_json::to_vec(&twin.decisions).unwrap()
        );
        let cp = host.checkpoint(&trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&cp).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        save(&format!("COLD-{index}.rows.json"), &row.decisions);
        save(&format!("COLD-{index}.cp.json"), &cp);
        collected.extend(row.decisions);
    }
    check_research_collection_training_rejection(&host, &trainer, &collected);
    let cp = host.checkpoint(&trainer).unwrap();
    let mut altered = cp.clone();
    altered.residents[0]
        .memory_readout
        .as_mut()
        .unwrap()
        .coefficient = -4.;
    assert!(CpuCanonicalHost::from_checkpoint(altered, &trainer).is_err());
    let mut changed_work = cp.clone();
    changed_work.residents[0]
        .memory_readout
        .as_mut()
        .unwrap()
        .work_policy_version = 2;
    assert!(CpuCanonicalHost::from_checkpoint(changed_work, &trainer).is_err());
    let mut legacy_schema = cp;
    legacy_schema.schema_version = 2;
    assert!(CpuCanonicalHost::from_checkpoint(legacy_schema, &trainer).is_err());
    assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), adam);
}

fn check_research_collection_training_rejection(
    host: &CpuCanonicalHost,
    trainer: &CpuFoundationTrainer,
    decisions: &[CpuCollectedDecision],
) {
    assert_eq!(decisions.len(), 2);
    assert!(decisions
        .iter()
        .all(|d| d.memory_readout_research.is_some()));
    assert!(decisions.iter().all(|d| d
        .replay_tick
        .candidates
        .iter()
        .any(|c| c.memory_readout.is_some())));
    let phenotype = host
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .phenotype();
    let value = hotpath_bench::value(trainer.phenotype());
    let config = PpoConfig {
        temperature: 1., // actual native collection temperature above
        ..hotpath_bench::configuration()
    };
    let mut legacy = decisions.to_vec();
    for row in &mut legacy {
        row.memory_readout_research = None;
        for candidate in &mut row.replay_tick.candidates {
            candidate.memory_readout = None;
        }
    }
    let windows = |rows: &[CpuCollectedDecision]| {
        cpu_training_windows(rows, None, trainer, &value, phenotype, 7400, config, 2, 0)
    };
    // Removing both research markers is only a structural test control. It is
    // never used as an admitted research trajectory or an optimizer input.
    assert!(!windows(&legacy).unwrap().is_empty());
    for (receipt, replay) in [(true, true), (true, false), (false, true)] {
        let mut rows = legacy.clone();
        // Even a research-only bootstrap row must prevent receipt loss.
        if receipt {
            rows[1].memory_readout_research = decisions[1].memory_readout_research.clone();
        }
        if replay {
            rows[1].replay_tick = decisions[1].replay_tick.clone();
        }
        assert!(windows(&rows).is_err(), "receipt={receipt} replay={replay}");
    }
}

#[test]
fn memory_readout_residual_cohort_rejects_before_any_optimizer_mutation() {
    check_research_cohort_rejection(true);
}

#[test]
fn memory_readout_ppo_cohort_rejects_before_any_optimizer_mutation() {
    check_research_cohort_rejection(false);
}

fn check_research_cohort_rejection(residual: bool) {
    let (phenotype, source) = hotpath_bench::fixture();
    let mut trainer = hotpath_bench::learner(&phenotype, &source);
    let mut value = hotpath_bench::value(&phenotype);
    let config = hotpath_bench::configuration();
    let ordinary = hotpath_bench::window(&trainer, 3, 1);
    super::cohort::validate_frozen_residual_window(&trainer, &value, &ordinary, config, 7400)
        .unwrap();
    let actor_before = trainer.checkpoint();
    let critic_before = value.checkpoint();
    // These retained fixtures have nonzero moments and exact ages above 2^24.
    assert!(actor_before.first_moment.iter().any(|m| *m != 0.));
    assert!(actor_before.second_moment.iter().any(|v| *v != 0.));
    assert!(actor_before.update_ages.iter().any(|age| *age > 1 << 24));
    for invalid_window in [0, 1] {
        for tick_index in [0, 2] {
            let mut research = ordinary.clone();
            let tick = &mut research.actor.sequence.ticks[tick_index];
            // A valid zero-source research term keeps all scores/probabilities
            // unchanged, so rejection cannot be explained by stale PPO data.
            tick.candidates[0].memory_readout = Some(Box::new(FrozenMemoryReadoutTermV1 {
                schema_version: 1,
                destination_family: CandidateActionFamily::Approach,
                source_family: CandidateActionFamily::Ingest,
                source_candidate_index: None,
                organism_id: OrganismId(1),
                current_entity: None,
                current_tracked_target: None,
                query_tick: Tick::new(1),
                profile: SensorProfileIdentity {
                    profile_id: phenotype.sensor_profile().into(),
                    profile_schema_version: 1,
                    sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
                },
                frame_digest: PerceptionFrameDigest([0; 4]),
                source_confidence: 0.,
                source_energy: 0.,
                coefficient: 4.,
                status: PreparedMemoryBindingStatusV1::NoTrackedTarget,
                provenance: None,
            }));
            research.actor.sequence.validate_for(&phenotype).unwrap();
            assert_eq!(
                trainer.evaluate_replay(&research.actor.sequence).unwrap(),
                trainer.evaluate_replay(&ordinary.actor.sequence).unwrap()
            );
            let adjoints = vec![vec![0.1; 16]; 2];
            assert_eq!(
                trainer
                    .probe_replay_gradients(&research.actor.sequence, &adjoints)
                    .unwrap(),
                trainer
                    .probe_replay_gradients(&ordinary.actor.sequence, &adjoints)
                    .unwrap()
            );
            let load = |i| {
                Ok(if i == invalid_window {
                    research.clone()
                } else {
                    ordinary.clone()
                })
            };
            let result = if residual {
                train_cpu_residual_cohort(&mut trainer, &mut value, 2, 1, load, config, 7400)
            } else {
                train_cpu_ppo_cohort(
                    &mut trainer,
                    &mut value,
                    2,
                    1,
                    |i| Ok(load(i)?.actor),
                    config,
                    7400,
                )
            };
            assert!(
                result.is_err(),
                "residual={residual} window={invalid_window} tick={tick_index}"
            );
            let after = trainer.checkpoint();
            assert_eq!(after.weights, actor_before.weights);
            assert_eq!(after.first_moment, actor_before.first_moment);
            assert_eq!(after.second_moment, actor_before.second_moment);
            assert_eq!(after.update_ages, actor_before.update_ages);
            assert_eq!(after.optimizer_step, actor_before.optimizer_step);
            assert_eq!(after, actor_before);
            assert_eq!(value.checkpoint(), critic_before);
        }
    }
}
#[test]
fn memory_readout_fresh_no_memory_has_zero_influence_at_matched_tariff() {
    let (mut zero, trainer) = parent::contact20_fixture(None);
    let mut positive = zero.snapshot();
    let (z, p) = (config(&zero, 0.), config(&positive, 4.));
    zero.enable_frozen_memory_readout(z, &trainer).unwrap();
    positive.enable_frozen_memory_readout(p, &trainer).unwrap();
    let a = zero.step(&trainer, 1.).unwrap();
    let b = positive.step(&trainer, 1.).unwrap();
    assert_eq!(a.decisions.len(), 1);
    let (a, b) = (&a.decisions[0], &b.decisions[0]);
    assert_eq!(a.initial, b.initial);
    assert_eq!(a.final_features, b.final_features);
    assert_eq!(a.action, b.action);
    assert_eq!(a.brain_work, b.brain_work);
    let mut found = 0;
    for (x, y) in a
        .replay_tick
        .candidates
        .iter()
        .zip(&b.replay_tick.candidates)
    {
        assert_eq!(x.decoder_inputs, y.decoder_inputs);
        if let Some(term) = &y.memory_readout {
            found += 1;
            assert_ne!(term.status, PreparedMemoryBindingStatusV1::Bound);
            assert_eq!(term.contribution(), 0.);
        }
    }
    assert!(found > 0);
    assert_eq!(a.patch.outcome(), b.patch.outcome());
    assert!(positive
        .enable_frozen_memory_readout(config(&positive, 1.), &trainer)
        .is_err());
    save("NO-MEMORY-ZERO.rows.json", &vec![a]);
    save("NO-MEMORY-POSITIVE.rows.json", &vec![b]);
    save(
        "NO-MEMORY-ZERO.cp.json",
        &zero.checkpoint(&trainer).unwrap(),
    );
    save(
        "NO-MEMORY-POSITIVE.cp.json",
        &positive.checkpoint(&trainer).unwrap(),
    );
}
#[test]
fn memory_readout_wire_off_is_legacy_and_enabled_is_explicit() {
    let candidate = TrainingReplayCandidate {
        memory_readout: None,
        family: CandidateActionFamily::Approach,
        decoder_inputs: [0.; 54],
        innate_bias: 0.,
    };
    let old =
        serde_json::json!({"family":"Approach","decoder_inputs":vec![0.;54],"innate_bias":0.});
    assert_eq!(serde_json::to_value(&candidate).unwrap(), old);
    assert_eq!(
        serde_json::from_value::<TrainingReplayCandidate>(old).unwrap(),
        candidate
    );
    let (mut host, trainer) = parent::contact20_fixture(None);
    host.enable_frozen_memory_readout(config(&host, 0.), &trainer)
        .unwrap();
    let step = host.step(&trainer, 1.).unwrap();
    let collected = serde_json::to_vec(&step.decisions).unwrap();
    assert!(serde_json::from_slice::<Vec<CpuCollectedDecision>>(&collected).is_err());
    let envelope: serde_json::Value = serde_json::from_slice(&collected).unwrap();
    assert!(
        envelope[0]["memory_readout_research_record_v1"]["memory_readout_research"].is_object()
    );
    let enabled = step.decisions[0]
        .replay_tick
        .candidates
        .iter()
        .find(|c| c.memory_readout.is_some())
        .unwrap();
    let bytes = serde_json::to_vec(enabled).unwrap();
    #[derive(serde::Deserialize)]
    struct Old {
        #[allow(dead_code)]
        family: CandidateActionFamily,
    }
    assert!(serde_json::from_slice::<Old>(&bytes).is_err());
    assert_eq!(
        &serde_json::from_slice::<TrainingReplayCandidate>(&bytes).unwrap(),
        enabled
    );
    let mut mixed = serde_json::to_value(enabled).unwrap();
    mixed["family"] = serde_json::json!("Approach");
    mixed["decoder_inputs"] = serde_json::json!(vec![0.; 54]);
    assert!(serde_json::from_value::<TrainingReplayCandidate>(mixed).is_err());
    save("WIRE-ENABLED.rows.json", &step.decisions);
    save("WIRE-ENABLED.cp.json", &host.checkpoint(&trainer).unwrap());
}
#[test]
fn memory_readout_numerical_cap_and_legacy_gradient_semantics() {
    let (host, trainer) = parent::contact20_fixture(None);
    let p = trainer.phenotype();
    let n = p.neuron_count() as usize;
    let profile = SensorProfileIdentity {
        profile_id: p.sensor_profile().into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    };
    let source = MemoryFamilySourceV1 {
        memory_id: MemoryId(1),
        organism_id: OrganismId(1),
        profile,
        family: CandidateActionFamily::Ingest,
        prior_tracked_target: Some(TrackedObjectId(7)),
        sequence_id: ExperienceSequenceId(1),
        first_tick: Tick::ZERO,
        last_tick: Tick::ZERO,
        observation_count: 1,
        episode_confidence: 1.,
        similarity: 1.,
        aggregate_weight: 1.,
        outcome_contact: Some(PhysicalContactKind::Touch),
    };
    let mut ordinary = TrainingReplayCandidate {
        memory_readout: None,
        family: CandidateActionFamily::Approach,
        decoder_inputs: [0.; 54],
        innate_bias: 0.,
    };
    ordinary.decoder_inputs[24] = 0.1;
    let mut ingest = TrainingReplayCandidate {
        memory_readout: None,
        family: CandidateActionFamily::Ingest,
        decoder_inputs: [0.; 54],
        innate_bias: 0.,
    };
    ingest.decoder_inputs[35] = 0.2;
    let mut q = TrainingSequence {
        phenotype_hash: p.phenotype_hash(),
        initial: TrainingInitialState {
            activations: vec![0.; n],
            activity_ema: vec![0.; n],
            metabolic_load: vec![0.; n],
            dendrites: Default::default(),
        },
        ticks: vec![TrainingReplayTick {
            encoded_inputs: vec![0.; n],
            projection_gain: 1.,
            local_threshold_shift: 0.,
            microstep_count: 0,
            enabled_routes: vec![true; p.projections().len()],
            effective_weight_offsets: vec![0.; p.synapses().len()],
            structural_synapses: vec![],
            candidates: vec![ordinary, ingest],
        }],
        burn_in_ticks: 0,
        memory_candidate_gain: 0.5,
    };
    let baseline = trainer.evaluate_replay(&q).unwrap();
    let term = FrozenMemoryReadoutTermV1 {
        schema_version: 1,
        destination_family: CandidateActionFamily::Approach,
        source_family: CandidateActionFamily::Ingest,
        source_candidate_index: Some(1),
        organism_id: OrganismId(1),
        current_entity: Some(WorldEntityId(1)),
        current_tracked_target: Some(TrackedObjectId(7)),
        query_tick: Tick::new(1),
        profile,
        frame_digest: PerceptionFrameDigest([0; 4]),
        source_confidence: 1.,
        source_energy: 0.2,
        coefficient: 1.,
        status: PreparedMemoryBindingStatusV1::Bound,
        provenance: Some(MemoryFamilySourcesV1 {
            sources: [Some(source), None, None, None],
            category_generalized: false,
        }),
    };
    q.ticks[0].candidates[0].memory_readout = Some(Box::new(term));
    let ids=p.synapses().iter().enumerate().filter_map(|(i,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c)if c.head()==DecoderHeadKind::MemoryContext&&c.family()==CandidateActionFamily::Approach&&c.input_lane()==24).then_some(i)).collect::<Vec<_>>();
    assert!(!ids.is_empty());
    for (coefficient, delta, gradient) in
        [(0., 0., 0.1), (1., 0.2, 0.1), (2.5, 0.5, 0.), (4., 0.5, 0.)]
    {
        q.ticks[0].candidates[0]
            .memory_readout
            .as_mut()
            .unwrap()
            .coefficient = coefficient;
        let actual = trainer.evaluate_replay(&q).unwrap();
        assert!(
            (actual.candidate_logits[0][0] - baseline.candidate_logits[0][0] - delta).abs() < 1e-6
        );
        assert_eq!(actual.final_activations, baseline.final_activations);
        let probe = trainer.probe_replay_gradients(&q, &[vec![1., 0.]]).unwrap();
        assert_eq!(probe.gradients.len(), p.synapses().len());
        for &id in &ids {
            assert_eq!(probe.gradients[id], gradient);
        }
    }
    for fault in 0..7 {
        let mut bad = q.clone();
        let term = bad.ticks[0].candidates[0].memory_readout.as_mut().unwrap();
        match fault {
            0 => term.source_candidate_index = Some(0),
            1 => term.source_candidate_index = Some(99),
            2 => term.organism_id = OrganismId(2),
            3 => term.current_tracked_target = None,
            4 => term.source_energy = 0.3,
            5 => {
                term.provenance.as_mut().unwrap().sources[0]
                    .as_mut()
                    .unwrap()
                    .last_tick = Tick::new(2)
            }
            6 => {
                term.provenance.as_mut().unwrap().sources[0]
                    .as_mut()
                    .unwrap()
                    .profile
                    .profile_id = SensorProfileId(1)
            }
            _ => unreachable!(),
        }
        assert!(trainer.evaluate_replay(&bad).is_err());
    }
    save("NUMERICAL-ADAM.json", &trainer.checkpoint());
    save(
        "NUMERICAL-BIRTH.cp.json",
        &host.checkpoint(&trainer).unwrap(),
    );
}

#[test]
#[ignore = "explicit local historical checkpoint proof; no world steps"]
fn memory_readout_legacy_private_checkpoint_roundtrip_is_exact() {
    let root = std::path::PathBuf::from(
        std::env::var_os("ALIFE_BINDING_LEGACY_RUN").expect("explicit local run required"),
    );
    let (_, trainer) = parent::contact20_fixture(None);
    for name in [
        "PREPARED-BIRTH.cp.json",
        "STEP-102-PRE.cp.json",
        "FINAL.cp.json",
    ] {
        let original = std::fs::read(root.join(name)).unwrap();
        let checkpoint: CpuCanonicalCheckpoint = serde_json::from_slice(&original).unwrap();
        assert_eq!(checkpoint.schema_version, 2);
        let restored = CpuCanonicalHost::from_checkpoint(checkpoint, &trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&restored.checkpoint(&trainer).unwrap()).unwrap(),
            original,
            "{name}"
        );
    }
    let request: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("REQUEST.json")).unwrap()).unwrap();
    for interval in [102, 108, 110] {
        let tape: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join(format!("STEP-{interval:03}-0-TAPE.json"))).unwrap(),
        )
        .unwrap();
        let sequence: TrainingSequence = serde_json::from_value(tape["sequence"].clone()).unwrap();
        let actual = trainer.evaluate_replay(&sequence).unwrap();
        let logits: Vec<f32> = serde_json::from_value(tape["native_logits"].clone()).unwrap();
        let activity: Vec<f32> = serde_json::from_value(tape["native_final"].clone()).unwrap();
        assert_eq!(actual.candidate_logits[0], logits);
        assert_eq!(actual.final_activations[0], activity);
        let rows: Vec<CpuCollectedDecision> = serde_json::from_slice(
            &std::fs::read(root.join(format!("STEP-{interval:03}.rows.json"))).unwrap(),
        )
        .unwrap();
        let row = &rows[0];
        assert_eq!(
            sample_cpu_joint_action(
                trainer.phenotype(),
                row.patch.pre_action().perception(),
                &logits,
                request["seed"].as_u64().unwrap() as u32,
                row.sampling_counter.unwrap(),
                row.action.temperature
            )
            .unwrap(),
            row.action
        );
    }
}

#[test]
fn memory_readout_missing_and_ambiguous_current_targets_do_not_borrow() {
    use alife_core::*;
    let (host, _) = parent::contact20_fixture(None);
    let mut gene = config(&host, 4.);
    let owner = OrganismId(9);
    let now = Tick::new(3);
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let slots = (0..2)
        .map(|i| GroundedObjectSlotV1 {
            slot_index: i,
            tracked_object_id: TrackedObjectId(7 + u64::from(i)),
            bearing: [0.; 2],
            distance: 0.2,
            relative_velocity: [0.; 3],
            color: [0.3; 3],
            material: [0.2; 3],
            shape: [0.4; 3],
            chemical: [0.; 3],
            contact: 0.,
            proprioception: [0.; 2],
            temperature: 0.,
            terrain: [0.5; 2],
            confidence: Confidence::new(1.).unwrap(),
        })
        .collect::<Vec<_>>();
    for (mode, expected) in [
        (0, PreparedMemoryBindingStatusV1::MissingSourceCandidate),
        (1, PreparedMemoryBindingStatusV1::MissingMemory),
        (2, PreparedMemoryBindingStatusV1::AmbiguousSourceCandidate),
        (3, PreparedMemoryBindingStatusV1::MissingSourceCandidate),
    ] {
        let selected = if mode == 0 { 1 } else { 0 };
        gene.source_family = if mode == 3 {
            CandidateActionFamily::Rest
        } else {
            CandidateActionFamily::Ingest
        };
        let mut candidates = Vec::new();
        for (index, slot_index, family, kind) in [
            (0, 0, CandidateActionFamily::Approach, ActionKind::Move),
            (
                1,
                selected,
                gene.source_family,
                if mode == 3 {
                    ActionKind::Rest
                } else {
                    ActionKind::Interact
                },
            ),
        ]
        .into_iter()
        .chain((mode == 2).then_some((
            2,
            0,
            CandidateActionFamily::Ingest,
            ActionKind::Interact,
        ))) {
            let slot = &slots[slot_index];
            candidates.push(
                ActionCandidate::new(
                    index,
                    ActionId(200 + u32::from(index)),
                    kind,
                    family,
                    if mode == 3 && index == 1 {
                        CandidateObservationRef::None
                    } else {
                        CandidateObservationRef::ObjectSlot(slot.slot_index)
                    },
                    if mode == 3 && index == 1 {
                        ActionTarget::NONE
                    } else {
                        ActionTarget::new(
                            Some(WorldEntityId(1 + slot_index as u64)),
                            Some(Vec3f::new(0.2, 0., 0.)),
                        )
                    },
                    if mode == 3 && index == 1 {
                        CandidateFeatureVector::zero()
                    } else {
                        slot.candidate_features().unwrap()
                    },
                    Confidence::new(1.).unwrap(),
                    NormalizedScalar::new(0.2).unwrap(),
                    DurationTicks::new(1),
                    DurationTicks::new(1),
                )
                .unwrap(),
            );
        }
        let draft = PerceptionFrameDraft::new(
            owner,
            now,
            profile,
            SensorySnapshot::new(
                owner,
                now,
                Vec3f::ZERO,
                SensoryChannels::ZERO,
                Default::default(),
            )
            .unwrap(),
            BodySnapshot {
                pose: Pose::IDENTITY,
                velocity: Velocity::ZERO,
            },
            HomeostaticSnapshot::new(
                now,
                DriveSnapshot::baseline(),
                EndocrineSnapshot::baseline(),
            )
            .unwrap(),
            candidates,
            SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, now).unwrap(),
            slots.clone(),
        )
        .unwrap();
        let bank = MemoryBank::new(
            MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
        )
        .unwrap();
        let (frame, recall) = bank
            .recall_frame_with_family_sources(&draft)
            .unwrap()
            .finalize(draft.clone())
            .unwrap();
        let mut tick = TrainingReplayTick {
            encoded_inputs: vec![],
            projection_gain: 1.,
            local_threshold_shift: 0.,
            microstep_count: 0,
            enabled_routes: vec![],
            effective_weight_offsets: vec![],
            structural_synapses: vec![],
            candidates: frame
                .candidates()
                .iter()
                .map(|c| TrainingReplayCandidate {
                    family: c.family,
                    decoder_inputs: [0.; 54],
                    innate_bias: 0.,
                    memory_readout: None,
                })
                .collect(),
        };
        let original = tick.clone();
        super::bind_prepared_memory_readout(&gene, &frame, &recall, &mut tick).unwrap();
        let mut unchanged = tick.clone();
        for candidate in &mut unchanged.candidates {
            candidate.memory_readout = None;
        }
        assert_eq!(unchanged, original);
        for index in [0, 1] {
            let mut bad = original.clone();
            bad.candidates[index].family = CandidateActionFamily::Avoid;
            assert!(super::bind_prepared_memory_readout(&gene, &frame, &recall, &mut bad).is_err());
        }
        let changed = draft.clone().with_remembered_novelty(0.5).unwrap();
        let (changed_frame, _) = bank
            .recall_frame_with_family_sources(&changed)
            .unwrap()
            .finalize(changed)
            .unwrap();
        assert!(super::bind_prepared_memory_readout(
            &gene,
            &changed_frame,
            &recall,
            &mut original.clone()
        )
        .is_err());
        let other = OrganismId(owner.raw() + 1);
        let foreign = PerceptionFrameDraft::new(
            other,
            now,
            profile,
            SensorySnapshot::new(
                other,
                now,
                Vec3f::ZERO,
                SensoryChannels::ZERO,
                Default::default(),
            )
            .unwrap(),
            BodySnapshot {
                pose: Pose::IDENTITY,
                velocity: Velocity::ZERO,
            },
            HomeostaticSnapshot::new(
                now,
                DriveSnapshot::baseline(),
                EndocrineSnapshot::baseline(),
            )
            .unwrap(),
            frame.candidates().to_vec(),
            SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, now).unwrap(),
            slots.clone(),
        )
        .unwrap();
        let (foreign_frame, _) = bank
            .recall_frame_with_family_sources(&foreign)
            .unwrap()
            .finalize(foreign)
            .unwrap();
        assert!(super::bind_prepared_memory_readout(
            &gene,
            &foreign_frame,
            &recall,
            &mut original.clone()
        )
        .is_err());
        let term = tick.candidates[0].memory_readout.as_ref().unwrap();
        assert_eq!(term.status, expected);
        assert_eq!(term.contribution(), 0.);
        assert!(term.provenance.is_none());
    }
}

use super::*;

fn meal_event(
    bank: &MemoryBank,
    sequence: u64,
    contact: PhysicalContactKind,
    reward: f32,
    pain: f32,
) -> ExperiencePatch {
    observation_patch(
        bank,
        sequence,
        sequence,
        slot(0, 71, 0.4, [0., 0.8, 0.9]),
        CandidateActionFamily::Ingest,
        reward,
        pain,
        |outcome| {
            // The fixture candidate has index zero and therefore entity9000.
            outcome.physical.target_entity = Some(WorldEntityId(9000));
            outcome.physical.contact = contact;
            // A novel preparation outcome is meaningful independently of harm.
            outcome.prediction_error = NormalizedScalar::new(0.2).unwrap();
        },
    )
}

fn expected_meal(bank: &MemoryBank, tick: u64) -> f32 {
    bank.recall_frame(&probe_for_object(tick, slot(0, 71, 0.4, [0., 0.8, 0.9])))
        .unwrap()
        .context()
        .candidates[0]
        .family_value[0]
}

#[test]
fn painless_ingest_pickup_does_not_reverse_a_harmful_meal() {
    let mut bank = empty_bank();
    let meal = meal_event(&bank, 1, PhysicalContactKind::Consumed, -0.9, 0.9);
    bank.observe_sealed_patch(&meal).unwrap();
    assert!(expected_meal(&bank, 2) < -0.8);
    let pickup = meal_event(&bank, 2, PhysicalContactKind::Touch, 0.01, 0.);
    bank.observe_sealed_patch(&pickup).unwrap();
    assert_eq!(bank.len(), 2);
    assert!(expected_meal(&bank, 3) < -0.8);
    let raw = serde_json::to_vec(&bank).unwrap();
    let mut restored: MemoryBank = serde_json::from_slice(&raw).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), raw);
    assert!(expected_meal(&restored, 3) < -0.8);
    let reversal = meal_event(&restored, 3, PhysicalContactKind::Consumed, 0.8, 0.);
    restored.observe_sealed_patch(&reversal).unwrap();
    assert!(expected_meal(&restored, 4) > 0.7);
}

#[test]
fn all_unknown_legacy_bytes_and_digest_streams_are_frozen() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, -0.9, 0.9))
        .unwrap();
    let raw = serde_json::to_vec(&bank).unwrap();
    let profile = SensorProfileProvenance::new(
        SensorProfile::GroundedObjectSlotsV1,
        SensoryAbiVersion::CURRENT,
        TICK,
    )
    .unwrap()
    .identity();
    let mut sidecar = MemorySidecarState::new_profiled(
        ORGANISM,
        profile,
        MemoryBankConfig::new(4, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    )
    .unwrap();
    sidecar
        .observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, -0.9, 0.9))
        .unwrap();
    let portable = sidecar.export_active_bank().unwrap();
    // Captured before the production change on mainfc7dd78. The old fixture
    // has contact-like floats but no target-bound phase: never infer a phase.
    assert_eq!(
        blake3::hash(&raw).to_hex().as_str(),
        "9e6f708dc1ade591198a2d2e5e99a741a38a89acf70490efb9118dc1875a176c"
    );
    assert_eq!(
        blake3::hash(&serde_json::to_vec(&portable).unwrap())
            .to_hex()
            .as_str(),
        "809f2bd501e991f532cac13f33dd5498c05fbee25b28551e64190538179507cb"
    );
    assert_eq!(
        portable.records[0].canonical_digest,
        [
            5385564082496650174,
            2716643615302296869,
            12791915244069648048,
            16434942257841790803
        ]
    );
    assert_eq!(
        portable.active_bank_digest,
        [
            17353935635923336848,
            11716519859850928482,
            12877535766558527199,
            17377705246078496016
        ]
    );
    assert_eq!(portable.records[0].outcome_contact, None);
    let restored: MemoryBank = serde_json::from_slice(&raw).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), raw);
    assert!(expected_meal(&restored, 3) < -0.8);
    let restored = MemorySidecarState::restore_portable(
        profile,
        *sidecar.compaction_checkpoint(),
        portable.clone(),
        None,
    )
    .unwrap();
    assert_eq!(restored.export_active_bank().unwrap(), portable);
}

#[test]
fn a_genuine_later_meal_can_reverse_both_directions() {
    for (old, new) in [(-0.9, 0.8), (0.8, -0.9)] {
        let mut bank = empty_bank();
        for (sequence, reward, contact) in [
            (1, old, PhysicalContactKind::Consumed),
            (2, 0.01, PhysicalContactKind::Touch),
            (3, new, PhysicalContactKind::Consumed),
            (4, 0.01, PhysicalContactKind::Touch),
        ] {
            let patch = meal_event(
                &bank,
                sequence,
                contact,
                reward,
                if reward < 0. { 0.9 } else { 0. },
            );
            bank.observe_sealed_patch(&patch).unwrap();
        }
        let value = expected_meal(&bank, 5);
        assert_eq!(value.is_sign_negative(), new.is_sign_negative());
        assert!(value.abs() > 0.7);
    }
}

#[test]
fn preparation_still_recalls_before_the_first_meal_and_does_not_merge_with_it() {
    let mut bank = empty_bank();
    for sequence in 1..=2 {
        let patch = meal_event(&bank, sequence, PhysicalContactKind::Touch, 0.01, 0.);
        bank.observe_sealed_patch(&patch).unwrap();
    }
    assert_eq!(bank.len(), 1);
    assert!(expected_meal(&bank, 3) > 0.);
    let patch = meal_event(&bank, 3, PhysicalContactKind::Consumed, 0.02, 0.);
    bank.observe_sealed_patch(&patch).unwrap();
    assert_eq!(
        bank.len(),
        2,
        "numerically compatible distinct phases must not merge"
    );
    let records = serde_json::to_value(&bank).unwrap();
    let contacts = records["candidate_store"]["records"]
        .as_object()
        .unwrap()
        .values()
        .map(|r| r["outcome_contact"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(contacts, vec!["Touch", "Consumed"]);
}

#[test]
fn legacy_unknown_meal_survives_new_preparation_but_can_be_reversed_by_real_consumption() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, -0.9, 0.9))
        .unwrap();
    let before = serde_json::to_value(&bank).unwrap()["candidate_store"]["records"]["1"].clone();
    let pickup = meal_event(&bank, 2, PhysicalContactKind::Touch, 0.01, 0.);
    bank.observe_sealed_patch(&pickup).unwrap();
    assert!(expected_meal(&bank, 3) < -0.8);
    assert_eq!(
        serde_json::to_value(&bank).unwrap()["candidate_store"]["records"]["1"],
        before
    );
    let safe = meal_event(&bank, 3, PhysicalContactKind::Consumed, 0.8, 0.);
    bank.observe_sealed_patch(&safe).unwrap();
    assert!(expected_meal(&bank, 4) > 0.7);
}

#[test]
fn preparation_at_capacity_cannot_take_the_direct_consumption_reversal_shortcut() {
    let mut bank = MemoryBank::new(
        MemoryBankConfig::new(2, 64, 2, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    )
    .unwrap();
    let meal = meal_event(&bank, 1, PhysicalContactKind::Consumed, -0.9, 0.9);
    bank.observe_sealed_patch(&meal).unwrap();
    let other = observation_patch(
        &bank,
        2,
        2,
        slot(0, 72, 0.4, [0.2, 0.5, 0.8]),
        CandidateActionFamily::Ingest,
        -0.9,
        0.9,
        |outcome| {
            outcome.physical.target_entity = Some(WorldEntityId(9000));
        },
    );
    bank.observe_sealed_patch(&other).unwrap();
    let pickup = meal_event(&bank, 3, PhysicalContactKind::Touch, 0.01, 0.);
    let receipt = bank.observe_sealed_patch(&pickup).unwrap();
    assert_eq!(
        receipt.kind,
        alife_core::MemoryUpdateKind::IgnoredLowerRetention
    );
    assert!(expected_meal(&bank, 4) < -0.8);
    let reversal = meal_event(&bank, 4, PhysicalContactKind::Consumed, 0.8, 0.);
    let receipt = bank.observe_sealed_patch(&reversal).unwrap();
    assert!(matches!(
        receipt.kind,
        alife_core::MemoryUpdateKind::Evicted {
            removed: alife_core::MemoryId(1),
            ..
        }
    ));
    assert!(expected_meal(&bank, 5) > 0.7);
    assert_eq!(bank.len(), 2);
}

#[test]
fn known_phases_survive_compaction_portable_restore_and_are_authenticated() {
    let config = MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap();
    let mut sidecar = grounded_sidecar(config);
    for (sequence, contact) in [
        (1, PhysicalContactKind::Consumed),
        (2, PhysicalContactKind::Touch),
    ] {
        let patch = meal_event(&empty_bank(), sequence, contact, 0.01, 0.);
        sidecar.observe_sealed_patch(&patch).unwrap();
    }
    let compact = sidecar.prepare_compaction(1, 8, 1).unwrap();
    sidecar.commit_compaction(compact).unwrap();
    let portable = sidecar.export_active_bank().unwrap();
    assert_eq!(portable.records.len(), 2);
    assert_eq!(
        portable.records[0].outcome_contact,
        Some(PhysicalContactKind::Consumed)
    );
    assert_eq!(
        portable.records[1].outcome_contact,
        Some(PhysicalContactKind::Touch)
    );
    let raw = serde_json::to_vec(&portable).unwrap();
    let profile = portable.profile;
    let restored = MemorySidecarState::restore_portable(
        profile,
        *sidecar.compaction_checkpoint(),
        serde_json::from_slice(&raw).unwrap(),
        None,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.export_active_bank().unwrap()).unwrap(),
        raw
    );
    let mut tampered = portable.clone();
    tampered.records[0].outcome_contact = Some(PhysicalContactKind::Touch);
    assert_ne!(
        tampered.records[0].recompute_canonical_digest().unwrap(),
        portable.records[0].canonical_digest
    );
    assert!(MemorySidecarState::restore_portable(
        profile,
        *sidecar.compaction_checkpoint(),
        tampered.clone(),
        None
    )
    .is_err());
    tampered.records[0].canonical_digest =
        tampered.records[0].recompute_canonical_digest().unwrap();
    tampered.canonical_digest = tampered.recompute_canonical_digest().unwrap();
    // Rehashing the portable wrapper cannot change the authenticated active-bank digest.
    assert!(MemorySidecarState::restore_portable(
        profile,
        *sidecar.compaction_checkpoint(),
        tampered,
        None
    )
    .is_err());
}

fn distinct_preparation(bank: &MemoryBank, sequence: u64, variant: u64) -> ExperiencePatch {
    observation_patch(
        bank,
        sequence,
        sequence,
        slot(0, 71, 0.4, [0., 0.8, 0.9]),
        CandidateActionFamily::Ingest,
        0.01,
        0.,
        |outcome| {
            outcome.physical.target_entity = Some(WorldEntityId(9000));
            outcome.physical.contact = PhysicalContactKind::Touch;
            outcome.prediction_error = NormalizedScalar::new(1.).unwrap();
            let mut code = variant;
            let mut values = [0.; 4];
            for value in &mut values {
                *value = [-0.95, 0., 0.95][(code % 3) as usize];
                code /= 3;
            }
            outcome.homeostatic_delta.drives.hunger = values[0];
            outcome.homeostatic_delta.drives.fear = values[1];
            outcome.homeostatic_delta.drives.pain = values[2];
            outcome.homeostatic_delta.drives.curiosity = values[3];
        },
    )
}

#[test]
fn many_preparations_cannot_hide_a_consumption_from_bounded_runtime_or_write_search() {
    let mut bank = MemoryBank::new(
        MemoryBankConfig::new(82, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    )
    .unwrap();
    let meal = meal_event(&bank, 1, PhysicalContactKind::Consumed, -0.9, 0.9);
    bank.observe_sealed_patch(&meal).unwrap();
    for variant in 0..81 {
        let patch = distinct_preparation(&bank, variant + 2, variant);
        bank.observe_sealed_patch(&patch).unwrap();
    }
    assert_eq!(bank.len(), 82);
    let draft = probe_for_object(84, slot(0, 71, 0.4, [0., 0.8, 0.9]));
    let (recall, _) = bank
        .recall_frame_runtime(&draft, &Default::default(), true)
        .unwrap();
    assert!(recall.context().candidates[0].family_value[0] < -0.8);
    assert!(
        recall.receipt().similarity_evaluations <= 2 * alife_core::RUNTIME_MEMORY_SEARCH_CAP as u32
    );
    let reversal = meal_event(&bank, 84, PhysicalContactKind::Consumed, 0.8, 0.);
    let receipt = bank.observe_sealed_patch(&reversal).unwrap();
    assert!(matches!(
        receipt.kind,
        alife_core::MemoryUpdateKind::Evicted {
            removed: alife_core::MemoryId(1),
            ..
        }
    ));
    assert!(expected_meal(&bank, 85) > 0.7);
}

#[test]
fn many_newer_preparations_do_not_hide_a_real_reversal_from_sleep_compaction() {
    let mut sidecar = grounded_sidecar(
        MemoryBankConfig::new(128, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    );
    sidecar
        .observe_sealed_patch(&meal_event(
            &empty_bank(),
            1,
            PhysicalContactKind::Consumed,
            -1.,
            1.,
        ))
        .unwrap();
    sidecar
        .observe_sealed_patch(&meal_event(
            &empty_bank(),
            2,
            PhysicalContactKind::Consumed,
            0.8,
            0.,
        ))
        .unwrap();
    for variant in 0..81 {
        sidecar
            .observe_sealed_patch(&distinct_preparation(&empty_bank(), variant + 3, variant))
            .unwrap();
    }
    assert_eq!(sidecar.export_active_bank().unwrap().records.len(), 83);
    let compact = sidecar.prepare_compaction(1, 2, 1).unwrap();
    sidecar.commit_compaction(compact).unwrap();
    let portable = sidecar.export_active_bank().unwrap();
    assert_eq!(portable.records.len(), 2);
    assert!(!portable.records.iter().any(|r| r.memory_id_raw == 1));
    assert!(portable.records.iter().any(|r| r.memory_id_raw == 2));
}

#[test]
fn same_tick_consumption_reversal_uses_causal_sequence_order() {
    let mut bank = empty_bank();
    for (sequence, tick, reward) in [(1, 1, 0.8), (2, 2, -0.9), (3, 2, 0.8)] {
        let patch = observation_patch(
            &bank,
            sequence,
            tick,
            slot(0, 71, 0.4, [0., 0.8, 0.9]),
            CandidateActionFamily::Ingest,
            reward,
            if reward < 0. { 0.9 } else { 0. },
            |outcome| {
                outcome.physical.target_entity = Some(WorldEntityId(9000));
            },
        );
        bank.observe_sealed_patch(&patch).unwrap();
    }
    assert!(expected_meal(&bank, 3) > 0.7);
}

#[test]
fn other_families_keep_unknown_provenance_and_legacy_outcome_selection() {
    let mut bank = empty_bank();
    for (sequence, contact, reward) in [
        (1, PhysicalContactKind::Consumed, -0.9),
        (2, PhysicalContactKind::Touch, 0.8),
    ] {
        let patch = observation_patch(
            &bank,
            sequence,
            sequence,
            slot(0, 71, 0.4, [0., 0.8, 0.9]),
            CandidateActionFamily::Contact,
            reward,
            0.,
            |outcome| {
                outcome.physical.target_entity = Some(WorldEntityId(9000));
                outcome.physical.contact = contact;
            },
        );
        bank.observe_sealed_patch(&patch).unwrap();
    }
    let probe = observation_patch(
        &bank,
        3,
        3,
        slot(0, 71, 0.4, [0., 0.8, 0.9]),
        CandidateActionFamily::Contact,
        0.,
        0.,
        |_| {},
    );
    assert!(probe.pre_action().perception().context().values()[8] > 0.7);
    let value = serde_json::to_value(&bank).unwrap();
    assert!(value["candidate_store"]["records"]
        .as_object()
        .unwrap()
        .values()
        .all(|r| r.get("outcome_contact").is_none()));
}

#[test]
fn provenance_outside_supported_tracked_ingest_scope_is_rejected() {
    let mut bank = empty_bank();
    let patch = observation_patch(
        &bank,
        1,
        1,
        slot(0, 71, 0.4, [0., 0.8, 0.9]),
        CandidateActionFamily::Contact,
        0.8,
        0.,
        |_| {},
    );
    bank.observe_sealed_patch(&patch).unwrap();
    let mut value = serde_json::to_value(&bank).unwrap();
    value["candidate_store"]["records"]["1"]["outcome_contact"] = serde_json::json!("Consumed");
    assert!(serde_json::from_value::<MemoryBank>(value).is_err());
}

#[test]
fn untracked_ingest_keeps_legacy_unknown_reversal_rules() {
    let mut bank = empty_bank();
    for (sequence, contact, reward) in [
        (1, PhysicalContactKind::Consumed, -0.9),
        (2, PhysicalContactKind::Touch, 0.8),
    ] {
        let patch = observation_from_draft(
            &bank,
            sequence,
            privileged_runtime_draft(Tick::new(sequence), &[(9000, 0.4)]),
            reward,
            0.,
            |outcome| {
                outcome.physical.target_entity = Some(WorldEntityId(9000));
                outcome.physical.contact = contact;
            },
        );
        bank.observe_sealed_patch(&patch).unwrap();
    }
    let value = serde_json::to_value(&bank).unwrap();
    assert!(value["candidate_store"]["records"]
        .as_object()
        .unwrap()
        .values()
        .all(|r| r.get("outcome_contact").is_none()));
    let recall = bank
        .recall_frame(&privileged_runtime_draft(Tick::new(3), &[(9000, 0.4)]))
        .unwrap();
    assert!(recall.context().candidates[0].family_value[0] > 0.7);
}

#[test]
fn tracked_object_slot_without_a_concrete_action_target_never_gains_contact_provenance() {
    let mut bank = empty_bank();
    let source = probe_for_object(1, slot(0, 71, 0.4, [0., 0.8, 0.9]));
    let mut candidate = source.candidates()[0];
    candidate.target.entity = None;
    let draft = PerceptionFrameDraft::new(
        ORGANISM,
        source.tick(),
        source.sensor_profile(),
        source.sensory().clone(),
        source.body(),
        HomeostaticSnapshot::baseline(source.tick()),
        vec![candidate],
        source.profile_provenance(),
        source.grounded_object_slots().to_vec(),
    )
    .unwrap();
    let patch = observation_from_draft(&bank, 1, draft, -0.9, 0.9, |outcome| {
        outcome.physical.target_entity = None;
        outcome.physical.contact = PhysicalContactKind::Consumed;
    });
    assert_eq!(
        patch
            .decision()
            .episodic_key()
            .unwrap()
            .query()
            .tracked_object_id(),
        Some(TrackedObjectId(71))
    );
    bank.observe_sealed_patch(&patch).unwrap();
    assert!(
        serde_json::to_value(&bank).unwrap()["candidate_store"]["records"]["1"]
            .get("outcome_contact")
            .is_none()
    );
}

#[test]
fn actual_consumption_teaches_a_saturated_legacy_bank_without_relabeling() {
    for (old, new) in [(-0.9, 0.8), (0.9, -0.8)] {
        let mut bank = MemoryBank::new(
            MemoryBankConfig::new(1, 64, 1, 0.72, Confidence::new(0.).unwrap()).unwrap(),
        )
        .unwrap();
        bank.observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, old, 0.9))
            .unwrap();
        let pickup = meal_event(&bank, 2, PhysicalContactKind::Touch, 0.01, 0.);
        assert_eq!(
            bank.observe_sealed_patch(&pickup).unwrap().kind,
            alife_core::MemoryUpdateKind::IgnoredLowerRetention
        );
        let meal = meal_event(&bank, 3, PhysicalContactKind::Consumed, new, 0.);
        assert!(matches!(
            bank.observe_sealed_patch(&meal).unwrap().kind,
            alife_core::MemoryUpdateKind::Evicted {
                removed: alife_core::MemoryId(1),
                ..
            }
        ));
        assert_eq!(
            expected_meal(&bank, 4).is_sign_negative(),
            new.is_sign_negative()
        );
    }
}

#[test]
fn compaction_cannot_resurrect_unknown_harm_over_a_newer_grounded_safe_meal() {
    let mut sidecar = grounded_sidecar(
        MemoryBankConfig::new(4, 64, 2, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    );
    sidecar
        .observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, -1., 1.))
        .unwrap();
    sidecar
        .observe_sealed_patch(&meal_event(
            &empty_bank(),
            2,
            PhysicalContactKind::Consumed,
            0.6,
            0.,
        ))
        .unwrap();
    let before = sidecar.export_active_bank().unwrap();
    assert_eq!(before.records[0].outcome_contact, None);
    assert_eq!(before.records.len(), 2);
    let compact = sidecar.prepare_compaction(1, 1, 1).unwrap();
    sidecar.commit_compaction(compact).unwrap();
    let after = sidecar.export_active_bank().unwrap();
    assert_eq!(after.records.len(), 1);
    assert_eq!(after.records[0].memory_id_raw, 2);
    assert_eq!(
        after.records[0].outcome_contact,
        Some(PhysicalContactKind::Consumed)
    );
}

#[test]
fn legacy_meal_reversal_keeps_the_existing_nearby_context_similarity_rule() {
    for capacity in [1, 4] {
        let mut sidecar = grounded_sidecar(
            MemoryBankConfig::new(capacity, 64, 1, 0.72, Confidence::new(0.).unwrap()).unwrap(),
        );
        sidecar
            .observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, -1., 1.))
            .unwrap();
        let newer = observation_patch(
            &empty_bank(),
            2,
            2,
            slot(0, 71, 0.40001, [0., 0.8, 0.9]),
            CandidateActionFamily::Ingest,
            0.6,
            0.,
            |outcome| {
                outcome.physical.target_entity = Some(WorldEntityId(9000));
                outcome.physical.contact = PhysicalContactKind::Consumed;
            },
        );
        sidecar.observe_sealed_patch(&newer).unwrap();
        let compact = sidecar.prepare_compaction(1, 1, 1).unwrap();
        sidecar.commit_compaction(compact).unwrap();
        let after = sidecar.export_active_bank().unwrap();
        assert_eq!(after.records.len(), 1);
        assert_eq!(after.records[0].memory_id_raw, 2);
        assert_eq!(
            after.records[0].outcome_contact,
            Some(PhysicalContactKind::Consumed)
        );
    }
}

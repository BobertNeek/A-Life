//! Storage fixtures exercise production recall without inventing a World life.
use super::*;
use alife_core::experience::ChannelPhysicalOutcome;
use alife_core::{BodyEventDelta, ExecutedActionMemoryV1, ExecutedChannelEvidenceV1, MotorChannel};

fn fact(sequence: u64, tracked: u64, consumed: bool) -> ExecutedActionMemoryV1 {
    let draft = probe_for_object(1, slot(0, tracked, 0.4, [0., 0.8, 0.9]));
    let (_, recall) = empty_bank()
        .recall_frame_with_execution_evidence(&draft)
        .unwrap()
        .finalize(draft)
        .unwrap();
    let query = recall.candidate_keys()[0].query().clone();
    let receipt = ChannelPhysicalOutcome::new(
        MotorChannel::Manipulation,
        PhysicalActionOutcome {
            contact: if consumed {
                PhysicalContactKind::Consumed
            } else {
                PhysicalContactKind::Touch
            },
            target_entity: Some(WorldEntityId(9000)),
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
    )
    .unwrap()
    .with_execution_evidence(
        ExecutedChannelEvidenceV1::new(
            query.action_id(),
            Some(WorldEntityId(9000)),
            true,
            BodyEventDelta {
                nutrition: if consumed { 0.6 } else { 0. },
                ..BodyEventDelta::zero()
            },
        )
        .unwrap(),
    )
    .unwrap();
    ExecutedActionMemoryV1 {
        query,
        source_sequence_id: ExperienceSequenceId(sequence),
        event_tick: Tick(2),
        bundle_digest: [1, 2, 3, 4],
        receipt,
    }
}
fn fixture_with_secondary(fact: &ExecutedActionMemoryV1) -> MemoryBank {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, 0.8, 0.))
        .unwrap();
    let mut wire = serde_json::to_value(bank).unwrap();
    wire["candidate_store"]["records"]["1"]["executed_actions_json"] =
        serde_json::json!(serde_json::to_vec(&vec![fact]).unwrap());
    serde_json::from_value(wire).unwrap()
}
#[test]
fn typed_secondary_recall_is_produced_by_audited_and_runtime_paths_without_scalar_relabeling() {
    let fact = fact(1, 72, true);
    let bank = fixture_with_secondary(&fact);
    let probe = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    let prepared = bank.recall_frame_with_execution_evidence(&probe).unwrap();
    assert_eq!(
        prepared.executed_actions()[0].as_ref().unwrap().evidence,
        fact
    );
    let (_, finalized) = prepared.finalize(probe.clone()).unwrap();
    assert_eq!(
        finalized.executed_actions()[0].as_ref().unwrap().evidence,
        fact
    );
    let (runtime, cache) = bank
        .recall_frame_runtime_with_execution_evidence(&probe, &Default::default(), true)
        .unwrap();
    let (cached, _) = bank
        .recall_frame_runtime_with_execution_evidence(&probe, &cache.unwrap(), false)
        .unwrap();
    assert_eq!(runtime.executed_actions(), finalized.executed_actions());
    assert_eq!(cached.executed_actions(), finalized.executed_actions());
    assert!(cached.receipt().execution_evidence_work > 0);
    assert_eq!(
        cached.receipt().execution_evidence_work,
        runtime.receipt().execution_evidence_work
    );
    let other = bank
        .recall_frame_with_execution_evidence(&probe_for_object(
            3,
            slot(0, 71, 0.4, [0., 0.8, 0.9]),
        ))
        .unwrap();
    assert!(other.executed_actions()[0].is_none());
    let wire = serde_json::to_value(&bank).unwrap();
    assert_eq!(
        wire["candidate_store"]["records"]["1"]["tracked_object_id_raw"],
        71
    );
    assert_eq!(
        wire["candidate_store"]["records"]["1"]["observation_count"],
        1
    );
    let bytes = serde_json::to_vec(&bank).unwrap();
    let restored: MemoryBank = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    assert_eq!(
        restored
            .recall_frame_with_execution_evidence(&probe)
            .unwrap()
            .executed_actions(),
        finalized.executed_actions()
    );
}
#[test]
fn unknown_merge_does_not_upgrade_the_actual_execution_sequence() {
    let measured = fact(1, 71, true);
    let mut bank = fixture_with_secondary(&measured);
    bank.observe_sealed_patch(&sequenced_patch(2, 2, 71, 0.4, 0.8, 0.))
        .unwrap();
    let recalled = bank
        .recall_frame_with_execution_evidence(&probe_for_object(
            3,
            slot(0, 71, 0.4, [0., 0.8, 0.9]),
        ))
        .unwrap();
    assert_eq!(
        recalled.executed_actions()[0]
            .as_ref()
            .unwrap()
            .evidence
            .source_sequence_id,
        ExperienceSequenceId(1)
    );
    assert!(bank
        .observe_sealed_patch(&sequenced_patch(2, 2, 71, 0.4, 0.8, 0.))
        .is_err());
}
#[test]
fn typed_metadata_is_authenticated_and_invalid_or_future_facts_are_rejected() {
    let config = MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap();
    let mut sidecar = grounded_sidecar(config);
    sidecar
        .observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, 0.8, 0.))
        .unwrap();
    let mut portable = sidecar.export_active_bank().unwrap();
    let original = portable.records[0].canonical_digest;
    portable.records[0].executed_actions_json =
        Some(serde_json::to_vec(&vec![fact(1, 72, true)]).unwrap());
    assert_ne!(
        portable.records[0].recompute_canonical_digest().unwrap(),
        original
    );
    assert!(MemorySidecarState::restore_portable(
        sidecar.profile(),
        *sidecar.compaction_checkpoint(),
        portable,
        None
    )
    .is_err());
    let mut wire = serde_json::to_value(fixture_with_secondary(&fact(1, 72, true))).unwrap();
    let mut bad = fact(1, 72, true);
    bad.receipt.execution_evidence.as_mut().unwrap().primitive = ActionId(9999);
    wire["candidate_store"]["records"]["1"]["executed_actions_json"] =
        serde_json::json!(serde_json::to_vec(&vec![bad]).unwrap());
    assert!(serde_json::from_value::<MemoryBank>(wire.clone()).is_err());
    wire["candidate_store"]["records"]["1"]["executed_actions_json"] =
        serde_json::json!(serde_json::to_vec(&vec![fact(2, 72, true)]).unwrap());
    assert!(serde_json::from_value::<MemoryBank>(wire).is_err());
}

#[test]
fn empty_bundle_proof_and_query_before_record_interval_are_rejected() {
    let mut wire = serde_json::to_value(fixture_with_secondary(&fact(1, 72, true))).unwrap();
    let mut invalid = fact(1, 72, true);
    invalid.bundle_digest = [0; 4];
    wire["candidate_store"]["records"]["1"]["executed_actions_json"] =
        serde_json::json!(serde_json::to_vec(&vec![invalid]).unwrap());
    assert!(serde_json::from_value::<MemoryBank>(wire).is_err());
    let mut wire = serde_json::to_value(fixture_with_secondary(&fact(1, 72, true))).unwrap();
    wire["candidate_store"]["records"]["1"]["first_tick"] = serde_json::json!(2);
    wire["candidate_store"]["records"]["1"]["last_tick"] = serde_json::json!(2);
    assert!(serde_json::from_value::<MemoryBank>(wire).is_err());
}

#[test]
fn execution_lookup_is_lazy_and_offboard_inspection_never_debits_or_mutates() {
    let bank = fixture_with_secondary(&fact(1, 72, true));
    let before = serde_json::to_vec(&bank).unwrap();
    let draft = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    let ordinary = bank.recall_frame(&draft).unwrap();
    let sourced = bank.recall_frame_with_family_sources(&draft).unwrap();
    let (fresh, cache) = bank
        .recall_frame_runtime(&draft, &Default::default(), true)
        .unwrap();
    let (cached, _) = bank
        .recall_frame_runtime(&draft, &cache.unwrap(), false)
        .unwrap();
    for recall in [&ordinary, &sourced, &fresh, &cached] {
        assert!(recall.executed_actions().iter().all(Option::is_none));
        assert_eq!(recall.executed_actions().len(), draft.candidates().len());
        assert_eq!(recall.receipt().execution_evidence_work, 0);
    }
    let cognitive = bank.recall_frame_with_execution_evidence(&draft).unwrap();
    assert!(cognitive.receipt().execution_evidence_work > 0);
    assert_eq!(ordinary.context(), cognitive.context());
    let (plain_frame, finalized) = ordinary.finalize(draft.clone()).unwrap();
    let (explicit_frame, _) = cognitive.clone().finalize(draft.clone()).unwrap();
    assert_eq!(plain_frame, explicit_frame);
    let receipt_before = finalized.receipt().clone();
    let inspected = bank.inspect_execution_evidence(&finalized).unwrap();
    assert_eq!(inspected.rows, cognitive.executed_actions());
    assert_eq!(
        inspected.diagnostic_work_units,
        cognitive.receipt().execution_evidence_work
    );
    assert_eq!(finalized.receipt(), &receipt_before);
    assert_eq!(finalized.receipt().execution_evidence_work, 0);
    assert_eq!(serde_json::to_vec(&bank).unwrap(), before);
    let (_, finalized_cached) = cached.finalize(draft).unwrap();
    assert_eq!(
        bank.inspect_execution_evidence(&finalized_cached).unwrap(),
        inspected
    );
}

#[test]
fn offboard_sidecar_inspection_checks_owner_and_does_not_change_legacy_bank() {
    let config = MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap();
    let sidecar = grounded_sidecar(config);
    let draft = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    let (_, finalized) = sidecar
        .recall_frame(&draft)
        .unwrap()
        .finalize(draft.clone())
        .unwrap();
    let before = sidecar.export_active_bank().unwrap();
    let inspected = sidecar.inspect_execution_evidence(&finalized).unwrap();
    assert!(inspected.rows.iter().all(Option::is_none));
    assert_eq!(inspected.diagnostic_work_units, 0);
    assert_eq!(sidecar.export_active_bank().unwrap(), before);
    let other =
        MemorySidecarState::new_profiled(OrganismId(812), sidecar.profile(), config).unwrap();
    assert!(other.inspect_execution_evidence(&finalized).is_err());
    assert!(other.recall_frame_with_execution_evidence(&draft).is_err());
    assert!(other
        .recall_frame_runtime_with_execution_evidence(&draft, &Default::default(), true)
        .is_err());
}

#[test]
fn checked_fact_cache_preserves_original_bytes_across_cold_load_warm_reads_and_merge() {
    let mut wire = serde_json::to_value(fixture_with_secondary(&fact(1, 71, true))).unwrap();
    // Noncanonical JSON whitespace is authenticated input, not disposable cache.
    let original = format!(
        "  {}\n",
        serde_json::to_string_pretty(&vec![fact(1, 71, true)]).unwrap()
    )
    .into_bytes();
    wire["candidate_store"]["records"]["1"]["executed_actions_json"] = serde_json::json!(original);
    let raw = serde_json::to_vec(&wire).unwrap();
    let mut bank: MemoryBank = serde_json::from_slice(&raw).unwrap();
    let canonical_before = serde_json::to_vec(&bank).unwrap();
    let draft = probe_for_object(3, slot(0, 71, 0.4, [0., 0.8, 0.9]));
    let digest = bank.recall_frame(&draft).unwrap().receipt().bank_digest;
    for _ in 0..3 {
        let default = bank.recall_frame(&draft).unwrap();
        assert_eq!(default.receipt().bank_digest, digest);
        assert_eq!(default.receipt().execution_evidence_work, 0);
        assert!(default.executed_actions().iter().all(Option::is_none));
        let explicit = bank.recall_frame_with_execution_evidence(&draft).unwrap();
        assert_eq!(
            explicit.executed_actions()[0]
                .as_ref()
                .unwrap()
                .evidence
                .receipt
                .physical
                .contact,
            PhysicalContactKind::Consumed
        );
        assert_eq!(serde_json::to_vec(&bank).unwrap(), canonical_before);
    }
    bank.observe_sealed_patch(&sequenced_patch(2, 2, 71, 0.4, 0.8, 0.))
        .unwrap();
    let merged = serde_json::to_value(&bank).unwrap();
    assert_eq!(
        merged["candidate_store"]["records"]["1"]["executed_actions_json"],
        serde_json::json!(original)
    );
    assert_eq!(
        bank.recall_frame_with_execution_evidence(&draft)
            .unwrap()
            .executed_actions()[0]
            .as_ref()
            .unwrap()
            .evidence
            .source_sequence_id,
        ExperienceSequenceId(1)
    );
    let restored: MemoryBank = serde_json::from_value(merged).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored).unwrap(),
        serde_json::to_vec(&bank).unwrap()
    );
    assert_eq!(
        restored.recall_frame(&draft).unwrap().receipt().bank_digest,
        bank.recall_frame(&draft).unwrap().receipt().bank_digest
    );
}

#[test]
fn cold_changed_fact_bytes_cannot_reuse_a_stale_checked_cache() {
    let original = fixture_with_secondary(&fact(1, 72, true));
    let mut wire = serde_json::to_value(&original).unwrap();
    wire["candidate_store"]["records"]["1"]["executed_actions_json"] =
        serde_json::json!(serde_json::to_vec(&vec![fact(1, 72, false)]).unwrap());
    let changed: MemoryBank = serde_json::from_value(wire.clone()).unwrap();
    let draft = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    assert_eq!(
        original
            .recall_frame_with_execution_evidence(&draft)
            .unwrap()
            .executed_actions()[0]
            .as_ref()
            .unwrap()
            .evidence
            .receipt
            .physical
            .contact,
        PhysicalContactKind::Consumed
    );
    assert_eq!(
        changed
            .recall_frame_with_execution_evidence(&draft)
            .unwrap()
            .executed_actions()[0]
            .as_ref()
            .unwrap()
            .evidence
            .receipt
            .physical
            .contact,
        PhysicalContactKind::Touch
    );
    assert_ne!(
        original.recall_frame(&draft).unwrap().receipt().bank_digest,
        changed.recall_frame(&draft).unwrap().receipt().bank_digest
    );
    wire["candidate_store"]["records"]["1"]["executed_actions_json"] =
        serde_json::json!(b"not valid facts".to_vec());
    assert!(serde_json::from_value::<MemoryBank>(wire).is_err());
}

#[test]
fn trusted_internal_export_equals_cold_public_checksums_and_mutation_still_revalidates() {
    let config = MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap();
    let mut source = grounded_sidecar(config);
    source
        .observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, 0.8, 0.))
        .unwrap();
    let mut portable = source.export_active_bank().unwrap();
    let facts = serde_json::to_vec_pretty(&vec![fact(1, 72, true)]).unwrap();
    portable.records[0].executed_actions_json = Some(facts.clone());
    portable.records[0].canonical_digest =
        portable.records[0].recompute_canonical_digest().unwrap();
    let mut raw = serde_json::to_value(fixture_with_secondary(&fact(1, 72, true))).unwrap();
    raw["candidate_store"]["records"]["1"]["executed_actions_json"] = serde_json::json!(facts);
    let bank: MemoryBank = serde_json::from_value(raw).unwrap();
    let draft = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    portable.active_bank_digest = bank.recall_frame(&draft).unwrap().receipt().bank_digest;
    portable.canonical_digest = portable.recompute_canonical_digest().unwrap();
    portable.validate_contract().unwrap();
    let mut checkpoint = *source.compaction_checkpoint();
    checkpoint.active_digest = portable.active_bank_digest;
    let restored =
        MemorySidecarState::restore_portable(source.profile(), checkpoint, portable.clone(), None)
            .unwrap();
    let exported = restored.export_active_bank().unwrap();
    assert_eq!(exported, portable);
    assert_eq!(
        serde_json::to_vec(&exported).unwrap(),
        serde_json::to_vec(&portable).unwrap()
    );
    for record in &exported.records {
        assert_eq!(
            record.canonical_digest,
            record.recompute_canonical_digest().unwrap()
        );
    }
    assert_eq!(
        exported.canonical_digest,
        exported.recompute_canonical_digest().unwrap()
    );
    exported.validate_contract().unwrap();
    let mut mutated = exported;
    mutated.records[0]
        .executed_actions_json
        .as_mut()
        .unwrap()
        .push(b' ');
    assert_ne!(
        mutated.records[0].canonical_digest,
        mutated.records[0].recompute_canonical_digest().unwrap()
    );
    assert!(mutated.validate_contract().is_err());
    mutated.records[0].executed_actions_json = Some(b"malformed".to_vec());
    assert!(mutated.records[0].recompute_canonical_digest().is_err());
    assert!(mutated.validate_contract().is_err());
}

#[test]
fn cached_facts_survive_compaction_rebuild_and_leave_no_stale_index_after_eviction() {
    let config = MemoryBankConfig::new(2, 64, 2, 0.72, Confidence::new(0.).unwrap()).unwrap();
    let mut source = grounded_sidecar(config);
    source
        .observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, 0.8, 0.))
        .unwrap();
    let mut portable = source.export_active_bank().unwrap();
    let bytes = serde_json::to_vec(&vec![fact(1, 72, true)]).unwrap();
    portable.records[0].executed_actions_json = Some(bytes.clone());
    portable.records[0].canonical_digest =
        portable.records[0].recompute_canonical_digest().unwrap();
    // Start from the real capacity-two bank, including its matching empty
    // legacy storage vector; inject only the optional authenticated fact bytes.
    let mut raw = serde_json::to_value(source.bank()).unwrap();
    raw["candidate_store"]["records"]["1"]["executed_actions_json"] = serde_json::json!(bytes);
    let bank: MemoryBank = serde_json::from_value(raw).unwrap();
    let draft = probe_for_object(10, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    portable.active_bank_digest = bank.recall_frame(&draft).unwrap().receipt().bank_digest;
    portable.canonical_digest = portable.recompute_canonical_digest().unwrap();
    let mut checkpoint = *source.compaction_checkpoint();
    checkpoint.active_digest = portable.active_bank_digest;
    let mut restored =
        MemorySidecarState::restore_portable(source.profile(), checkpoint, portable, None).unwrap();
    let initial = restored
        .recall_frame_with_execution_evidence(&draft)
        .unwrap()
        .executed_actions()
        .to_vec();
    assert!(initial[0].is_some());
    let prepared = restored.prepare_compaction(1, 1, 7).unwrap();
    restored.commit_compaction(prepared).unwrap();
    assert_eq!(
        restored
            .recall_frame_with_execution_evidence(&draft)
            .unwrap()
            .executed_actions(),
        initial
    );
    let compacted = restored.export_active_bank().unwrap();
    assert_eq!(
        compacted.records[0].executed_actions_json.as_ref(),
        Some(&bytes)
    );
    let mut restored = MemorySidecarState::restore_portable(
        restored.profile(),
        *restored.compaction_checkpoint(),
        compacted,
        None,
    )
    .unwrap();
    assert_eq!(
        restored
            .recall_frame_with_execution_evidence(&draft)
            .unwrap()
            .executed_actions(),
        initial
    );
    for (sequence, target) in [(2, 90), (3, 91), (4, 92)] {
        restored
            .observe_sealed_patch(&sequenced_patch(
                sequence, sequence, target, 0.4, -0.99, 0.99,
            ))
            .unwrap();
    }
    let evicted = restored.export_active_bank().unwrap();
    assert!(evicted.eviction_count > 0);
    assert!(evicted
        .records
        .iter()
        .all(|record| record.executed_actions_json.is_none()));
    assert!(restored
        .recall_frame_with_execution_evidence(&draft)
        .unwrap()
        .executed_actions()[0]
        .is_none());
    let cold = MemorySidecarState::restore_portable(
        restored.profile(),
        *restored.compaction_checkpoint(),
        evicted,
        None,
    )
    .unwrap();
    assert!(cold
        .recall_frame_with_execution_evidence(&draft)
        .unwrap()
        .executed_actions()[0]
        .is_none());
}

// Storage-only joint association fixtures; the native host regression separately
// proves capture from actual sealed World execution, without editing saved lives.
fn joint_fixture() -> MemoryBank {
    let f = fact(1, 72, true);
    let bank = fixture_with_secondary(&f);
    let mut wire = serde_json::to_value(bank).unwrap();
    let event = serde_json::json!({"schema_version":1,"sealed_causal_digest":[4,3,2,1],
        "members":serde_json::to_vec(&vec![f]).unwrap(),
        "shared_family_value":[-0.6,0.0,0.7,-0.1],"event_confidence":0.3});
    wire["candidate_store"]["records"]["1"]["schema_version"] = serde_json::json!(3);
    wire["candidate_store"]["records"]["1"]["joint_event"] =
        serde_json::json!(serde_json::to_vec(&event).unwrap());
    serde_json::from_value(wire).unwrap()
}
#[test]
fn joint_membership_retrieves_event_not_averaged_representative_and_leaves_default_alone() {
    let bank = joint_fixture();
    let probe = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    let plain = bank.recall_frame(&probe).unwrap();
    let explicit = bank.recall_frame_with_joint_membership(&probe).unwrap();
    assert_eq!(plain.receipt().schema_version, 2);
    assert_eq!(explicit.receipt().schema_version, 3);
    assert!(plain.receipt().joint_membership_sources.is_empty());
    let c = &explicit.context().candidates[0];
    assert_eq!(c.family_value, [-0.6, 0., 0.7, -0.1]);
    assert!(c.family_confidence.raw() <= 0.3);
    assert_eq!(c.family_source_count, 1);
    assert_eq!(c.target_latent, plain.context().candidates[0].target_latent);
    assert_eq!(
        c.target_confidence,
        plain.context().candidates[0].target_confidence
    );
    assert!(explicit.receipt().execution_evidence_work > 0);
    explicit.finalize(probe).unwrap();
    let wrong = bank
        .recall_frame_with_joint_membership(&probe_for_object(3, slot(0, 71, 0.4, [0., 0.8, 0.9])))
        .unwrap();
    assert!(wrong.receipt().joint_membership_sources.is_empty());
    let legacy = fixture_with_secondary(&fact(1, 72, true));
    assert!(legacy
        .recall_frame_with_joint_membership(&probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9])))
        .unwrap()
        .receipt()
        .joint_membership_sources
        .is_empty());
}
#[test]
fn joint_event_cold_roundtrip_and_merge_keep_exact_original_confidence_and_sequence() {
    let mut bank = joint_fixture();
    let raw = serde_json::to_vec(&bank).unwrap();
    assert_eq!(
        serde_json::to_vec(&serde_json::from_slice::<MemoryBank>(&raw).unwrap()).unwrap(),
        raw
    );
    let before = serde_json::to_value(&bank).unwrap()["candidate_store"]["records"]["1"]
        ["joint_event"]
        .clone();
    bank.observe_sealed_patch(&sequenced_patch(2, 2, 71, 0.4, 0.8, 0.))
        .unwrap();
    let wire = serde_json::to_value(&bank).unwrap();
    assert_eq!(
        wire["candidate_store"]["records"]["1"]["observation_count"],
        2
    );
    assert_eq!(
        wire["candidate_store"]["records"]["1"]["joint_event"],
        before
    );
    let restored: MemoryBank = serde_json::from_value(wire).unwrap();
    let recall = restored
        .recall_frame_with_joint_membership(&probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9])))
        .unwrap();
    assert_eq!(
        recall.receipt().joint_membership_sources[0].source_sequence_id,
        ExperienceSequenceId(1)
    );
    assert_eq!(
        recall.context().candidates[0].family_value,
        [-0.6, 0., 0.7, -0.1]
    );
}
#[test]
fn joint_event_invalid_owner_version_digest_and_duplicate_copy_are_rejected() {
    let base = serde_json::to_value(joint_fixture()).unwrap();
    let bytes: Vec<u8> =
        serde_json::from_value(base["candidate_store"]["records"]["1"]["joint_event"].clone())
            .unwrap();
    let event: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for name in ["version", "digest", "owner", "primitive"] {
        let mut bad = event.clone();
        match name {
            "version" => bad["schema_version"] = serde_json::json!(9),
            "digest" => bad["sealed_causal_digest"] = serde_json::json!([0, 0, 0, 0]),
            _ => {
                let m: Vec<u8> = serde_json::from_value(bad["members"].clone()).unwrap();
                let mut members: Vec<ExecutedActionMemoryV1> = serde_json::from_slice(&m).unwrap();
                if name == "primitive" {
                    members[0]
                        .receipt
                        .execution_evidence
                        .as_mut()
                        .unwrap()
                        .primitive = ActionId(999);
                } else {
                    members[0].source_sequence_id = ExperienceSequenceId(99);
                }
                bad["members"] = serde_json::json!(serde_json::to_vec(&members).unwrap());
            }
        }
        let mut wire = base.clone();
        wire["candidate_store"]["records"]["1"]["joint_event"] =
            serde_json::json!(serde_json::to_vec(&bad).unwrap());
        assert!(
            serde_json::from_value::<MemoryBank>(wire).is_err(),
            "{name}"
        );
    }
    let mut downgrade = base.clone();
    downgrade["candidate_store"]["records"]["1"]["schema_version"] = serde_json::json!(2);
    assert!(serde_json::from_value::<MemoryBank>(downgrade).is_err());
    for conflict in [false, true] {
        let mut wire = base.clone();
        let mut duplicate = wire["candidate_store"]["records"]["1"].clone();
        duplicate["memory_id"] = serde_json::json!(2);
        if conflict {
            let mut bad = event.clone();
            bad["shared_family_value"] = serde_json::json!([0.9, 1., 0., 0.1]);
            duplicate["joint_event"] = serde_json::json!(serde_json::to_vec(&bad).unwrap());
        }
        wire["candidate_store"]["records"]["2"] = duplicate;
        wire["candidate_store"]["next_memory_id"] = serde_json::json!(3);
        assert!(serde_json::from_value::<MemoryBank>(wire).is_err());
    }
}

#[test]
fn joint_member_index_finds_old_record_id_behind_many_unrelated_new_records() {
    let small = serde_json::to_value(joint_fixture()).unwrap();
    let config = MemoryBankConfig::new(128, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap();
    let mut wire = serde_json::to_value(MemoryBank::new(config).unwrap()).unwrap();
    wire["candidate_store"] = small["candidate_store"].clone();
    let mut other = small["candidate_store"]["records"]["1"].clone();
    other["schema_version"] = serde_json::json!(2);
    other.as_object_mut().unwrap().remove("joint_event");
    other
        .as_object_mut()
        .unwrap()
        .remove("executed_actions_json");
    for id in 2..=70 {
        let mut row = other.clone();
        row["memory_id"] = serde_json::json!(id);
        row["tracked_object_id_raw"] = serde_json::json!(1000 + id);
        wire["candidate_store"]["records"][id.to_string()] = row;
    }
    wire["candidate_store"]["next_memory_id"] = serde_json::json!(71);
    let bank: MemoryBank = serde_json::from_value(wire).unwrap();
    let recalled = bank
        .recall_frame_with_joint_membership(&probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9])))
        .unwrap();
    assert_eq!(recalled.receipt().joint_membership_sources.len(), 1);
    assert_eq!(
        recalled.receipt().joint_membership_sources[0]
            .memory_id
            .raw(),
        1
    );
    assert_eq!(recalled.receipt().execution_evidence_work, 4); // lookup+record+member+projection
}

#[test]
fn joint_portable_roundtrip_and_rehashed_duplicate_event_fail_closed() {
    let config = MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap();
    let mut source = grounded_sidecar(config);
    source
        .observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, 0.8, 0.))
        .unwrap();
    let bank = joint_fixture();
    let raw = serde_json::to_value(&bank).unwrap();
    let mut portable = source.export_active_bank().unwrap();
    portable.schema_version = 3;
    portable.records[0].schema_version = 3;
    portable.records[0].joint_event_json = Some(
        serde_json::from_value(raw["candidate_store"]["records"]["1"]["joint_event"].clone())
            .unwrap(),
    );
    portable.records[0].executed_actions_json = Some(
        serde_json::from_value(
            raw["candidate_store"]["records"]["1"]["executed_actions_json"].clone(),
        )
        .unwrap(),
    );
    portable.records[0].canonical_digest =
        portable.records[0].recompute_canonical_digest().unwrap();
    portable.active_bank_digest = bank
        .recall_frame(&probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9])))
        .unwrap()
        .receipt()
        .bank_digest;
    portable.canonical_digest = portable.recompute_canonical_digest().unwrap();
    portable.validate_contract().unwrap();
    let mut checkpoint = *source.compaction_checkpoint();
    checkpoint.active_digest = portable.active_bank_digest;
    assert!(MemorySidecarState::restore_portable(
        source.profile(),
        checkpoint,
        portable.clone(),
        None
    )
    .is_err());
    let restored = MemorySidecarState::restore_portable_with_joint_membership(
        source.profile(),
        checkpoint,
        portable.clone(),
        None,
    )
    .unwrap();
    assert!(restored.durable_founder_clone(OrganismId(99)).is_err());
    assert_eq!(restored.export_active_bank().unwrap(), portable);
    let mut duplicate = portable.clone();
    let mut row = duplicate.records[0].clone();
    row.memory_id_raw = 2;
    row.canonical_digest = row.recompute_canonical_digest().unwrap();
    duplicate.records.push(row);
    duplicate.next_memory_id_raw = 3;
    duplicate.canonical_digest = duplicate.recompute_canonical_digest().unwrap();
    assert!(duplicate.validate_contract().is_err());
    let mut downgraded = portable.clone();
    downgraded.schema_version = 2;
    downgraded.canonical_digest = downgraded.recompute_canonical_digest().unwrap();
    assert!(downgraded.validate_contract().is_err());
    let mut corrupt = portable;
    corrupt.records[0]
        .joint_event_json
        .as_mut()
        .unwrap()
        .push(b' ');
    assert!(corrupt.validate_contract().is_err());
}

fn joint_confidence_fixture(confidence: f32) -> MemoryBank {
    let mut wire = serde_json::to_value(joint_fixture()).unwrap();
    let bytes: Vec<u8> =
        serde_json::from_value(wire["candidate_store"]["records"]["1"]["joint_event"].clone())
            .unwrap();
    let mut event: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    event["event_confidence"] = serde_json::json!(confidence);
    wire["candidate_store"]["records"]["1"]["joint_event"] =
        serde_json::json!(serde_json::to_vec(&event).unwrap());
    serde_json::from_value(wire).unwrap()
}

#[test]
fn joint_zero_confidence_secondary_preserves_ordinary_recall() {
    let bank = joint_confidence_fixture(0.0);
    let probe = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    let ordinary = bank.recall_frame(&probe).unwrap();
    let recall = bank.recall_frame_with_joint_membership(&probe).unwrap();
    assert_eq!(recall.context(), ordinary.context());
    assert!(recall.receipt().joint_membership_sources.is_empty());
    assert!(recall.receipt().execution_evidence_work > 0);
    recall.finalize(probe).unwrap();
}

fn joint_confidence_population(confidences: &[f32]) -> MemoryBank {
    let template = serde_json::to_value(joint_fixture()).unwrap();
    let config = MemoryBankConfig::new(128, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap();
    let mut wire = serde_json::to_value(MemoryBank::new(config).unwrap()).unwrap();
    wire["candidate_store"] = template["candidate_store"].clone();
    for (i, confidence) in confidences.iter().enumerate() {
        let id = (i + 1) as u64;
        let mut row = template["candidate_store"]["records"]["1"].clone();
        row["memory_id"] = serde_json::json!(id);
        row["source_sequence_id"] = serde_json::json!(id);
        let bytes: Vec<u8> = serde_json::from_value(row["joint_event"].clone()).unwrap();
        let mut event: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        event["event_confidence"] = serde_json::json!(confidence);
        event["members"] =
            serde_json::json!(serde_json::to_vec(&vec![fact(id, 72, true)]).unwrap());
        row["joint_event"] = serde_json::json!(serde_json::to_vec(&event).unwrap());
        wire["candidate_store"]["records"][id.to_string()] = row;
    }
    wire["candidate_store"]["next_memory_id"] = serde_json::json!(confidences.len() + 1);
    wire["candidate_store"]["last_sequence_by_organism"][ORGANISM.raw().to_string()] =
        serde_json::json!(confidences.len());
    serde_json::from_value(wire).unwrap()
}

#[test]
fn joint_subnormal_support_and_retention_use_representability_not_epsilon() {
    let tiny = f32::from_bits(1);
    for (confidence, tick, usable) in [
        (tiny, 3, true),
        (tiny, 1_000_000, false),
        (f32::from_bits(16), 1_000_000, true),
        (f32::MIN_POSITIVE, 1_000_000, true),
    ] {
        let bank = joint_confidence_fixture(confidence);
        let bytes = serde_json::to_vec(&bank).unwrap();
        let cold: MemoryBank = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_vec(&cold).unwrap(), bytes);
        let probe = probe_for_object(tick, slot(0, 72, 0.4, [0., 0.8, 0.9]));
        let plain = bank.recall_frame(&probe).unwrap();
        let explicit = cold.recall_frame_with_joint_membership(&probe).unwrap();
        assert_eq!(
            !explicit.receipt().joint_membership_sources.is_empty(),
            usable
        );
        if usable {
            assert!(explicit.context().candidates[0].family_confidence.raw() > 0.0);
        } else {
            assert_eq!(explicit.context(), plain.context());
        }
        explicit.finalize(probe).unwrap();
    }
}

#[test]
fn joint_unusable_new_events_do_not_crowd_positive_top_k() {
    let mut values = vec![0.3];
    values.extend([
        0.,
        f32::from_bits(1),
        0.,
        f32::from_bits(1),
        0.,
        f32::from_bits(1),
    ]);
    let bank = joint_confidence_population(&values);
    let probe = probe_for_object(1_000_000, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    let recall = bank.recall_frame_with_joint_membership(&probe).unwrap();
    assert_eq!(recall.receipt().joint_membership_sources.len(), 1);
    assert_eq!(
        recall.receipt().joint_membership_sources[0].source_sequence_id,
        ExperienceSequenceId(1)
    );
    assert_eq!(recall.context().candidates[0].family_source_count, 1);
    assert!(recall.receipt().execution_evidence_work > 2 * values.len() as u64);
    recall.finalize(probe).unwrap();
}

#[test]
fn joint_four_smallest_positive_events_avoid_weighting_underflow() {
    let bank = joint_confidence_population(&[f32::from_bits(1); 4]);
    let probe = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    let recall = bank.recall_frame_with_joint_membership(&probe).unwrap();
    assert_eq!(recall.receipt().joint_membership_sources.len(), 4);
    assert_eq!(
        recall.context().candidates[0].family_confidence.raw(),
        f32::from_bits(1)
    );
    recall.finalize(probe).unwrap();
}

#[test]
fn joint_zero_event_survives_merge_and_fallback_without_storage_rewrite() {
    let mut bank = joint_confidence_fixture(0.);
    bank.observe_sealed_patch(&sequenced_patch(2, 2, 71, 0.4, 0.8, 0.))
        .unwrap();
    let before = serde_json::to_vec(&bank).unwrap();
    let probe = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    let plain = bank.recall_frame(&probe).unwrap();
    let explicit = bank.recall_frame_with_joint_membership(&probe).unwrap();
    assert_eq!(plain.context(), explicit.context());
    assert_eq!(explicit.receipt().execution_evidence_work, 3);
    assert!(explicit.receipt().joint_membership_sources.is_empty());
    assert_eq!(serde_json::to_vec(&bank).unwrap(), before);
    explicit.finalize(probe).unwrap();
}

#[test]
fn joint_positive_sources_order_and_context_digest_are_deterministic() {
    let bank = joint_confidence_population(&[0.2, 0.3, 0.4, 0.5, 0.6]);
    let probe = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    let a = bank.recall_frame_with_joint_membership(&probe).unwrap();
    let bytes = serde_json::to_vec(&bank).unwrap();
    let cold: MemoryBank = serde_json::from_slice(&bytes).unwrap();
    let b = cold.recall_frame_with_joint_membership(&probe).unwrap();
    assert_eq!(a.context(), b.context());
    assert_eq!(a.receipt(), b.receipt());
    assert_eq!(
        a.receipt()
            .joint_membership_sources
            .iter()
            .map(|x| x.source_sequence_id.raw())
            .collect::<Vec<_>>(),
        vec![5, 4, 3, 2]
    );
    a.finalize(probe).unwrap();
}

#[test]
fn joint_zero_search_keeps_existing_bounded_recent_search_policy() {
    let mut values = vec![0.3];
    values.extend([0.; 64]);
    let bank = joint_confidence_population(&values);
    let probe = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
    let ordinary = bank.recall_frame(&probe).unwrap();
    let recall = bank.recall_frame_with_joint_membership(&probe).unwrap();
    assert_eq!(recall.context(), ordinary.context());
    assert!(recall.receipt().joint_membership_sources.is_empty());
    assert_eq!(recall.receipt().execution_evidence_work, 129);
}

#[test]
fn joint_comparison_has_equal_work_and_truthful_applied_sources() {
    for confidence in [0.0, 0.6, f32::from_bits(1)] {
        let bank = joint_confidence_fixture(confidence);
        let probe = probe_for_object(3, slot(0, 72, 0.4, [0., 0.8, 0.9]));
        let ordinary = bank.recall_frame(&probe).unwrap();
        let joint = bank.recall_frame_with_joint_membership(&probe).unwrap();
        let (applied, queried_a) = bank.recall_frame_joint_comparison(&probe, true).unwrap();
        let (withheld, queried_b) = bank.recall_frame_joint_comparison(&probe, false).unwrap();
        assert_eq!(queried_a, queried_b);
        assert_eq!(applied.context(), joint.context());
        assert_eq!(withheld.context(), ordinary.context());
        assert_eq!(applied.receipt().joint_membership_sources, queried_a);
        assert!(withheld.receipt().joint_membership_sources.is_empty());
        assert_eq!(
            applied.receipt().execution_evidence_work,
            withheld.receipt().execution_evidence_work
        );
        assert!(
            applied.receipt().execution_evidence_work > joint.receipt().execution_evidence_work
        );
        applied.finalize(probe.clone()).unwrap();
        withheld.finalize(probe).unwrap();
    }
}

#[test]
fn contact_query2273_support_boundaries_preserve_estimator_and_fallback() {
    // Storage-only fixture, not an acquired World checkpoint or body claim.
    let object = slot(0, 72, 0.4, [0., 0.8, 0.9]);
    let source_draft = probe_for_object(1, object)
        .with_candidates(vec![candidate_for_family(
            &object,
            0,
            CandidateActionFamily::Contact,
        )])
        .unwrap();
    let mut member = fact(1, 72, false);
    member.query = empty_bank()
        .recall_frame(&source_draft)
        .unwrap()
        .finalize(source_draft)
        .unwrap()
        .1
        .candidate_keys()[0]
        .query()
        .clone();
    for (confidence, tick, usable) in [
        (0., 3, false),
        (f32::from_bits(1), 3, true),
        (f32::from_bits(1), 1_000_000, false),
        (f32::from_bits(16), 1_000_000, true),
        (0.6, 3, true),
    ] {
        let bank = fixture_with_secondary(&member);
        let mut wire = serde_json::to_value(bank).unwrap();
        let event = serde_json::json!({"schema_version":1,"sealed_causal_digest":[4,3,2,1],"members":serde_json::to_vec(&vec![member.clone()]).unwrap(),"shared_family_value":[-0.6,0.0,0.7,-0.1],"event_confidence":confidence});
        wire["candidate_store"]["records"]["1"]["schema_version"] = serde_json::json!(3);
        wire["candidate_store"]["records"]["1"]["joint_event"] =
            serde_json::json!(serde_json::to_vec(&event).unwrap());
        let bank: MemoryBank = serde_json::from_value(wire).unwrap();
        let probe = probe_for_object(tick, object)
            .with_candidates(vec![
                candidate_for_family(&object, 0, CandidateActionFamily::Contact),
                candidate_for_family(&object, 1, CandidateActionFamily::Ingest),
            ])
            .unwrap();
        let ordinary = bank.recall_frame(&probe).unwrap();
        let (old, _) = bank
            .recall_frame_joint_contact_comparison2266(&probe, true)
            .unwrap();
        let (a, aq) = bank
            .recall_frame_joint_contact_query_comparison2273(&probe, true)
            .unwrap();
        let (b, bq) = bank
            .recall_frame_joint_contact_query_comparison2273(&probe, false)
            .unwrap();
        assert_eq!(a.context(), old.context());
        assert_eq!(b.context(), ordinary.context());
        assert_eq!(!aq.is_empty(), usable);
        assert_eq!(aq, bq);
        assert_eq!(
            a.receipt().execution_evidence_work,
            b.receipt().execution_evidence_work
        );
        assert!(a.receipt().execution_evidence_work < old.receipt().execution_evidence_work);
        if !usable {
            assert_eq!(a.context(), ordinary.context());
        }
        a.finalize(probe.clone()).unwrap();
        b.finalize(probe).unwrap();
    }
}

#[test]
fn non_ingest2278_zero_and_subnormal_support_preserve_exact_full_query_policy() {
    // Checked storage fixture only, not a fabricated native World outcome.
    for family in [
        CandidateActionFamily::Contact,
        CandidateActionFamily::Ingest,
    ] {
        for (confidence, tick, usable) in [
            (0., 3, false),
            (f32::from_bits(1), 3, true),
            (f32::from_bits(1), 1_000_000, false),
            (f32::from_bits(16), 1_000_000, true),
        ] {
            let object = slot(0, 72, 0.4, [0., 0.8, 0.9]);
            let draft = probe_for_object(1, object)
                .with_candidates(vec![candidate_for_family(&object, 0, family)])
                .unwrap();
            let (_, recall) = empty_bank()
                .recall_frame(&draft)
                .unwrap()
                .finalize(draft)
                .unwrap();
            let mut member = fact(1, 72, true);
            member.query = recall.candidate_keys()[0].query().clone();
            let mut wire = serde_json::to_value(joint_confidence_fixture(confidence)).unwrap();
            let bytes: Vec<u8> = serde_json::from_value(
                wire["candidate_store"]["records"]["1"]["joint_event"].clone(),
            )
            .unwrap();
            let mut event: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            event["members"] = serde_json::json!(serde_json::to_vec(&vec![member]).unwrap());
            wire["candidate_store"]["records"]["1"]["joint_event"] =
                serde_json::json!(serde_json::to_vec(&event).unwrap());
            let bank: MemoryBank = serde_json::from_value(wire).unwrap();
            let probe = probe_for_object(tick, object)
                .with_candidates(vec![candidate_for_family(&object, 0, family)])
                .unwrap();
            let plain = bank.recall_frame(&probe).unwrap();
            let (global, queries) = bank.recall_frame_joint_comparison(&probe, true).unwrap();
            let (applied, aq) = bank
                .recall_frame_joint_non_ingest_comparison2278(&probe, true)
                .unwrap();
            let (withheld, wq) = bank
                .recall_frame_joint_non_ingest_comparison2278(&probe, false)
                .unwrap();
            assert_eq!(!queries.is_empty(), usable);
            assert_eq!(queries, aq);
            assert_eq!(queries, wq);
            assert_eq!(
                applied.context(),
                if family == CandidateActionFamily::Ingest {
                    plain.context()
                } else {
                    global.context()
                }
            );
            assert_eq!(withheld.context(), plain.context());
            assert_eq!(
                applied.receipt().execution_evidence_work,
                global.receipt().execution_evidence_work
            );
            assert_eq!(
                withheld.receipt().execution_evidence_work,
                global.receipt().execution_evidence_work
            );
            assert_eq!(
                !applied.receipt().joint_membership_sources.is_empty(),
                usable && family != CandidateActionFamily::Ingest
            );
            applied.finalize(probe.clone()).unwrap();
            withheld.finalize(probe).unwrap();
        }
    }
}

#[test]
fn ingest2286_retains_existing_ordinary_confidence_underflow_contract() {
    // Deliberate checked storage fixture, not an edited acquired World life.
    let mut wire = serde_json::to_value(joint_fixture()).unwrap();
    wire["candidate_store"]["records"]["1"]["confidence"] = serde_json::json!(f32::from_bits(1));
    let bank: MemoryBank = serde_json::from_value(wire).unwrap();
    for tick in [3, 1_000_000] {
        let probe = probe_for_object(tick, slot(0, 71, 0.4, [0., 0.8, 0.9]));
        let ordinary = bank.recall_frame(&probe);
        let next = bank.recall_frame_joint_ingest_outcome_basis2286(&probe);
        match ordinary {
            Ok(ordinary) => {
                let (_, _, basis) = next.unwrap();
                let c = &ordinary.context().candidates[0];
                let b = &basis.evidence().rows[0];
                assert_eq!(b.confidence, c.family_confidence.raw());
                assert_eq!(b.source_count, c.family_source_count);
                assert_eq!(
                    b.support_eligible,
                    c.family_source_count > 0 && c.family_confidence.raw() > 0.
                );
            }
            Err(error) => assert_eq!(next.err(), Some(error)),
        }
    }
}

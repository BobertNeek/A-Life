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
    let sidecar = grounded_sidecar(config.clone());
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

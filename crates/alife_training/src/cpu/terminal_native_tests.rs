use super::*;
use alife_world::*;
#[path = "../../tests/support/terminal_parent2135.rs"]
#[allow(dead_code)]
mod parent;

fn food_fixture(legacy: bool, near_is_food: bool) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    food_fixture_version(if legacy { 2 } else { 3 }, near_is_food)
}

fn food_fixture_version(
    version: u16,
    near_is_food: bool,
) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    let legacy = version == 2;
    let (mut host, trainer) = if legacy {
        parent::fixture(None)
    } else {
        parent::contact20_fixture(None)
    };
    let old = host
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let own = old.organism_id();
    // Fresh exact-parent fixture, matching the assay's native world construction.
    // No existing individual is reset, and every subsequent action is sampled.
    let mut world = HeadlessScenarioBuilder::new(213503)
        .agent("subject", own, Vec3f::ZERO)
        .build()
        .unwrap();
    let mut chemistry = *old.biochemistry();
    chemistry.body.set_energy(0.25).unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::new(
                own,
                world.entity_id("subject").unwrap(),
                old.genome().clone(),
                old.phenotype().clone(),
                chemistry,
                old.birth_tick(),
            )
            .unwrap(),
        )
        .unwrap();
    let placements = if version == 4 {
        vec![("near-food", 0.25), ("far-food", 2.5), ("other-food", 3.5)]
    } else {
        vec![("near-food", 0.25), ("far-food", 2.5)]
    };
    for (label, x) in placements {
        let entity = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: label.into(),
                kind: if label == "near-food" && !near_is_food {
                    WorldObjectKind::ActivityToy
                } else {
                    WorldObjectKind::Food
                },
                organism_id: None,
                position: Vec3f::new(x, 0.0, 0.0),
                nutrition: if label == "near-food" && !near_is_food {
                    0.0
                } else {
                    0.6
                },
                hazard_pain: 0.0,
                radius: 0.3,
                token_id: None,
            })
            .unwrap();
        world
            .set_grounded_physical_properties(
                entity,
                GroundedPhysicalProperties {
                    velocity: Vec3f::ZERO,
                    color: [0.2, 0.7, 0.1],
                    material: [0.2; 3],
                    shape: [0.7, 0.2, 0.4],
                    chemical: [0.0; 3],
                    surface_temperature: 0.0,
                    terrain: [0.5; 2],
                },
            )
            .unwrap();
    }
    for _ in 0..64 {
        world.try_advance_tick().unwrap();
    }
    host.world = world;
    host.seal_all_residents().unwrap();
    let config = if legacy {
        TerminalContextConfig2135::legacy_v2(1.0)
    } else if version == 4 {
        TerminalContextConfig2135::for_parent_batched(
            &trainer.source_foundation,
            old.genome(),
            trainer.phenotype(),
            1.0,
        )
    } else {
        TerminalContextConfig2135::for_parent(
            &trainer.source_foundation,
            old.genome(),
            trainer.phenotype(),
            1.0,
        )
    }
    .unwrap();
    host.enable_terminal_context2135(config, &trainer).unwrap();
    (host, trainer)
}

#[test]
fn real_nonrepresentative_consumption_is_recorded_once_and_survives_cold_continuation() {
    let (mut host, trainer) = food_fixture(false, true);
    let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    for interval in 0..64 {
        let step = host.step(&trainer, 1.0).unwrap();
        for row in &step.decisions {
            let resident = &host.residents[&row.organism_id.raw()];
            let (recall, patch) = resident.last_terminal_capture.as_ref().unwrap();
            let source = row
                .terminal_context2135
                .as_ref()
                .unwrap()
                .observation
                .as_ref()
                .unwrap();
            if let Some(recorded) = &source.recorded {
                let observation = resident
                    .terminal_channel2135
                    .as_ref()
                    .unwrap()
                    .observations
                    .last()
                    .unwrap();
                assert_eq!(observation.channel(), MotorChannel::Manipulation);
                assert_eq!(
                    observation.query().action_family(),
                    CandidateActionFamily::Ingest
                );
                assert_eq!(recorded.sequence_id, patch.header().sequence_id.raw());
                assert_eq!(recorded.observation_digest, observation.canonical_digest());
                assert_eq!(observation.patch_digest(), patch.causal_digest().unwrap());
                assert_eq!(
                    observation.bundle_digest(),
                    patch.selected_bundle().unwrap().canonical_digest().unwrap()
                );
                let mut corrupt = serde_json::to_value(observation).unwrap();
                corrupt["channel"] = serde_json::json!("Locomotion");
                assert!(serde_json::from_value::<ChannelTerminalObservationV1>(corrupt).is_err());
                assert_ne!(
                    patch.decision().selected_action.action_id,
                    observation.query().action_id(),
                    "fixture must consume through a different representative"
                );
                assert_eq!(
                    patch.decision().neural_evidence().unwrap().action_family,
                    CandidateActionFamily::Approach
                );
                assert_eq!(
                    resident
                        .terminal_channel2135
                        .as_ref()
                        .unwrap()
                        .observations
                        .len(),
                    1
                );
                // The ordinary observer keeps the actual representative family.
                assert!(!resident
                    .memory
                    .export_active_bank()
                    .unwrap()
                    .records
                    .iter()
                    .any(|r| r.sealed_sequence_id_raw == recorded.sequence_id
                        && r.family_raw == u16::from(CandidateActionFamily::Ingest.raw())));
                let key = observation.key();
                assert!(ChannelTerminalObservationV1::try_from_sealed_patch(
                    patch,
                    recall,
                    key,
                    MotorChannel::Locomotion
                )
                .is_err());
                let wrong = recall
                    .candidate_keys()
                    .iter()
                    .find(|k| {
                        k.query().tracked_object_id().is_some()
                            && k.query().tracked_object_id() != key.query().tracked_object_id()
                    })
                    .unwrap();
                assert!(ChannelTerminalObservationV1::try_from_sealed_patch(
                    patch,
                    recall,
                    wrong,
                    MotorChannel::Manipulation
                )
                .is_err());
                let mut duplicate = resident.terminal_channel2135.clone().unwrap();
                let before = serde_json::to_vec(&duplicate).unwrap();
                let mut admission =
                    admit_terminal_observation2135(duplicate.observations.len(), 65535);
                assert!(duplicate.observe(patch, recall, &mut admission).is_err());
                assert!(serde_json::to_vec(&duplicate).unwrap() == before);
                let bytes = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
                let mut mode_receipts = Vec::new();
                for (gain, mask) in [(1.0, false), (0.0, false), (1.0, true)] {
                    let mut cp: CpuCanonicalCheckpoint = serde_json::from_slice(&bytes).unwrap();
                    let config = cp.terminal_context2135.as_mut().unwrap();
                    config.head_recipe = crate::cpu::terminal_context::head::Head::new(true, gain)
                        .unwrap()
                        .to_bytes();
                    config.mask_new_context = mask;
                    let mut branch = CpuCanonicalHost::from_checkpoint(cp, &trainer).unwrap();
                    let row = branch.step(&trainer, 1.0).unwrap().decisions.remove(0);
                    let mut value =
                        serde_json::to_value(row.terminal_context2135.unwrap()).unwrap();
                    value.as_object_mut().unwrap().remove("config");
                    value["observation"]
                        .as_object_mut()
                        .unwrap()
                        .remove("recorded");
                    for c in value["candidates"].as_array_mut().unwrap() {
                        c.as_object_mut().unwrap().remove("delta");
                    }
                    mode_receipts.push(value);
                }
                assert_eq!(mode_receipts[0], mode_receipts[1]);
                assert_eq!(mode_receipts[0], mode_receipts[2]);
                let mut cold = CpuCanonicalHost::from_checkpoint(
                    serde_json::from_slice(&bytes).unwrap(),
                    &trainer,
                )
                .unwrap();
                assert!(serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap() == bytes);
                let hot_step = host.step(&trainer, 1.0).unwrap();
                let cold_step = cold.step(&trainer, 1.0).unwrap();
                assert!(
                    serde_json::to_vec(&hot_step.decisions).unwrap()
                        == serde_json::to_vec(&cold_step.decisions).unwrap()
                );
                assert!(
                    serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
                        == serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
                );
                assert!(hot_step
                    .decisions
                    .iter()
                    .flat_map(|r| &r.terminal_context2135.as_ref().unwrap().candidates)
                    .any(|c| !c.source_channel_observations.is_empty()));
                assert!(serde_json::to_vec(&trainer.checkpoint()).unwrap() == founder);
                eprintln!("native channel consumption: interval={interval}, sequence={}, representative=Approach, channel=Manipulation, recorded=1", recorded.sequence_id);
                return;
            }
            if let Some(bundle) = patch.selected_bundle() {
                if let Some(command) = bundle
                    .channels
                    .iter()
                    .find(|c| c.channel == MotorChannel::Manipulation)
                {
                    if let Some(key) = recall.candidate_keys().iter().find(|k| {
                        k.query().action_id() == command.primitive
                            && k.query().action_family() == CandidateActionFamily::Ingest
                    }) {
                        assert!(ChannelTerminalObservationV1::try_from_sealed_patch(
                            patch,
                            recall,
                            key,
                            MotorChannel::Manipulation
                        )
                        .unwrap()
                        .is_none());
                    }
                }
            }
        }
    }
    panic!("bounded native fixture did not consume through independent Manipulation");
}

#[test]
#[ignore = "requires an actual recipe2 checkpoint captured before the observer change"]
fn external_recipe2_checkpoint_keeps_exact_bytes_and_continuation() {
    let (_, trainer) = parent::fixture(None);
    let bytes = std::fs::read(std::env::var_os("ALIFE_NATIVE_LEGACY_CP").unwrap()).unwrap();
    let mut host =
        CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes).unwrap(), &trainer)
            .unwrap();
    assert_eq!(
        host.terminal_context2135.as_ref().unwrap().schema_version,
        2
    );
    assert!(host
        .residents
        .values()
        .all(|r| r.terminal_channel2135.is_none()));
    assert!(serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap() == bytes);
    let mut cold =
        CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes).unwrap(), &trainer)
            .unwrap();
    let a = host.step(&trainer, 1.0).unwrap();
    let b = cold.step(&trainer, 1.0).unwrap();
    assert!(serde_json::to_vec(&a.decisions).unwrap() == serde_json::to_vec(&b.decisions).unwrap());
    assert!(a.decisions.iter().all(|r| r
        .terminal_context2135
        .as_ref()
        .unwrap()
        .observation
        .is_none()));
}

#[test]
fn native_nonconsumed_toy_contact_never_creates_terminal_consumption() {
    let (mut host, trainer) = food_fixture(false, false);
    let toy = host.world.entity_id("near-food").unwrap();
    for _ in 0..16 {
        let step = host.step(&trainer, 1.0).unwrap();
        for row in step.decisions {
            let resident = &host.residents[&row.organism_id.raw()];
            let (recall, patch) = resident.last_terminal_capture.as_ref().unwrap();
            let Some(physical) = patch
                .outcome()
                .joint
                .as_ref()
                .unwrap()
                .channel_outcomes
                .iter()
                .find(|c| {
                    c.channel == MotorChannel::Manipulation && c.physical.target_entity == Some(toy)
                })
            else {
                continue;
            };
            if !matches!(
                physical.physical.contact,
                PhysicalContactKind::Touch | PhysicalContactKind::Blocked
            ) {
                continue;
            }
            let command = patch
                .selected_bundle()
                .unwrap()
                .channels
                .iter()
                .find(|c| c.channel == MotorChannel::Manipulation)
                .unwrap();
            let Some(key) = recall.candidate_keys().iter().find(|k| {
                k.query().action_id() == command.primitive
                    && k.query().action_family() == CandidateActionFamily::Ingest
                    && patch.pre_action().perception().candidates()
                        [k.query().candidate_index() as usize]
                        .target
                        .entity
                        == Some(toy)
            }) else {
                continue;
            };
            assert!(ChannelTerminalObservationV1::try_from_sealed_patch(
                patch,
                recall,
                key,
                MotorChannel::Manipulation
            )
            .unwrap()
            .is_none());
            assert!(row
                .terminal_context2135
                .as_ref()
                .unwrap()
                .observation
                .as_ref()
                .unwrap()
                .recorded
                .is_none());
            assert!(resident
                .terminal_channel2135
                .as_ref()
                .unwrap()
                .observations
                .is_empty());
            return;
        }
    }
    panic!("native toy fixture did not produce a nonconsumed Ingest contact");
}

#[test]
fn parent_bound_recipe_rejects_cross_parent_pairs() {
    let (bearing, bearing_trainer) = parent::fixture(None);
    let (contact, contact_trainer) = parent::contact20_fixture(None);
    let old_genome = bearing
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .genome();
    let genome = contact
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .genome();
    let config = TerminalContextConfig2135::for_parent(
        &contact_trainer.source_foundation,
        genome,
        contact_trainer.phenotype(),
        1.0,
    )
    .unwrap();
    assert_eq!(
        config.parent_sha256,
        "d83a995e802e41b8e17799b0871f75d9ede35f95850049777323112e636654d1"
    );
    assert!(config
        .validate_parent(
            &bearing_trainer.source_foundation,
            bearing_trainer.phenotype()
        )
        .is_err());
    assert!(TerminalContextConfig2135::for_parent(
        &contact_trainer.source_foundation,
        old_genome,
        contact_trainer.phenotype(),
        1.0
    )
    .is_err());
    assert!(TerminalContextConfig2135::legacy_v2(1.0)
        .unwrap()
        .validate_parent(
            &contact_trainer.source_foundation,
            contact_trainer.phenotype()
        )
        .is_err());
}

#[test]
fn recipe2_keeps_legacy_nonrepresentative_consumption_semantics() {
    let (mut host, trainer) = food_fixture(true, true);
    for _ in 0..16 {
        for row in host.step(&trainer, 1.0).unwrap().decisions {
            let joint = row.patch.outcome().joint.as_ref().unwrap();
            if !joint.channel_outcomes.iter().any(|c| {
                c.channel == MotorChannel::Manipulation
                    && c.physical.contact == PhysicalContactKind::Consumed
            }) {
                continue;
            }
            assert_ne!(
                row.patch
                    .decision()
                    .neural_evidence()
                    .unwrap()
                    .action_family,
                CandidateActionFamily::Ingest
            );
            assert!(row
                .terminal_context2135
                .as_ref()
                .unwrap()
                .observation
                .is_none());
            let cp = host.checkpoint(&trainer).unwrap();
            assert_eq!(cp.terminal_context2135.as_ref().unwrap().schema_version, 2);
            assert!(cp.residents[0].terminal_channel2135.is_none());
            assert!(!cp.residents[0].memory.records.iter().any(|r| r.family_raw
                == u16::from(CandidateActionFamily::Ingest.raw())
                && r.outcome_contact == Some(PhysicalContactKind::Consumed)));
            return;
        }
    }
    panic!("legacy native fixture did not reproduce independent consumption");
}

#[test]
fn recipe4_native_consumption_batch_equality_work_authority_and_cold() {
    use crate::cpu::terminal_context::reader;
    let (mut host, trainer) = food_fixture_version(4, true);
    let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    let mut had_source = false;
    let mut earlier_queries = Vec::new();
    for _ in 0..64 {
        let step = host.step(&trainer, 1.0).unwrap();
        let row = &step.decisions[0];
        let receipt = row.terminal_context2135.as_ref().unwrap();
        assert_eq!(receipt.schema_version, 4);
        let work = receipt.batch_work.as_ref().unwrap();
        if receipt.admission == TerminalAdmission2135::Admitted {
            assert_eq!(work.bank_exports, 1);
            assert_eq!(work.reader_bank_validations, 1);
            assert_eq!(work.combined_digest_builds, 1);
            assert_eq!(work.failed_contact_index_records, work.ordinary_records);
            assert_eq!(work.channel_contract_validations, work.channel_records);
            assert_eq!(work.query_contract_validations, receipt.candidates.len());
            assert_eq!(
                work.query_source_visits,
                receipt.candidates.len() * work.channel_records
            );
            assert!(work.appearance_comparisons <= work.query_source_visits * 9);
        } else {
            assert_eq!(work.bank_exports, 0);
            assert_eq!(work.reader_bank_validations, 0);
            assert!(receipt.candidates.is_empty());
        }
        if had_source
            && receipt
                .candidates
                .iter()
                .any(|c| c.consumed_source_count > 0)
        {
            let resident = &host.residents[&row.organism_id.raw()];
            let bank = resident.memory.export_active_bank().unwrap();
            let observations = &resident.terminal_channel2135.as_ref().unwrap().observations;
            let batch = reader::batch::Batch::new(&bank, observations).unwrap();
            let recall = &resident.last_terminal_capture.as_ref().unwrap().0;
            let mut queries = 0;
            let current_queries = recall
                .candidate_keys()
                .iter()
                .map(|key| key.query().clone())
                .collect::<Vec<_>>();
            assert!(earlier_queries
                .iter()
                .any(|q: &CandidateMemoryQueryV2| q.tick() < observations[0].query().tick()));
            for query in current_queries.iter().chain(earlier_queries.iter()) {
                if query.action_family() != CandidateActionFamily::Approach
                    || query.tracked_object_id().is_none()
                {
                    continue;
                }
                let mut old = reader::read_channels(&bank, query, observations)
                    .unwrap()
                    .unwrap();
                let new = batch.read(query).unwrap();
                // Functional output is exact; per-query B scans were removed.
                old.records_examined = new.records_examined;
                assert_eq!(old, new);
                queries += 1;
            }
            assert!(
                queries > 1,
                "qualify sharing across multiple real prepared queries"
            );
            let mut corrupt = bank.clone();
            corrupt.canonical_digest[0] ^= 1;
            assert!(reader::batch::Batch::new(&corrupt, observations).is_err());
            let cp = host.checkpoint(&trainer).unwrap();
            let bytes = serde_json::to_vec(&cp).unwrap();
            let mut cold = CpuCanonicalHost::from_checkpoint(
                serde_json::from_slice(&bytes).unwrap(),
                &trainer,
            )
            .unwrap();
            assert_eq!(
                serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap(),
                bytes
            );
            let mut changed_recipe = cp.clone();
            changed_recipe
                .terminal_context2135
                .as_mut()
                .unwrap()
                .schema_version = 3;
            assert!(CpuCanonicalHost::from_checkpoint(changed_recipe, &trainer).is_err());
            let mut bad = cp;
            bad.residents[0]
                .terminal_channel2135
                .as_mut()
                .unwrap()
                .observations
                .clear();
            assert!(CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err());
            let hot = host.step(&trainer, 1.0).unwrap();
            let cold_step = cold.step(&trainer, 1.0).unwrap();
            assert_eq!(
                serde_json::to_vec(&hot.decisions).unwrap(),
                serde_json::to_vec(&cold_step.decisions).unwrap()
            );
            assert_eq!(
                serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
                serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
            );
            assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), founder);
            return;
        }
        if earlier_queries.is_empty() {
            earlier_queries = host.residents[&row.organism_id.raw()]
                .last_terminal_capture
                .as_ref()
                .unwrap()
                .0
                .candidate_keys()
                .iter()
                .map(|key| key.query().clone())
                .collect();
        }
        had_source |= receipt.observation.as_ref().unwrap().recorded.is_some();
    }
    panic!("native Contact20 must actually consume and later use its source");
}

#[test]
#[ignore = "requires a preserved LOCAL real recipe3 capacity-crossing checkpoint"]
fn retained_real_bank_above64_has_paid_recipe4_read_without_migrating_the_life() {
    let input = std::path::PathBuf::from(std::env::var_os("ALIFE_TERMINAL_REAL_INPUT").unwrap());
    let bytes = std::fs::read(input).unwrap();
    let cp: CpuCanonicalCheckpoint = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(cp.terminal_context2135.as_ref().unwrap().schema_version, 3);
    let (_, trainer) = parent::contact20_fixture(None);
    let mut host = CpuCanonicalHost::from_checkpoint(cp, &trainer).unwrap();
    assert_eq!(
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
        bytes
    );
    // Continue the old observer once. Recipe4 is only a pure read counterfactual
    // on this retained snapshot, never enabled/restored into the existing life.
    let step = host.step(&trainer, 1.0).unwrap();
    let row = &step.decisions[0];
    let resident = &host.residents[&row.organism_id.raw()];
    let channels = resident.terminal_channel2135.as_ref().unwrap();
    let recall = &resident.last_terminal_capture.as_ref().unwrap().0;
    assert!(resident.memory.bank().len() > 64);
    let old = prepare_terminal_channels2135(
        host.terminal_context2135.as_ref().unwrap(),
        &resident.memory,
        channels,
        recall,
        65535,
    )
    .unwrap();
    assert!(matches!(
        old.admission,
        TerminalAdmission2135::BankCapacityExceeded { limit: 64, .. }
    ));
    let record = host.world.organism_registry().iter().next().unwrap();
    let config = TerminalContextConfig2135::for_parent_batched(
        &trainer.source_foundation,
        record.genome(),
        trainer.phenotype(),
        16.0,
    )
    .unwrap();
    let new =
        prepare_terminal_channels2135(&config, &resident.memory, channels, recall, 65535).unwrap();
    assert_eq!(new.admission, TerminalAdmission2135::Admitted);
    assert!(!new.capacity_rejected);
    assert_eq!(
        new.batch_work.as_ref().unwrap().ordinary_records,
        resident.memory.bank().len()
    );
    assert!(new.reserved_atp_q16 > 0);
    assert!(new
        .candidates
        .iter()
        .any(|c| c.delta != 0.0 && !c.source_channel_observations.is_empty()));
    let denied =
        prepare_terminal_channels2135(&config, &resident.memory, channels, recall, 0).unwrap();
    assert!(matches!(
        denied.admission,
        TerminalAdmission2135::InsufficientAtp { .. }
    ));
    assert_eq!(denied.batch_work.as_ref().unwrap().bank_exports, 0);
    assert_eq!(
        denied.batch_work.as_ref().unwrap().reader_bank_validations,
        0
    );
    eprintln!("retained real bank: B={}, C={}, old=capacity abstention, new work={}, new ATP={}, queries={}",resident.memory.bank().len(),channels.observations.len(),new.reserved_work_units,new.reserved_atp_q16,new.candidates.len());
}

#[test]
fn recipe4_constructed_low_atp_denies_only_extra_read_and_continues() {
    let (mut host, trainer) = food_fixture_version(4, true);
    for _ in 0..3 {
        host.step(&trainer, 1.0).unwrap();
    }
    let id = host.world.organism_entity_ids()[0].0;
    let mut registry = host.world.organism_registry().clone();
    registry
        .with_biology_mut(id, |biology| {
            biology.homeostasis.drives.brain_atp = 0.051;
            Ok(())
        })
        .unwrap();
    host.world
        .replace_organism_registry_exact(registry.iter().cloned())
        .unwrap();
    let step = host.step(&trainer, 1.0).unwrap();
    assert_eq!(step.decisions.len(), 1);
    let receipt = step.decisions[0].terminal_context2135.as_ref().unwrap();
    assert!(matches!(
        receipt.admission,
        TerminalAdmission2135::InsufficientAtp { .. }
    ));
    assert_eq!(receipt.batch_work.as_ref().unwrap().bank_exports, 0);
    assert_eq!(
        receipt.batch_work.as_ref().unwrap().reader_bank_validations,
        0
    );
    assert!(receipt.candidates.is_empty());
    assert_eq!(step.tick_after.raw(), step.tick_before.raw() + 1);
}

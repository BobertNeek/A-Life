use super::*;
use alife_world::*;
#[path = "../../tests/support/terminal_parent2135.rs"]
#[allow(dead_code)]
mod parent;

fn food_fixture(legacy: bool, near_is_food: bool) -> (CpuCanonicalHost, CpuFoundationTrainer) {
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
    for (label, x) in [("near-food", 0.25), ("far-food", 2.5)] {
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
                // The new independently typed record recovers the actual secondary
                // Ingest phase without relabeling the representative's bundle value.
                let bank = resident.memory.export_active_bank().unwrap();
                let facts = bank
                    .records
                    .iter()
                    .filter_map(|record| record.executed_actions_json.as_ref())
                    .flat_map(|bytes| {
                        serde_json::from_slice::<Vec<alife_core::ExecutedActionMemoryV1>>(bytes)
                            .unwrap()
                    })
                    .collect::<Vec<_>>();
                let fact = facts
                    .iter()
                    .find(|fact| {
                        fact.source_sequence_id.raw() == recorded.sequence_id
                            && fact.receipt.channel == MotorChannel::Manipulation
                    })
                    .unwrap();
                assert_eq!(fact.query.action_family(), CandidateActionFamily::Ingest);
                assert_eq!(fact.receipt.physical.contact, PhysicalContactKind::Consumed);
                let input = fact.receipt.execution_evidence.unwrap();
                assert_eq!(
                    input.command_target,
                    observation
                        .key()
                        .query()
                        .tracked_object_id()
                        .and_then(|_| fact.receipt.physical.target_entity)
                );
                assert!(input.body_inputs.nutrition > 0.);
                assert_eq!(input.body_inputs.player_reward, 0.);
                assert_eq!(input.body_inputs.social_disapproval_v1, 0.);
                assert_eq!(
                    fact.bundle_digest,
                    patch.selected_bundle().unwrap().canonical_digest().unwrap()
                );
                assert_eq!(fact.event_tick, patch.outcome().outcome_tick);
                assert_ne!(
                    fact.query.action_family(),
                    patch.decision().neural_evidence().unwrap().action_family
                );
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
                    // Ordinary cognition has no factual consumer: an external
                    // viewer must not silently cause charged brain lookup work.
                    assert_eq!(row.executed_action_recall_work, 0);
                    assert!(row.executed_action_recall.is_none());
                    let (step_recall, _) = branch.residents[&row.organism_id.raw()]
                        .last_terminal_capture
                        .as_ref()
                        .unwrap();
                    let recalled = step_recall.receipt();
                    assert!(step_recall.executed_actions().iter().all(Option::is_none));
                    let bank_before = serde_json::to_vec(
                        &branch.residents[&row.organism_id.raw()]
                            .memory
                            .export_active_bank()
                            .unwrap(),
                    )
                    .unwrap();
                    let inspection = branch.residents[&row.organism_id.raw()]
                        .memory
                        .inspect_execution_evidence(step_recall)
                        .unwrap();
                    assert!(inspection.diagnostic_work_units > 0);
                    assert_eq!(inspection.rows.len(), step_recall.candidate_keys().len());
                    assert_eq!(
                        serde_json::to_vec(
                            &branch.residents[&row.organism_id.raw()]
                                .memory
                                .export_active_bank()
                                .unwrap()
                        )
                        .unwrap(),
                        bank_before
                    );
                    assert_eq!(recalled.execution_evidence_work, 0);
                    let base_memory_ops = u64::from(recalled.exact_bucket_reads)
                        + u64::from(recalled.neighbor_bucket_reads)
                        + u64::from(recalled.similarity_evaluations)
                        + row
                            .terminal_context2135
                            .as_ref()
                            .map_or(0, |r| r.reserved_work_units)
                        + row
                            .memory_readout_research
                            .as_ref()
                            .map_or(0, |r| r.additional_work_ops);
                    assert_eq!(
                        row.executed_action_recall_work,
                        recalled.execution_evidence_work
                    );
                    let charged = row.patch.cognitive_work().unwrap();
                    assert_eq!(
                        charged.memory_ops,
                        base_memory_ops + row.executed_action_recall_work
                    );
                    let record = branch
                        .world
                        .organism_registry()
                        .get(row.organism_id)
                        .unwrap();
                    assert_eq!(record.cognitive_work(), charged);
                    let (actual_policy, actual_debit, before_energy, after_energy) =
                        row.cognitive_cost_capture.unwrap();
                    let phenotype = record.phenotype();
                    let requested = actual_policy.energy_debit(charged).unwrap()
                        * phenotype.body.metabolic_turnover
                        * (alife_core::ContinuousLocus::midpoint_value(0.52, 0.60)
                            - phenotype.chemistry.brain_atp_efficiency)
                            .exp2();
                    assert_eq!(actual_debit, requested.min(before_energy));
                    assert_eq!(record.cognitive_energy_debit(), actual_debit);
                    assert!((before_energy - after_energy - actual_debit).abs() < 2.0e-6);
                    let mut component = alife_core::cognitive_work::CognitiveWorkCounters::zero();
                    component.memory_ops = row.executed_action_recall_work;
                    let tariff_component = actual_policy
                        .energy_debit(&component.into_receipt().unwrap())
                        .unwrap();
                    eprintln!("OFFBOARD_FACT_INSPECTION diagnostic_work_units={} charged_extra_memory_ops=0", inspection.diagnostic_work_units);
                    eprintln!("EXECUTION_FACT_ACCOUNTING base_memory_ops={base_memory_ops} extra_memory_ops={} total_memory_ops={} actual_tariff={} component_tariff_calculation={tariff_component} actual_total_applied_debit={actual_debit} measured_total_energy_difference={}",
                        row.executed_action_recall_work, charged.memory_ops, actual_policy.energy_per_work_unit,
                        before_energy - after_energy);
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
fn joint_membership_observes_real_secondary_meal_without_rewriting_world_or_credit() {
    let (mut host, trainer) = food_fixture(false, true);
    let owner = host
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    let resident = &host.residents[&owner.raw()];
    let mut observed = resident.memory.clone();
    let mut legacy = resident.memory.clone();
    let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    let mut actual_prior_queries = 0;
    for _ in 0..64 {
        let step = host.step(&trainer, 1.0).unwrap();
        let host_bytes = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
        for row in &step.decisions {
            let frame = row.patch.pre_action().perception();
            let draft = PerceptionFrameDraft::new(
                frame.organism_id(),
                frame.tick(),
                frame.sensor_profile(),
                frame.sensory().clone(),
                frame.body(),
                *frame.homeostasis(),
                frame.candidates().to_vec(),
                frame.profile_provenance(),
                frame.grounded_object_slots().to_vec(),
            )
            .unwrap();
            let plain = observed.recall_frame(&draft).unwrap();
            let member = observed.recall_frame_with_joint_membership(&draft).unwrap();
            assert!(plain.receipt().joint_membership_sources.is_empty());
            for (a, b) in plain
                .context()
                .candidates
                .iter()
                .zip(&member.context().candidates)
            {
                assert_eq!(a.target_latent, b.target_latent);
                assert_eq!(a.target_confidence, b.target_confidence);
            }
            actual_prior_queries +=
                usize::from(!member.receipt().joint_membership_sources.is_empty());
            member.finalize(draft).unwrap();
            let admission = observed
                .observe_sealed_patch_with_joint_membership(&row.patch)
                .unwrap();
            assert!(admission.joint_membership_admission_work > 0);
            let ordinary_admission = legacy.observe_sealed_patch(&row.patch).unwrap();
            assert_eq!(ordinary_admission.joint_membership_admission_work, 0);
            let is_meal = row.patch.outcome().joint.as_ref().is_some_and(|j| {
                j.channel_outcomes.iter().any(|r| {
                    r.channel == MotorChannel::Manipulation
                        && r.physical.contact == PhysicalContactKind::Consumed
                })
            });
            if is_meal {
                assert_ne!(
                    row.patch
                        .decision()
                        .neural_evidence()
                        .unwrap()
                        .action_family,
                    CandidateActionFamily::Ingest
                );
                let exported = observed.export_active_bank().unwrap();
                assert_eq!(exported.schema_version, 3);
                assert!(!exported
                    .records
                    .iter()
                    .any(|r| r.family_raw == u16::from(CandidateActionFamily::Ingest.raw())));
                assert!(exported
                    .records
                    .iter()
                    .any(|r| r.joint_event_json.is_some()));
                let restored = MemorySidecarState::restore_portable_with_joint_membership(
                    observed.profile(),
                    *observed.compaction_checkpoint(),
                    exported.clone(),
                    None,
                )
                .unwrap();
                assert_eq!(restored.export_active_bank().unwrap(), exported);
                assert!(observed
                    .observe_sealed_patch_with_joint_membership(&row.patch)
                    .is_err());
                // Explicit saved-context probe, NOT a claim that consumed food is
                // still visible. Advance only the diagnostic query's clock.
                let tick = row.patch.outcome().outcome_tick;
                let mut sensory = frame.sensory().clone();
                sensory.tick = tick;
                let mut home = *frame.homeostasis();
                home.tick = tick;
                let provenance = SensorProfileProvenance::new(
                    frame.sensor_profile(),
                    SensoryAbiVersion::CURRENT,
                    tick,
                )
                .unwrap();
                let probe = PerceptionFrameDraft::new(
                    frame.organism_id(),
                    tick,
                    frame.sensor_profile(),
                    sensory,
                    frame.body(),
                    home,
                    frame.candidates().to_vec(),
                    provenance,
                    frame.grounded_object_slots().to_vec(),
                )
                .unwrap();
                let ordinary = observed.recall_frame(&probe).unwrap();
                let learned = restored.recall_frame_with_joint_membership(&probe).unwrap();
                let ingest = frame
                    .candidates()
                    .iter()
                    .find(|c| {
                        c.family == CandidateActionFamily::Ingest
                            && row
                                .patch
                                .outcome()
                                .joint
                                .as_ref()
                                .unwrap()
                                .channel_outcomes
                                .iter()
                                .any(|r| {
                                    r.channel == MotorChannel::Manipulation
                                        && r.execution_evidence.as_ref().is_some_and(|e| {
                                            e.primitive == c.action_id
                                                && e.command_target == c.target.entity
                                        })
                                })
                    })
                    .unwrap();
                let index = ingest.candidate_index as usize;
                assert_eq!(ordinary.context().candidates[index].family_source_count, 0);
                assert!(learned.context().candidates[index].family_source_count > 0);
                assert!(learned
                    .receipt()
                    .joint_membership_sources
                    .iter()
                    .any(|s| s.candidate_index == ingest.candidate_index
                        && s.source_sequence_id == row.patch.header().sequence_id));
                assert_eq!(
                    learned.context().candidates[index].family_value[1],
                    if row.patch.outcome().success {
                        1.0
                    } else {
                        0.0
                    }
                );
                assert!(learned.receipt().execution_evidence_work > 0);
                learned.finalize(probe).unwrap();
                assert_eq!(
                    host_bytes,
                    serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
                );
                assert_eq!(founder, serde_json::to_vec(&trainer.checkpoint()).unwrap());
                eprintln!("JOINT_MEMBERSHIP real secondary meal admitted; actual prior query frames={actual_prior_queries}; saved-context probe restores exact event; world/neural/credit/Adam unchanged");
                return;
            }
        }
        assert_eq!(
            host_bytes,
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
        );
    }
    panic!("fixture did not consume");
}

#[test]
fn grounded_orientation_v3_is_sampled_and_executed_without_forced_choices() {
    let (mut host, trainer) = parent::grounded_orientation_fixture();
    let owner = host
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    // Public encoder admission must bind repertoire to the actual phenotype,
    // not merely accept any same-profile frame carrying a valid V3 marker.
    let record = host.world.organism_registry().get(owner).unwrap().clone();
    let tick = record.biochemistry().homeostasis.tick;
    let (draft, candidate_binding) = host
        .world
        .perception_frame_draft_with_bindings(
            owner,
            tick,
            trainer.phenotype().sensor_profile(),
            record.biochemistry().homeostasis,
        )
        .unwrap();
    let receptors = record
        .biochemistry()
        .neural_receptor_frame(record.phenotype())
        .unwrap();
    let effects = NeuralReceptorEffects::from_frame(
        &receptors,
        &NeuralReceptorPhenotype::compile(trainer.phenotype()).unwrap(),
    )
    .unwrap();
    let resident = &host.residents[&owner.raw()];
    let prepared = prepare_canonical_frame(
        &CanonicalPreparationInput {
            draft,
            candidate_binding,
            sequence_id: ExperienceSequenceId(resident.next_sequence),
            homeostasis: record.biochemistry().homeostasis,
            hysteresis: resident.hysteresis,
            policy: attention_selection_policy_for(trainer.phenotype()),
            neural_receptors: receptors,
            receptor_effects: effects,
        },
        &resident.memory,
        &resident.topology,
        &resident.predictor,
        false,
    )
    .result
    .unwrap();
    let frame = prepared.frame;
    let recall = prepared.memory_recall;
    assert!(encode_cpu_frame(
        trainer.phenotype(),
        &frame,
        &recall,
        &effects,
        vec![true; trainer.phenotype().projections().len()],
        vec![]
    )
    .is_ok());
    let (_, legacy_trainer) = parent::contact20_fixture(None);
    assert!(encode_cpu_frame(
        legacy_trainer.phenotype(),
        &frame,
        &recall,
        &effects,
        vec![true; legacy_trainer.phenotype().projections().len()],
        vec![]
    )
    .is_err());
    let mut actual_orientation = 0;
    let mut changed_pose = false;
    for _ in 0..32 {
        let id = host
            .world
            .organism_registry()
            .get(owner)
            .unwrap()
            .world_entity_id();
        let before = host.world.entity(id).unwrap().clone();
        let step = host.step(&trainer, 1.0).unwrap();
        let after = host.world.entity(id).unwrap();
        changed_pose |= before.body_yaw != after.body_yaw || before.head_yaw != after.head_yaw;
        for row in &step.decisions {
            assert_ne!(row.action.motor_masks[1], 0);
            assert!(row.action.motor_candidates[1].is_some());
            let frame = row.patch.pre_action().perception();
            assert!(frame.grounded_orientation_repertoire());
            assert!(frame.candidates().len() <= MAX_ACTION_CANDIDATES);
            for action in [
                HeadlessActionIds::TURN_LEFT,
                HeadlessActionIds::TURN_RIGHT,
                HeadlessActionIds::LOOK_LEFT,
                HeadlessActionIds::LOOK_RIGHT,
                HeadlessActionIds::LOOK_CENTER,
                HeadlessActionIds::HOLD_GAZE,
            ] {
                assert_eq!(
                    frame
                        .candidates()
                        .iter()
                        .filter(|c| c.action_id == action)
                        .count(),
                    1
                );
            }
            for receipt in &row.patch.outcome().joint.as_ref().unwrap().channel_outcomes {
                if receipt.channel == MotorChannel::Orientation {
                    assert!(receipt.execution_evidence.as_ref().unwrap().succeeded);
                    actual_orientation += 1;
                }
            }
        }
    }
    assert!(actual_orientation > 0);
    assert!(changed_pose);
    eprintln!(
        "V3 normal native orientation receipts={actual_orientation}; actual body/head pose changed"
    );
    let checkpoint = host.checkpoint(&trainer).unwrap();
    let bytes = serde_json::to_vec(&checkpoint).unwrap();
    let restored =
        CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes).unwrap(), &trainer)
            .unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.checkpoint(&trainer).unwrap()).unwrap(),
        bytes
    );
}

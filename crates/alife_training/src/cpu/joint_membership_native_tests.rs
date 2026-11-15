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
    (host, trainer)
}

#[test]
fn joint_membership_mode_cold_work_and_default_boundaries() {
    let (mut host, trainer) = food_fixture(false, true);
    let original = host.checkpoint(&trainer).unwrap();
    assert_eq!(original.joint_membership2177, None);
    assert!(!serde_json::to_string(&original)
        .unwrap()
        .contains("joint_membership2177"));
    let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    host.enable_joint_membership2177(&trainer).unwrap();
    let mut queries = 0;
    let mut admissions = 0;
    for _ in 0..12 {
        let step = host.step(&trainer, 1.0).unwrap();
        for row in step.decisions {
            let receipt = row.joint_membership2177.as_ref().unwrap();
            assert_eq!(receipt.schema_version, 1);
            assert!(receipt.admission_actual_work <= receipt.admission_reserved_work);
            admissions += usize::from(receipt.admission_actual_work > 0);
            queries += receipt.sources.len();
            let bytes = serde_json::to_vec(&row).unwrap();
            assert!(serde_json::from_slice::<CpuCollectedDecision>(&bytes).is_err());
            assert!(serde_json::from_slice::<serde_json::Value>(&bytes)
                .unwrap()
                .get("joint_membership2177_research_record_v1")
                .is_some());
        }
    }
    assert!(admissions > 0);
    assert!(queries > 0);
    let cp = host.checkpoint(&trainer).unwrap();
    assert_eq!(cp.schema_version, 8);
    let mut cold = CpuCanonicalHost::from_checkpoint(cp.clone(), &trainer).unwrap();
    assert_eq!(
        serde_json::to_vec(&cp).unwrap(),
        serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
    );
    host.step(&trainer, 1.0).unwrap();
    cold.step(&trainer, 1.0).unwrap();
    assert_eq!(
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
        serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
    );
    let mut missing = cp.clone();
    missing.joint_membership2177 = None;
    assert!(CpuCanonicalHost::from_checkpoint(missing, &trainer).is_err());
    let mut downgrade = cp;
    downgrade.schema_version = 2;
    assert!(CpuCanonicalHost::from_checkpoint(downgrade, &trainer).is_err());
    assert!(host.enable_joint_membership2177(&trainer).is_err());
    assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), founder);
}

#[test]
fn joint_membership_refuses_existing_research_mode() {
    let (mut host, trainer) = parent::contact20_fixture(None);
    let config = TerminalContextConfig2135::for_parent(
        &trainer.source_foundation,
        host.world
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .genome(),
        trainer.phenotype(),
        1.0,
    )
    .unwrap();
    host.enable_terminal_context2135(config, &trainer).unwrap();
    let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
    assert!(host.enable_joint_membership2177(&trainer).is_err());
    assert_eq!(
        before,
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
    );
}

#[test]
fn first_empty_memory_decision_differs_only_by_declared_work_before_learning() {
    let (mut off, trainer) = food_fixture(false, true);
    let mut on = off.snapshot();
    on.enable_joint_membership2177(&trainer).unwrap();
    let off_step = off.step(&trainer, 1.0).unwrap();
    let on_step = on.step(&trainer, 1.0).unwrap();
    let a = &off_step.decisions[0];
    let b = &on_step.decisions[0];
    let evidence = b.joint_membership2177.as_ref().unwrap();
    assert!(evidence.sources.is_empty());
    assert_eq!(evidence.query_work, 0);
    assert_eq!(a.action, b.action);
    assert_eq!(a.initial, b.initial);
    assert_eq!(a.replay_tick, b.replay_tick);
    let aw = a.patch.cognitive_work().unwrap();
    let bw = b.patch.cognitive_work().unwrap();
    assert_eq!(
        bw.memory_ops - aw.memory_ops,
        evidence.admission_planning_work + evidence.admission_reserved_work
    );
    assert!(evidence.admission_actual_work <= evidence.admission_reserved_work);
}

#[test]
fn failed_staged_membership_step_restores_all_owned_state() {
    let (mut host, trainer) = food_fixture(false, true);
    host.enable_joint_membership2177(&trainer).unwrap();
    host.fail_joint_membership_after_admission = true;
    let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
    let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    let error = host.step(&trainer, 1.0).err().unwrap();
    assert!(error
        .to_string()
        .contains("after actual membership admission"));
    assert_eq!(
        before,
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
    );
    assert_eq!(founder, serde_json::to_vec(&trainer.checkpoint()).unwrap());
}

#[test]
fn membership_work_cannot_exceed_neural_atp_budget() {
    let mut activity = CpuActivityBudget {
        microsteps: 1,
        enabled_routes: vec![],
        work: BrainWorkCounters::default(),
        available_atp_q16: 0,
        debit_atp_q16: 0,
        affordable: true,
    };
    assert!(charge_joint_membership_work(&mut activity, 100_000).is_err());
    assert!(!activity.affordable);
}

#[test]
fn comparison_modes_equal_first_work_and_preserve_cold_route_identity() {
    let (mut applied, trainer) = food_fixture(false, true);
    let mut withheld = applied.snapshot();
    applied
        .enable_joint_membership_comparison2182(&trainer, true)
        .unwrap();
    withheld
        .enable_joint_membership_comparison2182(&trainer, false)
        .unwrap();
    let a = applied.step(&trainer, 1.0).unwrap();
    let b = withheld.step(&trainer, 1.0).unwrap();
    assert_eq!(a.decisions[0].action, b.decisions[0].action);
    assert_eq!(a.decisions[0].replay_tick, b.decisions[0].replay_tick);
    assert_eq!(
        a.decisions[0].patch.cognitive_work(),
        b.decisions[0].patch.cognitive_work()
    );
    let ar = a.decisions[0].joint_membership2177.as_ref().unwrap();
    let br = b.decisions[0].joint_membership2177.as_ref().unwrap();
    assert_eq!(ar.query_work, br.query_work);
    assert_eq!(ar.admission_reserved_work, br.admission_reserved_work);
    assert_eq!(ar.admission_actual_work, br.admission_actual_work);
    assert!(ar.comparison.as_ref().unwrap().apply);
    assert!(!br.comparison.as_ref().unwrap().apply);
    let mut withheld_queries = 0;
    for host in [&mut applied, &mut withheld] {
        for _ in 0..12 {
            for row in host.step(&trainer, 1.0).unwrap().decisions {
                let r = row.joint_membership2177.as_ref().unwrap();
                let comparison = r.comparison.as_ref().unwrap();
                if !comparison.apply {
                    withheld_queries += comparison.queried_sources.len();
                    assert!(r.sources.is_empty());
                } else {
                    assert_eq!(r.sources, comparison.queried_sources);
                }
                let wire = serde_json::to_value(&row).unwrap();
                assert!(wire
                    .get("joint_membership2182_comparison_record_v1")
                    .is_some());
                assert!(serde_json::from_value::<CpuCollectedDecision>(wire).is_err());
            }
        }
        let cp = host.checkpoint(&trainer).unwrap();
        assert_eq!(cp.schema_version, 7);
        let mut cold = CpuCanonicalHost::from_checkpoint(cp.clone(), &trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&cp).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        host.step(&trainer, 1.0).unwrap();
        cold.step(&trainer, 1.0).unwrap();
        assert_eq!(
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        let mut switched = cp.clone();
        switched.joint_membership2177 = Some(if cp.joint_membership2177 == Some(2) {
            3
        } else {
            2
        });
        assert!(CpuCanonicalHost::from_checkpoint(switched, &trainer).is_err());
        let mut invalid = cp;
        invalid.schema_version = 6;
        assert!(CpuCanonicalHost::from_checkpoint(invalid, &trainer).is_err());
    }
    assert!(withheld_queries > 0);
}

#[test]
fn comparison_preserves_genetic_orientation_capability_and_cold_state() {
    for apply in [false, true] {
        let (mut host, trainer) = parent::grounded_orientation_fixture();
        host.enable_joint_membership_comparison2182(&trainer, apply)
            .unwrap();
        let mut decisions = 0;
        for _ in 0..8 {
            for row in host.step(&trainer, 1.0).unwrap().decisions {
                let frame = row.patch.pre_action().perception();
                assert!(frame.grounded_orientation_repertoire());
                assert!(frame.candidates().len() <= MAX_ACTION_CANDIDATES);
                assert_ne!(row.action.motor_masks[1], 0);
                assert!(row.action.motor_candidates[1].is_some());
                let receipt = row.joint_membership2177.as_ref().unwrap();
                assert_eq!(receipt.comparison.as_ref().unwrap().apply, apply);
                if !apply {
                    assert!(receipt.sources.is_empty());
                }
                decisions += 1;
            }
        }
        assert_eq!(decisions, 8);
        let cp = host.checkpoint(&trainer).unwrap();
        let mut cold = CpuCanonicalHost::from_checkpoint(cp.clone(), &trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&cp).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        host.step(&trainer, 1.0).unwrap();
        cold.step(&trainer, 1.0).unwrap();
        assert_eq!(
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
    }
}

#[test]
fn comparison_enable_failure_preserves_valid_cold_host() {
    for mode in [1, 2, 3, 4, 5] {
        let (mut host, trainer) = parent::contact20_fixture(None);
        let record = host.world().organism_registry().iter().next().unwrap();
        let mut value = serde_json::to_value(record).unwrap();
        value["state_graph"]["transaction_revision"] = serde_json::json!(u64::MAX);
        let record: WorldOrganismRecord = serde_json::from_value(value).unwrap();
        record.validate_contract().unwrap();
        host.world_mut()
            .replace_organism_record_exact(record)
            .unwrap();
        let before = host.checkpoint(&trainer).unwrap();
        let mut host = CpuCanonicalHost::from_checkpoint(before.clone(), &trainer).unwrap();
        assert!(if mode == 1 {
            host.enable_joint_membership2177(&trainer)
        } else if mode <= 3 {
            host.enable_joint_membership_comparison2182(&trainer, mode == 2)
        } else {
            host.enable_joint_membership_contact_comparison2266(&trainer, mode == 4)
        }
        .is_err());
        assert!(
            serde_json::to_vec(&before).unwrap()
                == serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
        );
    }
}

#[test]
fn acquired_empty_joint_history_cannot_switch_mode_on_cold_restore() {
    for enabled in [false, true] {
        let (mut host, trainer) = parent::fixture(None);
        if enabled {
            host.enable_joint_membership2177(&trainer).unwrap();
        }
        host.step(&trainer, 1.0).unwrap();
        let cp = host.checkpoint(&trainer).unwrap();
        assert!(cp.sampling_counter > 0);
        let cold = CpuCanonicalHost::from_checkpoint(cp.clone(), &trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&cp).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        let mut changed = cp;
        changed.joint_membership2177 = if enabled { None } else { Some(1) };
        changed.schema_version = if enabled { 2 } else { 8 };
        assert!(
            CpuCanonicalHost::from_checkpoint(changed, &trainer).is_err(),
            "acquired route switch accepted: enabled={enabled}"
        );
    }
}

#[test]
fn comparison_enable_success_preserves_prior_sealed_contract() {
    for apply in [false, true] {
        let (mut current, trainer) = parent::contact20_fixture(None);
        let mut prior = current.clone();
        // Exact prior successful comparison transition: assign mode, seal once.
        prior.joint_membership2177 = Some(if apply { 2 } else { 3 });
        for resident in prior.residents.values_mut() {
            resident.joint_comparison_mode = prior.joint_membership2177;
        }
        prior.seal_all_residents().unwrap();
        current
            .enable_joint_membership_comparison2182(&trainer, apply)
            .unwrap();
        let old = prior.checkpoint(&trainer).unwrap();
        assert_eq!(old.schema_version, 7);
        assert!(
            serde_json::to_vec(&old).unwrap()
                == serde_json::to_vec(&current.checkpoint(&trainer).unwrap()).unwrap()
        );
        let mut restored = CpuCanonicalHost::from_checkpoint(old, &trainer).unwrap();
        current.step(&trainer, 1.0).unwrap();
        restored.step(&trainer, 1.0).unwrap();
        assert!(
            serde_json::to_vec(&restored.checkpoint(&trainer).unwrap()).unwrap()
                == serde_json::to_vec(&current.checkpoint(&trainer).unwrap()).unwrap()
        );
    }
}

#[test]
fn unbound_schema6_is_not_migrated_as_authenticated_mode1() {
    let (mut prior, trainer) = parent::fixture(None);
    // Construct the previous mode-1 boundary without a new mode seal.
    prior.joint_membership2177 = Some(1);
    prior.step(&trainer, 1.0).unwrap();
    let mut old = prior.checkpoint(&trainer).unwrap();
    old.schema_version = 6;
    assert!(CpuCanonicalHost::from_checkpoint(old, &trainer).is_err());
}

#[test]
fn contact2266_fresh_modes_bound1_cold_continuation_and_migration_boundaries() {
    for apply in [false, true] {
        let (mut host, trainer) = super::super::memory_bound_row_tests::fixture_bound2266(1.0);
        assert_eq!(
            trainer
                .phenotype()
                .candidate_decoder()
                .memory_channel()
                .unwrap()
                .max_candidate_gain(),
            1.0
        );
        let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
        host.enable_joint_membership_contact_comparison2266(&trainer, apply)
            .unwrap();
        let first = host.checkpoint(&trainer).unwrap();
        assert_eq!(first.schema_version, 9);
        assert_eq!(first.joint_membership2177, Some(if apply { 4 } else { 5 }));
        for _ in 0..4 {
            let step = host.step(&trainer, 1.0).unwrap();
            for row in step.decisions {
                assert_eq!(row.memory_plan2242.as_ref().unwrap().memory_bound, 1.0);
                let r = row.joint_membership2177.as_ref().unwrap();
                assert_eq!(r.schema_version, 3);
                assert_eq!(
                    r.comparison.as_ref().unwrap().applied_family2266,
                    Some(CandidateActionFamily::Contact)
                );
                assert_eq!(r.comparison.as_ref().unwrap().apply, apply);
                assert!(r.admission_actual_work <= r.admission_reserved_work);
                if !apply {
                    assert!(r.sources.is_empty());
                }
                // Plan-2 has its existing matching-reader envelope. Diagnostic
                // research rows still cannot enter the ordinary PPO cohort.
                let bytes = serde_json::to_vec(&row).unwrap();
                let decoded: CpuCollectedDecision = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(bytes, serde_json::to_vec(&decoded).unwrap());
                let value = CpuValueHead::from_checkpoint(crate::PpoValueHeadCheckpoint {
                    feature_count: trainer.phenotype().neuron_count(),
                    optimizer_step: 0,
                    parameters: vec![0.; (trainer.phenotype().neuron_count() as usize + 1) * 3],
                    last_updated_policy_version: None,
                })
                .unwrap();
                let phenotype = host
                    .world
                    .organism_registry()
                    .iter()
                    .next()
                    .unwrap()
                    .phenotype();
                assert!(cpu_training_windows(
                    &[decoded],
                    None,
                    &trainer,
                    &value,
                    phenotype,
                    2266,
                    crate::PpoConfig::default(),
                    2,
                    0
                )
                .is_err());
            }
        }
        let cp = host.checkpoint(&trainer).unwrap();
        let mut cold = CpuCanonicalHost::from_checkpoint(cp.clone(), &trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&cp).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        let a = host.step(&trainer, 1.0).unwrap();
        let b = cold.step(&trainer, 1.0).unwrap();
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        for mode in [
            None,
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(u16::MAX),
        ] {
            if mode == cp.joint_membership2177 {
                continue;
            }
            let mut bad = cp.clone();
            bad.joint_membership2177 = mode;
            bad.schema_version = match mode {
                Some(1) => 8,
                Some(2 | 3) => 7,
                Some(4 | 5) => 9,
                _ => 2,
            };
            assert!(
                CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err(),
                "accepted mode {mode:?}"
            );
        }
        for schema in [2, 5, 6, 7, 8, 10] {
            let mut bad = cp.clone();
            bad.schema_version = schema;
            assert!(CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err());
        }
        let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
        assert!(host
            .enable_joint_membership_contact_comparison2266(&trainer, !apply)
            .is_err());
        assert_eq!(
            before,
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
        );
        assert_eq!(founder, serde_json::to_vec(&trainer.checkpoint()).unwrap());
    }
    let (mut ordinary, trainer) = food_fixture(false, true);
    ordinary.step(&trainer, 1.).unwrap();
    let ordinary_cp = ordinary.checkpoint(&trainer).unwrap();
    let before = serde_json::to_vec(&ordinary_cp).unwrap();
    for mode in [4, 5] {
        let mut edited = ordinary_cp.clone();
        edited.schema_version = 9;
        edited.joint_membership2177 = Some(mode);
        assert!(CpuCanonicalHost::from_checkpoint(edited, &trainer).is_err());
    }
    assert!(ordinary
        .enable_joint_membership_contact_comparison2266(&trainer, true)
        .is_err());
    assert_eq!(
        before,
        serde_json::to_vec(&ordinary.checkpoint(&trainer).unwrap()).unwrap()
    );
}

#[test]
fn contact2266_same_initial_query_admission_and_work_as_all_family_controls() {
    let (base, trainer) = food_fixture(false, false);
    let mut rows = Vec::new();
    for mode in 2..=5 {
        let mut host = base.snapshot();
        if mode <= 3 {
            host.enable_joint_membership_comparison2182(&trainer, mode == 2)
                .unwrap();
        } else {
            host.enable_joint_membership_contact_comparison2266(&trainer, mode == 4)
                .unwrap();
        }
        rows.push(host.step(&trainer, 1.).unwrap().decisions.remove(0));
    }
    for row in &rows[1..] {
        assert_eq!(rows[0].action, row.action);
        assert_eq!(rows[0].replay_tick, row.replay_tick);
        assert_eq!(rows[0].patch.cognitive_work(), row.patch.cognitive_work());
        let a = rows[0].joint_membership2177.as_ref().unwrap();
        let b = row.joint_membership2177.as_ref().unwrap();
        assert_eq!(
            (
                a.query_work,
                a.admission_planning_work,
                a.admission_reserved_work,
                a.admission_actual_work
            ),
            (
                b.query_work,
                b.admission_planning_work,
                b.admission_reserved_work,
                b.admission_actual_work
            )
        );
    }
}

#[test]
fn contact2266_authentic_membership_preserves_every_untouched_candidate_exactly() {
    // Only real native World steps populate this bank. No fact bytes or acquired
    // checkpoint are edited to create eligible Contact or Ingest membership.
    let (mut host, trainer) = food_fixture(false, false);
    host.enable_joint_membership_contact_comparison2266(&trainer, false)
        .unwrap();
    let mut contact_sources = 0;
    let mut ingest_sources = 0;
    let mut changed_contact = 0;
    for _ in 0..32 {
        host.step(&trainer, 1.).unwrap();
        let drafts = host.semantic_prior_inputs(&trainer).unwrap();
        for (draft, _) in drafts {
            let r = &host.residents[&draft.organism_id().raw()];
            let before = serde_json::to_vec(&r.memory.export_active_bank().unwrap()).unwrap();
            let ordinary = r.memory.recall_frame(&draft).unwrap();
            let (all, queries) = r
                .memory
                .recall_frame_joint_comparison(&draft, true)
                .unwrap();
            let (contact, cq) = r
                .memory
                .recall_frame_joint_contact_comparison2266(&draft, true)
                .unwrap();
            let (withheld, wq) = r
                .memory
                .recall_frame_joint_contact_comparison2266(&draft, false)
                .unwrap();
            assert_eq!(queries, cq);
            assert_eq!(queries, wq);
            assert_eq!(ordinary.context(), withheld.context());
            assert_eq!(
                all.receipt().execution_evidence_work,
                contact.receipt().execution_evidence_work
            );
            assert_eq!(
                all.receipt().execution_evidence_work,
                withheld.receipt().execution_evidence_work
            );
            assert!(withheld.receipt().joint_membership_sources.is_empty());
            for (i, candidate) in draft.candidates().iter().enumerate() {
                let is_contact = candidate.family == CandidateActionFamily::Contact;
                if is_contact {
                    assert_eq!(contact.context().candidates[i], all.context().candidates[i]);
                    changed_contact += usize::from(
                        contact.context().candidates[i] != ordinary.context().candidates[i],
                    );
                } else {
                    assert_eq!(
                        contact.context().candidates[i],
                        ordinary.context().candidates[i]
                    );
                }
                let sources = queries
                    .iter()
                    .filter(|q| q.candidate_index == candidate.candidate_index)
                    .count();
                if is_contact {
                    contact_sources += sources;
                }
                if candidate.family == CandidateActionFamily::Ingest {
                    ingest_sources += sources;
                }
            }
            let expected: Vec<_> = queries
                .iter()
                .filter(|s| {
                    draft.candidates().iter().any(|c| {
                        c.candidate_index == s.candidate_index
                            && c.family == CandidateActionFamily::Contact
                    })
                })
                .cloned()
                .collect();
            assert_eq!(contact.receipt().joint_membership_sources, expected);
            assert_eq!(
                before,
                serde_json::to_vec(&r.memory.export_active_bank().unwrap()).unwrap()
            );
            ordinary.finalize(draft.clone()).unwrap();
            all.finalize(draft.clone()).unwrap();
            contact.finalize(draft.clone()).unwrap();
            withheld.finalize(draft).unwrap();
        }
    }
    assert!(contact_sources > 0, "no authentic eligible Contact sources");
    assert!(ingest_sources > 0, "no authentic eligible Ingest sources");
    assert!(changed_contact > 0, "Contact route never differed");
}

#[test]
fn contact2273_fresh_modes_bound1_cold_continuation_and_migration_boundaries() {
    for apply in [false, true] {
        let (mut host, trainer) = super::super::memory_bound_row_tests::fixture_bound2266(1.0);
        assert_eq!(
            trainer
                .phenotype()
                .candidate_decoder()
                .memory_channel()
                .unwrap()
                .max_candidate_gain(),
            1.0
        );
        let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
        host.enable_joint_membership_contact_query_comparison2273(&trainer, apply)
            .unwrap();
        let first = host.checkpoint(&trainer).unwrap();
        assert_eq!(first.schema_version, 10);
        assert_eq!(first.joint_membership2177, Some(if apply { 6 } else { 7 }));
        for _ in 0..4 {
            let step = host.step(&trainer, 1.0).unwrap();
            for row in step.decisions {
                assert_eq!(row.memory_plan2242.as_ref().unwrap().memory_bound, 1.0);
                let r = row.joint_membership2177.as_ref().unwrap();
                assert_eq!(r.schema_version, 4);
                assert_eq!(
                    r.comparison.as_ref().unwrap().applied_family2266,
                    Some(CandidateActionFamily::Contact)
                );
                assert_eq!(r.comparison.as_ref().unwrap().apply, apply);
                assert!(r.admission_actual_work <= r.admission_reserved_work);
                if !apply {
                    assert!(r.sources.is_empty());
                }
                // Plan-2 has its existing matching-reader envelope. Diagnostic
                // research rows still cannot enter the ordinary PPO cohort.
                let bytes = serde_json::to_vec(&row).unwrap();
                let decoded: CpuCollectedDecision = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(bytes, serde_json::to_vec(&decoded).unwrap());
                let value = CpuValueHead::from_checkpoint(crate::PpoValueHeadCheckpoint {
                    feature_count: trainer.phenotype().neuron_count(),
                    optimizer_step: 0,
                    parameters: vec![0.; (trainer.phenotype().neuron_count() as usize + 1) * 3],
                    last_updated_policy_version: None,
                })
                .unwrap();
                let phenotype = host
                    .world
                    .organism_registry()
                    .iter()
                    .next()
                    .unwrap()
                    .phenotype();
                assert!(cpu_training_windows(
                    &[decoded],
                    None,
                    &trainer,
                    &value,
                    phenotype,
                    2266,
                    crate::PpoConfig::default(),
                    2,
                    0
                )
                .is_err());
            }
        }
        let cp = host.checkpoint(&trainer).unwrap();
        let mut cold = CpuCanonicalHost::from_checkpoint(cp.clone(), &trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&cp).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        let a = host.step(&trainer, 1.0).unwrap();
        let b = cold.step(&trainer, 1.0).unwrap();
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        for mode in [
            None,
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            Some(8),
            Some(u16::MAX),
        ] {
            if mode == cp.joint_membership2177 {
                continue;
            }
            let mut bad = cp.clone();
            bad.joint_membership2177 = mode;
            bad.schema_version = match mode {
                Some(1) => 8,
                Some(2 | 3) => 7,
                Some(4 | 5) => 9,
                Some(6 | 7) => 10,
                _ => 2,
            };
            assert!(
                CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err(),
                "accepted mode {mode:?}"
            );
        }
        for schema in [2, 5, 6, 7, 8, 9, 11] {
            let mut bad = cp.clone();
            bad.schema_version = schema;
            assert!(CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err());
        }
        let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
        assert!(host
            .enable_joint_membership_contact_query_comparison2273(&trainer, !apply)
            .is_err());
        assert_eq!(
            before,
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
        );
        assert_eq!(founder, serde_json::to_vec(&trainer.checkpoint()).unwrap());
    }
    let (mut ordinary, trainer) = food_fixture(false, true);
    ordinary.step(&trainer, 1.).unwrap();
    let ordinary_cp = ordinary.checkpoint(&trainer).unwrap();
    let before = serde_json::to_vec(&ordinary_cp).unwrap();
    for mode in [6, 7] {
        let mut edited = ordinary_cp.clone();
        edited.schema_version = 10;
        edited.joint_membership2177 = Some(mode);
        assert!(CpuCanonicalHost::from_checkpoint(edited, &trainer).is_err());
    }
    assert!(ordinary
        .enable_joint_membership_contact_query_comparison2273(&trainer, true)
        .is_err());
    assert_eq!(
        before,
        serde_json::to_vec(&ordinary.checkpoint(&trainer).unwrap()).unwrap()
    );
}

#[test]
fn contact2273_authentic_membership_preserves_every_untouched_candidate_exactly() {
    // Only real native World steps populate this bank. No fact bytes or acquired
    // checkpoint are edited to create eligible Contact or Ingest membership.
    let (mut host, trainer) = food_fixture(false, false);
    host.enable_joint_membership_contact_comparison2266(&trainer, false)
        .unwrap();
    let mut contact_sources = 0;
    let mut ingest_sources = 0;
    let mut changed_contact = 0;
    for _ in 0..32 {
        host.step(&trainer, 1.).unwrap();
        let drafts = host.semantic_prior_inputs(&trainer).unwrap();
        for (draft, _) in drafts {
            let r = &host.residents[&draft.organism_id().raw()];
            let before = serde_json::to_vec(&r.memory.export_active_bank().unwrap()).unwrap();
            let ordinary = r.memory.recall_frame(&draft).unwrap();
            let non_contact = draft
                .clone()
                .with_candidates(
                    draft
                        .candidates()
                        .iter()
                        .filter(|c| c.family != CandidateActionFamily::Contact)
                        .cloned()
                        .enumerate()
                        .map(|(i, mut c)| {
                            c.candidate_index = i as u16;
                            c
                        })
                        .collect(),
                )
                .unwrap();
            let no_base = r.memory.recall_frame(&non_contact).unwrap();
            for apply in [true, false] {
                let (no, nq) = r
                    .memory
                    .recall_frame_joint_contact_query_comparison2273(&non_contact, apply)
                    .unwrap();
                assert!(nq.is_empty());
                assert_eq!(no.context(), no_base.context());
                assert_eq!(
                    no.receipt().execution_evidence_work,
                    no_base.receipt().execution_evidence_work
                );
                no.finalize(non_contact.clone()).unwrap();
            }

            let (all, queries) = r
                .memory
                .recall_frame_joint_comparison(&draft, true)
                .unwrap();
            let (contact, cq) = r
                .memory
                .recall_frame_joint_contact_query_comparison2273(&draft, true)
                .unwrap();
            let (withheld, wq) = r
                .memory
                .recall_frame_joint_contact_query_comparison2273(&draft, false)
                .unwrap();
            assert_eq!(cq, wq);
            let (old_contact, _) = r
                .memory
                .recall_frame_joint_contact_comparison2266(&draft, true)
                .unwrap();
            assert_eq!(contact.context(), old_contact.context());
            assert_eq!(contact.receipt().joint_membership_sources, cq);
            assert_eq!(ordinary.context(), withheld.context());
            assert!(
                all.receipt().execution_evidence_work > contact.receipt().execution_evidence_work
            );
            assert_eq!(
                contact.receipt().execution_evidence_work,
                withheld.receipt().execution_evidence_work
            );
            assert!(withheld.receipt().joint_membership_sources.is_empty());
            for (i, candidate) in draft.candidates().iter().enumerate() {
                let is_contact = candidate.family == CandidateActionFamily::Contact;
                if is_contact {
                    assert_eq!(contact.context().candidates[i], all.context().candidates[i]);
                    changed_contact += usize::from(
                        contact.context().candidates[i] != ordinary.context().candidates[i],
                    );
                } else {
                    assert_eq!(
                        contact.context().candidates[i],
                        ordinary.context().candidates[i]
                    );
                }
                let sources = queries
                    .iter()
                    .filter(|q| q.candidate_index == candidate.candidate_index)
                    .count();
                if is_contact {
                    contact_sources += sources;
                }
                if candidate.family == CandidateActionFamily::Ingest {
                    ingest_sources += sources;
                }
            }
            let expected: Vec<_> = queries
                .iter()
                .filter(|s| {
                    draft.candidates().iter().any(|c| {
                        c.candidate_index == s.candidate_index
                            && c.family == CandidateActionFamily::Contact
                    })
                })
                .cloned()
                .collect();
            assert_eq!(contact.receipt().joint_membership_sources, expected);
            assert_eq!(
                before,
                serde_json::to_vec(&r.memory.export_active_bank().unwrap()).unwrap()
            );
            ordinary.finalize(draft.clone()).unwrap();
            all.finalize(draft.clone()).unwrap();
            contact.finalize(draft.clone()).unwrap();
            withheld.finalize(draft).unwrap();
        }
    }
    assert!(contact_sources > 0, "no authentic eligible Contact sources");
    assert!(ingest_sources > 0, "no authentic eligible Ingest sources");
    assert!(changed_contact > 0, "Contact route never differed");
}

#[test]
fn contact2273_empty_bank_equal_control_work_and_unchanged_admission() {
    let (base, trainer) = food_fixture(false, false);
    let mut rows = Vec::new();
    for mode in [4, 6, 7] {
        let mut host = base.snapshot();
        if mode == 4 {
            host.enable_joint_membership_contact_comparison2266(&trainer, true)
                .unwrap();
        } else {
            host.enable_joint_membership_contact_query_comparison2273(&trainer, mode == 6)
                .unwrap();
        }
        rows.push(host.step(&trainer, 1.).unwrap().decisions.remove(0));
    }
    let old = rows[0].joint_membership2177.as_ref().unwrap();
    let applied = rows[1].joint_membership2177.as_ref().unwrap();
    let withheld = rows[2].joint_membership2177.as_ref().unwrap();
    assert_eq!(applied.query_work, 0);
    assert_eq!(applied.query_work, withheld.query_work);
    assert!(old.query_work > applied.query_work);
    assert_eq!(
        rows[1].patch.cognitive_work(),
        rows[2].patch.cognitive_work()
    );
    for r in [applied, withheld] {
        assert!(r.sources.is_empty());
        assert!(r.comparison.as_ref().unwrap().queried_sources.is_empty());
        assert_eq!(
            (
                r.admission_planning_work,
                r.admission_reserved_work,
                r.admission_actual_work
            ),
            (
                old.admission_planning_work,
                old.admission_reserved_work,
                old.admission_actual_work
            )
        );
    }
    assert_eq!(rows[0].action, rows[1].action);
    assert_eq!(rows[1].action, rows[2].action);
}

#[test]
fn success2275_fresh_modes_bound1_cold_continuation_and_migration_boundaries() {
    for apply in [false, true] {
        let (mut host, trainer) = super::super::memory_bound_row_tests::fixture_bound2266(1.0);
        assert_eq!(
            trainer
                .phenotype()
                .candidate_decoder()
                .memory_channel()
                .unwrap()
                .max_candidate_gain(),
            1.0
        );
        let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
        host.enable_joint_contact_success_projection2275(&trainer, apply)
            .unwrap();
        let first = host.checkpoint(&trainer).unwrap();
        assert_eq!(first.schema_version, 12);
        assert_eq!(first.joint_membership2177, Some(if apply { 8 } else { 9 }));
        for _ in 0..4 {
            let step = host.step(&trainer, 1.0).unwrap();
            for row in step.decisions {
                assert_eq!(row.memory_plan2242.as_ref().unwrap().memory_bound, 1.0);
                let r = row.joint_membership2177.as_ref().unwrap();
                assert_eq!(r.schema_version, 6);
                assert_eq!(
                    r.comparison.as_ref().unwrap().applied_family2266,
                    Some(CandidateActionFamily::Contact)
                );
                assert_eq!(r.comparison.as_ref().unwrap().apply, true);
                assert!(r.admission_actual_work <= r.admission_reserved_work);
                assert_eq!(r.success_projection2275.as_ref().unwrap().apply, apply);
                // Plan-2 has its existing matching-reader envelope. Diagnostic
                // research rows still cannot enter the ordinary PPO cohort.
                let bytes = serde_json::to_vec(&row).unwrap();
                let decoded: CpuCollectedDecision = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(bytes, serde_json::to_vec(&decoded).unwrap());
                let value = CpuValueHead::from_checkpoint(crate::PpoValueHeadCheckpoint {
                    feature_count: trainer.phenotype().neuron_count(),
                    optimizer_step: 0,
                    parameters: vec![0.; (trainer.phenotype().neuron_count() as usize + 1) * 3],
                    last_updated_policy_version: None,
                })
                .unwrap();
                let phenotype = host
                    .world
                    .organism_registry()
                    .iter()
                    .next()
                    .unwrap()
                    .phenotype();
                assert!(cpu_training_windows(
                    &[decoded],
                    None,
                    &trainer,
                    &value,
                    phenotype,
                    2266,
                    crate::PpoConfig::default(),
                    2,
                    0
                )
                .is_err());
            }
        }
        let cp = host.checkpoint(&trainer).unwrap();
        let mut cold = CpuCanonicalHost::from_checkpoint(cp.clone(), &trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&cp).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        let a = host.step(&trainer, 1.0).unwrap();
        let b = cold.step(&trainer, 1.0).unwrap();
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        for mode in [
            None,
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            Some(8),
            Some(9),
            Some(10),
            Some(u16::MAX),
        ] {
            if mode == cp.joint_membership2177 {
                continue;
            }
            let mut bad = cp.clone();
            bad.joint_membership2177 = mode;
            bad.schema_version = match mode {
                Some(1) => 8,
                Some(2 | 3) => 7,
                Some(4 | 5) => 9,
                Some(6 | 7) => 10,
                Some(8 | 9) => 12,
                _ => 2,
            };
            assert!(
                CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err(),
                "accepted mode {mode:?}"
            );
        }
        for schema in [2, 5, 6, 7, 8, 9, 10, 11, 13] {
            let mut bad = cp.clone();
            bad.schema_version = schema;
            assert!(CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err());
        }
        let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
        assert!(host
            .enable_joint_contact_success_projection2275(&trainer, !apply)
            .is_err());
        assert_eq!(
            before,
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
        );
        assert_eq!(founder, serde_json::to_vec(&trainer.checkpoint()).unwrap());
    }
    let (mut ordinary, trainer) = food_fixture(false, true);
    ordinary.step(&trainer, 1.).unwrap();
    let ordinary_cp = ordinary.checkpoint(&trainer).unwrap();
    let before = serde_json::to_vec(&ordinary_cp).unwrap();
    for mode in [8, 9] {
        let mut edited = ordinary_cp.clone();
        edited.schema_version = 12;
        edited.joint_membership2177 = Some(mode);
        assert!(CpuCanonicalHost::from_checkpoint(edited, &trainer).is_err());
    }
    assert!(ordinary
        .enable_joint_contact_success_projection2275(&trainer, true)
        .is_err());
    assert_eq!(
        before,
        serde_json::to_vec(&ordinary.checkpoint(&trainer).unwrap()).unwrap()
    );
}

#[test]
fn success2275_same_bank_component_only_projection_and_truthful_sources() {
    let (mut host, trainer) = food_fixture(false, false);
    host.enable_joint_membership_contact_comparison2266(&trainer, false)
        .unwrap();

    let mut absent = 0;
    for _ in 0..48 {
        host.step(&trainer, 1.).unwrap();
        for (draft, _) in host.semantic_prior_inputs(&trainer).unwrap() {
            let r = &host.residents[&draft.organism_id().raw()];
            let before = serde_json::to_vec(&r.memory.export_active_bank().unwrap()).unwrap();
            let (joint, queries, ordinary) = r
                .memory
                .recall_frame_joint_contact_success_basis2275(&draft)
                .unwrap();
            let (old, old_queries) = r
                .memory
                .recall_frame_joint_contact_comparison2266(&draft, true)
                .unwrap();
            assert_eq!(joint.context(), old.context());
            assert_eq!(
                alife_runtime::canonical_preparation::cognitive_memory_context_for_recall(
                    draft.organism_id(),
                    ExperienceSequenceId(9000),
                    &joint
                )
                .unwrap(),
                alife_runtime::canonical_preparation::cognitive_memory_context_for_recall(
                    draft.organism_id(),
                    ExperienceSequenceId(9000),
                    &old
                )
                .unwrap()
            );
            assert_eq!(queries, old_queries);
            assert_eq!(
                joint.receipt().joint_membership_sources,
                old.receipt().joint_membership_sources
            );

            let (frame, joint) = joint.finalize(draft.clone()).unwrap();
            let (_, old) = old.finalize(draft.clone()).unwrap();
            assert_eq!(joint.cognitive_context(), old.cognitive_context());
            let (ordinary_frame, ordinary) = ordinary.finalize(draft.clone()).unwrap();
            assert!(joint.family_sources().is_none());
            let tick = crate::TrainingReplayTick {
                encoded_inputs: vec![0.25],
                projection_gain: 1.,
                local_threshold_shift: 0.,
                microstep_count: 1,
                enabled_routes: vec![],
                effective_weight_offsets: vec![],
                structural_synapses: vec![],
                candidates: frame
                    .candidates()
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let mut decoder_inputs = std::array::from_fn(|n| n as f32 / 54.);
                        let m = &joint.context().candidates[i];
                        decoder_inputs[33] =
                            m.family_value[1] * m.family_confidence.raw().clamp(0., 1.);
                        crate::TrainingReplayCandidate {
                            family: c.family,
                            decoder_inputs,
                            innate_bias: 0.,
                            memory_readout: None,
                        }
                    })
                    .collect(),
            };
            let mut applied = tick.clone();
            let mut withheld = tick.clone();
            let a = project_joint_contact_success2275(
                &frame,
                &joint,
                &ordinary_frame,
                &ordinary,
                &mut applied,
                true,
            )
            .unwrap();
            let b = project_joint_contact_success2275(
                &frame,
                &joint,
                &ordinary_frame,
                &ordinary,
                &mut withheld,
                false,
            )
            .unwrap();
            assert_eq!(tick, withheld);
            assert_eq!(a.work, b.work);
            for c in &a.candidates {
                let i = c.candidate_index as usize;
                assert_eq!(
                    applied.candidates[i].decoder_inputs[33],
                    c.ordinary_success * c.ordinary_confidence
                );
                if c.ordinary_source_count == 0 {
                    absent += 1;
                    assert_eq!(c.projected_weighted_success, 0.);
                    assert!(c.ordinary_best_source.is_none());
                    assert!(c.ordinary_sources.sources.iter().all(Option::is_none));
                } else if c.projected_weighted_success > 0. {
                    assert!(c.ordinary_best_source.is_some());
                }
            }
            for (i, c) in applied.candidates.iter_mut().enumerate() {
                if c.family == CandidateActionFamily::Contact {
                    c.decoder_inputs[33] = tick.candidates[i].decoder_inputs[33];
                }
            }
            assert_eq!(applied, tick); // All other 53 lanes, all non-Contact rows, and global context exact.
            if draft.candidates().len() > 1 {
                let changed = draft
                    .clone()
                    .with_candidates(draft.candidates()[..draft.candidates().len() - 1].to_vec())
                    .unwrap();
                let (foreign_frame, foreign) = r
                    .memory
                    .recall_frame_with_family_sources(&changed)
                    .unwrap()
                    .finalize(changed)
                    .unwrap();
                assert!(project_joint_contact_success2275(
                    &frame,
                    &joint,
                    &foreign_frame,
                    &foreign,
                    &mut tick.clone(),
                    true
                )
                .is_err());
            }
            let mut bad = tick.clone();
            bad.candidates.pop();
            assert!(project_joint_contact_success2275(
                &frame,
                &joint,
                &ordinary_frame,
                &ordinary,
                &mut bad,
                true
            )
            .is_err());
            assert_eq!(
                before,
                serde_json::to_vec(&r.memory.export_active_bank().unwrap()).unwrap()
            );
        }
    }
    assert!(absent > 0, "no absent-support fixture");
}

#[test]
fn success2275_native_applied_withheld_equal_work_and_only_component_difference() {
    let (base, trainer) = food_fixture(false, false);
    let mut applied = base.snapshot();
    let mut withheld = base.snapshot();
    applied
        .enable_joint_contact_success_projection2275(&trainer, true)
        .unwrap();
    withheld
        .enable_joint_contact_success_projection2275(&trainer, false)
        .unwrap();
    let a = applied.step(&trainer, 1.).unwrap();
    let b = withheld.step(&trainer, 1.).unwrap();
    assert_eq!(a.decisions.len(), b.decisions.len());
    for (a, b) in a.decisions.iter().zip(&b.decisions) {
        assert_eq!(a.initial, b.initial);
        assert_eq!(a.brain_work, b.brain_work);
        let ar = a.joint_membership2177.as_ref().unwrap();
        let br = b.joint_membership2177.as_ref().unwrap();
        assert_eq!(ar.query_work, br.query_work);
        assert_eq!(ar.admission_planning_work, br.admission_planning_work);
        assert_eq!(ar.admission_actual_work, br.admission_actual_work);
        let mut actual = a.replay_tick.clone();
        for (x, y) in actual.candidates.iter_mut().zip(&b.replay_tick.candidates) {
            if x.family == CandidateActionFamily::Contact {
                x.decoder_inputs[33] = y.decoder_inputs[33];
            }
        }
        assert_eq!(actual, b.replay_tick);
    }
}

fn success2275_checked_fixture_patch(
    recall_bank: &MemoryBank,
    sequence_raw: u64,
    draft: PerceptionFrameDraft,
    reward: f32,
    pain: f32,
    edit_outcome: impl FnOnce(&mut PostActionOutcome),
) -> ExperiencePatch {
    let owner = draft.organism_id();
    let tick = draft.tick();
    let tick_raw = tick.raw();
    let sequence = ExperienceSequenceId(sequence_raw);
    let target_entity = draft.candidates()[0].target.entity;
    let (frame, finalized) = recall_bank
        .recall_frame(&draft)
        .unwrap()
        .finalize(draft)
        .unwrap();
    let selected = &frame.candidates()[0];
    let decision = DecisionSnapshot::from_neural_selection(
        sequence,
        PhenotypeHash([1, 2, 3, 4]),
        sequence_raw,
        (sequence_raw & 1) as u8,
        &frame,
        NeuralActionSelection {
            candidate_index: 0,
            logit: 0.7,
            confidence: Confidence::new(0.8).unwrap(),
            active_tiles: 8,
            active_synapses: 64,
        },
        selected
            .to_command(owner, Confidence::new(0.8).unwrap())
            .unwrap(),
    )
    .unwrap()
    .with_finalized_memory_recall(&frame, &finalized, 0)
    .unwrap();
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let genome = BrainGenome::scaffold(321, spec.id);
    let development = DevelopmentState::new(genome.id, tick, NormalizedScalar::new(0.5).unwrap())
        .with_enabled_lobes([
            LobeKind::PerceptualIntegration,
            LobeKind::TemporalPredictive,
            LobeKind::ActionPlanning,
        ]);
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence,
        spec.id,
        PhenotypeHash([1, 2, 3, 4]),
        genome.id,
        genome.schema_version,
        development,
        frame,
    )
    .unwrap();
    let mut outcome = PostActionOutcome::new(
        owner,
        sequence,
        Tick::new(tick_raw + 1),
        reward >= 0.0,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::Touch,
            target_entity,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta {
            drives: DriveDelta {
                pain,
                fear: pain,
                brain_atp: -0.1,
                ..DriveDelta::zero()
            },
            hormones: EndocrineDelta::zero(),
        },
        SignedValence::new(reward).unwrap(),
        NormalizedScalar::new(if reward < 0.0 { 0.5 } else { 0.0 }).unwrap(),
        NormalizedScalar::new(pain).unwrap(),
        SignedValence::new(-0.1).unwrap(),
        NormalizedScalar::new(pain.max(0.1)).unwrap(),
    )
    .unwrap();
    edit_outcome(&mut outcome);
    ExperiencePatchBuilder::new(sequence)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap()
}

#[test]
fn success2275_positive_contact_support_is_not_zero_ablation() {
    // A deliberately constructed, contract-sealed unit fixture, not a sampled
    // body outcome or an edit to any acquired checkpoint. Contact is explicitly
    // the representative action so ordinary recall has positive success support.
    let (mut host, trainer) = food_fixture(false, false);
    let (draft, _) = host.semantic_prior_inputs(&trainer).unwrap().remove(0);
    let mut contact = draft
        .candidates()
        .iter()
        .find(|c| c.family == CandidateActionFamily::Contact)
        .unwrap()
        .clone();
    contact.candidate_index = 0;
    let draft = draft.with_candidates(vec![contact]).unwrap();
    let mut bank = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    )
    .unwrap();
    let patch = success2275_checked_fixture_patch(&bank, 1, draft.clone(), 0.8, 0., |_| {});
    patch.validate_contract().unwrap();
    bank.observe_sealed_patch(&patch).unwrap();
    let (joint, _, ordinary) = bank
        .recall_frame_joint_contact_success_basis2275(&draft)
        .unwrap();
    let (frame, joint) = joint.finalize(draft.clone()).unwrap();
    let (of, ordinary) = ordinary.finalize(draft).unwrap();
    let c = &ordinary.context().candidates[0];
    assert!(c.family_source_count > 0);
    assert!(c.family_value[1] > 0.);
    assert!(c.family_confidence.raw() > 0.);
    let mut inputs = [0.125; 54];
    inputs[33] = joint.context().candidates[0].family_value[1]
        * joint.context().candidates[0].family_confidence.raw();
    let mut tick = crate::TrainingReplayTick {
        encoded_inputs: vec![0.5],
        projection_gain: 1.,
        local_threshold_shift: 0.,
        microstep_count: 1,
        enabled_routes: vec![],
        effective_weight_offsets: vec![],
        structural_synapses: vec![],
        candidates: vec![crate::TrainingReplayCandidate {
            family: CandidateActionFamily::Contact,
            decoder_inputs: inputs,
            innate_bias: 0.,
            memory_readout: None,
        }],
    };
    let before = tick.clone();
    let receipt =
        project_joint_contact_success2275(&frame, &joint, &of, &ordinary, &mut tick, true).unwrap();
    let r = &receipt.candidates[0];
    assert!(r.projected_weighted_success > 0.);
    assert_eq!(
        tick.candidates[0].decoder_inputs[33],
        c.family_value[1] * c.family_confidence.raw()
    );
    assert!(r.ordinary_sources.sources.iter().any(Option::is_some));
    tick.candidates[0].decoder_inputs[33] = before.candidates[0].decoder_inputs[33];
    assert_eq!(tick, before);
}

#[test]
fn success2275_projection_work_is_charged_once_in_actual_biological_interval() {
    for apply in [false, true] {
        let (mut host, trainer) = food_fixture(false, false);
        host.enable_joint_contact_success_projection2275(&trainer, apply)
            .unwrap();
        for _ in 0..8 {
            for row in host.step(&trainer, 1.).unwrap().decisions {
                let receipt = row.joint_membership2177.as_ref().unwrap();
                let projection = receipt.success_projection2275.as_ref().unwrap();
                assert!(projection.work > 0);
                let ordinary = &projection.ordinary_query;
                let base = u64::from(ordinary.exact_bucket_reads)
                    + u64::from(ordinary.neighbor_bucket_reads)
                    + u64::from(ordinary.similarity_evaluations);
                // This existing row field is the exact Core recall work, including
                // basis capture/finalization, but excluding CPU-only projection.
                let core_and_basis = row.executed_action_recall_work;
                assert_eq!(receipt.query_work, core_and_basis + projection.work);
                let expected = base
                    + core_and_basis
                    + projection.work
                    + receipt.admission_planning_work
                    + receipt.admission_reserved_work;
                let charged = row.patch.cognitive_work().unwrap();
                assert_eq!(
                    charged.memory_ops, expected,
                    "CPU projection must enter actual physiology exactly once"
                );
                let record = host.world.organism_registry().get(row.organism_id).unwrap();
                assert_eq!(record.cognitive_work(), charged);
                let (policy, actual, before, after) = row.cognitive_cost_capture.unwrap();
                let phenotype = record.phenotype();
                let requested = policy.energy_debit(charged).unwrap()
                    * phenotype.body.metabolic_turnover
                    * (ContinuousLocus::midpoint_value(0.52, 0.60)
                        - phenotype.chemistry.brain_atp_efficiency)
                        .exp2();
                assert_eq!(actual, requested.min(before));
                assert_eq!(record.cognitive_energy_debit(), actual);
                assert!((before - after - actual).abs() < 2.0e-6);
            }
        }
    }
}

#[test]
fn non_ingest2278_fresh_modes_bound1_cold_continuation_and_migration_boundaries() {
    for apply in [false, true] {
        let (mut host, trainer) = super::super::memory_bound_row_tests::fixture_bound2266(1.0);
        assert_eq!(
            trainer
                .phenotype()
                .candidate_decoder()
                .memory_channel()
                .unwrap()
                .max_candidate_gain(),
            1.0
        );
        let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
        host.enable_joint_membership_non_ingest_comparison2278(&trainer, apply)
            .unwrap();
        let first = host.checkpoint(&trainer).unwrap();
        assert_eq!(first.schema_version, 13);
        assert_eq!(
            first.joint_membership2177,
            Some(if apply { 10 } else { 11 })
        );
        for _ in 0..4 {
            let step = host.step(&trainer, 1.0).unwrap();
            for row in step.decisions {
                assert_eq!(row.memory_plan2242.as_ref().unwrap().memory_bound, 1.0);
                let r = row.joint_membership2177.as_ref().unwrap();
                assert_eq!(r.schema_version, 7);
                assert_eq!(r.comparison.as_ref().unwrap().applied_family2266, None);
                assert_eq!(r.comparison.as_ref().unwrap().apply, apply);
                assert!(r.admission_actual_work <= r.admission_reserved_work);
                assert!(r.success_projection2275.is_none());
                assert_eq!(
                    r.comparison.as_ref().unwrap().applied_family_mask2278,
                    Some(191)
                );
                // Plan-2 has its existing matching-reader envelope. Diagnostic
                // research rows still cannot enter the ordinary PPO cohort.
                let bytes = serde_json::to_vec(&row).unwrap();
                let decoded: CpuCollectedDecision = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(bytes, serde_json::to_vec(&decoded).unwrap());
                let value = CpuValueHead::from_checkpoint(crate::PpoValueHeadCheckpoint {
                    feature_count: trainer.phenotype().neuron_count(),
                    optimizer_step: 0,
                    parameters: vec![0.; (trainer.phenotype().neuron_count() as usize + 1) * 3],
                    last_updated_policy_version: None,
                })
                .unwrap();
                let phenotype = host
                    .world
                    .organism_registry()
                    .iter()
                    .next()
                    .unwrap()
                    .phenotype();
                assert!(cpu_training_windows(
                    &[decoded],
                    None,
                    &trainer,
                    &value,
                    phenotype,
                    2266,
                    crate::PpoConfig::default(),
                    2,
                    0
                )
                .is_err());
            }
        }
        let cp = host.checkpoint(&trainer).unwrap();
        let mut cold = CpuCanonicalHost::from_checkpoint(cp.clone(), &trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&cp).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        let a = host.step(&trainer, 1.0).unwrap();
        let b = cold.step(&trainer, 1.0).unwrap();
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        for mode in [
            None,
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            Some(8),
            Some(9),
            Some(10),
            Some(11),
            Some(12),
            Some(u16::MAX),
        ] {
            if mode == cp.joint_membership2177 {
                continue;
            }
            let mut bad = cp.clone();
            bad.joint_membership2177 = mode;
            bad.schema_version = match mode {
                Some(1) => 8,
                Some(2 | 3) => 7,
                Some(4 | 5) => 9,
                Some(6 | 7) => 10,
                Some(8 | 9) => 12,
                Some(10 | 11) => 13,
                _ => 2,
            };
            assert!(
                CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err(),
                "accepted mode {mode:?}"
            );
        }
        for schema in [2, 5, 6, 7, 8, 9, 10, 11, 12, 14] {
            let mut bad = cp.clone();
            bad.schema_version = schema;
            assert!(CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err());
        }
        let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
        assert!(host
            .enable_joint_membership_non_ingest_comparison2278(&trainer, !apply)
            .is_err());
        assert_eq!(
            before,
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
        );
        assert_eq!(founder, serde_json::to_vec(&trainer.checkpoint()).unwrap());
    }
    let (mut ordinary, trainer) = food_fixture(false, true);
    ordinary.step(&trainer, 1.).unwrap();
    let ordinary_cp = ordinary.checkpoint(&trainer).unwrap();
    let before = serde_json::to_vec(&ordinary_cp).unwrap();
    for mode in [10, 11] {
        let mut edited = ordinary_cp.clone();
        edited.schema_version = 13;
        edited.joint_membership2177 = Some(mode);
        assert!(CpuCanonicalHost::from_checkpoint(edited, &trainer).is_err());
    }
    assert!(ordinary
        .enable_joint_membership_non_ingest_comparison2278(&trainer, true)
        .is_err());
    assert_eq!(
        before,
        serde_json::to_vec(&ordinary.checkpoint(&trainer).unwrap()).unwrap()
    );
}

#[test]
fn non_ingest2278_exact_application_mask_full_query_provenance_and_work() {
    let (mut host, trainer) = food_fixture(false, false);
    host.enable_joint_membership_comparison2182(&trainer, false)
        .unwrap();
    let (mut ingest, mut other, mut changed) = (0, 0, 0);
    for step in 0..33 {
        if step > 0 {
            host.step(&trainer, 1.).unwrap();
        }
        for (draft, _) in host.semantic_prior_inputs(&trainer).unwrap() {
            let bank = &host.residents[&draft.organism_id().raw()].memory;
            let before = serde_json::to_vec(&bank.export_active_bank().unwrap()).unwrap();
            let ordinary = bank.recall_frame(&draft).unwrap();
            let (global, queries) = bank.recall_frame_joint_comparison(&draft, true).unwrap();
            let (applied, aq) = bank
                .recall_frame_joint_non_ingest_comparison2278(&draft, true)
                .unwrap();
            let (withheld, wq) = bank
                .recall_frame_joint_non_ingest_comparison2278(&draft, false)
                .unwrap();
            assert_eq!(aq, queries);
            assert_eq!(wq, queries);
            assert_eq!(
                applied.receipt().execution_evidence_work,
                global.receipt().execution_evidence_work
            );
            assert_eq!(
                withheld.receipt().execution_evidence_work,
                global.receipt().execution_evidence_work
            );
            assert_eq!(withheld.context(), ordinary.context());
            assert!(withheld.receipt().joint_membership_sources.is_empty());
            let expected: Vec<_> = queries
                .iter()
                .filter(|q| {
                    draft.candidates()[q.candidate_index as usize].family
                        != CandidateActionFamily::Ingest
                })
                .cloned()
                .collect();
            assert_eq!(applied.receipt().joint_membership_sources, expected);
            for (i, c) in draft.candidates().iter().enumerate() {
                let is_ingest = c.family == CandidateActionFamily::Ingest;
                let ref_context = if is_ingest {
                    ordinary.context()
                } else {
                    global.context()
                };
                assert_eq!(applied.context().candidates[i], ref_context.candidates[i]);
                let count = queries
                    .iter()
                    .filter(|q| q.candidate_index as usize == i)
                    .count();
                if is_ingest {
                    ingest += count;
                } else {
                    other += count;
                    changed += usize::from(
                        global.context().candidates[i] != ordinary.context().candidates[i],
                    );
                }
                let mask = 191_u8 & (1_u8 << c.family.raw());
                assert_eq!(mask != 0, !is_ingest);
            }
            if step == 0 {
                assert!(queries.is_empty());
                assert_eq!(applied.context(), ordinary.context());
            }
            assert_eq!(
                before,
                serde_json::to_vec(&bank.export_active_bank().unwrap()).unwrap()
            );
            applied.finalize(draft.clone()).unwrap();
            withheld.finalize(draft).unwrap();
        }
    }
    assert!(
        ingest > 0 && other > 0 && changed > 0,
        "authentic eligible Ingest and non-Ingest required"
    );
}

#[test]
fn non_ingest2278_native_no_projection_exact_cost_and_atomic_failure() {
    let (base, trainer) = food_fixture(false, false);
    let mut first = None;
    for apply in [false, true] {
        let mut host = base.snapshot();
        host.enable_joint_membership_non_ingest_comparison2278(&trainer, apply)
            .unwrap();
        let cp = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
        let mut failed = host.snapshot();
        failed.fail_joint_membership_after_admission = true;
        assert!(failed.step(&trainer, 1.).is_err());
        assert_eq!(
            cp,
            serde_json::to_vec(&failed.checkpoint(&trainer).unwrap()).unwrap()
        );
        let row = host.step(&trainer, 1.).unwrap().decisions.remove(0);
        let receipt = row.joint_membership2177.as_ref().unwrap();
        assert!(receipt.success_projection2275.is_none());
        assert_eq!(receipt.query_work, row.executed_action_recall_work);
        let (mut ordinary, _) = food_fixture(false, false);
        let ordinary_row = ordinary.step(&trainer, 1.).unwrap().decisions.remove(0);
        assert_eq!(row.replay_tick, ordinary_row.replay_tick);
        let charged = row.patch.cognitive_work().unwrap();
        assert_eq!(
            charged.memory_ops,
            ordinary_row.patch.cognitive_work().unwrap().memory_ops
                + receipt.query_work
                + receipt.admission_planning_work
                + receipt.admission_reserved_work
        );
        let record = host.world.organism_registry().get(row.organism_id).unwrap();
        let (policy, debit, before, after) = row.cognitive_cost_capture.unwrap();
        let phenotype = record.phenotype();
        let expected = policy.energy_debit(charged).unwrap()
            * phenotype.body.metabolic_turnover
            * (ContinuousLocus::midpoint_value(0.52, 0.60)
                - phenotype.chemistry.brain_atp_efficiency)
                .exp2();
        assert_eq!(debit, expected.min(before));
        assert!((before - after - debit).abs() < 2e-6);
        let tuple = (
            receipt.query_work,
            receipt.admission_planning_work,
            receipt.admission_reserved_work,
            receipt.admission_actual_work,
            charged.memory_ops,
        );
        if let Some(previous) = first {
            assert_eq!(tuple, previous);
        } else {
            first = Some(tuple);
        }
    }
}

#[test]
fn non_ingest2278_populated_intervals_bill_exact_routed_query_and_physiology() {
    for apply in [false, true] {
        let (mut host, trainer) = food_fixture(false, false);
        host.enable_joint_membership_non_ingest_comparison2278(&trainer, apply)
            .unwrap();
        let mut genuine_queries = 0;
        for _ in 0..16 {
            // Diagnostic snapshot only. Production performs no extra lookup.
            let pre_step = host.snapshot();
            for row in host.step(&trainer, 1.).unwrap().decisions {
                let frame = row.patch.pre_action().perception();
                // The serialized, validated base preserves the ACTUAL routed
                // candidates, semantic context and all profile capabilities.
                let wire = serde_json::to_value(frame).unwrap();
                let draft: PerceptionFrameDraft =
                    serde_json::from_value(wire["base"].clone()).unwrap();
                assert_eq!(draft.base_digest(), frame.base_digest());
                let bank = &pre_step.residents[&row.organism_id.raw()].memory;
                let before = serde_json::to_vec(&bank.export_active_bank().unwrap()).unwrap();
                let ordinary = bank.recall_frame(&draft).unwrap();
                let raw = ordinary.receipt();
                assert_eq!(raw.base_frame_digest, frame.base_digest());
                assert_eq!(
                    before,
                    serde_json::to_vec(&bank.export_active_bank().unwrap()).unwrap()
                );
                let receipt = row.joint_membership2177.as_ref().unwrap();
                assert!(receipt.success_projection2275.is_none());
                genuine_queries += receipt.comparison.as_ref().unwrap().queried_sources.len();
                assert_eq!(receipt.query_work, row.executed_action_recall_work);
                let base = u64::from(raw.exact_bucket_reads)
                    + u64::from(raw.neighbor_bucket_reads)
                    + u64::from(raw.similarity_evaluations);
                let expected = base
                    + receipt.query_work
                    + receipt.admission_planning_work
                    + receipt.admission_reserved_work;
                let charged = row.patch.cognitive_work().unwrap();
                assert_eq!(
                    charged.memory_ops, expected,
                    "populated actual interval must include full query and admission exactly once"
                );
                let record = host.world.organism_registry().get(row.organism_id).unwrap();
                assert_eq!(record.cognitive_work(), charged);
                let (policy, debit, before_energy, after_energy) =
                    row.cognitive_cost_capture.unwrap();
                let phenotype = record.phenotype();
                let requested = policy.energy_debit(charged).unwrap()
                    * phenotype.body.metabolic_turnover
                    * (ContinuousLocus::midpoint_value(0.52, 0.60)
                        - phenotype.chemistry.brain_atp_efficiency)
                        .exp2();
                assert_eq!(debit, requested.min(before_energy));
                assert_eq!(record.cognitive_energy_debit(), debit);
                assert!((before_energy - after_energy - debit).abs() < 2e-6);
            }
        }
        assert!(
            genuine_queries > 0,
            "populated eligible membership required for each arm"
        );
    }
}

#[test]
fn ingest2286_fresh_modes_bound1_cold_continuation_and_migration_boundaries() {
    for apply in [false, true] {
        let (mut host, trainer) = super::super::memory_bound_row_tests::fixture_bound2266(1.0);
        assert_eq!(
            trainer
                .phenotype()
                .candidate_decoder()
                .memory_channel()
                .unwrap()
                .max_candidate_gain(),
            1.0
        );
        let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
        host.enable_joint_ingest_outcome_projection2286(&trainer, apply)
            .unwrap();
        let first = host.checkpoint(&trainer).unwrap();
        assert_eq!(first.schema_version, 14);
        assert_eq!(
            first.joint_membership2177,
            Some(if apply { 12 } else { 13 })
        );
        for _ in 0..4 {
            let step = host.step(&trainer, 1.0).unwrap();
            for row in step.decisions {
                assert_eq!(row.memory_plan2242.as_ref().unwrap().memory_bound, 1.0);
                let r = row.joint_membership2177.as_ref().unwrap();
                assert_eq!(r.schema_version, 8);
                assert_eq!(r.comparison.as_ref().unwrap().applied_family2266, None);
                assert_eq!(r.comparison.as_ref().unwrap().apply, true);
                assert!(r.admission_actual_work <= r.admission_reserved_work);
                assert!(r.success_projection2275.is_none());
                let projection = r.ingest_outcome_projection2286.as_ref().unwrap();
                assert_eq!(projection.apply, apply);
                assert!(projection.requires_positive_ordinary_support);
                assert_eq!(projection.component_mask, 9);
                assert_eq!(projection.cost_revision, 1);
                assert_eq!(
                    projection.actual_cpu_ops,
                    projection.ordinary.work.cpu_transform
                );
                for c in &projection.candidates {
                    assert_eq!(c.applied, apply && c.support_eligible);
                }
                assert_eq!(r.comparison.as_ref().unwrap().applied_family_mask2278, None);
                // Plan-2 has its existing matching-reader envelope. Diagnostic
                // research rows still cannot enter the ordinary PPO cohort.
                let bytes = serde_json::to_vec(&row).unwrap();
                let decoded: CpuCollectedDecision = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(bytes, serde_json::to_vec(&decoded).unwrap());
                let value = CpuValueHead::from_checkpoint(crate::PpoValueHeadCheckpoint {
                    feature_count: trainer.phenotype().neuron_count(),
                    optimizer_step: 0,
                    parameters: vec![0.; (trainer.phenotype().neuron_count() as usize + 1) * 3],
                    last_updated_policy_version: None,
                })
                .unwrap();
                let phenotype = host
                    .world
                    .organism_registry()
                    .iter()
                    .next()
                    .unwrap()
                    .phenotype();
                assert!(cpu_training_windows(
                    &[decoded],
                    None,
                    &trainer,
                    &value,
                    phenotype,
                    2266,
                    crate::PpoConfig::default(),
                    2,
                    0
                )
                .is_err());
            }
        }
        let cp = host.checkpoint(&trainer).unwrap();
        let mut cold = CpuCanonicalHost::from_checkpoint(cp.clone(), &trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&cp).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        let a = host.step(&trainer, 1.0).unwrap();
        let b = cold.step(&trainer, 1.0).unwrap();
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        for mode in [
            None,
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            Some(8),
            Some(9),
            Some(10),
            Some(11),
            Some(12),
            Some(13),
            Some(14),
            Some(u16::MAX),
        ] {
            if mode == cp.joint_membership2177 {
                continue;
            }
            let mut bad = cp.clone();
            bad.joint_membership2177 = mode;
            bad.schema_version = match mode {
                Some(1) => 8,
                Some(2 | 3) => 7,
                Some(4 | 5) => 9,
                Some(6 | 7) => 10,
                Some(8 | 9) => 12,
                Some(10 | 11) => 13,
                Some(12 | 13) => 14,
                _ => 2,
            };
            assert!(
                CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err(),
                "accepted mode {mode:?}"
            );
        }
        for schema in [2, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15] {
            let mut bad = cp.clone();
            bad.schema_version = schema;
            assert!(CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err());
        }
        let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
        assert!(host
            .enable_joint_ingest_outcome_projection2286(&trainer, !apply)
            .is_err());
        assert_eq!(
            before,
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
        );
        assert_eq!(founder, serde_json::to_vec(&trainer.checkpoint()).unwrap());
    }
    let (mut ordinary, trainer) = food_fixture(false, true);
    ordinary.step(&trainer, 1.).unwrap();
    let ordinary_cp = ordinary.checkpoint(&trainer).unwrap();
    let before = serde_json::to_vec(&ordinary_cp).unwrap();
    for mode in [12, 13] {
        let mut edited = ordinary_cp.clone();
        edited.schema_version = 14;
        edited.joint_membership2177 = Some(mode);
        assert!(CpuCanonicalHost::from_checkpoint(edited, &trainer).is_err());
    }
    assert!(ordinary
        .enable_joint_ingest_outcome_projection2286(&trainer, true)
        .is_err());
    assert_eq!(
        before,
        serde_json::to_vec(&ordinary.checkpoint(&trainer).unwrap()).unwrap()
    );
}

#[test]
fn ingest2286_populated_actual_work_and_malformed_binding_rollback() {
    for apply in [false, true] {
        let (mut host, trainer) = food_fixture(false, false);
        host.enable_joint_ingest_outcome_projection2286(&trainer, apply)
            .unwrap();
        let saved = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
        let mut failed = host.snapshot();
        failed.fail_ingest_outcome_binding2286 = true;
        assert!(failed.step(&trainer, 1.).is_err());
        assert_eq!(
            saved,
            serde_json::to_vec(&failed.checkpoint(&trainer).unwrap()).unwrap()
        );
        let mut queried = 0;
        for _ in 0..16 {
            let pre = host.snapshot();
            for row in host.step(&trainer, 1.).unwrap().decisions {
                let frame = row.patch.pre_action().perception();
                let wire = serde_json::to_value(frame).unwrap();
                let draft: PerceptionFrameDraft =
                    serde_json::from_value(wire["base"].clone()).unwrap();
                assert_eq!(draft.base_digest(), frame.base_digest());
                let bank = &pre.residents[&row.organism_id.raw()].memory;
                let ordinary = bank.recall_frame(&draft).unwrap();
                let (global, gq) = bank.recall_frame_joint_comparison(&draft, true).unwrap();
                let live = row.joint_membership2177.as_ref().unwrap();
                let projection = live.ingest_outcome_projection2286.as_ref().unwrap();
                queried += gq.len();
                assert_eq!(live.sources, gq);
                assert_eq!(live.comparison.as_ref().unwrap().queried_sources, gq);
                assert_eq!(
                    live.query_work,
                    global.receipt().execution_evidence_work
                        + projection.ordinary.work.total().unwrap()
                );
                assert_eq!(live.query_work, row.executed_action_recall_work);
                assert_eq!(
                    projection.actual_cpu_ops,
                    projection.ordinary.work.cpu_transform
                );
                let r = ordinary.receipt();
                let base = u64::from(r.exact_bucket_reads)
                    + u64::from(r.neighbor_bucket_reads)
                    + u64::from(r.similarity_evaluations);
                let charged = row.patch.cognitive_work().unwrap();
                assert_eq!(
                    charged.memory_ops,
                    base + global.receipt().execution_evidence_work
                        + projection.ordinary.work.total().unwrap()
                        + live.admission_planning_work
                        + live.admission_reserved_work
                );
                let record = host.world.organism_registry().get(row.organism_id).unwrap();
                assert_eq!(record.cognitive_work(), charged);
                let (policy, debit, before, after) = row.cognitive_cost_capture.unwrap();
                let p = record.phenotype();
                let expected = policy.energy_debit(charged).unwrap()
                    * p.body.metabolic_turnover
                    * (ContinuousLocus::midpoint_value(0.52, 0.60)
                        - p.chemistry.brain_atp_efficiency)
                        .exp2();
                assert_eq!(debit, expected.min(before));
                assert_eq!(record.cognitive_energy_debit(), debit);
                assert!((before - after - debit).abs() < 2e-6);
                eprintln!(
                    "INGEST2286_ACCOUNTING {}",
                    serde_json::json!({
                        "apply":apply,"tick":frame.tick().raw(),"base":base,
                        "global_query":global.receipt().execution_evidence_work,
                        "basis_work":projection.ordinary.work,
                        "actual_cpu":projection.actual_cpu_ops,"query_work":live.query_work,
                        "planning":live.admission_planning_work,"reserved":live.admission_reserved_work,
                        "actual_memory_ops":charged.memory_ops,"debit":debit,"expected_debit":expected.min(before),
                        "ingest_rows":projection.ordinary.rows.len(),"eligible":projection.candidates.iter().filter(|c|c.support_eligible).count()
                    })
                );
            }
        }
        assert!(queried > 0);
    }
}

fn ingest2286_tick(
    frame: &PerceptionFrame,
    recall: &FinalizedMemoryRecall,
) -> crate::TrainingReplayTick {
    crate::TrainingReplayTick {
        encoded_inputs: vec![0.25],
        projection_gain: 1.,
        local_threshold_shift: 0.,
        microstep_count: 1,
        enabled_routes: vec![],
        effective_weight_offsets: vec![],
        structural_synapses: vec![],
        candidates: frame
            .candidates()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let mut inputs = std::array::from_fn(|n| n as f32 / 54.);
                let m = &recall.context().candidates[i];
                for k in 0..4 {
                    inputs[32 + k] = m.family_value[k] * m.family_confidence.raw();
                }
                crate::TrainingReplayCandidate {
                    family: c.family,
                    decoder_inputs: inputs,
                    innate_bias: 0.,
                    memory_readout: None,
                }
            })
            .collect(),
    }
}

#[test]
fn ingest2286_same_lookup_bounded_basis_and_only_supported_two_lanes() {
    let (mut host, trainer) = food_fixture(false, false);
    host.enable_joint_membership_comparison2182(&trainer, false)
        .unwrap();
    let mut joint_sources = 0;
    for _ in 0..24 {
        host.step(&trainer, 1.).unwrap();
        for (draft, _) in host.semantic_prior_inputs(&trainer).unwrap() {
            let bank = &host.residents[&draft.organism_id().raw()].memory;
            let before = serde_json::to_vec(&bank.export_active_bank().unwrap()).unwrap();
            let (global, gq) = bank.recall_frame_joint_comparison(&draft, true).unwrap();
            let (joint, q, basis) = bank
                .recall_frame_joint_ingest_outcome_basis2286(&draft)
                .unwrap();
            assert_eq!(global.context(), joint.context());
            assert_eq!(gq, q);
            joint_sources += q.len();
            assert_eq!(
                alife_runtime::canonical_preparation::cognitive_memory_context_for_recall(
                    draft.organism_id(),
                    ExperienceSequenceId(999),
                    &global
                )
                .unwrap(),
                alife_runtime::canonical_preparation::cognitive_memory_context_for_recall(
                    draft.organism_id(),
                    ExperienceSequenceId(999),
                    &joint
                )
                .unwrap()
            );
            let (frame, joint) = joint.finalize(draft.clone()).unwrap();
            let tick = ingest2286_tick(&frame, &joint);
            let mut applied = tick.clone();
            let a = project_joint_ingest_outcome2286(&frame, &joint, basis, &mut applied, true)
                .unwrap();
            let (_, _, basis2) = bank
                .recall_frame_joint_ingest_outcome_basis2286(&draft)
                .unwrap();
            let mut withheld = tick.clone();
            let b = project_joint_ingest_outcome2286(&frame, &joint, basis2, &mut withheld, false)
                .unwrap();
            assert_eq!(withheld, tick);
            assert_eq!(a.ordinary.work, b.ordinary.work);
            for c in &a.candidates {
                let i = usize::from(c.candidate_index);
                assert_eq!(c.applied, c.support_eligible);
                for (k, lane) in [32, 35].into_iter().enumerate() {
                    assert_eq!(
                        applied.candidates[i].decoder_inputs[lane],
                        if c.support_eligible {
                            c.ordinary_weighted[k]
                        } else {
                            c.joint_weighted[k]
                        }
                    );
                    applied.candidates[i].decoder_inputs[lane] =
                        tick.candidates[i].decoder_inputs[lane];
                }
            }
            assert_eq!(applied, tick);
            assert_eq!(
                before,
                serde_json::to_vec(&bank.export_active_bank().unwrap()).unwrap()
            );
            let (_, _, wrong) = bank
                .recall_frame_joint_ingest_outcome_basis2286(&draft)
                .unwrap();
            let mut malformed = tick.clone();
            malformed.candidates.pop();
            assert!(
                project_joint_ingest_outcome2286(&frame, &joint, wrong, &mut malformed, true)
                    .is_err()
            );
        }
    }
    assert!(joint_sources > 0);
}

#[test]
fn ingest2286_zero_one_mixed_cache_positive_absent_and_weighted_underflow() {
    let (mut host, trainer) = food_fixture(false, false);
    let (draft, _) = host.semantic_prior_inputs(&trainer).unwrap().remove(0);
    let mut ingest = draft
        .candidates()
        .iter()
        .find(|c| c.family == CandidateActionFamily::Ingest)
        .unwrap()
        .clone();
    ingest.candidate_index = 0;
    let mut duplicate = ingest.clone();
    duplicate.candidate_index = 1;
    duplicate.action_id = ActionId(990);
    let mut other = draft
        .candidates()
        .iter()
        .find(|c| c.family == CandidateActionFamily::Contact)
        .unwrap()
        .clone();
    other.candidate_index = 2;
    let one = draft.clone().with_candidates(vec![ingest.clone()]).unwrap();
    let mut bank = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    )
    .unwrap();
    let patch = success2275_checked_fixture_patch(&bank, 1, one.clone(), 0.8, 0., |_| {});
    patch.validate_contract().unwrap();
    bank.observe_sealed_patch(&patch).unwrap();
    let mut wire = serde_json::to_value(&bank).unwrap();
    wire["candidate_store"]["records"]["1"]["confidence"] = serde_json::json!(f32::from_bits(1));
    let tiny: MemoryBank = serde_json::from_value(wire).unwrap();
    let empty = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    )
    .unwrap();
    for (bank, supported, underflow) in [
        (&empty, false, false),
        (&bank, true, false),
        (&tiny, true, true),
    ] {
        for mode in 0..3 {
            let candidates = match mode {
                0 => {
                    let mut c = other.clone();
                    c.candidate_index = 0;
                    vec![c]
                }
                1 => vec![ingest.clone()],
                _ => vec![ingest.clone(), duplicate.clone(), other.clone()],
            };
            let d = draft.clone().with_candidates(candidates).unwrap();
            let n = d.candidates().len() as u64;
            let (global, _) = bank.recall_frame_joint_comparison(&d, true).unwrap();
            let (joint, _, basis) = bank
                .recall_frame_joint_ingest_outcome_basis2286(&d)
                .unwrap();
            assert_eq!(global.context(), joint.context());
            let e = basis.evidence();
            let m = e.rows.len() as u64;
            assert_eq!(
                m,
                match mode {
                    0 => 0,
                    1 => 1,
                    _ => 2,
                }
            );
            assert_eq!(e.work.selection, n);
            assert_eq!(e.work.cpu_transform, 16 + n + 17 * m);
            let provenance: u64 = e
                .rows
                .iter()
                .map(|r| 4 + 16 * r.sources.sources.iter().flatten().count() as u64)
                .sum();
            assert_eq!(e.work.basis_construction, 16 + 12 * m + provenance);
            assert_eq!(e.work.support_validation, 4 * m + provenance);
            if m == 0 {
                assert_eq!(e.work.total().unwrap(), 32 + 2 * n);
                assert_eq!(e.work.provenance_capture, 0);
                assert_eq!(e.work.provenance_cache_copy, 0);
            } else {
                let per = 4 + 16 * u64::from(supported);
                assert_eq!(e.work.provenance_capture, per);
                assert_eq!(
                    e.work.provenance_cache_copy,
                    if m == 2 { 2 * per } else { 0 }
                );
            }
            assert_eq!(
                joint.receipt().execution_evidence_work,
                global.receipt().execution_evidence_work + e.work.total().unwrap()
            );
            let (frame, joint) = joint.finalize(d.clone()).unwrap();
            let tick = ingest2286_tick(&frame, &joint);
            let mut projected = tick.clone();
            let output =
                project_joint_ingest_outcome2286(&frame, &joint, basis, &mut projected, true)
                    .unwrap();
            for (row, c) in output.ordinary.rows.iter().zip(&output.candidates) {
                assert_eq!(row.support_eligible, supported);
                assert_eq!(c.applied, supported);
                if supported {
                    assert!(row.source_count > 0 && row.confidence > 0.);
                    assert!(row.best_source.is_some());
                    if underflow {
                        assert_eq!(c.ordinary_weighted[1], 0.);
                        assert!(row.values[1] != 0.);
                    }
                } else {
                    assert_eq!(row.source_count, 0);
                    assert!(row.best_source.is_none());
                    assert!(row.sources.sources.iter().all(Option::is_none));
                    assert_eq!(c.output_weighted, c.joint_weighted);
                }
                for (k, lane) in [32, 35].into_iter().enumerate() {
                    assert_eq!(
                        c.ordinary_weighted[k].to_bits(),
                        (row.values[k] * row.confidence).to_bits()
                    );
                    projected.candidates[usize::from(row.candidate_index)].decoder_inputs[lane] =
                        tick.candidates[usize::from(row.candidate_index)].decoder_inputs[lane];
                }
            }
            assert_eq!(projected, tick);
            let (_, _, basis) = bank
                .recall_frame_joint_ingest_outcome_basis2286(&d)
                .unwrap();
            let (foreign_frame, foreign) =
                empty.recall_frame(&d).unwrap().finalize(d.clone()).unwrap();
            if supported {
                assert!(basis.validate_for_recall(&foreign_frame, &foreign).is_err());
            }
        }
    }
}

#[test]
fn ingest2286_positive_ordinary_and_different_joint_change_exactly_two_components() {
    // Explicit checked storage fixture, not a measured World outcome or an edit
    // to an acquired life. Ordinary and joint estimates intentionally disagree.
    let (mut host, trainer) = food_fixture(false, false);
    let (draft, _) = host.semantic_prior_inputs(&trainer).unwrap().remove(0);
    let mut c = draft
        .candidates()
        .iter()
        .find(|c| c.family == CandidateActionFamily::Ingest)
        .unwrap()
        .clone();
    c.candidate_index = 0;
    let draft = draft.with_candidates(vec![c]).unwrap();
    let mut bank = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    )
    .unwrap();
    let patch = success2275_checked_fixture_patch(&bank, 1, draft.clone(), 0.8, 0., |_| {});
    bank.observe_sealed_patch(&patch).unwrap();
    let (_, ordinary) = bank
        .recall_frame(&draft)
        .unwrap()
        .finalize(draft.clone())
        .unwrap();
    let query = ordinary.candidate_keys()[0].query().clone();
    let target = draft.candidates()[0].target.entity;
    let fact = ExecutedActionMemoryV1 {
        query: query.clone(),
        source_sequence_id: ExperienceSequenceId(1),
        event_tick: Tick::new(draft.tick().raw() + 1),
        bundle_digest: [1, 2, 3, 4],
        receipt: alife_core::experience::ChannelPhysicalOutcome::new(
            MotorChannel::Manipulation,
            PhysicalActionOutcome {
                contact: PhysicalContactKind::Touch,
                target_entity: target,
                displacement: Vec3f::ZERO,
                collision_normal: None,
                energy_cost: NormalizedScalar::new(0.1).unwrap(),
            },
        )
        .unwrap()
        .with_execution_evidence(
            ExecutedChannelEvidenceV1::new(query.action_id(), target, true, BodyEventDelta::zero())
                .unwrap(),
        )
        .unwrap(),
    };
    let facts = serde_json::to_vec(&vec![fact]).unwrap();
    let event = serde_json::json!({"schema_version":1,"sealed_causal_digest":[4,3,2,1],"members":facts,"shared_family_value":[-0.6,1.0,0.7,-0.8],"event_confidence":0.3});
    let mut wire = serde_json::to_value(&bank).unwrap();
    let record = &mut wire["candidate_store"]["records"]["1"];
    record["schema_version"] = serde_json::json!(3);
    record["executed_actions_json"] = event["members"].clone();
    record["joint_event"] = serde_json::json!(serde_json::to_vec(&event).unwrap());
    let bank: MemoryBank = serde_json::from_value(wire).unwrap();
    // Probe after the recorded event, never bypass temporal eligibility.
    let now = Tick::new(draft.tick().raw() + 2);
    let draft = PerceptionFrameDraft::new(
        draft.organism_id(),
        now,
        draft.sensor_profile(),
        SensorySnapshot::new(
            draft.organism_id(),
            now,
            draft.sensory().observer_position,
            draft.sensory().channels,
            Default::default(),
        )
        .unwrap(),
        draft.body(),
        HomeostaticSnapshot::new(
            now,
            draft.homeostasis().drives,
            draft.homeostasis().hormones,
        )
        .unwrap(),
        draft.candidates().to_vec(),
        SensorProfileProvenance::new(draft.sensor_profile(), SensoryAbiVersion::CURRENT, now)
            .unwrap(),
        draft.grounded_object_slots().to_vec(),
    )
    .unwrap();
    let (joint, sources, basis) = bank
        .recall_frame_joint_ingest_outcome_basis2286(&draft)
        .unwrap();
    assert!(!sources.is_empty());
    assert!(basis.evidence().rows[0].support_eligible);
    assert!(basis.evidence().rows[0].source_count > 0);
    let (frame, joint) = joint.finalize(draft).unwrap();
    let original = ingest2286_tick(&frame, &joint);
    let mut transformed = original.clone();
    let receipt =
        project_joint_ingest_outcome2286(&frame, &joint, basis, &mut transformed, true).unwrap();
    assert!(receipt.candidates[0].applied);
    for lane in [32, 35] {
        assert_ne!(
            transformed.candidates[0].decoder_inputs[lane].to_bits(),
            original.candidates[0].decoder_inputs[lane].to_bits(),
            "supported component must actually change"
        );
    }
    for lane in [33, 34] {
        assert_eq!(
            transformed.candidates[0].decoder_inputs[lane].to_bits(),
            original.candidates[0].decoder_inputs[lane].to_bits()
        );
    }
    for (k, lane) in [32, 35].into_iter().enumerate() {
        let row = &receipt.ordinary.rows[0];
        assert_eq!(
            transformed.candidates[0].decoder_inputs[lane].to_bits(),
            (row.values[k] * row.confidence).to_bits()
        );
        transformed.candidates[0].decoder_inputs[lane] =
            original.candidates[0].decoder_inputs[lane];
    }
    assert_eq!(transformed, original);
}

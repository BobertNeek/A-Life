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

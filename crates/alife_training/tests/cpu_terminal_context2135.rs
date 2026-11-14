#![cfg(feature = "cpu-backend")]
use alife_core::*;
use alife_training::{cpu::*, PpoConfig, PpoValueHeadCheckpoint};
use alife_world::PlayerInteractionV1;
#[path = "support/terminal_parent2135.rs"]
mod parent;

fn enabled(atp: Option<f32>, gain: f32, mask: bool) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    let (mut host, trainer) = parent::fixture(atp);
    let mut config = TerminalContextConfig2135::new(gain).unwrap();
    config.mask_new_context = mask;
    host.enable_terminal_context2135(config, &trainer).unwrap();
    (host, trainer)
}

#[test]
fn enabled_full_parent_cold_roundtrip_preserves_residents_optimizer_and_next_step() {
    let (mut host, trainer) = enabled(None, 1.0, false);
    let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    let output = parent::local_output();
    let mut last_row = None;
    for _ in 0..3 {
        let step = host.step(&trainer, 1.0).unwrap();
        assert_eq!(step.decisions.len(), 1);
        step.decisions[0].patch.validate_contract().unwrap();
        last_row = step.decisions.into_iter().next();
    }
    let cp = host.checkpoint(&trainer).unwrap();
    assert_eq!(cp.schema_version, 3);
    assert!(cp.terminal_context2135.is_some());
    let resident = &cp.residents[0];
    assert!(resident.neural.last_outcome_tick.is_some());
    assert!(resident.neural.eligibility.iter().any(|v| *v != 0.0));
    assert!(resident.neural.fast.iter().any(|v| *v != 0.0));
    assert!(!resident.memory.records.is_empty());
    let bytes = serde_json::to_vec(&cp).unwrap();
    // Founder optimizer is separate from the private host CP. Restore both.
    // The exported parent supplies no historical Adam file: these are its real
    // native birth moments/ages, never invented historical optimizer markers.
    let (_, mut cold_trainer) = parent::fixture(None);
    cold_trainer
        .restore_checkpoint(serde_json::from_slice(&founder).unwrap())
        .unwrap();
    let mut cold =
        CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes).unwrap(), &cold_trainer)
            .unwrap();
    assert!(serde_json::to_vec(&cold.checkpoint(&cold_trainer).unwrap()).unwrap() == bytes);
    assert!(serde_json::to_vec(&cold_trainer.checkpoint()).unwrap() == founder);
    let hot_step = host.step(&trainer, 1.0).unwrap();
    let cold_step = cold.step(&cold_trainer, 1.0).unwrap();
    assert!(
        serde_json::to_vec(&hot_step.decisions).unwrap()
            == serde_json::to_vec(&cold_step.decisions).unwrap()
    );
    assert!(
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap()
            == serde_json::to_vec(&cold.checkpoint(&cold_trainer).unwrap()).unwrap()
    );
    assert!(serde_json::to_vec(&trainer.checkpoint()).unwrap() == founder);
    if let Some(root) = output {
        std::fs::write(root.join("enabled.cp.json"), bytes).unwrap();
        std::fs::write(
            root.join("enabled.row.json"),
            serde_json::to_vec(&last_row.unwrap()).unwrap(),
        )
        .unwrap();
        std::fs::write(root.join("founder.json"), founder).unwrap();
    }
}

#[test]
fn real_host_rejects_inconsistent_schema_and_recipe() {
    let (host, trainer) = enabled(None, 1.0, false);
    let good = host.checkpoint(&trainer).unwrap();
    let mut invalid = Vec::new();
    let mut prototype = good.clone();
    prototype.schema_version = 2;
    invalid.push(prototype);
    let mut missing = good.clone();
    missing.terminal_context2135 = None;
    invalid.push(missing);
    let mut future = good.clone();
    future.schema_version = 4;
    invalid.push(future);
    let mut wrong = good;
    wrong
        .terminal_context2135
        .as_mut()
        .unwrap()
        .construction_genome_sha256 = "wrong".into();
    invalid.push(wrong);
    for cp in invalid {
        assert!(CpuCanonicalHost::from_checkpoint(cp, &trainer).is_err());
    }
}

#[test]
fn full_native_rows_reject_ordinary_readers_and_training_even_when_head_abstains() {
    let (host, trainer) = parent::fixture(None);
    let capacity = BrainCapacityClass::n2048();
    let budget = cpu_activity_budget(
        trainer.phenotype(),
        CpuActivityAnchor {
            organism_id: host.world().organism_entity_ids()[0].0,
            tick: Tick::ZERO,
            sequence_cursor: 1,
            previous_dispatch_generation: 0,
            source_frame_digest: [0; 4],
            frame_digest: trainer.phenotype().phenotype_hash().0,
        },
        0.1,
        u32::from(capacity.execution().max_candidates()),
        u32::from(capacity.execution().max_memory_context_records()),
    )
    .unwrap();
    // Admit ordinary minimum dispatch while leaving too little for the extra
    // head. Derive this from the actual parent tariff, not a guessed ATP level.
    let low_atp = ((budget.debit_atp_q16 + 2) as f32 / BRAIN_ATP_Q16_MAX as f32)
        .max(HomeostaticParameters::reference().catatonia_brain_atp_threshold + 0.001);
    let mut admissions = 0;
    let mut refusals = 0;
    for (atp, gain, mask) in [
        (None, 0.0, false),
        (None, 1e-20, false),
        (None, 1.0, true),
        (Some(low_atp), 1.0, false),
    ] {
        let (mut host, trainer) = enabled(None, gain, mask);
        if let Some(atp) = atp {
            // First acquire a real native bank through ordinary player cues and
            // unforced decisions. Then construct one low-ATP budget branch;
            // this is not a claim of biologically arisen deprivation.
            for cue in [
                PlayerInteractionV1::Praise,
                PlayerInteractionV1::Scold,
                PlayerInteractionV1::Pet,
            ] {
                let (id, entity) = host.world().organism_entity_ids()[0];
                let position = host.world().entity(entity).unwrap().position;
                host.world_mut()
                    .queue_player_interaction_v1(id, position, cue)
                    .unwrap();
                host.step(&trainer, 1.0).unwrap();
            }
            let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap().residents).unwrap();
            let id = host.world().organism_entity_ids()[0].0;
            let mut registry = host.world().organism_registry().clone();
            registry
                .with_biology_mut(id, |biology| {
                    biology.homeostasis.drives.brain_atp = atp;
                    Ok(())
                })
                .unwrap();
            host.world_mut()
                .replace_organism_registry_exact(registry.iter().cloned())
                .unwrap();
            assert!(
                serde_json::to_vec(&host.checkpoint(&trainer).unwrap().residents).unwrap()
                    == before
            );
        }
        let mut rows = Vec::new();
        for _ in 0..2 {
            let step = host.step(&trainer, 1.0).unwrap();
            assert_eq!(step.tick_after.raw(), step.tick_before.raw() + 1);
            rows.extend(step.decisions);
        }
        assert!(
            !rows.is_empty(),
            "low ATP must permit the funded ordinary decision"
        );
        for row in &rows {
            let receipt = row.terminal_context2135.as_ref().unwrap();
            if atp.is_some() {
                eprintln!(
                    "native low-ATP admission: {:?}; requested_work={}",
                    receipt.admission, receipt.requested_work_units
                );
            }
            match receipt.admission {
                TerminalAdmission2135::Admitted => admissions += 1,
                TerminalAdmission2135::InsufficientAtp {
                    required_atp_q16,
                    available_atp_q16,
                } => {
                    assert!(required_atp_q16 > available_atp_q16);
                    let observation_debit = receipt
                        .observation
                        .as_ref()
                        .map_or(0, |r| r.reserved_atp_q16);
                    assert!(receipt.reserved_atp_q16 - observation_debit <= available_atp_q16);
                    assert!(receipt.candidates.is_empty());
                    refusals += 1;
                }
                _ => panic!("fresh fixture unexpectedly exceeds bank capacity"),
            }
            if gain == 0.0 || mask || receipt.admission != TerminalAdmission2135::Admitted {
                assert!(receipt.candidates.iter().all(|c| c.delta == 0.0));
            }
            let bytes = serde_json::to_vec(row).unwrap();
            assert!(serde_json::from_slice::<CpuCollectedDecision>(&bytes).is_err());
            let wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(wire.as_object().unwrap().len(), 1);
            assert!(wire
                .get("terminal_context2135_research_record_v1")
                .is_some());
        }
        let n = trainer.phenotype().neuron_count();
        let value = CpuValueHead::from_checkpoint(PpoValueHeadCheckpoint {
            feature_count: n,
            optimizer_step: 0,
            parameters: vec![0.0; (n as usize + 1) * 3],
            last_updated_policy_version: None,
        })
        .unwrap();
        let phenotype = host
            .world()
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .phenotype();
        assert!(cpu_training_windows(
            &rows,
            None,
            &trainer,
            &value,
            phenotype,
            1,
            PpoConfig::default(),
            2,
            0
        )
        .is_err());
    }
    assert!(admissions > 0);
    assert!(
        refusals > 0,
        "exercise an actual native extra-head ATP abstention"
    );
    eprintln!("full native rows: admitted={admissions}, insufficient_atp={refusals}, constructed_atp={low_atp}");
    let (mut off, trainer) = parent::fixture(None);
    let mut off_rows = off.step(&trainer, 1.0).unwrap().decisions;
    off_rows.extend(off.step(&trainer, 1.0).unwrap().decisions);
    let row = &off_rows[0];
    assert!(row.terminal_context2135.is_none());
    let bytes = serde_json::to_vec(&row).unwrap();
    let decoded: CpuCollectedDecision = serde_json::from_slice(&bytes).unwrap();
    assert!(serde_json::to_vec(&decoded).unwrap() == bytes);
    assert!(!std::str::from_utf8(&bytes)
        .unwrap()
        .contains("terminal_context2135"));
    let n = trainer.phenotype().neuron_count();
    let value = CpuValueHead::from_checkpoint(PpoValueHeadCheckpoint {
        feature_count: n,
        optimizer_step: 0,
        parameters: vec![0.0; (n as usize + 1) * 3],
        last_updated_policy_version: None,
    })
    .unwrap();
    let phenotype = off
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .phenotype();
    assert!(cpu_training_windows(
        &off_rows,
        None,
        &trainer,
        &value,
        phenotype,
        1,
        PpoConfig::default(),
        2,
        0
    )
    .is_ok());
}

#[test]
fn active_zero_and_mask_read_identical_native_state_and_charge_identical_work() {
    let (mut host, trainer) = enabled(None, 1.0, false);
    for _ in 0..3 {
        host.step(&trainer, 1.0).unwrap();
    }
    let cp = host.checkpoint(&trainer).unwrap();
    assert!(!cp.residents[0].memory.records.is_empty());
    let mut receipts = Vec::new();
    for (gain, mask) in [(1.0, false), (0.0, false), (1.0, true)] {
        // Explicit counterfactual cold branches of one completed native boundary.
        let mut branch = cp.clone();
        let mut config = TerminalContextConfig2135::new(gain).unwrap();
        config.mask_new_context = mask;
        branch.terminal_context2135 = Some(config);
        let mut branch = CpuCanonicalHost::from_checkpoint(branch, &trainer).unwrap();
        let row = branch.step(&trainer, 1.0).unwrap().decisions.remove(0);
        let receipt = row.terminal_context2135.unwrap();
        assert_eq!(
            receipt.records,
            cp.residents[0].memory.records.len()
                + cp.residents[0]
                    .terminal_channel2135
                    .as_ref()
                    .map_or(0, |s| s.observations.len())
        );
        let mut value = serde_json::to_value(receipt).unwrap();
        value.as_object_mut().unwrap().remove("config");
        if let Some(observation) = value
            .get_mut("observation")
            .and_then(serde_json::Value::as_object_mut)
        {
            observation.remove("recorded");
        }
        for c in value["candidates"].as_array_mut().unwrap() {
            c.as_object_mut().unwrap().remove("delta");
        }
        receipts.push(value);
    }
    assert_eq!(receipts[0], receipts[1]);
    assert_eq!(receipts[0], receipts[2]);
}

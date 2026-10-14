//! CPU-only authoritative-age and restore regressions; no adapter or neural execution.
use super::*;
use alife_core::{
    BiochemistryState, CreatureGenome, FoundationGeneticIdentity, FoundationWeightAsset,
};
use alife_gpu_backend::{GpuDevelopmentalPlasticity, GpuPhenotypeUpload};

fn record_at(age: Tick, birth: Tick) -> WorldOrganismRecord {
    let sensor = SensorProfile::PrivilegedAffordanceV1;
    let asset = FoundationWeightAsset::builtin_nano512_v1(sensor).unwrap();
    let manifest = asset.manifest();
    let foundation = FoundationGeneticIdentity::new(
        manifest.foundation_id().raw(),
        manifest.foundation_version().raw() as u16,
        manifest.compatibility_family_id().raw(),
        BrainCapacityClass::N512_ID,
    )
    .unwrap();
    let genome = CreatureGenome::early_mammal_founder(77, foundation).unwrap();
    let phenotype = genome.express().unwrap();
    let now = Tick(birth.raw() + age.raw());
    let biochemistry = BiochemistryState::new_with_age(&phenotype, now, age).unwrap();
    WorldOrganismRecord::new(
        OrganismId(77),
        WorldEntityId(77),
        genome,
        phenotype,
        biochemistry,
        birth,
    )
    .unwrap()
}

fn plan_at(age: Tick, birth: Tick) -> ResidentAuthorityPlan {
    let record = record_at(age, birth);
    let now = Tick(birth.raw() + age.raw());
    let sensor = SensorProfile::PrivilegedAffordanceV1;
    // Exercise the actual persisted biological record and production restore/admission planner.
    let restored: WorldOrganismRecord =
        serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();
    let make_plan = |record: &WorldOrganismRecord| {
        resident_authority_plan_from_record(
            record,
            OrganismId(77),
            WorldEntityId(77),
            now,
            BrainScaleTier::Nano512,
            sensor,
        )
        .unwrap()
    };
    let original = make_plan(&record);
    let restored = make_plan(&restored);
    assert_eq!(original.development, restored.development);
    assert_eq!(original.phenotype, restored.phenotype);
    assert_eq!(original.compiler_inputs, restored.compiler_inputs);
    restored
}

fn boundaries() -> (Tick, Tick) {
    // The immutable genome retains period timing even though stable compilation clears open periods.
    let plan = plan_at(Tick::ZERO, Tick::ZERO);
    let period = plan.genome.developmental_schedule.critical_periods[0];
    (period.opens_at, period.closes_at)
}

#[test]
fn audit_sensitive_period_age_restore_and_stable_upload_identity() {
    let (opens, closes) = boundaries();
    assert!(opens.raw() > 0);
    let before = plan_at(Tick(opens.raw() - 1), Tick(10_000));
    let baseline = GpuPhenotypeUpload::try_from(&before.phenotype).unwrap();
    for (age, active) in [
        (Tick(opens.raw() - 1), false),
        (opens, true),
        (closes, true),
        (Tick(closes.raw() + 1), false),
    ] {
        let plan = plan_at(age, Tick(10_000));
        assert_eq!(plan.development.age_ticks, age);
        assert_eq!(!plan.development.open_critical_periods.is_empty(), active);
        assert_eq!(
            plan.phenotype.phenotype_hash(),
            before.phenotype.phenotype_hash()
        );
        assert_eq!(plan.compiler_inputs, before.compiler_inputs);
        let upload = GpuPhenotypeUpload::try_from(&plan.phenotype).unwrap();
        assert_eq!(upload.genetic_weights, baseline.genetic_weights);
        assert_eq!(upload.alpha, baseline.alpha);
        assert_eq!(
            upload.synapse_learning_metadata,
            baseline.synapse_learning_metadata
        );
    }
}

#[test]
fn learning_age_uses_outcome_and_current_retry_or_sleep_tick_after_restore() {
    let (opens, closes) = boundaries();
    let birth = Tick(10_000);
    // Keep the persisted body's snapshot before opening: the mutation tick,
    // rather than its previous homeostatic snapshot or source experience,
    // selects the inherited schedule.
    let record = record_at(Tick(opens.raw() - 1), birth);
    let restored: WorldOrganismRecord =
        serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();
    for record in [&record, &restored] {
        let source_tick = Tick(birth.raw() + opens.raw() - 1);
        let outcome_tick = Tick(source_tick.raw() + 1);
        assert!(learning_development_at(record, source_tick)
            .unwrap()
            .open_critical_periods
            .is_empty());
        let outcome = learning_development_at(record, outcome_tick).unwrap();
        assert_eq!(outcome.age_ticks, opens);
        assert!(!outcome.open_critical_periods.is_empty());
        let closing_tick = Tick(birth.raw() + closes.raw());
        assert!(!learning_development_at(record, closing_tick)
            .unwrap()
            .open_critical_periods
            .is_empty());
        // A retry or sleep replay now applies after closure, even when its
        // experience was originally acquired inside the sensitive period.
        let now = Tick(closing_tick.raw() + 1);
        let current = learning_development_at(record, now).unwrap();
        assert_eq!(current.age_ticks, Tick(closes.raw() + 1));
        assert!(current.open_critical_periods.is_empty());
    }
}

#[test]
fn authoritative_transition_updates_mutable_transport_without_recompilation() {
    let (opens, closes) = boundaries();
    let birth = Tick(10_000);
    let before = plan_at(Tick(opens.raw() - 1), birth);
    let mut transport = GpuDevelopmentalPlasticity::default();
    let parameters = before.compiler_inputs.genome().plasticity_parameters();
    assert!(transport
        .update(&before.phenotype, &before.development, parameters)
        .unwrap());
    let closed_words = transport.words().to_vec();
    let during = plan_at(opens, birth);
    assert!(transport
        .update(&before.phenotype, &during.development, parameters)
        .unwrap());
    let open_words = transport.words().to_vec();
    assert_ne!(open_words, closed_words);
    assert!(open_words[4..]
        .iter()
        .any(|word| f32::from_bits(*word) > 1.0));
    // Biological age advances while the same period remains open: no dispatch
    // metadata rebuild is needed, including when sleep applies old memories.
    let closing = plan_at(closes, birth);
    assert!(!transport
        .update(&before.phenotype, &closing.development, parameters)
        .unwrap());
    assert_eq!(transport.words(), open_words);
    let adult = plan_at(Tick(closes.raw() + 1), birth);
    assert!(transport
        .update(&before.phenotype, &adult.development, parameters)
        .unwrap());
    assert_eq!(transport.words(), closed_words);
    // A fresh cache reconstructed from the restored biological record produces
    // the same payload; no age-specific phenotype or learned bank is restored.
    let mut restored_transport = GpuDevelopmentalPlasticity::default();
    restored_transport
        .update(&adult.phenotype, &adult.development, parameters)
        .unwrap();
    assert_eq!(restored_transport.words(), transport.words());
    assert_eq!(adult.compiler_inputs, before.compiler_inputs);
    assert_eq!(adult.phenotype, before.phenotype);
}

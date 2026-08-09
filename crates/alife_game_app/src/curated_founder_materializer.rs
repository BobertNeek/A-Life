use crate::{
    CuratedFounderPlan, CuratedFounderPlanEntry, CuratedFounderResetError,
    CuratedFounderResetReceipt,
};
use alife_core::{
    BiochemistryState, BrainCapacityClass, CreatureGenome, CreaturePhenotype,
    FoundationWeightAsset, N512FounderFoundationProjection, ScaffoldContractError, Validate,
};
use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratedFounderBundleIdentity {
    pub(crate) plan_receipt: CuratedFounderResetReceipt,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CuratedFounderBundleEntry {
    pub(crate) plan_entry: CuratedFounderPlanEntry,
    pub(crate) genome: CreatureGenome,
    pub(crate) phenotype: CreaturePhenotype,
    pub(crate) biochemistry: BiochemistryState,
    pub(crate) projection: N512FounderFoundationProjection,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CuratedFounderBundle {
    pub(crate) identity: CuratedFounderBundleIdentity,
    pub(crate) entries: Vec<CuratedFounderBundleEntry>,
}

#[allow(dead_code)]
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum CuratedFounderMaterializationError {
    #[error("curated founder plan validation failed: {0}")]
    Plan(#[from] CuratedFounderResetError),
    #[error("curated founder core validation failed: {0}")]
    Core(#[from] ScaffoldContractError),
    #[error("curated founder materialization mismatch at {slot:?}: {field}")]
    Mismatch {
        slot: Option<u32>,
        field: &'static str,
    },
}

#[allow(dead_code)]
pub(crate) fn materialize_curated_founder_bundle(
    plan: &CuratedFounderPlan,
) -> Result<CuratedFounderBundle, CuratedFounderMaterializationError> {
    plan.validate()?;

    let foundation_asset = FoundationWeightAsset::builtin_nano512_v1(plan.sensor_profile)?;
    let foundation_manifest = foundation_asset.manifest();
    if plan.foundation.brain_class_id != BrainCapacityClass::N512_ID
        || plan.foundation.foundation_id != foundation_manifest.foundation_id().raw()
        || u32::from(plan.foundation.version) != foundation_manifest.foundation_version().raw()
        || plan.foundation.compatibility_family_id
            != foundation_manifest.compatibility_family_id().raw()
        || plan.foundation_content_digest != foundation_asset.digest()
    {
        return Err(CuratedFounderMaterializationError::Mismatch {
            slot: None,
            field: "checked Nano512 foundation asset",
        });
    }

    let mut entries = Vec::with_capacity(plan.entries.len());
    for plan_entry in &plan.entries {
        let genome =
            CreatureGenome::early_mammal_founder(plan_entry.conception_seed, plan.foundation)?;
        if genome.id != plan_entry.genome_id
            || genome.lineage_id != plan_entry.lineage_id
            || genome.conception_seed != plan_entry.conception_seed
            || genome.foundation != plan.foundation
            || !genome.parent_genome_ids.is_empty()
            || genome.provenance.ordinary_birth
            || !genome.provenance.recombination.is_empty()
            || !genome.provenance.mutations.is_empty()
        {
            return Err(materialization_mismatch(
                plan_entry.final_population_slot,
                "re-derived founder genome",
            ));
        }
        genome.validate_contract()?;

        let phenotype = genome.express()?;
        if phenotype.source_genome_id != genome.id
            || phenotype.lineage_id != genome.lineage_id
            || phenotype.foundation != genome.foundation
            || phenotype.genetic_provenance != genome.provenance
            || phenotype.brain_genome.id != genome.id
            || phenotype.brain_genome.lineage_id != Some(genome.lineage_id)
            || phenotype.brain_genome.parent_genome_ids != genome.parent_genome_ids
        {
            return Err(materialization_mismatch(
                plan_entry.final_population_slot,
                "expressed founder phenotype",
            ));
        }

        let projection = N512FounderFoundationProjection::compile(
            &phenotype,
            plan.sensor_profile,
            &foundation_asset,
        )?;
        if projection.source_genome_id() != plan_entry.genome_id
            || projection.lineage_id() != plan_entry.lineage_id
            || projection.foundation() != &plan.foundation
            || projection.sensor_profile() != plan.sensor_profile
            || projection.foundation_asset_digest() != plan.foundation_content_digest
            || projection.source_brain_genome() != &phenotype.brain_genome
            || projection.genetic_provenance() != &phenotype.genetic_provenance
            || projection.receipt().source_genome_id() != phenotype.source_genome_id
            || projection.receipt().lineage_id() != phenotype.lineage_id
        {
            return Err(materialization_mismatch(
                plan_entry.final_population_slot,
                "Nano512 founder projection",
            ));
        }
        projection
            .receipt()
            .validate_against_projection(&projection)?;

        let biochemistry = BiochemistryState::new(&phenotype, plan.restored_tick)?;
        if biochemistry.source_genome_id != plan_entry.genome_id
            || biochemistry.tick != plan.restored_tick
            || biochemistry.homeostasis.tick != plan.restored_tick
        {
            return Err(materialization_mismatch(
                plan_entry.final_population_slot,
                "fresh founder biochemistry",
            ));
        }
        biochemistry.validate_contract()?;

        entries.push(CuratedFounderBundleEntry {
            plan_entry: *plan_entry,
            genome,
            phenotype,
            biochemistry,
            projection,
        });
    }

    for entry in &entries {
        entry.genome.validate_contract()?;
        entry.biochemistry.validate_contract()?;
        entry.projection.validate()?;
    }

    Ok(CuratedFounderBundle {
        identity: CuratedFounderBundleIdentity {
            plan_receipt: plan.receipt.clone(),
        },
        entries,
    })
}

fn materialization_mismatch(slot: u32, field: &'static str) -> CuratedFounderMaterializationError {
    CuratedFounderMaterializationError::Mismatch {
        slot: Some(slot),
        field,
    }
}

#[cfg(test)]
mod tests {
    use super::{materialize_curated_founder_bundle, CuratedFounderMaterializationError};
    use crate::{
        plan_curated_founder_reset, CuratedFounderAgentInput, CuratedFounderPlan,
        CuratedFounderResetRequest, CURATED_FOUNDER_RESET_POLICY,
    };
    use alife_core::{
        BiochemistryState, Blake3Digest, BrainCapacityClass, CreatureGenome,
        FoundationGeneticIdentity, FoundationWeightAsset, GenomeId, LineageId, OrganismId,
        SensorProfile, Tick, WorldEntityId,
    };

    const WORLD_ENTITY_IDS_BY_SLOT: [u64; 3] = [10_002, 10_000, 10_001];
    const ORGANISM_IDS_BY_SLOT: [u64; 3] = [20_001, 20_002, 20_000];

    fn checked_foundation(
        sensor_profile: SensorProfile,
    ) -> (FoundationGeneticIdentity, Blake3Digest) {
        let asset = FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
        let manifest = asset.manifest();
        let foundation = FoundationGeneticIdentity::new(
            manifest.foundation_id().raw(),
            manifest.foundation_version().raw() as u16,
            manifest.compatibility_family_id().raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap();
        (foundation, asset.digest())
    }

    fn checked_foundation_for_class(
        sensor_profile: SensorProfile,
        brain_class_id: alife_core::BrainClassId,
    ) -> (FoundationGeneticIdentity, Blake3Digest) {
        let asset = match brain_class_id {
            BrainCapacityClass::N2048_ID => {
                FoundationWeightAsset::builtin_n2048_v1(sensor_profile).unwrap()
            }
            _ => panic!("test helper only constructs the wrong N2048 foundation"),
        };
        let manifest = asset.manifest();
        (
            FoundationGeneticIdentity::new(
                manifest.foundation_id().raw(),
                manifest.foundation_version().raw() as u16,
                manifest.compatibility_family_id().raw(),
                brain_class_id,
            )
            .unwrap(),
            asset.digest(),
        )
    }

    fn plan_for_profile(
        sensor_profile: SensorProfile,
        target_population: u32,
    ) -> CuratedFounderPlan {
        let (foundation, foundation_content_digest) = checked_foundation(sensor_profile);
        let final_agents = (0..target_population)
            .rev()
            .map(|slot| CuratedFounderAgentInput {
                world_entity_id: WorldEntityId(WORLD_ENTITY_IDS_BY_SLOT[slot as usize]),
                organism_id: Some(OrganismId(ORGANISM_IDS_BY_SLOT[slot as usize])),
                final_population_slot: slot,
                legacy_genome_id: None,
            })
            .collect();
        plan_curated_founder_reset(&CuratedFounderResetRequest {
            policy_label: Some(CURATED_FOUNDER_RESET_POLICY.to_string()),
            source_save_identity: "save:curated-founder-materializer".to_string(),
            source_save_label: "curated founder materializer test source".to_string(),
            source_save_seed: 0x1111_2222_3333_4444,
            world_seed: 0x5555_6666_7777_8888,
            restored_tick: Tick::new(42_000),
            target_population,
            sensor_profile,
            foundation,
            foundation_content_digest,
            source_run_identity: "run:curated-founder-materializer".to_string(),
            final_agents,
        })
        .unwrap()
    }

    fn serialized_tamper(
        plan: &CuratedFounderPlan,
        mutate: impl FnOnce(&mut CuratedFounderPlan),
    ) -> CuratedFounderPlan {
        let serialized = serde_json::to_vec(plan).unwrap();
        let mut tampered: CuratedFounderPlan = serde_json::from_slice(&serialized).unwrap();
        mutate(&mut tampered);
        let resealed = serde_json::to_vec(&tampered).unwrap();
        serde_json::from_slice(&resealed).unwrap()
    }

    fn assert_plan_rejected(label: &str, plan: CuratedFounderPlan) {
        let result = materialize_curated_founder_bundle(&plan);
        match result {
            Err(CuratedFounderMaterializationError::Plan(_)) => {}
            other => panic!("{label} must fail through plan validation, got {other:?}"),
        }
    }

    #[test]
    fn curated_founder_materializer_builds_complete_plan_ordered_bundle() {
        for sensor_profile in [
            SensorProfile::PrivilegedAffordanceV1,
            SensorProfile::GroundedObjectSlotsV1,
        ] {
            let plan = plan_for_profile(sensor_profile, 3);
            let bundle = materialize_curated_founder_bundle(&plan).unwrap();
            let expected_world_entity_ids = WORLD_ENTITY_IDS_BY_SLOT
                .into_iter()
                .map(WorldEntityId)
                .collect::<Vec<_>>();
            let expected_organism_ids = ORGANISM_IDS_BY_SLOT
                .into_iter()
                .map(OrganismId)
                .collect::<Vec<_>>();

            assert_eq!(bundle.identity.plan_receipt, plan.receipt);
            assert_eq!(bundle.entries.len(), 3);
            assert_eq!(
                plan.entries
                    .iter()
                    .map(|entry| entry.world_entity_id)
                    .collect::<Vec<_>>(),
                expected_world_entity_ids
            );
            assert_eq!(
                plan.entries
                    .iter()
                    .map(|entry| entry.organism_id)
                    .collect::<Vec<_>>(),
                expected_organism_ids
            );
            assert_eq!(
                bundle
                    .entries
                    .iter()
                    .map(|entry| entry.plan_entry.world_entity_id)
                    .collect::<Vec<_>>(),
                expected_world_entity_ids
            );
            assert_eq!(
                bundle
                    .entries
                    .iter()
                    .map(|entry| entry.plan_entry.organism_id)
                    .collect::<Vec<_>>(),
                expected_organism_ids
            );
            for (bundle_entry, plan_entry) in bundle.entries.iter().zip(&plan.entries) {
                assert_eq!(bundle_entry.plan_entry, *plan_entry);
                let expected_genome = CreatureGenome::early_mammal_founder(
                    plan_entry.conception_seed,
                    plan.foundation,
                )
                .unwrap();
                let expected_phenotype = expected_genome.express().unwrap();
                let expected_biochemistry =
                    BiochemistryState::new(&expected_phenotype, plan.restored_tick).unwrap();

                assert_eq!(bundle_entry.genome, expected_genome);
                assert_eq!(bundle_entry.phenotype, expected_phenotype);
                assert_eq!(bundle_entry.biochemistry, expected_biochemistry);
                assert_eq!(
                    bundle_entry.biochemistry.source_genome_id,
                    expected_genome.id
                );
                assert_eq!(bundle_entry.biochemistry.tick, plan.restored_tick);
                assert_eq!(
                    bundle_entry.biochemistry.homeostasis.tick,
                    plan.restored_tick
                );

                let projection = &bundle_entry.projection;
                projection.validate().unwrap();
                projection
                    .receipt()
                    .validate_against_projection(projection)
                    .unwrap();
                assert_eq!(projection.source_genome_id(), expected_genome.id);
                assert_eq!(projection.lineage_id(), expected_genome.lineage_id);
                assert_eq!(projection.foundation(), &plan.foundation);
                assert_eq!(projection.sensor_profile(), sensor_profile);
                assert_eq!(
                    projection.foundation_asset_digest(),
                    plan.foundation_content_digest
                );
                assert_eq!(
                    projection.source_brain_genome(),
                    &expected_phenotype.brain_genome
                );
                assert_eq!(
                    projection.genetic_provenance(),
                    &expected_phenotype.genetic_provenance
                );
                assert_eq!(
                    projection.runtime_development_state().genome_id,
                    expected_genome.id
                );
                assert_ne!(
                    projection.frozen_abi().coordinate_genome().id,
                    expected_phenotype.brain_genome.id
                );
            }
        }
    }

    #[test]
    fn curated_founder_materializer_is_deterministic_and_fresh() {
        let plan = plan_for_profile(SensorProfile::PrivilegedAffordanceV1, 3);
        let mut first = materialize_curated_founder_bundle(&plan).unwrap();
        let second = materialize_curated_founder_bundle(&plan).unwrap();

        assert_eq!(first.identity, second.identity);
        for (first_entry, second_entry) in first.entries.iter().zip(&second.entries) {
            assert_eq!(first_entry.plan_entry, second_entry.plan_entry);
            assert_eq!(first_entry.genome, second_entry.genome);
            assert_eq!(first_entry.phenotype, second_entry.phenotype);
            assert_eq!(first_entry.projection, second_entry.projection);
            assert_eq!(first_entry.biochemistry, second_entry.biochemistry);
        }

        first.entries[0].biochemistry.tick = Tick::new(plan.restored_tick.raw() + 1);
        assert_ne!(
            first.entries[0].biochemistry,
            second.entries[0].biochemistry
        );
        assert_eq!(second.entries[0].biochemistry.tick, plan.restored_tick);
    }

    #[test]
    fn curated_founder_materializer_rejects_forged_plan_before_output() {
        let plan = plan_for_profile(SensorProfile::PrivilegedAffordanceV1, 3);
        let cases: &[(&str, fn(&mut CuratedFounderPlan))] = &[
            ("world entity ID", |plan| {
                plan.entries[0].world_entity_id = WorldEntityId(90_001)
            }),
            ("organism ID", |plan| {
                plan.entries[0].organism_id = OrganismId(90_002)
            }),
            ("genome ID", |plan| {
                plan.entries[0].genome_id = GenomeId(90_003)
            }),
            ("lineage ID", |plan| {
                plan.entries[0].lineage_id = LineageId(90_004)
            }),
            ("conception seed", |plan| {
                plan.entries[0].conception_seed = plan.entries[0].conception_seed.wrapping_add(1)
            }),
            ("profile", |plan| {
                plan.sensor_profile = SensorProfile::GroundedObjectSlotsV1
            }),
            ("restored tick", |plan| {
                plan.restored_tick = Tick::new(plan.restored_tick.raw() + 1)
            }),
            ("plan receipt", |plan| plan.receipt.receipt_digest[0] ^= 1),
        ];

        for (label, mutate) in cases {
            assert_plan_rejected(label, serialized_tamper(&plan, *mutate));
        }
    }

    #[test]
    fn curated_founder_materializer_rejects_cross_class_and_wrong_foundation() {
        let plan = plan_for_profile(SensorProfile::PrivilegedAffordanceV1, 3);
        let (n2048_foundation, n2048_digest) = checked_foundation_for_class(
            SensorProfile::PrivilegedAffordanceV1,
            BrainCapacityClass::N2048_ID,
        );
        assert_plan_rejected(
            "N2048 foundation",
            serialized_tamper(&plan, |plan| {
                plan.foundation = n2048_foundation;
                plan.foundation_content_digest = n2048_digest;
            }),
        );
        assert_plan_rejected(
            "wrong profile asset digest",
            serialized_tamper(&plan, |plan| {
                plan.foundation_content_digest =
                    FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1)
                        .unwrap()
                        .digest();
            }),
        );
        assert_plan_rejected(
            "forged foundation identity",
            serialized_tamper(&plan, |plan| plan.foundation.foundation_id ^= 1),
        );
    }

    #[test]
    fn curated_founder_materializer_returns_no_partial_bundle() {
        let plan = plan_for_profile(SensorProfile::GroundedObjectSlotsV1, 3);
        let tampered = serialized_tamper(&plan, |plan| {
            plan.entries[2].conception_seed = plan.entries[1].conception_seed;
        });

        let result = materialize_curated_founder_bundle(&tampered);
        assert!(matches!(
            result,
            Err(CuratedFounderMaterializationError::Plan(_))
        ));
    }

    #[test]
    fn curated_founder_materializer_never_uses_fixed_scaffold_as_source() {
        let plan = plan_for_profile(SensorProfile::GroundedObjectSlotsV1, 3);
        let bundle = materialize_curated_founder_bundle(&plan).unwrap();

        assert_ne!(
            bundle.entries[0].projection.source_brain_genome(),
            bundle.entries[0]
                .projection
                .frozen_abi()
                .coordinate_genome()
        );
        assert_ne!(
            bundle.entries[0].projection.source_brain_genome(),
            bundle.entries[1].projection.source_brain_genome()
        );
        assert_ne!(
            bundle.entries[0].projection.receipt().source_genome_id(),
            bundle.entries[1].projection.receipt().source_genome_id()
        );
        for entry in &bundle.entries {
            assert_eq!(
                entry.projection.source_brain_genome(),
                &entry.phenotype.brain_genome
            );
            assert_eq!(
                entry.projection.genetic_provenance(),
                &entry.phenotype.genetic_provenance
            );
        }
    }
}

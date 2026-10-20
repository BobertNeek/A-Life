//! New procedural founders must use the same inherited compiler contract as New Game.
use super::*;

pub(super) fn for_canonical_founder(seed: u64) -> Result<FoundationWeightAsset> {
    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    // This provisional asset is only a New Game construction token. No neural
    // resident or exported policy uses its raw scaffold decoder/weights.
    let provisional = scaffold_asset(seed)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = provisional.manifest().sensor_profile();
    let game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &provisional)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("initial founder missing from canonical New Game")?;
    let genome = record.phenotype().brain_genome.clone();
    let capacity = BrainCapacityClass::n2048();
    let development = crate::gpu_live_runtime::foundation_construction_development(
        &genome,
        &capacity,
        &record
            .phenotype()
            .development_state_at(record.age_at(game.world.tick())?)?,
    )?;
    let native = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        config.sensor_profile,
    )?;
    Ok(FoundationWeightAsset::from_phenotype_for_genetic_birth(
        &native,
    )?)
}

fn scaffold_asset(seed: u64) -> Result<FoundationWeightAsset> {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(seed, capacity.id());
    let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0)?);
    let native = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedTerrainVisionV1,
    )?;
    Ok(FoundationWeightAsset::from_phenotype_for_genetic_birth(
        &native,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn newborn(
        asset: &FoundationWeightAsset,
        founder: u64,
        scene: u64,
    ) -> alife_world::CanonicalNewGame {
        let mut config = alife_world::CanonicalNewGameConfig::phase3(scene, 1).unwrap();
        config.brain_class = BrainScaleTier::Standard2048;
        config.sensor_profile = asset.manifest().sensor_profile();
        config.founder_seed_base = founder;
        alife_world::create_canonical_new_game_with_n2048_candidate(&config, asset).unwrap()
    }

    #[test]
    fn raw_scaffold_asset_is_rejected_by_the_expressed_founder_contract() {
        let raw = scaffold_asset(77_777).unwrap();
        let game = newborn(&raw, 77_777, 99_001);
        let expressed = &game
            .world
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .phenotype()
            .brain_genome;
        assert_ne!(
            expressed.innate_priority.food_attraction.to_bits(),
            BrainGenome::scaffold(77_777, BrainCapacityClass::N2048_ID)
                .innate_priority
                .food_attraction
                .to_bits(),
        );
        let repaired = for_canonical_founder(77_777).unwrap();
        assert_ne!(
            raw.manifest().action_decoder_digest(),
            repaired.manifest().action_decoder_digest()
        );
        assert_eq!(
            raw.manifest().layout_digest(),
            repaired.manifest().layout_digest()
        );
        assert_eq!(
            raw.manifest().address_map_digest(),
            repaired.manifest().address_map_digest()
        );
        assert!(matches!(
            GpuLiveBrainRuntime::validate_foundation_birth_for_test(
                &game.world,
                raw.manifest().sensor_profile(),
                alife_core::OrganismId(1),
            ),
            Err(crate::GameAppShellError::Core(
                ScaffoldContractError::PhenotypeCompile
            ))
        ));
    }

    #[test]
    fn initial_asset_admits_a_canonical_newborn_without_a_device() {
        for (founder, scene) in [(77_777, 99_001), (53, 53)] {
            let asset = for_canonical_founder(founder).unwrap();
            let game = newborn(&asset, founder, scene);
            GpuLiveBrainRuntime::validate_foundation_birth_for_test(
                &game.world,
                asset.manifest().sensor_profile(),
                alife_core::OrganismId(1),
            )
            .unwrap();
            assert_eq!(
                asset.encode_canonical().unwrap(),
                for_canonical_founder(founder)
                    .unwrap()
                    .encode_canonical()
                    .unwrap()
            );
        }
    }

    #[test]
    fn invalid_seed_cannot_create_a_procedural_founder() {
        assert!(for_canonical_founder(0).is_err());
        assert!(for_canonical_founder(u64::MAX).is_err());
    }
}

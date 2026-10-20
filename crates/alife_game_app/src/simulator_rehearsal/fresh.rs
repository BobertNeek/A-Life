//! Canonical scene setup shared by the runner and device-free admission tests.
use super::*;

pub(super) struct SetupContext {
    pub(super) founder: u64,
    pub(super) seed: u64,
    pub(super) rollout: u64,
}

#[derive(Debug, thiserror::Error)]
#[error("rehearsal fresh setup at {stage} (founder={founder}, scene={seed}, rollout={rollout}): {source}")]
struct SetupError {
    stage: &'static str,
    founder: u64,
    seed: u64,
    rollout: u64,
    #[source]
    source: Box<dyn std::error::Error>,
}

impl SetupContext {
    pub(super) fn stage<T, E>(
        &self,
        stage: &'static str,
        result: std::result::Result<T, E>,
    ) -> Result<T>
    where
        E: Into<Box<dyn std::error::Error>>,
    {
        result.map_err(|source| {
            Box::new(SetupError {
                stage,
                founder: self.founder,
                seed: self.seed,
                rollout: self.rollout,
                source: source.into(),
            }) as Box<dyn std::error::Error>
        })
    }
}

pub(super) fn prepare_scene(
    asset: &FoundationWeightAsset,
    context: &SetupContext,
) -> Result<(alife_world::CanonicalNewGame, WorldEntityId)> {
    let mut config = context.stage(
        "new-game configuration",
        alife_world::CanonicalNewGameConfig::phase3(context.seed, 1),
    )?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = context.founder;
    config.sensor_profile = asset.manifest().sensor_profile();
    let mut game = context.stage(
        "canonical founder construction",
        alife_world::create_canonical_new_game_with_n2048_candidate(&config, asset),
    )?;
    context.stage(
        "grounded terrain admission",
        crate::foundation_training::ground_foundation_training_world(&mut game.world),
    )?;
    let scenario = context.stage(
        "feeding scene preparation",
        crate::configure_foundation_scenario(
            &mut game.world,
            context.seed,
            Some(crate::FoundationTeacherLesson::GrabFood),
            None,
            true,
        ),
    )?;
    Ok((game, scenario.food))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_qualification_scene_admits_the_production_resident_without_a_device() {
        let asset = crate::initial_n2048_care_asset(77_777).unwrap();
        let (game, target) = prepare_scene(
            &asset,
            &SetupContext {
                founder: 77_777,
                seed: 99_001,
                rollout: 0,
            },
        )
        .unwrap();
        assert!(game.world.entity(target).is_some());
        let id = learner(&game.world).unwrap();
        GpuLiveBrainRuntime::validate_foundation_birth_for_test(
            &game.world,
            asset.manifest().sensor_profile(),
            id,
        )
        .unwrap();
    }

    #[test]
    fn inspected_native_asset_admits_fresh_scene_without_mutating_source() {
        // A synthetic native package exercises the real admission interface;
        // this is not evidence about private policy7400 weights or optimizer ages.
        let directory = crate::foundation_archived_success_replay::tests::checkpoint_fixture(17);
        let admitted = continuation::inspect(&directory, 17).unwrap();
        let asset = FoundationWeightAsset::decode_canonical(
            &continuation::read_verified(&directory, "trained.alife-foundation", &admitted)
                .unwrap(),
        )
        .unwrap();
        let (game, _) = prepare_scene(
            &asset,
            &SetupContext {
                founder: admitted.founder_seed_base,
                seed: 99_001,
                rollout: 7,
            },
        )
        .unwrap();
        GpuLiveBrainRuntime::validate_foundation_birth_for_test(
            &game.world,
            asset.manifest().sensor_profile(),
            learner(&game.world).unwrap(),
        )
        .unwrap();
        assert!(GpuLiveBrainRuntime::validate_foundation_birth_for_test(
            &game.world,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            learner(&game.world).unwrap(),
        )
        .is_err());
        for file in &admitted.file_digests {
            continuation::read_verified(&directory, file.file_name, &admitted).unwrap();
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn scene_setup_error_identifies_stage_and_lineage_without_hiding_cause() {
        let asset = crate::initial_n2048_care_asset(53).unwrap();
        let error = prepare_scene(
            &asset,
            &SetupContext {
                founder: 53,
                seed: 0,
                rollout: 9,
            },
        )
        .unwrap_err();
        let message = error.to_string();
        for expected in [
            "new-game configuration",
            "founder=53",
            "scene=0",
            "rollout=9",
        ] {
            assert!(message.contains(expected), "{message}");
        }
        assert!(error.source().is_some());
    }
}

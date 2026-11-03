//! One prespecified coverage follow-up. Every seed, failure and trajectory is
//! retained. Flat terrain makes the public simulator's locomotion plane X/Z;
//! preparation finishes before the first native perception/action.
use super::*;
use alife_core::Vec3f;
use alife_world::{LocomotionLimits, TerrainData, WorldObjectKind, WorldTerrain};

pub fn capture(config: &ResidualConfig) -> ResidualResult<Vec<PublicSimulationRecording>> {
    if !(5..=128).contains(&config.world_ticks) || config.seed == 0 {
        return Err("invalid coverage capture configuration".into());
    }
    let mut captures = Vec::new();
    for scene in 0..6_u64 {
        let base = config
            .seed
            .checked_add(10_000 + scene * 10)
            .ok_or("seed overflow")?;
        let capture = capture_prepared_public(
            PublicCaptureConfig {
                founder_seed: config.seed,
                world_seeds: [base + 1, base + 2, base + 3],
                sampling_seeds: [
                    0x510e7100 + scene as u32,
                    0x9b057200 + scene as u32,
                    0x1f837300 + scene as u32,
                ],
                world_ticks: config.world_ticks,
            },
            Some(PublicPreparation {
                scenario: format!("flat-xz-motion-coverage-v1-scene{scene}"),
                temperature: config.temperature,
            }),
            |variation, game| {
                game.world.enable_terrain_for_new_game(
                    WorldTerrain::new(
                        TerrainData {
                            width: 2,
                            depth: 2,
                            origin_x: -32.,
                            origin_z: -32.,
                            spacing: 64.,
                            heights: vec![0.; 4],
                            obstacles: vec![],
                            water_level: None,
                        },
                        LocomotionLimits::default(),
                    )?,
                    Vec3f::ZERO,
                )?;
                let agent = game
                    .world
                    .object_snapshots()
                    .into_iter()
                    .find(|o| o.kind == WorldObjectKind::Agent)
                    .ok_or("coverage scene lacks agent")?
                    .position;
                // Six evenly spread bearings, with heldout offsets fixed before any
                // capture. All splits contain real positive/negative Z approaches.
                let angle = (scene as f32 - 2.5) * std::f32::consts::FRAC_PI_3
                    + [0., 0.17, -0.23][variation];
                let distance = 3. + (scene % 3) as f32 + 0.25 * variation as f32;
                let at = |along: f32, side: f32| {
                    Vec3f::new(
                        agent.x + along * angle.cos() - side * angle.sin(),
                        agent.y,
                        agent.z + along * angle.sin() + side * angle.cos(),
                    )
                };
                for (label, position) in [
                    ("food-01", at(distance, 0.)),
                    (
                        "meadow-ball",
                        at(1.5, if scene % 2 == 0 { 0.8 } else { -0.8 }),
                    ),
                    (
                        "obstacle-01",
                        at(if scene % 3 == 0 { 1.7 } else { 7. }, 0.7),
                    ),
                    ("hazard-01", at(if scene % 3 == 1 { 1.8 } else { 7. }, -0.7)),
                ] {
                    let id = game
                        .world
                        .entity_id(label)
                        .ok_or("coverage scene object missing")?;
                    game.world.editor_move_object(id, position)?;
                }
                Ok(())
            },
        )?;
        if captures
            .first()
            .is_some_and(|first: &PublicSimulationRecording| {
                identity(&first.source).ok() != identity(&capture.source).ok()
            })
        {
            return Err("coverage worlds changed the frozen source".into());
        }
        captures.push(capture);
    }
    Ok(captures)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_coverage_capture_produces_real_train_z_motion_with_disjoint_worlds() {
        let config = ResidualConfig {
            motion_coverage: true,
            world_ticks: 6,
            epochs: 1,
            ..ResidualConfig::default()
        };
        let captures = capture(&config).unwrap();
        assert_eq!(captures.len(), 6);
        let data = horizon_dataset(&captures, 1).unwrap();
        let scaling = fit_training_scales(&data, config.scale_floor).unwrap();
        assert_eq!(scaling.train_pairs, 30);
        let names = data
            .schema
            .sensors
            .iter()
            .chain(&data.schema.body)
            .collect::<Vec<_>>();
        for channel in ["body.position_z", "body.velocity_z"] {
            let i = names.iter().position(|n| n.as_str() == channel).unwrap();
            assert!(
                scaling.train_delta_rms[i] > 0.001,
                "{channel}: {}",
                scaling.train_delta_rms[i]
            );
        }
        let groups = captures.iter().flat_map(|c| &c.groups).collect::<Vec<_>>();
        assert_eq!(
            groups
                .iter()
                .map(|g| g.world_seed)
                .collect::<BTreeSet<_>>()
                .len(),
            18
        );
        assert!(groups.iter().all(|g| g.decisions.len() == 6));
    }
}

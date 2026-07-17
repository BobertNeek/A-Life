use crate::{CreatureAnimationState, CreaturePartSlot};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureRootPose {
    pub translation: [f32; 3],
    pub rotation_xyz: [f32; 3],
    pub scale: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreaturePartPose {
    pub translation: [f32; 3],
    pub rotation_xyz: [f32; 3],
    pub scale: [f32; 3],
}

pub fn creature_root_pose(
    animation: CreatureAnimationState,
    wave: f32,
    lateral: f32,
) -> CreatureRootPose {
    let wave = wave.clamp(-1.0, 1.0);
    let lateral = lateral.clamp(-1.0, 1.0);
    let (translation, rotation_xyz) = match animation {
        CreatureAnimationState::Sleeping => (
            [0.0, -0.38 + wave.abs() * 0.012, 0.02],
            [0.08, wave * 0.025, 1.28],
        ),
        CreatureAnimationState::Resting => (
            [0.0, -0.20 + wave.abs() * 0.018, 0.02],
            [-0.18, wave * 0.04, 0.0],
        ),
        CreatureAnimationState::Afraid => (
            [lateral * 0.035, wave.abs() * 0.025, wave * 0.018],
            [0.0, lateral * 0.12, lateral * 0.04],
        ),
        CreatureAnimationState::Hurt => (
            [lateral * 0.025, -0.08 + wave.abs() * 0.018, 0.0],
            [0.10, lateral * 0.08, lateral * 0.15],
        ),
        CreatureAnimationState::Curious | CreatureAnimationState::Inspecting => {
            ([0.0, wave.abs() * 0.07, 0.0], [-0.04, wave * 0.20, 0.0])
        }
        CreatureAnimationState::Moving => (
            [0.0, wave.abs() * 0.11, 0.0],
            [0.0, wave * 0.14, lateral * 0.025],
        ),
        CreatureAnimationState::Interacting | CreatureAnimationState::Signaling => {
            ([0.0, wave.abs() * 0.08, 0.0], [0.0, wave * 0.25, 0.0])
        }
        CreatureAnimationState::Idle => {
            ([0.0, wave * 0.025, 0.0], [wave * 0.012, wave * 0.045, 0.0])
        }
    };
    CreatureRootPose {
        translation,
        rotation_xyz,
        scale: [1.0; 3],
    }
}

pub fn creature_part_pose(
    animation: CreatureAnimationState,
    slot: CreaturePartSlot,
    wave: f32,
) -> CreaturePartPose {
    let wave = wave.clamp(-1.0, 1.0);
    let mut pose = CreaturePartPose {
        translation: [0.0; 3],
        rotation_xyz: [0.0; 3],
        scale: [1.0; 3],
    };
    match animation {
        CreatureAnimationState::Moving => match slot {
            CreaturePartSlot::LeftArm => pose.rotation_xyz[0] = wave * 0.42,
            CreaturePartSlot::RightArm => pose.rotation_xyz[0] = -wave * 0.42,
            CreaturePartSlot::LeftLeg => pose.rotation_xyz[0] = -wave * 0.32,
            CreaturePartSlot::RightLeg => pose.rotation_xyz[0] = wave * 0.32,
            CreaturePartSlot::Head => pose.rotation_xyz[1] = wave * 0.08,
            CreaturePartSlot::TailBack => pose.rotation_xyz[2] = wave * 0.16,
            CreaturePartSlot::Torso => {}
        },
        CreatureAnimationState::Resting => match slot {
            CreaturePartSlot::Head => pose.rotation_xyz[0] = 0.16,
            CreaturePartSlot::Torso => pose.rotation_xyz[0] = -0.10,
            CreaturePartSlot::LeftArm | CreaturePartSlot::RightArm => {
                pose.rotation_xyz[0] = 0.58;
                pose.translation[1] = -0.025;
            }
            CreaturePartSlot::LeftLeg | CreaturePartSlot::RightLeg => {
                pose.rotation_xyz[0] = 0.78;
                pose.translation[2] = 0.035;
            }
            CreaturePartSlot::TailBack => pose.rotation_xyz[0] = -0.24,
        },
        CreatureAnimationState::Sleeping => match slot {
            CreaturePartSlot::Head => pose.rotation_xyz = [0.24, -0.08, -0.18],
            CreaturePartSlot::Torso => pose.rotation_xyz[0] = 0.10,
            CreaturePartSlot::LeftArm => {
                pose.rotation_xyz = [0.92, 0.0, 0.30];
                pose.translation[0] = 0.045;
            }
            CreaturePartSlot::RightArm => {
                pose.rotation_xyz = [0.88, 0.0, -0.24];
                pose.translation[0] = -0.045;
            }
            CreaturePartSlot::LeftLeg => {
                pose.rotation_xyz = [1.02, 0.0, 0.18];
                pose.translation[0] = 0.035;
            }
            CreaturePartSlot::RightLeg => {
                pose.rotation_xyz = [1.08, 0.0, -0.16];
                pose.translation[0] = -0.035;
            }
            CreaturePartSlot::TailBack => pose.rotation_xyz = [-0.38, 0.15, 0.22],
        },
        CreatureAnimationState::Interacting => match slot {
            CreaturePartSlot::LeftArm => pose.rotation_xyz = [-0.42 + wave * 0.18, 0.0, -0.18],
            CreaturePartSlot::RightArm => pose.rotation_xyz = [-0.42 - wave * 0.18, 0.0, 0.18],
            CreaturePartSlot::Head => pose.rotation_xyz[1] = wave * 0.18,
            _ => {}
        },
        CreatureAnimationState::Signaling => match slot {
            CreaturePartSlot::RightArm => pose.rotation_xyz = [-1.05, 0.0, -0.18 + wave * 0.18],
            CreaturePartSlot::LeftArm => pose.rotation_xyz[0] = wave * 0.12,
            CreaturePartSlot::Head => pose.rotation_xyz[1] = wave * 0.12,
            _ => {}
        },
        CreatureAnimationState::Curious | CreatureAnimationState::Inspecting => match slot {
            CreaturePartSlot::Head => pose.rotation_xyz = [-0.12, wave * 0.22, wave * 0.08],
            CreaturePartSlot::LeftArm => pose.rotation_xyz[0] = 0.16 + wave * 0.08,
            CreaturePartSlot::RightArm => pose.rotation_xyz[0] = 0.16 - wave * 0.08,
            _ => {}
        },
        CreatureAnimationState::Afraid => match slot {
            CreaturePartSlot::LeftArm => pose.rotation_xyz = [0.48, 0.0, -0.18],
            CreaturePartSlot::RightArm => pose.rotation_xyz = [0.48, 0.0, 0.18],
            CreaturePartSlot::LeftLeg => pose.rotation_xyz[0] = -wave * 0.12,
            CreaturePartSlot::RightLeg => pose.rotation_xyz[0] = wave * 0.12,
            CreaturePartSlot::Head => pose.rotation_xyz[0] = 0.12,
            _ => {}
        },
        CreatureAnimationState::Hurt => match slot {
            CreaturePartSlot::LeftArm => pose.rotation_xyz[0] = 0.34,
            CreaturePartSlot::RightArm => pose.rotation_xyz[0] = 0.52,
            CreaturePartSlot::LeftLeg => pose.rotation_xyz[0] = 0.18,
            CreaturePartSlot::RightLeg => pose.rotation_xyz[0] = 0.30,
            CreaturePartSlot::Head => pose.rotation_xyz[2] = -0.12,
            _ => {}
        },
        CreatureAnimationState::Idle => match slot {
            CreaturePartSlot::LeftArm => pose.rotation_xyz[0] = wave * 0.08,
            CreaturePartSlot::RightArm => pose.rotation_xyz[0] = -wave * 0.08,
            CreaturePartSlot::Head => pose.rotation_xyz[1] = wave * 0.035,
            CreaturePartSlot::TailBack => pose.rotation_xyz[2] = wave * 0.10,
            _ => {}
        },
    }
    pose
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATES: [CreatureAnimationState; 10] = [
        CreatureAnimationState::Idle,
        CreatureAnimationState::Moving,
        CreatureAnimationState::Inspecting,
        CreatureAnimationState::Interacting,
        CreatureAnimationState::Sleeping,
        CreatureAnimationState::Afraid,
        CreatureAnimationState::Hurt,
        CreatureAnimationState::Curious,
        CreatureAnimationState::Signaling,
        CreatureAnimationState::Resting,
    ];

    #[test]
    fn every_root_pose_is_finite_and_never_crushes_anatomy() {
        for state in STATES {
            for wave in [-1.0, 0.0, 1.0] {
                let pose = creature_root_pose(state, wave, -wave);
                assert!(pose
                    .translation
                    .into_iter()
                    .chain(pose.rotation_xyz)
                    .chain(pose.scale)
                    .all(f32::is_finite));
                assert!(
                    pose.scale
                        .into_iter()
                        .all(|axis| (0.82..=1.18).contains(&axis)),
                    "{state:?} must pose joints instead of crushing the root: {:?}",
                    pose.scale
                );
            }
        }
    }

    #[test]
    fn resting_and_sleeping_are_articulated_crouch_and_side_curl_poses() {
        let rest_root = creature_root_pose(CreatureAnimationState::Resting, 0.0, 0.0);
        let sleep_root = creature_root_pose(CreatureAnimationState::Sleeping, 0.0, 0.0);
        assert!(rest_root.rotation_xyz[0].abs() >= 0.12);
        assert!(sleep_root.rotation_xyz[2].abs() >= 1.0);
        assert!(sleep_root.translation[1] <= -0.25);

        for state in [
            CreatureAnimationState::Resting,
            CreatureAnimationState::Sleeping,
        ] {
            let left_arm = creature_part_pose(state, CreaturePartSlot::LeftArm, 0.0);
            let right_arm = creature_part_pose(state, CreaturePartSlot::RightArm, 0.0);
            let left_leg = creature_part_pose(state, CreaturePartSlot::LeftLeg, 0.0);
            let right_leg = creature_part_pose(state, CreaturePartSlot::RightLeg, 0.0);
            assert!(left_arm.rotation_xyz[0].abs() >= 0.30);
            assert!(right_arm.rotation_xyz[0].abs() >= 0.30);
            assert!(left_leg.rotation_xyz[0].abs() >= 0.35);
            assert!(right_leg.rotation_xyz[0].abs() >= 0.35);
        }
    }

    #[test]
    fn moving_pose_swings_opposite_limbs_without_detaching_parts() {
        let left_arm = creature_part_pose(
            CreatureAnimationState::Moving,
            CreaturePartSlot::LeftArm,
            1.0,
        );
        let right_arm = creature_part_pose(
            CreatureAnimationState::Moving,
            CreaturePartSlot::RightArm,
            1.0,
        );
        let left_leg = creature_part_pose(
            CreatureAnimationState::Moving,
            CreaturePartSlot::LeftLeg,
            1.0,
        );
        let right_leg = creature_part_pose(
            CreatureAnimationState::Moving,
            CreaturePartSlot::RightLeg,
            1.0,
        );
        assert!(left_arm.rotation_xyz[0] * right_arm.rotation_xyz[0] < 0.0);
        assert!(left_leg.rotation_xyz[0] * right_leg.rotation_xyz[0] < 0.0);
        for pose in [left_arm, right_arm, left_leg, right_leg] {
            assert!(pose.translation.into_iter().all(|axis| axis.abs() <= 0.12));
            assert_eq!(pose.scale, [1.0; 3]);
        }
    }
}

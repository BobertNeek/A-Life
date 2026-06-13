//! v0 scaffold: explicit Bevy math conversion helpers.

use alife_core::{Pose, Quatf, ScaffoldContractError, Vec3f};
use bevy::prelude::{Quat, Transform, Vec3};

pub fn bevy_vec3_to_core(value: Vec3) -> Result<Vec3f, ScaffoldContractError> {
    Vec3f::new(value.x, value.y, value.z).validate()
}

pub fn core_vec3_to_bevy(value: Vec3f) -> Result<Vec3, ScaffoldContractError> {
    value.validate()?;
    Ok(Vec3::new(value.x, value.y, value.z))
}

pub fn bevy_quat_to_core(value: Quat) -> Result<Quatf, ScaffoldContractError> {
    Quatf::new(value.x, value.y, value.z, value.w).validate()
}

pub fn core_quat_to_bevy(value: Quatf) -> Result<Quat, ScaffoldContractError> {
    value.validate()?;
    Ok(Quat::from_xyzw(value.x, value.y, value.z, value.w))
}

pub fn bevy_transform_to_core_pose(value: Transform) -> Result<Pose, ScaffoldContractError> {
    let pose = Pose {
        translation: bevy_vec3_to_core(value.translation)?,
        rotation: bevy_quat_to_core(value.rotation)?,
    };
    pose.validate()
}

pub fn core_pose_to_bevy_transform(value: Pose) -> Result<Transform, ScaffoldContractError> {
    value.validate()?;
    Ok(
        Transform::from_translation(core_vec3_to_bevy(value.translation)?)
            .with_rotation(core_quat_to_bevy(value.rotation)?),
    )
}

//! v0 scaffold: engine-independent math primitives.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::ScaffoldContractError;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Pod, Zeroable)]
pub struct Vec2f {
    pub x: f32,
    pub y: f32,
}

impl Vec2f {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        validate_finite_slice(&[self.x, self.y])?;
        Ok(self)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Pod, Zeroable)]
pub struct Vec3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3f {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    pub const fn from_array(values: [f32; 3]) -> Self {
        Self {
            x: values[0],
            y: values[1],
            z: values[2],
        }
    }

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        validate_finite_slice(&self.to_array())?;
        Ok(self)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Pod, Zeroable)]
pub struct Quatf {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quatf {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        validate_finite_slice(&[self.x, self.y, self.z, self.w])?;
        if self.x == 0.0 && self.y == 0.0 && self.z == 0.0 && self.w == 0.0 {
            Err(ScaffoldContractError::ScalarOutOfRange)
        } else {
            Ok(self)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    pub min: Vec3f,
    pub max: Vec3f,
}

impl Aabb {
    pub fn new(min: Vec3f, max: Vec3f) -> Result<Self, ScaffoldContractError> {
        min.validate()?;
        max.validate()?;
        if min.x <= max.x && min.y <= max.y && min.z <= max.z {
            Ok(Self { min, max })
        } else {
            Err(ScaffoldContractError::InvalidBounds)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub translation: Vec3f,
    pub rotation: Quatf,
}

impl Pose {
    pub const IDENTITY: Self = Self {
        translation: Vec3f::ZERO,
        rotation: Quatf::IDENTITY,
    };

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        self.translation.validate()?;
        self.rotation.validate()?;
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Velocity {
    pub linear: Vec3f,
    pub angular: Vec3f,
}

impl Velocity {
    pub const ZERO: Self = Self {
        linear: Vec3f::ZERO,
        angular: Vec3f::ZERO,
    };

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        self.linear.validate()?;
        self.angular.validate()?;
        Ok(self)
    }
}

pub fn validate_finite(value: f32) -> Result<f32, ScaffoldContractError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ScaffoldContractError::NonFiniteFloat)
    }
}

pub fn validate_finite_slice(values: &[f32]) -> Result<(), ScaffoldContractError> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(ScaffoldContractError::NonFiniteFloat)
    }
}

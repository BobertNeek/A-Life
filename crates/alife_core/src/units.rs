//! v0 scaffold: simulation units and bounded scalar wrappers.

use serde::{Deserialize, Serialize};

use crate::{math::validate_finite, ScaffoldContractError};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Tick(pub u64);

impl Tick {
    pub const ZERO: Self = Self(0);

    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn validate_monotonic(
        previous: Self,
        current: Self,
    ) -> Result<Self, ScaffoldContractError> {
        if current.0 >= previous.0 {
            Ok(current)
        } else {
            Err(ScaffoldContractError::NonMonotonicTick)
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DurationTicks(pub u32);

impl DurationTicks {
    pub const ZERO: Self = Self(0);

    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Seconds(pub f32);

impl Seconds {
    pub fn new(value: f32) -> Result<Self, ScaffoldContractError> {
        validate_finite(value)?;
        if value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(ScaffoldContractError::ScalarOutOfRange)
        }
    }

    pub const fn raw(self) -> f32 {
        self.0
    }
}

macro_rules! bounded_scalar {
    ($name:ident, $min:expr, $max:expr) => {
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
        pub struct $name(pub f32);

        impl $name {
            pub fn new(value: f32) -> Result<Self, ScaffoldContractError> {
                validate_finite(value)?;
                if ($min..=$max).contains(&value) {
                    Ok(Self(value))
                } else {
                    Err(ScaffoldContractError::ScalarOutOfRange)
                }
            }

            pub const fn raw(self) -> f32 {
                self.0
            }
        }
    };
}

bounded_scalar!(NormalizedScalar, 0.0, 1.0);
bounded_scalar!(Confidence, 0.0, 1.0);
bounded_scalar!(Intensity, 0.0, 1.0);
bounded_scalar!(SignedValence, -1.0, 1.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixedPointScale {
    pub fractional_bits: u8,
}

impl FixedPointScale {
    pub const Q8_8: Self = Self { fractional_bits: 8 };
    pub const Q16_16: Self = Self {
        fractional_bits: 16,
    };

    pub fn new(fractional_bits: u8) -> Result<Self, ScaffoldContractError> {
        if fractional_bits <= 30 {
            Ok(Self { fractional_bits })
        } else {
            Err(ScaffoldContractError::ScalarOutOfRange)
        }
    }
}

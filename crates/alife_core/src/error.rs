//! v0 scaffold: validation errors for architecture contract checks.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScaffoldContractError {
    #[error("brain class must contain at least 512 neurons")]
    BrainClassTooSmall,
    #[error("near-term GPU brain classes must align neuron count to 128")]
    BrainClassAlignment,
    #[error("lobe layout total does not match brain neuron count")]
    LobeTotalMismatch,
    #[error("lobe starts and lengths must align to 16")]
    LobeAlignment,
    #[error("requested brain tier has no canonical neuron count")]
    MissingCanonicalNeuronCount,
    #[error("ID value zero is reserved as invalid")]
    InvalidId,
    #[error("float value must be finite")]
    NonFiniteFloat,
    #[error("scalar value is outside its allowed range")]
    ScalarOutOfRange,
    #[error("tick value moved backward")]
    NonMonotonicTick,
    #[error("axis-aligned bounds are invalid")]
    InvalidBounds,
}

//! v0 scaffold: engine-agnostic A-Life cognitive contracts.

pub mod action;
pub mod action_abi;
pub mod brain_class;
pub mod chemistry;
pub mod error;
pub mod experience;
pub mod genome;
pub mod ids;
pub mod lineage;
pub mod lobe;
pub mod sensory_abi;
pub mod traits;

pub use action::{ActionCommand, ActionKind};
pub use action_abi::ActionAbiVersion;
pub use brain_class::{BrainClassSpec, BrainScaleTier};
pub use chemistry::EndocrineProfile;
pub use error::ScaffoldContractError;
pub use experience::{ExperiencePatchHeader, ExperiencePatchPhase};
pub use genome::BrainGenome;
pub use ids::{BrainClassId, GenomeId, LineageId, OrganismId, WorldEntityId};
pub use lineage::LineageExportManifest;
pub use lobe::{LobeKind, LobeLayout, LobeRegion};
pub use sensory_abi::{SensoryAbiVersion, TeacherPerceptionChannel};
pub use traits::{
    NeuralComputeBackend, SemanticPriorPacket, SemanticPriorProvider, SemanticPriorRequest,
};

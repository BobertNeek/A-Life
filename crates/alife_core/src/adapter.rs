//! v0 scaffold: adapter-boundary conversion traits.
//!
//! External engine handles, adapter math, Gaussian adapter handles, and
//! renderer/backend objects must be converted outside `alife_core`. Adapter
//! crates implement these traits on their side and pass only core IDs/math into
//! cognition.

use crate::WorldEntityId;

pub trait CoreFromAdapter<T>: Sized {
    fn core_from_adapter(value: T) -> Self;
}

pub trait CoreIntoAdapter<T> {
    fn core_into_adapter(self) -> T;
}

pub trait WorldEntityIdMapper<AdapterEntity> {
    fn to_world_entity_id(&self, entity: AdapterEntity) -> WorldEntityId;
}

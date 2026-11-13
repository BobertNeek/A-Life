//! Experimental detached decision-context primitives, CPU only.
//! No production replay, collector, checkpoint or body route is changed or admitted.
//! Kept behind an explicit feature while full host integration is reviewed.
use super::{adam_coordinate, finite, invalid, AdamWConfig, TrainingError};
mod current_self;
mod retained;
pub use current_self::*;
pub use retained::*;
#[cfg(test)]
mod tests;
mod work;
pub use work::*;

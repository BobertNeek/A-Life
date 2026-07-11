//! Contract-only identifiers shared by phenotype compilation and GPU receipts.

use serde::{Deserialize, Serialize};

/// Stable content hash of one compiled brain phenotype.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhenotypeHash(pub [u64; 4]);

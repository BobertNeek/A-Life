//! v0 scaffold: optional semantic-code and concept binding conversion.

use alife_core::{
    CompressedSemanticCode, ConceptCellId, Confidence, ContextFeatureFlags, NormalizedScalar,
    ScaffoldContractError, SemanticContextRef, SemanticSalienceEntry,
};

/// Maximum number of semantic code slots kept for a packed context.
pub const MAX_SEMANTIC_CODE_COUNT: usize = 12;

/// Maximum number of semantic concept binding slots kept for a packed context.
pub const MAX_SEMANTIC_CONTEXT_BINDINGS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticConceptBinding {
    pub concept_id: ConceptCellId,
    pub salience: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticCodeDescriptor {
    pub codebook_id: u16,
    pub descriptor: [i8; 32],
    pub salience: f32,
}

impl SemanticConceptBinding {
    fn to_entry(self) -> Result<SemanticSalienceEntry, ScaffoldContractError> {
        self.concept_id.validate()?;
        Ok(SemanticSalienceEntry {
            concept_id: self.concept_id,
            salience: NormalizedScalar::new(self.salience)?,
        })
    }
}

impl SemanticCodeDescriptor {
    fn to_entry(self) -> Result<CompressedSemanticCode, ScaffoldContractError> {
        if self.codebook_id == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        let salience = NormalizedScalar::new(self.salience)?;
        Ok(CompressedSemanticCode {
            codebook_id: self.codebook_id,
            code: stable_descriptor_code(self.descriptor),
            salience,
        })
    }
}

/// Convert adapter-side semantic hints into optional core semantic context.
pub fn build_semantic_context(
    bindings: &[SemanticConceptBinding],
    descriptors: &[SemanticCodeDescriptor],
    confidence: f32,
) -> Result<Option<SemanticContextRef>, ScaffoldContractError> {
    let mut salience_entries: Vec<SemanticSalienceEntry> = bindings
        .iter()
        .copied()
        .filter(|binding| binding.salience > 0.0)
        .map(SemanticConceptBinding::to_entry)
        .collect::<Result<_, _>>()?;

    if salience_entries.is_empty() && descriptors.is_empty() {
        return Ok(None);
    }

    let confidence = Confidence::new(confidence)?;
    if confidence.raw() == 0.0 {
        return Ok(None);
    }

    salience_entries.sort_by(|lhs, rhs| {
        rhs.salience
            .raw()
            .total_cmp(&lhs.salience.raw())
            .then(rhs.concept_id.raw().cmp(&lhs.concept_id.raw()))
    });
    salience_entries.truncate(MAX_SEMANTIC_CONTEXT_BINDINGS);

    let mut code_entries: Vec<CompressedSemanticCode> = descriptors
        .iter()
        .copied()
        .filter(|descriptor| descriptor.salience > 0.0)
        .map(SemanticCodeDescriptor::to_entry)
        .collect::<Result<_, _>>()?;
    code_entries.sort_by(|lhs, rhs| {
        rhs.salience
            .raw()
            .total_cmp(&lhs.salience.raw())
            .then_with(|| rhs.code.cmp(&lhs.code))
    });
    code_entries.truncate(MAX_SEMANTIC_CODE_COUNT);

    if salience_entries.is_empty() && code_entries.is_empty() {
        return Ok(None);
    }

    let mut feature_flags = ContextFeatureFlags::INTERNAL_SLM_MODULATION;
    if !code_entries.is_empty() {
        feature_flags |= ContextFeatureFlags::SEMANTIC_CODES;
    }

    Ok(Some(SemanticContextRef {
        feature_flags,
        confidence,
        compressed_codes: code_entries,
        salience: salience_entries,
    }))
}

fn stable_descriptor_code(descriptor: [i8; 32]) -> u32 {
    let mut hash = 0x811c9dc5u32;
    for value in descriptor {
        hash ^= u32::from(value.to_le_bytes()[0]);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

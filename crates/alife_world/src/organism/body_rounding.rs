//! Explicit, source-bound admission of saved f32 exp2 rounding. No saved value
//! is replaced. Ordinary validation remains exact outside this lexical scope.
use super::*;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedBodyRoundingCompatibilityV1 {
    pub schema: u16,
    pub kind: String,
    pub organism_id: u64,
    pub source_genome_id: u64,
    /// Hashes include the entire genome and phenotype, not just body scalars.
    pub saved_genetics_digest: [u64; 4],
    pub local_genetics_digest: [u64; 4],
    pub exp2_input_bits: u32,
    pub saved_metabolic_turnover_bits: u32,
    pub local_metabolic_turnover_bits: u32,
}

impl WorldOrganismRecord {
    /// Proposes a proof only for one positive, finite, adjacent f32 exp2 result.
    /// Every other phenotype field must match its ordinary expression exactly.
    /// This does not admit a record or validate its private neural closure.
    pub fn propose_saved_body_rounding_compatibility(
        &self,
    ) -> Result<SavedBodyRoundingCompatibilityV1, ScaffoldContractError> {
        self.genome.validate_contract()?;
        let mut local = self.genome.express()?;
        let input = self.genome.body.metabolic_turnover_log2.expressed()?;
        let saved_value = self.phenotype.body.metabolic_turnover;
        let local_value = local.body.metabolic_turnover;
        if !saved_value.is_finite()
            || !local_value.is_finite()
            || saved_value <= 0.
            || local_value <= 0.
            || saved_value.to_bits().abs_diff(local_value.to_bits()) != 1
            || input.fract() == 0.
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        let local_genetics_digest = genetics_development_digest(&self.genome, &local)?;
        local.body.metabolic_turnover = saved_value;
        // Serialization distinguishes signed zero and includes every field.
        if serde_json::to_vec(&local).map_err(|_| ScaffoldContractError::InvalidId)?
            != serde_json::to_vec(self.phenotype.as_ref())
                .map_err(|_| ScaffoldContractError::InvalidId)?
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(SavedBodyRoundingCompatibilityV1 {
            schema: 1,
            kind: "explicit-saved-body-exp2-adjacent-f32-v1".into(),
            organism_id: self.organism_id.raw(),
            source_genome_id: self.genome.id.raw(),
            saved_genetics_digest: genetics_development_digest(&self.genome, &self.phenotype)?,
            local_genetics_digest,
            exp2_input_bits: input.to_bits(),
            saved_metabolic_turnover_bits: saved_value.to_bits(),
            local_metabolic_turnover_bits: local_value.to_bits(),
        })
    }
}

thread_local! {
    static APPROVED: RefCell<Option<(SavedBodyRoundingCompatibilityV1, usize)>> = const { RefCell::new(None) };
}

struct Scope(PhantomData<Rc<()>>);
impl Drop for Scope {
    fn drop(&mut self) {
        APPROVED.with(|slot| *slot.borrow_mut() = None);
    }
}

/// The caller must bind this exact proof to the selected immutable source and
/// record it in the admission receipt. The bool confirms it was actually used.
/// Scope cannot cross threads, nest, or survive unwinding. All other identity,
/// subsystem digest, topology and private-asset checks still execute normally.
pub fn with_saved_body_rounding_compatibility<T>(
    proof: SavedBodyRoundingCompatibilityV1,
    action: impl FnOnce() -> T,
) -> Result<(T, bool), &'static str> {
    APPROVED.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_some() {
            return Err("saved body compatibility scopes cannot nest");
        }
        *slot = Some((proof, 0));
        Ok(())
    })?;
    let _scope = Scope(PhantomData);
    let result = action();
    let used = APPROVED.with(|slot| slot.borrow().as_ref().is_some_and(|(_, n)| *n > 0));
    Ok((result, used))
}

pub(super) fn explicitly_matches(
    record: &WorldOrganismRecord,
) -> Result<bool, ScaffoldContractError> {
    let proof = APPROVED.with(|slot| slot.borrow().as_ref().map(|(p, _)| p.clone()));
    let Some(proof) = proof else {
        return Ok(false);
    };
    if record.propose_saved_body_rounding_compatibility()? != proof {
        return Ok(false);
    }
    APPROVED.with(|slot| {
        if let Some((_, count)) = slot.borrow_mut().as_mut() {
            *count += 1;
        }
    });
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{BrainCapacityClass, FoundationGeneticIdentity};

    fn fixture() -> WorldOrganismRecord {
        let mut genome = CreatureGenome::early_mammal_founder(
            0xE10_31FF,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        genome.body.metabolic_turnover_log2.maternal = -8.743856;
        genome.body.metabolic_turnover_log2.paternal = -8.543856;
        let phenotype = genome.express().unwrap();
        let mut record = WorldOrganismRecord::newborn(
            OrganismId(1),
            WorldEntityId(101),
            genome,
            phenotype,
            Tick(10_000),
        )
        .unwrap();
        let p = std::sync::Arc::make_mut(&mut record.phenotype);
        p.body.metabolic_turnover = f32::from_bits(p.body.metabolic_turnover.to_bits() + 1);
        record.state_graph.genetics_development.content_digest =
            genetics_development_digest(&record.genome, &record.phenotype).unwrap();
        record.genetic_validation = GeneticValidationCache::default();
        record
    }

    #[test]
    fn explicit_saved_rounding_preserves_bits_and_all_other_checks() {
        let record = fixture();
        let bytes = serde_json::to_vec(&record).unwrap();
        assert!(record.validate_contract().is_err());
        let proof = record.propose_saved_body_rounding_compatibility().unwrap();
        let (valid, used) = with_saved_body_rounding_compatibility(proof.clone(), || {
            let fresh: WorldOrganismRecord = serde_json::from_slice(&bytes).unwrap();
            fresh.validate_contract()
        })
        .unwrap();
        valid.unwrap();
        assert!(used);
        assert_eq!(bytes, serde_json::to_vec(&record).unwrap());
        assert!(record.validate_contract().is_err());
        let mut wrong = proof.clone();
        wrong.source_genome_id += 1;
        assert!(
            with_saved_body_rounding_compatibility(wrong, || record.validate_contract())
                .unwrap()
                .0
                .is_err()
        );
        let mut changed = record.clone();
        std::sync::Arc::make_mut(&mut changed.phenotype)
            .body
            .movement_efficiency += 0.01;
        assert!(changed.propose_saved_body_rounding_compatibility().is_err());
        let mut changed = record.clone();
        let p = std::sync::Arc::make_mut(&mut changed.phenotype);
        p.body.metabolic_turnover = f32::from_bits(p.body.metabolic_turnover.to_bits() + 1);
        assert!(changed.propose_saved_body_rounding_compatibility().is_err());
        assert!(with_saved_body_rounding_compatibility(proof.clone(), || {
            with_saved_body_rounding_compatibility(proof, || ())
        })
        .unwrap()
        .0
        .is_err());
    }
}

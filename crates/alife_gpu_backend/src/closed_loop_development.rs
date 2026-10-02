//! Derived, submission-local sensitive-period metadata. Never a learned-state owner.
use alife_core::{
    BrainPhenotype, CriticalPeriod, DevelopmentState, PlasticityGenomeParameters,
    ScaffoldContractError, Validate,
};

/// Cached route factors and inherited rate ceilings for the age at actual mutation.
/// The phenotype must retain its stable, period-free compilation. No phenotype or bank is edited.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuDevelopmentalPlasticity {
    periods: Option<Vec<CriticalPeriod>>,
    phenotype_hash: Option<alife_core::PhenotypeHash>,
    words: Vec<u32>,
}

impl Default for GpuDevelopmentalPlasticity {
    fn default() -> Self {
        Self {
            periods: None,
            phenotype_hash: None,
            words: vec![0, 0, 0, 0],
        }
    }
}

impl GpuDevelopmentalPlasticity {
    /// Refresh only when open periods or inherited ceilings change, not when age increments.
    pub fn update(
        &mut self,
        phenotype: &BrainPhenotype,
        development: &DevelopmentState,
        parameters: &PlasticityGenomeParameters,
    ) -> Result<bool, ScaffoldContractError> {
        development.validate_contract()?;
        parameters.validate_contract()?;
        let ceilings = [
            parameters.base_learning_rate().to_bits(),
            parameters.sleep_replay_rate().to_bits(),
            parameters.normalization_rate().to_bits(),
        ];
        if self.phenotype_hash == Some(phenotype.phenotype_hash())
            && self.periods.as_ref() == Some(&development.open_critical_periods)
            && self.words[1..4] == ceilings
        {
            return Ok(false);
        }
        let count = phenotype.projections().len();
        let mut words = vec![1.0_f32.to_bits(); 4 + count];
        words[0] = u32::try_from(count).map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
        words[1..4].copy_from_slice(&ceilings);
        // Route indices are canonical dense identities, unlike packed synapse spans.
        for (index, projection) in phenotype.projections().iter().enumerate() {
            if usize::from(projection.route_index()) != index {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            let bias = development
                .open_critical_periods
                .iter()
                .filter(|p| {
                    p.lobe == projection.source_lobe() || p.lobe == projection.target_lobe()
                })
                .map(|p| p.plasticity_bias.raw())
                .fold(0.0_f32, f32::max);
            words[4 + index] = (1.0 + bias).to_bits();
        }
        self.words = words;
        self.phenotype_hash = Some(phenotype.phenotype_hash());
        self.periods = Some(development.open_critical_periods.clone());
        Ok(true)
    }

    /// Exact transient GPU payload: route count, three ceilings, then route multipliers.
    pub fn words(&self) -> &[u32] {
        &self.words
    }
}

/// Assemble exactly the transient waking row used by the production batch dispatch.
pub(crate) fn append_developmental_outcome(
    payload: &mut Vec<u32>,
    outcome: &crate::GpuOutcomeCreditRecord,
    development: &GpuDevelopmentalPlasticity,
) -> Result<u32, crate::GpuClosedLoopError> {
    let offset =
        u32::try_from(payload.len()).map_err(|_| crate::GpuClosedLoopError::CapacityExceeded)?;
    payload.extend_from_slice(outcome.words());
    payload.extend_from_slice(development.words());
    Ok(offset)
}

/// Append current-age metadata after the existing replay samples, preserving the replay journal.
pub(crate) fn append_sleep_development(
    payload: &mut Vec<u32>,
    sample_offset: u32,
    sample_count: u32,
    development: &GpuDevelopmentalPlasticity,
) -> Result<(), ScaffoldContractError> {
    let end = sample_offset
        .checked_add(sample_count)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    if payload.len() != end as usize {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    payload.extend_from_slice(development.words());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    #[test]
    fn sleep_payload_uses_current_metadata_without_rewriting_old_experiences() {
        let old_experience_and_samples = vec![0x5a5a; 32];
        let current = GpuDevelopmentalPlasticity {
            periods: None,
            phenotype_hash: None,
            words: vec![
                1,
                0.2_f32.to_bits(),
                0.1_f32.to_bits(),
                0.05_f32.to_bits(),
                1.0_f32.to_bits(),
            ],
        };
        let mut payload = old_experience_and_samples.clone();
        append_sleep_development(&mut payload, 30, 2, &current).unwrap();
        assert_eq!(&payload[..32], old_experience_and_samples);
        assert_eq!(&payload[32..], current.words());
        let mut rejected = old_experience_and_samples.clone();
        assert!(append_sleep_development(&mut rejected, 30, 3, &current).is_err());
        assert_eq!(rejected, old_experience_and_samples);
    }

    #[test]
    fn waking_batch_rows_bind_distinct_modulation_after_outcome_without_overwriting_prefix() {
        let mut payload = vec![0x12345678; 7];
        let mut outcome = crate::GpuOutcomeCreditRecord::zeroed();
        outcome.sequence_id = [23, 0];
        let first = GpuDevelopmentalPlasticity {
            periods: None,
            phenotype_hash: None,
            words: vec![
                1,
                0.2_f32.to_bits(),
                0.1_f32.to_bits(),
                0.05_f32.to_bits(),
                1.5_f32.to_bits(),
            ],
        };
        let start = append_developmental_outcome(&mut payload, &outcome, &first).unwrap() as usize;
        assert_eq!(start, 7);
        assert_eq!(&payload[..7], &[0x12345678; 7]);
        assert_eq!(
            &payload[start..start + crate::GPU_OUTCOME_CREDIT_WORDS],
            outcome.words()
        );
        assert_eq!(
            &payload[start + crate::GPU_OUTCOME_CREDIT_WORDS..],
            first.words()
        );
        let prefix = payload.clone();
        let second = GpuDevelopmentalPlasticity::default();
        let second_start =
            append_developmental_outcome(&mut payload, &outcome, &second).unwrap() as usize;
        assert_eq!(second_start, prefix.len());
        assert_eq!(&payload[..second_start], prefix);
        assert_eq!(
            &payload[second_start + crate::GPU_OUTCOME_CREDIT_WORDS..],
            second.words()
        );
    }
}

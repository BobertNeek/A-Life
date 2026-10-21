//! Checked byte layout and lossless decoding for one actor checkpoint capture.

use crate::TrainingError;

#[derive(Debug, Clone, Copy)]
pub(super) struct CheckpointReadbackLayout {
    pub lane_bytes: u64,
    pub optimizer_bytes: u64,
    pub total_bytes: u64,
}

pub(super) struct CheckpointReadback {
    pub weights: Vec<f32>,
    pub first_moment: Vec<f32>,
    pub second_moment: Vec<f32>,
    pub update_ages: Vec<u32>,
}

impl CheckpointReadbackLayout {
    pub fn new(count: usize, max_buffer_size: u64) -> Result<Self, TrainingError> {
        let malformed = || TrainingError::MalformedReadback;
        let count = u64::try_from(count).map_err(|_| malformed())?;
        let lane_bytes = count.checked_mul(4).ok_or_else(malformed)?;
        let optimizer_bytes = lane_bytes.checked_mul(3).ok_or_else(malformed)?;
        let total_bytes = lane_bytes
            .checked_add(optimizer_bytes)
            .ok_or_else(malformed)?;
        // Mapping and host slicing must fit as well as the device allocation.
        if count == 0 || total_bytes > max_buffer_size || total_bytes > isize::MAX as u64 {
            return Err(malformed());
        }
        Ok(Self {
            lane_bytes,
            optimizer_bytes,
            total_bytes,
        })
    }

    pub fn validate_source_sizes(
        self,
        weights_bytes: u64,
        optimizer_bytes: u64,
    ) -> Result<(), TrainingError> {
        if weights_bytes < self.lane_bytes || optimizer_bytes < self.optimizer_bytes {
            return Err(TrainingError::MalformedReadback);
        }
        Ok(())
    }

    pub fn decode(self, bytes: &[u8]) -> Result<CheckpointReadback, TrainingError> {
        if bytes.len() as u64 != self.total_bytes {
            return Err(TrainingError::MalformedReadback);
        }
        let n = self.lane_bytes as usize;
        let (weights, optimizer) = bytes.split_at(n);
        let (first_moment, optimizer) = optimizer.split_at(n);
        let (second_moment, update_ages) = optimizer.split_at(n);
        // Ages are integer words, including patterns that are NaN as f32.
        let first_moment = decode_floats(first_moment)?;
        let second_moment = decode_floats(second_moment)?;
        let weights = decode_floats(weights)?;
        Ok(CheckpointReadback {
            weights,
            first_moment,
            second_moment,
            update_ages: words(update_ages).collect(),
        })
    }
}

fn words(bytes: &[u8]) -> impl Iterator<Item = u32> + '_ {
    // Whole-word layout without cast alignment assumptions.
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_ne_bytes(*word))
}

fn decode_floats(bytes: &[u8]) -> Result<Vec<f32>, TrainingError> {
    let values: Vec<_> = words(bytes).map(f32::from_bits).collect();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(TrainingError::MalformedReadback);
    }
    Ok(values)
}

#[cfg(test)]
#[path = "checkpoint_readback/tests.rs"]
mod tests;

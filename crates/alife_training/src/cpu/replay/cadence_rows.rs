//! Canonical cadence-only sparse index views for explicit CPU replay opt-in.
use super::route_fires;

pub(super) struct CadenceRows {
    pub(super) offsets: [Vec<usize>; 3],
    pub(super) ids: [Vec<usize>; 3],
}

fn class(step: usize) -> usize {
    if step == 0 {
        0
    } else if step.is_multiple_of(2) {
        2
    } else {
        1
    }
}

impl CadenceRows {
    pub(super) fn new(incoming: &[Vec<usize>], cadence: impl Fn(usize) -> u8) -> Self {
        let mut offsets: [Vec<usize>; 3] = Default::default();
        let mut ids: [Vec<usize>; 3] = Default::default();
        for step in 0..3 {
            offsets[step].push(0);
            for row in incoming {
                ids[step].extend(
                    row.iter()
                        .copied()
                        .filter(|&id| route_fires(cadence(id), step)),
                );
                offsets[step].push(ids[step].len());
            }
        }
        Self { offsets, ids }
    }
    pub(super) fn row(&self, target: usize, step: usize) -> &[usize] {
        let c = class(step);
        &self.ids[c][self.offsets[c][target]..self.offsets[c][target + 1]]
    }
    #[cfg(test)]
    pub(super) fn payload_bytes(&self) -> usize {
        self.offsets
            .iter()
            .chain(&self.ids)
            .map(|v| v.len() * size_of::<usize>())
            .sum()
    }
}

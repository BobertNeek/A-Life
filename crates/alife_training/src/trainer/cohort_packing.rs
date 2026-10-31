//! Test-only host experiment. No runtime integration or device state is cached.
use super::{pack_replay_sequence, PackedLayout};
use crate::PpoTrainingWindow;
use std::{
    borrow::Cow,
    mem::size_of,
    sync::{Arc, Weak},
};

const MAX_WINDOWS: usize = 8;
const MAX_CHARGED_BYTES: u64 = 256 * 1024 * 1024;

// Exactly the layout fields read by pack_replay_sequence, plus its neuron stride.
// Burn-in, memory gain, optimizer step and all other headers remain dynamic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PackingKey {
    neurons: u32,
    training_words: u64,
    inputs: u32,
    candidate_records: u32,
    candidate_capacity: u32,
    context: u32,
    initial: u32,
}
impl PackingKey {
    fn new(layout: PackedLayout, neurons: u32) -> Self {
        Self {
            neurons,
            training_words: layout.training_words,
            inputs: layout.inputs,
            candidate_records: layout.candidate_records,
            candidate_capacity: layout.candidate_capacity,
            context: layout.context,
            initial: layout.initial,
        }
    }
}

struct Packet {
    owner: Weak<PpoTrainingWindow>,
    key: PackingKey,
    words: Vec<u32>,
    charge: u64,
}

/// Lives inside one cohort call. Weak ownership avoids retaining replay vectors.
/// A Weak also prevents Arc::get_mut; Arc::make_mut dissociates it or clones the
/// data. A changed snapshot consequently cannot alias the old packet identity.
pub(crate) struct CohortPackedReplay {
    packets: Vec<Packet>,
    capacity: usize,
    metadata_charge: u64,
    charged_bytes: u64,
    disabled: bool,
    byte_limit: u64,
}
impl CohortPackedReplay {
    pub(crate) fn new(window_count: usize) -> Self {
        Self {
            packets: Vec::new(),
            capacity: window_count,
            metadata_charge: 0,
            charged_bytes: 0,
            disabled: window_count == 0 || window_count > MAX_WINDOWS,
            byte_limit: MAX_CHARGED_BYTES,
        }
    }

    fn packet_charge(words: u64) -> Option<u64> {
        // Conservative reservation, not RSS: packet payload doubled, plus the
        // fixed Arc allocation that Weak can keep alive after its vectors drop.
        words
            .checked_mul(8)?
            .checked_add((size_of::<PpoTrainingWindow>() as u64) * 2 + 256)
    }

    pub(super) fn words<'a>(
        &'a mut self,
        window: &Arc<PpoTrainingWindow>,
        layout: PackedLayout,
        neurons: u32,
        admit: &mut dyn FnMut(u64) -> bool,
    ) -> Cow<'a, [u32]> {
        // Caller retains the ordinary validate_for call before reaching here.
        if !self.disabled && self.metadata_charge == 0 {
            let metadata = (self.capacity * size_of::<Packet>()) as u64 * 2 + 4096;
            if metadata <= self.byte_limit && admit(metadata) {
                self.packets = Vec::with_capacity(self.capacity);
                self.metadata_charge = metadata;
                self.charged_bytes = metadata;
            } else {
                self.disabled = true;
            }
        }
        let key = PackingKey::new(layout, neurons);
        self.packets
            .retain(|packet| packet.owner.strong_count() != 0);
        self.charged_bytes =
            self.metadata_charge + self.packets.iter().map(|p| p.charge).sum::<u64>();
        if !self.disabled {
            if let Some(index) = self.packets.iter().position(|packet| {
                packet.owner.as_ptr() == Arc::as_ptr(window) && packet.key == key
            }) {
                if admit(0) {
                    return Cow::Borrowed(&self.packets[index].words);
                }
                // Retention is optional: release packets before taking the
                // ordinary already-admitted uncached packing path.
                self.packets.clear();
                self.charged_bytes = self.metadata_charge;
            }
        }
        let retain_charge = Self::packet_charge(layout.training_words).filter(|charge| {
            !self.disabled
                && self.packets.len() < self.capacity
                && self
                    .charged_bytes
                    .checked_add(*charge)
                    .is_some_and(|n| n <= self.byte_limit)
                && admit(*charge)
        });
        let words = pack_replay_sequence(&window.sequence, layout, neurons);
        if let Some(charge) = retain_charge.filter(|charge| {
            Self::packet_charge(words.capacity() as u64).is_some_and(|actual| actual <= *charge)
        }) {
            self.packets.push(Packet {
                owner: Arc::downgrade(window),
                key,
                words,
                charge,
            });
            self.charged_bytes += charge;
            Cow::Borrowed(&self.packets.last().expect("inserted packet").words)
        } else {
            Cow::Owned(words)
        }
    }
}

#[path = "cohort_packing_tests.rs"]
mod tests;

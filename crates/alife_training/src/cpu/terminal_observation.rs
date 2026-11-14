//! Recipe3-only terminal channel evidence. Ordinary episodic recall is unchanged.
use alife_core::*;
use serde::{Deserialize, Serialize};

pub const TERMINAL_CHANNEL_CAPACITY2135: usize = 64;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalChannelState2135 {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub last_processed_sequence: u64,
    pub observations: Vec<ChannelTerminalObservationV1>,
}
impl TerminalChannelState2135 {
    pub fn new(organism_id: OrganismId) -> Self {
        Self {
            schema_version: 1,
            organism_id,
            last_processed_sequence: 0,
            observations: Vec::new(),
        }
    }
    pub fn validate(&self, next_sequence: u64) -> Result<(), String> {
        if self.schema_version != 1
            || self.organism_id.raw() == 0
            || self.observations.len() > TERMINAL_CHANNEL_CAPACITY2135
            || self.last_processed_sequence.checked_add(1) != Some(next_sequence)
        {
            return Err("terminal channel state/sequence mismatch".into());
        }
        let mut last = 0;
        for observation in &self.observations {
            observation.validate_contract().map_err(|e| e.to_string())?;
            if observation.query().organism_id() != self.organism_id
                || observation.sequence_id().raw() <= last
                || observation.sequence_id().raw() > self.last_processed_sequence
            {
                return Err("terminal channel observation owner/order mismatch".into());
            }
            last = observation.sequence_id().raw();
        }
        Ok(())
    }
    /// Called once at the completed native boundary, inside the host transaction.
    pub fn observe(
        &mut self,
        patch: &ExperiencePatch,
        recall: &FinalizedMemoryRecall,
        receipt: &mut TerminalObservationReceipt2135,
    ) -> Result<(), String> {
        let sequence = patch.header().sequence_id.raw();
        if patch.header().organism_id != self.organism_id
            || Some(sequence) != self.last_processed_sequence.checked_add(1)
        {
            return Err("terminal channel duplicate/out-of-order delivery".into());
        }
        if receipt.admission == TerminalObservationAdmission2135::Admitted {
            // The actual committed bundle chooses the key. No replacement decision
            // or channel-specific reward is manufactured.
            let bundle = patch
                .decision()
                .selected_bundle
                .as_ref()
                .ok_or("missing committed bundle")?;
            if let Some(command) = bundle
                .channels
                .iter()
                .find(|c| c.channel == MotorChannel::Manipulation)
            {
                let mut keys = recall.candidate_keys().iter().filter(|key| {
                    let q = key.query();
                    patch
                        .pre_action()
                        .perception()
                        .candidates()
                        .get(q.candidate_index() as usize)
                        .is_some_and(|candidate| {
                            q.action_id() == command.primitive
                                && candidate.target.entity == command.target.and_then(|t| t.entity)
                        })
                });
                let key = keys.next().ok_or("committed manipulation query missing")?;
                if keys.next().is_some() {
                    return Err("ambiguous committed manipulation query".into());
                }
                if let Some(observation) = ChannelTerminalObservationV1::try_from_sealed_patch(
                    patch,
                    recall,
                    key,
                    MotorChannel::Manipulation,
                )
                .map_err(|e| e.to_string())?
                {
                    if self.observations.len() >= TERMINAL_CHANNEL_CAPACITY2135 {
                        return Err("terminal observation admission exceeded capacity".into());
                    }
                    receipt.recorded =
                        Some(TerminalChannelSource2135::from_observation(&observation));
                    self.observations.push(observation);
                }
            }
        }
        self.last_processed_sequence = sequence;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalChannelSource2135 {
    pub sequence_id: u64,
    pub channel: MotorChannel,
    pub action_id: u32,
    pub tracked_object_id: u64,
    pub observation_digest: [u64; 4],
}
impl TerminalChannelSource2135 {
    pub fn from_observation(observation: &ChannelTerminalObservationV1) -> Self {
        Self {
            sequence_id: observation.sequence_id().raw(),
            channel: observation.channel(),
            action_id: observation.query().action_id().raw(),
            tracked_object_id: observation.query().tracked_object_id().unwrap().raw(),
            observation_digest: observation.canonical_digest(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalObservationAdmission2135 {
    Admitted,
    InsufficientAtp {
        required_atp_q16: u32,
        available_atp_q16: u32,
    },
    CapacityExceeded {
        records: usize,
        limit: usize,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalObservationReceipt2135 {
    pub admission: TerminalObservationAdmission2135,
    pub requested_work_units: u64,
    pub requested_atp_q16: u32,
    pub reserved_work_units: u64,
    pub reserved_atp_q16: u32,
    pub admission_check_unfunded_atp_q16: u32,
    pub recorded: Option<TerminalChannelSource2135>,
}
pub fn admit_terminal_observation2135(
    records: usize,
    available: u32,
) -> TerminalObservationReceipt2135 {
    // Version1 logical tariff: bounded sealed frame/query/bundle validation,
    // source construction, duplicate scan, hashing and copying. Not hardware FLOPs.
    let requested_work = 32768 + records.min(TERMINAL_CHANNEL_CAPACITY2135) as u64 * 256;
    let requested_atp = (requested_work * 65535).div_ceil(1_000_000) as u32;
    let admission = if records >= TERMINAL_CHANNEL_CAPACITY2135 {
        TerminalObservationAdmission2135::CapacityExceeded {
            records,
            limit: TERMINAL_CHANNEL_CAPACITY2135,
        }
    } else if requested_atp > available {
        TerminalObservationAdmission2135::InsufficientAtp {
            required_atp_q16: requested_atp,
            available_atp_q16: available,
        }
    } else {
        TerminalObservationAdmission2135::Admitted
    };
    let work = if admission == TerminalObservationAdmission2135::Admitted {
        requested_work
    } else {
        128
    };
    let debit = (work * 65535).div_ceil(1_000_000) as u32;
    TerminalObservationReceipt2135 {
        admission,
        requested_work_units: requested_work,
        requested_atp_q16: requested_atp,
        reserved_work_units: work,
        reserved_atp_q16: debit.min(available),
        admission_check_unfunded_atp_q16: debit.saturating_sub(available),
        recorded: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_observation_budget_or_capacity_keeps_paid_bounded_checks() {
        let no_atp = admit_terminal_observation2135(0, 0);
        assert!(matches!(
            no_atp.admission,
            TerminalObservationAdmission2135::InsufficientAtp { .. }
        ));
        assert_eq!(no_atp.reserved_work_units, 128);
        assert_eq!(no_atp.reserved_atp_q16, 0);
        assert!(no_atp.admission_check_unfunded_atp_q16 > 0);
        let capacity = admit_terminal_observation2135(64, 65535);
        assert_eq!(
            capacity.admission,
            TerminalObservationAdmission2135::CapacityExceeded {
                records: 64,
                limit: 64
            }
        );
        assert_eq!(capacity.reserved_work_units, 128);
        assert!(capacity.reserved_atp_q16 > 0);
        let admitted = admit_terminal_observation2135(0, 65535);
        assert_eq!(
            admitted.admission,
            TerminalObservationAdmission2135::Admitted
        );
        assert_eq!(admitted.reserved_work_units, admitted.requested_work_units);
        assert_eq!(admitted.reserved_atp_q16, admitted.requested_atp_q16);
    }
}

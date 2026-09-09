use std::path::PathBuf;

use alife_core::{ScaffoldContractError, Tick};

pub(super) const EXACT_POPULATION_CHECKPOINT_TRANSACTION_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExactPopulationCheckpointStageV1 {
    Idle,
    CaptureSubmitted,
    MappingPending,
    CpuBytesReady,
    Encoding,
    ManifestPrepared,
    CasCommitted,
    ReloadValidated,
    DurablePermitInstalled,
    DeferredJournalPublishing,
    Complete,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExactPopulationCheckpointIdentityV1 {
    pub schema_version: u16,
    pub transaction_id: u64,
    pub checkpoint_tick: Tick,
    pub expected_base_digest: String,
}

impl ExactPopulationCheckpointIdentityV1 {
    pub(super) fn try_new(
        transaction_id: u64,
        checkpoint_tick: Tick,
        expected_base_digest: String,
    ) -> Result<Self, ScaffoldContractError> {
        if transaction_id == 0 || expected_base_digest.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(Self {
            schema_version: EXACT_POPULATION_CHECKPOINT_TRANSACTION_SCHEMA_VERSION,
            transaction_id,
            checkpoint_tick,
            expected_base_digest,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ManualCheckpointRequestV1 {
    /// Minimum requested world tick that may satisfy this manual request.
    pub checkpoint_tick: Tick,
    pub destination: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExactCheckpointRequestDispositionV1 {
    Started { transaction_id: u64 },
    CoalescedWithActive,
    FollowUpQueued,
    FollowUpAlreadyQueued,
    ManualQueued,
    ManualCoalesced,
    Busy,
}

#[derive(Debug, Clone)]
pub(super) struct ExactPopulationCheckpointCoordinatorV1 {
    next_transaction_id: u64,
    active: Option<ExactPopulationCheckpointIdentityV1>,
    stage: ExactPopulationCheckpointStageV1,
    checkpoint_needed_after_current: bool,
    pending_manual: Option<ManualCheckpointRequestV1>,
}

impl Default for ExactPopulationCheckpointCoordinatorV1 {
    fn default() -> Self {
        Self {
            next_transaction_id: 1,
            active: None,
            stage: ExactPopulationCheckpointStageV1::Idle,
            checkpoint_needed_after_current: false,
            pending_manual: None,
        }
    }
}

impl ExactPopulationCheckpointCoordinatorV1 {
    pub(super) const fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub(super) const fn stage(&self) -> ExactPopulationCheckpointStageV1 {
        self.stage
    }

    pub(super) fn active_identity(&self) -> Option<&ExactPopulationCheckpointIdentityV1> {
        self.active.as_ref()
    }

    #[cfg(feature = "gpu-tests")]
    pub(super) fn force_pre_worker_transition_failure_for_test(&mut self) {
        self.stage = ExactPopulationCheckpointStageV1::Encoding;
    }

    pub(super) const fn checkpoint_needed_after_current(&self) -> bool {
        self.checkpoint_needed_after_current
    }

    pub(super) fn request_exact(
        &mut self,
        checkpoint_tick: Tick,
        expected_base_digest: String,
    ) -> Result<ExactCheckpointRequestDispositionV1, ScaffoldContractError> {
        if self.stage == ExactPopulationCheckpointStageV1::Failed {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        if let Some(active) = &self.active {
            if active.checkpoint_tick == checkpoint_tick
                && active.expected_base_digest == expected_base_digest
            {
                return Ok(ExactCheckpointRequestDispositionV1::CoalescedWithActive);
            }
            if self.checkpoint_needed_after_current {
                return Ok(ExactCheckpointRequestDispositionV1::FollowUpAlreadyQueued);
            }
            self.checkpoint_needed_after_current = true;
            return Ok(ExactCheckpointRequestDispositionV1::FollowUpQueued);
        }
        if self.stage != ExactPopulationCheckpointStageV1::Idle {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let transaction_id = self.next_transaction_id;
        let next_transaction_id = self
            .next_transaction_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let identity = ExactPopulationCheckpointIdentityV1::try_new(
            transaction_id,
            checkpoint_tick,
            expected_base_digest,
        )?;
        self.active = Some(identity);
        self.next_transaction_id = next_transaction_id;
        self.stage = ExactPopulationCheckpointStageV1::CaptureSubmitted;
        Ok(ExactCheckpointRequestDispositionV1::Started { transaction_id })
    }

    pub(super) fn admit_durable_recommit(
        &mut self,
        checkpoint_tick: Tick,
        expected_base_digest: String,
    ) -> Result<u64, ScaffoldContractError> {
        if self.active.is_some() || self.stage != ExactPopulationCheckpointStageV1::Idle {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let transaction_id = self.next_transaction_id;
        let next_transaction_id = self
            .next_transaction_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let identity = ExactPopulationCheckpointIdentityV1::try_new(
            transaction_id,
            checkpoint_tick,
            expected_base_digest,
        )?;
        self.active = Some(identity);
        self.next_transaction_id = next_transaction_id;
        self.stage = ExactPopulationCheckpointStageV1::DurablePermitInstalled;
        Ok(transaction_id)
    }

    pub(super) fn request_manual(
        &mut self,
        request: ManualCheckpointRequestV1,
    ) -> ExactCheckpointRequestDispositionV1 {
        if self.stage == ExactPopulationCheckpointStageV1::Failed {
            return ExactCheckpointRequestDispositionV1::Busy;
        }
        let Some(active_checkpoint_tick) =
            self.active.as_ref().map(|active| active.checkpoint_tick)
        else {
            return ExactCheckpointRequestDispositionV1::Busy;
        };
        if let Some(pending) = self.pending_manual.as_mut() {
            if pending.destination != request.destination {
                return ExactCheckpointRequestDispositionV1::Busy;
            }
            pending.checkpoint_tick = pending.checkpoint_tick.max(request.checkpoint_tick);
            if active_checkpoint_tick < pending.checkpoint_tick
                || matches!(
                    self.stage,
                    ExactPopulationCheckpointStageV1::DeferredJournalPublishing
                        | ExactPopulationCheckpointStageV1::Complete
                )
            {
                self.checkpoint_needed_after_current = true;
            }
            return ExactCheckpointRequestDispositionV1::ManualCoalesced;
        }
        if active_checkpoint_tick < request.checkpoint_tick
            || matches!(
                self.stage,
                ExactPopulationCheckpointStageV1::DeferredJournalPublishing
                    | ExactPopulationCheckpointStageV1::Complete
            )
        {
            self.checkpoint_needed_after_current = true;
        }
        self.pending_manual = Some(request);
        ExactCheckpointRequestDispositionV1::ManualQueued
    }

    pub(super) fn transition(
        &mut self,
        next: ExactPopulationCheckpointStageV1,
    ) -> Result<(), ScaffoldContractError> {
        let valid = matches!(
            (self.stage, next),
            (
                ExactPopulationCheckpointStageV1::CaptureSubmitted,
                ExactPopulationCheckpointStageV1::MappingPending
            ) | (
                ExactPopulationCheckpointStageV1::MappingPending,
                ExactPopulationCheckpointStageV1::CpuBytesReady
            ) | (
                ExactPopulationCheckpointStageV1::CpuBytesReady,
                ExactPopulationCheckpointStageV1::Encoding
            ) | (
                ExactPopulationCheckpointStageV1::Encoding,
                ExactPopulationCheckpointStageV1::ManifestPrepared
            ) | (
                ExactPopulationCheckpointStageV1::ManifestPrepared,
                ExactPopulationCheckpointStageV1::CasCommitted
            ) | (
                ExactPopulationCheckpointStageV1::CasCommitted,
                ExactPopulationCheckpointStageV1::ReloadValidated
            ) | (
                ExactPopulationCheckpointStageV1::ReloadValidated,
                ExactPopulationCheckpointStageV1::DurablePermitInstalled
            ) | (
                ExactPopulationCheckpointStageV1::DurablePermitInstalled,
                ExactPopulationCheckpointStageV1::DeferredJournalPublishing
            ) | (
                ExactPopulationCheckpointStageV1::DurablePermitInstalled,
                ExactPopulationCheckpointStageV1::Complete
            ) | (
                ExactPopulationCheckpointStageV1::DeferredJournalPublishing,
                ExactPopulationCheckpointStageV1::Complete
            )
        );
        if !valid {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        self.stage = next;
        Ok(())
    }

    pub(super) fn fail_stop(&mut self) {
        self.stage = ExactPopulationCheckpointStageV1::Failed;
    }

    pub(super) fn take_pending_manual_after_durable_permit(
        &mut self,
    ) -> Result<Option<ManualCheckpointRequestV1>, ScaffoldContractError> {
        if self.stage != ExactPopulationCheckpointStageV1::DurablePermitInstalled {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let active_checkpoint_tick = self
            .active
            .as_ref()
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?
            .checkpoint_tick;
        if self
            .pending_manual
            .as_ref()
            .is_some_and(|request| request.checkpoint_tick > active_checkpoint_tick)
        {
            return Ok(None);
        }
        Ok(self.pending_manual.take())
    }

    pub(super) fn finish(&mut self) -> Result<bool, ScaffoldContractError> {
        if self.stage != ExactPopulationCheckpointStageV1::Complete
            || self.active.is_none()
            || (self.pending_manual.is_some() && !self.checkpoint_needed_after_current)
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        self.active = None;
        self.stage = ExactPopulationCheckpointStageV1::Idle;
        let follow_up = std::mem::take(&mut self.checkpoint_needed_after_current);
        Ok(follow_up)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advance_to_durable_permit(coordinator: &mut ExactPopulationCheckpointCoordinatorV1) {
        for stage in [
            ExactPopulationCheckpointStageV1::MappingPending,
            ExactPopulationCheckpointStageV1::CpuBytesReady,
            ExactPopulationCheckpointStageV1::Encoding,
            ExactPopulationCheckpointStageV1::ManifestPrepared,
            ExactPopulationCheckpointStageV1::CasCommitted,
            ExactPopulationCheckpointStageV1::ReloadValidated,
            ExactPopulationCheckpointStageV1::DurablePermitInstalled,
        ] {
            coordinator.transition(stage).unwrap();
        }
    }

    #[test]
    fn transaction_order_is_strict_and_follow_up_is_one_bit() {
        let mut coordinator = ExactPopulationCheckpointCoordinatorV1::default();
        assert_eq!(
            coordinator
                .request_exact(Tick::new(10), "base-a".to_string())
                .unwrap(),
            ExactCheckpointRequestDispositionV1::Started { transaction_id: 1 }
        );
        assert_eq!(
            coordinator
                .request_exact(Tick::new(10), "base-a".to_string())
                .unwrap(),
            ExactCheckpointRequestDispositionV1::CoalescedWithActive
        );
        assert_eq!(
            coordinator
                .request_exact(Tick::new(11), "base-a".to_string())
                .unwrap(),
            ExactCheckpointRequestDispositionV1::FollowUpQueued
        );
        assert_eq!(
            coordinator
                .request_exact(Tick::new(12), "base-a".to_string())
                .unwrap(),
            ExactCheckpointRequestDispositionV1::FollowUpAlreadyQueued
        );
        assert!(coordinator.checkpoint_needed_after_current());
        assert!(coordinator
            .transition(ExactPopulationCheckpointStageV1::CasCommitted)
            .is_err());
        for stage in [
            ExactPopulationCheckpointStageV1::MappingPending,
            ExactPopulationCheckpointStageV1::CpuBytesReady,
            ExactPopulationCheckpointStageV1::Encoding,
            ExactPopulationCheckpointStageV1::ManifestPrepared,
            ExactPopulationCheckpointStageV1::CasCommitted,
            ExactPopulationCheckpointStageV1::ReloadValidated,
            ExactPopulationCheckpointStageV1::DurablePermitInstalled,
            ExactPopulationCheckpointStageV1::Complete,
        ] {
            coordinator.transition(stage).unwrap();
        }
        assert!(coordinator.finish().unwrap());
        assert_eq!(coordinator.stage(), ExactPopulationCheckpointStageV1::Idle);
    }

    #[test]
    fn stale_manual_request_survives_old_capture_and_fresh_follow_up() {
        let mut coordinator = ExactPopulationCheckpointCoordinatorV1::default();
        coordinator
            .request_exact(Tick::new(20), "base-b".to_string())
            .unwrap();
        let request = ManualCheckpointRequestV1 {
            checkpoint_tick: Tick::new(21),
            destination: PathBuf::from("manual-a.json"),
        };
        assert_eq!(
            coordinator.request_manual(request.clone()),
            ExactCheckpointRequestDispositionV1::ManualQueued
        );
        assert!(coordinator.checkpoint_needed_after_current());
        advance_to_durable_permit(&mut coordinator);
        assert_eq!(
            coordinator
                .take_pending_manual_after_durable_permit()
                .unwrap(),
            None
        );
        coordinator
            .transition(ExactPopulationCheckpointStageV1::Complete)
            .unwrap();
        assert!(coordinator.finish().unwrap());
        assert_eq!(coordinator.pending_manual.as_ref(), Some(&request));
        assert_eq!(coordinator.stage(), ExactPopulationCheckpointStageV1::Idle);
        assert_eq!(
            coordinator
                .request_exact(Tick::new(21), "base-c".to_string())
                .unwrap(),
            ExactCheckpointRequestDispositionV1::Started { transaction_id: 2 }
        );
        advance_to_durable_permit(&mut coordinator);
        assert_eq!(
            coordinator
                .take_pending_manual_after_durable_permit()
                .unwrap(),
            Some(request)
        );
        coordinator
            .transition(ExactPopulationCheckpointStageV1::Complete)
            .unwrap();
        assert!(!coordinator.finish().unwrap());
    }

    #[test]
    fn equal_tick_manual_request_attaches_to_active_capture() {
        let mut coordinator = ExactPopulationCheckpointCoordinatorV1::default();
        coordinator
            .request_exact(Tick::new(20), "base-equal".to_string())
            .unwrap();
        let request = ManualCheckpointRequestV1 {
            checkpoint_tick: Tick::new(20),
            destination: PathBuf::from("manual-equal.json"),
        };
        assert_eq!(
            coordinator.request_manual(request.clone()),
            ExactCheckpointRequestDispositionV1::ManualQueued
        );
        assert!(!coordinator.checkpoint_needed_after_current());
        advance_to_durable_permit(&mut coordinator);
        assert_eq!(
            coordinator
                .take_pending_manual_after_durable_permit()
                .unwrap(),
            Some(request)
        );
        coordinator
            .transition(ExactPopulationCheckpointStageV1::Complete)
            .unwrap();
        assert!(!coordinator.finish().unwrap());
    }

    #[test]
    fn same_destination_later_request_coalesces_to_maximum_tick() {
        let mut coordinator = ExactPopulationCheckpointCoordinatorV1::default();
        coordinator
            .request_exact(Tick::new(20), "base-max".to_string())
            .unwrap();
        let destination = PathBuf::from("manual-max.json");
        assert_eq!(
            coordinator.request_manual(ManualCheckpointRequestV1 {
                checkpoint_tick: Tick::new(21),
                destination: destination.clone(),
            }),
            ExactCheckpointRequestDispositionV1::ManualQueued
        );
        assert_eq!(
            coordinator.request_manual(ManualCheckpointRequestV1 {
                checkpoint_tick: Tick::new(22),
                destination: destination.clone(),
            }),
            ExactCheckpointRequestDispositionV1::ManualCoalesced
        );
        assert_eq!(
            coordinator.pending_manual.as_ref(),
            Some(&ManualCheckpointRequestV1 {
                checkpoint_tick: Tick::new(22),
                destination,
            })
        );
        assert!(coordinator.checkpoint_needed_after_current());
    }

    #[test]
    fn different_destination_is_busy_and_preserves_prior_manual_request() {
        let mut coordinator = ExactPopulationCheckpointCoordinatorV1::default();
        coordinator
            .request_exact(Tick::new(20), "base-destination".to_string())
            .unwrap();
        let prior = ManualCheckpointRequestV1 {
            checkpoint_tick: Tick::new(21),
            destination: PathBuf::from("manual-prior.json"),
        };
        assert_eq!(
            coordinator.request_manual(prior.clone()),
            ExactCheckpointRequestDispositionV1::ManualQueued
        );
        assert_eq!(
            coordinator.request_manual(ManualCheckpointRequestV1 {
                checkpoint_tick: Tick::new(22),
                destination: PathBuf::from("manual-other.json"),
            }),
            ExactCheckpointRequestDispositionV1::Busy
        );
        assert_eq!(coordinator.pending_manual.as_ref(), Some(&prior));
        assert!(coordinator.checkpoint_needed_after_current());
    }

    #[test]
    fn invalid_admission_and_terminal_failure_cannot_poison_or_retry() {
        let mut coordinator = ExactPopulationCheckpointCoordinatorV1::default();
        assert!(coordinator
            .request_exact(Tick::new(30), String::new())
            .is_err());
        assert!(!coordinator.is_active());
        assert_eq!(coordinator.stage(), ExactPopulationCheckpointStageV1::Idle);

        coordinator
            .request_exact(Tick::new(30), "base-c".to_string())
            .unwrap();
        let queued_manual = ManualCheckpointRequestV1 {
            checkpoint_tick: Tick::new(31),
            destination: PathBuf::from("manual-terminal.json"),
        };
        assert_eq!(
            coordinator.request_manual(queued_manual.clone()),
            ExactCheckpointRequestDispositionV1::ManualQueued
        );
        coordinator.fail_stop();
        assert!(coordinator.is_active());
        assert_eq!(
            coordinator.stage(),
            ExactPopulationCheckpointStageV1::Failed
        );
        let snapshot = format!("{coordinator:?}");
        assert!(matches!(
            coordinator.request_exact(Tick::new(30), "base-c".to_string()),
            Err(ScaffoldContractError::ConsolidationGenerationMismatch)
        ));
        assert_eq!(format!("{coordinator:?}"), snapshot);
        assert!(matches!(
            coordinator.request_exact(Tick::new(31), "base-c".to_string()),
            Err(ScaffoldContractError::ConsolidationGenerationMismatch)
        ));
        assert_eq!(format!("{coordinator:?}"), snapshot);
        assert_eq!(
            coordinator.request_manual(ManualCheckpointRequestV1 {
                checkpoint_tick: Tick::new(99),
                destination: queued_manual.destination.clone(),
            }),
            ExactCheckpointRequestDispositionV1::Busy
        );
        assert_eq!(format!("{coordinator:?}"), snapshot);
    }
}

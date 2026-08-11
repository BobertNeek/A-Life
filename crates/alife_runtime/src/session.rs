use std::ops::{Deref, DerefMut};

use alife_core::{PhenotypeGrowthMigration, ScaffoldContractError, Tick};
use alife_gpu_backend::{
    GpuBrainCheckpointSnapshot, GpuBrainHandle, GpuClosedLoopBackend,
    GpuCuratedResidencyCohort, GpuCuratedResidencyOutcome, GpuResearchGrowthEquivalenceReceipt,
    GpuResearchGrowthHandoffOutcome,
};

/// Identifies the production consumer using the shared neural session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuSessionConsumerKind {
    Gameplay,
    Training,
    Evolution,
    Challenge,
}

/// Stable reason that neural authority was revoked for the rest of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuSessionFailStopCause {
    DeviceLost,
    BackendUnavailable,
    CheckpointRestoreFailed,
}

/// Reference to the last durable exact-resume point that survived publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableGpuCheckpointRef {
    pub checkpoint_tick: Tick,
    pub manifest_digest: String,
    pub neural_state_digest: [u64; 4],
}

impl DurableGpuCheckpointRef {
    pub fn try_new(
        checkpoint_tick: Tick,
        manifest_digest: String,
        neural_state_digest: [u64; 4],
    ) -> Result<Self, ScaffoldContractError> {
        let valid_manifest_digest =
            manifest_digest
                .strip_prefix("fnv1a64:")
                .is_some_and(|suffix| {
                    suffix.len() == 16 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
                });
        if !valid_manifest_digest || neural_state_digest == [0; 4] {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(Self {
            checkpoint_tick,
            manifest_digest,
            neural_state_digest,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuSessionAuthorityState {
    Ready,
    FailedStop { cause: GpuSessionFailStopCause },
}

/// Small durable authority ledger kept outside GPU-resident mutable state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuSessionAuthority {
    consumer: GpuSessionConsumerKind,
    state: GpuSessionAuthorityState,
    latest_durable_checkpoint: Option<DurableGpuCheckpointRef>,
}

impl GpuSessionAuthority {
    pub const fn new(consumer: GpuSessionConsumerKind) -> Self {
        Self {
            consumer,
            state: GpuSessionAuthorityState::Ready,
            latest_durable_checkpoint: None,
        }
    }

    pub const fn consumer(&self) -> GpuSessionConsumerKind {
        self.consumer
    }

    pub const fn state(&self) -> &GpuSessionAuthorityState {
        &self.state
    }

    pub fn latest_durable_checkpoint(&self) -> Option<&DurableGpuCheckpointRef> {
        self.latest_durable_checkpoint.as_ref()
    }

    pub fn note_durable_checkpoint(
        &mut self,
        checkpoint: DurableGpuCheckpointRef,
    ) -> Result<(), ScaffoldContractError> {
        if self
            .latest_durable_checkpoint
            .as_ref()
            .is_some_and(|current| checkpoint.checkpoint_tick.raw() < current.checkpoint_tick.raw())
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        self.latest_durable_checkpoint = Some(checkpoint);
        Ok(())
    }

    pub fn fail_stop(&mut self, cause: GpuSessionFailStopCause) {
        self.state = GpuSessionAuthorityState::FailedStop { cause };
    }

    pub fn ensure_neural_actions_available(&self) -> Result<(), ScaffoldContractError> {
        match self.state {
            GpuSessionAuthorityState::Ready => Ok(()),
            GpuSessionAuthorityState::FailedStop { .. } => {
                Err(ScaffoldContractError::NeuralBackendUnavailable)
            }
        }
    }
}

/// Sole owner of one production GPU neural backend and its fail-stop ledger.
///
/// Game, training, evolution, and challenge worlds all drive this same type.
/// Bulk checkpoint codecs operate only at sealed boundaries and report the
/// resulting durable reference back through this session.
pub struct GpuAuthoritativeSession {
    backend: GpuClosedLoopBackend,
    authority: GpuSessionAuthority,
}

impl GpuAuthoritativeSession {
    pub const fn new(backend: GpuClosedLoopBackend, consumer: GpuSessionConsumerKind) -> Self {
        Self {
            backend,
            authority: GpuSessionAuthority::new(consumer),
        }
    }

    pub const fn authority(&self) -> &GpuSessionAuthority {
        &self.authority
    }

    pub const fn backend(&self) -> &GpuClosedLoopBackend {
        &self.backend
    }

    pub fn ensure_neural_actions_available(&self) -> Result<(), ScaffoldContractError> {
        self.authority.ensure_neural_actions_available()
    }

    pub fn note_durable_checkpoint(
        &mut self,
        checkpoint: DurableGpuCheckpointRef,
    ) -> Result<(), ScaffoldContractError> {
        self.authority.note_durable_checkpoint(checkpoint)
    }

    pub fn fail_stop(&mut self, cause: GpuSessionFailStopCause) {
        self.authority.fail_stop(cause);
    }

    pub fn record_contract_failure(&mut self, error: &ScaffoldContractError) {
        if *error == ScaffoldContractError::NeuralBackendUnavailable {
            self.fail_stop(GpuSessionFailStopCause::BackendUnavailable);
        }
    }

    /// Commits an already verified sealed growth handoff. Cognitive sidecars
    /// remain owned by the caller and are therefore preserved unchanged while
    /// only the opaque GPU handle is replaced.
    pub fn commit_research_growth(
        &mut self,
        source_handle: GpuBrainHandle,
        migration: &PhenotypeGrowthMigration,
        rollback: GpuBrainCheckpointSnapshot,
        target: GpuBrainCheckpointSnapshot,
        equivalence: &GpuResearchGrowthEquivalenceReceipt,
    ) -> Result<GpuResearchGrowthHandoffOutcome, ScaffoldContractError> {
        self.ensure_neural_actions_available()?;
        let result = self.backend.replace_brain_with_research_growth(
            source_handle,
            migration,
            rollback,
            target,
            equivalence,
        );
        if let Err(error) = &result {
            self.record_contract_failure(error);
        }
        result
    }

    pub fn replace_curated_cohort(
        &mut self,
        cohort: &GpuCuratedResidencyCohort,
    ) -> GpuCuratedResidencyOutcome {
        if let Err(error) = self.ensure_neural_actions_available() {
            return GpuCuratedResidencyOutcome::PreSubmitFailure {
                error,
                retryable: true,
            };
        }
        let result = self.backend.replace_curated_cohort(cohort);
        if matches!(result, GpuCuratedResidencyOutcome::Unknown { .. }) {
            self.fail_stop(GpuSessionFailStopCause::DeviceLost);
        }
        result
    }
}

impl Deref for GpuAuthoritativeSession {
    type Target = GpuClosedLoopBackend;

    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}

impl DerefMut for GpuAuthoritativeSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.backend
    }
}

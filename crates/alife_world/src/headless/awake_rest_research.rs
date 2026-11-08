//! Explicit awake-rest physiology experiment, outside neural action enumeration.
use super::*;
use alife_core::AwakeRestEventV1;

/// A sealed one-interval request. NO_POSTURE alone retains its ordinary no-op
/// meaning; this separate typed request authorizes the candidate body event.
/// No request/configuration is installed in the world or silently lost on save.
#[derive(Debug, Clone)]
pub struct AwakeRestRequestV1 {
    bundle: MotorCommandBundle,
}

impl AwakeRestRequestV1 {
    pub fn new(bundle: MotorCommandBundle) -> Result<Self, ScaffoldContractError> {
        bundle.validate_contract()?;
        let posture = bundle
            .channels
            .iter()
            .find(|row| row.channel == MotorChannel::Posture)
            .ok_or(ScaffoldContractError::InvalidActionDecision)?;
        if posture.primitive != HeadlessActionIds::NO_POSTURE
            || posture.target.is_some()
            || posture.direction != Vec3f::ZERO
            || !posture.payload.values.is_empty()
            || bundle.channels.iter().any(|row| {
                !matches!(
                    row.channel,
                    MotorChannel::Posture | MotorChannel::Locomotion
                )
            })
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        Ok(Self { bundle })
    }

    pub fn bundle(&self) -> &MotorCommandBundle {
        &self.bundle
    }
}

/// The dose is separate from the unchanged legacy motor transaction/wire ABI.
#[derive(Debug, Clone, PartialEq)]
pub struct AwakeRestReceiptV1 {
    pub motor: HeadlessMotorTransactionReceipt,
    pub recovery: AwakeRestEventV1,
}

impl HeadlessWorld {
    /// Atomic candidate operation on an explicitly inherited V1 circuit. The
    /// ordinary Rest/Idle/NO_POSTURE paths never construct this event. No sleep
    /// phase, memory, reward, concentrations or retained phenotype are patched.
    pub fn apply_awake_rest_research_v1(
        &mut self,
        request: &AwakeRestRequestV1,
        world_entity_id: WorldEntityId,
    ) -> Result<AwakeRestReceiptV1, HeadlessMotorTransactionError> {
        let bundle = request.bundle();
        self.validate_registered_motor_bundle(bundle, world_entity_id)?;
        let record = self
            .organism_registry
            .get(bundle.organism_id)
            .ok_or(ScaffoldContractError::InvalidId)?;
        if !record
            .phenotype()
            .chemistry
            .biochemical
            .supports_awake_rest_recovery_v1()
            || record.sleep_phase() != alife_core::SleepPhase::Awake
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        let before = self.clone();
        let mut recovery = AwakeRestEventV1::NONE;
        match self.apply_registered_motor_bundle_inner(
            bundle,
            world_entity_id,
            None,
            None,
            None,
            Some(&mut recovery),
        ) {
            Ok(motor) => Ok(AwakeRestReceiptV1 { motor, recovery }),
            Err(error) => {
                *self = before;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests;

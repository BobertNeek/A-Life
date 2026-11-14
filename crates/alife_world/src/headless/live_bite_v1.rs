//! Opt-in, world-only live ingestion experiment. No policy or persistent rule.
//!
//! V1 consumes living reserves, not structural tissue or a carcass. Both
//! organisms must submit ordinary registered bundles for the same interval.
//! Their entire mutation commits together; neither biological clock is stepped
//! behind a neural host. Hosts need explicit paired orchestration before use.

use super::*;

/// One bite transfers at most this fraction of the donor's mean organ reserve.
/// In raw organ units the loss is six times this amount. Nutrition is bounded
/// to the mean debit, so even maximum-efficiency digestion (2.35 total organ
/// units per nutrition unit) cannot create more reserve than was removed.
const MAX_RESERVE_DEBIT: f32 = 0.04;
const BITE_EFFORT: f32 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveBiteTransferV1 {
    pub actor: OrganismId,
    pub recipient: OrganismId,
    pub target: WorldEntityId,
    /// Actual removed mean reserve, before either interval's ordinary upkeep.
    pub reserve_debit: f32,
    pub nutrition: f32,
    /// The ordinary damage input; inherited resistance determines injury.
    pub damage: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveBitePairReceiptV1 {
    pub schema_version: u16,
    pub actor: HeadlessMotorTransactionReceipt,
    pub recipient: HeadlessMotorTransactionReceipt,
    /// Whole-pair preimage. The recipient motor receipt begins after the
    /// physical reserve debit, then applies injury and ordinary upkeep once.
    pub recipient_biology_before: BiochemistryState,
    pub transfer: Option<LiveBiteTransferV1>,
}

#[derive(Debug, Clone)]
pub(super) struct LiveBitePairContextV1 {
    actor: OrganismId,
    recipient: OrganismId,
    target: WorldEntityId,
    incoming: Option<BodyEventDelta>,
    transfer: Option<LiveBiteTransferV1>,
}

impl HeadlessWorld {
    /// Execute two already-selected ordinary bundles under the V1 live-reserve
    /// ingestion law. This entry is the opt-in: normal Eat remains Food-only.
    /// The actor's selected Manipulation must be Eat on this exact recipient.
    /// Neither bundle is synthesized, changed, or given policy credit here.
    ///
    /// V1 serializes physical actions actor-first, then recipient, at one world
    /// tick. Thus a successful contact precedes the recipient's submitted move.
    /// Both registered biology clocks advance exactly once to world_tick+1.
    /// The caller still owns the ordinary world boundary. Do not invoke this
    /// behind a canonical neural host: it must own both submitted decisions and
    /// the corresponding learning/activity commits. No sealed patch is made.
    ///
    /// No configuration/pending effect escapes this atomic call. Cold saves
    /// contain the resulting ordinary bodies, chemistry and clocks, and remain
    /// readable by existing readers; there is no silently lost enabled rule.
    pub fn apply_registered_live_bite_pair_v1(
        &mut self,
        actor: &MotorCommandBundle,
        actor_entity: WorldEntityId,
        recipient: &MotorCommandBundle,
        recipient_entity: WorldEntityId,
    ) -> Result<LiveBitePairReceiptV1, HeadlessMotorTransactionError> {
        if self.live_bite_pair_v1.is_some() || actor.organism_id == recipient.organism_id {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        self.validate_registered_motor_bundle(actor, actor_entity)?;
        self.validate_registered_motor_bundle(recipient, recipient_entity)?;
        let manipulation = actor
            .channels
            .iter()
            .find(|channel| channel.channel == MotorChannel::Manipulation)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if manipulation.primitive != HeadlessActionIds::EAT
            || manipulation.target.and_then(|target| target.entity) != Some(recipient_entity)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        let recipient_biology_before = *self
            .organism_registry
            .get(recipient.organism_id)
            .ok_or(ScaffoldContractError::InvalidId)?
            .biochemistry();
        let mut staged = self.clone();
        staged.live_bite_pair_v1 = Some(LiveBitePairContextV1 {
            actor: actor.organism_id,
            recipient: recipient.organism_id,
            target: recipient_entity,
            incoming: None,
            transfer: None,
        });
        let actor = staged.apply_registered_motor_bundle(actor, actor_entity)?;
        let recipient = staged.apply_registered_motor_bundle(recipient, recipient_entity)?;
        let context = staged
            .live_bite_pair_v1
            .take()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if context.incoming.is_some() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        #[cfg(test)]
        if staged.injected_live_bite_late_failure {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        let receipt = LiveBitePairReceiptV1 {
            schema_version: 1,
            actor,
            recipient,
            recipient_biology_before,
            transfer: context.transfer,
        };
        *self = staged;
        Ok(receipt)
    }

    pub(super) fn take_live_bite_recipient_event_v1(
        &mut self,
        organism: OrganismId,
    ) -> Option<BodyEventDelta> {
        self.live_bite_pair_v1
            .as_mut()
            .filter(|context| context.recipient == organism)
            .and_then(|context| context.incoming.take())
    }

    /// Called only after Eat's normal reach/solid-contact check. None preserves
    /// the original execution path byte for byte outside a paired operation.
    pub(super) fn execute_live_bite_v1(
        &mut self,
        command: ActionCommand,
        target: WorldEntityId,
    ) -> Result<Option<HeadlessActionResult>, ScaffoldContractError> {
        let Some(context) = self.live_bite_pair_v1.as_ref() else {
            return Ok(None);
        };
        if context.actor != command.organism_id || context.target != target {
            return Ok(None);
        }
        let recipient = context.recipient;
        if context.transfer.is_some() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let record = self
            .organism_registry
            .get(recipient)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let object = self
            .objects
            .get(&target.raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        if !record.lifecycle().is_alive()
            || record.biochemistry().tick != self.tick
            || record.world_entity_id() != target
            || object.kind != WorldObjectKind::Agent
            || object.organism_id != Some(recipient)
            || object.consumed
            || record.biochemistry().body.health <= 0.0
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        let requested = MAX_RESERVE_DEBIT * command.intensity.raw();
        let debit = requested.min(record.biochemistry().body.energy);
        if debit <= 0.0 {
            return Ok(Some(self.finish_action(
                command,
                false,
                Some(ReferenceActionFailure::MissingAffordance),
                physical(PhysicalContactKind::Touch, Some(target), Vec3f::ZERO, 0.04)?,
                OutcomeProfile::missing_affordance(),
                vec![target],
            )?));
        }
        let applied = self
            .organism_registry
            .with_biology_mut(recipient, |biology| {
                Ok(biology.body.debit_energy_pro_rata(debit)?)
            })
            .map_err(map_organism_registry_error)?;
        let transfer = LiveBiteTransferV1 {
            actor: command.organism_id,
            recipient,
            target,
            reserve_debit: applied,
            nutrition: applied,
            damage: applied,
        };
        let context = self.live_bite_pair_v1.as_mut().unwrap();
        context.incoming = Some(BodyEventDelta {
            damage: transfer.damage,
            ..BodyEventDelta::zero()
        });
        context.transfer = Some(transfer);
        let taste = self.objects[&target.raw()].grounded_physical.chemical[2];
        self.ingestion_observations.insert(
            command.organism_id.raw(),
            IngestionObservation {
                ingested_at: self.tick,
                taste,
            },
        );
        // Material enters through nutrition only. No reward, positive energy
        // shortcut, player signal, or whole-body Consumed marker is invented.
        let profile = OutcomeProfile::new(
            DriveDelta::zero(),
            EndocrineDelta::zero(),
            0.0,
            0.0,
            -BITE_EFFORT,
            0.0,
            false,
        )
        .with_body_event(BodyEventDelta {
            nutrition: transfer.nutrition,
            ..BodyEventDelta::zero()
        });
        Ok(Some(self.finish_action(
            command,
            true,
            None,
            physical(
                PhysicalContactKind::Touch,
                Some(target),
                Vec3f::ZERO,
                BITE_EFFORT,
            )?,
            profile,
            vec![target],
        )?))
    }
}

#[cfg(test)]
mod tests;

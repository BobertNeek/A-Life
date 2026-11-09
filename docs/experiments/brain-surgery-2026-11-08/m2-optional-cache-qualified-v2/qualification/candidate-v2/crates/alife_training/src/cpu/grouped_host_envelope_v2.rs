//! Fresh grouped M2-only source-bound CPU envelope. The fixed prefix is inspected before
//! any source/world/enum decoding. No old research import or ordinary export.
use super::super::{
    GroupedMotorReceiptV1, GroupedMotorStateV1, MediationModeV1, RecoveryGroupJointActionV2,
    RECOVERY_GROUP_CONFIDENCE_RECIPE_SHA256,
};
use super::*;
use serde::{Deserialize, Serialize};
#[path = "optional_vocal_host_v2.rs"]
mod optional_vocal;
pub use optional_vocal::{
    CpuOptionalVocalM2HostCheckpointV1, CpuOptionalVocalM2HostDecisionV1,
    CpuOptionalVocalM2HostStepV1, CpuOptionalVocalM2HostV1,
};
pub(super) use optional_vocal::{
    GroupedPolicyStrategy, LegacyGroupedPolicy, OptionalVocalResidentV1,
};

const MAGIC: &[u8] =
    b"ALIFE-GROUPED-M2-HOST\0V1\0CONFIDENCE-V2\0SYNAPTIC-CLOSED\0PROFILE5\0SAMPLER-BITS1\0";
const LINEAGE: &str = "fresh-grouped-m2-host-v1/confidence-v2/class5/no-historical-import";
const POLICY: &str = "synaptic-learning-closed;source-once;typed-grouped-sampling;profile5;canonical-memory-predictor-body;no-structural-sleep;one-cognitive-debit;source-logit-regional-arousal;memory-sleep-output-generation-is-cycle-id";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VoluntarySleepReceiptV1 {
    sequence: ExperienceSequenceId,
    frame: PerceptionFrameDigest,
    bundle_digest: [u64; 4],
    outcome_tick: Tick,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GroupedResidentV1 {
    pub(super) motor: GroupedMotorStateV1,
    pub(super) memory_proofs: Vec<SleepMemorySourceProofV1>,
    pub(super) last_physical: Option<alife_world::SleepChoicePhysicalReceiptV1>,
    consumed_voluntary: Option<VoluntarySleepReceiptV1>,
    pub(super) last_action: Option<RecoveryGroupJointActionV2>,
    // Absent legacy field must serialize to no bytes, preserving old envelopes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) optional_vocal: Option<OptionalVocalResidentV1>,
    last_bundle: Option<MotorCommandBundle>,
    last_joint: Option<JointActionSelectionV1>,
    last_representative: Option<u16>,
}
impl GroupedResidentV1 {
    pub(super) fn validate_source_boundary(
        &self,
        source: &CpuOrganismCheckpoint,
        activity: &CpuActivityState,
    ) -> Result<(), CpuHostError> {
        match (&self.motor.last_receipt, &self.last_physical) {
            (None, None)
                if source.dispatch_generation == 0
                    && source.last_decision_tick.is_none()
                    && source.last_outcome_tick.is_none()
                    && activity.source_frame_digest == [0; 4] =>
            {
                Ok(())
            }
            (Some(motor), Some(physical)) => {
                if motor.native.core.final_generation != source.dispatch_generation {
                    return Err(CpuHostError::State("grouped/source generation mismatch"));
                }
                if source.last_decision_tick != Some(motor.tick.raw())
                    || source.last_outcome_tick != Some(physical.outcome_tick.raw())
                    || activity.source_frame_digest != motor.frame_digest.0
                    || !activity
                        .last_world_tick
                        .is_some_and(|tick| tick >= motor.tick.raw())
                {
                    return Err(CpuHostError::State(
                        "grouped/source frame or clock mismatch",
                    ));
                }
                Ok(())
            }
            _ => Err(CpuHostError::State("grouped/source generation mismatch")),
        }
    }
    pub(super) fn validate_replay(&self, events: &[CpuReplayEvent]) -> Result<(), CpuHostError> {
        if events.len() != self.memory_proofs.len() {
            return Err(CpuHostError::State("grouped M2 replay provenance omitted"));
        }
        for (event, proof) in events.iter().zip(&self.memory_proofs) {
            proof.validate_against(&event.event, &event.target)?;
        }
        if let Some(p) = &self.last_physical {
            if !p.awake_recovery.is_finite()
                || !p.sleep_recovery.is_finite()
                || !(0. ..=1.).contains(&p.awake_recovery)
                || !(0. ..=1.).contains(&p.sleep_recovery)
                || p.source_tick >= p.outcome_tick
                || (p.successful_stationary_body_sleep
                    && (p.operation != Some(RecoveryOperationV1::BodySleep)
                        || p.sleep_recovery <= 0.
                        || !p.body_sleeping))
            {
                return Err(CpuHostError::State("invalid grouped M2 physical receipt"));
            }
        }
        Ok(())
    }
    pub(super) fn validate_boundary(
        &self,
        owner: OrganismId,
        next_sequence: u64,
        world_tick: Tick,
    ) -> Result<(), CpuHostError> {
        match (&self.last_physical, &self.motor.last_receipt) {
            (None, None)
                if next_sequence == 1
                    && self.consumed_voluntary.is_none()
                    && self.last_action.is_none()
                    && self.last_bundle.is_none()
                    && self.last_joint.is_none()
                    && self.last_representative.is_none()
                    && self.optional_vocal.as_ref().is_none_or(|v| v.is_fresh()) =>
            {
                Ok(())
            }
            (Some(physical), Some(motor)) => {
                let (representative, slots, selected_operation) = self.physical_selection()?;
                let bundle = self
                    .last_bundle
                    .as_ref()
                    .ok_or(CpuHostError::State("missing grouped physical bundle"))?;
                let joint = self
                    .last_joint
                    .as_ref()
                    .ok_or(CpuHostError::State("missing grouped physical selection"))?;
                joint.validate_bundle(bundle)?;
                if self.last_representative != Some(representative)
                    || joint.candidate_slots() != slots.map(|i| i.map_or(0, |i| i + 1))
                    || bundle.organism_id != owner
                    || bundle.sequence_id != physical.sequence_id
                    || bundle.tick != physical.source_tick
                {
                    return Err(CpuHostError::State(
                        "grouped physical/sample provenance mismatch",
                    ));
                }
                let operation = match selected_operation {
                    None => None,
                    Some(0) => Some(RecoveryOperationV1::BodySleep),
                    Some(1) => Some(RecoveryOperationV1::AwakeRest),
                    _ => return Err(CpuHostError::State("invalid grouped operation")),
                };
                if physical.operation != operation {
                    return Err(CpuHostError::State(
                        "grouped sample/physical operation mismatch",
                    ));
                }
                if physical.organism_id != owner
                    || physical.sequence_id.raw().checked_add(1) != Some(next_sequence)
                    || physical.outcome_tick > world_tick
                    || physical.source_tick != motor.tick
                    || motor.native.core.final_generation.checked_add(1) != Some(next_sequence)
                {
                    return Err(CpuHostError::State("grouped M2 physical boundary mismatch"));
                }
                if let Some(consumed) = &self.consumed_voluntary {
                    if consumed.sequence.raw() == 0
                        || consumed.sequence.raw() > physical.sequence_id.raw()
                        || consumed.outcome_tick > physical.outcome_tick
                        || (physical.successful_stationary_body_sleep
                            && (consumed.sequence != physical.sequence_id
                                || consumed.outcome_tick != physical.outcome_tick
                                || consumed.frame != motor.frame_digest))
                    {
                        return Err(CpuHostError::State(
                            "grouped M2 consumed sleep boundary mismatch",
                        ));
                    }
                } else if physical.successful_stationary_body_sleep {
                    return Err(CpuHostError::State(
                        "grouped M2 successful sleep receipt not consumed",
                    ));
                }
                Ok(())
            }
            _ => Err(CpuHostError::State(
                "grouped M2 incomplete physical boundary",
            )),
        }
    }
    /// Consume only a just-produced typed physical receipt. The host commits
    /// this state together with world and memory state in its existing clone.
    pub(super) fn observe_physical(
        &mut self,
        frame: &PerceptionFrame,
        bundle: &MotorCommandBundle,
        physical: &alife_world::SleepChoiceWorldReceiptV1,
        sleep: &mut SleepController,
    ) -> Result<(), CpuHostError> {
        if physical.motor.bundle != *bundle
            || frame.organism_id() != bundle.organism_id
            || frame.tick() != bundle.tick
            || frame.sensor_profile() != SensorProfile::GroundedObjectSlotsSleepChoiceV1
        {
            return Err(CpuHostError::State(
                "grouped M2 physical receipt selection mismatch",
            ));
        }
        if physical.successful_stationary_body_sleep {
            if sleep.state().phase != SleepPhase::Awake {
                return Err(CpuHostError::State(
                    "grouped M2 voluntary receipt outside Awake",
                ));
            }
            self.consume_voluntary_receipt(
                bundle.sequence_id,
                frame.frame_digest(),
                bundle,
                physical.motor.outcome_tick,
            )?;
            sleep.request_voluntary_sleep(physical.motor.outcome_tick)?;
        }
        let motor = self
            .motor
            .last_receipt
            .as_ref()
            .ok_or(CpuHostError::State("missing grouped source receipt"))?;
        if motor.frame_digest != frame.frame_digest() || motor.tick != frame.tick() {
            return Err(CpuHostError::State("grouped physical frame mismatch"));
        }
        let (representative, slots, _) = self.physical_selection()?;
        self.last_joint = Some(JointActionSelectionV1::new(
            slots.map(|i| i.map_or(0, |i| i + 1)),
            bundle,
        )?);
        self.last_bundle = Some(bundle.clone());
        self.last_representative = Some(representative);
        self.last_physical = Some(physical.summary());
        Ok(())
    }
    pub(super) fn consume_voluntary_receipt(
        &mut self,
        sequence: ExperienceSequenceId,
        frame: PerceptionFrameDigest,
        bundle: &MotorCommandBundle,
        outcome_tick: Tick,
    ) -> Result<(), CpuHostError> {
        if self.consumed_voluntary.as_ref().is_some_and(|old| {
            old.sequence.raw() >= sequence.raw() || old.outcome_tick >= outcome_tick
        }) {
            return Err(CpuHostError::State(
                "grouped M2 voluntary receipt already consumed",
            ));
        }
        self.consumed_voluntary = Some(VoluntarySleepReceiptV1 {
            sequence,
            frame,
            bundle_digest: json_digest(b"ALIFE-GROUPED-M2-VOLUNTARY-BUNDLE-V1", bundle)?,
            outcome_tick,
        });
        Ok(())
    }
}

pub struct CpuGroupedM2HostV1 {
    host: CpuCanonicalHost,
    founder: CpuFoundationTrainer,
    source_inputs: PhenotypeCompilerInputs,
    mode: MediationModeV1,
}
/// Only a grouped M2 byte envelope creates this object. No public unqualified Serde
/// decoder is implemented, so historic positional inputs cannot enter restore.
#[derive(Debug, Clone)]
pub struct CpuGroupedM2HostCheckpointV1 {
    payload: GroupedPayloadV1,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupedPayloadV1 {
    schema_version: u16,
    source_lineage: String,
    policy: String,
    runtime_profile_raw: u16,
    selector_tag: u16,
    decoder_stride: u16,
    mode: MediationModeV1,
    source_inputs: PhenotypeCompilerInputs,
    recipe_digest: String,
    source_asset: FoundationWeightAsset,
    founder: crate::FoundationTrainerCheckpoint,
    canonical: CpuCanonicalCheckpoint,
    controllers: BTreeMap<u64, GroupedResidentV1>,
    sampler_bits: BTreeMap<u64, GroupedSamplerBitsV1>,
}
/// JSON's historical decimal decoder is retained for ordinary/M1 formats.
/// This new envelope additionally seals exact IEEE bits for its new f64 sampler
/// scores/receipt. This prevents a decimal-parser rounding difference from
/// changing a future sample or rejecting an otherwise exact continuation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupedSamplerBitsV1 {
    ordinary: Option<Vec<u64>>,
    group: Option<u64>,
    operations: Option<[u64; 2]>,
    action: Option<[u64; 2]>,
}
fn grouped_sampler_bits(
    states: &BTreeMap<u64, GroupedResidentV1>,
) -> BTreeMap<u64, GroupedSamplerBitsV1> {
    states
        .iter()
        .map(|(&id, r)| {
            (
                id,
                GroupedSamplerBitsV1 {
                    ordinary: r.motor.last_receipt.as_ref().map(|m| {
                        m.native
                            .scores
                            .ordinary
                            .iter()
                            .map(|x| x.to_bits())
                            .collect()
                    }),
                    group: r
                        .motor
                        .last_receipt
                        .as_ref()
                        .map(|m| m.native.scores.group.to_bits()),
                    operations: r
                        .motor
                        .last_receipt
                        .as_ref()
                        .map(|m| m.native.scores.operations.map(f64::to_bits)),
                    action: r.last_action.as_ref().map(|a| {
                        [
                            a.temperature.to_bits(),
                            a.old_joint_log_probability.to_bits(),
                        ]
                    }),
                },
            )
        })
        .collect()
}
impl GroupedPayloadV1 {
    fn restore_sampler_bits(&mut self) -> Result<(), CpuHostError> {
        if self.sampler_bits.len() != self.controllers.len() {
            return Err(CpuHostError::State("missing exact sampler bits"));
        }
        for (&id, r) in &mut self.controllers {
            let b = self
                .sampler_bits
                .get(&id)
                .ok_or(CpuHostError::State("missing exact sampler owner"))?;
            match (
                &mut r.motor.last_receipt,
                &b.ordinary,
                b.group,
                b.operations,
            ) {
                (Some(m), Some(ordinary), Some(group), Some(operations))
                    if ordinary.len() == m.native.scores.ordinary.len() =>
                {
                    m.native.scores.ordinary =
                        ordinary.iter().map(|&x| f64::from_bits(x)).collect();
                    m.native.scores.group = f64::from_bits(group);
                    m.native.scores.operations = operations.map(f64::from_bits);
                }
                (None, None, None, None) => (),
                _ => return Err(CpuHostError::State("invalid exact sampler score banks")),
            }
            match (&mut r.last_action, b.action) {
                (Some(a), Some([temperature, logp])) => {
                    a.temperature = f64::from_bits(temperature);
                    a.old_joint_log_probability = f64::from_bits(logp);
                }
                (None, None) => (),
                _ => return Err(CpuHostError::State("invalid exact sampler action bits")),
            }
        }
        Ok(())
    }
}
impl CpuGroupedM2HostCheckpointV1 {
    pub fn encode(&self) -> Result<Vec<u8>, CpuHostError> {
        let mut out = MAGIC.to_vec();
        out.extend(serde_json::to_vec(&self.payload)?);
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, CpuHostError> {
        // This check is structurally before even a generic JSON parse, and well
        // before PhenotypeCompilerInputs' historic positional enum codecs.
        let body = bytes.strip_prefix(MAGIC).ok_or(CpuHostError::State(
            "not the explicit fresh grouped M2 lineage envelope",
        ))?;
        let metadata: serde_json::Value = serde_json::from_slice(body)?;
        if metadata["schema_version"] != 1
            || metadata["source_lineage"] != LINEAGE
            || metadata["policy"] != POLICY
            || metadata["runtime_profile_raw"] != 5
            || metadata["selector_tag"] != 12
            || metadata["decoder_stride"] != 55
            || metadata["recipe_digest"] != RECOVERY_GROUP_CONFIDENCE_RECIPE_SHA256
            || metadata["canonical"]["schema_version"] != 2
            || !metadata["canonical"]["residents"]
                .as_array()
                .is_some_and(|rows| {
                    !rows.is_empty() && rows.iter().all(|r| r["neural"]["schema_version"] == 2)
                })
        {
            return Err(CpuHostError::State(
                "grouped M2 outer lineage/profile/policy mismatch",
            ));
        }
        let mut payload: GroupedPayloadV1 = serde_json::from_value(metadata.clone())?;
        payload.restore_sampler_bits()?;
        for state in payload.controllers.values() {
            state.validate_policy::<LegacyGroupedPolicy>()?;
        }
        // Legacy nested types have intentionally permissive historical codecs.
        // This new opaque envelope must not silently erase unknown fields or
        // invent omitted defaults anywhere in its exact continuation payload.
        // Compare through the actual wire serializer: to_value(f32) widens
        // to f64 and can differ from the shortest f32 JSON token.
        if serde_json::from_slice::<serde_json::Value>(&serde_json::to_vec(&payload)?)? != metadata
        {
            return Err(CpuHostError::State(
                "grouped envelope noncanonical nested state",
            ));
        }
        Ok(Self { payload })
    }
}
/// Separately named, process-local research output. Never saved or consumed by policy.
/// The ordinary step return type and entry point remain unchanged.
pub struct CpuGroupedM2ResearchStepV1<A = RecoveryGroupJointActionV2> {
    pub step: CpuGroupedM2HostStepV1<A>,
    pub world_receipts: Vec<alife_world::HeadlessMotorTransactionReceipt>,
}
pub struct CpuGroupedM2HostStepV1<A = RecoveryGroupJointActionV2> {
    pub tick_before: Tick,
    pub tick_after: Tick,
    pub decisions: Vec<CpuGroupedM2HostDecisionV1<A>>,
    pub sleeping_organisms: usize,
    pub sleep_consolidations: usize,
}
pub struct CpuGroupedM2HostDecisionV1<A = RecoveryGroupJointActionV2> {
    pub organism_id: OrganismId,
    pub action: A,
    pub sampling_counter: u32,
    /// Direct source physical-row readouts, not grouped sampling logits.
    /// Also used by the declared source-logit regional-arousal emission policy.
    pub source_diagnostic_logits: Vec<f32>,
    /// Actual source-driven neural signals supplied to physical world biology.
    pub biological_emissions: NeuralEmissionFrame,
    pub brain_work: BrainWorkCounters,
    pub patch: ExperiencePatch,
    pub motor: GroupedMotorReceiptV1,
    pub physical: alife_world::SleepChoicePhysicalReceiptV1,
    pub sleep_phase: SleepPhase,
    pub sleep_trigger: Option<SleepTrigger>,
}
/// The generic SleepController is shared, but this envelope admits only its
/// synchronous, closed-memory completion contract, never pending GPU work.
pub(super) fn grouped_sleep_output_digest(r: &Resident) -> Result<[u64; 4], CpuHostError> {
    json_digest(
        if r.grouped
            .as_ref()
            .is_some_and(|g| g.optional_vocal.is_some())
        {
            b"ALIFE-OPTIONAL-VOCAL-M2-MEMORY-SLEEP-OUTPUT-V1".as_slice()
        } else {
            b"ALIFE-GROUPED-M2-MEMORY-SLEEP-OUTPUT-V1".as_slice()
        },
        &(
            r.neural.checkpoint()?,
            r.memory.export_active_bank()?,
            &r.predictor,
            r.topology.export_portable()?,
            r.sleep.state().active_cycle_id,
        ),
    )
}
pub(super) fn validate_grouped_sleep_boundary(r: &Resident) -> Result<(), CpuHostError> {
    let s = r.sleep.state();
    match s.consolidation {
        ConsolidationState::None => Ok(()),
        ConsolidationState::Committed {
            cycle_id,
            output_generation,
            output_digest,
        } if cycle_id == s.active_cycle_id
            && output_generation == cycle_id
            && output_digest == grouped_sleep_output_digest(r)? =>
        {
            Ok(())
        }
        _ => Err(CpuHostError::State(
            "invalid closed grouped memory-sleep boundary",
        )),
    }
}
impl CpuGroupedM2HostV1 {
    /// Explicitly unsupported in this fresh, synaptic-learning-closed version.
    pub fn actor_optimization(&mut self) -> Result<(), CpuHostError> {
        Err(CpuHostError::State(
            "grouped M2 V1 synaptic learning is closed",
        ))
    }
    pub fn waking_synaptic_plasticity(&mut self) -> Result<(), CpuHostError> {
        Err(CpuHostError::State(
            "grouped M2 V1 synaptic learning is closed",
        ))
    }
    pub fn synaptic_sleep_consolidation(&mut self) -> Result<(), CpuHostError> {
        Err(CpuHostError::State(
            "grouped M2 V1 synaptic learning is closed",
        ))
    }
    fn validate_pressure_valuation(value: BiologicalValueProfile) -> Result<(), CpuHostError> {
        if value.hormones[8] != 0. {
            return Err(CpuHostError::State(
                "initial grouped M2 routing requires unchanged pressure valuation",
            ));
        }
        Ok(())
    }
    fn validate_source_biology(world: &HeadlessWorld) -> Result<(), CpuHostError> {
        for record in world.organism_registry().iter() {
            if record.lifecycle().is_alive() {
                alife_world::SleepChoiceWorldAdmissionV1::for_organism(
                    world,
                    record.organism_id(),
                )?;
            } else if !record
                .phenotype()
                .chemistry
                .biochemical
                .supports_awake_rest_recovery_v1()
                || record.embodiment().effector_gain(EffectorCapability::Rest) <= 0.
            {
                return Err(CpuHostError::State(
                    "terminal grouped source biology mismatch",
                ));
            }
            Self::validate_pressure_valuation(
                record.phenotype().chemistry.biochemical.value_profile(),
            )?;
        }
        Ok(())
    }
    fn validate_source_channels(source: &BrainPhenotype) -> Result<(), CpuHostError> {
        if !source
            .candidate_decoder()
            .factorized_motor_channels(source)?
            .contains(&MotorChannel::Posture)
        {
            return Err(CpuHostError::State(
                "grouped M2 requires an inherited Posture motor head",
            ));
        }
        Ok(())
    }
    pub fn new_prepared<T, E: std::fmt::Display>(
        game: CanonicalNewGame,
        founder: CpuFoundationTrainer,
        source_inputs: PhenotypeCompilerInputs,
        sampling_seed: u32,
        mode: MediationModeV1,
        prepare: impl FnOnce(&mut HeadlessWorld) -> Result<T, E>,
    ) -> Result<(Self, T), CpuHostError> {
        Self::new_prepared_policy::<LegacyGroupedPolicy, T, E>(
            game,
            founder,
            source_inputs,
            sampling_seed,
            mode,
            prepare,
        )
    }
    fn new_prepared_policy<P: GroupedPolicyStrategy, T, E: std::fmt::Display>(
        game: CanonicalNewGame,
        founder: CpuFoundationTrainer,
        source_inputs: PhenotypeCompilerInputs,
        sampling_seed: u32,
        mode: MediationModeV1,
        prepare: impl FnOnce(&mut HeadlessWorld) -> Result<T, E>,
    ) -> Result<(Self, T), CpuHostError> {
        Self::validate_source_channels(founder.phenotype())?;
        let motor = GroupedMotorStateV1::fresh(&source_inputs, founder.phenotype(), mode)?;
        motor.validate(founder.phenotype())?;
        // The caller supplied a fresh genome whose circuit was present before
        // developmental expression. Never replace stored chemistry here.
        Self::validate_source_biology(&game.world)?;
        let (mut host, prepared) = CpuCanonicalHost::new_prepared_grounded_drive_research_cohort(
            game,
            &founder,
            sampling_seed,
            prepare,
        )?;
        Self::validate_source_biology(&host.world)?;
        for (&id, r) in &mut host.residents {
            if r.neural.checkpoint()?.last_decision_tick.is_some()
                || r.next_sequence != 1
                || !r.replay.is_empty()
            {
                return Err(CpuHostError::State(
                    "grouped M2 is fresh-only, cannot reset acquired resident",
                ));
            }
            if P::OPTIONAL {
                // This is the fresh birth path above, before any decision or
                // acquired state. Construct the new source owner directly.
                r.neural = CpuOrganismState::new_optional_vocal(
                    founder.phenotype(),
                    OrganismId(id),
                    r.neural.neural_state().clone(),
                )?;
            }
            let profile = super::sleep_choice_profile_identity();
            r.memory = MemorySidecarState::new_profiled(
                OrganismId(id),
                profile,
                MemoryBankConfig::new(256, 64, 4, 0.72, Confidence::new(0.)?)?,
            )?;
            r.topology = TopologySidecar::new_profiled(
                OrganismId(id),
                profile,
                TopologicalMapConfig::default(),
            )?;
            r.grouped = Some(GroupedResidentV1 {
                motor: motor.clone(),
                memory_proofs: Vec::new(),
                last_physical: None,
                consumed_voluntary: None,
                last_action: None,
                optional_vocal: P::OPTIONAL.then(OptionalVocalResidentV1::fresh),
                last_bundle: None,
                last_joint: None,
                last_representative: None,
            });
        }
        host.seal_all_residents()?;
        Ok((
            Self {
                host,
                founder,
                source_inputs,
                mode,
            },
            prepared,
        ))
    }
    pub fn world(&self) -> &HeadlessWorld {
        self.host.world()
    }
    /// Read-only observation, including emergency intervals without a decision.
    pub fn sleep_state(&self, organism: OrganismId) -> Option<alife_core::sleep::SleepState> {
        self.host
            .residents
            .get(&organism.raw())
            .map(|resident| resident.sleep.state())
    }
    pub fn checkpoint(&self) -> Result<CpuGroupedM2HostCheckpointV1, CpuHostError> {
        Ok(CpuGroupedM2HostCheckpointV1 {
            payload: self.checkpoint_payload::<LegacyGroupedPolicy>()?,
        })
    }
    fn checkpoint_payload<P: GroupedPolicyStrategy>(
        &self,
    ) -> Result<GroupedPayloadV1, CpuHostError> {
        self.host.validate_founder(&self.founder)?;
        for (&id, r) in &self.host.residents {
            validate_grouped_sleep_boundary(r)?;
            let record = self
                .host
                .world
                .organism_registry()
                .get(OrganismId(id))
                .ok_or(CpuHostError::State("missing grouped owner"))?;
            validate_resident_authority(
                record,
                r,
                self.host.sampling_seed,
                self.host.sampling_counter,
            )?;
            let state = r
                .grouped
                .as_ref()
                .ok_or(CpuHostError::State("missing grouped state"))?;
            state.validate_policy::<P>()?;
            state.validate_optional_source(&self.founder, &r.neural)?;
            state.motor.validate(self.founder.phenotype())?;
            state.validate_source_boundary(&r.neural.checkpoint()?, &r.activity)?;
            state.validate_replay(&r.replay)?;
            if state.sampling_cursor().is_some_and(|(seed, counter)| {
                seed != self.host.sampling_seed
                    || counter.checked_add(1) != Some(self.host.sampling_counter)
            }) {
                return Err(CpuHostError::State("grouped sampling cursor mismatch"));
            }
            state.validate_boundary(OrganismId(id), r.next_sequence, self.host.world.tick())?;
        }
        Ok(GroupedPayloadV1 {
            schema_version: 1,
            source_lineage: P::LINEAGE.into(),
            policy: P::POLICY.into(),
            runtime_profile_raw: 5,
            selector_tag: 12,
            decoder_stride: 55,
            mode: self.mode,
            source_inputs: self.source_inputs.clone(),
            recipe_digest: RECOVERY_GROUP_CONFIDENCE_RECIPE_SHA256.into(),
            source_asset: self.founder.source_foundation.clone(),
            founder: self.founder.checkpoint(),
            canonical: self.host.checkpoint_inner(&self.founder)?,
            sampler_bits: grouped_sampler_bits(
                &self
                    .host
                    .residents
                    .iter()
                    .map(|(&id, r)| {
                        r.grouped
                            .clone()
                            .map(|g| (id, g))
                            .ok_or(CpuHostError::State("missing grouped state"))
                    })
                    .collect::<Result<BTreeMap<_, _>, CpuHostError>>()?,
            ),
            controllers: self
                .host
                .residents
                .iter()
                .map(|(&id, r)| {
                    Ok((
                        id,
                        r.grouped
                            .clone()
                            .ok_or(CpuHostError::State("missing grouped M2 controller"))?,
                    ))
                })
                .collect::<Result<_, CpuHostError>>()?,
        })
    }
    pub fn restore(
        checkpoint: CpuGroupedM2HostCheckpointV1,
        expected_source: &PhenotypeCompilerInputs,
        expected_mode: MediationModeV1,
    ) -> Result<Self, CpuHostError> {
        let wire = checkpoint.encode()?;
        Self::restore_payload::<LegacyGroupedPolicy>(
            checkpoint.payload,
            expected_source,
            expected_mode,
            wire,
        )
    }
    fn restore_payload<P: GroupedPolicyStrategy>(
        mut c: GroupedPayloadV1,
        expected_source: &PhenotypeCompilerInputs,
        expected_mode: MediationModeV1,
        wire: Vec<u8>,
    ) -> Result<Self, CpuHostError> {
        if c.sampler_bits != grouped_sampler_bits(&c.controllers) {
            return Err(CpuHostError::State("grouped exact sampler bits differ"));
        }
        if c.schema_version != 1
            || c.source_lineage != P::LINEAGE
            || c.policy != P::POLICY
            || c.runtime_profile_raw != 5
            || c.selector_tag != 12
            || c.decoder_stride != 55
            || c.mode != expected_mode
            || &c.source_inputs != expected_source
            || c.recipe_digest != RECOVERY_GROUP_CONFIDENCE_RECIPE_SHA256
        {
            return Err(CpuHostError::State(
                "grouped M2 source, controller, profile or policy mismatch",
            ));
        }
        if c.canonical.schema_version
            != if P::OPTIONAL {
                OPTIONAL_VOCAL_CANONICAL_SCHEMA
            } else {
                2
            }
            || c.canonical.residents.iter().any(|r| {
                r.neural.schema_version
                    != if P::OPTIONAL {
                        crate::cpu::organism::OPTIONAL_VOCAL_NEURAL_SCHEMA
                    } else {
                        2
                    }
            })
        {
            return Err(CpuHostError::State(
                "inner canonical/source policy mismatch",
            ));
        }
        for state in c.controllers.values() {
            state.validate_policy::<P>()?;
        }
        // Restore is a cold admission boundary for every typed policy, including
        // in-process checkpoint clones. Clear only the transient compiler output;
        // source, owner, mutable banks and receipts retain their existing checks.
        for state in c.controllers.values_mut() {
            state.motor.clear_validation_cache();
        }
        let p = PhenotypeCompiler::compile_validated(
            expected_source,
            &BrainCapacityClass::n2048_grounded_drive_research(),
        )?;
        Self::validate_source_channels(&p)?;
        let founder = CpuFoundationTrainer::from_checkpoint(p, c.source_asset, c.founder)?;
        let host = CpuCanonicalHost::from_checkpoint_with_grouped(
            c.canonical,
            &founder,
            true,
            None,
            None,
            Some(c.controllers),
        )?;
        for r in host.residents.values() {
            if !r
                .grouped
                .as_ref()
                .is_some_and(|s| s.motor.matches(expected_source, expected_mode))
            {
                return Err(CpuHostError::State(
                    "grouped M2 resident mode/source mismatch",
                ));
            }
        }
        Self::validate_source_biology(&host.world)?;
        let result = Self {
            host,
            founder,
            source_inputs: c.source_inputs,
            mode: c.mode,
        };
        let payload = result.checkpoint_payload::<P>()?;
        let mut restored_wire = P::MAGIC.to_vec();
        restored_wire.extend(serde_json::to_vec(&payload)?);
        if restored_wire != wire {
            return Err(CpuHostError::State(
                "grouped M2 restore would rewrite acquired state",
            ));
        }
        Ok(result)
    }
    pub fn step_research_observed(
        &mut self,
        temperature: f32,
    ) -> Result<CpuGroupedM2ResearchStepV1, CpuHostError> {
        self.host
            .grouped_policy_step_observed::<LegacyGroupedPolicy>(&self.founder, temperature)
    }
    pub fn step(&mut self, temperature: f32) -> Result<CpuGroupedM2HostStepV1, CpuHostError> {
        self.host.grouped_step(&self.founder, temperature)
    }
}

#[cfg(test)]
#[path = "grouped_host_tests.rs"]
mod tests;

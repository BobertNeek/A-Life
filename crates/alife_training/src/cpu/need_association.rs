//! Reconstructed protected CPU association port. Comparison is predictive own
//! history advantage, not isolated causal credit or an externally corrected reward.
//! Default foundations, V1 acquired state and legacy replay/checkpoints are unchanged.
use super::need_association_source::*;
use super::CpuHostError;
use alife_core::*;
use alife_world::{HeadlessMotorTransactionReceipt, WorldOrganismRecord};
use std::collections::BTreeMap;

pub const CPU_NEED_ASSOCIATION_PORT_V4: &str =
    "cpu-phenotype-bound-stationary-own-history-advantage-v4";
const CAPACITY: usize = 32;
const MAX_AGE: u64 = 16;
const MAX_PHYSIOLOGY_DRIFT: f32 = 0.125;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub(super) struct StationaryContext {
    subject: AssociationSubjectV1,
    candidate: ActionCandidate,
    body: BodySnapshot,
    slots: Vec<GroundedObjectSlotV1>,
    immutable_binding: Vec<(&'static str, [u64; 4])>,
    physiology: BiochemistryState,
    hash_bytes: u64,
}
impl StationaryContext {
    fn from_native(
        frame: &PerceptionFrame,
        candidate: &ActionCandidate,
        record: &WorldOrganismRecord,
    ) -> Result<Self, CpuHostError> {
        let subject = candidate_subject(frame, candidate)
            .map_err(|_| CpuHostError::State("unsupported association candidate"))?;
        if frame.organism_id() != record.organism_id()
            || frame.homeostasis() != &record.biochemistry().homeostasis
        {
            return Err(CpuHostError::State(
                "association source physiology mismatch",
            ));
        }
        let mut candidate = *candidate;
        candidate.candidate_index = 0;
        let mut embodiment = serde_json::to_value(record.embodiment())?;
        embodiment
            .as_object_mut()
            .ok_or(CpuHostError::State("embodiment shape"))?
            .remove("source_tick");
        let development = record.phenotype().development_state_at(frame.tick())?;
        // Only clocks leave stable identity. Actual calibration, channels and
        // expressed developmental/effector states remain exactly bound.
        let values = serde_json::to_value((
            CPU_NEED_ASSOCIATION_PORT_V4,
            record.organism_id(),
            record.birth_tick(),
            record.phenotype(),
            embodiment,
            frame.sensor_profile(),
            frame.profile_provenance().identity(),
            development.enabled_lobes,
            development.active_sensor_channels,
            development.active_motor_affordances,
            development.open_critical_periods,
            &frame.sensory().channels,
        ))?;
        const LABELS: [&str; 12] = [
            "port identity",
            "owner",
            "birth",
            "phenotype",
            "embodiment",
            "sensor profile",
            "profile identity",
            "expressed lobes",
            "expressed sensors",
            "expressed effectors",
            "critical periods",
            "sensory channels",
        ];
        let mut hash_bytes = 0u64;
        let immutable_binding = values
            .as_array()
            .ok_or(CpuHostError::State("stationary binding shape"))?
            .iter()
            .zip(LABELS)
            .map(|(value, label)| {
                let mut d = CanonicalDigestBuilder::new(b"ALIFE-STATIONARY-CONTEXT-PART-V1");
                d.write_bytes(label.as_bytes());
                let bytes = serde_json::to_vec(value)?;
                hash_bytes += bytes.len() as u64;
                d.write_bytes(&bytes);
                Ok((label, d.finish256()))
            })
            .collect::<Result<Vec<_>, CpuHostError>>()?;
        Ok(Self {
            subject,
            candidate,
            body: frame.body(),
            slots: frame.grounded_object_slots().to_vec(),
            immutable_binding,
            physiology: *record.biochemistry(),
            hash_bytes,
        })
    }
    fn mismatch_fields(&self, other: &Self, reference: bool) -> Vec<String> {
        let mut fields = Vec::new();
        if !reference && self.subject != other.subject {
            fields.push("subject".into());
        }
        if !reference && self.candidate != other.candidate {
            fields.push("candidate action/target/features".into());
        }
        if self.body != other.body {
            fields.push("body pose/velocity".into());
        }
        if self.slots != other.slots {
            fields.push("observed slots/geometry".into());
        }
        for (a, b) in self.immutable_binding.iter().zip(&other.immutable_binding) {
            if a != b {
                fields.push(a.0.into());
            }
        }
        if self.immutable_binding.len() != other.immutable_binding.len() {
            fields.push("binding shape".into());
        }
        if !physiological_locality(self.physiology, other.physiology) {
            let a = self.physiology;
            let b = other.physiology;
            if a.body != b.body {
                fields.push("organ/reserve locality or injury/thermal/sleep".into());
            }
            if a.homeostasis.drives != b.homeostasis.drives {
                fields.push("drive locality".into());
            }
            if a.homeostasis.hormones != b.homeostasis.hormones {
                fields.push("hormone locality".into());
            }
            if a.development != b.development {
                fields.push("development expression".into());
            }
        }
        fields
    }
    fn compatible(&self, other: &Self) -> bool {
        self.subject == other.subject
            && self.candidate == other.candidate
            && self.same_stationary_scene(other)
    }
    fn same_stationary_scene(&self, other: &Self) -> bool {
        self.body == other.body
            && self.slots == other.slots
            && self.immutable_binding == other.immutable_binding
            && physiological_locality(self.physiology, other.physiology)
    }
}
fn physiological_locality(a: BiochemistryState, b: BiochemistryState) -> bool {
    let close =
        |x: f32, y: f32| x.is_finite() && y.is_finite() && (x - y).abs() <= MAX_PHYSIOLOGY_DRIFT;
    let aa = a.body;
    let bb = b.body;
    a.source_genome_id == b.source_genome_id
        && a.cadence == b.cadence
        && aa.health == bb.health
        && aa.injury == bb.injury
        && aa.temperature_stress == bb.temperature_stress
        && aa.sleeping == bb.sleeping
        && close(aa.energy, bb.energy)
        && aa.organs().iter().zip(bb.organs()).all(|(a, b)| {
            let mut a = *a;
            let mut b = *b;
            let energies = close(a.energy, b.energy);
            a.energy = 0.;
            b.energy = 0.;
            energies && a == b
        })
        && a.homeostasis
            .drives
            .to_array()
            .iter()
            .zip(b.homeostasis.drives.to_array())
            .all(|(a, b)| close(*a, b))
        && a.homeostasis
            .hormones
            .to_array()
            .iter()
            .zip(b.homeostasis.hormones.to_array())
            .all(|(a, b)| close(*a, b))
        && a.development.puberty_reached == b.development.puberty_reached
        && a.development.critical_period_active == b.development.critical_period_active
        && a.development.critical_period_plasticity_bias
            == b.development.critical_period_plasticity_bias
        && a.development.biochemical_expression == b.development.biochemical_expression
        && a.development.migration_ready == b.development.migration_ready
}
#[derive(Debug, Clone, serde::Serialize)]
struct RetainedAssociation {
    context: StationaryContext,
    sequence: ExperienceSequenceId,
    source_frame: PerceptionFrameDigest,
    outcome_tick: Tick,
    physical: PhysicalActionOutcome,
    representative_channel: MotorChannel,
    benign_noop_channels: usize,
    before: [f32; 11],
    after: [f32; 11],
    raw_delta: [f32; 11],
    confidence: f32,
}
/// Construction and writes are private to the canonical native host. No
/// Deserialize or legacy checkpoint path can populate these acquired records.
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct CpuNeedAssociationStateV4 {
    identity: &'static str,
    owner: OrganismId,
    reference: SelfActionV1,
    last_sequence: u64,
    records: Vec<RetainedAssociation>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum CpuNeedObservationStatusV4 {
    Stored,
    Unsupported(&'static str),
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuNeedAssociationSupportV4 {
    pub action_sequence: Option<u64>,
    pub reference_sequence: Option<u64>,
    pub action_raw_delta: Option<[f32; 11]>,
    pub reference_raw_delta: Option<[f32; 11]>,
    pub action_confidence: f32,
    pub reference_confidence: f32,
    pub lookup_records: u64,
    pub action_mismatch_fields: Vec<String>,
    pub reference_mismatch_fields: Vec<String>,
}
impl CpuNeedAssociationStateV4 {
    pub(super) fn new(owner: OrganismId, reference: SelfActionV1) -> Self {
        Self {
            identity: CPU_NEED_ASSOCIATION_PORT_V4,
            owner,
            reference,
            last_sequence: 0,
            records: Vec::new(),
        }
    }
    pub(super) fn observe(
        &mut self,
        patch: &ExperiencePatch,
        receipt: &HeadlessMotorTransactionReceipt,
        before: &WorldOrganismRecord,
        after: &BiochemistryState,
        contexts: &BTreeMap<u16, StationaryContext>,
    ) -> Result<CpuNeedObservationStatusV4, CpuHostError> {
        if patch.header().organism_id != self.owner
            || patch.header().sequence_id.0 <= self.last_sequence
        {
            return Err(CpuHostError::State("association owner or replay mismatch"));
        }
        self.last_sequence = patch.header().sequence_id.0;
        let source = match qualify_native_interval(patch, receipt, before, after) {
            Ok(source) => source,
            Err(reason) => return Ok(CpuNeedObservationStatusV4::Unsupported(reason.label())),
        };
        let pre = patch.pre_action();
        let evidence = patch.decision().neural_evidence()?;
        let candidate = &pre.perception().candidates()[usize::from(evidence.candidate_index)];
        let context = contexts
            .get(&evidence.candidate_index)
            .ok_or(CpuHostError::State(
                "qualified source lacks prepared context",
            ))?
            .clone();
        let mut expected_candidate = *candidate;
        expected_candidate.candidate_index = 0;
        if context.candidate != expected_candidate || context.physiology != *before.biochemistry() {
            return Err(CpuHostError::State("prepared source context mismatch"));
        }
        if context.subject != source.subject
            || source.representative_physical.displacement != Vec3f::ZERO
        {
            return Ok(CpuNeedObservationStatusV4::Unsupported(
                "nonstationary_source",
            ));
        }
        let confidence = evidence
            .confidence
            .raw()
            .min(candidate.sensor_confidence.raw());
        self.records.push(RetainedAssociation {
            context,
            sequence: source.source_sequence,
            source_frame: source.source_frame_digest,
            outcome_tick: patch.outcome().outcome_tick,
            physical: source.representative_physical,
            representative_channel: source.representative_channel,
            benign_noop_channels: source.benign_noop_channels,
            before: source.drive_before,
            after: source.drive_after,
            raw_delta: source.raw_delta,
            confidence,
        });
        if self.records.len() > CAPACITY {
            self.records.remove(0);
        }
        Ok(CpuNeedObservationStatusV4::Stored)
    }
    #[cfg(test)]
    fn recall(
        &self,
        frame: &PerceptionFrame,
        candidate: &ActionCandidate,
        record: &WorldOrganismRecord,
    ) -> Result<CpuNeedAssociationSupportV4, CpuHostError> {
        let context = StationaryContext::from_native(frame, candidate, record)?;
        self.recall_context(&context, frame.tick())
    }
    fn recall_context(
        &self,
        context: &StationaryContext,
        tick: Tick,
    ) -> Result<CpuNeedAssociationSupportV4, CpuHostError> {
        let mut action = None;
        let mut reference = None;
        let mut action_mismatch_fields = vec!["no source in namespace".into()];
        let mut reference_mismatch_fields = vec!["no own-reference source".into()];
        // V2 diagnostic repair: eligibility and mismatch reporting share the same
        // age check. No retrieval eligibility, genes or protocol changed.
        for item in &self.records {
            let age_reason = if item.outcome_tick.raw() > tick.raw() {
                Some("source outcome is in the future")
            } else if tick.raw() - item.outcome_tick.raw() > MAX_AGE {
                Some("source exceeds maximum age")
            } else {
                None
            };
            if item.context.subject == context.subject {
                action_mismatch_fields = item.context.mismatch_fields(context, false);
                if let Some(reason) = age_reason {
                    action_mismatch_fields.push(reason.into());
                }
            }
            if item.context.subject
                == (AssociationSubjectV1::SelfAction {
                    organism: self.owner,
                    action: self.reference,
                })
            {
                reference_mismatch_fields = item.context.mismatch_fields(context, true);
                if let Some(reason) = age_reason {
                    reference_mismatch_fields.push(reason.into());
                }
            }
            if age_reason.is_some() {
                continue;
            }
            if item.context.compatible(context) {
                action = Some(item);
            }
            if item.context.subject
                == (AssociationSubjectV1::SelfAction {
                    organism: self.owner,
                    action: self.reference,
                })
                && item.context.same_stationary_scene(context)
            {
                reference = Some(item);
            }
        }
        Ok(CpuNeedAssociationSupportV4 {
            action_sequence: action.map(|r| r.sequence.0),
            reference_sequence: reference.map(|r| r.sequence.0),
            action_raw_delta: action.map(|r| r.raw_delta),
            reference_raw_delta: reference.map(|r| r.raw_delta),
            action_confidence: action.map_or(0., |r| r.confidence),
            reference_confidence: reference.map_or(0., |r| r.confidence),
            lookup_records: self.records.len() as u64 * 2,
            action_mismatch_fields: if action.is_some() {
                Vec::new()
            } else {
                action_mismatch_fields
            },
            reference_mismatch_fields: if reference.is_some() {
                Vec::new()
            } else {
                reference_mismatch_fields
            },
        })
    }
}
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct CpuNeedAssociationPlanV4 {
    pub recipe: NativeNeedAssociationRecipeV2,
    pub reference: SelfActionV1,
    pub execution: NativeNeedExecutionV1,
}
impl CpuNeedAssociationPlanV4 {
    pub fn identity_control(reference: SelfActionV1) -> Self {
        Self {
            recipe: NativeNeedAssociationRecipeV2::phase3_species22(),
            reference,
            execution: NativeNeedExecutionV1::FullScratchReference,
        }
    }
}
#[derive(Debug, Clone)]
pub(super) struct CpuNeedAssociationPortV4 {
    pub(super) state: CpuNeedAssociationStateV4,
    pub(super) circuit: NativeNeedAssociationCircuitV2,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuNeedCandidateReceiptV4 {
    pub candidate_index: u16,
    pub action_id: ActionId,
    pub family: CandidateActionFamily,
    pub support: CpuNeedAssociationSupportV4,
    pub native: NativeNeedAssociationOutputV2,
    pub base_logit: f32,
    pub final_logit: f32,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuNeedDecisionReceiptV4 {
    pub port_identity: &'static str,
    pub organism: OrganismId,
    pub tick: Tick,
    pub binding_work: NativeNeedBindingWorkV2,
    pub binding_memory_ops: u64,
    pub context_hash_bytes: u64,
    pub context_memory_ops: u64,
    pub candidates: Vec<CpuNeedCandidateReceiptV4>,
    pub source_observation: CpuNeedObservationStatusV4,
    pub source_audit: Option<CpuNeedSourceAuditV4>,
}
#[derive(Debug, Clone)]
pub(super) struct CpuNeedPreparedV4 {
    pub(super) receipt: CpuNeedDecisionReceiptV4,
    pub(super) work: BrainWorkCounters,
    pub(super) dendritic_ops: u64,
    pub(super) contexts: BTreeMap<u16, StationaryContext>,
}
impl CpuNeedAssociationPortV4 {
    pub(super) fn new(
        owner: OrganismId,
        phenotype: &CreaturePhenotype,
        plan: CpuNeedAssociationPlanV4,
    ) -> Result<Self, CpuHostError> {
        Ok(Self {
            state: CpuNeedAssociationStateV4::new(owner, plan.reference),
            circuit: NativeNeedAssociationCircuitV2::compile_bound(
                phenotype,
                plan.recipe,
                plan.execution,
            )
            .map_err(|_| CpuHostError::State("invalid association port recipe"))?,
        })
    }
    pub(super) fn prepare(
        &self,
        frame: &PerceptionFrame,
        record: &WorldOrganismRecord,
    ) -> Result<CpuNeedPreparedV4, CpuHostError> {
        let bound = self
            .circuit
            .bind_current(record.phenotype(), record.biochemistry())
            .map_err(|_| CpuHostError::State("phenotype-bound current physiology rejected"))?;
        let binding_memory_ops = binding_memory_ops(bound.work())?;
        let mut candidates = Vec::new();
        let mut contexts = BTreeMap::new();
        let mut context_hash_bytes = 0;
        let mut context_memory_ops = 0;
        let mut work = BrainWorkCounters {
            memory_context_ops: binding_memory_ops,
            ..BrainWorkCounters::default()
        };
        let mut dendritic_ops = 0;
        let mut common_context: Option<StationaryContext> = None;
        for candidate in frame.candidates() {
            if candidate_subject(frame, candidate).is_err() {
                continue;
            }
            // Shared immutable/physiological context is built once per decision.
            let context = if let Some(common) = &common_context {
                let mut context = common.clone();
                context.subject = candidate_subject(frame, candidate)
                    .map_err(|_| CpuHostError::State("candidate admission changed"))?;
                context.candidate = *candidate;
                context.candidate.candidate_index = 0;
                context
            } else {
                let context = StationaryContext::from_native(frame, candidate, record)?;
                context_hash_bytes = context.hash_bytes;
                // One operation per eight serialized bytes for context hashing.
                // Full phenotype binding above uses its separate declared tariff.
                context_memory_ops = context_hash_bytes.div_ceil(8);
                work.memory_context_ops = work
                    .memory_context_ops
                    .checked_add(context_memory_ops)
                    .ok_or(CpuHostError::State("context work overflow"))?;
                common_context = Some(context.clone());
                context
            };
            let support = self.state.recall_context(&context, frame.tick())?;
            contexts.insert(candidate.candidate_index, context);
            let input = |delta, confidence| -> Result<_, CpuHostError> {
                match delta {
                    None => Ok(NativeNeedTransitionV1::Unknown),
                    Some(delta) => NativeNeedTransitionV1::learned(
                        delta,
                        NativeNeedConfidenceV1::new(confidence)
                            .map_err(|_| CpuHostError::State("invalid source confidence"))?,
                        NativeNeedSupportV1::OwnHistoryAssociation,
                    )
                    .map_err(|_| CpuHostError::State("invalid raw association")),
                }
            };
            let native = self
                .circuit
                .run(
                    &bound,
                    input(support.action_raw_delta, support.action_confidence)?,
                    NativeNeedReferenceV1 {
                        kind: match self.state.reference {
                            SelfActionV1::Rest => NativeNeedReferenceKindV1::Rest,
                            SelfActionV1::Noop => NativeNeedReferenceKindV1::Noop,
                        },
                        transition: input(
                            support.reference_raw_delta,
                            support.reference_confidence,
                        )?,
                    },
                )
                .map_err(|_| CpuHostError::State("native association execution rejected"))?;
            accumulate_work(
                &mut work,
                native.work.brain_work_counters(support.lookup_records),
            )?;
            work.decoder_candidate_ops += 1;
            dendritic_ops += u64::from(native.work.dendritic.branches_evaluated)
                + u64::from(native.work.dendritic.inputs_evaluated);
            candidates.push(CpuNeedCandidateReceiptV4 {
                candidate_index: candidate.candidate_index,
                action_id: candidate.action_id,
                family: candidate.family,
                support,
                native,
                base_logit: 0.,
                final_logit: 0.,
            });
        }
        Ok(CpuNeedPreparedV4 {
            receipt: CpuNeedDecisionReceiptV4 {
                port_identity: CPU_NEED_ASSOCIATION_PORT_V4,
                organism: frame.organism_id(),
                tick: frame.tick(),
                binding_work: bound.work().clone(),
                binding_memory_ops,
                context_hash_bytes,
                context_memory_ops,
                candidates,
                source_observation: CpuNeedObservationStatusV4::Unsupported("not_observed"),
                source_audit: None,
            },
            work,
            dendritic_ops,
            contexts,
        })
    }
}
/// Explicit V4 semantic tariff: each encoded value, each 8-byte hash chunk,
/// each receptor evaluation/comparison/copy and each validation traversal call
/// is one memory-context operation. These are not recurrent neural microsteps.
/// The same tariff reaches ATP admission and final body cognitive charging.
fn binding_memory_ops(work: &NativeNeedBindingWorkV2) -> Result<u64, CpuHostError> {
    [
        work.encoded_values,
        work.encoded_bytes.div_ceil(8),
        u64::from(work.state_validation_calls),
        u64::from(work.homeostasis_derivations),
        u64::from(work.homeostatic_receptor_evaluations),
        u64::from(work.compared_homeostasis_components),
        u64::from(work.copied_drive_components),
    ]
    .into_iter()
    .try_fold(0u64, |sum, n| {
        sum.checked_add(n)
            .ok_or(CpuHostError::State("binding work overflow"))
    })
}
pub(super) fn accumulate_work(
    a: &mut BrainWorkCounters,
    b: BrainWorkCounters,
) -> Result<(), CpuHostError> {
    fn add(a: &mut u64, b: u64) -> Result<(), CpuHostError> {
        *a = a
            .checked_add(b)
            .ok_or(CpuHostError::State("association work overflow"))?;
        Ok(())
    }
    a.microsteps = a
        .microsteps
        .checked_add(b.microsteps)
        .ok_or(CpuHostError::State("association work overflow"))?;
    add(&mut a.neuron_updates, b.neuron_updates)?;
    add(&mut a.tile_visits, b.tile_visits)?;
    add(&mut a.synapse_ops, b.synapse_ops)?;
    add(&mut a.decoder_candidate_ops, b.decoder_candidate_ops)?;
    add(&mut a.memory_context_ops, b.memory_context_ops)
}
impl CpuNeedPreparedV4 {
    /// Only actual native circuit output enters this typed decoder mix.
    /// It is neither a host-computed utility nor an ActionScoreBias.
    pub(super) fn mix_native_logits(
        &mut self,
        logits: &mut [f32],
    ) -> Result<(), super::TrainingError> {
        for row in &mut self.receipt.candidates {
            let logit = logits
                .get_mut(usize::from(row.candidate_index))
                .ok_or_else(super::invalid)?;
            row.base_logit = *logit;
            *logit += row.native.confidence_product_gated_readout;
            row.final_logit = *logit;
            if !logit.is_finite() {
                return Err(super::invalid());
            }
        }
        Ok(())
    }
}
/// Dedicated output omits legacy replay ticks and private neural banks; this
/// port has no admitted PPO/backward or GPU deployment/checkpoint path.
pub struct CpuNeedAssociationStepV4 {
    pub tick_before: Tick,
    pub tick_after: Tick,
    pub decisions: Vec<CpuNeedAssociationDecisionV4>,
    pub sleeping_organisms: usize,
    pub sleep_consolidations: usize,
}
pub struct CpuNeedAssociationDecisionV4 {
    pub organism_id: OrganismId,
    pub action: crate::PpoJointAction,
    pub patch: ExperiencePatch,
    pub brain_work: BrainWorkCounters,
    pub biological_reward: f32,
    pub port: CpuNeedDecisionReceiptV4,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_world::*;
    fn fixture() -> (StationaryContext, PerceptionFrame, WorldOrganismRecord) {
        let asset =
            FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
        let mut config = CanonicalNewGameConfig::phase3(0xA551, 1).unwrap();
        config.sensor_profile = SensorProfile::GroundedObjectSlotsV1;
        config.brain_class = BrainScaleTier::Standard2048;
        let mut game = create_canonical_new_game_with_n2048_candidate(&config, &asset).unwrap();
        let record = game
            .world
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .clone();
        let frame = game
            .world
            .perception_frame(
                record.organism_id(),
                Tick::ZERO,
                config.sensor_profile,
                record.biochemistry().homeostasis,
            )
            .unwrap();
        let candidate = frame
            .candidates()
            .iter()
            .find(|c| c.kind == ActionKind::Rest)
            .unwrap();
        (
            StationaryContext::from_native(&frame, candidate, &record).unwrap(),
            frame,
            record,
        )
    }
    #[test]
    fn self_action_namespace_cannot_alias_observed_target() {
        assert_ne!(
            AssociationSubjectV1::SelfAction {
                organism: OrganismId(1),
                action: SelfActionV1::Rest
            },
            AssociationSubjectV1::SelfAction {
                organism: OrganismId(1),
                action: SelfActionV1::Noop
            }
        );
    }
    #[test]
    fn physiological_drift_is_separate_from_action_and_stationary_context() {
        let (context, _, _) = fixture();
        let mut c = context.clone();
        c.physiology.homeostasis.drives.hunger += 0.01;
        c.physiology.body.debit_energy_pro_rata(0.01).unwrap();
        c.physiology.tick = Tick(1);
        assert!(context.compatible(&c));
        c.physiology.homeostasis.drives.hunger += 0.2;
        assert!(!context.compatible(&c));
        assert_ne!(context.physiology, c.physiology);
    }
    #[test]
    fn injury_development_sleep_and_effector_context_fail_closed() {
        let (context, _, _) = fixture();
        for mutate in [
            |c: &mut StationaryContext| c.physiology.body.injury += 0.001,
            |c: &mut StationaryContext| c.physiology.development.biochemical_expression += 0.001,
            |c: &mut StationaryContext| c.physiology.body.sleeping = true,
            |c: &mut StationaryContext| c.immutable_binding[0].1[0] ^= 1,
        ] {
            let mut c = context.clone();
            mutate(&mut c);
            assert!(!context.compatible(&c));
        }
    }
    #[test]
    fn changed_action_target_or_geometry_cannot_reuse_association() {
        let (context, _, _) = fixture();
        let mut c = context.clone();
        c.candidate.action_id = ActionId(999);
        assert!(!context.compatible(&c));
        let mut c = context.clone();
        c.candidate.target.entity = Some(WorldEntityId(999));
        assert!(!context.compatible(&c));
        let mut c = context.clone();
        c.body.pose.translation.x += 0.01;
        assert!(!context.compatible(&c));
        let mut c = context.clone();
        c.subject = AssociationSubjectV1::SelfAction {
            organism: OrganismId(999),
            action: SelfActionV1::Rest,
        };
        assert!(!context.compatible(&c));
    }
    #[test]
    fn fresh_store_is_unknown_and_lookup_does_not_fabricate_reference() {
        let (_, frame, record) = fixture();
        let state = CpuNeedAssociationStateV4::new(record.organism_id(), SelfActionV1::Noop);
        let candidate = frame
            .candidates()
            .iter()
            .find(|c| c.kind == ActionKind::Rest)
            .unwrap();
        let result = state.recall(&frame, candidate, &record).unwrap();
        assert!(result.action_raw_delta.is_none());
        assert!(result.reference_raw_delta.is_none());
        assert_eq!(result.lookup_records, 0);
    }
    #[test]
    fn actual_fresh_candidate_count_and_full_reader_cost_are_reported_without_a_life() {
        let (_, frame, record) = fixture();
        let port = CpuNeedAssociationPortV4::new(
            record.organism_id(),
            record.phenotype(),
            CpuNeedAssociationPlanV4::identity_control(SelfActionV1::Noop),
        )
        .unwrap();
        let prepared = port.prepare(&frame, &record).unwrap();
        let policy = BrainActivityPolicyV1::production_v1();
        let debit = policy
            .cost
            .q24_to_atp_q16_round_half_up(policy.cost.neural_cost_q24(&prepared.work).unwrap())
            .unwrap();
        println!("need_preflight candidates={} reader_rows={} lookup_ops={} neuron_updates={} extra_atp_q16={} available_atp={}",
            frame.candidates().len(),prepared.receipt.candidates.len(),prepared.work.memory_context_ops,
            prepared.work.neuron_updates,debit,record.biochemistry().homeostasis.drives.brain_atp);
        assert!(prepared
            .receipt
            .candidates
            .iter()
            .all(|r| r.native.confidence_product_gated_readout == 0.));
        assert!(debit > 0);
    }
    // Synthetic metadata records below test diagnostic filtering only. They do
    // not create neural envelopes or qualify acquisition from a native life.
    fn metadata_record(context: StationaryContext, outcome_tick: Tick) -> RetainedAssociation {
        RetainedAssociation {
            context,
            sequence: ExperienceSequenceId(1),
            source_frame: PerceptionFrameDigest([0; 4]),
            outcome_tick,
            physical: PhysicalActionOutcome {
                contact: PhysicalContactKind::None,
                target_entity: None,
                displacement: Vec3f::ZERO,
                collision_normal: None,
                energy_cost: NormalizedScalar::ZERO,
            },
            representative_channel: MotorChannel::Posture,
            benign_noop_channels: 1,
            before: [0.2; 11],
            after: [0.2; 11],
            raw_delta: [0.; 11],
            confidence: 1.,
        }
    }
    #[test]
    fn future_source_missing_support_has_explicit_diagnostic() {
        let (context, _, record) = fixture();
        let mut state = CpuNeedAssociationStateV4::new(record.organism_id(), SelfActionV1::Rest);
        state
            .records
            .push(metadata_record(context.clone(), Tick(2)));
        let recalled = state.recall_context(&context, Tick(1)).unwrap();
        assert!(recalled.action_sequence.is_none());
        assert!(recalled.reference_sequence.is_none());
        assert!(recalled
            .action_mismatch_fields
            .iter()
            .any(|s| s.contains("future")));
        assert!(recalled
            .reference_mismatch_fields
            .iter()
            .any(|s| s.contains("future")));
    }
    #[test]
    fn expired_source_missing_support_has_explicit_diagnostic() {
        let (context, _, record) = fixture();
        let mut state = CpuNeedAssociationStateV4::new(record.organism_id(), SelfActionV1::Rest);
        state
            .records
            .push(metadata_record(context.clone(), Tick(1)));
        let recalled = state.recall_context(&context, Tick(MAX_AGE + 2)).unwrap();
        assert!(recalled.action_sequence.is_none());
        assert!(recalled.reference_sequence.is_none());
        assert!(recalled
            .action_mismatch_fields
            .iter()
            .any(|s| s.contains("maximum age")));
        assert!(recalled
            .reference_mismatch_fields
            .iter()
            .any(|s| s.contains("maximum age")));
    }
    #[test]
    fn v4_namespace_admits_exact_targetless_no_locomotion_not_idle_or_approach() {
        let (_, frame, _) = fixture();
        let noop = frame
            .candidates()
            .iter()
            .find(|c| c.action_id == HeadlessActionIds::NO_LOCOMOTION)
            .unwrap();
        assert!(matches!(
            candidate_subject(&frame, noop),
            Ok(AssociationSubjectV1::SelfAction {
                action: SelfActionV1::Noop,
                ..
            })
        ));
        let mut tampered = *noop;
        tampered.action_id = HeadlessActionIds::APPROACH;
        assert!(candidate_subject(&frame, &tampered).is_err());
        tampered = *noop;
        tampered.kind = ActionKind::Idle;
        tampered.action_id = ActionKind::Idle.canonical_id();
        assert!(candidate_subject(&frame, &tampered).is_err());
        tampered = *noop;
        tampered.target = ActionTarget::new(Some(WorldEntityId(999)), None);
        assert!(candidate_subject(&frame, &tampered).is_err());
    }
    #[test]
    fn v4_binding_tariff_is_explicit_and_zero_gain_still_executes_same_work() {
        let (_, frame, record) = fixture();
        let mut plan = CpuNeedAssociationPlanV4::identity_control(SelfActionV1::Noop);
        plan.execution = NativeNeedExecutionV1::ActiveCoordinates;
        let on = CpuNeedAssociationPortV4::new(record.organism_id(), record.phenotype(), plan)
            .unwrap()
            .prepare(&frame, &record)
            .unwrap();
        plan.recipe.gain = 0.;
        let off = CpuNeedAssociationPortV4::new(record.organism_id(), record.phenotype(), plan)
            .unwrap()
            .prepare(&frame, &record)
            .unwrap();
        assert_eq!(on.work, off.work);
        assert_eq!(
            on.receipt.binding_memory_ops,
            binding_memory_ops(&on.receipt.binding_work).unwrap()
        );
        assert!(on.receipt.binding_work.encoded_bytes > 0);
        assert!(on.receipt.context_hash_bytes > 0);
        assert!(
            on.work.memory_context_ops
                >= on.receipt.binding_memory_ops + on.receipt.context_memory_ops
        );
        println!(
            "v4_binding_work {}",
            serde_json::to_string(&on.receipt).unwrap()
        );
    }
}

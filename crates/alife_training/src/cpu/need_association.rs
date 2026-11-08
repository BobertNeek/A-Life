//! Reconstructed protected CPU association port. Comparison is predictive own
//! history advantage, not isolated causal credit or an externally corrected reward.
//! Default foundations, V1 acquired state and legacy replay/checkpoints are unchanged.
use super::need_association_source::*;
use super::CpuHostError;
use alife_core::*;
use alife_world::{HeadlessMotorTransactionReceipt, WorldOrganismRecord};

pub const CPU_NEED_ASSOCIATION_PORT_V3: &str = "cpu-stationary-own-history-advantage-v3";
const CAPACITY: usize = 32;
const MAX_AGE: u64 = 16;
const MAX_PHYSIOLOGY_DRIFT: f32 = 0.125;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
struct StationaryContext {
    subject: AssociationSubjectV1,
    candidate: ActionCandidate,
    body: BodySnapshot,
    slots: Vec<GroundedObjectSlotV1>,
    immutable_binding: Vec<(&'static str, [u64; 4])>,
    physiology: BiochemistryState,
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
            CPU_NEED_ASSOCIATION_PORT_V3,
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
        let immutable_binding = values
            .as_array()
            .ok_or(CpuHostError::State("stationary binding shape"))?
            .iter()
            .zip(LABELS)
            .map(|(value, label)| {
                let mut d = CanonicalDigestBuilder::new(b"ALIFE-STATIONARY-CONTEXT-PART-V1");
                d.write_bytes(label.as_bytes());
                d.write_bytes(&serde_json::to_vec(value)?);
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
pub(super) struct CpuNeedAssociationStateV1 {
    identity: &'static str,
    owner: OrganismId,
    reference: SelfActionV1,
    last_sequence: u64,
    records: Vec<RetainedAssociation>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum CpuNeedObservationStatusV1 {
    Stored,
    Unsupported(&'static str),
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuNeedAssociationSupportV1 {
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
impl CpuNeedAssociationStateV1 {
    pub(super) fn new(owner: OrganismId, reference: SelfActionV1) -> Self {
        Self {
            identity: CPU_NEED_ASSOCIATION_PORT_V3,
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
    ) -> Result<CpuNeedObservationStatusV1, CpuHostError> {
        if patch.header().organism_id != self.owner
            || patch.header().sequence_id.0 <= self.last_sequence
        {
            return Err(CpuHostError::State("association owner or replay mismatch"));
        }
        self.last_sequence = patch.header().sequence_id.0;
        let source = match qualify_native_interval(patch, receipt, before, after) {
            Ok(source) => source,
            Err(reason) => return Ok(CpuNeedObservationStatusV1::Unsupported(reason.label())),
        };
        let pre = patch.pre_action();
        let evidence = patch.decision().neural_evidence()?;
        let candidate = &pre.perception().candidates()[usize::from(evidence.candidate_index)];
        let context = StationaryContext::from_native(pre.perception(), candidate, before)?;
        if context.subject != source.subject
            || source.representative_physical.displacement != Vec3f::ZERO
        {
            return Ok(CpuNeedObservationStatusV1::Unsupported(
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
        Ok(CpuNeedObservationStatusV1::Stored)
    }
    #[cfg(test)]
    fn recall(
        &self,
        frame: &PerceptionFrame,
        candidate: &ActionCandidate,
        record: &WorldOrganismRecord,
    ) -> Result<CpuNeedAssociationSupportV1, CpuHostError> {
        let context = StationaryContext::from_native(frame, candidate, record)?;
        self.recall_context(&context, frame.tick())
    }
    fn recall_context(
        &self,
        context: &StationaryContext,
        tick: Tick,
    ) -> Result<CpuNeedAssociationSupportV1, CpuHostError> {
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
        Ok(CpuNeedAssociationSupportV1 {
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
pub struct CpuNeedAssociationPlanV1 {
    pub recipe: NativeNeedAssociationRecipeV1,
    pub reference: SelfActionV1,
    pub execution: NativeNeedExecutionV1,
}
impl CpuNeedAssociationPlanV1 {
    pub fn identity_control(reference: SelfActionV1) -> Self {
        Self {
            recipe: NativeNeedAssociationRecipeV1::reference_species22(),
            reference,
            execution: NativeNeedExecutionV1::FullScratchReference,
        }
    }
}
#[derive(Debug, Clone)]
pub(super) struct CpuNeedAssociationPortV1 {
    pub(super) state: CpuNeedAssociationStateV1,
    pub(super) circuit: NativeNeedAssociationCircuitV1,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuNeedCandidateReceiptV1 {
    pub candidate_index: u16,
    pub action_id: ActionId,
    pub family: CandidateActionFamily,
    pub support: CpuNeedAssociationSupportV1,
    pub native: NativeNeedAssociationOutputV1,
    pub base_logit: f32,
    pub final_logit: f32,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuNeedDecisionReceiptV1 {
    pub port_identity: &'static str,
    pub organism: OrganismId,
    pub tick: Tick,
    pub candidates: Vec<CpuNeedCandidateReceiptV1>,
    pub source_observation: CpuNeedObservationStatusV1,
    pub source_audit: Option<CpuNeedSourceAuditV3>,
}
#[derive(Debug, Clone)]
pub(super) struct CpuNeedPreparedV1 {
    pub(super) receipt: CpuNeedDecisionReceiptV1,
    pub(super) work: BrainWorkCounters,
    pub(super) dendritic_ops: u64,
}
impl CpuNeedAssociationPortV1 {
    pub(super) fn new(
        owner: OrganismId,
        plan: CpuNeedAssociationPlanV1,
    ) -> Result<Self, CpuHostError> {
        Ok(Self {
            state: CpuNeedAssociationStateV1::new(owner, plan.reference),
            circuit: NativeNeedAssociationCircuitV1::compile_with_execution(
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
    ) -> Result<CpuNeedPreparedV1, CpuHostError> {
        let mut candidates = Vec::new();
        let mut work = BrainWorkCounters::default();
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
                common_context = Some(context.clone());
                context
            };
            let support = self.state.recall_context(&context, frame.tick())?;
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
                    frame.homeostasis().drives.to_array(),
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
            candidates.push(CpuNeedCandidateReceiptV1 {
                candidate_index: candidate.candidate_index,
                action_id: candidate.action_id,
                family: candidate.family,
                support,
                native,
                base_logit: 0.,
                final_logit: 0.,
            });
        }
        Ok(CpuNeedPreparedV1 {
            receipt: CpuNeedDecisionReceiptV1 {
                port_identity: CPU_NEED_ASSOCIATION_PORT_V3,
                organism: frame.organism_id(),
                tick: frame.tick(),
                candidates,
                source_observation: CpuNeedObservationStatusV1::Unsupported("not_observed"),
                source_audit: None,
            },
            work,
            dendritic_ops,
        })
    }
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
impl CpuNeedPreparedV1 {
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
pub struct CpuNeedAssociationStepV1 {
    pub tick_before: Tick,
    pub tick_after: Tick,
    pub decisions: Vec<CpuNeedAssociationDecisionV1>,
    pub sleeping_organisms: usize,
    pub sleep_consolidations: usize,
}
pub struct CpuNeedAssociationDecisionV1 {
    pub organism_id: OrganismId,
    pub action: crate::PpoJointAction,
    pub patch: ExperiencePatch,
    pub brain_work: BrainWorkCounters,
    pub biological_reward: f32,
    pub port: CpuNeedDecisionReceiptV1,
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
        let state = CpuNeedAssociationStateV1::new(record.organism_id(), SelfActionV1::Noop);
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
        let port = CpuNeedAssociationPortV1::new(
            record.organism_id(),
            CpuNeedAssociationPlanV1::identity_control(SelfActionV1::Noop),
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
        let mut state = CpuNeedAssociationStateV1::new(record.organism_id(), SelfActionV1::Rest);
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
        let mut state = CpuNeedAssociationStateV1::new(record.organism_id(), SelfActionV1::Rest);
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
}

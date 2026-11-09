//! Fresh optional-vocal host identity and a shared-loop policy strategy.
//! No old host conversion, reset, imported checkpoint, or gradient entry point.
use super::*;
use crate::cpu::{
    OptionalVocalActionV1, OptionalVocalScoresV1, OptionalVocalSourceV1, OPTIONAL_VOCAL_POLICY_V1,
};
const OPTIONAL_MAGIC: &[u8] = b"ALIFE-OPTIONAL-VOCAL-M2-HOST\0V1\0IDLE-QUIET\0SOURCE-PAYLOAD\0SYNAPTIC-CLOSED\0PROFILE5\0BITS1\0";
const OPTIONAL_LINEAGE: &str = "fresh-optional-vocal-m2-host-v1/class5/no-historical-import";
const OPTIONAL_RESIDENT: &str = "optional-vocal-m2-resident-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::cpu::host) struct OptionalVocalResidentV1 {
    schema: String,
    last: Option<OptionalVocalSelectionV1>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OptionalVocalSelectionV1 {
    scores: OptionalVocalScoresV1,
    action: OptionalVocalActionV1,
}
impl OptionalVocalResidentV1 {
    pub(super) fn fresh() -> Self {
        Self {
            schema: OPTIONAL_RESIDENT.into(),
            last: None,
        }
    }
    pub(super) fn is_fresh(&self) -> bool {
        self.schema == OPTIONAL_RESIDENT && self.last.is_none()
    }
}
impl GroupedResidentV1 {
    pub(in crate::cpu::host) fn validate_policy<P: GroupedPolicyStrategy>(
        &self,
    ) -> Result<(), CpuHostError> {
        if self.optional_vocal.is_some() != P::OPTIONAL
            || self
                .optional_vocal
                .as_ref()
                .is_some_and(|v| v.schema != OPTIONAL_RESIDENT || self.last_action.is_some())
        {
            return Err(CpuHostError::State("closed grouped policy owner mismatch"));
        }
        if let Some(v) = &self.optional_vocal {
            if v.last.is_some() != self.motor.last_receipt.is_some() {
                return Err(CpuHostError::State("optional Vocal incomplete owner"));
            }
        }
        Ok(())
    }
    pub(super) fn sampling_cursor(&self) -> Option<(u32, u32)> {
        match &self.optional_vocal {
            None => self.last_action.as_ref().map(|a| (a.seed, a.counter)),
            Some(v) => v
                .last
                .as_ref()
                .map(|s| (s.action.seed(), s.action.counter())),
        }
    }
    pub(super) fn physical_selection(
        &self,
    ) -> Result<(u16, [Option<u16>; 6], Option<u8>), CpuHostError> {
        let native = &self
            .motor
            .last_receipt
            .as_ref()
            .ok_or(CpuHostError::State("missing native proposal"))?
            .native
            .scores;
        if let Some(v) = &self.optional_vocal {
            self.validate_policy::<OptionalVocalPolicy>()?;
            let last = v
                .last
                .as_ref()
                .ok_or(CpuHostError::State("missing optional Vocal sample"))?;
            last.scores.validate_native(native)?;
            let (i, m) = last.scores.physical_selection(&last.action)?;
            Ok((i, m, last.action.operation()))
        } else {
            let a = self
                .last_action
                .as_ref()
                .ok_or(CpuHostError::State("missing legacy grouped sample"))?;
            let (i, m) = native.physical_selection(a)?;
            Ok((i, m, a.operation))
        }
    }
    pub(super) fn validate_optional_source(
        &self,
        founder: &CpuFoundationTrainer,
        neural: &CpuOrganismState,
    ) -> Result<(), CpuHostError> {
        let Some(v) = &self.optional_vocal else {
            return Ok(());
        };
        self.validate_policy::<OptionalVocalPolicy>()?;
        let Some(last) = &v.last else {
            return Ok(());
        };
        let m = self
            .motor
            .last_receipt
            .as_ref()
            .ok_or(CpuHostError::State("missing optional native output"))?;
        let checkpoint = neural.checkpoint()?;
        let offsets = neural.effective_offsets(founder.phenotype())?;
        let actual = OptionalVocalSourceV1::from_forward(
            checkpoint.organism_id,
            m.tick,
            founder.phenotype().phenotype_hash(),
            m.frame_digest,
            checkpoint.dispatch_generation,
            checkpoint.active_activation_side,
            &checkpoint.neural.activations,
            &offsets,
            decode_cpu_speech(founder, &checkpoint.neural.activations, &offsets)?,
        )?;
        last.scores.validate_source(&actual)?;
        last.scores.validate_native(&m.native.scores)?;
        Ok(())
    }
}

type Selection<A> = (A, u16, [Option<u16>; 6], Option<SpeechMotorPayload>);
pub(in crate::cpu::host) trait GroupedPolicyStrategy {
    type Action: Clone;
    const OPTIONAL: bool;
    const MAGIC: &'static [u8];
    const LINEAGE: &'static str;
    const POLICY: &'static str;
    #[allow(clippy::too_many_arguments)]
    fn select(
        founder: &CpuFoundationTrainer,
        resident: &mut Resident,
        frame: &PerceptionFrame,
        output: &CpuDecisionOutput,
        seed: u32,
        counter: u32,
        temperature: f64,
    ) -> Result<Selection<Self::Action>, CpuHostError>;
    fn stage(
        resident: &mut Resident,
        p: &BrainPhenotype,
        action: &Self::Action,
        binding: CpuDecisionBinding,
    ) -> Result<(), CpuHostError>;
    fn close(
        neural: &mut CpuOrganismState,
        founder: &CpuFoundationTrainer,
        patch: &ExperiencePatch,
        receptors: &NeuralReceptorFrame,
    ) -> Result<(), CpuHostError>;
}
pub(in crate::cpu::host) struct LegacyGroupedPolicy;
pub(in crate::cpu::host) struct OptionalVocalPolicy;
impl GroupedPolicyStrategy for LegacyGroupedPolicy {
    type Action = RecoveryGroupJointActionV2;
    const OPTIONAL: bool = false;
    const MAGIC: &'static [u8] = super::MAGIC;
    const LINEAGE: &'static str = super::LINEAGE;
    const POLICY: &'static str = super::POLICY;
    fn select(
        founder: &CpuFoundationTrainer,
        r: &mut Resident,
        _frame: &PerceptionFrame,
        output: &CpuDecisionOutput,
        seed: u32,
        counter: u32,
        t: f64,
    ) -> Result<Selection<Self::Action>, CpuHostError> {
        let state = r
            .grouped
            .as_mut()
            .ok_or(CpuHostError::State("missing grouped controller"))?;
        state.validate_policy::<Self>()?;
        let scores = &state
            .motor
            .last_receipt
            .as_ref()
            .ok_or(CpuHostError::State("missing grouped scores"))?
            .native
            .scores;
        let action = scores.sample(seed, counter, t)?;
        let (index, motors) = scores.physical_selection(&action)?;
        state.last_action = Some(action.clone());
        let speech = if motors[3].is_some() {
            decode_cpu_speech(
                founder,
                &r.neural.neural_state().activations,
                &output.replay_tick.effective_weight_offsets,
            )?
        } else {
            None
        };
        Ok((action, index, motors, speech))
    }
    fn stage(
        r: &mut Resident,
        p: &BrainPhenotype,
        a: &Self::Action,
        b: CpuDecisionBinding,
    ) -> Result<(), CpuHostError> {
        let state = r
            .grouped
            .as_ref()
            .ok_or(CpuHostError::State("missing grouped controller"))?;
        state.validate_policy::<Self>()?;
        let scores = &state
            .motor
            .last_receipt
            .as_ref()
            .ok_or(CpuHostError::State("missing grouped scores"))?
            .native
            .scores;
        Ok(r.neural.stage_grouped_closed_selection(p, a, scores, b)?)
    }
    fn close(
        n: &mut CpuOrganismState,
        f: &CpuFoundationTrainer,
        p: &ExperiencePatch,
        r: &NeuralReceptorFrame,
    ) -> Result<(), CpuHostError> {
        Ok(n.close_grouped_outcome(f, p, r)?)
    }
}
impl GroupedPolicyStrategy for OptionalVocalPolicy {
    type Action = OptionalVocalActionV1;
    const OPTIONAL: bool = true;
    const MAGIC: &'static [u8] = OPTIONAL_MAGIC;
    const LINEAGE: &'static str = OPTIONAL_LINEAGE;
    const POLICY: &'static str = OPTIONAL_VOCAL_POLICY_V1;
    fn select(
        founder: &CpuFoundationTrainer,
        r: &mut Resident,
        frame: &PerceptionFrame,
        _output: &CpuDecisionOutput,
        seed: u32,
        counter: u32,
        t: f64,
    ) -> Result<Selection<Self::Action>, CpuHostError> {
        let source = r.neural.optional_vocal_source()?.clone();
        let state = r
            .grouped
            .as_mut()
            .ok_or(CpuHostError::State("missing optional controller"))?;
        // During the new forward native generation is one ahead of last sample;
        // use explicit owner checks, not the completed-boundary validator.
        if state.last_action.is_some()
            || state
                .optional_vocal
                .as_ref()
                .is_none_or(|v| v.schema != OPTIONAL_RESIDENT)
        {
            return Err(CpuHostError::State("optional sampler owner mismatch"));
        }
        let native = &state
            .motor
            .last_receipt
            .as_ref()
            .ok_or(CpuHostError::State("missing optional scores"))?
            .native
            .scores;
        let scores = OptionalVocalScoresV1::bind(founder.phenotype(), frame, native, source)?;
        let action = scores.sample(seed, counter, t)?;
        let (index, motors) = scores.physical_selection(&action)?;
        let speech = if motors[3].is_some() {
            scores.source().speech()?
        } else {
            None
        };
        state.optional_vocal.as_mut().unwrap().last = Some(OptionalVocalSelectionV1 {
            scores,
            action: action.clone(),
        });
        Ok((action, index, motors, speech))
    }
    fn stage(
        r: &mut Resident,
        p: &BrainPhenotype,
        a: &Self::Action,
        b: CpuDecisionBinding,
    ) -> Result<(), CpuHostError> {
        let state = r
            .grouped
            .as_ref()
            .ok_or(CpuHostError::State("missing optional controller"))?;
        state.validate_policy::<Self>()?;
        let last = state
            .optional_vocal
            .as_ref()
            .and_then(|v| v.last.as_ref())
            .ok_or(CpuHostError::State("missing optional sample"))?;
        if last.action != *a {
            return Err(CpuHostError::State("optional staged sample mismatch"));
        }
        last.scores
            .validate_native(&state.motor.last_receipt.as_ref().unwrap().native.scores)?;
        Ok(r.neural
            .stage_optional_vocal_closed_selection(p, a, &last.scores, b)?)
    }
    fn close(
        n: &mut CpuOrganismState,
        f: &CpuFoundationTrainer,
        p: &ExperiencePatch,
        r: &NeuralReceptorFrame,
    ) -> Result<(), CpuHostError> {
        Ok(n.close_optional_vocal_outcome(f, p, r)?)
    }
}

pub type CpuOptionalVocalM2HostStepV1 = CpuGroupedM2HostStepV1<OptionalVocalActionV1>;
pub type CpuOptionalVocalM2HostDecisionV1 = CpuGroupedM2HostDecisionV1<OptionalVocalActionV1>;
pub struct CpuOptionalVocalM2HostV1 {
    inner: CpuGroupedM2HostV1,
}
#[derive(Debug, Clone)]
pub struct CpuOptionalVocalM2HostCheckpointV1 {
    payload: GroupedPayloadV1,
}
impl CpuOptionalVocalM2HostCheckpointV1 {
    pub fn encode(&self) -> Result<Vec<u8>, CpuHostError> {
        let mut bytes = OPTIONAL_MAGIC.to_vec();
        bytes.extend(serde_json::to_vec(&self.payload)?);
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, CpuHostError> {
        let body = bytes
            .strip_prefix(OPTIONAL_MAGIC)
            .ok_or(CpuHostError::State("not fresh optional Vocal M2 envelope"))?;
        let metadata: serde_json::Value = serde_json::from_slice(body)?;
        // Identity is inspected before decoding source or private acquired state.
        if metadata["schema_version"] != 1
            || metadata["source_lineage"] != OPTIONAL_LINEAGE
            || metadata["policy"] != OPTIONAL_VOCAL_POLICY_V1
            || metadata["runtime_profile_raw"] != 5
            || metadata["selector_tag"] != 12
            || metadata["decoder_stride"] != 55
            || metadata["recipe_digest"] != RECOVERY_GROUP_CONFIDENCE_RECIPE_SHA256
            || metadata["canonical"]["schema_version"] != OPTIONAL_VOCAL_CANONICAL_SCHEMA
            || !metadata["canonical"]["residents"]
                .as_array()
                .is_some_and(|rows| {
                    !rows.is_empty()
                        && rows.iter().all(|r| {
                            r["neural"]["schema_version"]
                                == crate::cpu::organism::OPTIONAL_VOCAL_NEURAL_SCHEMA
                        })
                })
        {
            return Err(CpuHostError::State("optional Vocal outer policy mismatch"));
        }
        let mut payload: GroupedPayloadV1 = serde_json::from_value(metadata.clone())?;
        payload.restore_sampler_bits()?;
        for state in payload.controllers.values() {
            state.validate_policy::<OptionalVocalPolicy>()?;
        }
        if serde_json::from_slice::<serde_json::Value>(&serde_json::to_vec(&payload)?)? != metadata
        {
            return Err(CpuHostError::State(
                "optional Vocal noncanonical nested state",
            ));
        }
        Ok(Self { payload })
    }
}
impl CpuOptionalVocalM2HostV1 {
    pub fn new_prepared<T, E: std::fmt::Display>(
        game: CanonicalNewGame,
        founder: CpuFoundationTrainer,
        source_inputs: PhenotypeCompilerInputs,
        sampling_seed: u32,
        mode: MediationModeV1,
        prepare: impl FnOnce(&mut HeadlessWorld) -> Result<T, E>,
    ) -> Result<(Self, T), CpuHostError> {
        let (inner, value) = CpuGroupedM2HostV1::new_prepared_policy::<OptionalVocalPolicy, T, E>(
            game,
            founder,
            source_inputs,
            sampling_seed,
            mode,
            prepare,
        )?;
        Ok((Self { inner }, value))
    }
    pub fn world(&self) -> &HeadlessWorld {
        self.inner.world()
    }
    /// Read-only optional sampling evidence for this organism's last decision.
    /// The shared motor receipt contains native proposals, not this policy's
    /// selected-action distribution. Old grouped/PPO evaluation is not valid here.
    pub fn last_selection_scores(&self, organism: OrganismId) -> Option<&OptionalVocalScoresV1> {
        self.inner
            .host
            .residents
            .get(&organism.raw())?
            .grouped
            .as_ref()?
            .optional_vocal
            .as_ref()?
            .last
            .as_ref()
            .map(|s| &s.scores)
    }
    pub fn sleep_state(&self, id: OrganismId) -> Option<alife_core::sleep::SleepState> {
        self.inner.sleep_state(id)
    }
    pub fn checkpoint(&self) -> Result<CpuOptionalVocalM2HostCheckpointV1, CpuHostError> {
        Ok(CpuOptionalVocalM2HostCheckpointV1 {
            payload: self.inner.checkpoint_payload::<OptionalVocalPolicy>()?,
        })
    }
    pub fn restore(
        c: CpuOptionalVocalM2HostCheckpointV1,
        expected_source: &PhenotypeCompilerInputs,
        mode: MediationModeV1,
    ) -> Result<Self, CpuHostError> {
        let wire = c.encode()?;
        let inner = CpuGroupedM2HostV1::restore_payload::<OptionalVocalPolicy>(
            c.payload,
            expected_source,
            mode,
            wire,
        )?;
        Ok(Self { inner })
    }
    pub fn step_research_observed(
        &mut self,
        temperature: f32,
    ) -> Result<CpuGroupedM2ResearchStepV1<OptionalVocalActionV1>, CpuHostError> {
        self.inner
            .host
            .grouped_policy_step_observed::<OptionalVocalPolicy>(&self.inner.founder, temperature)
    }
    pub fn step(&mut self, temperature: f32) -> Result<CpuOptionalVocalM2HostStepV1, CpuHostError> {
        self.inner
            .host
            .grouped_policy_step::<OptionalVocalPolicy>(&self.inner.founder, temperature)
    }
    pub fn actor_optimization(&mut self) -> Result<(), CpuHostError> {
        Err(CpuHostError::State(
            "optional Vocal synaptic learning is closed",
        ))
    }
    pub fn waking_synaptic_plasticity(&mut self) -> Result<(), CpuHostError> {
        Err(CpuHostError::State(
            "optional Vocal synaptic learning is closed",
        ))
    }
    pub fn synaptic_sleep_consolidation(&mut self) -> Result<(), CpuHostError> {
        Err(CpuHostError::State(
            "optional Vocal synaptic learning is closed",
        ))
    }
}

#[cfg(test)]
#[path = "optional_vocal_host_tests_v2.rs"]
mod tests;

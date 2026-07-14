//! v0 scaffold: episodic associative memory as expectancy bias, not action replay.

use serde::{Deserialize, Serialize};

use crate::{
    validate_finite_slice, ActionId, ActionKind, Confidence, DriveDelta, ExperiencePatch,
    ExperienceSequenceId, MemoryId, NormalizedScalar, OrganismId, PhysicalContactKind,
    PreActionSnapshot, ScaffoldContractError, SignedValence, Tick, Validate,
};

pub const MEMORY_FEATURE_VECTOR_MAX_LEN: usize = 64;
pub const MEMORY_BANK_MAX_CAPACITY: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryOutcomeSummary {
    pub success_likelihood: NormalizedScalar,
    pub contact_likelihood: NormalizedScalar,
    pub prediction_error: NormalizedScalar,
    pub pain_delta: NormalizedScalar,
    pub energy_delta: SignedValence,
}

impl MemoryOutcomeSummary {
    pub const fn neutral() -> Self {
        Self {
            success_likelihood: NormalizedScalar(0.0),
            contact_likelihood: NormalizedScalar(0.0),
            prediction_error: NormalizedScalar(0.0),
            pain_delta: NormalizedScalar(0.0),
            energy_delta: SignedValence(0.0),
        }
    }
}

impl Validate for MemoryOutcomeSummary {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        NormalizedScalar::new(self.success_likelihood.raw())?;
        NormalizedScalar::new(self.contact_likelihood.raw())?;
        NormalizedScalar::new(self.prediction_error.raw())?;
        NormalizedScalar::new(self.pain_delta.raw())?;
        SignedValence::new(self.energy_delta.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryExpectancy {
    pub expected_valence: SignedValence,
    pub predicted_drive_delta: DriveDelta,
    pub predicted_sensory_outcome: MemoryOutcomeSummary,
    pub affordance_bias: NormalizedScalar,
    pub danger_bias: NormalizedScalar,
    pub safety_bias: NormalizedScalar,
    pub social_trust_bias: NormalizedScalar,
    pub social_fear_bias: NormalizedScalar,
    pub novelty_bias: NormalizedScalar,
    pub curiosity_bias: NormalizedScalar,
    pub confidence: Confidence,
    pub source_memory_ids: Vec<MemoryId>,
}

impl MemoryExpectancy {
    pub fn neutral(confidence: Confidence) -> Result<Self, ScaffoldContractError> {
        Confidence::new(confidence.raw())?;
        Ok(Self {
            expected_valence: SignedValence(0.0),
            predicted_drive_delta: DriveDelta::zero(),
            predicted_sensory_outcome: MemoryOutcomeSummary::neutral(),
            affordance_bias: NormalizedScalar(0.0),
            danger_bias: NormalizedScalar(0.0),
            safety_bias: NormalizedScalar(0.0),
            social_trust_bias: NormalizedScalar(0.0),
            social_fear_bias: NormalizedScalar(0.0),
            novelty_bias: NormalizedScalar(0.0),
            curiosity_bias: NormalizedScalar(0.0),
            confidence,
            source_memory_ids: Vec::new(),
        })
    }
}

impl Validate for MemoryExpectancy {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        SignedValence::new(self.expected_valence.raw())?;
        self.predicted_drive_delta.validate_contract()?;
        self.predicted_sensory_outcome.validate_contract()?;
        NormalizedScalar::new(self.affordance_bias.raw())?;
        NormalizedScalar::new(self.danger_bias.raw())?;
        NormalizedScalar::new(self.safety_bias.raw())?;
        NormalizedScalar::new(self.social_trust_bias.raw())?;
        NormalizedScalar::new(self.social_fear_bias.raw())?;
        NormalizedScalar::new(self.novelty_bias.raw())?;
        NormalizedScalar::new(self.curiosity_bias.raw())?;
        Confidence::new(self.confidence.raw())?;
        for memory_id in &self.source_memory_ids {
            memory_id.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub features: Vec<f32>,
}

impl MemoryQuery {
    pub fn new(
        organism_id: OrganismId,
        tick: Tick,
        features: Vec<f32>,
    ) -> Result<Self, ScaffoldContractError> {
        let query = Self {
            organism_id,
            tick,
            features,
        };
        query.validate_contract()?;
        Ok(query)
    }

    pub fn from_pre_action(
        pre_action: &PreActionSnapshot,
        max_feature_len: usize,
    ) -> Result<Self, ScaffoldContractError> {
        pre_action.validate_contract()?;
        validate_feature_cap(max_feature_len)?;
        Self::new(
            pre_action.organism_id,
            pre_action.tick,
            feature_vector_from_pre_action(pre_action, max_feature_len)?,
        )
    }
}

impl Validate for MemoryQuery {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        validate_feature_vector(&self.features)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryMatch {
    pub memory_id: MemoryId,
    pub score: f32,
    pub source_tick: Tick,
}

impl Validate for MemoryMatch {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.memory_id.validate()?;
        crate::validate_finite(self.score)?;
        if (0.0..=1.0).contains(&self.score) {
            Ok(())
        } else {
            Err(ScaffoldContractError::ScalarOutOfRange)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryBankConfig {
    pub capacity: usize,
    pub max_feature_len: usize,
    pub max_match_count: usize,
    pub min_match_score: f32,
    pub empty_confidence: Confidence,
}

impl MemoryBankConfig {
    pub fn new(
        capacity: usize,
        max_feature_len: usize,
        max_match_count: usize,
        min_match_score: f32,
        empty_confidence: Confidence,
    ) -> Result<Self, ScaffoldContractError> {
        let config = Self {
            capacity,
            max_feature_len,
            max_match_count,
            min_match_score,
            empty_confidence,
        };
        config.validate_contract()?;
        Ok(config)
    }
}

impl Validate for MemoryBankConfig {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.capacity == 0
            || self.capacity > MEMORY_BANK_MAX_CAPACITY
            || self.max_match_count == 0
            || self.max_match_count > self.capacity
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        validate_feature_cap(self.max_feature_len)?;
        crate::validate_finite(self.min_match_score)?;
        if !(0.0..=1.0).contains(&self.min_match_score) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Confidence::new(self.empty_confidence.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub memory_id: MemoryId,
    pub organism_id: OrganismId,
    pub source_sequence_id: ExperienceSequenceId,
    pub source_tick: Tick,
    pub features: Vec<f32>,
    pub expected_valence: SignedValence,
    pub predicted_drive_delta: DriveDelta,
    pub outcome_summary: MemoryOutcomeSummary,
    pub affordance_bias: NormalizedScalar,
    pub danger_bias: NormalizedScalar,
    pub safety_bias: NormalizedScalar,
    pub social_trust_bias: NormalizedScalar,
    pub social_fear_bias: NormalizedScalar,
    pub novelty_bias: NormalizedScalar,
    pub curiosity_bias: NormalizedScalar,
    pub selected_action_id: Option<ActionId>,
    pub selected_action_kind: Option<ActionKind>,
}

impl MemoryRecord {
    pub fn from_sealed_patch(
        memory_id: MemoryId,
        patch: &ExperiencePatch,
        max_feature_len: usize,
    ) -> Result<Self, ScaffoldContractError> {
        memory_id.validate()?;
        patch.validate_contract()?;
        validate_feature_cap(max_feature_len)?;

        let pre_action = patch.pre_action();
        let decision = patch.decision();
        let outcome = patch.outcome();
        let contact_likelihood = match outcome.physical.contact {
            PhysicalContactKind::None => 0.0,
            _ => 1.0,
        };
        let positive_reward = outcome.reward_valence.raw().max(0.0);
        let negative_reward = (-outcome.reward_valence.raw()).max(0.0);
        let social_bias = social_biases(pre_action);
        let record = Self {
            memory_id,
            organism_id: pre_action.organism_id,
            source_sequence_id: pre_action.sequence_id,
            source_tick: pre_action.tick,
            features: feature_vector_from_pre_action(pre_action, max_feature_len)?,
            expected_valence: outcome.reward_valence,
            predicted_drive_delta: outcome.homeostatic_delta.drives,
            outcome_summary: MemoryOutcomeSummary {
                success_likelihood: NormalizedScalar(if outcome.success { 1.0 } else { 0.0 }),
                contact_likelihood: NormalizedScalar(contact_likelihood),
                prediction_error: outcome.prediction_error,
                pain_delta: outcome.pain_delta,
                energy_delta: outcome.energy_delta,
            },
            affordance_bias: NormalizedScalar::new(max_affordance(pre_action))?,
            danger_bias: NormalizedScalar::new(
                negative_reward
                    .max(outcome.pain_delta.raw())
                    .max(outcome.frustration_delta.raw()),
            )?,
            safety_bias: NormalizedScalar::new(if outcome.success {
                positive_reward.max(0.25)
            } else {
                0.0
            })?,
            social_trust_bias: NormalizedScalar::new(social_bias.0)?,
            social_fear_bias: NormalizedScalar::new(social_bias.1)?,
            novelty_bias: pre_action.sensory().channels.novelty_signal,
            curiosity_bias: NormalizedScalar::new(pre_action.homeostasis().drives.curiosity)?,
            selected_action_id: Some(decision.selected_action.action_id),
            selected_action_kind: Some(decision.selected_action.kind),
        };
        record.validate_contract()?;
        Ok(record)
    }
}

impl Validate for MemoryRecord {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.memory_id.validate()?;
        self.organism_id.validate()?;
        self.source_sequence_id.validate()?;
        validate_feature_vector(&self.features)?;
        SignedValence::new(self.expected_valence.raw())?;
        self.predicted_drive_delta.validate_contract()?;
        self.outcome_summary.validate_contract()?;
        NormalizedScalar::new(self.affordance_bias.raw())?;
        NormalizedScalar::new(self.danger_bias.raw())?;
        NormalizedScalar::new(self.safety_bias.raw())?;
        NormalizedScalar::new(self.social_trust_bias.raw())?;
        NormalizedScalar::new(self.social_fear_bias.raw())?;
        NormalizedScalar::new(self.novelty_bias.raw())?;
        NormalizedScalar::new(self.curiosity_bias.raw())?;
        if let Some(action_id) = self.selected_action_id {
            action_id.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryBank {
    config: MemoryBankConfig,
    records: Vec<Option<MemoryRecord>>,
    next_write_index: usize,
    len: usize,
    next_memory_id: u64,
    last_inserted_ticks: Vec<(OrganismId, Tick)>,
}

impl MemoryBank {
    pub fn new(config: MemoryBankConfig) -> Result<Self, ScaffoldContractError> {
        config.validate_contract()?;
        Ok(Self {
            records: vec![None; config.capacity],
            config,
            next_write_index: 0,
            len: 0,
            next_memory_id: 1,
            last_inserted_ticks: Vec::new(),
        })
    }

    pub const fn capacity(&self) -> usize {
        self.config.capacity
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn insert_from_patch(
        &mut self,
        patch: &ExperiencePatch,
    ) -> Result<MemoryId, ScaffoldContractError> {
        patch.validate_contract()?;
        let organism_id = patch.pre_action().organism_id;
        let source_tick = patch.pre_action().tick;
        self.validate_monotonic_insert(organism_id, source_tick)?;

        let memory_id = MemoryId(self.next_memory_id);
        let record =
            MemoryRecord::from_sealed_patch(memory_id, patch, self.config.max_feature_len)?;
        self.insert_record(record)?;
        self.next_memory_id = self
            .next_memory_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        self.record_last_tick(organism_id, source_tick);
        Ok(memory_id)
    }

    pub fn insert_record(
        &mut self,
        record: MemoryRecord,
    ) -> Result<MemoryId, ScaffoldContractError> {
        record.validate_contract()?;
        if record.features.len() > self.config.max_feature_len {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let memory_id = record.memory_id;
        self.records[self.next_write_index] = Some(record);
        self.next_write_index = (self.next_write_index + 1) % self.config.capacity;
        self.len = (self.len + 1).min(self.config.capacity);
        Ok(memory_id)
    }

    pub fn query(&self, query: &MemoryQuery) -> Result<Vec<MemoryMatch>, ScaffoldContractError> {
        query.validate_contract()?;
        if query.features.len() > self.config.max_feature_len {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }

        let mut matches = Vec::new();
        for record in self.records_chronological() {
            if record.organism_id != query.organism_id {
                continue;
            }
            let score = normalized_dot(&query.features, &record.features)?;
            if score >= self.config.min_match_score {
                matches.push(MemoryMatch {
                    memory_id: record.memory_id,
                    score,
                    source_tick: record.source_tick,
                });
            }
        }

        matches.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.memory_id.raw().cmp(&b.memory_id.raw()))
        });
        matches.truncate(self.config.max_match_count);
        for memory_match in &matches {
            memory_match.validate_contract()?;
        }
        Ok(matches)
    }

    pub fn recall(&self, query: &MemoryQuery) -> Result<MemoryExpectancy, ScaffoldContractError> {
        let matches = self.query(query)?;
        if matches.is_empty() {
            return MemoryExpectancy::neutral(self.config.empty_confidence);
        }

        let total_weight: f32 = matches.iter().map(|memory_match| memory_match.score).sum();
        if total_weight <= 0.0 {
            return MemoryExpectancy::neutral(self.config.empty_confidence);
        }

        let mut expected_valence = 0.0;
        let mut drive_delta = DriveDelta::zero();
        let mut outcome = WeightedOutcomeSummary::default();
        let mut affordance_bias = 0.0;
        let mut danger_bias = 0.0;
        let mut safety_bias = 0.0;
        let mut social_trust_bias = 0.0;
        let mut social_fear_bias = 0.0;
        let mut novelty_bias = 0.0;
        let mut curiosity_bias = 0.0;
        let mut source_memory_ids = Vec::with_capacity(matches.len());

        for memory_match in &matches {
            let Some(record) = self.record_by_id(memory_match.memory_id) else {
                return Err(ScaffoldContractError::InvalidId);
            };
            let weight = memory_match.score / total_weight;
            expected_valence += record.expected_valence.raw() * weight;
            drive_delta = weighted_drive_delta(drive_delta, record.predicted_drive_delta, weight);
            outcome.add(record.outcome_summary, weight);
            affordance_bias += record.affordance_bias.raw() * weight;
            danger_bias += record.danger_bias.raw() * weight;
            safety_bias += record.safety_bias.raw() * weight;
            social_trust_bias += record.social_trust_bias.raw() * weight;
            social_fear_bias += record.social_fear_bias.raw() * weight;
            novelty_bias += record.novelty_bias.raw() * weight;
            curiosity_bias += record.curiosity_bias.raw() * weight;
            source_memory_ids.push(record.memory_id);
        }

        let average_score = total_weight / matches.len() as f32;
        let empty_confidence = self.config.empty_confidence.raw();
        let confidence = empty_confidence + average_score * (1.0 - empty_confidence);
        let expectancy = MemoryExpectancy {
            expected_valence: SignedValence::new(expected_valence.clamp(-1.0, 1.0))?,
            predicted_drive_delta: drive_delta,
            predicted_sensory_outcome: outcome.finish()?,
            affordance_bias: NormalizedScalar::new(affordance_bias.clamp(0.0, 1.0))?,
            danger_bias: NormalizedScalar::new(danger_bias.clamp(0.0, 1.0))?,
            safety_bias: NormalizedScalar::new(safety_bias.clamp(0.0, 1.0))?,
            social_trust_bias: NormalizedScalar::new(social_trust_bias.clamp(0.0, 1.0))?,
            social_fear_bias: NormalizedScalar::new(social_fear_bias.clamp(0.0, 1.0))?,
            novelty_bias: NormalizedScalar::new(novelty_bias.clamp(0.0, 1.0))?,
            curiosity_bias: NormalizedScalar::new(curiosity_bias.clamp(0.0, 1.0))?,
            confidence: Confidence::new(confidence.clamp(0.0, 1.0))?,
            source_memory_ids,
        };
        expectancy.validate_contract()?;
        Ok(expectancy)
    }

    pub fn records_chronological(&self) -> Vec<&MemoryRecord> {
        let mut records = Vec::with_capacity(self.len);
        if self.len == 0 {
            return records;
        }

        let start = if self.len == self.config.capacity {
            self.next_write_index
        } else {
            0
        };
        for offset in 0..self.len {
            let index = (start + offset) % self.config.capacity;
            if let Some(record) = &self.records[index] {
                records.push(record);
            }
        }
        records
    }

    pub fn replace_with_consolidated_records(
        &mut self,
        records: Vec<MemoryRecord>,
    ) -> Result<(), ScaffoldContractError> {
        if records.len() > self.config.capacity {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.records.fill(None);
        self.next_write_index = 0;
        self.len = 0;
        for record in records {
            self.insert_record(record)?;
        }
        Ok(())
    }

    fn record_by_id(&self, memory_id: MemoryId) -> Option<&MemoryRecord> {
        self.records
            .iter()
            .flatten()
            .find(|record| record.memory_id == memory_id)
    }

    fn validate_monotonic_insert(
        &self,
        organism_id: OrganismId,
        tick: Tick,
    ) -> Result<(), ScaffoldContractError> {
        organism_id.validate()?;
        if let Some((_, previous)) = self
            .last_inserted_ticks
            .iter()
            .find(|(known_organism, _)| *known_organism == organism_id)
        {
            Tick::validate_monotonic(*previous, tick)?;
        }
        Ok(())
    }

    fn record_last_tick(&mut self, organism_id: OrganismId, tick: Tick) {
        if let Some((_, previous)) = self
            .last_inserted_ticks
            .iter_mut()
            .find(|(known_organism, _)| *known_organism == organism_id)
        {
            *previous = tick;
        } else {
            self.last_inserted_ticks.push((organism_id, tick));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryConsolidationBatch {
    pub records: Vec<MemoryRecord>,
    pub max_records_after: usize,
}

impl Validate for MemoryConsolidationBatch {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.max_records_after == 0 || self.records.len() > self.max_records_after {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for record in &self.records {
            record.validate_contract()?;
        }
        Ok(())
    }
}

pub trait MemoryConsolidator {
    fn consolidate(
        &self,
        batch: MemoryConsolidationBatch,
    ) -> Result<Vec<MemoryRecord>, ScaffoldContractError>;
}

#[derive(Default)]
struct WeightedOutcomeSummary {
    success_likelihood: f32,
    contact_likelihood: f32,
    prediction_error: f32,
    pain_delta: f32,
    energy_delta: f32,
}

impl WeightedOutcomeSummary {
    fn add(&mut self, summary: MemoryOutcomeSummary, weight: f32) {
        self.success_likelihood += summary.success_likelihood.raw() * weight;
        self.contact_likelihood += summary.contact_likelihood.raw() * weight;
        self.prediction_error += summary.prediction_error.raw() * weight;
        self.pain_delta += summary.pain_delta.raw() * weight;
        self.energy_delta += summary.energy_delta.raw() * weight;
    }

    fn finish(self) -> Result<MemoryOutcomeSummary, ScaffoldContractError> {
        Ok(MemoryOutcomeSummary {
            success_likelihood: NormalizedScalar::new(self.success_likelihood.clamp(0.0, 1.0))?,
            contact_likelihood: NormalizedScalar::new(self.contact_likelihood.clamp(0.0, 1.0))?,
            prediction_error: NormalizedScalar::new(self.prediction_error.clamp(0.0, 1.0))?,
            pain_delta: NormalizedScalar::new(self.pain_delta.clamp(0.0, 1.0))?,
            energy_delta: SignedValence::new(self.energy_delta.clamp(-1.0, 1.0))?,
        })
    }
}

fn validate_feature_cap(max_feature_len: usize) -> Result<(), ScaffoldContractError> {
    if max_feature_len == 0 || max_feature_len > MEMORY_FEATURE_VECTOR_MAX_LEN {
        Err(ScaffoldContractError::ScalarOutOfRange)
    } else {
        Ok(())
    }
}

fn validate_feature_vector(features: &[f32]) -> Result<(), ScaffoldContractError> {
    if features.is_empty() || features.len() > MEMORY_FEATURE_VECTOR_MAX_LEN {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    validate_finite_slice(features)?;
    if features.iter().all(|value| (-1.0..=1.0).contains(value)) {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn feature_vector_from_pre_action(
    pre_action: &PreActionSnapshot,
    max_feature_len: usize,
) -> Result<Vec<f32>, ScaffoldContractError> {
    validate_feature_cap(max_feature_len)?;
    let mut features = Vec::with_capacity(max_feature_len);
    extend_bounded(
        &mut features,
        &pre_action.sensory().channels.as_flat_array(),
        max_feature_len,
    );
    extend_bounded(
        &mut features,
        &pre_action.homeostasis().drives.to_array(),
        max_feature_len,
    );
    extend_bounded(
        &mut features,
        &pre_action.homeostasis().hormones.to_array(),
        max_feature_len,
    );
    validate_feature_vector(&features)?;
    Ok(features)
}

fn extend_bounded(features: &mut Vec<f32>, values: &[f32], max_feature_len: usize) {
    for value in values {
        if features.len() == max_feature_len {
            break;
        }
        features.push(*value);
    }
}

fn normalized_dot(query: &[f32], record: &[f32]) -> Result<f32, ScaffoldContractError> {
    validate_feature_vector(query)?;
    validate_feature_vector(record)?;
    let len = query.len().min(record.len());
    let mut dot = 0.0;
    let mut query_norm = 0.0;
    let mut record_norm = 0.0;
    for index in 0..len {
        dot += query[index] * record[index];
        query_norm += query[index] * query[index];
        record_norm += record[index] * record[index];
    }
    if query_norm == 0.0 || record_norm == 0.0 {
        return Ok(0.0);
    }
    let score = dot / (query_norm.sqrt() * record_norm.sqrt());
    crate::validate_finite(score)?;
    Ok(score.clamp(0.0, 1.0))
}

fn max_affordance(pre_action: &PreActionSnapshot) -> f32 {
    pre_action
        .sensory()
        .channels
        .visual_affordance
        .iter()
        .copied()
        .fold(0.0, f32::max)
}

fn social_biases(pre_action: &PreActionSnapshot) -> (f32, f32) {
    let mut trust = 0.0_f32;
    let mut fear = 0.0_f32;
    for agent in pre_action
        .sensory()
        .social_context
        .nearest_agents
        .iter()
        .flatten()
    {
        let weighted_affinity = agent.affinity.raw() * agent.proximity.raw();
        trust = trust.max(weighted_affinity.max(0.0));
        fear = fear.max((-weighted_affinity).max(0.0));
    }
    (trust.clamp(0.0, 1.0), fear.clamp(0.0, 1.0))
}

fn weighted_drive_delta(current: DriveDelta, next: DriveDelta, weight: f32) -> DriveDelta {
    DriveDelta {
        hunger: current.hunger + next.hunger * weight,
        fatigue: current.fatigue + next.fatigue * weight,
        fear: current.fear + next.fear * weight,
        pain: current.pain + next.pain * weight,
        loneliness: current.loneliness + next.loneliness * weight,
        curiosity: current.curiosity + next.curiosity * weight,
        brain_atp: current.brain_atp + next.brain_atp * weight,
        temperature_stress: current.temperature_stress + next.temperature_stress * weight,
        reproductive_drive: current.reproductive_drive + next.reproductive_drive * weight,
        extension: [
            current.extension[0] + next.extension[0] * weight,
            current.extension[1] + next.extension[1] * weight,
        ],
    }
}

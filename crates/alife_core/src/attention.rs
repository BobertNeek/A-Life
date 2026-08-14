//! Bounded two-tier attention contracts for the v1.1 cognitive spine.

use serde::{Deserialize, Serialize};

use crate::{
    CanonicalDigestBuilder, ConceptCellId, Confidence, ExperienceSequenceId, NormalizedScalar,
    OrganismId, ScaffoldContractError, Tick, TrackedObjectId, Validate,
};

pub const ATTENTION_SCHEMA_VERSION: u16 = 1;
pub const MAX_PERIPHERAL_SUMMARIES: usize = 64;
pub const MAX_FOCAL_TARGETS: usize = 8;
pub const MAX_ATTENTION_SALIENCE_COMPONENTS: usize = 64;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NeuralStructuralIdentity(pub u64);

impl NeuralStructuralIdentity {
    pub const fn new(raw: u64) -> Option<Self> {
        if raw == 0 {
            None
        } else {
            Some(Self(raw))
        }
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        if self.0 == 0 {
            Err(ScaffoldContractError::InvalidId)
        } else {
            Ok(self)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StableFocusIdentity {
    TrackedObject(TrackedObjectId),
    Organism(OrganismId),
    Concept(ConceptCellId),
    NeuralStructural(NeuralStructuralIdentity),
}

impl StableFocusIdentity {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        match self {
            Self::TrackedObject(id) => id.validate().map(|_| ()),
            Self::Organism(id) => id.validate().map(|_| ()),
            Self::Concept(id) => id.validate().map(|_| ()),
            Self::NeuralStructural(id) => id.validate().map(|_| ()),
        }
    }

    fn canonical_key(self) -> (u8, u64) {
        match self {
            Self::TrackedObject(id) => (0, id.raw()),
            Self::Organism(id) => (1, id.raw()),
            Self::Concept(id) => (2, id.raw()),
            Self::NeuralStructural(id) => (3, id.raw()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SalienceComponents {
    #[serde(default = "default_peripheral_intensity")]
    pub peripheral_intensity: NormalizedScalar,
    pub drive: NormalizedScalar,
    pub memory_expectancy: NormalizedScalar,
    pub concept: NormalizedScalar,
    pub novelty: NormalizedScalar,
    pub uncertainty: NormalizedScalar,
    pub gap_voltage: NormalizedScalar,
}

fn default_peripheral_intensity() -> NormalizedScalar {
    NormalizedScalar(0.0)
}

impl Default for SalienceComponents {
    fn default() -> Self {
        Self {
            peripheral_intensity: NormalizedScalar(0.0),
            drive: NormalizedScalar(0.0),
            memory_expectancy: NormalizedScalar(0.0),
            concept: NormalizedScalar(0.0),
            novelty: NormalizedScalar(0.0),
            uncertainty: NormalizedScalar(0.0),
            gap_voltage: NormalizedScalar(0.0),
        }
    }
}

impl Validate for SalienceComponents {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        for value in [
            self.peripheral_intensity,
            self.drive,
            self.memory_expectancy,
            self.concept,
            self.novelty,
            self.uncertainty,
            self.gap_voltage,
        ] {
            NormalizedScalar::new(value.raw())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PeripheralSummary {
    pub identity: StableFocusIdentity,
    pub salience: SalienceComponents,
    pub confidence: Confidence,
}

impl Default for PeripheralSummary {
    fn default() -> Self {
        Self {
            identity: StableFocusIdentity::TrackedObject(TrackedObjectId(1)),
            salience: SalienceComponents::default(),
            confidence: Confidence(0.0),
        }
    }
}

impl Validate for PeripheralSummary {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.identity.validate_contract()?;
        self.salience.validate_contract()?;
        Confidence::new(self.confidence.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HysteresisState {
    pub previous_identity: Option<StableFocusIdentity>,
    pub retained_ticks: u16,
    pub switch_margin: NormalizedScalar,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AttentionSelectionPolicy {
    pub focal_capacity: u8,
    pub protected_minimum: u8,
    pub requested_focal_count: u8,
    pub switch_cost: NormalizedScalar,
    pub hysteresis_margin: NormalizedScalar,
}

impl Default for AttentionSelectionPolicy {
    fn default() -> Self {
        Self {
            focal_capacity: MAX_FOCAL_TARGETS as u8,
            protected_minimum: 0,
            requested_focal_count: MAX_FOCAL_TARGETS as u8,
            switch_cost: NormalizedScalar(0.05),
            hysteresis_margin: NormalizedScalar(0.05),
        }
    }
}

impl Validate for AttentionSelectionPolicy {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if usize::from(self.focal_capacity) > MAX_FOCAL_TARGETS
            || self.protected_minimum > self.focal_capacity
            || self.requested_focal_count > self.focal_capacity
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        NormalizedScalar::new(self.switch_cost.raw())?;
        NormalizedScalar::new(self.hysteresis_margin.raw())?;
        Ok(())
    }
}

impl Default for HysteresisState {
    fn default() -> Self {
        Self {
            previous_identity: None,
            retained_ticks: 0,
            switch_margin: NormalizedScalar(0.0),
        }
    }
}

impl Validate for HysteresisState {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if let Some(identity) = self.previous_identity {
            identity.validate_contract()?;
        }
        NormalizedScalar::new(self.switch_margin.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttentionBudgetReceipt {
    pub schema_version: u16,
    pub peripheral_capacity: u16,
    pub focal_capacity: u8,
    pub protected_minimum: u8,
    pub requested_focal_count: u8,
    pub granted_focal_count: u8,
    pub work_units: u64,
}

impl AttentionBudgetReceipt {
    pub fn new(
        peripheral_capacity: u16,
        focal_capacity: u8,
        protected_minimum: u8,
        requested_focal_count: u8,
        granted_focal_count: u8,
        work_units: u64,
    ) -> Result<Self, ScaffoldContractError> {
        let receipt = Self {
            schema_version: ATTENTION_SCHEMA_VERSION,
            peripheral_capacity,
            focal_capacity,
            protected_minimum,
            requested_focal_count,
            granted_focal_count,
            work_units,
        };
        receipt.validate_contract()?;
        Ok(receipt)
    }
}

impl Default for AttentionBudgetReceipt {
    fn default() -> Self {
        Self {
            schema_version: ATTENTION_SCHEMA_VERSION,
            peripheral_capacity: MAX_PERIPHERAL_SUMMARIES as u16,
            focal_capacity: MAX_FOCAL_TARGETS as u8,
            protected_minimum: 0,
            requested_focal_count: 0,
            granted_focal_count: 0,
            work_units: 0,
        }
    }
}

impl Validate for AttentionBudgetReceipt {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != ATTENTION_SCHEMA_VERSION
            || usize::from(self.peripheral_capacity) > MAX_PERIPHERAL_SUMMARIES
            || usize::from(self.focal_capacity) > MAX_FOCAL_TARGETS
            || self.protected_minimum > self.focal_capacity
            || self.requested_focal_count > self.focal_capacity
            || self.granted_focal_count > self.requested_focal_count
            || self.granted_focal_count < self.protected_minimum
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttentionFrame {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub world_tick: Tick,
    pub peripheral_summaries: Vec<PeripheralSummary>,
    pub focal_targets: Vec<StableFocusIdentity>,
    pub salience_components: Vec<SalienceComponents>,
    pub hysteresis: HysteresisState,
    pub budget_receipt: AttentionBudgetReceipt,
}

impl AttentionFrame {
    pub fn empty(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        world_tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        Self::new(
            organism_id,
            sequence_id,
            world_tick,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            HysteresisState::default(),
            AttentionBudgetReceipt::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        world_tick: Tick,
        peripheral_summaries: Vec<PeripheralSummary>,
        focal_targets: Vec<StableFocusIdentity>,
        salience_components: Vec<SalienceComponents>,
        hysteresis: HysteresisState,
        budget_receipt: AttentionBudgetReceipt,
    ) -> Result<Self, ScaffoldContractError> {
        let frame = Self {
            schema_version: ATTENTION_SCHEMA_VERSION,
            organism_id,
            sequence_id,
            world_tick,
            peripheral_summaries,
            focal_targets,
            salience_components,
            hysteresis,
            budget_receipt,
        };
        frame.validate_contract()?;
        Ok(frame)
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-ATTENTION-FRAME");
        builder.write_u16(self.schema_version);
        builder.write_u64(self.organism_id.raw());
        builder.write_u64(self.sequence_id.raw());
        builder.write_u64(self.world_tick.raw());
        builder.write_sequence_len(self.peripheral_summaries.len());
        for summary in &self.peripheral_summaries {
            write_identity(&mut builder, summary.identity);
            write_salience(&mut builder, summary.salience)?;
            builder.write_f32(summary.confidence.raw())?;
        }
        builder.write_sequence_len(self.focal_targets.len());
        for identity in &self.focal_targets {
            write_identity(&mut builder, *identity);
        }
        builder.write_sequence_len(self.salience_components.len());
        for salience in &self.salience_components {
            write_salience(&mut builder, *salience)?;
        }
        write_hysteresis(&mut builder, self.hysteresis)?;
        builder.write_u16(self.budget_receipt.schema_version);
        builder.write_u16(self.budget_receipt.peripheral_capacity);
        builder.write_u8(self.budget_receipt.focal_capacity);
        builder.write_u8(self.budget_receipt.protected_minimum);
        builder.write_u8(self.budget_receipt.requested_focal_count);
        builder.write_u8(self.budget_receipt.granted_focal_count);
        builder.write_u64(self.budget_receipt.work_units);
        Ok(builder.finish256())
    }
}

impl Validate for AttentionFrame {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != ATTENTION_SCHEMA_VERSION {
            return Err(ScaffoldContractError::IncompatibleAbi {
                kind: crate::SchemaKind::Experience,
                expected: ATTENTION_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        if self.peripheral_summaries.len() > MAX_PERIPHERAL_SUMMARIES
            || self.focal_targets.len() > MAX_FOCAL_TARGETS
            || self.salience_components.len() > MAX_ATTENTION_SALIENCE_COMPONENTS
            || self.focal_targets.len() > usize::from(self.budget_receipt.granted_focal_count)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for summary in &self.peripheral_summaries {
            summary.validate_contract()?;
        }
        for identity in &self.focal_targets {
            identity.validate_contract()?;
        }
        if self.focal_targets.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for salience in &self.salience_components {
            salience.validate_contract()?;
        }
        self.hysteresis.validate_contract()?;
        self.budget_receipt.validate_contract()?;
        if usize::from(self.budget_receipt.peripheral_capacity) < self.peripheral_summaries.len()
            || self.budget_receipt.granted_focal_count as usize != self.focal_targets.len()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

fn write_identity(builder: &mut CanonicalDigestBuilder, identity: StableFocusIdentity) {
    let (kind, raw) = identity.canonical_key();
    builder.write_u8(kind);
    builder.write_u64(raw);
}

fn write_salience(
    builder: &mut CanonicalDigestBuilder,
    salience: SalienceComponents,
) -> Result<(), ScaffoldContractError> {
    for value in [
        salience.peripheral_intensity,
        salience.drive,
        salience.memory_expectancy,
        salience.concept,
        salience.novelty,
        salience.uncertainty,
        salience.gap_voltage,
    ] {
        builder.write_f32(value.raw())?;
    }
    Ok(())
}

pub fn select_focal_targets(
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    world_tick: Tick,
    peripheral_summaries: &[PeripheralSummary],
    previous_hysteresis: HysteresisState,
    policy: AttentionSelectionPolicy,
) -> Result<AttentionFrame, ScaffoldContractError> {
    policy.validate_contract()?;
    if peripheral_summaries.len() > MAX_PERIPHERAL_SUMMARIES
        || usize::from(policy.requested_focal_count) < usize::from(policy.protected_minimum)
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    for summary in peripheral_summaries {
        summary.validate_contract()?;
    }

    let requested = usize::from(policy.requested_focal_count);
    let mut ranked = peripheral_summaries
        .iter()
        .enumerate()
        .map(|(index, summary)| (index, salience_score(summary.salience)))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right.1.total_cmp(&left.1).then_with(|| {
            peripheral_summaries[left.0]
                .identity
                .canonical_key()
                .cmp(&peripheral_summaries[right.0].identity.canonical_key())
        })
    });

    let mut selected = ranked
        .iter()
        .take(requested)
        .map(|(index, _)| *index)
        .collect::<Vec<_>>();
    let mut retained_previous = false;
    if let Some(previous_identity) = previous_hysteresis.previous_identity {
        if let Some(previous_index) = peripheral_summaries
            .iter()
            .position(|summary| summary.identity == previous_identity)
        {
            let previous_score = salience_score(peripheral_summaries[previous_index].salience);
            let challenger_exceeds_switch_cost = ranked
                .iter()
                .find_map(|(index, score)| (*index != previous_index).then_some(*score))
                .is_some_and(|score| score > previous_score + policy.switch_cost.raw());
            if selected.contains(&previous_index)
                && !challenger_exceeds_switch_cost
                && requested > 0
            {
                selected.retain(|index| *index != previous_index);
                retained_previous = true;
                selected.insert(0, previous_index);
            }
        }
    }
    if retained_previous {
        selected[1..].sort_by(|left, right| {
            salience_score(peripheral_summaries[*right].salience)
                .total_cmp(&salience_score(peripheral_summaries[*left].salience))
                .then_with(|| {
                    peripheral_summaries[*left]
                        .identity
                        .canonical_key()
                        .cmp(&peripheral_summaries[*right].identity.canonical_key())
                })
        });
        selected.truncate(requested);
    } else {
        selected.sort_by(|left, right| {
            salience_score(peripheral_summaries[*right].salience)
                .total_cmp(&salience_score(peripheral_summaries[*left].salience))
                .then_with(|| {
                    peripheral_summaries[*left]
                        .identity
                        .canonical_key()
                        .cmp(&peripheral_summaries[*right].identity.canonical_key())
                })
        });
    }
    selected.dedup();

    if selected.len() < usize::from(policy.protected_minimum) {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    let first_identity = selected
        .first()
        .map(|index| peripheral_summaries[*index].identity);
    let switches: u64 = if previous_hysteresis
        .previous_identity
        .is_some_and(|previous| Some(previous) != first_identity)
    {
        1
    } else {
        0
    };
    let work_units = u64::try_from(peripheral_summaries.len())
        .unwrap_or(u64::MAX)
        .saturating_add(u64::try_from(ranked.len()).unwrap_or(u64::MAX).saturating_mul(2))
        .saturating_add(u64::try_from(selected.len()).unwrap_or(u64::MAX).saturating_mul(3))
        .saturating_add(switches.saturating_mul(7));
    let budget_receipt = AttentionBudgetReceipt::new(
        u16::try_from(peripheral_summaries.len())
            .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?,
        policy.focal_capacity,
        policy.protected_minimum,
        policy.requested_focal_count,
        u8::try_from(selected.len())
            .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?,
        work_units,
    )?;
    let retained_ticks = if previous_hysteresis.previous_identity == first_identity {
        previous_hysteresis.retained_ticks.saturating_add(1)
    } else {
        if first_identity.is_some() { 1 } else { 0 }
    };
    AttentionFrame::new(
        organism_id,
        sequence_id,
        world_tick,
        peripheral_summaries.to_vec(),
        selected
            .iter()
            .map(|index| peripheral_summaries[*index].identity)
            .collect(),
        selected
            .iter()
            .map(|index| peripheral_summaries[*index].salience)
            .collect(),
        HysteresisState {
            previous_identity: first_identity,
            retained_ticks,
            switch_margin: policy.hysteresis_margin,
        },
        budget_receipt,
    )
}

fn salience_score(salience: SalienceComponents) -> f32 {
    0.20 * salience.peripheral_intensity.raw()
        + 0.10 * salience.novelty.raw()
        + 0.10 * salience.uncertainty.raw()
        + 0.20 * salience.drive.raw()
        + 0.20 * salience.memory_expectancy.raw()
        + 0.10 * salience.concept.raw()
        + 0.10 * salience.gap_voltage.raw()
}

fn write_hysteresis(
    builder: &mut CanonicalDigestBuilder,
    hysteresis: HysteresisState,
) -> Result<(), ScaffoldContractError> {
    match hysteresis.previous_identity {
        Some(identity) => {
            builder.write_some();
            write_identity(builder, identity);
        }
        None => builder.write_none(),
    }
    builder.write_u16(hysteresis.retained_ticks);
    builder.write_f32(hysteresis.switch_margin.raw())
}

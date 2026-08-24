//! Bounded multi-rate body, chemistry, development, and reproduction integration.

use serde::{Deserialize, Serialize};

use crate::{
    validate_finite, BiochemicalGraphState, BiochemicalWorkReceipt, CreaturePhenotype, GenomeId,
    HomeostaticSnapshot, NeuralEmissionFrame, NeuralReceptorFrame, ScaffoldContractError, Tick,
    Validate,
};

pub const MAX_BIOCHEMISTRY_CATCH_UP_STEPS: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BiochemistryCadence {
    pub fast_hormone_ticks: u32,
    pub metabolism_ticks: u32,
    pub development_ticks: u32,
    pub reproduction_ticks: u32,
    pub max_catch_up_steps: u32,
}

impl BiochemistryCadence {
    pub const fn early_mammal() -> Self {
        Self {
            fast_hormone_ticks: 1,
            metabolism_ticks: 6,
            development_ticks: 60,
            reproduction_ticks: 120,
            max_catch_up_steps: MAX_BIOCHEMISTRY_CATCH_UP_STEPS,
        }
    }
}

impl Validate for BiochemistryCadence {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.fast_hormone_ticks == 0
            || self.metabolism_ticks < self.fast_hormone_ticks
            || self.development_ticks < self.metabolism_ticks
            || self.reproduction_ticks < self.development_ticks
            || self.max_catch_up_steps == 0
            || self.max_catch_up_steps > MAX_BIOCHEMISTRY_CATCH_UP_STEPS
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BodyState {
    pub energy: f32,
    pub health: f32,
    pub injury: f32,
    pub temperature_stress: f32,
    pub sleeping: bool,
}

impl BodyState {
    fn baseline(phenotype: &CreaturePhenotype) -> Self {
        Self {
            energy: clamp01(0.50 + 0.50 * phenotype.body.metabolic_efficiency),
            health: 1.0,
            injury: 0.0,
            temperature_stress: 0.0,
            sleeping: false,
        }
    }

    fn apply_event(self, event: BodyEventDelta, phenotype: &CreaturePhenotype) -> Self {
        let injury_gain = event.damage * (1.0 - phenotype.body.injury_resistance);
        let recovery = event.sleep_recovery;
        let injury = clamp01(
            self.injury + injury_gain - recovery * (0.10 + 0.20 * phenotype.body.injury_resistance),
        );
        let health = clamp01(self.health - injury_gain + recovery * 0.15);
        let energy = clamp01(
            self.energy
                + event.energy
                + event.nutrition * phenotype.body.metabolic_efficiency
                + recovery * (0.10 + 0.15 * phenotype.body.metabolic_efficiency),
        );
        let temperature_stress = clamp01(
            self.temperature_stress
                + event.temperature_stress * (1.0 - phenotype.body.temperature_tolerance)
                - recovery * 0.20,
        );
        Self {
            energy,
            health,
            injury,
            temperature_stress,
            sleeping: recovery > 0.0,
        }
    }
}

impl Validate for BodyState {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_unit_values(&[
            self.energy,
            self.health,
            self.injury,
            self.temperature_stress,
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BodyEventDelta {
    pub energy: f32,
    pub damage: f32,
    pub temperature_stress: f32,
    pub nutrition: f32,
    pub social_contact: f32,
    pub sleep_recovery: f32,
    pub mating_opportunity: f32,
}

impl BodyEventDelta {
    pub const fn zero() -> Self {
        Self {
            energy: 0.0,
            damage: 0.0,
            temperature_stress: 0.0,
            nutrition: 0.0,
            social_contact: 0.0,
            sleep_recovery: 0.0,
            mating_opportunity: 0.0,
        }
    }
}

impl Validate for BodyEventDelta {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_signed_unit_values(&[self.energy])?;
        validate_unit_values(&[
            self.damage,
            self.temperature_stress,
            self.nutrition,
            self.social_contact,
            self.sleep_recovery,
            self.mating_opportunity,
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveBodyUpkeepPolicy;

impl PassiveBodyUpkeepPolicy {
    pub const ADULT_LIFETIME_BASE_MULTIPLIER: f32 = 2.0;
    pub const NO_FOOD_RESERVE_FRACTION: f32 = 0.55;
    pub const MATURATION_RESERVE_BUFFER: f32 = 1.25;
    pub const BODY_SIZE_LOAD_BASE: f32 = 0.90;
    pub const BODY_SIZE_LOAD_SPAN: f32 = 0.20;
    pub const EFFICIENCY_LOAD_BASE: f32 = 1.10;
    pub const EFFICIENCY_LOAD_SPAN: f32 = 0.20;

    pub fn maximum_lifespan_ticks(phenotype: &CreaturePhenotype) -> u64 {
        rounded_ticks(
            f64::from(phenotype.development.maturation_duration_ticks)
                * f64::from(Self::ADULT_LIFETIME_BASE_MULTIPLIER + phenotype.body.lifespan_scale),
        )
    }

    pub fn is_terminal(body: &BodyState, age_ticks: u64, phenotype: &CreaturePhenotype) -> bool {
        body.health <= 0.0
            || body.energy <= 0.0
            || age_ticks >= Self::maximum_lifespan_ticks(phenotype)
    }

    pub fn body_load(phenotype: &CreaturePhenotype) -> f32 {
        (Self::BODY_SIZE_LOAD_BASE + Self::BODY_SIZE_LOAD_SPAN * phenotype.body.size_scale)
            * (Self::EFFICIENCY_LOAD_BASE
                - Self::EFFICIENCY_LOAD_SPAN * phenotype.body.metabolic_efficiency)
    }

    pub fn reserve_horizon_ticks(phenotype: &CreaturePhenotype) -> u64 {
        let maturation_ticks = f64::from(phenotype.development.maturation_duration_ticks);
        let maximum_lifespan_ticks = Self::maximum_lifespan_ticks(phenotype) as f64;
        let body_load = f64::from(Self::body_load(phenotype).max(f32::EPSILON));
        rounded_ticks(
            (f64::from(Self::MATURATION_RESERVE_BUFFER) * maturation_ticks).max(
                f64::from(Self::NO_FOOD_RESERVE_FRACTION) * maximum_lifespan_ticks / body_load,
            ),
        )
    }

    pub fn upkeep_event(
        phenotype: &CreaturePhenotype,
        cadence: BiochemistryCadence,
        crossed_metabolism_steps: u32,
    ) -> BodyEventDelta {
        if crossed_metabolism_steps == 0 {
            return BodyEventDelta::zero();
        }
        let reserve_horizon_ticks = Self::reserve_horizon_ticks(phenotype).max(1) as f32;
        let baseline_energy = 0.50 + 0.50 * phenotype.body.metabolic_efficiency;
        let cost = baseline_energy * cadence.metabolism_ticks as f32 / reserve_horizon_ticks
            * crossed_metabolism_steps as f32;
        BodyEventDelta {
            energy: -cost.clamp(0.0, 1.0),
            ..BodyEventDelta::zero()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentReadiness {
    pub last_update_tick: Tick,
    pub age_ticks: Tick,
    pub maturation: f32,
    pub puberty_reached: bool,
    pub critical_period_active: bool,
    pub critical_period_plasticity_bias: f32,
    pub sleep_maturation_ready: bool,
    pub migration_ready: bool,
}

impl DevelopmentReadiness {
    fn derive(
        phenotype: &CreaturePhenotype,
        age: Tick,
        last_update_tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        let state = phenotype.development_state_at(age)?;
        let critical_period_active = !state.open_critical_periods.is_empty();
        let critical_period_plasticity_bias = state
            .open_critical_periods
            .first()
            .map_or(0.0, |period| period.plasticity_bias.raw());
        let maturation = state.maturation.raw();
        let value = Self {
            last_update_tick,
            age_ticks: age,
            maturation,
            puberty_reached: age >= phenotype.development.puberty_tick,
            critical_period_active,
            critical_period_plasticity_bias,
            sleep_maturation_ready: maturation
                >= phenotype
                    .brain_genome
                    .developmental_schedule
                    .sleep_pressure_maturation_gate
                    .raw(),
            migration_ready: maturation >= phenotype.development.migration_checkpoint.raw(),
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for DevelopmentReadiness {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        Tick::validate_monotonic(self.last_update_tick, self.age_ticks)?;
        validate_unit_values(&[self.maturation, self.critical_period_plasticity_bias])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReproductionReadiness {
    pub last_update_tick: Tick,
    pub puberty_reached: bool,
    pub healthy_enough: bool,
    pub energy_sufficient: bool,
    pub hormone_ready: bool,
    pub mating_opportunity: f32,
    pub ready: bool,
}

impl ReproductionReadiness {
    fn derive(
        last_update_tick: Tick,
        body: BodyState,
        homeostasis: &HomeostaticSnapshot,
        development: DevelopmentReadiness,
        mating_opportunity: f32,
        phenotype: &CreaturePhenotype,
    ) -> Result<Self, ScaffoldContractError> {
        let healthy_enough = body.health >= 0.55 && body.injury <= 0.45;
        let energy_sufficient = body.energy >= 0.35 && homeostasis.drives.brain_atp >= 0.25;
        let hormone_ready = homeostasis.drives.reproductive_drive
            >= phenotype.chemistry.reproductive_threshold
            && homeostasis.hormones.developmental_hormone >= 0.20;
        let ready = development.puberty_reached
            && healthy_enough
            && energy_sufficient
            && hormone_ready
            && mating_opportunity >= 0.50
            && phenotype.reproduction.fertility > 0.0;
        let value = Self {
            last_update_tick,
            puberty_reached: development.puberty_reached,
            healthy_enough,
            energy_sufficient,
            hormone_ready,
            mating_opportunity,
            ready,
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for ReproductionReadiness {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_unit_values(&[self.mating_opportunity])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BiochemistryState {
    pub source_genome_id: GenomeId,
    pub tick: Tick,
    pub body: BodyState,
    pub homeostasis: HomeostaticSnapshot,
    graph_state: BiochemicalGraphState,
    biochemical_work: BiochemicalWorkReceipt,
    pub development: DevelopmentReadiness,
    pub reproduction: ReproductionReadiness,
    pub cadence: BiochemistryCadence,
}

impl BiochemistryState {
    pub fn new(phenotype: &CreaturePhenotype, tick: Tick) -> Result<Self, ScaffoldContractError> {
        Self::new_with_age(phenotype, tick, tick)
    }

    pub fn new_with_age(
        phenotype: &CreaturePhenotype,
        tick: Tick,
        age: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        phenotype.brain_genome.validate_contract()?;
        phenotype.chemistry.endocrine.validate_contract()?;
        let cadence = BiochemistryCadence::early_mammal();
        cadence.validate_contract()?;
        let body = BodyState::baseline(phenotype);
        let graph_state = BiochemicalGraphState::new(&phenotype.chemistry.biochemical, tick)?;
        let homeostasis = graph_state.derive_homeostasis(&phenotype.chemistry.biochemical)?;
        let development_tick = cadence_boundary(age, cadence.development_ticks);
        let development = DevelopmentReadiness::derive(phenotype, age, development_tick)?;
        let reproduction_tick = cadence_boundary(age, cadence.reproduction_ticks);
        let reproduction = ReproductionReadiness::derive(
            reproduction_tick,
            body,
            &homeostasis,
            development,
            0.0,
            phenotype,
        )?;
        let value = Self {
            source_genome_id: phenotype.source_genome_id,
            tick,
            body,
            homeostasis,
            graph_state,
            biochemical_work: BiochemicalWorkReceipt::default(),
            development,
            reproduction,
            cadence,
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub fn advance(
        &self,
        next_tick: Tick,
        event: BodyEventDelta,
        phenotype: &CreaturePhenotype,
    ) -> Result<Self, ScaffoldContractError> {
        self.advance_with_age(next_tick, next_tick, event, phenotype)
    }

    pub fn advance_with_age(
        &self,
        next_tick: Tick,
        next_age: Tick,
        event: BodyEventDelta,
        phenotype: &CreaturePhenotype,
    ) -> Result<Self, ScaffoldContractError> {
        self.advance_with_neural_emission(next_tick, next_age, event, None, phenotype)
    }

    pub fn advance_with_neural_emission(
        &self,
        next_tick: Tick,
        next_age: Tick,
        event: BodyEventDelta,
        neural: Option<&NeuralEmissionFrame>,
        phenotype: &CreaturePhenotype,
    ) -> Result<Self, ScaffoldContractError> {
        self.validate_contract()?;
        event.validate_contract()?;
        phenotype.brain_genome.validate_contract()?;
        if phenotype.source_genome_id != self.source_genome_id {
            return Err(ScaffoldContractError::InvalidId);
        }
        Tick::validate_monotonic(self.tick, next_tick)?;
        Tick::validate_monotonic(self.development.age_ticks, next_age)?;
        Tick::validate_monotonic(next_age, next_tick)?;

        let metabolic_steps = crossed_boundaries(
            self.tick,
            next_tick,
            self.cadence.metabolism_ticks,
            self.cadence.max_catch_up_steps,
        );
        let development_steps = crossed_boundaries(
            self.development.age_ticks,
            next_age,
            self.cadence.development_ticks,
            self.cadence.max_catch_up_steps,
        );
        let reproduction_steps = crossed_boundaries(
            self.development.age_ticks,
            next_age,
            self.cadence.reproduction_ticks,
            self.cadence.max_catch_up_steps,
        );
        let upkeep =
            PassiveBodyUpkeepPolicy::upkeep_event(phenotype, self.cadence, metabolic_steps);
        let event = BodyEventDelta {
            energy: signed_clamp(event.energy + upkeep.energy),
            ..event
        };
        let body = self.body.apply_event(event, phenotype);
        body.validate_contract()?;
        let (graph_state, biochemical_work) = self.graph_state.advance(
            next_tick,
            body,
            event,
            neural,
            &phenotype.chemistry.biochemical,
        )?;
        let homeostasis = graph_state.derive_homeostasis(&phenotype.chemistry.biochemical)?;
        let development = if development_steps > 0 {
            DevelopmentReadiness::derive(
                phenotype,
                next_age,
                cadence_boundary(next_age, self.cadence.development_ticks),
            )?
        } else {
            DevelopmentReadiness {
                age_ticks: next_age,
                ..self.development
            }
        };
        let reproduction = if reproduction_steps > 0 {
            ReproductionReadiness::derive(
                cadence_boundary(next_age, self.cadence.reproduction_ticks),
                body,
                &homeostasis,
                development,
                event.mating_opportunity,
                phenotype,
            )?
        } else {
            self.reproduction
        };
        let value = Self {
            source_genome_id: self.source_genome_id,
            tick: next_tick,
            body,
            homeostasis,
            graph_state,
            biochemical_work,
            development,
            reproduction,
            cadence: self.cadence,
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub const fn graph_state(&self) -> &BiochemicalGraphState {
        &self.graph_state
    }

    pub const fn biochemical_work(&self) -> BiochemicalWorkReceipt {
        self.biochemical_work
    }

    pub fn neural_receptor_frame(
        &self,
        phenotype: &CreaturePhenotype,
    ) -> Result<NeuralReceptorFrame, ScaffoldContractError> {
        self.graph_state
            .neural_receptor_frame(&phenotype.chemistry.biochemical)
    }
}

impl Validate for BiochemistryState {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.source_genome_id.validate()?;
        self.body.validate_contract()?;
        self.homeostasis.validate_contract()?;
        self.graph_state.validate_contract()?;
        self.development.validate_contract()?;
        self.reproduction.validate_contract()?;
        self.cadence.validate_contract()?;
        if self.homeostasis.tick != self.tick
            || self.development.age_ticks.raw() > self.tick.raw()
            || self.development.last_update_tick.raw() > self.tick.raw()
            || self.reproduction.last_update_tick.raw() > self.development.age_ticks.raw()
            || self.reproduction.last_update_tick.raw() > self.tick.raw()
        {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        Ok(())
    }
}

fn crossed_boundaries(from: Tick, to: Tick, period: u32, cap: u32) -> u32 {
    let period = u64::from(period);
    let crossed = to.raw() / period - from.raw() / period;
    u32::try_from(crossed.min(u64::from(cap))).unwrap_or(cap)
}

fn rounded_ticks(value: f64) -> u64 {
    let rounded = value.round();
    if !rounded.is_finite() || rounded <= 0.0 {
        0
    } else if rounded >= u64::MAX as f64 {
        u64::MAX
    } else {
        rounded as u64
    }
}

fn cadence_boundary(tick: Tick, period: u32) -> Tick {
    let period = u64::from(period);
    Tick(tick.raw() / period * period)
}

fn validate_unit_values(values: &[f32]) -> Result<(), ScaffoldContractError> {
    for value in values {
        validate_finite(*value)?;
        if !(0.0..=1.0).contains(value) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
    }
    Ok(())
}

fn validate_signed_unit_values(values: &[f32]) -> Result<(), ScaffoldContractError> {
    for value in values {
        validate_finite(*value)?;
        if !(-1.0..=1.0).contains(value) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
    }
    Ok(())
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn signed_clamp(value: f32) -> f32 {
    value.clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity};

    #[test]
    fn newborn_at_late_world_tick_starts_development_at_biological_age_zero() {
        let genome = CreatureGenome::early_mammal_founder(
            0xE10_31FF,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        let mut phenotype = genome.express().unwrap();
        phenotype.development.puberty_tick = Tick(1);

        let state = BiochemistryState::new_with_age(&phenotype, Tick(10_000), Tick(0)).unwrap();

        assert_eq!(state.development.age_ticks, Tick(0));
        assert!(!state.development.puberty_reached);
    }
}

//! Explicit, nonpersistent thermal-physics research factor. Not a default rule.
use super::*;
use alife_core::ThermalBalanceCandidate;

#[derive(Debug, Clone)]
pub struct ThermalContactCandidate {
    /// Only these explicitly configured physical surfaces participate. Values
    /// are dimensionless conductance [0,1], not object kinds or reward labels.
    /// Zero represents an insulating surface; inherited body tolerance remains
    /// the distinct organism response. No redundant actor-insulation parameter.
    pub surface_conductance: BTreeMap<u64, f32>,
    /// Signed descriptor interval [-neutral_band,+neutral_band] is comfortable.
    pub neutral_band: f32,
    /// Unsigned pre-tolerance load per second at maximum severity/conductance.
    pub exposure_per_second: f32,
    /// Passive body-load removal per second when no harmful contact is present.
    pub relief_per_second: f32,
}

impl ThermalContactCandidate {
    /// Deliberately conservative dimensionless research timescales: ten seconds
    /// to unit load before tolerance, twenty seconds to remove unit load.
    pub fn research(surface_conductance: BTreeMap<u64, f32>) -> Self {
        Self {
            surface_conductance,
            neutral_band: 0.25,
            exposure_per_second: 0.10,
            relief_per_second: 0.05,
        }
    }

    fn validate(&self, world: &HeadlessWorld) -> Result<(), ScaffoldContractError> {
        for value in [
            self.neutral_band,
            self.exposure_per_second,
            self.relief_per_second,
        ]
        .into_iter()
        .chain(self.surface_conductance.values().copied())
        {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        if self.neutral_band >= 1.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for entity in self.surface_conductance.keys() {
            WorldEntityId(*entity).validate()?;
            if !world.objects.contains_key(entity) {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }
}

impl HeadlessWorld {
    /// Run an explicitly configured research operation atomically. Configuration
    /// never enters portable state and must be supplied again after loading.
    /// Ordinary operations and existing saves retain their original semantics.
    /// Nested candidate scopes are rejected rather than silently replacing rules.
    pub fn with_thermal_contact_candidate<T>(
        &mut self,
        parameters: ThermalContactCandidate,
        operation: impl FnOnce(&mut Self) -> Result<T, ScaffoldContractError>,
    ) -> Result<T, ScaffoldContractError> {
        parameters.validate(self)?;
        if self.thermal_contact_candidate.is_some() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let mut staged = self.clone();
        staged.thermal_contact_candidate = Some(parameters);
        let result = operation(&mut staged)?;
        staged.thermal_contact_candidate = None;
        *self = staged;
        Ok(result)
    }

    pub(super) fn thermal_contact_severity_at(&self, own: WorldEntityId, position: Vec3f) -> f32 {
        let Some(parameters) = &self.thermal_contact_candidate else {
            return 0.0;
        };
        parameters
            .surface_conductance
            .iter()
            .filter_map(|(id, conductance)| {
                let object = self.objects.get(id)?;
                if object.id == own
                    || object.consumed
                    || distance(position, object.position)
                        > object.radius.max(HEADLESS_CONTACT_RADIUS)
                    || !self.solid_segment_clear_from(
                        position,
                        object.position,
                        Some(object.id),
                        self.objects.values(),
                    )
                {
                    return None;
                }
                let severity = ((object.grounded_physical.surface_temperature.abs()
                    - parameters.neutral_band)
                    / (1.0 - parameters.neutral_band))
                    .clamp(0.0, 1.0);
                Some(severity * conductance)
            })
            .fold(0.0, f32::max)
    }

    pub(super) fn thermal_interval_event(
        &self,
        mut event: BodyEventDelta,
        severity: f32,
    ) -> (BodyEventDelta, ThermalBalanceCandidate) {
        let Some(parameters) = &self.thermal_contact_candidate else {
            return (event, ThermalBalanceCandidate::default());
        };
        // Each authoritative path admits exactly current_tick+1. A long command
        // duration is not elapsed physiology; moving samples endpoint contact,
        // with the stronger endpoint receiving at most one full interval dose.
        let seconds = 1.0 / WORLD_TICKS_PER_SECOND as f32;
        let exposure = parameters.exposure_per_second * severity * seconds;
        if exposure > 0.0 {
            event.temperature_stress = (event.temperature_stress + exposure).clamp(0.0, 1.0);
        }
        let relief = if severity == 0.0 {
            parameters.relief_per_second * seconds
        } else {
            0.0
        };
        (
            event,
            ThermalBalanceCandidate {
                passive_relief: relief,
                suppress_sleep_thermal_relief: exposure > 0.0,
            },
        )
    }

    pub(super) fn advance_biology_with_thermal_candidate(
        &mut self,
        organism: OrganismId,
        tick: Tick,
        event: BodyEventDelta,
        neural: Option<&NeuralEmissionFrame>,
        relief: ThermalBalanceCandidate,
    ) -> Result<(), ScaffoldContractError> {
        if self.thermal_contact_candidate.is_some() {
            self.organism_registry
                .advance_biology_thermal_candidate(organism, tick, event, neural, relief)
        } else if let Some(neural) = neural {
            self.organism_registry
                .advance_biology_with_neural_emission(organism, tick, event, neural)
        } else {
            self.organism_registry
                .advance_biology(organism, tick, event)
        }
        .map_err(map_organism_registry_error)
    }
}

#[cfg(test)]
mod tests;

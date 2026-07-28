use alife_core::{
    ScaffoldContractError, SemanticPriorPacket, SemanticPriorRequest, Tick, Validate,
    SEMANTIC_PRIOR_MAX_GAIN, SEMANTIC_PRIOR_MAX_LEXICON_BIAS_SLOTS,
    SEMANTIC_PRIOR_MAX_PACKET_TICKS,
};
use serde::{Deserialize, Serialize};

pub const UNAIDED_PROBE_INTERVAL_EXPOSURES: u32 = 64;
pub const FADE_START_UNAIDED_EXPOSURES: u32 = 128;
pub const PASSING_PROBE_LOWER_CONFIDENCE: f32 = 0.75;
pub const PASSING_PROBES_TO_ZERO: u8 = 3;
pub const NOVELTY_REACTIVATION_GAIN: f32 = 0.05;
pub const NOVELTY_REACTIVATION_TICKS: u64 = 128;
pub const NOVELTY_REACTIVATION_COOLDOWN_TICKS: u64 = 1_024;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentalPriorController {
    unaided_exposures: u32,
    last_probe_exposure: u32,
    consecutive_passing_probes: u8,
    last_reactivation_tick: Option<Tick>,
    active_reactivation_until: Option<Tick>,
}

impl DevelopmentalPriorController {
    pub const fn unaided_exposures(&self) -> u32 {
        self.unaided_exposures
    }

    pub const fn consecutive_passing_probes(&self) -> u8 {
        self.consecutive_passing_probes
    }

    pub const fn active_reactivation_until(&self) -> Option<Tick> {
        self.active_reactivation_until
    }

    pub fn record_relevant_exposure(&mut self, assisted: bool) {
        if !assisted {
            self.unaided_exposures = self.unaided_exposures.saturating_add(1);
        }
    }

    pub fn probe_due(&self) -> bool {
        self.unaided_exposures
            .saturating_sub(self.last_probe_exposure)
            >= UNAIDED_PROBE_INTERVAL_EXPOSURES
    }

    pub fn record_unaided_probe(
        &mut self,
        lower_confidence_bound: f32,
    ) -> Result<(), ScaffoldContractError> {
        if !lower_confidence_bound.is_finite() || !(0.0..=1.0).contains(&lower_confidence_bound) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.last_probe_exposure = self.unaided_exposures;
        if lower_confidence_bound >= PASSING_PROBE_LOWER_CONFIDENCE {
            self.consecutive_passing_probes = self
                .consecutive_passing_probes
                .saturating_add(1)
                .min(PASSING_PROBES_TO_ZERO);
        } else {
            self.consecutive_passing_probes = 0;
        }
        Ok(())
    }

    pub fn developmental_gain(&self) -> f32 {
        if self.unaided_exposures < FADE_START_UNAIDED_EXPOSURES {
            return SEMANTIC_PRIOR_MAX_GAIN;
        }
        let remaining = PASSING_PROBES_TO_ZERO.saturating_sub(self.consecutive_passing_probes);
        SEMANTIC_PRIOR_MAX_GAIN * f32::from(remaining) / f32::from(PASSING_PROBES_TO_ZERO)
    }

    pub fn issue_packet(
        &mut self,
        request: SemanticPriorRequest,
        tick: Tick,
        mut lexicon_bias_slots: Vec<u16>,
        novelty: bool,
    ) -> Result<SemanticPriorPacket, ScaffoldContractError> {
        request.validate_contract()?;
        lexicon_bias_slots.sort_unstable();
        lexicon_bias_slots.dedup();
        lexicon_bias_slots.truncate(SEMANTIC_PRIOR_MAX_LEXICON_BIAS_SLOTS);
        if lexicon_bias_slots.iter().any(|slot| *slot >= 256) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }

        let developmental_gain = self.developmental_gain();
        if developmental_gain == 0.0 && novelty && self.reactivation_available(tick) {
            self.last_reactivation_tick = Some(tick);
            self.active_reactivation_until =
                Some(Tick(tick.raw().saturating_add(NOVELTY_REACTIVATION_TICKS)));
        }
        let reactivation_active = developmental_gain == 0.0
            && self
                .active_reactivation_until
                .is_some_and(|until| tick.raw() < until.raw());
        let packet = SemanticPriorPacket {
            request,
            lexicon_bias_slots,
            plasticity_modulation: if reactivation_active {
                NOVELTY_REACTIVATION_GAIN
            } else {
                developmental_gain
            },
            issued_at_tick: tick,
            expires_at_tick: Tick(tick.raw().saturating_add(SEMANTIC_PRIOR_MAX_PACKET_TICKS)),
            assisted: true,
        };
        packet.validate_contract()?;
        Ok(packet)
    }

    fn reactivation_available(&self, tick: Tick) -> bool {
        self.last_reactivation_tick.is_none_or(|last| {
            tick.raw()
                >= last
                    .raw()
                    .saturating_add(NOVELTY_REACTIVATION_COOLDOWN_TICKS)
        })
    }
}

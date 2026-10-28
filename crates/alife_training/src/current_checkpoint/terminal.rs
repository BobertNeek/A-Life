//! A later observed death retires continuation of the last valid retained brain.
//! This is disposition metadata, not a fabricated dead private-world snapshot.
use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentTerminalDispositionV1 {
    pub schema: u16,
    pub kind: String,
    pub baseline_current_sha256: String,
    pub request_sha256: String,
    pub retained_state_sha256: String,
    pub session_nonce: u64,
    pub organism_id: u64,
    pub retained_world_seed: u64,
    pub retained_world_tick: u64,
    pub observed_world_tick: u64,
    pub terminal_death_tick: Option<u64>,
    pub terminal_event: serde_json::Value,
}

#[derive(Deserialize)]
struct TerminalObservation {
    kind: String,
    terminal: bool,
    session_nonce: u64,
    source_world_seed: u64,
    organism_id: u64,
    world_tick: u64,
    terminal_death_tick: Option<u64>,
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

impl CurrentTerminalDispositionV1 {
    pub(super) fn validate(&self, current: &CurrentTrainingCheckpointV1) -> Result<()> {
        let event: TerminalObservation = serde_json::from_value(self.terminal_event.clone())?;
        let life = current
            .life
            .as_ref()
            .ok_or("terminal retained private life missing")?;
        let save = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
        if self.schema != 1
            || self.kind != "observed-terminal-retained-brain-v1"
            || !valid_hash(&self.baseline_current_sha256)
            || !valid_hash(&self.request_sha256)
            || save.creatures.len() != 1
            || save.creatures[0].organism_id.raw() != self.organism_id
            || self.organism_id == 0
            || self.retained_world_seed != current.seed
            || self.retained_world_tick != save.world.tick.raw()
            || self.session_nonce == 0
            || self.session_nonce == current.seed
            || !current.seeds.contains(self.session_nonce)
            || current
                .seeds
                .pending_range
                .is_some_and(|[a, b]| a <= self.session_nonce && self.session_nonce <= b)
            || self.observed_world_tick < self.retained_world_tick
            || self.terminal_death_tick.is_some_and(|tick| {
                tick < self.retained_world_tick || tick > self.observed_world_tick
            })
            || event.kind != "continuous-terminal-v1"
            || !event.terminal
            || event.session_nonce != self.session_nonce
            || event.source_world_seed != self.retained_world_seed
            || event.organism_id != self.organism_id
            || event.world_tick != self.observed_world_tick
            || event.terminal_death_tick != self.terminal_death_tick
        {
            return Err(
                "terminal disposition differs from retained life or committed observation".into(),
            );
        }
        let mut retained = current.clone();
        retained.terminal_disposition = None;
        if self.retained_state_sha256 != format!("{:x}", Sha256::digest(retained.encode()?)) {
            return Err("terminal disposition retained brain/private-state binding differs".into());
        }
        Ok(())
    }
}

impl CurrentTrainingCheckpointV1 {
    /// Inspection and archival admission remain available. No training lane or
    /// private-life materialization may silently resume this retired individual.
    pub fn require_living_continuation(&self) -> Result<()> {
        if self.terminal_disposition.is_some() {
            return Err(
                "Current records an observed terminal individual; continuation is retired".into(),
            );
        }
        Ok(())
    }

    pub fn with_terminal_disposition(
        &self,
        baseline_current_sha256: &str,
        request_sha256: &str,
        session_nonce: u64,
        terminal_event: serde_json::Value,
    ) -> Result<Self> {
        self.validate()?;
        self.require_living_continuation()?;
        let event: TerminalObservation = serde_json::from_value(terminal_event.clone())?;
        if event.session_nonce != session_nonce {
            return Err("terminal observation belongs to a different dispatched session".into());
        }
        let life = self
            .life
            .as_ref()
            .ok_or("terminal retained private life missing")?;
        let save = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
        let mut next = self.clone();
        next.terminal_disposition = Some(CurrentTerminalDispositionV1 {
            schema: 1,
            kind: "observed-terminal-retained-brain-v1".into(),
            baseline_current_sha256: baseline_current_sha256.into(),
            request_sha256: request_sha256.into(),
            retained_state_sha256: format!("{:x}", Sha256::digest(self.encode()?)),
            session_nonce,
            organism_id: event.organism_id,
            retained_world_seed: self.seed,
            retained_world_tick: save.world.tick.raw(),
            observed_world_tick: event.world_tick,
            terminal_death_tick: event.terminal_death_tick,
            terminal_event,
        });
        next.validate()?;
        Ok(next)
    }
}

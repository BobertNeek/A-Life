//! Explicit counterfactual training branches. The original life remains dead;
//! a new attempt restores its retained preterminal acquired-state snapshot.
use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainingAttemptOriginV1 {
    pub schema: u16,
    pub kind: String,
    pub attempt_nonce: u64,
    pub source_current_sha256: String,
    pub source_terminal: CurrentTerminalDispositionV1,
    pub retained_world_seed: u64,
    pub retained_world_tick: u64,
    pub organism_id: u64,
    pub policy_version: u64,
    pub actor_optimizer_step: u32,
    pub value_optimizer_step: u32,
    pub handoff_sha256: String,
    pub private_life_sha256: String,
    /// Oldest first, constant-depth evidence. Never prior brains/private assets.
    #[serde(
        default,
        deserialize_with = "read_previous",
        serialize_with = "write_previous"
    )]
    pub previous: Vec<TrainingAttemptEvidenceV1>,
    pub record_sha256: String,
}

/// One flat hash-chained origin. It cannot contain another ancestry collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainingAttemptEvidenceV1 {
    pub schema: u16,
    pub kind: String,
    pub attempt_nonce: u64,
    pub source_current_sha256: String,
    pub source_terminal: CurrentTerminalDispositionV1,
    pub retained_world_seed: u64,
    pub retained_world_tick: u64,
    pub organism_id: u64,
    pub policy_version: u64,
    pub actor_optimizer_step: u32,
    pub value_optimizer_step: u32,
    pub handoff_sha256: String,
    pub private_life_sha256: String,
    pub previous_record_sha256: Option<String>,
    pub record_sha256: String,
}

fn sha(value: &impl Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}
fn is_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
// The first already-published origin used `previous: null`. Preserve both its
// exact encoded fields and its original digest; only later ancestry is an array.
fn read_previous<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<TrainingAttemptEvidenceV1>, D::Error> {
    Ok(Option::<Vec<TrainingAttemptEvidenceV1>>::deserialize(d)?.unwrap_or_default())
}
fn write_previous<S: serde::Serializer>(
    records: &[TrainingAttemptEvidenceV1],
    s: S,
) -> std::result::Result<S::Ok, S::Error> {
    if records.is_empty() {
        s.serialize_none()
    } else {
        records.serialize(s)
    }
}
impl TrainingAttemptEvidenceV1 {
    fn validate(&self) -> Result<()> {
        let t = &self.source_terminal;
        let e = &t.terminal_event;
        if self.schema != 1
            || self.kind != "explicit-preterminal-training-branch-v1"
            || self.attempt_nonce == 0
            || self.attempt_nonce == self.retained_world_seed
            || self.attempt_nonce == t.session_nonce
            || self.organism_id == 0
            || self.policy_version == 0
            || !is_hash(&self.source_current_sha256)
            || !is_hash(&self.handoff_sha256)
            || !is_hash(&self.private_life_sha256)
            || !is_hash(&t.retained_state_sha256)
            || !is_hash(&t.baseline_current_sha256)
            || !is_hash(&t.request_sha256)
            || t.schema != 1
            || t.kind != "observed-terminal-retained-brain-v1"
            || t.retained_world_seed != self.retained_world_seed
            || t.retained_world_tick != self.retained_world_tick
            || t.organism_id != self.organism_id
            || t.observed_world_tick < t.retained_world_tick
            || t.terminal_death_tick
                .is_some_and(|v| v < t.retained_world_tick || v > t.observed_world_tick)
            || e["kind"] != "continuous-terminal-v1"
            || e["terminal"] != true
            || e["session_nonce"].as_u64() != Some(t.session_nonce)
            || e["source_world_seed"].as_u64() != Some(t.retained_world_seed)
            || e["organism_id"].as_u64() != Some(t.organism_id)
            || e["world_tick"].as_u64() != Some(t.observed_world_tick)
            || e["terminal_death_tick"] != serde_json::to_value(t.terminal_death_tick)?
            || !is_hash(&self.record_sha256)
        {
            return Err(
                "training attempt origin differs from its bound terminal observation".into(),
            );
        }
        Ok(())
    }
    fn origin(&self, previous: Vec<Self>) -> TrainingAttemptOriginV1 {
        TrainingAttemptOriginV1 {
            schema: self.schema,
            kind: self.kind.clone(),
            attempt_nonce: self.attempt_nonce,
            source_current_sha256: self.source_current_sha256.clone(),
            source_terminal: self.source_terminal.clone(),
            retained_world_seed: self.retained_world_seed,
            retained_world_tick: self.retained_world_tick,
            organism_id: self.organism_id,
            policy_version: self.policy_version,
            actor_optimizer_step: self.actor_optimizer_step,
            value_optimizer_step: self.value_optimizer_step,
            handoff_sha256: self.handoff_sha256.clone(),
            private_life_sha256: self.private_life_sha256.clone(),
            previous,
            record_sha256: self.record_sha256.clone(),
        }
    }
}

impl TrainingAttemptOriginV1 {
    fn evidence(&self) -> TrainingAttemptEvidenceV1 {
        TrainingAttemptEvidenceV1 {
            schema: self.schema,
            kind: self.kind.clone(),
            attempt_nonce: self.attempt_nonce,
            source_current_sha256: self.source_current_sha256.clone(),
            source_terminal: self.source_terminal.clone(),
            retained_world_seed: self.retained_world_seed,
            retained_world_tick: self.retained_world_tick,
            organism_id: self.organism_id,
            policy_version: self.policy_version,
            actor_optimizer_step: self.actor_optimizer_step,
            value_optimizer_step: self.value_optimizer_step,
            handoff_sha256: self.handoff_sha256.clone(),
            private_life_sha256: self.private_life_sha256.clone(),
            previous_record_sha256: self.previous.last().map(|p| p.record_sha256.clone()),
            record_sha256: self.record_sha256.clone(),
        }
    }
    fn digest(&self) -> Result<String> {
        // Retains the original leaf origin's exact serialization/digest. A flat
        // predecessor is validated against the same algorithm and its prefix.
        let mut value = self.clone();
        value.record_sha256.clear();
        sha(&value)
    }
    fn validate_record(&self) -> Result<()> {
        let current = self.evidence();
        let mut previous: Option<&TrainingAttemptEvidenceV1> = None;
        let mut nonces = std::collections::BTreeSet::new();
        for (index, record) in self
            .previous
            .iter()
            .chain(std::iter::once(&current))
            .enumerate()
        {
            record.validate()?;
            if record.previous_record_sha256.as_deref()
                != previous.map(|p| p.record_sha256.as_str())
                || record.record_sha256
                    != record.origin(self.previous[..index].to_vec()).digest()?
                || !nonces.insert(record.attempt_nonce)
                || previous.is_some_and(|p| {
                    p.retained_world_seed != record.retained_world_seed
                        || p.organism_id != record.organism_id
                        || p.policy_version > record.policy_version
                        || p.actor_optimizer_step > record.actor_optimizer_step
                        || p.value_optimizer_step > record.value_optimizer_step
                })
            {
                return Err("training attempt origin ancestry or optimizer history changed".into());
            }
            previous = Some(record);
        }
        Ok(())
    }
    pub(super) fn validate(&self, current: &CurrentTrainingCheckpointV1) -> Result<()> {
        self.validate_record()?;
        let life = current
            .life
            .as_ref()
            .ok_or("training branch private life missing")?;
        let save = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
        let attempt_consumed = current.seeds.contains(self.attempt_nonce)
            && !current
                .seeds
                .pending_range
                .is_some_and(|[a, b]| a <= self.attempt_nonce && self.attempt_nonce <= b);
        if !attempt_consumed
            && (sha(&current.handoff)? != self.handoff_sha256
                || sha(life)? != self.private_life_sha256)
        {
            return Err(
                "unconsumed training branch differs from its retained heads or private life".into(),
            );
        }
        if current.seed != self.retained_world_seed
            || save.creatures.len() != 1
            || save.creatures[0].organism_id.raw() != self.organism_id
            || save.world.tick.raw() < self.retained_world_tick
            || current.handoff.policy_version < self.policy_version
            || current.handoff.actor.optimizer_step < self.actor_optimizer_step
            || current.handoff.value.optimizer_step < self.value_optimizer_step
        {
            return Err(
                "training branch replaced its retained life or rolled back acquired heads".into(),
            );
        }
        Ok(())
    }
}

impl CurrentTrainingCheckpointV1 {
    /// Device-free, create-new branch construction. This does NOT reserve or
    /// consume a nonce, change a world, train, or erase the original death.
    pub fn begin_training_attempt_from_terminal(
        &self,
        source_bytes: &[u8],
        attempt_nonce: u64,
    ) -> Result<Self> {
        self.validate()?;
        let source_current_sha256 = self.admitted_source_sha256(source_bytes)?;
        let terminal = self
            .terminal_disposition
            .as_ref()
            .ok_or("explicit training branch requires an observed terminal source")?;
        if attempt_nonce == 0
            || attempt_nonce == self.seed
            || (self.seeds.contains(attempt_nonce)
                && !self
                    .seeds
                    .pending_range
                    .is_some_and(|[a, _]| a == attempt_nonce))
            || self
                .seeds
                .pending_range
                .is_some_and(|[a, _]| a != attempt_nonce)
        {
            return Err(
                "training attempt nonce must be the next unused owned seed or a fresh unused nonce"
                    .into(),
            );
        }
        let mut probe = self.seeds.clone();
        if probe.pending_range.is_none() {
            probe.reserve(attempt_nonce, attempt_nonce)?;
        }
        let life = self
            .life
            .as_ref()
            .ok_or("training branch requires retained private life")?;
        let save = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
        let world = save.restore_headless_world()?;
        let residents = world.organism_registry().iter().collect::<Vec<_>>();
        if residents.len() != 1 || !residents[0].lifecycle().is_alive() {
            return Err("training branch requires a living PRETERMINAL snapshot; no dead body is resurrected".into());
        }
        let mut origin = TrainingAttemptOriginV1 {
            schema: 1,
            kind: "explicit-preterminal-training-branch-v1".into(),
            attempt_nonce,
            source_current_sha256,
            source_terminal: terminal.clone(),
            retained_world_seed: self.seed,
            retained_world_tick: save.world.tick.raw(),
            organism_id: residents[0].organism_id().raw(),
            policy_version: self.handoff.policy_version,
            actor_optimizer_step: self.handoff.actor.optimizer_step,
            value_optimizer_step: self.handoff.value.optimizer_step,
            handoff_sha256: sha(&self.handoff)?,
            private_life_sha256: sha(life)?,
            previous: self
                .training_attempt_origin
                .as_ref()
                .map_or_else(Vec::new, |p| {
                    let mut records = p.previous.clone();
                    records.push(p.evidence());
                    records
                }),
            record_sha256: String::new(),
        };
        origin.record_sha256 = origin.digest()?;
        let mut next = self.clone();
        next.terminal_disposition = None;
        next.training_attempt_origin = Some(origin);
        next.validate()?;
        Ok(next)
    }
    /// Until its first real dispatch, this explicit attempt owns one intended
    /// nonce. Later sessions can use the ordinary allocator after it is consumed.
    pub fn require_training_session_nonce(&self, nonce: u64) -> Result<()> {
        self.require_living_continuation()?;
        if let Some(origin) = &self.training_attempt_origin {
            let consumed = self.seeds.contains(origin.attempt_nonce)
                && !self
                    .seeds
                    .pending_range
                    .is_some_and(|[a, b]| a <= origin.attempt_nonce && origin.attempt_nonce <= b);
            if !consumed && nonce != origin.attempt_nonce {
                return Err(
                    "first training branch dispatch must use its explicit attempt nonce".into(),
                );
            }
        }
        Ok(())
    }
    pub fn require_fresh_cohort_continuation(&self) -> Result<()> {
        self.require_living_continuation()?;
        if self.training_attempt_origin.is_some() {
            return Err("retained-private training branch requires continuous care; fresh cohort would discard its private life".into());
        }
        Ok(())
    }
}

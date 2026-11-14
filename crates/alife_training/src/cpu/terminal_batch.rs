//! Recipe4 decision-local immutable reader. Recipe2/3 readers stay frozen.
use super::{failed_contact, terminal_kernel, Receipt, Source};
use alife_core::*;
use std::collections::BTreeMap;

/// Counts of explicit reader stages, not instructions or measured hardware FLOPs.
/// Export itself includes Core validation/hashing; it is counted as one export,
/// separately from the one additional portable-bank validation in this reader.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BatchWork {
    pub ordinary_records: usize,
    pub channel_records: usize,
    pub bank_exports: usize,
    pub reader_bank_validations: usize,
    pub failed_contact_index_records: usize,
    pub channel_contract_validations: usize,
    pub combined_digest_builds: usize,
    pub query_contract_validations: usize,
    pub query_source_visits: usize,
    pub appearance_comparisons: usize,
}

pub struct Batch {
    owner: u64,
    profile: SensorProfileIdentity,
    generation: u64,
    digest: [u64; 4],
    records: usize,
    // Earliest qualifying tick preserves the old existential <= query.tick test.
    earliest_failed_contact: BTreeMap<u64, u64>,
    sources: Vec<(Source, f32)>,
    work: BatchWork,
}
impl Batch {
    /// Called only after host cardinality and ATP admission, before any query.
    pub fn new(
        bank: &PortableMemoryBankAssetV2,
        channels: &[ChannelTerminalObservationV1],
    ) -> Result<Self, String> {
        if bank.records.len() > MEMORY_BANK_MAX_CAPACITY || channels.len() > 64 {
            return Err("terminal batch capacity exceeded".into());
        }
        if !matches!(bank.profile.profile_id.raw(), 2..=4) {
            return Err("terminal context requires grounded object observations".into());
        }
        bank.validate_contract().map_err(|e| e.to_string())?;
        let mut earliest_failed_contact = BTreeMap::<u64, u64>::new();
        for record in &bank.records {
            if record.family_raw == u16::from(CandidateActionFamily::Ingest.raw())
                && failed_contact(
                    record.outcome_contact,
                    f32::from_bits(record.family_value_bits[1]),
                )
            {
                earliest_failed_contact
                    .entry(record.tracked_object_id_raw)
                    .and_modify(|tick| *tick = (*tick).min(record.last_tick_raw))
                    .or_insert(record.last_tick_raw);
            }
        }
        let mut digest = CanonicalDigestBuilder::new(b"ALIFE-TERMINAL-CHANNEL-READ-V1");
        for word in bank.canonical_digest {
            digest.write_u64(word);
        }
        let mut sources = Vec::with_capacity(channels.len());
        for observation in channels {
            observation.validate_contract().map_err(|e| e.to_string())?;
            let query = observation.query();
            if query.organism_id().raw() != bank.organism_id_raw || query.profile() != bank.profile
            {
                return Err("terminal channel source owner/profile mismatch".into());
            }
            for word in observation.canonical_digest() {
                digest.write_u64(word);
            }
            sources.push((
                Source {
                    channel: Some(observation.channel()),
                    observation_digest: Some(observation.canonical_digest()),
                    memory_id: observation.sequence_id().raw(),
                    sequence_id: observation.sequence_id().raw(),
                    action_id: query.action_id().raw(),
                    tracked_object_id: query.tracked_object_id().unwrap().raw(),
                    last_tick: query.tick().raw(),
                    observed_hunger_delta: observation.measured_hunger_delta(),
                    observed_danger: observation.danger(),
                    confidence: observation.confidence(),
                    appearance: std::array::from_fn(|i| query.features()[63 + i]),
                },
                query.features()[11],
            ));
        }
        let work = BatchWork {
            ordinary_records: bank.records.len(),
            channel_records: channels.len(),
            reader_bank_validations: 1,
            failed_contact_index_records: bank.records.len(),
            channel_contract_validations: channels.len(),
            combined_digest_builds: 1,
            ..BatchWork::default()
        };
        Ok(Self {
            owner: bank.organism_id_raw,
            profile: bank.profile,
            generation: bank.generation,
            digest: digest.finish256(),
            records: bank
                .records
                .len()
                .checked_add(channels.len())
                .ok_or("terminal count overflow")?,
            earliest_failed_contact,
            sources,
            work,
        })
    }
    pub fn work(&self) -> BatchWork {
        self.work.clone()
    }
    pub fn digest(&self) -> [u64; 4] {
        self.digest
    }
    pub fn read(&self, query: &CandidateMemoryQueryV2) -> Result<Receipt, String> {
        query.validate_contract().map_err(|e| e.to_string())?;
        if query.organism_id().raw() != self.owner || query.profile() != self.profile {
            return Err("terminal query owner/profile mismatch".into());
        }
        let features = query.features();
        let mut sources = Vec::with_capacity(self.sources.len());
        let mut skipped_uncertain_appearance = 0;
        for (source, sensor_confidence) in &self.sources {
            if source.last_tick > query.tick().raw() {
                continue;
            }
            if features[11] <= 0.0 || *sensor_confidence <= 0.0 {
                skipped_uncertain_appearance += 1;
                continue;
            }
            sources.push(*source);
        }
        let same_object_failed_contact = query.tracked_object_id().is_some_and(|id| {
            self.earliest_failed_contact
                .get(&id.raw())
                .is_some_and(|tick| *tick <= query.tick().raw())
        });
        let mut context = terminal_kernel::summarize(
            std::array::from_fn(|i| features[63 + i]),
            features[12],
            &sources,
        )
        .map_err(|e| format!("terminal batch values: {e:?}"))?;
        if same_object_failed_contact {
            if let Some(context) = context.as_mut() {
                context.desirability = context.desirability.min(0.0);
            }
        }
        Ok(Receipt {
            bank_generation: self.generation,
            bank_digest: self.digest,
            query_digest: query.canonical_digest(),
            records_validated: self.records,
            // B records were indexed once; this query visits only bounded C sources.
            records_examined: self.sources.len(),
            appearance_comparisons: sources.len() * 9,
            context,
            same_object_failed_contact,
            skipped_uncertain_appearance,
        })
    }
}

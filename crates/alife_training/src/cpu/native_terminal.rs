//! Opt-in terminal reader for legacy native-bank and recipe3 channel sources.
//! Core validates source/query codecs; charged host admission owns dispatch.
#[path = "terminal_batch.rs"]
pub mod batch;
#[path = "terminal_kernel.rs"]
mod terminal_kernel;
use alife_core::{
    CandidateActionFamily, CandidateMemoryQueryV2, CanonicalDigestBuilder,
    ChannelTerminalObservationV1, PhysicalContactKind, PortableMemoryBankAssetV2,
};
use terminal_kernel::{Context, Error, Source, MAX_RECORDS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    Off,
    AppearanceOnlyConsumedV1,
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Receipt {
    pub bank_generation: u64,
    pub bank_digest: [u64; 4],
    pub query_digest: [u64; 4],
    pub records_validated: usize,
    pub records_examined: usize,
    pub appearance_comparisons: usize,
    pub context: Option<Context>,
    pub same_object_failed_contact: bool,
    pub skipped_uncertain_appearance: usize,
}

/// Returned separately from ordinary family/target recall. This pure reader
/// changes no sensory block, ordinary memory, physiology or neural credit.
/// The versioned host reserves its work before using it in the optional head.
pub fn read(
    policy: Policy,
    bank: &PortableMemoryBankAssetV2,
    query: &CandidateMemoryQueryV2,
) -> Result<Option<Receipt>, String> {
    read_inner(policy, bank, query, None)
}
pub fn read_channels(
    bank: &PortableMemoryBankAssetV2,
    query: &CandidateMemoryQueryV2,
    channels: &[ChannelTerminalObservationV1],
) -> Result<Option<Receipt>, String> {
    read_inner(
        Policy::AppearanceOnlyConsumedV1,
        bank,
        query,
        Some(channels),
    )
}
fn read_inner(
    policy: Policy,
    bank: &PortableMemoryBankAssetV2,
    query: &CandidateMemoryQueryV2,
    channels: Option<&[ChannelTerminalObservationV1]>,
) -> Result<Option<Receipt>, String> {
    if policy == Policy::Off {
        return Ok(None);
    }
    // Reject before hash/validation work; never silently truncate by record ID.
    let record_count = bank.records.len() + channels.map_or(0, |s| s.len());
    if record_count > MAX_RECORDS {
        return Err("terminal query record budget exceeded".into());
    }
    if !matches!(bank.profile.profile_id.raw(), 2..=4) {
        return Err("terminal context requires grounded object observations".into());
    }
    bank.validate_contract().map_err(|e| e.to_string())?;
    query.validate_contract().map_err(|e| e.to_string())?;
    if bank.organism_id_raw != query.organism_id().raw() || bank.profile != query.profile() {
        return Err("terminal query owner/profile mismatch".into());
    }
    let q = query.features();
    let appearance = std::array::from_fn(|i| q[63 + i]);
    let mut sources = Vec::new();
    let mut skipped_uncertain_appearance = 0;
    let same_object_failed_contact = bank.records.iter().any(|r| {
        Some(r.tracked_object_id_raw) == query.tracked_object_id().map(|id| id.raw())
            && r.family_raw == u16::from(CandidateActionFamily::Ingest.raw())
            && r.last_tick_raw <= query.tick().raw()
            && failed_contact(r.outcome_contact, f32::from_bits(r.family_value_bits[1]))
    });
    for r in &bank.records {
        // Recipe3 gets positive consumption sources solely from authenticated
        // channel observations. The ordinary bank still supplies contact vetoes.
        if channels.is_some() {
            continue;
        }
        // Never infer consumption from success, valence or unknown legacy tags.
        if r.family_raw != u16::from(CandidateActionFamily::Ingest.raw())
            || r.outcome_contact != Some(PhysicalContactKind::Consumed)
            || r.query_version_raw != query.version().raw()
            || r.last_tick_raw > query.tick().raw()
        {
            continue;
        }
        // Query lane11 is the native candidate sensor confidence, sourced from
        // calibrated visual slot confidence. Legacy merged records cannot prove
        // every contributing observation had this witness; abstain on those.
        if q[11] <= 0.0
            || f32::from_bits(r.query_feature_bits[11]) <= 0.0
            || r.observation_count != 1
        {
            skipped_uncertain_appearance += 1;
            continue;
        }
        sources.push(Source {
            channel: None,
            observation_digest: None,
            memory_id: r.memory_id_raw,
            sequence_id: r.sealed_sequence_id_raw,
            action_id: r.action_id_raw,
            tracked_object_id: r.tracked_object_id_raw,
            last_tick: r.last_tick_raw,
            observed_hunger_delta: f32::from_bits(r.target_latent_bits[0]),
            observed_danger: f32::from_bits(r.family_value_bits[2]),
            confidence: f32::from_bits(r.confidence_bits),
            appearance: std::array::from_fn(|i| f32::from_bits(r.query_feature_bits[63 + i])),
        });
    }
    let mut bank_digest = bank.canonical_digest;
    if let Some(channels) = channels {
        let mut digest = CanonicalDigestBuilder::new(b"ALIFE-TERMINAL-CHANNEL-READ-V1");
        for word in bank.canonical_digest {
            digest.write_u64(word);
        }
        for observation in channels {
            observation.validate_contract().map_err(|e| e.to_string())?;
            let source = observation.query();
            if source.organism_id() != query.organism_id() || source.profile() != query.profile() {
                return Err("terminal channel source owner/profile mismatch".into());
            }
            for word in observation.canonical_digest() {
                digest.write_u64(word);
            }
            if source.tick() > query.tick() {
                continue;
            }
            if q[11] <= 0.0 || source.features()[11] <= 0.0 {
                skipped_uncertain_appearance += 1;
                continue;
            }
            sources.push(Source {
                channel: Some(observation.channel()),
                observation_digest: Some(observation.canonical_digest()),
                memory_id: observation.sequence_id().raw(),
                sequence_id: observation.sequence_id().raw(),
                action_id: source.action_id().raw(),
                tracked_object_id: source.tracked_object_id().unwrap().raw(),
                last_tick: source.tick().raw(),
                observed_hunger_delta: observation.measured_hunger_delta(),
                observed_danger: observation.danger(),
                confidence: observation.confidence(),
                appearance: std::array::from_fn(|i| source.features()[63 + i]),
            });
        }
        bank_digest = digest.finish256();
    }
    let comparisons = sources.len() * 9;
    let mut context = terminal_kernel::summarize(appearance, q[12], &sources).map_err(|e| {
        match e {
            Error::InvalidInput => "invalid terminal values",
            Error::BudgetExceeded => "terminal query source budget exceeded",
        }
        .to_string()
    })?;
    if same_object_failed_contact {
        if let Some(c) = context.as_mut() {
            c.desirability = c.desirability.min(0.0);
        }
    }
    Ok(Some(Receipt {
        same_object_failed_contact,
        skipped_uncertain_appearance,
        bank_generation: bank.generation,
        bank_digest,
        query_digest: query.canonical_digest(),
        records_validated: record_count,
        records_examined: record_count,
        appearance_comparisons: comparisons,
        context,
    }))
}

/// Only an actual failed contact contradicts a same-object terminal prior.
/// Successful preparation, noncontact/unreachable attempts, and consumption do not.
fn failed_contact(contact: Option<PhysicalContactKind>, success: f32) -> bool {
    success < 0.5
        && matches!(
            contact,
            Some(
                PhysicalContactKind::Touch
                    | PhysicalContactKind::Blocked
                    | PhysicalContactKind::Collision
            )
        )
}
#[cfg(test)]
mod native_tests {
    use super::*;
    #[test]
    fn successful_preparation_and_noncontact_never_veto() {
        assert!(!failed_contact(Some(PhysicalContactKind::Touch), 1.0));
        assert!(!failed_contact(Some(PhysicalContactKind::Moved), 1.0));
        assert!(!failed_contact(Some(PhysicalContactKind::None), 0.0));
        assert!(!failed_contact(None, 0.0));
        assert!(!failed_contact(Some(PhysicalContactKind::Consumed), 1.0));
        assert!(failed_contact(Some(PhysicalContactKind::Touch), 0.0));
        assert!(failed_contact(Some(PhysicalContactKind::Blocked), 0.0));
    }
}

//! Recipe4-only admission and dispatch. Earlier recipes retain their old code.
use super::*;
use reader::batch::{Batch, BatchWork};

fn debit(work: u64) -> Result<u32, String> {
    let scaled = work
        .checked_mul(65535)
        .ok_or("terminal batch debit overflow")?;
    u32::try_from(scaled.div_ceil(1_000_000)).map_err(|_| "terminal batch debit overflow".into())
}

fn admit(
    ordinary: usize,
    channels: usize,
    queries: usize,
    approaches: usize,
    available: u32,
) -> Result<TerminalAdmissionPlan2135, String> {
    if queries > MAX_ACTION_CANDIDATES || approaches > queries {
        return Err("invalid terminal batch query counts".into());
    }
    let check = 128_u64
        .checked_add(queries as u64)
        .ok_or("terminal batch work overflow")?;
    let capacity = if ordinary > MEMORY_BANK_MAX_CAPACITY {
        Some((ordinary, MEMORY_BANK_MAX_CAPACITY))
    } else if channels > 64 {
        Some((channels, 64))
    } else {
        None
    };
    let requested = if capacity.is_some() {
        check
    } else {
        // Keep the old4096-unit record/query envelope, removing only repeated B
        // passes. Export/validation/index/hash of B+C once; each query still
        // validates its identity and scans all C sources. The fixed64 source
        // bound also covers kernel sorting/reduction. Logical tariff, not FLOPs.
        let records = ordinary
            .checked_add(channels)
            .ok_or("terminal batch count overflow")? as u64;
        let once = records
            .checked_mul(4096)
            .and_then(|n| n.checked_add(128))
            .ok_or("terminal batch work overflow")?;
        let per_query = (channels as u64)
            .checked_mul(4096)
            .and_then(|n| n.checked_add(4096))
            .ok_or("terminal batch work overflow")?;
        once.checked_add(
            (approaches as u64)
                .checked_mul(per_query)
                .ok_or("terminal batch work overflow")?,
        )
        .ok_or("terminal batch work overflow")?
        .max(check)
    };
    let requested_atp = debit(requested)?;
    let admission = if let Some((records, limit)) = capacity {
        TerminalAdmission2135::BankCapacityExceeded { records, limit }
    } else if requested_atp > available {
        TerminalAdmission2135::InsufficientAtp {
            required_atp_q16: requested_atp,
            available_atp_q16: available,
        }
    } else {
        TerminalAdmission2135::Admitted
    };
    let work = if admission == TerminalAdmission2135::Admitted {
        requested
    } else {
        check
    };
    let charged = debit(work)?;
    Ok(TerminalAdmissionPlan2135 {
        admission,
        requested_work_units: requested,
        requested_atp_q16: requested_atp,
        charged_work_units: work,
        charged_atp_q16: charged.min(available),
        admission_check_unfunded_atp_q16: charged.saturating_sub(available),
    })
}

pub(super) fn prepare(
    config: &TerminalContextConfig2135,
    memory: &MemorySidecarState,
    channels: Option<&TerminalChannelState2135>,
    recall: &FinalizedMemoryRecall,
    available: u32,
) -> Result<TerminalContextReceipt2135, String> {
    let channels = channels.ok_or("terminal batch requires authenticated channel state")?;
    let ordinary_count = memory.bank().len();
    let channel_count = channels.observations.len();
    let records = ordinary_count
        .checked_add(channel_count)
        .ok_or("terminal batch count overflow")?;
    let observation = admit_terminal_observation2135(channel_count, available);
    let keys = recall.candidate_keys();
    let approaches = keys
        .iter()
        .filter(|key| {
            key.query().action_family() == CandidateActionFamily::Approach
                && key.query().tracked_object_id().is_some()
        })
        .count();
    let plan = admit(
        ordinary_count,
        channel_count,
        keys.len(),
        approaches,
        available
            .checked_sub(observation.reserved_atp_q16)
            .ok_or("terminal observation debit overflow")?,
    )?;
    let capacity_rejected = matches!(
        plan.admission,
        TerminalAdmission2135::BankCapacityExceeded { .. }
    );
    let mut result = TerminalContextReceipt2135 {
        batch_work: Some(BatchWork {
            ordinary_records: ordinary_count,
            channel_records: channel_count,
            ..BatchWork::default()
        }),
        schema_version: 4,
        config: config.clone(),
        bank_digest: [0; 4],
        bank_generation: 0,
        records,
        candidates: Vec::new(),
        capacity_rejected,
        admission: plan.admission,
        reserved_work_units: plan
            .charged_work_units
            .checked_add(observation.reserved_work_units)
            .ok_or("terminal batch work overflow")?,
        reserved_atp_q16: plan
            .charged_atp_q16
            .checked_add(observation.reserved_atp_q16)
            .ok_or("terminal batch debit overflow")?,
        requested_work_units: plan
            .requested_work_units
            .checked_add(observation.requested_work_units)
            .ok_or("terminal batch work overflow")?,
        requested_atp_q16: plan
            .requested_atp_q16
            .checked_add(observation.requested_atp_q16)
            .ok_or("terminal batch debit overflow")?,
        admission_check_unfunded_atp_q16: plan
            .admission_check_unfunded_atp_q16
            .checked_add(observation.admission_check_unfunded_atp_q16)
            .ok_or("terminal batch debit overflow")?,
        observation: Some(observation),
    };
    if result.admission != TerminalAdmission2135::Admitted {
        return Ok(result);
    }
    let bank = memory.export_active_bank().map_err(|e| e.to_string())?;
    let batch = Batch::new(&bank, &channels.observations)?;
    result.bank_digest = batch.digest();
    result.bank_generation = bank.generation;
    let mut work = batch.work();
    work.bank_exports = 1;
    let head = head::Head::from_bytes(&config.head_recipe).map_err(|_| "invalid terminal head")?;
    for key in keys {
        let query = key.query();
        if query.action_family() != CandidateActionFamily::Approach
            || query.tracked_object_id().is_none()
        {
            continue;
        }
        let read = batch.read(query)?;
        work.query_contract_validations = work
            .query_contract_validations
            .checked_add(1)
            .ok_or("terminal batch work overflow")?;
        work.query_source_visits = work
            .query_source_visits
            .checked_add(channel_count)
            .ok_or("terminal batch work overflow")?;
        work.appearance_comparisons = work
            .appearance_comparisons
            .checked_add(read.appearance_comparisons)
            .ok_or("terminal batch work overflow")?;
        let desirability = read.context.as_ref().map(|context| context.desirability);
        let head_work = head::Work::reader(
            read.records_examined as u32,
            read.appearance_comparisons as u32,
        )
        .map_err(|_| "invalid batch head work")?;
        let score = head
            .evaluate(
                true,
                if config.mask_new_context {
                    desirability.map(|_| 0.0)
                } else {
                    desirability
                },
                head_work,
            )
            .map_err(|_| "invalid terminal score")?;
        let sources = read
            .context
            .as_ref()
            .map(|c| c.sources.as_slice())
            .unwrap_or(&[]);
        result.candidates.push(TerminalCandidateReceipt2135 {
            candidate_index: query.candidate_index(),
            query_digest: query.canonical_digest(),
            source_memory_ids: Vec::new(),
            source_sequence_ids: sources.iter().map(|s| s.sequence_id).collect(),
            source_action_ids: sources.iter().map(|s| s.action_id).collect(),
            source_channel_observations: sources
                .iter()
                .map(|s| TerminalChannelSource2135 {
                    sequence_id: s.sequence_id,
                    channel: s.channel.unwrap(),
                    action_id: s.action_id,
                    tracked_object_id: s.tracked_object_id,
                    observation_digest: s.observation_digest.unwrap(),
                })
                .collect(),
            consumed_source_count: sources.len(),
            hunger_now: query.features()[12],
            desirability,
            delta: score.delta,
        });
    }
    result.batch_work = Some(work);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cardinality_atp_and_work_precede_export_without_combined64_limit() {
        let accepted = admit(65, 1, 10, 4, 65535).unwrap();
        assert_eq!(accepted.admission, TerminalAdmission2135::Admitted);
        assert_eq!(
            accepted.requested_work_units,
            128 + 66 * 4096 + 4 * (4096 + 4096)
        );
        let old = admit_terminal_work2135(66, 10, 4, 65535).unwrap();
        assert!(matches!(
            old.admission,
            TerminalAdmission2135::BankCapacityExceeded { limit: 64, .. }
        ));
        let large = admit(
            MEMORY_BANK_MAX_CAPACITY,
            64,
            MAX_ACTION_CANDIDATES,
            MAX_ACTION_CANDIDATES,
            65535,
        )
        .unwrap();
        assert!(matches!(
            large.admission,
            TerminalAdmission2135::InsufficientAtp { .. }
        ));
        assert_eq!(large.charged_work_units, 128 + MAX_ACTION_CANDIDATES as u64);
        for (b, c, limit) in [
            (usize::MAX, 0, MEMORY_BANK_MAX_CAPACITY),
            (0, usize::MAX, 64),
        ] {
            let p = admit(b, c, 1, 1, 65535).unwrap();
            assert!(
                matches!(p.admission, TerminalAdmission2135::BankCapacityExceeded { limit:l, .. } if l==limit)
            );
        }
        let zero = admit(1, 1, 2, 1, 0).unwrap();
        assert_eq!(zero.charged_atp_q16, 0);
        assert!(zero.admission_check_unfunded_atp_q16 > 0);
        assert!(debit(u64::MAX).is_err());
        assert!(admit(0, 0, MAX_ACTION_CANDIDATES + 1, 0, 65535).is_err());
    }
}

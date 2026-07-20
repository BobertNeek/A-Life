use super::*;

use std::collections::BTreeSet;

pub(super) fn candidate_record_from_patch(
    memory_id: MemoryId,
    patch: &ExperiencePatch,
) -> Result<CandidateMemoryRecordV2, ScaffoldContractError> {
    patch.validate_contract()?;
    let decision = patch.decision();
    let key = decision
        .episodic_key()
        .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
    let query = key.query();
    let outcome = patch.outcome();
    let drives = outcome.homeostatic_delta.drives;
    let contact = if outcome.physical.contact == PhysicalContactKind::None {
        0.0
    } else {
        1.0
    };
    let target_latent = [
        drives.hunger,
        drives.fear,
        drives.pain,
        drives.curiosity,
        drives.brain_atp,
        patch.pre_action().sensory().channels.novelty_signal.raw(),
        contact,
        outcome.prediction_error.raw(),
    ]
    .map(|value| value.clamp(-1.0, 1.0));
    let danger = outcome
        .pain_delta
        .raw()
        .max(outcome.frustration_delta.raw())
        .max((-outcome.reward_valence.raw()).max(0.0));
    let family_value = [
        outcome.reward_valence.raw(),
        if outcome.success { 1.0 } else { 0.0 },
        danger,
        outcome.energy_delta.raw(),
    ]
    .map(|value| value.clamp(-1.0, 1.0));
    let salience = outcome
        .reward_valence
        .raw()
        .abs()
        .max(outcome.pain_delta.raw())
        .max(outcome.prediction_error.raw())
        .max(patch.pre_action().sensory().channels.novelty_signal.raw());
    let record = CandidateMemoryRecordV2 {
        schema_version: MEMORY_RECALL_SCHEMA_VERSION,
        memory_id,
        organism_id_raw: query.organism_id().raw(),
        source_sequence_id: patch.header().sequence_id,
        first_tick: query.tick(),
        last_tick: query.tick(),
        profile_id_raw: query.profile().profile_id.raw(),
        profile_schema_version: query.profile().profile_schema_version,
        sensory_abi_version_raw: query.profile().sensory_abi_version,
        query_version_raw: query.version().raw(),
        action_id_raw: query.action_id().raw(),
        action_kind_raw: query.action_kind().raw(),
        family_raw: u16::from(query.action_family().raw()),
        tracked_object_id_raw: query.tracked_object_id().map_or(0, |id| id.raw()),
        query_features: query.features().to_vec(),
        target_latent,
        family_value,
        confidence: decision.confidence.raw(),
        salience_q16: (salience.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16,
        observation_count: 1,
    };
    record.validate_contract()?;
    Ok(record)
}

pub(super) fn merge_candidate_records(
    retained: &CandidateMemoryRecordV2,
    observation: &CandidateMemoryRecordV2,
) -> Result<CandidateMemoryRecordV2, ScaffoldContractError> {
    retained.validate_contract()?;
    observation.validate_contract()?;
    if retained.identity() != observation.identity() {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    let old_count = retained.observation_count;
    let new_count = old_count
        .checked_add(observation.observation_count)
        .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
    let old_weight = old_count as f32 / new_count as f32;
    let new_weight = observation.observation_count as f32 / new_count as f32;
    let mut merged = retained.clone();
    for (value, next) in merged
        .query_features
        .iter_mut()
        .zip(&observation.query_features)
    {
        *value = (*value * old_weight + *next * new_weight).clamp(-1.0, 1.0);
    }
    for (value, next) in merged
        .target_latent
        .iter_mut()
        .zip(observation.target_latent)
    {
        *value = (*value * old_weight + next * new_weight).clamp(-1.0, 1.0);
    }
    for (value, next) in merged.family_value.iter_mut().zip(observation.family_value) {
        *value = (*value * old_weight + next * new_weight).clamp(-1.0, 1.0);
    }
    merged.confidence =
        (merged.confidence * old_weight + observation.confidence * new_weight).clamp(0.0, 1.0);
    merged.salience_q16 = (f32::from(merged.salience_q16) * old_weight
        + f32::from(observation.salience_q16) * new_weight)
        .round() as u16;
    merged.source_sequence_id = observation.source_sequence_id;
    merged.last_tick = observation.last_tick;
    merged.observation_count = new_count;
    merged.validate_contract()?;
    Ok(merged)
}

pub(super) struct TargetRecallResult {
    pub(super) values: [f32; MEMORY_LATENT_V1_COUNT],
    pub(super) confidence: Confidence,
    pub(super) source_count: u16,
    pub(super) best_source: Option<MemoryId>,
    pub(super) eligible: u32,
    pub(super) searched: u32,
    pub(super) matches: u16,
}

pub(super) struct FamilyRecallResult {
    pub(super) values: [f32; MEMORY_VALUE_V1_COUNT],
    pub(super) confidence: Confidence,
    pub(super) source_count: u16,
    pub(super) best_source: Option<MemoryId>,
    pub(super) eligible: u32,
    pub(super) searched: u32,
    pub(super) matches: u16,
}

pub(super) fn recall_target_channel(
    store: &CandidateMemoryStoreV2,
    query: &CandidateMemoryQueryV2,
    exact_key: &TargetMemoryBucketKey,
) -> Result<TargetRecallResult, ScaffoldContractError> {
    if query.tracked_object_id().is_none() {
        return Ok(TargetRecallResult {
            values: [0.0; MEMORY_LATENT_V1_COUNT],
            confidence: Confidence::new(0.0)?,
            source_count: 0,
            best_source: None,
            eligible: 0,
            searched: 0,
            matches: 0,
        });
    }
    let ids = collect_shortlist(
        &neighbor_target_keys(exact_key),
        |key| store.target_index.get(key),
        store,
        exact_key.target_bins,
        MEMORY_TARGET_SEARCH_CAP,
    );
    let eligible = ids.0;
    let searched = u32::try_from(ids.1.len()).unwrap_or(u32::MAX);
    let mut matches = ids
        .1
        .into_iter()
        .filter_map(|id| {
            let record = store.records.get(&id.raw())?;
            let score = target_similarity(query.features(), &record.query_features);
            (score >= MEMORY_MIN_SIMILARITY).then_some((id, score))
        })
        .collect::<Vec<_>>();
    sort_and_truncate_matches(&mut matches);
    let (values, confidence, source_count, best_source) =
        aggregate_target_matches(store, &matches)?;
    Ok(TargetRecallResult {
        values,
        confidence,
        source_count,
        best_source,
        eligible,
        searched,
        matches: u16::try_from(matches.len())
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
    })
}

pub(super) fn recall_family_channel(
    store: &CandidateMemoryStoreV2,
    query: &CandidateMemoryQueryV2,
    exact_key: &MemoryBucketKey,
) -> Result<FamilyRecallResult, ScaffoldContractError> {
    let ids = collect_shortlist(
        &neighbor_family_keys(exact_key),
        |key| store.family_index.get(key),
        store,
        exact_key.target_bins,
        MEMORY_FAMILY_SEARCH_CAP,
    );
    let eligible = ids.0;
    let searched = u32::try_from(ids.1.len()).unwrap_or(u32::MAX);
    let mut matches = ids
        .1
        .into_iter()
        .filter_map(|id| {
            let record = store.records.get(&id.raw())?;
            let score = family_similarity(query.features(), &record.query_features);
            (score >= MEMORY_MIN_SIMILARITY).then_some((id, score))
        })
        .collect::<Vec<_>>();
    sort_and_truncate_matches(&mut matches);
    let (values, confidence, source_count, best_source) =
        aggregate_family_matches(store, &matches)?;
    Ok(FamilyRecallResult {
        values,
        confidence,
        source_count,
        best_source,
        eligible,
        searched,
        matches: u16::try_from(matches.len())
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
    })
}

fn collect_shortlist<'a, K: Ord>(
    keys: &[K],
    lookup: impl Fn(&K) -> Option<&'a Vec<MemoryId>>,
    store: &'a CandidateMemoryStoreV2,
    query_bins: [i8; CANDIDATE_FEATURE_COUNT],
    cap: usize,
) -> (u32, Vec<MemoryId>) {
    let mut unique = BTreeSet::new();
    for key in keys {
        if let Some(ids) = lookup(key) {
            unique.extend(ids.iter().map(|id| id.raw()));
        }
    }
    let eligible = u32::try_from(unique.len()).unwrap_or(u32::MAX);
    let mut ids = unique.into_iter().map(MemoryId).collect::<Vec<_>>();
    ids.sort_by_key(|id| {
        let distance = store.records.get(&id.raw()).map_or(u32::MAX, |record| {
            target_bin_distance(query_bins, record.identity().exact_target_bins)
        });
        (distance, id.raw())
    });
    ids.truncate(cap);
    (eligible, ids)
}

pub(super) fn neighbor_family_keys(exact: &MemoryBucketKey) -> Vec<MemoryBucketKey> {
    neighbor_target_bins(exact.target_bins)
        .into_iter()
        .map(|target_bins| MemoryBucketKey {
            target_bins,
            ..exact.clone()
        })
        .collect()
}

pub(super) fn neighbor_target_keys(exact: &TargetMemoryBucketKey) -> Vec<TargetMemoryBucketKey> {
    neighbor_target_bins(exact.target_bins)
        .into_iter()
        .map(|target_bins| TargetMemoryBucketKey {
            target_bins,
            ..exact.clone()
        })
        .collect()
}

fn neighbor_target_bins(
    exact: [i8; CANDIDATE_FEATURE_COUNT],
) -> Vec<[i8; CANDIDATE_FEATURE_COUNT]> {
    let mut candidates = Vec::with_capacity(4);
    candidates.push(exact);
    let mut nearer = exact;
    nearer[2] = nearer[2].saturating_sub(1).max(-7);
    candidates.push(nearer);
    let mut farther = exact;
    farther[2] = farther[2].saturating_add(1).min(7);
    candidates.push(farther);
    let mut bearing_neutral = exact;
    bearing_neutral[0] = 0;
    bearing_neutral[1] = 0;
    candidates.push(bearing_neutral);
    let mut unique = Vec::with_capacity(4);
    for candidate in candidates {
        if !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }
    unique
}

fn target_bin_distance(
    left: [i8; CANDIDATE_FEATURE_COUNT],
    right: [i8; CANDIDATE_FEATURE_COUNT],
) -> u32 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| u32::from(left.abs_diff(right)))
        .sum()
}

pub(super) fn family_similarity(query: &[f32], record: &[f32]) -> f32 {
    0.30 * cosine_segment(query, record, 0..40)
        + 0.10 * cosine_segment(query, record, 40..49)
        + 0.20 * cosine_segment(query, record, 49..57)
        + 0.35 * cosine_segment(query, record, 57..81)
        + 0.05 * cosine_segment(query, record, 81..83)
}

fn target_similarity(query: &[f32], record: &[f32]) -> f32 {
    0.45 * cosine_segment(query, record, 0..40)
        + 0.50 * cosine_segment(query, record, 57..81)
        + 0.05 * cosine_segment(query, record, 81..83)
}

fn cosine_segment(query: &[f32], record: &[f32], range: std::ops::Range<usize>) -> f32 {
    let mut dot = 0.0;
    let mut query_norm = 0.0;
    let mut record_norm = 0.0;
    for index in range {
        dot += query[index] * record[index];
        query_norm += query[index] * query[index];
        record_norm += record[index] * record[index];
    }
    if query_norm == 0.0 && record_norm == 0.0 {
        1.0
    } else if query_norm == 0.0 || record_norm == 0.0 {
        0.0
    } else {
        (dot / (query_norm.sqrt() * record_norm.sqrt())).clamp(-1.0, 1.0)
    }
}

fn sort_and_truncate_matches(matches: &mut Vec<(MemoryId, f32)>) {
    matches.sort_by(|(left_id, left_score), (right_id, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left_id.raw().cmp(&right_id.raw()))
    });
    matches.truncate(MEMORY_RECALL_TOP_K);
}

fn aggregate_target_matches(
    store: &CandidateMemoryStoreV2,
    matches: &[(MemoryId, f32)],
) -> Result<
    (
        [f32; MEMORY_LATENT_V1_COUNT],
        Confidence,
        u16,
        Option<MemoryId>,
    ),
    ScaffoldContractError,
> {
    let mut output = [0.0; MEMORY_LATENT_V1_COUNT];
    let total = matches.iter().map(|(_, score)| *score).sum::<f32>();
    if total <= 0.0 {
        return Ok((output, Confidence::new(0.0)?, 0, None));
    }
    let mut weighted_confidence = 0.0;
    for (memory_id, score) in matches {
        let record = &store.records[&memory_id.raw()];
        let weight = *score / total;
        for (value, source) in output.iter_mut().zip(record.target_latent) {
            *value += source * weight;
        }
        weighted_confidence += record.confidence * weight;
    }
    let average_similarity = total / matches.len() as f32;
    Ok((
        output.map(|value| value.clamp(-1.0, 1.0)),
        Confidence::new((weighted_confidence * average_similarity).clamp(0.0, 1.0))?,
        u16::try_from(matches.len()).map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        matches.first().map(|(id, _)| *id),
    ))
}

fn aggregate_family_matches(
    store: &CandidateMemoryStoreV2,
    matches: &[(MemoryId, f32)],
) -> Result<
    (
        [f32; MEMORY_VALUE_V1_COUNT],
        Confidence,
        u16,
        Option<MemoryId>,
    ),
    ScaffoldContractError,
> {
    let mut output = [0.0; MEMORY_VALUE_V1_COUNT];
    let total = matches.iter().map(|(_, score)| *score).sum::<f32>();
    if total <= 0.0 {
        return Ok((output, Confidence::new(0.0)?, 0, None));
    }
    let mut weighted_confidence = 0.0;
    for (memory_id, score) in matches {
        let record = &store.records[&memory_id.raw()];
        let weight = *score / total;
        for (value, source) in output.iter_mut().zip(record.family_value) {
            *value += source * weight;
        }
        weighted_confidence += record.confidence * weight;
    }
    let average_similarity = total / matches.len() as f32;
    Ok((
        output.map(|value| value.clamp(-1.0, 1.0)),
        Confidence::new((weighted_confidence * average_similarity).clamp(0.0, 1.0))?,
        u16::try_from(matches.len()).map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        matches.first().map(|(id, _)| *id),
    ))
}

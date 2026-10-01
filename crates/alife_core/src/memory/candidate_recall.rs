use super::candidate_index::has_category_cues;
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
    let (valence, pain, disappointment) = memory_consequence(outcome);
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
    let danger = pain.max(disappointment).max((-valence).max(0.0));
    let family_value = [
        valence,
        if outcome.success { 1.0 } else { 0.0 },
        danger,
        outcome.energy_delta.raw(),
    ]
    .map(|value| value.clamp(-1.0, 1.0));
    let salience = valence
        .abs()
        .max(pain)
        .max(disappointment)
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
    if retained.identity() != observation.identity()
        || !compatible_memory_outcomes(retained, observation)
    {
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
    merged.source_sequence_id = ExperienceSequenceId(
        retained
            .source_sequence_id
            .raw()
            .max(observation.source_sequence_id.raw()),
    );
    merged.first_tick = Tick::new(retained.first_tick.raw().min(observation.first_tick.raw()));
    merged.last_tick = Tick::new(retained.last_tick.raw().max(observation.last_tick.raw()));
    merged.observation_count = new_count;
    merged.validate_contract()?;
    Ok(merged)
}

#[derive(Clone)]
pub(super) struct TargetRecallResult {
    pub(super) values: [f32; MEMORY_LATENT_V1_COUNT],
    pub(super) confidence: Confidence,
    pub(super) source_count: u16,
    pub(super) best_source: Option<MemoryId>,
    pub(super) eligible: u32,
    pub(super) searched: u32,
    pub(super) matches: u16,
    pub(super) category_read: bool,
}

#[derive(Clone)]
pub(super) struct FamilyRecallResult {
    pub(super) values: [f32; MEMORY_VALUE_V1_COUNT],
    pub(super) confidence: Confidence,
    pub(super) source_count: u16,
    pub(super) best_source: Option<MemoryId>,
    pub(super) eligible: u32,
    pub(super) searched: u32,
    pub(super) matches: u16,
    pub(super) category_read: bool,
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
            category_read: false,
        });
    }
    let ids = collect_shortlist(
        &neighbor_target_keys(exact_key),
        |key| store.target_namespace_index.get(key),
        store,
        exact_key.target_bins,
        MEMORY_TARGET_SEARCH_CAP,
    );
    let mut eligible = ids.0;
    let mut searched = u32::try_from(ids.1.len()).unwrap_or(u32::MAX);
    let mut matches = ids
        .1
        .into_iter()
        .filter_map(|id| {
            let record = store.records.get(&id.raw())?;
            let score = target_similarity(query.features(), &record.query_features);
            (score >= MEMORY_MIN_SIMILARITY).then_some((id, score))
        })
        .collect::<Vec<_>>();
    // A matching individual's episodes win. Only an unknown/mismatching
    // individual may borrow evidence from equivalent observable properties.
    let generalized = eligible == 0 && has_category_cues(query.features());
    if generalized {
        let key = exact_key.category_key(query.features());
        let category_ids = store.target_category_index.get(&key);
        eligible = eligible.saturating_add(category_ids.map_or(0, |ids| ids.len() as u32));
        for id in category_ids
            .into_iter()
            .flatten()
            .take(MEMORY_TARGET_SEARCH_CAP.saturating_sub(searched as usize))
        {
            searched += 1;
            let record = &store.records[&id.raw()];
            let score = target_similarity(query.features(), &record.query_features);
            if score >= MEMORY_MIN_SIMILARITY
                && equivalent_category_cues(query.features(), &record.query_features)
            {
                matches.push((*id, score));
            }
        }
    }
    sort_and_truncate_matches(&mut matches);
    let (values, confidence, source_count, best_source) =
        aggregate_target_matches(store, &matches, query.tick())?;
    let confidence = Confidence::new(confidence.raw() * if generalized { 0.75 } else { 1.0 })?;
    Ok(TargetRecallResult {
        values,
        confidence,
        source_count,
        best_source,
        eligible,
        searched,
        matches: u16::try_from(matches.len())
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        category_read: generalized,
    })
}

pub(super) fn recall_family_channel(
    store: &CandidateMemoryStoreV2,
    query: &CandidateMemoryQueryV2,
    exact_key: &MemoryBucketKey,
) -> Result<FamilyRecallResult, ScaffoldContractError> {
    let ids = collect_shortlist(
        &neighbor_family_keys(exact_key),
        |key| {
            if query.tracked_object_id().is_some() {
                store.family_namespace_index.get(key)
            } else {
                store.family_index.get(key)
            }
        },
        store,
        exact_key.target_bins,
        MEMORY_FAMILY_SEARCH_CAP,
    );
    let mut eligible = ids.0;
    let mut searched = u32::try_from(ids.1.len()).unwrap_or(u32::MAX);
    let mut matches = ids
        .1
        .into_iter()
        .filter_map(|id| {
            let record = store.records.get(&id.raw())?;
            let score = family_similarity(query.features(), &record.query_features);
            (score >= MEMORY_MIN_SIMILARITY).then_some((id, score))
        })
        .collect::<Vec<_>>();
    select_current_individual_outcome(store, &mut matches);
    let generalized =
        eligible == 0 && query.tracked_object_id().is_some() && has_category_cues(query.features());
    if generalized {
        let key = exact_key.category_key(query.features());
        let category_ids = store.family_category_index.get(&key);
        eligible = eligible.saturating_add(category_ids.map_or(0, |ids| ids.len() as u32));
        for id in category_ids
            .into_iter()
            .flatten()
            .take(MEMORY_FAMILY_SEARCH_CAP.saturating_sub(searched as usize))
        {
            searched += 1;
            let record = &store.records[&id.raw()];
            let score = family_similarity(query.features(), &record.query_features);
            if score >= MEMORY_MIN_SIMILARITY
                && equivalent_category_cues(query.features(), &record.query_features)
            {
                matches.push((*id, score));
            }
        }
    }
    sort_and_truncate_matches(&mut matches);
    let (values, confidence, source_count, best_source) =
        aggregate_family_matches(store, &matches, query.tick())?;
    let confidence = Confidence::new(confidence.raw() * if generalized { 0.75 } else { 1.0 })?;
    Ok(FamilyRecallResult {
        values,
        confidence,
        source_count,
        best_source,
        eligible,
        searched,
        matches: u16::try_from(matches.len())
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        category_read: generalized,
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
    let mut eligible = 0_u32;
    for key in keys {
        if let Some(ids) = lookup(key) {
            // A tracked-object query reads one ranked namespace. Untracked
            // queries retain disjoint exact-bin neighbors. Count all eligible
            // records, but read at most one search cap from each bucket before
            // applying the global similarity cap.
            eligible = eligible.saturating_add(u32::try_from(ids.len()).unwrap_or(u32::MAX));
            unique.extend(ids.iter().take(cap).map(|id| id.raw()));
        }
    }
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
    if exact.tracked_object_id_raw != 0 {
        return vec![exact.namespace_key()];
    }
    neighbor_target_bins(exact.target_bins)
        .into_iter()
        .map(|target_bins| MemoryBucketKey {
            target_bins,
            ..exact.clone()
        })
        .collect()
}

pub(super) fn neighbor_target_keys(exact: &TargetMemoryBucketKey) -> Vec<TargetMemoryBucketKey> {
    if exact.tracked_object_id_raw != 0 {
        return vec![exact.namespace_key()];
    }
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
    tick: Tick,
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
        weighted_confidence += record.confidence * weight * retention_factor(record, tick);
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
    tick: Tick,
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
        weighted_confidence += record.confidence * weight * retention_factor(record, tick);
    }
    let average_similarity = total / matches.len() as f32;
    Ok((
        output.map(|value| value.clamp(-1.0, 1.0)),
        Confidence::new((weighted_confidence * average_similarity).clamp(0.0, 1.0))?,
        u16::try_from(matches.len()).map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        matches.first().map(|(id, _)| *id),
    ))
}

// The target channel ignores action lanes, while family recall includes them.
// Keep exact floats in frame-local cache keys so close quantized observations
// cannot accidentally share evidence.
pub(super) fn target_recall_features(query: &CandidateMemoryQueryV2) -> [u32; 66] {
    std::array::from_fn(|lane| query.features()[if lane < 40 { lane } else { lane + 17 }].to_bits())
}

pub(super) fn compatible_memory_outcomes(
    left: &CandidateMemoryRecordV2,
    right: &CandidateMemoryRecordV2,
) -> bool {
    // Repeated appearance/action is not evidence that conflicting consequences
    // are equivalent. Preserve reversals and painful individual exceptions.
    left.family_value
        .iter()
        .zip(right.family_value)
        .all(|(left, right)| (*left - right).abs() <= 0.2)
        && left
            .target_latent
            .iter()
            .zip(right.target_latent)
            .all(|(left, right)| (*left - right).abs() <= 0.2)
}

pub(super) fn meaningful_memory_event(
    patch: &ExperiencePatch,
    record: &CandidateMemoryRecordV2,
    first_matching_event: bool,
) -> bool {
    let (valence, pain, disappointment) = memory_consequence(patch.outcome());
    valence.abs() >= 0.05
        || pain >= 0.02
        || (disappointment > 0.0 && patch.outcome().frustration_delta.raw() >= 0.15)
        || record.target_latent[..4]
            .iter()
            .any(|value| value.abs() >= 0.1)
        || record.target_latent[4].abs() >= 0.15
        || patch.outcome().prediction_error.raw() >= 0.15
        || matches!(
            patch.outcome().physical.contact,
            PhysicalContactKind::Consumed | PhysicalContactKind::Blocked
        )
        || (patch.outcome().physical.contact == PhysicalContactKind::Moved
            && matches!(
                patch.decision().selected_action.kind,
                ActionKind::Interact | ActionKind::Hold
            ))
        || (first_matching_event
            && matches!(
                patch.decision().selected_action.kind,
                ActionKind::Look | ActionKind::Inspect
            )
            && patch.pre_action().sensory().channels.novelty_signal.raw() >= 0.2)
}

fn equivalent_category_cues(query: &[f32], record: &[f32]) -> bool {
    // Each property group must agree. A shared "fruit" affordance or large
    // state similarity cannot conceal different colour or chemical evidence.
    (6..18).chain(std::iter::once(21)).all(|lane| {
        (query[MEMORY_TARGET_RANGE.start + lane] - record[MEMORY_TARGET_RANGE.start + lane]).abs()
            <= 0.2
    }) && (6..18).step_by(3).all(|lane| {
        cosine_segment(
            query,
            record,
            MEMORY_TARGET_RANGE.start + lane..MEMORY_TARGET_RANGE.start + lane + 3,
        ) >= 0.9
    })
}

// Tunable retention horizon, in simulation ticks. Strong measured danger/reward
// is retained at full strength; unused low-value evidence loses confidence and
// capacity priority. Reads do not mutate belief or persistence state.
const LOW_VALUE_RETENTION_HALF_LIFE_TICKS: f32 = 16_384.0;

pub(super) fn retention_factor(record: &CandidateMemoryRecordV2, tick: Tick) -> f32 {
    if consequence_strength(record) >= 0.5 {
        return 1.0;
    }
    let age = tick.raw().saturating_sub(record.last_tick.raw()) as f32;
    (1.0 / (1.0 + age / LOW_VALUE_RETENTION_HALF_LIFE_TICKS)).max(0.05)
}

fn consequence_strength(record: &CandidateMemoryRecordV2) -> f32 {
    record.family_value[0]
        .abs()
        .max(record.family_value[2])
        .max(record.target_latent[2].abs())
}

pub(super) fn retention_priority(record: &CandidateMemoryRecordV2, tick: Tick) -> u16 {
    let strength = consequence_strength(record);
    if strength >= 0.5 {
        32_768 + (strength * 32_767.0).round() as u16
    } else {
        (f32::from(record.salience_q16) * retention_factor(record, tick) * 0.499).round() as u16
    }
}

pub(super) fn corroborated_retrieval(
    patch: &ExperiencePatch,
    observation: &CandidateMemoryRecordV2,
) -> bool {
    let frame = patch.pre_action().perception();
    let offset = usize::from(
        patch
            .decision()
            .episodic_key()
            .unwrap()
            .query()
            .candidate_index(),
    ) * crate::MEMORY_CONTEXT_V1_LANES_PER_CANDIDATE;
    let Some(lanes) = frame
        .context()
        .values()
        .get(offset..offset + crate::MEMORY_CONTEXT_V1_LANES_PER_CANDIDATE)
    else {
        return false;
    };
    lanes[15] > 0.0
        && lanes[13] > 0.0
        && lanes[8..12]
            .iter()
            .zip(observation.family_value)
            .all(|(expected, measured)| (*expected - measured).abs() <= 0.2)
}

// Conflicting individual events remain stored, but a newer measured outcome
// under essentially the same query is the current expectation. This supports
// reversal without averaging a recent poison encounter into harmlessness.
fn select_current_individual_outcome(
    store: &CandidateMemoryStoreV2,
    matches: &mut Vec<(MemoryId, f32)>,
) {
    let best_similarity = matches.iter().map(|(_, score)| *score).fold(0.0, f32::max);
    let Some((anchor, _)) = matches
        .iter()
        .filter(|(_, score)| *score >= best_similarity - 0.02)
        .max_by_key(|(id, _)| {
            (
                store.records[&id.raw()].last_tick.raw(),
                store.records[&id.raw()].source_sequence_id.raw(),
                id.raw(),
            )
        })
    else {
        return;
    };
    let anchor = &store.records[&anchor.raw()];
    matches.retain(|(id, _)| compatible_memory_outcomes(anchor, &store.records[&id.raw()]));
}

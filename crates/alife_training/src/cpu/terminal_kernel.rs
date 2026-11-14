//! Experimental read-only terminal consumption context, not an action policy.
//! Appearance-only is an explicit lossy lookup, never a claim of known chemistry.
//! No current sensory lane, memory record, reward, or neural weight is modified.

pub const MAX_RECORDS: usize = 64;
pub const MAX_SOURCES: usize = 4;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Source {
    /// None identifies an ordinary memory ID. Channel observations use their
    /// sealed sequence as the collection-local ID and carry explicit provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<alife_core::MotorChannel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_digest: Option<[u64; 4]>,
    pub memory_id: u64,
    pub sequence_id: u64,
    pub action_id: u32,
    pub tracked_object_id: u64,
    pub last_tick: u64,
    pub observed_hunger_delta: f32,
    pub observed_danger: f32,
    pub confidence: f32,
    pub appearance: [f32; 9],
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct Context {
    /// Explicit terminal consumption sources. These are not Approach outcomes.
    pub sources: Vec<Source>,
    pub experienced_hunger_delta: f32,
    pub experienced_danger: f32,
    /// Discounted because contact chemistry and other properties are not compared.
    pub confidence: f32,
    pub hunger_now: f32,
    /// Need-sensitive summary, NOT a logit or immediate Approach prediction.
    pub desirability: f32,
    pub comparisons: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub enum Error {
    InvalidInput,
    BudgetExceeded,
}

/// Requires an explicitly chosen appearance-only policy at the adapter boundary.
/// This is a bounded aggregate over already-qualified actual consumption records.
/// No-match returns None, not a fabricated measured-neutral experience.
pub fn summarize(
    appearance: [f32; 9],
    hunger: f32,
    records: &[Source],
) -> Result<Option<Context>, Error> {
    if records.len() > MAX_RECORDS {
        return Err(Error::BudgetExceeded);
    }
    if !appearance
        .iter()
        .all(|v| v.is_finite() && (-1.0..=1.0).contains(v))
        || !hunger.is_finite()
        || !(0.0..=1.0).contains(&hunger)
    {
        return Err(Error::InvalidInput);
    }
    // A completely silent appearance is not enough evidence for generalization.
    if appearance.iter().all(|v| *v == 0.0) {
        return Ok(None);
    }
    let mut selected = Vec::new();
    for r in records {
        if r.memory_id == 0
            || r.sequence_id == 0
            || r.action_id == 0
            || r.tracked_object_id == 0
            || !r.observed_hunger_delta.is_finite()
            || !(-1.0..=1.0).contains(&r.observed_hunger_delta)
            || !r.observed_danger.is_finite()
            || !(0.0..=1.0).contains(&r.observed_danger)
            || !r.confidence.is_finite()
            || !(0.0..=1.0).contains(&r.confidence)
            || !r
                .appearance
                .iter()
                .all(|v| v.is_finite() && (-1.0..=1.0).contains(v))
        {
            return Err(Error::InvalidInput);
        }
        if appearance
            .iter()
            .zip(r.appearance)
            .all(|(a, b)| (a - b).abs() <= 0.05)
        {
            selected.push(*r);
        }
    }
    // Stable, bounded evidence selection, not target-ID preference or reward ranking.
    selected.sort_by_key(|r| (std::cmp::Reverse(r.last_tick), r.memory_id));
    selected.truncate(MAX_SOURCES);
    if selected.is_empty() {
        return Ok(None);
    }
    let total: f32 = selected.iter().map(|r| r.confidence).sum();
    if total <= 0.0 {
        return Ok(None);
    }
    let delta = selected
        .iter()
        .map(|r| r.observed_hunger_delta * r.confidence)
        .sum::<f32>()
        / total;
    let danger = selected
        .iter()
        .map(|r| r.observed_danger * r.confidence)
        .sum::<f32>()
        / total;
    // Fixed uncertainty discount, not reconstructed availability metadata.
    let confidence = 0.5 * total / selected.len() as f32;
    let relief = -delta;
    let desirability = confidence * (hunger * relief.max(0.0) + relief.min(0.0) - danger);
    Ok(Some(Context {
        sources: selected,
        experienced_hunger_delta: delta,
        experienced_danger: danger,
        confidence,
        hunger_now: hunger,
        desirability,
        comparisons: records.len() * 9,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(id: u64, appearance: [f32; 9], delta: f32) -> Source {
        Source {
            channel: None,
            observation_digest: None,
            memory_id: id,
            sequence_id: id,
            action_id: 210,
            tracked_object_id: id,
            last_tick: id,
            observed_hunger_delta: delta,
            observed_danger: 0.0,
            confidence: 1.0,
            appearance,
        }
    }
    const A: [f32; 9] = [0.75, 0.25, 0.75, 0.2, 0.2, 0.2, 0.25, 0.75, 0.25];
    const B: [f32; 9] = [0.2, 0.7, 0.1, 0.2, 0.2, 0.2, 0.7, 0.2, 0.4];
    #[test]
    fn counterbalanced_appearance_not_id() {
        for (food, toy) in [(A, B), (B, A)] {
            let records = [source(11, food, -0.3), source(12, toy, 0.1)];
            let good = summarize(food, 1.0, &records).unwrap().unwrap();
            let bad = summarize(toy, 1.0, &records).unwrap().unwrap();
            assert!(good.desirability > 0.0 && bad.desirability < 0.0);
            assert_eq!(good.sources[0].action_id, 210);
            assert_eq!(good.sources[0].sequence_id, 11);
        }
    }
    #[test]
    fn hunger_gates_relief_not_harm() {
        let mut records = [source(1, A, -0.4)];
        assert_eq!(
            summarize(A, 0.0, &records).unwrap().unwrap().desirability,
            0.0
        );
        assert!(summarize(A, 1.0, &records).unwrap().unwrap().desirability > 0.0);
        records[0].observed_danger = 0.2;
        assert!(summarize(A, 0.0, &records).unwrap().unwrap().desirability < 0.0);
    }
    #[test]
    fn unknown_is_not_neutral() {
        assert_eq!(summarize(A, 1.0, &[]).unwrap(), None);
        assert_eq!(summarize(B, 1.0, &[source(1, A, -0.4)]).unwrap(), None);
        assert_eq!(
            summarize([0.0; 9], 1.0, &[source(1, [0.0; 9], -0.4)]).unwrap(),
            None
        );
    }
    #[test]
    fn finite_bounded_and_budgeted() {
        assert_eq!(summarize(A, f32::NAN, &[]), Err(Error::InvalidInput));
        assert_eq!(
            summarize(A, 1.0, &[source(1, A, -0.4); 65]),
            Err(Error::BudgetExceeded)
        );
        let mut r = source(1, A, -0.4);
        r.confidence = f32::NAN;
        assert_eq!(summarize(A, 1.0, &[r]), Err(Error::InvalidInput));
    }
    #[test]
    fn repeat_exposure_preserves_sources_and_caps() {
        let records: Vec<_> = (1..=8).map(|id| source(id, A, -0.3)).collect();
        let before = records.clone();
        let c = summarize(A, 0.7, &records).unwrap().unwrap();
        assert_eq!(c.sources.len(), 4);
        assert_eq!(c.sources[0].memory_id, 8);
        assert_eq!(c.comparisons, 72);
        assert_eq!(records, before);
        assert!(c.confidence <= 0.5);
    }
}

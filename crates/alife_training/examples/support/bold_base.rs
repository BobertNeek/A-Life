//! Disposable, authored fresh N2048 base-brain candidate. No saved life is read.
use alife_core::*;
use serde_json::{json, Value};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn build(p: &BrainPhenotype, original: &[f32]) -> Result<(Vec<f32>, Vec<Value>)> {
    if original.len() != p.synapses().len()
        || p.foundation_abi_selection()
            .n2048_experimental_graph()
            .is_none()
        || p.sensor_profile() != SensorProfile::GroundedObjectSlotsV1
    {
        return Err("bold candidate requires the explicit experimental N2048 graph".into());
    }
    let mut w = original.to_vec();
    let mut edits = Vec::new();
    let mut change = |i: usize, after: f32, reason: &str| -> Result<()> {
        let before = *w.get(i).ok_or("weight index out of range")?;
        if !after.is_finite() {
            return Err("nonfinite authored weight".into());
        }
        w[i] = after;
        edits.push(json!({"index":i,"before":before,"after":after,"reason":reason}));
        Ok(())
    };
    for (i, expected) in [(25968usize, 4f32), (25983, 20.)] {
        let s = &p.synapses()[i];
        if original[i] != expected
            || !matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Approach && c.input_lane()==15)
        {
            return Err("Approach taste coordinate mismatch".into());
        }
        change(i, expected * 0.70, "approach-taste")?;
    }
    for (i, expected) in [(26927usize, 30f32), (27135, -60.)] {
        let s = &p.synapses()[i];
        if original[i] != expected
            || !matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Ingest)
        {
            return Err("Ingest reference coordinate mismatch".into());
        }
        change(i, expected * 1.25, "ingest-reference")?;
    }
    // Optional fresh-founder experiment: amplify the existing hunger path.
    // Never applies to a saved organism or to the candidate unless explicitly selected.
    if std::env::var_os("ALIFE_RESEARCH_HUNGER_GAIN").is_some() {
        for (i, expected, after) in [(6656usize, 2f32, 4f32), (7680, 2., 4.), (9502, 4., 8.)] {
            if original[i] != expected || p.synapses()[i].kind() != CompiledSynapseKind::Recurrent {
                return Err(format!("hunger pathway mismatch at {i}").into());
            }
            change(i, after, "hunger-sensitivity")?;
        }
    }
    let fatigue = &p.synapses()[9697];
    if original[9697] != 0.
        || fatigue.kind() != CompiledSynapseKind::Recurrent
        || fatigue.route_index() != 5
        || fatigue.source() != 899
        || fatigue.target() != 1779
    {
        return Err("fatigue route mismatch".into());
    }
    change(9697, 4., "fatigue-approach-relay")?;
    let mut fatigue_rows = 0;
    for (i, s) in p.synapses().iter().enumerate() {
        if matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Approach && (matches!(c.input_lane(),6..=14|18)) )
            && [1779, 1791].contains(&s.source())
        {
            let delta = if s.source() == 1779 { -0.4 } else { 0.4 };
            if s.source() == 1779 && original[i] != 0. {
                return Err("fatigue signal row already used".into());
            }
            if s.source() == 1791 && !matches!(original[i], 0. | 10. | -10.) {
                return Err(format!(
                    "fatigue reference row changed: index {i}, lane {}, value {}",
                    match s.kind() {
                        CompiledSynapseKind::Decoder(c) => c.input_lane(),
                        _ => 999,
                    },
                    original[i]
                )
                .into());
            }
            change(i, original[i] + delta, "fatigue-approach-readout")?;
            fatigue_rows += 1;
        }
    }
    if fatigue_rows != 20 {
        return Err("fatigue readout row count mismatch".into());
    }
    for (lane, gain, reason) in [
        (24u16, -2f32, "remembered-hunger-relief"),
        (26, -4., "remembered-pain"),
        (28, 2., "remembered-brain-atp"),
    ] {
        let ids=p.synapses().iter().enumerate().filter_map(|(i,s)| matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::MemoryContext && c.family()==CandidateActionFamily::Ingest && c.input_lane()==lane).then_some(i)).collect::<Vec<_>>();
        if ids.len() != if lane == 28 { 40 } else { 48 } || ids.iter().any(|&i| original[i] != 0.) {
            return Err(format!(
                "memory lane {lane}, count {}, nonzero {:?}",
                ids.len(),
                ids.iter()
                    .filter(|&&i| original[i] != 0.)
                    .map(|&i| (i, original[i]))
                    .take(3)
                    .collect::<Vec<_>>()
            )
            .into());
        }
        let per_weight = gain / ids.len() as f32;
        for i in ids {
            change(i, per_weight, reason)?;
        }
    }
    drop(change);
    Ok((w, edits))
}

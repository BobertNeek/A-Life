//! Historical authored recipes for an opt-in, controlled CPU comparison.
//!
//! Numeric recipe provenance: source commit 845cee6f6b39b588d81e0de3f2a190ad5f297144,
//! based on main 1c74ee33528f452895ae255caaac916b722d909c. These are source proposals,
//! not accepted brains or a production policy. See the neutral-contact study report.
//!
//! The contact term changes Ingest lane 18 on two signed motor activations. The
//! study found improved neutral-contact food intake and repeated failed ingestion
//! of matched nonfood; it did not establish hunger-exclusive consumption.
//! Approach lanes 6..=14 are RGB, material and shape, not edible affordances.
//! Their readouts can affect generic objects, including obstacles. A raw negative
//! distance weight does not establish the effective slope with signed activations.
//! Full-v2 also changes memory readouts, so it is a confounded comparison. Memory
//! remains family-isolated, bounded by the existing +/-0.5 contribution clamp,
//! with the existing same-object fallback caveat. No cross-family knowledge or
//! general navigation improvement is established by adding these readouts.
//!
//! This helper never reads or mutates a saved life. Comments distinguish the
//! historical design intent from the measured outcomes; numeric code is retained.

use alife_core::*;
use serde_json::{json, Value};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub const INGEST_SIGNAL_MOTOR: u32 = 1824;
pub const INGEST_REFERENCE_MOTOR: u32 = 1839;
pub const APPROACH_HUNGER_SIGNAL_MOTOR: u32 = 1776;
pub const APPROACH_FATIGUE_SIGNAL_MOTOR: u32 = 1779;
pub const APPROACH_REFERENCE_MOTOR: u32 = 1791;
pub const NONCONTACT_PHYSICAL_OBJECT_LANES: [u16; 9] = [6, 7, 8, 9, 10, 11, 12, 13, 14];

/// Reconstructs the authored `whole-care-bold-v1` weight vector from `whole_care` v6.
pub fn build_bold_v1(p: &BrainPhenotype, whole_care_v6: &[f32]) -> Result<(Vec<f32>, Vec<Value>)> {
    if whole_care_v6.len() != p.synapses().len()
        || p.foundation_abi().n2048_experimental_graph().is_none()
    {
        return Err("bold_v1 requires the explicit experimental N2048 graph".into());
    }
    let mut w = whole_care_v6.to_vec();
    let mut edits = Vec::new();
    let mut change = |i: usize, after: f32, reason: &str| -> Result<()> {
        let before = *w.get(i).ok_or("weight index out of range")?;
        if !after.is_finite() {
            return Err("nonfinite authored weight".into());
        }
        w[i] = after;
        edits.push(json!({"index": i, "before": before, "after": after, "reason": reason}));
        Ok(())
    };

    for (i, expected) in [(25968usize, 4.0f32), (25983, 20.0)] {
        let s = &p.synapses()[i];
        if whole_care_v6[i] != expected
            || !matches!(
                s.kind(),
                CompiledSynapseKind::Decoder(c)
                    if c.head() == DecoderHeadKind::ActionCandidate
                        && c.family() == CandidateActionFamily::Approach
                        && c.input_lane() == 15
            )
        {
            return Err("Approach taste coordinate mismatch".into());
        }
        change(i, expected * 0.70, "approach-taste")?;
    }

    for (i, expected) in [(26927usize, 30.0f32), (27135, -60.0)] {
        let s = &p.synapses()[i];
        if whole_care_v6[i] != expected
            || !matches!(
                s.kind(),
                CompiledSynapseKind::Decoder(c)
                    if c.head() == DecoderHeadKind::ActionCandidate
                        && c.family() == CandidateActionFamily::Ingest
            )
        {
            return Err("Ingest reference coordinate mismatch".into());
        }
        change(i, expected * 1.25, "ingest-reference")?;
    }

    let fatigue = &p.synapses()[9697];
    if whole_care_v6[9697] != 0.0
        || fatigue.kind() != CompiledSynapseKind::Recurrent
        || fatigue.route_index() != 5
        || fatigue.source() != 899
        || fatigue.target() != APPROACH_FATIGUE_SIGNAL_MOTOR
    {
        return Err("fatigue route mismatch".into());
    }
    change(9697, 4.0, "fatigue-approach-relay")?;

    let mut fatigue_rows = 0usize;
    for (i, s) in p.synapses().iter().enumerate() {
        if matches!(
            s.kind(),
            CompiledSynapseKind::Decoder(c)
                if c.head() == DecoderHeadKind::ActionCandidate
                    && c.family() == CandidateActionFamily::Approach
                    && matches!(c.input_lane(), 6..=14 | 18)
        ) && [APPROACH_FATIGUE_SIGNAL_MOTOR, APPROACH_REFERENCE_MOTOR].contains(&s.source())
        {
            let delta = if s.source() == APPROACH_FATIGUE_SIGNAL_MOTOR {
                -0.4
            } else {
                0.4
            };
            if s.source() == APPROACH_FATIGUE_SIGNAL_MOTOR && whole_care_v6[i] != 0.0 {
                return Err("fatigue signal row already used".into());
            }
            change(i, whole_care_v6[i] + delta, "fatigue-approach-readout")?;
            fatigue_rows += 1;
        }
    }
    if fatigue_rows != 20 {
        return Err("fatigue readout row count mismatch".into());
    }

    for (lane, gain, reason) in [
        (24u16, -2.0f32, "remembered-hunger-relief"),
        (26, -4.0, "remembered-pain"),
        (28, 2.0, "remembered-brain-atp"),
    ] {
        set_memory_lane_total_gain(
            p,
            &mut w,
            &mut edits,
            CandidateActionFamily::Ingest,
            lane,
            gain,
            reason,
        )?;
    }

    Ok((w, edits))
}

/// Reconstructs the complete historical v2 proposal for comparison, not adoption.
/// Adds contact, generic appearance and family-isolated memory readouts.
pub fn build_bold_v2(p: &BrainPhenotype, whole_care_v6: &[f32]) -> Result<(Vec<f32>, Vec<Value>)> {
    let (mut w, mut edits) = build_bold_v1(p, whole_care_v6)?;

    // Contact readout on Ingest lane 18. Effective response depends on both
    // signed motor activations; these constants do not enforce an appetite gate.
    let mut ingest_contact_rows = 0usize;
    for (i, s) in p.synapses().iter().enumerate() {
        let CompiledSynapseKind::Decoder(c) = s.kind() else {
            continue;
        };
        if c.head() == DecoderHeadKind::ActionCandidate
            && c.family() == CandidateActionFamily::Ingest
            && c.input_lane() == 18
        {
            if s.source() == INGEST_SIGNAL_MOTOR {
                let before = w[i];
                w[i] = 8.0;
                edits.push(json!({
                    "index": i,
                    "before": before,
                    "after": w[i],
                    "reason": "ingest-contact-hunger-signal"
                }));
                ingest_contact_rows += 1;
            } else if s.source() == INGEST_REFERENCE_MOTOR {
                let before = w[i];
                w[i] = -42.0;
                edits.push(json!({
                    "index": i,
                    "before": before,
                    "after": w[i],
                    "reason": "ingest-contact-reference"
                }));
                ingest_contact_rows += 1;
            }
        }
    }
    if ingest_contact_rows != 2 {
        return Err("expected exactly 2 Ingest lane-18 contact decoder rows".into());
    }

    // Generic RGB/material/shape readouts on Approach lanes 6..=14.
    // No edible-category information is supplied. Preserve the historical distance
    // coefficient without interpreting its raw sign as the effective slope.
    let mut approach_physical_rows = 0usize;
    for (i, s) in p.synapses().iter().enumerate() {
        let CompiledSynapseKind::Decoder(c) = s.kind() else {
            continue;
        };
        if c.head() == DecoderHeadKind::ActionCandidate
            && c.family() == CandidateActionFamily::Approach
            && NONCONTACT_PHYSICAL_OBJECT_LANES.contains(&c.input_lane())
        {
            if s.source() == APPROACH_HUNGER_SIGNAL_MOTOR {
                let before = w[i];
                w[i] += 0.8;
                edits.push(json!({
                    "index": i,
                    "before": before,
                    "after": w[i],
                    "reason": "approach-physical-hunger-signal"
                }));
                approach_physical_rows += 1;
            } else if s.source() == APPROACH_REFERENCE_MOTOR {
                let before = w[i];
                w[i] -= 0.4;
                edits.push(json!({
                    "index": i,
                    "before": before,
                    "after": w[i],
                    "reason": "approach-physical-reference-balance"
                }));
                approach_physical_rows += 1;
            }
        }
    }
    if approach_physical_rows != 18 {
        return Err("expected 18 Approach generic appearance rows across lanes 6..=14".into());
    }

    // 3. Separate per-family MemoryContext readouts on target_latent (lanes 24..28)
    // and Consumed-filtered / outcome-filtered family_value (lanes 32..35).
    for (family, lane, gain, reason) in [
        // Ingest: add threat (25) and Consumed-filtered family_value (32..=35)
        (
            CandidateActionFamily::Ingest,
            25u16,
            -2.0f32,
            "ingest-remembered-threat",
        ),
        (
            CandidateActionFamily::Ingest,
            32,
            3.0,
            "ingest-family-valence",
        ),
        (
            CandidateActionFamily::Ingest,
            33,
            1.0,
            "ingest-family-success",
        ),
        (
            CandidateActionFamily::Ingest,
            34,
            -6.0,
            "ingest-family-danger",
        ),
        (
            CandidateActionFamily::Ingest,
            35,
            3.0,
            "ingest-family-energy-delta",
        ),
        // Approach: steer toward nourishing cues and away from painful/blocked objects before contact
        (
            CandidateActionFamily::Approach,
            24,
            -1.5,
            "approach-remembered-hunger-relief",
        ),
        (
            CandidateActionFamily::Approach,
            25,
            -2.0,
            "approach-remembered-threat",
        ),
        (
            CandidateActionFamily::Approach,
            26,
            -3.5,
            "approach-remembered-pain",
        ),
        (
            CandidateActionFamily::Approach,
            28,
            1.5,
            "approach-remembered-brain-atp",
        ),
        (
            CandidateActionFamily::Approach,
            32,
            2.0,
            "approach-family-valence",
        ),
        (
            CandidateActionFamily::Approach,
            33,
            2.0,
            "approach-family-success",
        ),
        (
            CandidateActionFamily::Approach,
            34,
            -4.5,
            "approach-family-danger",
        ),
        (
            CandidateActionFamily::Approach,
            35,
            2.0,
            "approach-family-energy-delta",
        ),
        // Contact: mirror Approach memory sensitivity for manipulation/touch
        (
            CandidateActionFamily::Contact,
            24,
            -1.5,
            "contact-remembered-hunger-relief",
        ),
        (
            CandidateActionFamily::Contact,
            25,
            -2.0,
            "contact-remembered-threat",
        ),
        (
            CandidateActionFamily::Contact,
            26,
            -3.5,
            "contact-remembered-pain",
        ),
        (
            CandidateActionFamily::Contact,
            28,
            1.5,
            "contact-remembered-brain-atp",
        ),
        (
            CandidateActionFamily::Contact,
            32,
            2.0,
            "contact-family-valence",
        ),
        (
            CandidateActionFamily::Contact,
            33,
            1.5,
            "contact-family-success",
        ),
        (
            CandidateActionFamily::Contact,
            34,
            -4.5,
            "contact-family-danger",
        ),
        (
            CandidateActionFamily::Contact,
            35,
            2.0,
            "contact-family-energy-delta",
        ),
        // Avoid: positive excitation from remembered threat, pain, and danger
        (
            CandidateActionFamily::Avoid,
            25,
            2.5,
            "avoid-remembered-threat",
        ),
        (
            CandidateActionFamily::Avoid,
            26,
            4.0,
            "avoid-remembered-pain",
        ),
        (
            CandidateActionFamily::Avoid,
            32,
            -2.0,
            "avoid-family-valence",
        ),
        (CandidateActionFamily::Avoid, 34, 5.0, "avoid-family-danger"),
    ] {
        set_memory_lane_total_gain(p, &mut w, &mut edits, family, lane, gain, reason)?;
    }

    Ok((w, edits))
}

fn set_memory_lane_total_gain(
    p: &BrainPhenotype,
    w: &mut [f32],
    edits: &mut Vec<Value>,
    family: CandidateActionFamily,
    lane: u16,
    total_gain: f32,
    reason: &str,
) -> Result<()> {
    let ids = p
        .synapses()
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            matches!(
                s.kind(),
                CompiledSynapseKind::Decoder(c)
                    if c.head() == DecoderHeadKind::MemoryContext
                        && c.family() == family
                        && c.input_lane() == lane
            )
            .then_some(i)
        })
        .collect::<Vec<_>>();
    let expected_count = if lane < 28 { 48 } else { 40 };
    if ids.len() != expected_count {
        return Err(format!(
            "memory lane {lane} for {family:?}: expected {expected_count} synapses, found {}",
            ids.len()
        )
        .into());
    }
    let per_weight = total_gain / ids.len() as f32;
    for i in ids {
        let before = w[i];
        w[i] = per_weight;
        edits.push(json!({
            "index": i,
            "before": before,
            "after": per_weight,
            "reason": reason
        }));
    }
    Ok(())
}

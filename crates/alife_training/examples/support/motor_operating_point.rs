//! Explicit weight-only research candidate. No inherited dendrite or ABI edits.
use alife_core::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Factorial controls keep frozen whole-care6 as the starting point. Negative
/// tonic state is produced by a signed Modulatory edge; motor edges stay positive.
pub fn counterbalance(
    p: &BrainPhenotype,
    original: &[f32],
    circuits: &[Value],
    gain: f32,
    differential_readout: bool,
) -> Result<(Vec<f32>, Vec<Value>)> {
    if original.len() != p.synapses().len()
        || p.foundation_abi_selection()
            .n2048_experimental_graph()
            .is_none()
        || p.sensor_profile() != SensorProfile::GroundedObjectSlotsV1
        || p.projections()
            .get(3)
            .is_none_or(|r| r.projection_type() != ProjectionType::Modulatory)
        || p.projections()
            .get(5)
            .is_none_or(|r| r.projection_type() != ProjectionType::MotorProposal)
    {
        return Err(
            "operating-point candidate requires its explicit research graph and weight layout"
                .into(),
        );
    }
    if !gain.is_finite() || !(0.0..=8.0).contains(&gain) {
        return Err("counterbalance gain outside declared sweep".into());
    }
    let mut w = original.to_vec();
    let mut edits = vec![];
    let mut occupied = BTreeSet::new();
    for c in circuits.iter().filter(|c| c["role"] == "need") {
        let bridge = c["bridge_synapses"][1]
            .as_u64()
            .ok_or("missing frozen bridge")? as usize;
        occupied.insert(p.synapses()[bridge].target());
    }
    let tonic = circuits
        .iter()
        .find(|c| c["role"] == "tonic")
        .ok_or("missing frozen tonic")?;
    let first = tonic["bridge_synapses"][0]
        .as_u64()
        .ok_or("missing tonic source")? as usize;
    let reservoir = p.synapses()[first].target();
    for family in [
        CandidateActionFamily::Rest,
        CandidateActionFamily::Inspect,
        CandidateActionFamily::Approach,
    ] {
        let family_value = serde_json::to_value(family)?;
        let record = circuits
            .iter()
            .find(|c| c["role"] == "reference" && c["family"] == family_value)
            .ok_or("missing family reference")?;
        let reference = record["reference_motor"]
            .as_u64()
            .ok_or("missing reference motor")? as u32;
        let mut motors = record["signal_motors"]
            .as_array()
            .ok_or("missing signal motors")?
            .iter()
            .map(|v| v.as_u64().map(|v| v as u32).ok_or("invalid motor"))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        motors.push(reference);
        if differential_readout && family != CandidateActionFamily::Rest {
            for (i, s) in p.synapses().iter().enumerate() {
                if s.source() == reference
                    && matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==family)
                    && w[i] != 0.
                {
                    let before = w[i];
                    w[i] *= -0.2; // frozen +5 reference becomes a -1 contrast
                    edits.push(json!({"role":"differential_reference","family":family,"synapse":i,"before":before,"after":w[i]}));
                }
            }
        }
        if gain == 0. {
            continue;
        }
        for motor in motors {
            let mut found = None;
            for (middle, edge) in p.synapses().iter().enumerate() {
                if edge.kind() != CompiledSynapseKind::Recurrent
                    || edge.route_index() != 3
                    || edge.source() != reservoir
                    || occupied.contains(&edge.target())
                {
                    continue;
                }
                if let Some((output, _)) = p.synapses().iter().enumerate().find(|(_, s)| {
                    s.kind() == CompiledSynapseKind::Recurrent
                        && s.route_index() == 5
                        && s.source() == edge.target()
                        && s.target() == motor
                }) {
                    found = Some((middle, output, edge.target()));
                    break;
                }
            }
            let (middle, output, relay) =
                found.ok_or("no noncolliding signed tonic path to motor")?;
            if original[middle] != 0. || original[output] != 0. {
                return Err("counterbalance would overwrite an existing non-need path".into());
            }
            if original[first] != 8. {
                return Err("frozen tonic source identity changed".into());
            }
            // Positive Homeostatic input, negative signed Modulatory state,
            // positive MotorProposal output. No negative motor projection.
            w[first] = 8.;
            w[middle] = -8.;
            w[output] = gain;
            edits.push(json!({"role":"tonic_counterbalance","family":family,"motor":motor,"is_reference":motor==reference,"relay":relay,"first":first,"middle":middle,"output":output,"weights":[8.,-8.,gain]}));
        }
    }
    for (i, s) in p.synapses().iter().enumerate() {
        if s.kind() == CompiledSynapseKind::Recurrent
            && p.projections()[s.route_index() as usize].projection_type()
                == ProjectionType::MotorProposal
            && w[i] < 0.
        {
            return Err("negative MotorProposal weight rejected".into());
        }
    }
    Ok((w, edits))
}

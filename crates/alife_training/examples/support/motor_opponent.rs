//! Prospective weight-only fatigue opponent circuit. No fictional tonic input.
use alife_core::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum OpponentFactor {
    Original,
    Opponent,
    SourceCut,
    RelayCut,
}
/// The one declared circuit drives Rest's reference down and Approach's signal
/// down. Existing need relays and nonzero edges cannot be overwritten.
pub fn build(
    p: &BrainPhenotype,
    original: &[f32],
    circuits: &[Value],
    factor: OpponentFactor,
) -> Result<(Vec<f32>, Value)> {
    if p.foundation_abi_selection()
        .n2048_experimental_graph()
        .is_none()
        || original.len() != p.synapses().len()
    {
        return Err("opponent recipe requires the explicit research graph".into());
    }
    let input = p
        .sensor_encoder()
        .assignments()
        .iter()
        .find(|s| {
            s.source_group() == SensorEncoderSourceGroup::Homeostasis && s.source_index() == 1
        })
        .ok_or("missing fatigue input")?
        .target_neuron();
    let mut reserved_reserve = BTreeSet::new();
    let mut reserved_temporal = BTreeSet::new();
    for c in circuits
        .iter()
        .filter(|c| c["role"] == "need" || c["role"] == "tonic")
    {
        let first = c["bridge_synapses"][0]
            .as_u64()
            .ok_or("missing frozen first edge")? as usize;
        let second = c["bridge_synapses"][1]
            .as_u64()
            .ok_or("missing frozen second edge")? as usize;
        reserved_reserve.insert(p.synapses()[first].target());
        reserved_temporal.insert(p.synapses()[second].target());
    }
    let rest = circuits
        .iter()
        .find(|c| {
            c["role"] == "reference"
                && c["family"] == serde_json::to_value(CandidateActionFamily::Rest).unwrap()
        })
        .ok_or("missing Rest reference")?["reference_motor"]
        .as_u64()
        .ok_or("missing Rest motor")? as u32;
    let approach = circuits
        .iter()
        .find(|c| {
            c["role"] == "need"
                && c["family"] == serde_json::to_value(CandidateActionFamily::Approach).unwrap()
        })
        .ok_or("missing Approach signal")?["motor_neuron"]
        .as_u64()
        .ok_or("missing Approach motor")? as u32;
    let edge = |route: u16, from: u32, to: u32| {
        p.synapses()
            .iter()
            .enumerate()
            .find(|(_, s)| {
                s.kind() == CompiledSynapseKind::Recurrent
                    && s.route_index() == route
                    && s.source() == from
                    && s.target() == to
            })
            .map(|(i, _)| i)
    };
    let mut found = None;
    'paths: for (first, a) in p.synapses().iter().enumerate() {
        if a.kind() != CompiledSynapseKind::Recurrent
            || a.route_index() != 2
            || a.source() != input
            || reserved_reserve.contains(&a.target())
            || original[first] != 0.
        {
            continue;
        }
        for (second, b) in p.synapses().iter().enumerate() {
            if b.kind() != CompiledSynapseKind::Recurrent
                || b.route_index() != 3
                || b.source() != a.target()
                || reserved_temporal.contains(&b.target())
                || original[second] != 0.
            {
                continue;
            }
            if let (Some(r), Some(m)) = (edge(5, b.target(), rest), edge(5, b.target(), approach)) {
                if original[r] == 0. && original[m] == 0. {
                    found = Some((first, second, r, m));
                    break 'paths;
                }
            }
        }
    }
    let (first, second, r, m) =
        found.ok_or("no compatible unused fatigue opponent path; graph unchanged")?;
    let mut w = original.to_vec();
    if factor != OpponentFactor::Original {
        w[first] = if factor == OpponentFactor::SourceCut {
            0.
        } else {
            2.
        };
        w[second] = if factor == OpponentFactor::RelayCut {
            0.
        } else {
            -2.
        };
        w[r] = 6.;
        w[m] = 6.;
    }
    for (i, s) in p.synapses().iter().enumerate() {
        if s.kind() == CompiledSynapseKind::Recurrent
            && matches!(
                p.projections()[s.route_index() as usize].projection_type(),
                ProjectionType::MotorProposal | ProjectionType::Homeostatic
            )
            && w[i] < 0.
        {
            return Err("positive projection sign contract violated".into());
        }
    }
    let plan = json!({"schema":"fatigue_opponent_weights_v1","factor":factor,"fatigue_lane":1,"source":input,"reserve":p.synapses()[first].target(),"relay":p.synapses()[second].target(),"rest_reference":rest,"approach_signal":approach,"edges":[first,second,r,m],"edge_weights":[w[first],w[second],w[r],w[m]],"shared_relay":true,"existing_nonzero_edges_overwritten":false});
    Ok((w, plan))
}

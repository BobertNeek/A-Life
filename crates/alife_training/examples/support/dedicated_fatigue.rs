//! Explicit fresh-asset fatigue readout. No runtime, graph, or acquired-state edits.
use alife_core::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub const B_LANES: [u16; 10] = [6, 7, 8, 9, 10, 11, 12, 13, 14, 18];
pub const SIGNAL: u32 = 1779;
pub const REFERENCE: u32 = 1791;
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Factor {
    Baseline,
    Dedicated,
    SourceCut,
    SourceCutBaseline,
    RelayCut,
    RelayCutBaseline,
    ReadoutCut,
}
impl Factor {
    pub const ALL: [Self; 7] = [
        Self::Baseline,
        Self::Dedicated,
        Self::SourceCut,
        Self::SourceCutBaseline,
        Self::RelayCut,
        Self::RelayCutBaseline,
        Self::ReadoutCut,
    ];
    fn active(self) -> bool {
        !matches!(
            self,
            Self::Baseline | Self::SourceCutBaseline | Self::RelayCutBaseline
        )
    }
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct DrivePath {
    pub lane: u16,
    pub source: u32,
    pub reserve: u32,
    pub relay: u32,
    pub source_edge: usize,
    pub relay_edge: usize,
    pub output_edges: Vec<usize>,
    pub motors: Vec<u32>,
}
fn edge(p: &BrainPhenotype, route: u16, source: u32, target: u32) -> Result<usize> {
    let matches = p
        .synapses()
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            s.kind() == CompiledSynapseKind::Recurrent
                && s.route_index() == route
                && s.source() == source
                && s.target() == target
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!("expected exactly one edge ({route},{source},{target})").into());
    }
    Ok(matches[0])
}
/// Resolve all eleven actual compiler tuples, never extrapolate packed offsets.
pub fn drive_paths(p: &BrainPhenotype) -> Result<Vec<DrivePath>> {
    if p.foundation_abi_selection()
        .n2048_experimental_graph()
        .is_none()
        || p.sensor_profile() != SensorProfile::GroundedObjectSlotsV1
        || p.projections()[2].projection_type() != ProjectionType::Homeostatic
        || p.projections()[3].projection_type() != ProjectionType::Modulatory
        || p.projections()[5].projection_type() != ProjectionType::MotorProposal
    {
        return Err("requires the explicit repaired N2048 graph and grounded profile".into());
    }
    let mut paths = vec![];
    for (lane, source) in [256, 259, 260, 257, 261, 262, 263, 258, 264, 265, 266]
        .into_iter()
        .enumerate()
    {
        let roots = p
            .sensor_encoder()
            .assignments()
            .iter()
            .filter(|s| {
                s.source_group() == SensorEncoderSourceGroup::Homeostasis
                    && s.source_index() == lane as u16
            })
            .map(|s| s.target_neuron())
            .collect::<BTreeSet<_>>();
        if roots != BTreeSet::from([source]) {
            return Err(format!("drive {lane} encoder identity changed").into());
        }
        let offset = source - 256;
        let reserve = 1952 + offset;
        let relay = 896 + offset;
        let mut motors = vec![];
        let mut output_edges = vec![];
        for raw in 0..8 {
            let family = CandidateActionFamily::try_from_raw(raw)?;
            let motor = 1728 + u32::from(raw) * 16 + offset;
            let rows = p
                .synapses()
                .iter()
                .filter_map(|s| match s.kind() {
                    CompiledSynapseKind::Decoder(c)
                        if c.head() == DecoderHeadKind::ActionCandidate
                            && c.family() == family
                            && s.source() == motor =>
                    {
                        Some(c.input_lane())
                    }
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            if rows != (0..24).collect() {
                return Err("drive motor does not own exactly the raw24 rows".into());
            }
            motors.push(motor);
            output_edges.push(edge(p, 5, relay, motor)?);
        }
        paths.push(DrivePath {
            lane: lane as u16,
            source,
            reserve,
            relay,
            source_edge: edge(p, 2, source, reserve)?,
            relay_edge: edge(p, 3, reserve, relay)?,
            output_edges,
            motors,
        });
    }
    Ok(paths)
}
pub fn physical_basis(features: CandidateFeatureVector) -> Result<f32> {
    features.validate()?;
    if B_LANES.iter().any(|&lane| features.0[lane as usize] < 0.) {
        return Err("B requires grounded nonnegative color/material/shape/contact".into());
    }
    Ok(B_LANES.iter().map(|&i| features.0[i as usize]).sum::<f32>() / 10.)
}
pub fn build(p: &BrainPhenotype, original: &[f32], factor: Factor) -> Result<(Vec<f32>, Value)> {
    if original.len() != p.synapses().len() {
        return Err("weight length mismatch".into());
    }
    let paths = drive_paths(p)?;
    let path = &paths[1];
    let output = path.output_edges[CandidateActionFamily::Approach.raw() as usize];
    if (path.source_edge, path.relay_edge, output) != (6680, 7713, 9697)
        || (path.source, path.reserve, path.relay, path.motors[3]) != (259, 1955, 899, SIGNAL)
        || original[path.source_edge] != 2.
        || original[path.relay_edge] != 2.
        || original[output] != 0.
    {
        return Err("frozen fatigue source/relay or unused output prior changed".into());
    }
    let mut readout_rows = vec![];
    let mut added = vec![];
    for (i, s) in p.synapses().iter().enumerate() {
        let CompiledSynapseKind::Decoder(c) = s.kind() else {
            continue;
        };
        if c.head() != DecoderHeadKind::ActionCandidate
            || c.family() != CandidateActionFamily::Approach
        {
            continue;
        }
        if s.source() == SIGNAL && original[i] != 0. {
            return Err("dedicated motor readout is not unused".into());
        }
        if B_LANES.contains(&c.input_lane()) && [SIGNAL, REFERENCE].contains(&s.source()) {
            // The existing hunger contrast already owns reference lane18.
            // Preserve that +2 coefficient and add our independent +.4 term.
            let expected_prior = if s.source() == REFERENCE && c.input_lane() == 18 {
                2.
            } else {
                0.
            };
            if original[i] != expected_prior {
                return Err("dedicated row or shared reference prior changed".into());
            }
            readout_rows.push((i, if s.source() == SIGNAL { -0.4 } else { 0.4 }));
        }
    }
    if readout_rows.len() != 20 {
        return Err("expected ten dedicated and ten reference rows".into());
    }
    let mut w = original.to_vec();
    if factor.active() {
        w[output] = 4.;
    }
    if factor.active() && factor != Factor::ReadoutCut {
        for &(i, value) in &readout_rows {
            w[i] += value;
            added.push(i);
        }
    }
    if matches!(factor, Factor::SourceCut | Factor::SourceCutBaseline) {
        w[path.source_edge] = 0.;
    }
    if matches!(factor, Factor::RelayCut | Factor::RelayCutBaseline) {
        w[path.relay_edge] = 0.;
    }
    let allowed = BTreeSet::from_iter(readout_rows.iter().map(|(i, _)| *i).chain([
        output,
        path.source_edge,
        path.relay_edge,
    ]));
    for (i, (a, b)) in w.iter().zip(original).enumerate() {
        if a.to_bits() != b.to_bits() && !allowed.contains(&i) {
            return Err("undeclared weight edit".into());
        }
        let s = &p.synapses()[i];
        if s.kind() == CompiledSynapseKind::Recurrent
            && matches!(
                p.projections()[s.route_index() as usize].projection_type(),
                ProjectionType::Homeostatic | ProjectionType::MotorProposal
            )
            && *a < 0.
        {
            return Err("positive projection sign violation".into());
        }
    }
    Ok((
        w,
        json!({"schema":"dedicated_fatigue_nonnegative_basis_v1","factor":factor,
        "drive_paths":paths,"fatigue_path":path,"approach_signal":SIGNAL,"approach_reference":REFERENCE,
        "new_output_edge":output,"output_gain":if factor.active(){4.}else{0.},"basis_lanes":B_LANES,
        "basis_divisor":10.,"decoder_gain":-4.,"readout_rows":readout_rows,"added_readout_rows":added,"reference_rows_are_additive":true,"shared_reference_contact_prior":2.,
        "readiness_is_distress":false,"unbound_extension_is_distress":false,
        "cuts_affect_shared_fatigue_rest_path":matches!(factor,Factor::SourceCut|Factor::SourceCutBaseline|Factor::RelayCut|Factor::RelayCutBaseline),
        "runtime_activation":false,"acquired_state_changed":false}),
    ))
}

/// Independent decoder calibration. Existing Approach/Inspect differential
/// references are untouched; only the three remaining frozen -5 references vary.
pub fn full_reference_calibration(
    p: &BrainPhenotype,
    original: &[f32],
    circuits: &[Value],
    enabled: bool,
) -> Result<(Vec<f32>, Value)> {
    if original.len() != p.synapses().len() {
        return Err("weight length mismatch".into());
    }
    let mut w = original.to_vec();
    let mut edits = vec![];
    for family in [
        CandidateActionFamily::Avoid,
        CandidateActionFamily::Contact,
        CandidateActionFamily::Ingest,
    ] {
        let record = circuits
            .iter()
            .find(|c| {
                c["role"] == "reference" && c["family"] == serde_json::to_value(family).unwrap()
            })
            .ok_or("missing frozen reference record")?;
        let reference = record["reference_motor"]
            .as_u64()
            .ok_or("missing reference motor")? as u32;
        let signals = record["signal_motors"]
            .as_array()
            .ok_or("missing signal motors")?
            .iter()
            .map(|n| n.as_u64().map(|n| n as u32).ok_or("invalid signal"))
            .collect::<std::result::Result<BTreeSet<_>, _>>()?;
        for lane in 0..24 {
            let sum = p.synapses().iter().enumerate().filter(|(_,s)|signals.contains(&s.source()) && matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==family && c.input_lane()==lane)).map(|(i,_)|original[i]).sum::<f32>();
            let rows = p.synapses().iter().enumerate().filter(|(_,s)|s.source()==reference && matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==family && c.input_lane()==lane)).map(|(i,_)|i).collect::<Vec<_>>();
            if rows.len() != 1 || original[rows[0]] != -5. * sum {
                return Err("frozen -5 reference does not match signal coefficients".into());
            }
            if enabled && sum != 0. {
                let i = rows[0];
                w[i] = -sum;
                edits.push(json!({"family":family,"lane":lane,"reference_motor":reference,"row":i,"before":original[i],"after":w[i]}));
            }
        }
    }
    Ok((
        w,
        json!({"schema":"wholecare_reference_calibration_v1","full_differential":enabled,"edits":edits,"ordinary_recurrence_changed":false}),
    ))
}

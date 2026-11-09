//! Fresh authored source revision: match the baseline of three need readouts.
//! No changes to decoder coefficients, controller, soma equations or world rules.
use alife_core::*;
use serde_json::{json, Value};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub const RECIPE: &str = "care-matched-reference-six-rows-v1";
pub fn build(p: &BrainPhenotype, original: &[f32]) -> Result<(Vec<f32>, Value)> {
    if original.len() != p.synapses().len()
        || !p.foundation_abi().uses_bounded_fresh_branches()
        || p.sensor_profile() != SensorProfile::GroundedObjectSlotsV1
        || p.sensor_encoder()
            .assignments()
            .iter()
            .any(|a| [382, 383].contains(&a.target_neuron()))
        || p.synapses()
            .iter()
            .any(|s| s.kind() == CompiledSynapseKind::Recurrent && [382, 383].contains(&s.target()))
    {
        return Err("reference source requires an unsensed, undriven baseline cell".into());
    }
    for (route, kind, cadence) in [
        (2, ProjectionType::Homeostatic, UpdateCadence::Hot10To30Hz),
        (3, ProjectionType::Modulatory, UpdateCadence::Hot10To30Hz),
        (5, ProjectionType::MotorProposal, UpdateCadence::Hot60Hz),
    ] {
        if p.projections()
            .get(route)
            .is_none_or(|r| r.projection_type() != kind || r.update_cadence() != cadence)
        {
            return Err("matched reference route kind or cadence changed".into());
        }
    }
    for layer in [
        &[256, 257, 260, 262, 382, 383][..],
        &[1952, 1953, 1956, 1958, 1966][..],
        &[896, 897, 900, 902, 911][..],
        &[1793, 1796, 1807, 1814, 1823, 1824, 1839][..],
    ] {
        if layer
            .iter()
            .any(|&n| p.neuron_dynamics()[n] != p.neuron_dynamics()[layer[0]])
        {
            return Err("matched reference donor soma dynamics changed".into());
        }
    }
    let edge = |route, source, target| -> Result<usize> {
        let rows = p
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
        if rows.len() != 1 {
            return Err("matched reference donor edge is not unique".into());
        }
        Ok(rows[0])
    };
    if original[edge(2, 383, 1967)?].to_bits() != 8_f32.to_bits() {
        return Err("matched reference shared tonic changed".into());
    }
    for (source, reserve, relay, motor) in [
        (256, 1952, 896, 1824),
        (257, 1953, 897, 1793),
        (260, 1956, 900, 1796),
        (262, 1958, 902, 1814),
    ] {
        for (route, from, to, value) in [
            (2, source, reserve, 2_f32),
            (3, reserve, relay, 2.),
            (3, 1967, relay, -4.),
            (5, relay, motor, 4.),
        ] {
            if original[edge(route, from, to)?].to_bits() != value.to_bits() {
                return Err("matched reference donor path weights changed".into());
            }
        }
    }
    // Source382 has the same native soma and common encoded offset as need
    // cells256/257/260/262 at zero sensed drive. This is a dynamical reference,
    // not a positive literal constant. Match the original need path's2,2,4 and
    // its shared8,-4 tonic path, including the same route cadence.
    let plan = [
        (2, 382, 1966, 2.),
        (3, 1966, 911, 2.),
        (3, 1967, 911, -4.),
        (5, 911, 1807, 4.),
        (5, 911, 1823, 4.),
        (5, 911, 1839, 4.),
    ];
    let mut w = original.to_vec();
    let mut edits = vec![];
    for (route, source, target, value) in plan {
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
            .collect::<Vec<_>>();
        if matches.len() != 1 || original[matches[0].0] != 0. {
            return Err(
                format!("missing or occupied reference edge ({route},{source},{target})").into(),
            );
        }
        let (index, s) = matches[0];
        let map = p.persistent_address_map();
        let address = map
            .synapses()
            .iter()
            .find(|a| a.packed_index() == index as u32)
            .ok_or("reference packed row address missing")?
            .address();
        let neuron_address = |n| {
            map.neurons()
                .iter()
                .find(|a| a.packed_index() == n)
                .map(|a| a.address())
        };
        let projection = map
            .projections()
            .iter()
            .find(|a| a.packed_index() == route)
            .map(|a| a.address());
        if Some(address.source) != neuron_address(source)
            || Some(address.target) != neuron_address(target)
            || Some(address.projection) != projection
            || address.decoder.is_some()
        {
            return Err("reference packed row address disagrees with recurrent tuple".into());
        }
        w[index] = value;
        edits.push(json!({"row":index,"address":address,"route":route,"source":source,"target":target,"before":original[index],"after":value,"cadence":p.projections()[s.route_index() as usize].update_cadence().raw()}));
    }
    Ok((
        w,
        json!({"recipe":RECIPE,"authored_not_learned":true,"added_nonzero_source_rows":6,"source":382,"reserve":1966,"relay":911,"reference_motors":[1807,1823,1839],"edits":edits,"decoder_coefficients_changed":false,"controller_rows_changed":false,"acquired_state_changed":false}),
    ))
}

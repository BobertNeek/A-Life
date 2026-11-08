//! Frozen constructor copy from probe_weight_space_care.rs at 10eecff4.
//! Source-only assay support. No rollout code is included.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_world::*;
use serde_json::json;
use std::error::Error;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn source(
    seed: u64,
    experimental: bool,
) -> Result<(
    CanonicalNewGameConfig,
    BrainPhenotype,
    FoundationWeightAsset,
)> {
    let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    let asset = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile)?;
    let game = create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
    let r = game.world.organism_registry().iter().next().unwrap();
    let capacity = BrainCapacityClass::n2048();
    let genome = r.phenotype().brain_genome.clone();
    let development = foundation_construction_development(
        &genome,
        &capacity,
        &r.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    if experimental {
        let (p, _) = PhenotypeCompiler::compile_n2048_experimental_graph(
            genome.clone(),
            development.clone(),
            config.sensor_profile,
        )?;
        let asset = FoundationWeightAsset::from_trained_weights(
            &p,
            p.synapses().iter().map(|s| s.genetic_weight()).collect(),
            TrainingStageManifest::bootstrap(),
        )?;
        let (p, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            genome,
            development,
            asset.clone(),
        )?;
        return Ok((config, p, asset));
    }
    let p = PhenotypeCompiler::compile(&genome, &capacity, &development, config.sensor_profile)?;
    let asset = FoundationWeightAsset::from_trained_weights(
        &p,
        p.synapses().iter().map(|s| s.genetic_weight()).collect(),
        asset.manifest().training_stage(),
    )?;
    let (p, _) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset.clone())?;
    Ok((config, p, asset))
}
// Whole-weight revision 6. Feedback-informed: explicit signed tonic/need circuits
// and a native reference motor population; never an external action policy.
pub fn whole_care(p: &BrainPhenotype) -> Result<(Vec<f32>, Vec<serde_json::Value>)> {
    whole_care_with_contact(p, -2.)
}
fn whole_care_with_contact(
    p: &BrainPhenotype,
    contact_coefficient: f32,
) -> Result<(Vec<f32>, Vec<serde_json::Value>)> {
    use std::collections::BTreeSet;
    let mut w = vec![0.; p.synapses().len()];
    let mut circuits = Vec::new();
    let mut incoming = vec![0usize; p.neuron_count() as usize];
    let mut outgoing = vec![Vec::new(); p.neuron_count() as usize];
    for (i, s) in p.synapses().iter().enumerate() {
        if s.kind() == CompiledSynapseKind::Recurrent {
            incoming[s.target() as usize] += 1;
            outgoing[s.source() as usize].push(i);
        }
    }
    for (i, s) in p.synapses().iter().enumerate() {
        if s.kind() != CompiledSynapseKind::Recurrent {
            continue;
        }
        let route = &p.projections()[s.route_index() as usize];
        if route.projection_type() == ProjectionType::LateralInhibition {
            w[i] = -0.03;
        } else if [
            LobeKind::WorkingContextExecutive,
            LobeKind::MemoryInterface,
            LobeKind::MultimodalAssociation,
        ]
        .contains(&route.source_lobe())
            || [
                LobeKind::WorkingContextExecutive,
                LobeKind::MemoryInterface,
                LobeKind::MultimodalAssociation,
            ]
            .contains(&route.target_lobe())
        {
            w[i] = 0.12 / incoming[s.target() as usize] as f32;
        }
    }
    let intero = p
        .lobe_layout()
        .region(LobeKind::InteroceptiveMotivational)
        .unwrap();
    let reserve = p.lobe_layout().region(LobeKind::FlexibleReserve).unwrap();
    let tonic_source = (intero.start..intero.start + intero.len)
        .rev()
        .find(|n| {
            !p.sensor_encoder()
                .assignments()
                .iter()
                .any(|a| a.target_neuron() == *n)
        })
        .ok_or("no unsensed tonic neuron")?;
    let tonic_reserve = reserve.start + 15;
    let tonic_first = *outgoing[tonic_source as usize]
        .iter()
        .find(|&&i| p.synapses()[i].route_index() == 2 && p.synapses()[i].target() == tonic_reserve)
        .ok_or("tonic input route absent")?;
    let mut need_axes = std::collections::BTreeMap::<u16, (u32, u32)>::new();
    for (family, lanes) in [
        (CandidateActionFamily::Ingest, vec![0]),
        (CandidateActionFamily::Approach, vec![0]),
        (CandidateActionFamily::Rest, vec![1]),
        (CandidateActionFamily::Avoid, vec![3, 2]),
        (CandidateActionFamily::Inspect, vec![5]),
        (CandidateActionFamily::Contact, vec![5]),
    ] {
        let motors = p
            .synapses()
            .iter()
            .filter_map(|s| match s.kind() {
                CompiledSynapseKind::Decoder(c)
                    if c.head() == DecoderHeadKind::ActionCandidate && c.family() == family =>
                {
                    Some(s.source())
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let reference = *motors.last().unwrap();
        let mut selected = BTreeSet::new();
        for lane in lanes {
            let input = p
                .sensor_encoder()
                .assignments()
                .iter()
                .find(|a| {
                    a.source_group() == SensorEncoderSourceGroup::Homeostasis
                        && a.source_index() == lane
                })
                .ok_or("essential homeostatic port absent")?
                .target_neuron();
            let mut found = None;
            'paths: for &first in &outgoing[input as usize] {
                let first_edge = &p.synapses()[first];
                if first_edge.route_index() != 2
                    || first_edge.target() == tonic_reserve
                    || need_axes
                        .iter()
                        .any(|(other, (r, _))| *other != lane && *r == first_edge.target())
                    || need_axes
                        .get(&lane)
                        .is_some_and(|(r, _)| *r != first_edge.target())
                {
                    continue;
                }
                for &second in &outgoing[first_edge.target() as usize] {
                    let second_edge = &p.synapses()[second];
                    if second_edge.route_index() != 3
                        || need_axes
                            .iter()
                            .any(|(other, (_, t))| *other != lane && *t == second_edge.target())
                        || need_axes
                            .get(&lane)
                            .is_some_and(|(_, t)| *t != second_edge.target())
                    {
                        continue;
                    }
                    let Some(&tonic_second) = outgoing[tonic_reserve as usize].iter().find(|&&i| {
                        p.synapses()[i].route_index() == 3
                            && p.synapses()[i].target() == second_edge.target()
                    }) else {
                        continue;
                    };
                    for &third in &outgoing[second_edge.target() as usize] {
                        let target = p.synapses()[third].target();
                        if p.synapses()[third].route_index() == 5
                            && motors.contains(&target)
                            && target != reference
                        {
                            found = Some((first, second, third, tonic_second, target));
                            break 'paths;
                        }
                    }
                }
            }
            let (first, second, third, tonic_second, motor) =
                found.ok_or("compatible tonic and need circuit absent")?;
            need_axes.insert(
                lane,
                (p.synapses()[first].target(), p.synapses()[second].target()),
            );
            w[first] = 2.;
            w[second] = 2.;
            w[third] = 4.;
            w[tonic_first] = 8.;
            w[tonic_second] = -4.;
            selected.insert(motor);
            circuits.push(json!({"role":"need","family":family,"homeostatic_lane":lane,"source_neuron":input,"motor_neuron":motor,"bridge_synapses":[first,second],"output_synapse":third}));
            circuits.push(json!({"role":"tonic","family":family,"source_neuron":tonic_source,"motor_neuron":motor,"bridge_synapses":[tonic_first,tonic_second],"output_synapse":third}));
        }
        for (i, synapse) in p.synapses().iter().enumerate() {
            let CompiledSynapseKind::Decoder(c) = synapse.kind() else {
                continue;
            };
            if c.head() != DecoderHeadKind::ActionCandidate || c.family() != family {
                continue;
            }
            let coefficient = match (family, c.input_lane()) {
                (CandidateActionFamily::Ingest, 15) => 12.,
                (CandidateActionFamily::Ingest, 2) => -6.,
                (CandidateActionFamily::Approach, 15) => 4.,
                (CandidateActionFamily::Approach, 2) => -2.,
                (CandidateActionFamily::Approach, 18) => contact_coefficient,
                (CandidateActionFamily::Rest, 23) => 2.,
                (CandidateActionFamily::Avoid, 15) => -10.,
                (CandidateActionFamily::Inspect, 6..=8) => 0.5,
                (CandidateActionFamily::Contact, 15) => 2.,
                _ => 0.,
            };
            if selected.contains(&synapse.source()) {
                w[i] = coefficient;
            }
            if synapse.source() == reference {
                let reference_gain = match family {
                    CandidateActionFamily::Approach | CandidateActionFamily::Inspect => 5.,
                    CandidateActionFamily::Rest => -1.,
                    _ => -5.,
                };
                if coefficient != 0. {
                    w[i] = reference_gain * selected.len() as f32 * coefficient;
                }
            }
        }
        circuits.push(json!({"role":"reference","family":family,"reference_motor":reference,"signal_motors":selected}));
    }
    Ok((w, circuits))
}

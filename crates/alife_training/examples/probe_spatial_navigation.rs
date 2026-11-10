//! Disposable exploratory direct-weight probe. CPU only; never reads a life.
#[allow(dead_code)]
#[path = "support/bold_base.rs"]
mod bold;
#[allow(dead_code)]
#[path = "support/dedicated_fatigue.rs"]
mod dedicated;
#[allow(dead_code)]
#[path = "support/motor_operating_point.rs"]
mod readout;
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask, TrainingSequence};
use alife_world::*;
use serde_json::json;
use std::error::Error;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(crate) fn source(
    seed: u64,
    experimental: bool,
) -> Result<(
    CanonicalNewGameConfig,
    BrainPhenotype,
    FoundationWeightAsset,
)> {
    let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    let navigation = std::env::var_os("ALIFE_RESEARCH_NAV_TERRAIN").is_some();
    if navigation {
        config.founder_seed_base = 2026111560;
    }
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
    if experimental && navigation {
        // Explicitly recompile an experimental terrain-vision graph and transfer
        // the chosen V2 genetic weights only after structural coordinate checks.
        // Sensor meanings change, so this is a fresh research asset, not a save migration.
        let (reference, _) = PhenotypeCompiler::compile_n2048_anchored_v2_graph(
            genome.clone(),
            development.clone(),
            SensorProfile::GroundedObjectSlotsV1,
        )?;
        let chosen = FoundationWeightAsset::decode_canonical(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/foundations/experimental-anchored-v2-research.alife-foundation"
        )))?;
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            genome.clone(),
            development.clone(),
            chosen.clone(),
        )?;
        let (terrain, _) = PhenotypeCompiler::compile_n2048_experimental_graph(
            genome.clone(),
            development.clone(),
            SensorProfile::GroundedTerrainVisionV1,
        )?;
        if reference.synapses().len() != terrain.synapses().len()
            || !reference
                .synapses()
                .iter()
                .zip(terrain.synapses())
                .all(|(a, b)| {
                    a.source() == b.source()
                        && a.target() == b.target()
                        && a.route_index() == b.route_index()
                        && a.kind() == b.kind()
                })
        {
            return Err("terrain graph coordinate mismatch; refusing weight transfer".into());
        }
        let asset = FoundationWeightAsset::from_trained_weights(
            &terrain,
            chosen.weights().to_vec(),
            chosen.manifest().training_stage(),
        )?;
        let (bound, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            genome,
            development,
            asset.clone(),
        )?;
        config.sensor_profile = SensorProfile::GroundedTerrainVisionV1;
        return Ok((config, bound, asset));
    }
    if experimental {
        let (p, _) = if std::env::var_os("ALIFE_RESEARCH_ANCHORED_V2").is_some() {
            PhenotypeCompiler::compile_n2048_anchored_v2_graph(
                genome.clone(),
                development.clone(),
                config.sensor_profile,
            )?
        } else {
            PhenotypeCompiler::compile_n2048_experimental_graph(
                genome.clone(),
                development.clone(),
                config.sensor_profile,
            )?
        };
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
pub(crate) fn whole_care(p: &BrainPhenotype) -> Result<(Vec<f32>, Vec<serde_json::Value>)> {
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
                LobeKind::TemporalPredictive,
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
/// Exploratory native circuit: feed the lateral sight rays through existing
/// recurrent context paths into opposing turn preferences, with a modest
/// context recurrence boost. This is NOT a claimed spatial map or solver.
fn wire_spatial_trace(p: &BrainPhenotype, weights: &mut [f32]) -> Result<Vec<serde_json::Value>> {
    use std::collections::{BTreeSet, VecDeque};
    let context_regions = [LobeKind::WorkingContextExecutive, LobeKind::MemoryInterface]
        .into_iter()
        .filter_map(|k| p.lobe_layout().region(k))
        .collect::<Vec<_>>();
    let is_context = |n: u32| {
        context_regions
            .iter()
            .any(|r| n >= r.start && n < r.start + r.len)
    };
    let mut outgoing = vec![Vec::new(); p.neuron_count() as usize];
    for (i, edge) in p.synapses().iter().enumerate() {
        if edge.kind() == CompiledSynapseKind::Recurrent
            && p.projections()[edge.route_index() as usize].projection_type()
                != ProjectionType::LateralInhibition
        {
            outgoing[edge.source() as usize].push(i);
        }
    }
    let motors = p
        .synapses()
        .iter()
        .filter_map(|edge| match edge.kind() {
            CompiledSynapseKind::Decoder(c)
                if c.head() == DecoderHeadKind::ActionCandidate
                    && c.family() == CandidateActionFamily::Inspect
                    && c.input_lane() == 0 =>
            {
                Some(edge.source())
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut used_nodes = BTreeSet::new();
    let mut used_motors = BTreeSet::new();
    let mut edits = Vec::new();
    for (ray, direction) in [(0u16, -1.0f32), (7u16, 1.0f32)] {
        let sources = p
            .sensor_encoder()
            .assignments()
            .iter()
            .filter(|a| {
                a.source_group() == SensorEncoderSourceGroup::SensoryChannel
                    && a.source_index() == ray
            })
            .map(|a| a.target_neuron())
            .collect::<Vec<_>>();
        let mut selected = None;
        for source in sources {
            if used_nodes.contains(&source) {
                continue;
            }
            let mut queue = VecDeque::from([(source, is_context(source), Vec::<usize>::new())]);
            let mut visited = BTreeSet::from([(source, is_context(source))]);
            while let Some((node, seen_context, path)) = queue.pop_front() {
                if !path.is_empty()
                    && seen_context
                    && motors.contains(&node)
                    && !used_motors.contains(&node)
                {
                    selected = Some((source, node, path));
                    break;
                }
                if path.len() >= 6 {
                    continue;
                }
                for &i in &outgoing[node as usize] {
                    let next = p.synapses()[i].target();
                    if used_nodes.contains(&next) {
                        continue;
                    }
                    let next_context = seen_context || is_context(next);
                    if visited.insert((next, next_context)) {
                        let mut more = path.clone();
                        more.push(i);
                        queue.push_back((next, next_context, more));
                    }
                }
            }
            if selected.is_some() {
                break;
            }
        }
        let (source, motor, path) = selected.ok_or("no disjoint visual-to-context-to-turn path")?;
        used_nodes.insert(source);
        used_motors.insert(motor);
        for &i in &path {
            let edge = &p.synapses()[i];
            used_nodes.insert(edge.target());
            let before = weights[i];
            weights[i] = before.max(2.0);
            edits.push(json!({"role":"spatial_visual_context_relay","ray":ray,"index":i,"source":edge.source(),"target":edge.target(),"before":before,"after":weights[i]}));
        }
        let mut decoder_count = 0;
        for (i, edge) in p.synapses().iter().enumerate() {
            if edge.source() == motor
                && matches!(edge.kind(),CompiledSynapseKind::Decoder(c) if
                c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Inspect && c.input_lane()==0)
            {
                let before = weights[i];
                weights[i] = before + direction * 6.0;
                decoder_count += 1;
                edits.push(json!({"role":"spatial_turn_bias","ray":ray,"index":i,"source":motor,"before":before,"after":weights[i]}));
            }
        }
        if decoder_count == 0 {
            return Err("turn decoder missing".into());
        }
    }
    // Strengthen both legs of existing predictive-context feedback loops.
    // This preserves temporal structure but is not a claim of a learned map.
    let predictive = p
        .lobe_layout()
        .region(LobeKind::TemporalPredictive)
        .ok_or("predictive region missing")?;
    let in_predictive = |n: u32| n >= predictive.start && n < predictive.start + predictive.len;
    let mut inbound = 0usize;
    let mut outbound = 0usize;
    for (i, edge) in p.synapses().iter().enumerate() {
        if edge.kind() != CompiledSynapseKind::Recurrent
            || p.projections()[edge.route_index() as usize].projection_type()
                == ProjectionType::LateralInhibition
        {
            continue;
        }
        let entering = in_predictive(edge.source()) && is_context(edge.target());
        let leaving = is_context(edge.source()) && in_predictive(edge.target());
        if (entering && inbound < 32) || (leaving && outbound < 32) {
            let before = weights[i];
            weights[i] = before + 0.05;
            if entering {
                inbound += 1;
            } else {
                outbound += 1;
            }
            edits.push(json!({"role":"spatial_context_feedback","index":i,"before":before,"after":weights[i]}));
        }
    }
    if inbound == 0 || outbound == 0 {
        return Err("incomplete predictive-context loop".into());
    }
    Ok(edits)
}
/// Lay out a local 4x4 recurrent lattice across an existing reciprocal
/// predictive/context connection block. This is an experimental wave substrate,
/// not yet an occupancy map, path extractor, or guaranteed stable attractor.
fn wire_local_spatial_lattice(
    p: &BrainPhenotype,
    weights: &mut [f32],
    persistent: bool,
) -> Result<Vec<serde_json::Value>> {
    let mut counts = [0usize; 2];
    let mut edits = Vec::new();
    for (i, edge) in p.synapses().iter().enumerate() {
        if edge.kind() != CompiledSynapseKind::Recurrent {
            continue;
        }
        let pair = match (edge.route_index(), edge.source(), edge.target()) {
            (7, source @ 896..=911, target @ 1600..=1615) => Some((0, source - 896, target - 1600)),
            (8, source @ 1600..=1615, target @ 896..=911) => Some((1, source - 1600, target - 896)),
            _ => None,
        };
        if let Some((direction, a, b)) = pair {
            counts[direction] += 1;
            let ar = a / 4;
            let ac = a % 4;
            let br = b / 4;
            let bc = b % 4;
            let adjacent = ar.abs_diff(br) + ac.abs_diff(bc) == 1;
            let after = if a == b {
                if persistent {
                    0.85
                } else {
                    0.4
                }
            } else if adjacent {
                if persistent {
                    0.06
                } else {
                    0.1
                }
            } else {
                0.0
            };
            let before = weights[i];
            weights[i] = after;
            if before.to_bits() != after.to_bits() {
                edits.push(json!({"role":"spatial_lattice","index":i,"source":edge.source(),"target":edge.target(),"before":before,"after":after}));
            }
        }
    }
    if counts != [256, 256] {
        return Err(format!("unexpected lattice block geometry {counts:?}").into());
    }
    Ok(edits)
}
/// Map the eight real lateral range rays into four adjacent input cells.
/// This is an egocentric sensory trace, not a privileged world map.
fn wire_visual_lattice_inputs(
    p: &BrainPhenotype,
    weights: &mut [f32],
) -> Result<Vec<serde_json::Value>> {
    use std::collections::BTreeSet;
    let mut used_sources = BTreeSet::new();
    let mut edits = Vec::new();
    for ray in 0..8u16 {
        let target = 896 + u32::from(ray / 2);
        let edge = p
            .sensor_encoder()
            .assignments()
            .iter()
            .filter(|a| {
                a.source_group() == SensorEncoderSourceGroup::SensoryChannel
                    && a.source_index() == ray
            })
            .filter(|a| !used_sources.contains(&a.target_neuron()))
            // A shared encoder neuron is not a clean ray channel.
            .filter(|a| {
                p.sensor_encoder()
                    .assignments()
                    .iter()
                    .filter(|b| b.target_neuron() == a.target_neuron())
                    .count()
                    == 1
            })
            .find_map(|a| {
                p.synapses()
                    .iter()
                    .enumerate()
                    .find(|(_, e)| {
                        e.kind() == CompiledSynapseKind::Recurrent
                            && e.route_index() == 0
                            && e.source() == a.target_neuron()
                            && e.target() == target
                    })
                    .map(|(i, _)| (i, a.target_neuron()))
            })
            .ok_or_else(|| format!("no disjoint ray {ray} input to grid cell {target}"))?;
        used_sources.insert(edge.1);
        let before = weights[edge.0];
        weights[edge.0] = before + 2.0;
        edits.push(json!({"role":"egocentric_visual_grid_input","ray":ray,"index":edge.0,"source":edge.1,"target":target,"before":before,"after":weights[edge.0]}));
    }
    Ok(edits)
}
struct TrialSpec<'a> {
    seed: u64,
    arm: &'a str,
    gain: f32,
    scene: &'a str,
    ticks: usize,
    receipt_root: &'a str,
    write_asset: bool,
    initial_energy: f32,
    preparation_ticks: usize,
    target_hazard_pain: f32,
}
fn trial(spec: TrialSpec<'_>) -> Result<serde_json::Value> {
    let TrialSpec {
        seed,
        arm,
        gain,
        scene,
        ticks,
        receipt_root,
        write_asset,
        initial_energy,
        preparation_ticks,
        target_hazard_pain,
    } = spec;
    let experimental =
        arm == "zero-excitatory" || arm.starts_with("whole-care") || arm.starts_with("nav-");
    let (config, p, source) = source(seed, experimental)?;
    let mut weights = p
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    let mut edits = Vec::new();
    if arm == "zero-excitatory" {
        weights.fill(0.);
        for (i, s) in p.synapses().iter().enumerate() {
            if s.kind() == CompiledSynapseKind::Recurrent
                && p.projections()[s.route_index() as usize].projection_type()
                    == ProjectionType::LateralInhibition
            {
                weights[i] = -0.03;
            }
        }
    } else if arm.starts_with("whole-care") {
        let (whole, circuits) = if arm == "whole-care-v5" {
            whole_care_with_contact(&p, -5.)?
        } else {
            whole_care(&p)?
        };
        weights = whole;
        if matches!(
            arm,
            "whole-care-balanced"
                | "whole-care-narrow"
                | "whole-care-ingest-only"
                | "whole-care-ingest-boost"
        ) {
            let (w, _) = readout::counterbalance(&p, &weights, &circuits, 0., true)?;
            let (w, _) = dedicated::full_reference_calibration(&p, &w, &circuits, true)?;
            weights = w;
            if arm == "whole-care-ingest-boost" {
                let (baseline, _) = whole_care(&p)?;
                for (i, synapse) in p.synapses().iter().enumerate() {
                    let is_ingest = matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate && c.family() == CandidateActionFamily::Ingest);
                    weights[i] = if is_ingest && weights[i].to_bits() != baseline[i].to_bits() {
                        baseline[i] * 1.25
                    } else {
                        baseline[i]
                    };
                }
            }
            if arm == "whole-care-ingest-only" {
                let (baseline, _) = whole_care(&p)?;
                for (i, synapse) in p.synapses().iter().enumerate() {
                    if !matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate && c.family() == CandidateActionFamily::Ingest)
                    {
                        weights[i] = baseline[i];
                    }
                }
            }
            if arm == "whole-care-narrow" {
                let (baseline, _) = whole_care(&p)?;
                let reference = circuits
                    .iter()
                    .find(|c| {
                        c["role"] == "reference"
                            && c["family"]
                                == serde_json::to_value(CandidateActionFamily::Approach).unwrap()
                    })
                    .and_then(|c| c["reference_motor"].as_u64())
                    .ok_or("missing Approach reference")? as u32;
                for (i, synapse) in p.synapses().iter().enumerate() {
                    if synapse.source() == reference
                        && matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate && c.family() == CandidateActionFamily::Approach)
                    {
                        weights[i] = baseline[i];
                    }
                }
            }
        }
        edits = circuits;
        if arm == "whole-care-bold" {
            let (candidate, authored) = bold::build(&p, &weights)?;
            weights = candidate;
            edits.extend(authored);
        }
        if arm == "whole-care-approach70" {
            let mut count = 0usize;
            for (i, synapse) in p.synapses().iter().enumerate() {
                if matches!(synapse.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Approach && c.input_lane()==15)
                    && weights[i] != 0.
                {
                    let before = weights[i];
                    weights[i] *= 0.70;
                    count += 1;
                    edits.push(json!({"role":"approach_taste_weight_edit","index":i,"before":before,"after":weights[i]}));
                }
            }
            if count != 2 {
                return Err("unexpected Approach taste weight count".into());
            }
        }
        if matches!(
            arm,
            "whole-care-balanced"
                | "whole-care-narrow"
                | "whole-care-ingest-only"
                | "whole-care-ingest-boost"
        ) {
            let (baseline, _) = whole_care(&p)?;
            for (i, (before, after)) in baseline.iter().zip(&weights).enumerate() {
                if before.to_bits() != after.to_bits() {
                    if !matches!(p.synapses()[i].kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate)
                    {
                        return Err(
                            "reference calibration changed a non-action decoder weight".into()
                        );
                    }
                    edits.push(json!({"role":"reference_weight_edit","index":i,"before":before,"after":after}));
                }
            }
            if !edits.iter().any(|e| e["role"] == "reference_weight_edit") {
                return Err("reference calibration made no changes".into());
            }
        }
        if arm == "whole-care-no-need" || arm == "whole-care-no-tonic" {
            let role = if arm.ends_with("no-need") {
                "need"
            } else {
                "tonic"
            };
            for circuit in &edits {
                if circuit["role"] == role {
                    for i in circuit["bridge_synapses"].as_array().unwrap() {
                        weights[i.as_u64().unwrap() as usize] = 0.;
                    }
                }
            }
        }
        if arm == "whole-care-no-reference" {
            for circuit in &edits {
                if circuit["role"] == "reference" {
                    let reference = circuit["reference_motor"].as_u64().unwrap() as u32;
                    for (i, s) in p.synapses().iter().enumerate() {
                        if matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate)
                            && s.source() == reference
                        {
                            weights[i] = 0.;
                        }
                    }
                }
            }
        }
    } else if gain != 0. {
        for (i, s) in p.synapses().iter().enumerate() {
            if matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Ingest && c.input_lane()==15)
            {
                weights[i] += gain;
                edits.push(json!({"index":i,"source":s.source(),"before":s.genetic_weight(),"after":weights[i]}));
            }
        }
    }
    if arm == "nav-edit"
        || arm == "nav-combined"
        || arm == "nav-lattice"
        || arm == "nav-strong"
        || arm == "nav-care"
        || arm == "nav-care2"
        || arm == "nav-map"
    {
        if config.sensor_profile != SensorProfile::GroundedTerrainVisionV1 {
            return Err("navigation circuit requires terrain vision".into());
        }
        edits.extend(wire_spatial_trace(&p, &mut weights)?);
        if arm == "nav-lattice"
            || arm == "nav-strong"
            || arm == "nav-care"
            || arm == "nav-care2"
            || arm == "nav-map"
        {
            edits.extend(wire_local_spatial_lattice(
                &p,
                &mut weights,
                arm == "nav-map",
            )?);
            if arm == "nav-map" {
                edits.extend(wire_visual_lattice_inputs(&p, &mut weights)?);
            }
        }
        if arm == "nav-combined"
            || arm == "nav-lattice"
            || arm == "nav-strong"
            || arm == "nav-care"
            || arm == "nav-care2"
            || arm == "nav-map"
        {
            // Grounded target distance is available only for a seen object.
            // Bias exploratory approach without inventing a food label or taste.
            let reference = p
                .synapses()
                .iter()
                .filter_map(|edge| match edge.kind() {
                    CompiledSynapseKind::Decoder(c)
                        if c.head() == DecoderHeadKind::ActionCandidate
                            && c.family() == CandidateActionFamily::Approach =>
                    {
                        Some(edge.source())
                    }
                    _ => None,
                })
                .max()
                .ok_or("no approach reference")?;
            let mut count = 0;
            for (i, edge) in p.synapses().iter().enumerate() {
                if edge.source() == reference
                    && matches!(edge.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Approach && c.input_lane()==2)
                {
                    let before = weights[i];
                    weights[i] = before
                        + if arm == "nav-strong"
                            || arm == "nav-care"
                            || arm == "nav-care2"
                            || arm == "nav-map"
                        {
                            80.0
                        } else {
                            24.0
                        };
                    count += 1;
                    edits.push(json!({"role":"seen_object_exploration","index":i,"before":before,"after":weights[i]}));
                }
            }
            if count == 0 {
                return Err("no approach distance decoder for reference motor".into());
            }
        }
    }
    if arm == "nav-care" || arm == "nav-care2" || arm == "nav-map" {
        // Curiosity about an actually contacted object, not a food or safety oracle.
        // Strong bitter/pain avoidance remains a separate signal and is tested.
        let reference = p
            .synapses()
            .iter()
            .filter_map(|edge| match edge.kind() {
                CompiledSynapseKind::Decoder(c)
                    if c.head() == DecoderHeadKind::ActionCandidate
                        && c.family() == CandidateActionFamily::Ingest =>
                {
                    Some(edge.source())
                }
                _ => None,
            })
            .max()
            .ok_or("no ingest reference")?;
        let mut count = 0;
        for (i, edge) in p.synapses().iter().enumerate() {
            if edge.source() == reference
                && matches!(edge.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Ingest && c.input_lane()==18)
            {
                let before = weights[i];
                weights[i] = before
                    + if arm == "nav-care2" || arm == "nav-map" {
                        -20.0
                    } else {
                        4.0
                    };
                count += 1;
                edits.push(json!({"role":"contacted_object_sampling","index":i,"before":before,"after":weights[i]}));
            }
        }
        if count == 0 {
            return Err("no ingest contact decoder for reference motor".into());
        }
    }
    let asset = if arm == "untouched" {
        source.clone()
    } else {
        FoundationWeightAsset::from_trained_weights(
            &p,
            weights,
            source.manifest().training_stage(),
        )?
    };
    if write_asset {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(format!("{receipt_root}/{seed}.{arm}.alife-foundation"))?;
        file.write_all(&asset.encode_canonical()?)?;
        file.sync_all()?;
    }
    let candidate_digest = asset.digest();
    let changed_weights = p
        .synapses()
        .iter()
        .zip(asset.weights())
        .filter(|(s, w)| s.genetic_weight().to_bits() != w.to_bits())
        .count();
    let mut game = if experimental {
        create_canonical_new_game_with_n2048_experimental_candidate(&config, &asset)?
    } else {
        create_canonical_new_game_with_n2048_candidate(&config, &asset)?
    };
    let old = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let cap = BrainCapacityClass::n2048();
    let development = foundation_construction_development(
        &old.phenotype().brain_genome,
        &cap,
        &old.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    let (p, _) = if experimental {
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            old.phenotype().brain_genome.clone(),
            development,
            asset.clone(),
        )?
    } else {
        PhenotypeCompiler::compile_n2048_foundation_candidate(
            old.phenotype().brain_genome.clone(),
            development,
            asset.clone(),
        )?
    };
    let mask = StageTrainableMask::recurrent_only(&p)?;
    let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default())?;
    let before = serde_json::to_vec(&trainer.checkpoint())?;
    let own = old.organism_id();
    let distance = if scene.starts_with("far-")
        || scene == "obstacle-food"
        || (scene.starts_with("maze-")
            && !["maze-near", "maze-neutral", "maze-bitter"].contains(&scene))
    {
        2.5
    } else {
        0.25
    };
    let target_position = if scene.starts_with("maze-") {
        Vec3f::new(distance, 0., 0.)
    } else if scene.ends_with("-left") {
        Vec3f::new(0., 0., distance)
    } else if scene.ends_with("-back") {
        Vec3f::new(-distance, 0., 0.)
    } else {
        Vec3f::new(distance, 0., 0.)
    };
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", own, Vec3f::ZERO)
        .build()?;
    if scene.starts_with("maze-") {
        let wall = match scene {
            "maze-left" => vec![[1.0, -4.0, 1.3, 0.8, 0.0, 2.0]],
            "maze-right" => vec![[1.0, -0.8, 1.3, 4.0, 0.0, 2.0]],
            "maze-clear" | "maze-near" | "maze-neutral" | "maze-bitter" => vec![],
            _ => return Err("unknown maze scene".into()),
        };
        let terrain = WorldTerrain::new(
            TerrainData {
                width: 17,
                depth: 17,
                origin_x: -4.0,
                origin_z: -4.0,
                spacing: 0.5,
                heights: vec![0.0; 17 * 17],
                obstacles: wall,
                water_level: None,
            },
            LocomotionLimits::default(),
        )?;
        world.enable_terrain_for_new_game(terrain, Vec3f::ZERO)?;
    }
    if scene == "obstacle-food" {
        world.editor_spawn_object(WorldEditorSpawnSpec {
            label: "obstacle".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(1.25, 0., 0.),
            nutrition: 0.,
            hazard_pain: 0.,
            radius: 0.35,
            token_id: None,
        })?;
    }
    let scene_hazard = if scene == "maze-bitter" {
        0.3
    } else {
        target_hazard_pain
    };
    let object = if scene == "toy" {
        world.spawn_toy("target", target_position, true)?
    } else {
        world.editor_spawn_object(WorldEditorSpawnSpec {
            label: "target".into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: target_position,
            nutrition: 0.6,
            hazard_pain: scene_hazard,
            radius: 0.5,
            token_id: None,
        })?
    };
    let mut physical = world.entity(object).unwrap().grounded_physical;
    physical.chemical[0] = if scene.starts_with("bitter-food") || scene == "maze-bitter" {
        -0.85
    } else if scene == "neutral-food" || scene == "maze-neutral" {
        0.
    } else if scene == "toy" {
        0.
    } else {
        0.85
    };
    world.set_grounded_physical_properties(object, physical)?;
    let mut chemistry = *old.biochemistry();
    chemistry.body.set_energy(initial_energy)?;
    world.register_organism_record(WorldOrganismRecord::new(
        own,
        world.entity_id("learner").unwrap(),
        old.genome().clone(),
        old.phenotype().clone(),
        chemistry,
        old.birth_tick(),
    )?)?;
    game.world = world;
    let sampling_seed = (seed ^ (seed >> 32)) as u32;
    let mut host = if preparation_ticks == 0 {
        CpuCanonicalHost::new_cohort(game, &trainer, sampling_seed)?
    } else {
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, sampling_seed, |world| {
            for _ in 0..preparation_ticks {
                world.try_advance_tick()?;
            }
            Ok::<_, ScaffoldContractError>(())
        })?
        .0
    };
    let initial_biology = *host
        .world()
        .organism_registry()
        .get(own)
        .unwrap()
        .biochemistry();
    let initial_checkpoint = host.checkpoint(&trainer)?;
    let initial_neural = initial_checkpoint
        .residents
        .iter()
        .find(|r| r.neural.organism_id == own)
        .ok_or("missing initial neural owner")?
        .neural
        .clone();
    let mut rows = Vec::new();
    for tick in 0..ticks {
        let step = host.step(&trainer, 1.)?;
        for row in step.decisions {
            let eval = trainer.evaluate_replay(&TrainingSequence {
                phenotype_hash: trainer.phenotype().phenotype_hash(),
                initial: row.initial.clone(),
                ticks: vec![row.replay_tick.clone()],
                burn_in_ticks: 0,
                memory_candidate_gain: trainer
                    .phenotype()
                    .candidate_decoder()
                    .memory_channel()
                    .map_or(0., |m| m.max_candidate_gain()),
            })?;
            let frame = row.patch.pre_action().perception();
            if tick == 0 && (scene == "maze-left" || scene == "maze-right") {
                assert!(
                    frame
                        .candidates()
                        .iter()
                        .all(|c| c.target.entity != Some(object)),
                    "hidden food leaked into first-step candidates"
                );
            }
            if tick == 0 && scene != "toy" && !scene.starts_with("maze-") {
                let expected = if scene.starts_with("bitter-food") {
                    -0.85
                } else if scene == "neutral-food" {
                    0.
                } else {
                    0.85
                };
                assert!(frame
                    .candidates()
                    .iter()
                    .filter(|c| c.target.entity == Some(object))
                    .all(|c| (c.features.0[15] - expected).abs() < 1e-5));
            }
            let motors=trainer.phenotype().synapses().iter().filter_map(|s|match s.kind() {CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Ingest && c.input_lane()==15=>Some(json!({"source":s.source(),"activation":eval.final_activations[0][s.source() as usize],"weight":s.genetic_weight()})),_=>None}).collect::<Vec<_>>();
            // Detached neural-input sensitivity, not a world action or a learning update.
            // Innate biases stay fixed so this isolates the encoded recurrent path.
            let hunger_probe = if tick == 0 {
                let mut changed = row.replay_tick.clone();
                for a in trainer
                    .phenotype()
                    .sensor_encoder()
                    .assignments()
                    .iter()
                    .filter(|a| {
                        a.source_group() == SensorEncoderSourceGroup::Homeostasis
                            && a.source_index() == 0
                    })
                {
                    changed.encoded_inputs[a.target_neuron() as usize] += 0.1;
                }
                let response = trainer.evaluate_replay(&TrainingSequence {
                    phenotype_hash: trainer.phenotype().phenotype_hash(),
                    initial: row.initial.clone(),
                    ticks: vec![changed],
                    burn_in_ticks: 0,
                    memory_candidate_gain: trainer
                        .phenotype()
                        .candidate_decoder()
                        .memory_channel()
                        .map_or(0., |m| m.max_candidate_gain()),
                })?;
                Some(
                    frame
                        .candidates()
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c.family == CandidateActionFamily::Ingest)
                        .map(|(i, _)| response.candidate_logits[0][i] - eval.candidate_logits[0][i])
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };
            // Detached impulse/ablation experiment: inputs are edited only in a
            // copied replay, never in the live world or policy state.
            let lattice_pulse = if tick == 0
                && (arm == "nav-lattice" || arm == "nav-map")
                && std::env::var_os("ALIFE_RESEARCH_GRID_PULSE").is_some()
            {
                let grid_edges = trainer
                    .phenotype()
                    .synapses()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, e)| {
                        let inside = matches!(
                            (e.route_index(), e.source(), e.target()),
                            (7, 896..=911, 1600..=1615) | (8, 1600..=1615, 896..=911)
                        );
                        inside.then_some(i)
                    })
                    .collect::<Vec<_>>();
                let run = |steps: usize, pulse: bool, ablate: bool| -> Result<Vec<f32>> {
                    let mut ticks = vec![row.replay_tick.clone(); steps];
                    if pulse {
                        ticks[0].encoded_inputs[896] += 0.25;
                    }
                    if ablate {
                        for tick in &mut ticks {
                            for &i in &grid_edges {
                                tick.effective_weight_offsets[i] =
                                    -trainer.phenotype().synapses()[i].genetic_weight();
                            }
                        }
                    }
                    let out = trainer.evaluate_replay(&TrainingSequence {
                        phenotype_hash: trainer.phenotype().phenotype_hash(),
                        initial: row.initial.clone(),
                        ticks,
                        burn_in_ticks: 0,
                        memory_candidate_gain: trainer
                            .phenotype()
                            .candidate_decoder()
                            .memory_channel()
                            .map_or(0., |m| m.max_candidate_gain()),
                    })?;
                    Ok(out
                        .final_activations
                        .last()
                        .ok_or("empty detached replay")?
                        .clone())
                };
                let mut samples = Vec::new();
                for steps in 1..=4 {
                    let normal = run(steps, false, false)?;
                    let pulsed = run(steps, true, false)?;
                    let off = run(steps, false, true)?;
                    let off_pulsed = run(steps, true, true)?;
                    let deltas = |a: &[f32], b: &[f32], start: usize| {
                        (start..start + 16).map(|i| a[i] - b[i]).collect::<Vec<_>>()
                    };
                    samples.push(json!({"steps":steps,"predictive_delta":deltas(&pulsed,&normal,896),"context_delta":deltas(&pulsed,&normal,1600),"ablated_predictive_delta":deltas(&off_pulsed,&off,896),"ablated_context_delta":deltas(&off_pulsed,&off,1600)}));
                }
                Some(samples)
            } else {
                None
            };
            let mut debug_neurons = std::collections::BTreeSet::new();
            let mut bridge_ids = std::collections::BTreeSet::new();
            for circuit in &edits {
                for key in ["source_neuron", "motor_neuron", "reference_motor"] {
                    if let Some(n) = circuit[key].as_u64() {
                        debug_neurons.insert(n as u32);
                    }
                }
                if let Some(indices) = circuit["bridge_synapses"].as_array() {
                    for i in indices {
                        let i = i.as_u64().unwrap() as usize;
                        bridge_ids.insert(i);
                        debug_neurons.insert(trainer.phenotype().synapses()[i].target());
                    }
                }
            }
            let debug_activations = debug_neurons
                .into_iter()
                .map(|n| (n, eval.final_activations[0][n as usize]))
                .collect::<std::collections::BTreeMap<_, _>>();
            let effective_bridges=bridge_ids.into_iter().map(|i|json!({"index":i,"genetic":trainer.phenotype().synapses()[i].genetic_weight(),"offset":row.replay_tick.effective_weight_offsets[i],"effective":trainer.phenotype().synapses()[i].genetic_weight()+row.replay_tick.effective_weight_offsets[i]})).collect::<Vec<_>>();
            let hunger_three_frames = if tick == 0 {
                let frozen = vec![row.replay_tick.clone(); 3];
                let mut changed = frozen.clone();
                for t in &mut changed {
                    for a in trainer
                        .phenotype()
                        .sensor_encoder()
                        .assignments()
                        .iter()
                        .filter(|a| {
                            a.source_group() == SensorEncoderSourceGroup::Homeostasis
                                && a.source_index() == 0
                        })
                    {
                        t.encoded_inputs[a.target_neuron() as usize] += 0.1;
                    }
                }
                let sequence = |ticks| TrainingSequence {
                    phenotype_hash: trainer.phenotype().phenotype_hash(),
                    initial: row.initial.clone(),
                    ticks,
                    burn_in_ticks: 0,
                    memory_candidate_gain: trainer
                        .phenotype()
                        .candidate_decoder()
                        .memory_channel()
                        .map_or(0., |m| m.max_candidate_gain()),
                };
                let baseline = trainer.evaluate_replay(&sequence(frozen))?;
                let response = trainer.evaluate_replay(&sequence(changed))?;
                Some(
                    frame
                        .candidates()
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c.family == CandidateActionFamily::Ingest)
                        .map(|(i, _)| {
                            response.candidate_logits[2][i] - baseline.candidate_logits[2][i]
                        })
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };
            let outcome = row.patch.outcome();
            // Observer-only evidence: never supplied to the policy.
            let observed_position = host
                .world()
                .entity(host.world().entity_id("learner").unwrap())
                .unwrap()
                .position;
            let visible_goal = frame
                .candidates()
                .iter()
                .any(|c| c.target.entity == Some(object));
            let latent_context = [LobeKind::WorkingContextExecutive, LobeKind::MemoryInterface, LobeKind::TemporalPredictive].into_iter().filter_map(|kind| {
                trainer.phenotype().lobe_layout().region(kind).map(|region| json!({
                    "lobe": format!("{kind:?}"),
                    "activation": &eval.final_activations[0][region.start as usize..(region.start+region.len) as usize],
                }))
            }).collect::<Vec<_>>();
            rows.push(json!({"tick":tick,"position":observed_position,"visible_goal":visible_goal,"latent_context":latent_context,"lattice_pulse":lattice_pulse,"counter":row.sampling_counter,"action":row.action,"candidates":frame.candidates(),"logits":eval.candidate_logits[0],"ingest_taste_motors":motors,"encoded_hunger_plus_0_1_logit_delta":hunger_probe,"encoded_hunger_3_frozen_frames_delta":hunger_three_frames,"constructed_neuron_activations":debug_activations,"effective_bridges":effective_bridges,"enabled_routes":row.replay_tick.enabled_routes,"outcome":outcome,"reward":row.biological_reward}));
        }
    }
    assert_eq!(before, serde_json::to_vec(&trainer.checkpoint())?);
    let ck = host.checkpoint(&trainer)?;
    let final_neural = &ck
        .residents
        .iter()
        .find(|r| r.neural.organism_id == own)
        .ok_or("missing final neural owner")?
        .neural;
    let change_stats = |a: &[f32], b: &[f32]| -> Result<serde_json::Value> {
        if a.len() != b.len() || a.len() != trainer.phenotype().synapses().len() {
            return Err("neural weight state length mismatch".into());
        }
        let mut changed = 0usize;
        let mut recurrent = 0usize;
        let mut decoder = 0usize;
        let mut max_abs_delta = 0f32;
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            if x.to_bits() != y.to_bits() {
                changed += 1;
                match trainer.phenotype().synapses()[i].kind() {
                    CompiledSynapseKind::Recurrent => recurrent += 1,
                    CompiledSynapseKind::Decoder(_) => decoder += 1,
                }
                max_abs_delta = max_abs_delta.max((y - x).abs());
            }
        }
        Ok(
            json!({"changed":changed,"recurrent":recurrent,"decoder":decoder,"max_abs_delta":max_abs_delta}),
        )
    };
    let neural_changes = json!({"fast":change_stats(&initial_neural.fast, &final_neural.fast)?,"lifetime":change_stats(&initial_neural.lifetime, &final_neural.lifetime)?,"eligibility":change_stats(&initial_neural.eligibility, &final_neural.eligibility)?,"initial_weight_generation":initial_neural.weight_generation,"final_weight_generation":final_neural.weight_generation});
    let bytes = serde_json::to_vec(&ck)?;
    let mut cold = CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes)?, &trainer)?;
    assert_eq!(bytes, serde_json::to_vec(&cold.checkpoint(&trainer)?)?);
    let hot = host.snapshot().step(&trainer, 1.)?;
    let fresh = cold.step(&trainer, 1.)?;
    assert_eq!(
        serde_json::to_vec(&hot.decisions)?,
        serde_json::to_vec(&fresh.decisions)?
    );
    let mut hot_host = host.snapshot();
    hot_host.step(&trainer, 1.)?;
    assert_eq!(
        serde_json::to_vec(&hot_host.checkpoint(&trainer)?)?,
        serde_json::to_vec(&cold.checkpoint(&trainer)?)?
    );
    let sleep_probe = if std::env::var_os("ALIFE_RESEARCH_SLEEP").is_some() {
        let mut sleeper = host.snapshot();
        sleeper.request_recovery_sleep(own)?;
        let mut phases = Vec::new();
        let mut consolidated = None;
        let mut resumed_at = None;
        for i in 0..80 {
            let step = sleeper.step(&trainer, 1.)?;
            let snap = sleeper.checkpoint(&trainer)?;
            let resident = snap
                .residents
                .iter()
                .find(|r| r.neural.organism_id == own)
                .ok_or("missing sleeping owner")?;
            phases.push(format!("{:?}", resident.sleep.state().phase));
            if step.sleep_consolidations > 0 {
                consolidated = Some(resident.neural.clone());
            }
            if consolidated.is_some() && !step.decisions.is_empty() {
                resumed_at = Some(i + 1);
                break;
            }
        }
        let consolidated = consolidated.ok_or("no sleep consolidation within 80 intervals")?;
        let resumed_at = resumed_at.ok_or("no decision resumed within 80 intervals")?;
        let wake = sleeper.checkpoint(&trainer)?;
        let wake_neural = &wake
            .residents
            .iter()
            .find(|r| r.neural.organism_id == own)
            .ok_or("missing waking owner")?
            .neural;
        let bytes = serde_json::to_vec(&wake)?;
        let mut restored =
            CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&bytes)?, &trainer)?;
        let mut continued = sleeper.snapshot();
        continued.step(&trainer, 1.)?;
        restored.step(&trainer, 1.)?;
        if serde_json::to_vec(&continued.checkpoint(&trainer)?)?
            != serde_json::to_vec(&restored.checkpoint(&trainer)?)?
        {
            return Err("sleep wake cold continuation mismatch".into());
        }
        json!({"forced_recovery":true,"phases":phases,"resumed_at":resumed_at,"pre_sleep_fast_to_consolidated":change_stats(&final_neural.fast,&consolidated.fast)?,"pre_sleep_lifetime_to_consolidated":change_stats(&final_neural.lifetime,&consolidated.lifetime)?,"pre_sleep_eligibility_to_consolidated":change_stats(&final_neural.eligibility,&consolidated.eligibility)?,"pre_sleep_generation":final_neural.weight_generation,"consolidated_generation":consolidated.weight_generation,"resumed_generation":wake_neural.weight_generation,"exact_cold_next":true})
    } else {
        serde_json::Value::Null
    };
    let r = host.world().organism_registry().get(own).unwrap();
    Ok(
        json!({"sleep_probe":sleep_probe,"research_fast_write_sham":std::env::var_os("ALIFE_RESEARCH_FAST_WRITE_SHAM").is_some(),"neural_changes":neural_changes,"seed":seed,"arm":arm,"scene":scene,"edits":edits,"source_digest":source.digest(),"candidate_digest":candidate_digest,"changed_weights":changed_weights,"experimental_graph":experimental,"active_tiles":trainer.phenotype().budgets().global.active_tiles,"microstep_count":trainer.phenotype().microstep_count(),"total_synapses":trainer.phenotype().synapses().len(),"preparation_ticks":preparation_ticks,"target_hazard_pain":scene_hazard,"initial_biology":initial_biology,"optimizer_unchanged":true,"cold_next_step_exact":true,"target_active":!host.world().entity(object).unwrap().consumed,"final_position":host.world().entity(host.world().entity_id("learner").unwrap()).unwrap().position,"final_target_carrier":host.world().entity(object).unwrap().carried_by,"final_body":r.biochemistry().body,"final_homeostasis":r.biochemistry().homeostasis,"rows":rows}),
    )
}
fn graph_report(p: &BrainPhenotype) -> serde_json::Value {
    use std::collections::{BTreeSet, VecDeque};
    let mut rows = Vec::new();
    for a in p.sensor_encoder().assignments() {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::new();
        seen.insert(a.target_neuron());
        queue.push_back((a.target_neuron(), 0usize));
        let mut family_depth = std::collections::BTreeMap::new();
        while let Some((n, depth)) = queue.pop_front() {
            for s in p.synapses().iter().filter(|s| s.source() == n) {
                match s.kind() {
                    CompiledSynapseKind::Recurrent => {
                        if seen.insert(s.target()) {
                            queue.push_back((s.target(), depth + 1));
                        }
                    }
                    CompiledSynapseKind::Decoder(c)
                        if c.head() == DecoderHeadKind::ActionCandidate =>
                    {
                        family_depth
                            .entry(format!("{:?}", c.family()))
                            .or_insert(depth);
                    }
                    _ => {}
                }
            }
        }
        rows.push(json!({"group":a.source_group(),"lane":a.source_index(),"neuron":a.target_neuron(),"outgoing_recurrent":p.synapses().iter().filter(|s|s.source()==a.target_neuron() && s.kind()==CompiledSynapseKind::Recurrent).count(),"reachable_neurons":seen.len(),"shortest_recurrent_depth_to_family":family_depth}));
    }
    let routes = p.projections().iter().map(|route| json!({"index":route.route_index(), "source":format!("{:?}",route.source_lobe()), "target":format!("{:?}",route.target_lobe()), "kind":format!("{:?}",route.projection_type()), "range":route.synapse_range()})).collect::<Vec<_>>();
    let edges = p.synapses().iter().enumerate().filter(|(_,s)|s.kind()==CompiledSynapseKind::Recurrent && [7u16,8,9,10,11,12,13,14].contains(&s.route_index())).map(|(i,s)|json!({"index":i,"route":s.route_index(),"source":s.source(),"target":s.target(),"weight":s.genetic_weight()})).collect::<Vec<_>>();
    json!({"structural_reachability_only":true,"all_directed_hops":true,"sensor_assignments":rows,"context_routes":routes,"context_edges":edges})
}
fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let path = args.get(1).ok_or("output JSON required")?;
    if matches!(
        args.get(2).map(String::as_str),
        Some("graph" | "graph-experimental")
    ) {
        let (_, p, _) = source(2026110701, args[2] == "graph-experimental")?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        use std::io::Write;
        output.write_all(&serde_json::to_vec_pretty(&graph_report(&p))?)?;
        return Ok(());
    }
    let ticks: usize = args.get(2).map_or(Ok(12), |s| s.parse())?;
    let gain: f32 = args.get(3).map_or(Ok(-8.), |s| s.parse())?;
    let mode = args.get(4).map_or("surgery", String::as_str);
    let seeds: usize = args.get(5).map_or(Ok(8), |s| s.parse())?;
    let seed_start: u64 = args.get(6).map_or(Ok(2026110701), |s| s.parse())?;
    let scenes = args
        .get(7)
        .map_or("food,bitter-food,toy,far-food", String::as_str)
        .split(',')
        .collect::<Vec<_>>();
    let initial_energy: f32 = args.get(8).map_or(Ok(0.4), |s| s.parse())?;
    let preparation_ticks: usize = args.get(9).map_or(Ok(0), |s| s.parse())?;
    let target_hazard_pain: f32 = args.get(10).map_or(Ok(0.), |s| s.parse())?;
    if !target_hazard_pain.is_finite() || !(0.0..=1.).contains(&target_hazard_pain) {
        return Err("target hazard pain must be finite0..1".into());
    }
    if preparation_ticks > 1200 {
        return Err("preparation ticks must be0..1200".into());
    }
    if seed_start.checked_add(seeds as u64).is_none()
        || scenes.is_empty()
        || scenes.len() > 10
        || scenes.iter().any(|s| {
            ![
                "food",
                "bitter-food",
                "neutral-food",
                "toy",
                "far-food",
                "food-left",
                "bitter-food-left",
                "food-back",
                "far-food-left",
                "far-food-back",
                "obstacle-food",
                "maze-clear",
                "maze-left",
                "maze-right",
                "maze-near",
                "maze-neutral",
                "maze-bitter",
            ]
            .contains(s)
        })
        || !initial_energy.is_finite()
        || !(0.1..=1.).contains(&initial_energy)
    {
        return Err("invalid bounded seed, scene or energy".into());
    }
    if !(1..=120).contains(&ticks)
        || !(1..=32).contains(&seeds)
        || !gain.is_finite()
        || gain.abs() > 32.
    {
        return Err("bounded disposable probe: ticks1..120,seeds1..32,gain finite abs<=32".into());
    }
    let arms = match mode {
        "surgery" => vec![
            ("untouched", 0.),
            ("taste-readout", gain),
            ("wrong-sign", -gain),
        ],
        "validate" => vec![("untouched", 0.), ("whole-care", 0.)],
        "balanced" => vec![("whole-care", 0.), ("whole-care-balanced", 0.)],
        "narrow" => vec![
            ("whole-care", 0.),
            ("whole-care-balanced", 0.),
            ("whole-care-narrow", 0.),
        ],
        "ingest-factor" => vec![("whole-care", 0.), ("whole-care-ingest-only", 0.)],
        "boost-factor" => vec![("whole-care", 0.), ("whole-care-ingest-boost", 0.)],
        "approach-factor" => vec![("whole-care", 0.), ("whole-care-approach70", 0.)],
        "nav-baseline" => vec![("nav-base", 0.)],
        "nav-factor" => vec![("nav-base", 0.), ("nav-edit", 0.)],
        "nav-combined-factor" => vec![("nav-base", 0.), ("nav-combined", 0.)],
        "nav-lattice-factor" => vec![("nav-base", 0.), ("nav-lattice", 0.)],
        "nav-strong-factor" => vec![("nav-base", 0.), ("nav-strong", 0.)],
        "nav-care-factor" => vec![("nav-base", 0.), ("nav-care", 0.)],
        "nav-care2-factor" => vec![("nav-base", 0.), ("nav-care2", 0.)],
        "nav-map-factor" => vec![("nav-base", 0.), ("nav-map", 0.)],
        "bold-factor" => vec![("whole-care", 0.), ("whole-care-bold", 0.)],
        "contact" => vec![("whole-care-v5", 0.), ("whole-care", 0.)],
        "whole" => vec![
            ("untouched", 0.),
            ("zero-excitatory", 0.),
            ("whole-care", 0.),
            ("whole-care-no-need", 0.),
            ("whole-care-no-tonic", 0.),
            ("whole-care-no-reference", 0.),
        ],
        _ => return Err("mode must be surgery, whole, validate or contact".into()),
    };
    // Reserve a new output directory before any simulation. Never overwrite a prior receipt.
    std::fs::create_dir(path)?;
    let mut trial_files = Vec::new();
    for seed in (seed_start..).take(seeds) {
        for (scene_index, &scene) in scenes.iter().enumerate() {
            for &(arm, gain) in &arms {
                let r = trial(TrialSpec {
                    seed,
                    arm,
                    gain,
                    scene,
                    ticks,
                    receipt_root: path,
                    write_asset: scene_index == 0,
                    initial_energy,
                    preparation_ticks,
                    target_hazard_pain,
                })?;
                use std::io::Write;
                let mut receipt = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(format!("{path}/{seed}.{scene}.{arm}.json"))?;
                receipt.write_all(&serde_json::to_vec(&r)?)?;
                receipt.sync_all()?;
                eprintln!(
                    "{seed} {scene} {arm} energy={} active={}",
                    r["final_body"]["energy"], r["target_active"]
                );
                trial_files.push(format!("{seed}.{scene}.{arm}.json"));
            }
        }
    }
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(format!("{path}/receipt.pending.json"))?;
    serde_json::to_writer(
        &mut output,
        &json!({"receipt_format":"journal-v2","complete":true,"backend":"native-cpu","experimental":true,"promoted":false,"initial_energy":initial_energy,"seed_start":seed_start,"seed_count":seeds,"arms":arms.iter().map(|(arm,_)|*arm).collect::<Vec<_>>(),"scenes":scenes,"preparation_ticks":preparation_ticks,"target_hazard_pain":target_hazard_pain,"newborn_homeostasis_unchanged":preparation_ticks==0,"gain":gain,"ticks":ticks,"mode":mode,"trial_files":trial_files}),
    )?;
    output.sync_all()?;
    // Publish only after every per-trial receipt and the compact index are durable.
    // hard_link fails rather than replacing an existing completed receipt.
    std::fs::hard_link(
        format!("{path}/receipt.pending.json"),
        format!("{path}/receipt.json"),
    )?;
    std::fs::remove_file(format!("{path}/receipt.pending.json"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_weight_builder_preserves_signs_and_separates_need_axes() {
        let (_, p, source) = source(2026110711, true).unwrap();
        let (weights, circuits) = whole_care(&p).unwrap();
        assert_eq!(weights.len(), 32768);
        let (old, _) = whole_care_with_contact(&p, -5.).unwrap();
        let changes = p
            .synapses()
            .iter()
            .zip(weights.iter().zip(&old))
            .filter(|(_, (a, b))| a.to_bits() != b.to_bits())
            .collect::<Vec<_>>();
        assert_eq!(changes.len(), 2);
        for (s, _) in changes {
            assert!(matches!(s.kind(), CompiledSynapseKind::Decoder(c)
                if c.head() == DecoderHeadKind::ActionCandidate
                && c.family() == CandidateActionFamily::Approach && c.input_lane() == 18));
        }
        for (s, w) in p.synapses().iter().zip(&weights) {
            assert!(w.is_finite());
            if s.kind() == CompiledSynapseKind::Recurrent {
                match p.projections()[s.route_index() as usize].projection_type() {
                    ProjectionType::LateralInhibition => assert!(*w < 0.),
                    ProjectionType::Homeostatic | ProjectionType::MotorProposal => {
                        assert!(*w >= 0.)
                    }
                    _ => {}
                }
            }
        }
        let mut axes = std::collections::BTreeMap::new();
        for c in circuits.iter().filter(|c| c["role"] == "need") {
            let lane = c["homeostatic_lane"].as_u64().unwrap();
            let indices = c["bridge_synapses"].as_array().unwrap();
            let reserve = p.synapses()[indices[0].as_u64().unwrap() as usize].target();
            let temporal = p.synapses()[indices[1].as_u64().unwrap() as usize].target();
            if let Some(old) = axes.insert(lane, (reserve, temporal)) {
                assert_eq!(old, (reserve, temporal));
            }
        }
        let values = axes.values().collect::<Vec<_>>();
        for (i, a) in values.iter().enumerate() {
            for b in &values[i + 1..] {
                assert_ne!(a.0, b.0);
                assert_ne!(a.1, b.1);
            }
        }
        FoundationWeightAsset::from_trained_weights(
            &p,
            weights,
            source.manifest().training_stage(),
        )
        .unwrap();
    }
}

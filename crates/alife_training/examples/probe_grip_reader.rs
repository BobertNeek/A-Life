//! Source-only construction and detached neural fixtures for the grip candidate.
//! No autonomous assay entry point is enabled. A reviewed exposure plan is required.
#[cfg(test)]
#[path = "support/care_candidate.rs"]
#[allow(dead_code)]
mod care_candidate;
#[cfg(test)]
use alife_core::*;
#[cfg(test)]
use alife_runtime::canonical_preparation::foundation_construction_development;
#[cfg(test)]
use alife_world::*;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Fresh inherited genes only. No acquired life, optimizer or private file input.
#[cfg(test)]
fn candidate(seed: u64, grip_reader: bool) -> Result<(BrainPhenotype, FoundationWeightAsset)> {
    let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    let builtin = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile)?;
    let game = create_canonical_new_game_with_n2048_candidate(&config, &builtin)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("no fresh founder")?;
    let genome = record.phenotype().brain_genome.clone();
    let development = foundation_construction_development(
        &genome,
        &BrainCapacityClass::n2048(),
        &record.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    let (p, _) = PhenotypeCompiler::compile_n2048_grip_reader_graph(
        genome.clone(),
        development.clone(),
        SensorProfile::GroundedGripObjectSlotsV1,
    )?;
    let (mut weights, _) = care_candidate::whole_care(&p)?;
    let mut counts = std::collections::BTreeMap::new();
    for synapse in p.synapses() {
        if let CompiledSynapseKind::Decoder(c) = synapse.kind() {
            if c.head() == DecoderHeadKind::MemoryContext {
                *counts
                    .entry((c.family().raw(), c.input_lane()))
                    .or_insert(0usize) += 1;
            }
        }
    }
    for (synapse, weight) in p.synapses().iter().zip(&mut weights) {
        let CompiledSynapseKind::Decoder(c) = synapse.kind() else {
            continue;
        };
        if c.head() == DecoderHeadKind::ActionCandidate {
            *weight *= 0.25;
        }
        if c.head() != DecoderHeadKind::MemoryContext {
            continue;
        }
        // Preserve the v12 inherited target-reader and Ingest-family coefficients.
        // Its source file and historical assets are not edited or loaded here.
        let total = match (c.family(), c.input_lane()) {
            (CandidateActionFamily::Ingest, 25) => -16.,
            (CandidateActionFamily::Ingest, 26) => -64.,
            (CandidateActionFamily::Ingest, 32) => 32.,
            (CandidateActionFamily::Approach | CandidateActionFamily::Contact, 25) => -8.,
            (CandidateActionFamily::Approach | CandidateActionFamily::Contact, 26) => -32.,
            (CandidateActionFamily::Avoid, 25) => 16.,
            (CandidateActionFamily::Avoid, 26) => 128.,
            (CandidateActionFamily::Contact, 53) if grip_reader => 32.,
            (CandidateActionFamily::Other, 53) if grip_reader => -32.,
            _ => 0.,
        };
        *weight = total / counts[&(c.family().raw(), c.input_lane())] as f32;
    }
    let asset = FoundationWeightAsset::from_trained_weights(
        &p,
        weights,
        TrainingStageManifest::bootstrap(),
    )?;
    let (bound, _) = PhenotypeCompiler::compile_n2048_grip_reader_foundation_candidate(
        genome,
        development,
        asset.clone(),
    )?;
    Ok((bound, asset))
}

fn main() -> Result<()> {
    Err("grip candidate exposure is disabled; this source currently permits compiled unit fixtures only".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_training::{
        cpu::CpuFoundationTrainer, AdamWConfig, StageTrainableMask, TrainingInitialState,
        TrainingReplayCandidate, TrainingReplayTick, TrainingSequence,
    };

    #[test]
    fn inherited_reader_changes_only_sixteen_explicit_port_genes() {
        let (off, off_asset) = candidate(2026110901, false).unwrap();
        let (on, on_asset) = candidate(2026110901, true).unwrap();
        assert_eq!(off.synapses().len(), 32768);
        assert_eq!(on.synapses().len(), 32768);
        assert_eq!(off.candidate_decoder(), on.candidate_decoder());
        let mut changed = 0;
        for ((synapse, off), on) in off
            .synapses()
            .iter()
            .zip(off_asset.weights())
            .zip(on_asset.weights())
        {
            if off != on {
                let CompiledSynapseKind::Decoder(c) = synapse.kind() else {
                    panic!("port must be neural decoder")
                };
                assert_eq!(c.head(), DecoderHeadKind::MemoryContext);
                assert_eq!(c.input_lane(), 53);
                assert_eq!(*off, 0.0);
                assert_eq!(
                    *on,
                    match c.family() {
                        CandidateActionFamily::Contact => 4.0,
                        CandidateActionFamily::Other => -4.0,
                        _ => panic!("unexpected family"),
                    }
                );
                changed += 1;
            }
        }
        assert_eq!(changed, 16);
    }

    fn sequence(
        p: &BrainPhenotype,
        inputs: &[(CandidateActionFamily, f32, f32)],
    ) -> TrainingSequence {
        let n = p.neuron_count() as usize;
        TrainingSequence {
            phenotype_hash: p.phenotype_hash(),
            initial: TrainingInitialState {
                activations: vec![0.; n],
                activity_ema: vec![0.; n],
                metabolic_load: vec![0.; n],
                dendrites: DendriticBranchSet::new(Vec::new()).unwrap(),
            },
            ticks: vec![TrainingReplayTick {
                encoded_inputs: vec![0.; n],
                projection_gain: 1.,
                local_threshold_shift: 0.,
                microstep_count: u32::from(p.microstep_count()),
                enabled_routes: vec![true; p.projections().len()],
                effective_weight_offsets: vec![0.; p.synapses().len()],
                structural_synapses: Vec::new(),
                candidates: inputs
                    .iter()
                    .map(|(family, ordinary, grip)| {
                        let mut row = [0.; 54];
                        row[32] = *ordinary;
                        row[53] = *grip;
                        TrainingReplayCandidate {
                            family: *family,
                            decoder_inputs: row,
                            innate_bias: 0.,
                        }
                    })
                    .collect(),
            }],
            burn_in_ticks: 0,
            memory_candidate_gain: 12.,
        }
    }

    #[test]
    fn native_readout_represents_matched_four_contexts_and_zero_port_controls() {
        let (p, asset) = candidate(2026110901, true).unwrap();
        let trainer = CpuFoundationTrainer::new(
            p.clone(),
            asset,
            StageTrainableMask::recurrent_only(&p).unwrap(),
            AdamWConfig::default(),
        )
        .unwrap();
        // Detached input fixtures, not observed learning or four exposed lives.
        // All current bodily pain, senses and target-memory inputs are zero.
        let q = sequence(
            &p,
            &[
                (CandidateActionFamily::Contact, 0.25, 0.25), // benign/unheld Close
                (CandidateActionFamily::Contact, -0.25, -0.25), // harmful/unheld Close
                (CandidateActionFamily::Other, 0.25, 0.25),   // benign/held Release
                (CandidateActionFamily::Other, -0.25, -0.25), // harmful/held Release
                (CandidateActionFamily::Other, -0.25, 0.),    // bad Play/Vocal own memory
                (CandidateActionFamily::Other, 0., 0.),       // no acquired memory
            ],
        );
        let scores = &trainer.evaluate_replay(&q).unwrap().candidate_logits[0];
        assert_eq!(scores, &[8., -8., -8., 8., 0., 0.]);
        let before = serde_json::to_vec(&trainer.checkpoint()).unwrap();
        let mut no_port = q.clone();
        for c in &mut no_port.ticks[0].candidates {
            c.decoder_inputs[53] = 0.;
        }
        assert_eq!(
            trainer.evaluate_replay(&no_port).unwrap().candidate_logits[0],
            vec![0.; 6]
        );
        assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), before);
    }

    #[test]
    fn native_port_gradients_use_the_exact_frozen_input_and_not_other_memory() {
        let (p, asset) = candidate(2026110901, true).unwrap();
        let ids = p.synapses().iter().enumerate().filter_map(|(i,s)|
            matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::MemoryContext && c.input_lane()==53)
                .then_some(i as u32)).collect::<Vec<_>>();
        let trainer = CpuFoundationTrainer::new(
            p.clone(),
            asset,
            StageTrainableMask::from_synapse_indices(&p, &ids).unwrap(),
            AdamWConfig::default(),
        )
        .unwrap();
        let q = sequence(
            &p,
            &[
                (CandidateActionFamily::Other, -0.25, -0.25),
                (CandidateActionFamily::Other, -0.25, 0.),
            ],
        );
        let gradient = trainer.probe_replay_gradients(&q, &[vec![1., 0.]]).unwrap();
        for id in &ids {
            let CompiledSynapseKind::Decoder(c) = p.synapses()[*id as usize].kind() else {
                unreachable!()
            };
            assert_eq!(
                gradient.gradients[*id as usize],
                if c.family() == CandidateActionFamily::Other {
                    -0.25
                } else {
                    0.
                }
            );
        }
        let control = trainer.probe_replay_gradients(&q, &[vec![0., 1.]]).unwrap();
        assert!(ids.iter().all(|id| control.gradients[*id as usize] == 0.));
    }
}

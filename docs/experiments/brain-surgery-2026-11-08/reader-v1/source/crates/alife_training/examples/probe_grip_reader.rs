//! Source-only construction and detached neural fixtures for the grip candidate.
//! No autonomous assay entry point is enabled. A reviewed exposure plan is required.
#[cfg(test)]
#[path = "support/care_candidate.rs"]
#[allow(dead_code)]
mod care_candidate;
#[cfg(test)]
#[path = "support/world_swap_legacy_v12.rs"]
mod world_swap_legacy_v12;
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

    // Constructed causal fixtures: task priors and projected memories are supplied,
    // not learned. No autonomous-life claim or persistent brain input is made.
    #[test]
    fn matched_reader_utility_counterexamples_and_controls() {
        use alife_core::grip_research::GripOperationV1;
        let (off, off_asset) = candidate(2026110901, false).unwrap();
        let (on, on_asset) = candidate(2026110901, true).unwrap();
        assert_eq!(off.projections(), on.projections());
        assert_eq!(off.neuron_dynamics(), on.neuron_dynamics());
        assert_eq!(off.plasticity_receptors(), on.plasticity_receptors());
        let trainers = [(&off, off_asset), (&on, on_asset)].map(|(p, a)| {
            CpuFoundationTrainer::new(
                p.clone(),
                a,
                StageTrainableMask::recurrent_only(p).unwrap(),
                AdamWConfig::default(),
            )
            .unwrap()
        });
        // name, operation, lane53, target pain delta, held, operation task prior,
        // ordinary-Other task prior. Task labels are observer-only, not new policy.
        let cases = [
            (
                "aversive_ingestion_pickup",
                GripOperationV1::Close,
                -0.25,
                0.,
                false,
                1.,
                0.,
            ),
            (
                "safe_food_pickup",
                GripOperationV1::Close,
                0.25,
                0.,
                false,
                1.,
                0.,
            ),
            (
                "useful_harmless_aversive_object_pickup",
                GripOperationV1::Close,
                -0.25,
                0.,
                false,
                1.,
                0.,
            ),
            (
                "useful_aversive_object_transport_hold",
                GripOperationV1::Release,
                -0.25,
                0.,
                true,
                0.,
                1.,
            ),
            (
                "aversive_object_release_at_destination",
                GripOperationV1::Release,
                -0.25,
                0.,
                true,
                1.,
                0.,
            ),
            (
                "safe_food_release_at_destination",
                GripOperationV1::Release,
                0.25,
                0.,
                true,
                1.,
                0.,
            ),
            (
                "other_target_no_ingestion_evidence",
                GripOperationV1::Close,
                0.,
                0.,
                false,
                1.,
                0.,
            ),
            (
                "unrelated_pain_no_target_evidence",
                GripOperationV1::Close,
                0.,
                0.,
                false,
                1.,
                0.,
            ),
            (
                "empty_memory",
                GripOperationV1::Close,
                0.,
                0.,
                false,
                1.,
                0.,
            ),
            (
                "unknown_memory_zero_projection",
                GripOperationV1::Release,
                0.,
                0.,
                true,
                1.,
                0.,
            ),
            (
                "own_family_only_negative",
                GripOperationV1::Close,
                0.,
                0.,
                false,
                1.,
                0.,
            ),
            (
                "target_pain_existing_route",
                GripOperationV1::Close,
                0.,
                0.125,
                false,
                1.,
                0.,
            ),
        ];
        let mut rows = Vec::new();
        for (name, operation, port, target_pain, held, prior, other_prior) in cases {
            for (arm, trainer) in trainers.iter().enumerate() {
                let p = trainer.phenotype();
                let family = if operation == GripOperationV1::Close {
                    CandidateActionFamily::Contact
                } else {
                    CandidateActionFamily::Other
                };
                let mut q = sequence(
                    p,
                    &[
                        (family, -0.25, port),
                        (CandidateActionFamily::Other, -0.25, 0.),
                    ],
                );
                q.ticks[0].candidates[0].innate_bias = prior;
                q.ticks[0].candidates[1].innate_bias = other_prior;
                q.ticks[0].candidates[0].decoder_inputs[26] = target_pain;
                // Independent bodily signal is common to both arms and does not
                // masquerade as a target consequence or operation recall.
                if name == "unrelated_pain_no_target_evidence" {
                    q.ticks[0].encoded_inputs[0] = 0.125;
                }
                let before = serde_json::to_vec(&trainer.checkpoint()).unwrap();
                let scores = trainer.evaluate_replay(&q).unwrap().candidate_logits[0].clone();
                assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), before);
                let port_component = if arm == 0 {
                    0.
                } else if operation == GripOperationV1::Close {
                    32. * port
                } else {
                    -32. * port
                };
                let target_component = if operation == GripOperationV1::Close {
                    -32. * target_pain
                } else {
                    0.
                };
                let memory_component = (port_component + target_component).clamp(-12., 12.);
                assert!((scores[0] - (prior + memory_component)).abs() < 0.00001);
                assert_eq!(scores[1], other_prior);
                let selected_operation = scores[0] > scores[1]; // fixture argmax, not production stochastic sampling
                let organism = OrganismId(7);
                let mut world = HeadlessScenarioBuilder::new(0xfeed_2400)
                    .agent("fixture", organism, Vec3f::ZERO)
                    .food("object", Vec3f::new(0.5, 0., 0.), 0.4)
                    .food("destination", Vec3f::new(4., 0., 0.), 0.4)
                    .build()
                    .unwrap();
                let target = world.entity_id("object").unwrap();
                let execute = |world: &mut HeadlessWorld, op: GripOperationV1| {
                    let draft = world
                        .perception_frame_draft(
                            organism,
                            world.tick(),
                            SensorProfile::GroundedGripObjectSlotsV1,
                            HomeostaticSnapshot::baseline(world.tick()),
                        )
                        .unwrap();
                    let observation = world.observe_research_grip(&draft, target).unwrap();
                    let command = ActionCommand::structured(
                        organism,
                        op.action_id(),
                        ActionKind::Interact,
                        ActionTarget::new(Some(target), None),
                        Intensity::new(1.).unwrap(),
                        DurationTicks::new(1),
                        Confidence::new(1.).unwrap(),
                        0,
                        None,
                        None,
                        None,
                    )
                    .unwrap();
                    world
                        .execute_research_grip(&draft, &observation, op, command)
                        .unwrap()
                };
                if held {
                    assert!(
                        execute(&mut world, GripOperationV1::Close)
                            .execution
                            .succeeded
                    );
                }
                let origin_x = world.entity(target).unwrap().position.x;
                let movement = ActionCommand::structured(
                    organism,
                    ActionKind::Move.canonical_id(),
                    ActionKind::Move,
                    ActionTarget::new(None, Some(Vec3f::new(3., 0., 0.))),
                    Intensity::new(1.).unwrap(),
                    DurationTicks::new(1),
                    Confidence::new(1.).unwrap(),
                    0,
                    None,
                    None,
                    None,
                )
                .unwrap();
                if name.ends_with("at_destination") {
                    for _ in 0..2 {
                        let moved = world.apply_command(&movement).unwrap();
                        assert!(moved.execution.succeeded);
                        world.try_advance_tick().unwrap();
                    }
                    assert!(world.entity(target).unwrap().position.x > origin_x);
                }
                let held_before = world.entity(target).unwrap().carried_by.is_some();
                let succeeded = if selected_operation {
                    execute(&mut world, operation).execution.succeeded
                } else {
                    world
                        .apply_command(&HeadlessWorldCommand::idle(organism).unwrap())
                        .unwrap()
                        .execution
                        .succeeded
                };
                if name == "useful_aversive_object_transport_hold" {
                    for _ in 0..2 {
                        let moved = world.apply_command(&movement).unwrap();
                        assert!(moved.execution.succeeded);
                        world.try_advance_tick().unwrap();
                    }
                    assert_eq!(
                        world.entity(target).unwrap().position.x > origin_x,
                        arm == 0
                    );
                }
                let held_after = world.entity(target).unwrap().carried_by.is_some();
                assert!(succeeded);
                assert_eq!(held_before, held);
                assert_eq!(
                    held_after,
                    if selected_operation {
                        operation == GripOperationV1::Close
                    } else {
                        held
                    }
                );
                rows.push(serde_json::json!({"case":name,"arm":if arm==0 {"off"} else {"on"},"action_id":operation.action_id().0,"lane53":port,"own_family_lane32":-0.25,"own_family_contribution":0.,"target_lane26":target_pain,"target_contribution":target_component,"operation_contribution":port_component,"memory_clamped":memory_component,"task_prior":prior,"ordinary_other_prior":other_prior,"logits":scores,"fixture_argmax":if selected_operation {"operation"} else {"ordinary_other_idle"},"held_before":held_before,"held_after":held_after,"reachable":true,"physical_success":succeeded,"origin_x":origin_x,"final_x":world.entity(target).unwrap().position.x,"consumed":world.entity(target).unwrap().consumed}));
            }
        }
        println!(
            "UTILITY_FIXTURE_JSON={}",
            serde_json::to_string(&rows).unwrap()
        );
        assert_eq!(rows.len(), 24);
        // Explicitly preserve the negative result: aversive valence suppresses a
        // useful pickup and premature release defeats holding during transport.
        assert_eq!(rows[4]["held_after"], true);
        assert_eq!(rows[5]["held_after"], false);
        assert_eq!(rows[6]["held_after"], true);
        assert_eq!(rows[7]["held_after"], false);
        assert_eq!(rows[10]["held_after"], false);
        assert_eq!(rows[11]["held_after"], true);
    }

    #[test]
    fn reconstructed_v12_identity_and_all_coordinate_changes_are_accounted_for() {
        let inherited = world_swap_legacy_v12::construct().unwrap();
        let hex = |d: Blake3Digest| {
            d.bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        use sha2::{Digest, Sha256};
        let inherited_sha256 = format!(
            "{:x}",
            Sha256::digest(inherited.encode_canonical().unwrap())
        );
        assert_eq!(
            inherited_sha256,
            "6edcd6dd1dc5bfedf8f7b73bf605bb480e36c0ba022a316840a6f0bd87629632"
        );
        let (_, old, _) = care_candidate::source_memory_gain(2026110901).unwrap();
        let (off, off_asset) = candidate(2026110901, false).unwrap();
        let (on, on_asset) = candidate(2026110901, true).unwrap();
        assert_eq!(inherited.weights().len(), off_asset.weights().len());
        assert!(inherited
            .weights()
            .iter()
            .zip(off_asset.weights())
            .all(|(a, b)| a.to_bits() == b.to_bits()));
        let mut retargeted = Vec::new();
        for (index, (a, b)) in old.synapses().iter().zip(off.synapses()).enumerate() {
            assert_eq!(a.source(), b.source());
            assert_eq!(a.target(), b.target());
            assert_eq!(a.route_index(), b.route_index());
            assert_eq!(a.alpha(), b.alpha());
            assert_eq!(a.receptor_index(), b.receptor_index());
            if a.kind() != b.kind() {
                let (CompiledSynapseKind::Decoder(a), CompiledSynapseKind::Decoder(b)) =
                    (a.kind(), b.kind())
                else {
                    panic!("nondecoder change")
                };
                assert_eq!(a.head(), DecoderHeadKind::MemoryContext);
                assert_eq!(a.head(), b.head());
                assert_eq!(a.family(), b.family());
                assert_eq!(a.motor_index(), b.motor_index());
                assert_eq!(a.input_lane(), 31);
                assert_eq!(b.input_lane(), 53);
                assert_eq!(off_asset.weights()[index], 0.);
                retargeted.push(index);
            }
        }
        assert_eq!(retargeted.len(), 64);
        assert!(off.cognitive_channel_plan().is_none());
        assert_eq!(off.projections(), on.projections());
        // Family/head/row endpoint identity and every non-weight synapse field
        // are identical between the two new arms.
        for (a, b) in off.synapses().iter().zip(on.synapses()) {
            assert_eq!(a.source(), b.source());
            assert_eq!(a.target(), b.target());
            assert_eq!(a.kind(), b.kind());
            assert_eq!(a.route_index(), b.route_index());
            assert_eq!(a.alpha(), b.alpha());
            assert_eq!(a.receptor_index(), b.receptor_index());
        }
        println!(
            "IDENTITY_FIXTURE_JSON={}",
            serde_json::json!({"legacy_v12_sha256":inherited_sha256,"legacy_v12_blake3_digest":hex(inherited.digest()),"off_digest":hex(off_asset.digest()),"on_digest":hex(on_asset.digest()),"legacy_to_off_weight_bit_changes":0,"legacy_to_off_coordinate_changes":retargeted,"off_to_on_weight_changes":16,"synapses":32768,"cognitive_decoder":false})
        );
    }
}

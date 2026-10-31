use super::*;

#[test]
fn bulk_offsets_match_reference_across_compiled_layouts() {
    use alife_core::{
        BrainCapacityClass, BrainGenome, DendriticBranch, DendriticBranchSet, DendriticInputRef,
        DevelopmentState, NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
    };
    let mut cases = 0;
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        for profile in [
            SensorProfile::GroundedObjectSlotsV1,
            SensorProfile::GroundedTerrainVisionV1,
        ] {
            let genome = BrainGenome::scaffold(99, capacity.id());
            let development =
                DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
            let p = PhenotypeCompiler::compile(&genome, &capacity, &development, profile).unwrap();
            for (ticks, burn) in [(1, 0), (3, 0), (4, 2)] {
                for topology in 0..3 {
                    for candidates in [1, 25, 32] {
                        for structural in [false, true] {
                            let mut q = sequence(&p, ticks, burn);
                            if topology == 1 {
                                q.initial.dendrites = Default::default();
                            }
                            if topology == 2 {
                                q.initial.dendrites =
                                    DendriticBranchSet::new(vec![DendriticBranch::new(
                                        35,
                                        -0.0,
                                        0.4,
                                        vec![DendriticInputRef::new(6, -0.0).unwrap()],
                                    )
                                    .unwrap()])
                                    .unwrap();
                            }
                            for (i, tick) in q.ticks.iter_mut().enumerate() {
                                let candidate = tick.candidates[0].clone();
                                tick.candidates.resize(candidates, candidate);
                                tick.microstep_count = if i % 2 == 0 {
                                    0
                                } else {
                                    u32::from(p.microstep_count())
                                };
                                tick.enabled_routes[0] = false;
                                tick.effective_weight_offsets[0] = -0.0;
                                if structural {
                                    tick.structural_synapses.push(crate::TrainingFrozenSynapse {
                                        source: 1,
                                        target: 35,
                                        route: 0,
                                        cadence: u32::from(
                                            p.projections()[0].update_cadence().raw(),
                                        ),
                                        effective_weight: -0.125,
                                    });
                                }
                            }
                            q.validate_for(&p).unwrap();
                            let (_, layout) = pack_metadata_and_layout(&p, Some(&q)).unwrap();
                            let expected = reference_pack(&q, layout, p.neuron_count());
                            let actual = pack_replay_sequence(&q, layout, p.neuron_count());
                            assert_eq!(actual, expected);
                            assert!(actual
                                [layout.targets as usize..layout.candidate_records as usize]
                                .iter()
                                .all(|w| *w == 0));
                            assert!(actual[layout.replay_speech_targets as usize..]
                                .iter()
                                .all(|w| *w == 0));
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 324);
}

#[test]
fn bulk_offsets_preserve_float_bits_and_public_nonfinite_rejection() {
    let p = phenotype();
    let bits_cases = [
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0001,
        0xffc1_2345,
        0x7fa0_0001,
    ];
    for bits in bits_cases {
        let mut q = sequence(p, 3, 0);
        for tick in &mut q.ticks {
            let n = tick.effective_weight_offsets.len();
            for index in [0, n / 2, n - 1] {
                tick.effective_weight_offsets[index] = f32::from_bits(bits);
            }
        }
        let (_, layout) = pack_metadata_and_layout(p, Some(&q)).unwrap();
        let actual = pack_replay_sequence(&q, layout, p.neuron_count());
        assert_eq!(actual, reference_pack(&q, layout, p.neuron_count()));
        for (i, tick) in q.ticks.iter().enumerate() {
            let base = layout.context as usize
                + i * (4
                    + tick.enabled_routes.len()
                    + tick.effective_weight_offsets.len()
                    + alife_core::MAX_STRUCTURAL_EDGES * 5)
                + 4
                + tick.enabled_routes.len();
            for index in [
                0,
                tick.effective_weight_offsets.len() / 2,
                tick.effective_weight_offsets.len() - 1,
            ] {
                assert_eq!(actual[base + index], bits);
            }
        }
        assert_eq!(q.validate_for(p).is_ok(), f32::from_bits(bits).is_finite());
    }
}

#[test]
fn invalid_inputs_are_rejected_before_either_packer() {
    let p = phenotype();
    let valid = sequence(p, 3, 0);
    let (_, layout) = pack_metadata_and_layout(p, Some(&valid)).unwrap();
    for case in 0..14 {
        let mut bad = valid.clone();
        match case {
            0 => bad.ticks.clear(),
            1 => bad.burn_in_ticks = 3,
            2 => bad.initial.activations[0] = f32::NAN,
            3 => bad.initial.activity_ema[0] = 2.0,
            4 => {
                bad.ticks[0].encoded_inputs.pop();
            }
            5 => bad.ticks[0].effective_weight_offsets[0] = f32::INFINITY,
            6 => bad.ticks[0].candidates[0].innate_bias = f32::NEG_INFINITY,
            7 => bad.ticks[0].candidates.clear(),
            8 => bad.ticks[0].enabled_routes.clear(),
            9 => bad.ticks[0].microstep_count = 255,
            10 => bad.memory_candidate_gain = -1.0,
            11 => bad.ticks[0].projection_gain = f32::NAN,
            12 => {
                bad.ticks[0].effective_weight_offsets.pop();
            }
            13 => bad.phenotype_hash.0[0] ^= 1,
            _ => unreachable!(),
        }
        let expected = bad.validate_for(p).unwrap_err().to_string();
        for packer in [reference_pack as Packer, pack_replay_sequence as Packer] {
            let mut called = false;
            let result = bad.validate_for(p).map(|()| {
                called = true;
                packer(&bad, layout, p.neuron_count())
            });
            assert_eq!(result.unwrap_err().to_string(), expected);
            assert!(!called);
        }
    }
}

#[test]
#[ignore = "bounded validated host-prefix comparison; no device"]
fn single_call_packing_validated_prefix() {
    let p = phenotype();
    let mut shapes = Vec::new();
    for (ticks, burn) in [(512, 0), (1, 0), (256, 0), (384, 128)] {
        let q = sequence(p, ticks, burn);
        let (_, layout) = pack_metadata_and_layout(p, Some(&q)).unwrap();
        let expected = reference_pack(&q, layout, p.neuron_count());
        let expected_headers =
            build_step_headers(p, layout, AdamWConfig::default(), 16_777_219).unwrap();
        let mut rows = Vec::new();
        for trial in 0..13 {
            for candidate in if trial % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let packer = black_box(if candidate {
                    pack_replay_sequence as Packer
                } else {
                    reference_pack as Packer
                });
                let start = Instant::now();
                black_box(&q).validate_for(black_box(p)).unwrap();
                let words = black_box(packer(&q, layout, p.neuron_count()));
                let headers = black_box(
                    build_step_headers(p, layout, AdamWConfig::default(), 16_777_219).unwrap(),
                );
                let elapsed_ns = start.elapsed().as_nanos() as u64;
                assert_eq!(words, expected);
                assert_eq!(headers, expected_headers);
                if trial >= 2 {
                    rows.push(serde_json::json!({"trial":trial-2,"candidate":candidate,"elapsed_ns":elapsed_ns}));
                }
            }
        }
        shapes.push(serde_json::json!({"ticks":ticks,"burn_in_ticks":burn,"trials":rows}));
    }
    println!(
        "PACKING_PREFIX_JSON={}",
        serde_json::json!({"assay":"single_call_packing_validated_prefix",
        "build_tag":std::env::var("ALIFE_SINGLE_PACK_PROFILE").unwrap_or_default(),"warmups_per_mode":2,
        "trials_per_mode":11,"device_requested":false,"full_equality_every_trial":true,
        "scope":"fresh validation + pack + headers; excludes layout construction, output destruction, queue/device work",
        "shapes":shapes})
    );
}

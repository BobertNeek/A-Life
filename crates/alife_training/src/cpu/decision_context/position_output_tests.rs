use super::*;
fn model(a: [f32; 2]) -> PositionOutputHead2029 {
    PositionOutputHead2029::fresh(
        PositionHead2018::fresh([1., -0.7], AdamWConfig::default()).unwrap(),
        a,
        AdamWConfig::default(),
    )
    .unwrap()
}
fn effect(k: PositionEffectKind2029) -> PositionEffect2018 {
    match k {
        PositionEffectKind2029::Translation => PositionEffect2018 {
            translation: [0.3, -0.4],
            angular: [0.; 2],
        },
        PositionEffectKind2029::Angular => PositionEffect2018 {
            translation: [0.; 2],
            angular: [0.3, -0.4],
        },
        PositionEffectKind2029::Other => PositionEffect2018 {
            translation: [0.; 2],
            angular: [0.; 2],
        },
    }
}
#[test]
fn all_gradients() {
    let state = [0.2, -0.1, 0.9, 0.8];
    for k in [
        PositionEffectKind2029::Translation,
        PositionEffectKind2029::Angular,
    ] {
        let h = model([2., -1.3]);
        let e = effect(k);
        let s = score_output2029(&h, state, k, e, 0.).unwrap();
        let g = backward_output_score2029(&s, 0.7).unwrap();
        let loss = |h: &PositionOutputHead2029, x, e| {
            score_output2029(h, x, k, e, 0.).unwrap().component * 0.7
        };
        for i in 0..4 {
            let mut a = state;
            let mut b = state;
            a[i] += 0.001;
            b[i] -= 0.001;
            assert!(((loss(&h, a, e) - loss(&h, b, e)) / 0.002 - g.inner.state[i]).abs() < 2e-4);
        }
        for i in 0..2 {
            let mut a = h.clone();
            let mut b = h.clone();
            a.inner.gains[i] += 0.001;
            b.inner.gains[i] -= 0.001;
            assert!(
                ((loss(&a, state, e) - loss(&b, state, e)) / 0.002 - g.inner.gains[i]).abs() < 2e-4
            );
            let mut a = h.clone();
            let mut b = h.clone();
            a.output.amplitudes[i] += 0.001;
            b.output.amplitudes[i] -= 0.001;
            assert!(
                ((loss(&a, state, e) - loss(&b, state, e)) / 0.002 - g.amplitudes[i]).abs() < 2e-4
            );
            let mut a = e;
            let mut b = e;
            let expected = if k == PositionEffectKind2029::Translation {
                a.translation[i] += 0.001;
                b.translation[i] -= 0.001;
                g.inner.translation[i]
            } else {
                a.angular[i] += 0.001;
                b.angular[i] -= 0.001;
                g.inner.angular[i]
            };
            assert!(((loss(&h, state, a) - loss(&h, state, b)) / 0.002 - expected).abs() < 2e-4);
        }
    }
}
#[test]
fn zero_output_is_base_exact_and_live() {
    let h = model([0.; 2]);
    for k in [
        PositionEffectKind2029::Translation,
        PositionEffectKind2029::Angular,
    ] {
        let s = score_output2029(&h, [0.2, -0.1, 1., 0.8], k, effect(k), -0.).unwrap();
        assert_eq!(s.final_score.to_bits(), (-0f32).to_bits());
        let g = backward_output_score2029(&s, 1.).unwrap();
        assert!(g.amplitudes.iter().any(|x| *x != 0.));
        assert_eq!(g.inner.state, [0.; 4]);
        assert_eq!(g.inner.gains, [0.; 2]);
    }
}
#[test]
fn explicit_zero_effect_kinds_and_bad_pairing() {
    for k in [
        PositionEffectKind2029::Translation,
        PositionEffectKind2029::Angular,
        PositionEffectKind2029::Other,
    ] {
        let s = score_output2029(
            &model([2., -3.]),
            [0.2, 0.1, 1., 1.],
            k,
            effect(PositionEffectKind2029::Other),
            -0.,
        )
        .unwrap();
        assert_eq!(s.final_score.to_bits(), (-0f32).to_bits());
        assert_eq!(
            backward_output_score2029(&s, 1.).unwrap().amplitudes,
            [0.; 2]
        );
    }
    assert!(
        score_output2029(
            &model([1.; 2]),
            [0.; 4],
            PositionEffectKind2029::Angular,
            effect(PositionEffectKind2029::Translation),
            0.
        )
        .is_err()
    );
}
#[test]
fn independent_history_cold_signed_and_atomic() {
    let mut inner = PositionHead2018::fresh([1.; 2], AdamWConfig::default()).unwrap();
    for _ in 0..3 {
        inner.apply_gradient([0.1, -0.2]).unwrap();
    }
    let old = serde_json::to_vec(&inner).unwrap();
    let mut h = PositionOutputHead2029::fresh(inner, [-2., 3.], AdamWConfig::default()).unwrap();
    assert_eq!(old, serde_json::to_vec(&h.inner).unwrap());
    assert_eq!((h.inner.optimizer_step, h.output.optimizer_step), (3, 0));
    h.update([0.1, 0.2], [0.3, -0.4]).unwrap();
    assert_eq!((h.inner.optimizer_step, h.output.optimizer_step), (4, 1));
    let bytes = h.cold().unwrap();
    assert_eq!(
        bytes,
        PositionOutputHead2029::restore(&bytes)
            .unwrap()
            .cold()
            .unwrap()
    );
    assert!(h.update([0.1, 0.2], [f32::NAN, 0.]).is_err());
    assert_eq!(bytes, h.cold().unwrap());
    h.output.update_ages[1] = u32::MAX;
    let before = h.cold().unwrap();
    assert!(h.update([0.1, 0.2], [0.3, 0.4]).is_err());
    assert_eq!(before, h.cold().unwrap());
    let oldbytes = old;
    assert!(PositionOutputHead2029::restore(&oldbytes).is_err());
}
#[test]
fn overflow_and_tape_tamper_reject() {
    let mut h = model([f32::MAX; 2]);
    h.inner.gains = [f32::MAX, 1.];
    assert!(
        score_output2029(
            &h,
            [1., 1., 1., 1.],
            PositionEffectKind2029::Translation,
            PositionEffect2018 {
                translation: [1., 0.],
                angular: [0.; 2]
            },
            f32::MAX
        )
        .is_err()
    );
    let mut s = score_output2029(
        &model([1.; 2]),
        [0.2, 0.1, 1., 1.],
        PositionEffectKind2029::Translation,
        effect(PositionEffectKind2029::Translation),
        0.,
    )
    .unwrap();
    s.component += 0.1;
    assert!(backward_output_score2029(&s, 1.).is_err());
    assert!(
        PositionOutputHead2029::fresh(h.inner, [f32::INFINITY, 0.], AdamWConfig::default())
            .is_err()
    );
}
#[test]
fn amplitudes_are_not_capped() {
    let h = model([8., -8.]);
    let s = score_output2029(
        &h,
        [1., 0., 1., 1.],
        PositionEffectKind2029::Translation,
        PositionEffect2018 {
            translation: [1., 0.],
            angular: [0.; 2],
        },
        0.,
    )
    .unwrap();
    assert!(s.component > 1.);
}
#[test]
fn saturated_unused_chain_does_not_overflow() {
    let mut h = model([f32::MAX, 1.]);
    h.inner.gains = [f32::MAX, 1.];
    let s = score_output2029(
        &h,
        [1., 1., 1., 1.],
        PositionEffectKind2029::Translation,
        PositionEffect2018 {
            translation: [1., 0.],
            angular: [0.; 2],
        },
        0.,
    )
    .unwrap();
    assert_eq!(s.hidden.component, 1.);
    let g = backward_output_score2029(&s, 2.).unwrap();
    assert_eq!(g.amplitudes, [2., 0.]);
    assert_eq!(g.inner.state, [0.; 4]);
    assert_eq!(g.inner.gains, [0.; 2]);
}
#[test]
fn zero_scale_and_saturated_huge_weights_have_finite_correct_adjoints() {
    let mut h = model([0., 1.]);
    h.inner.gains = [f32::MAX; 2];
    let e = PositionEffect2018 {
        translation: [1., 0.],
        angular: [0.; 2],
    };
    for amplitude in [0., 1.] {
        h.output.amplitudes[0] = amplitude;
        let t = h
            .forward_detached([1., 0., 1., 1.], PositionEffectKind2029::Translation, e, 0.)
            .unwrap();
        let g = h.backward_detached(&t, 1.).unwrap();
        assert_eq!(g.inner.state, [0.; 4]);
        assert_eq!(g.inner.gains, [0.; 2]);
        assert_eq!(g.amplitudes, [1., 0.]);
    }
    h.output.amplitudes[0] = 1.;
    let t = h
        .forward_detached([1., 0., 1., 0.], PositionEffectKind2029::Translation, e, 0.)
        .unwrap();
    let g = h.backward_detached(&t, 1.).unwrap();
    assert_eq!(g.inner.state[..3], [0.; 3]);
    assert!(g.inner.state[3].is_finite() && g.inner.state[3] > f32::MAX * 0.9);
    assert_eq!(g.inner.gains, [0.; 2]);
}
#[test]
fn immutable_tape_binds_both_weight_vectors() {
    let mut h = model([1.; 2]);
    let t = h
        .forward_detached(
            [0.2, 0.1, 1., 1.],
            PositionEffectKind2029::Translation,
            effect(PositionEffectKind2029::Translation),
            0.,
        )
        .unwrap();
    h.inner.gains[0] += 0.1;
    assert!(h.backward_detached(&t, 1.).is_err());
    let mut h = model([1.; 2]);
    h.output.amplitudes[1] += 0.1;
    assert!(h.backward_detached(&t, 1.).is_err());
}
#[test]
fn logical_work_and_empty_batch_are_explicit() {
    let w = PositionReadoutWork2018::derive(19).unwrap();
    assert_eq!(
        (
            w.candidate_reads,
            w.candidate_arithmetic,
            w.bound_and_merge_ops
        ),
        (133, 190, 38)
    );
    assert_eq!(
        PositionReadoutWork2018::derive(0)
            .unwrap()
            .source_arithmetic,
        9
    );
    assert!(PositionReadoutWork2018::derive(u64::MAX).is_err());
    let w = PositionOutputWork2029::for_kinds(&[
        PositionEffectKind2029::Translation,
        PositionEffectKind2029::Angular,
        PositionEffectKind2029::Other,
    ])
    .unwrap();
    assert_eq!(
        (
            w.kind_reads,
            w.amplitude_reads,
            w.output_multiplies,
            w.final_merges
        ),
        (3, 2, 2, 3)
    );
    assert!(PositionOutputWork2029::derive(1, 2).is_err());
}
#[test]
fn typed_frame_preserves_features_and_rejects_zero_effect_kind_relabel() {
    use alife_core::*;
    let mut features = [0.; 24];
    features[3] = -0.25;
    features[4] = 0.5;
    let c = ActionCandidate::new(
        0,
        ActionId(606),
        ActionKind::Look,
        CandidateActionFamily::Inspect,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector(features),
        Confidence::new(1.).unwrap(),
        NormalizedScalar::new(0.).unwrap(),
        DurationTicks(1),
        DurationTicks(1),
    )
    .unwrap();
    let mut d = [0.; 54];
    d[..24].copy_from_slice(&features);
    let t = crate::TrainingReplayTick {
        encoded_inputs: vec![0.; 2048],
        projection_gain: 0.5,
        local_threshold_shift: 0.,
        microstep_count: 0,
        enabled_routes: vec![],
        effective_weight_offsets: vec![],
        structural_synapses: vec![],
        candidates: vec![crate::TrainingReplayCandidate {
            family: CandidateActionFamily::Inspect,
            decoder_inputs: d,
            innate_bias: 0.,
        }],
    };
    let old = PositionReadoutFrame2018::bind_replay(
        &t,
        1,
        7,
        9,
        6,
        Some(2),
        vec![effect(PositionEffectKind2029::Other)],
    )
    .unwrap();
    let bytes = serde_json::to_vec(&t).unwrap();
    let f = PositionOutputFrame2029::bind(&t, old.clone(), &[c]).unwrap();
    assert_eq!(f.inherited, old);
    assert_eq!(bytes, serde_json::to_vec(&t).unwrap());
    assert_eq!(f.kinds, [PositionEffectKind2029::Angular]);
    let mut wrong = f;
    wrong.kinds[0] = PositionEffectKind2029::Other;
    assert!(wrong.validate_candidates(&t, &[c]).is_err());
}
#[test]
fn unit_and_zero_outputs_match_original_across_grid() {
    for x in [-1., -0., 0.1, 1.] {
        for z in [-1., 0., 0.3, 1.] {
            for confidence in [0., 0.2, 1.] {
                for gains in [[1., 1.], [-2., 3.], [0., 0.]] {
                    for k in [
                        PositionEffectKind2029::Translation,
                        PositionEffectKind2029::Angular,
                        PositionEffectKind2029::Other,
                    ] {
                        let inner = PositionHead2018::fresh(gains, AdamWConfig::default()).unwrap();
                        let old =
                            score2018(&inner, [x, z, 1., confidence], effect(k), -0.).unwrap();
                        let h =
                            PositionOutputHead2029::fresh(inner, [1.; 2], AdamWConfig::default())
                                .unwrap();
                        let s = score_output2029(&h, old.state, k, old.effect, old.before_score)
                            .unwrap();
                        assert_eq!(s.component.to_bits(), old.component.to_bits());
                        assert_eq!(s.final_score.to_bits(), old.final_score.to_bits());
                        let mut zero = h;
                        zero.output.amplitudes = [0.; 2];
                        assert_eq!(
                            score_output2029(&zero, old.state, k, old.effect, old.before_score)
                                .unwrap()
                                .final_score
                                .to_bits(),
                            (-0f32).to_bits()
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn shared_source_batch_matches_single_scores() {
    let h = model([-2., 3.]);
    let kinds = [
        PositionEffectKind2029::Translation,
        PositionEffectKind2029::Angular,
        PositionEffectKind2029::Other,
    ];
    let effects = kinds.map(effect);
    let bases = [-0., 3., -5.];
    let s = [0.2, -0.1, 1., 0.8];
    let batch = score_output_batch2029(&h, s, &kinds, &effects, &bases).unwrap();
    for i in 0..3 {
        assert_eq!(
            serde_json::to_vec(&batch[i]).unwrap(),
            serde_json::to_vec(&score_output2029(&h, s, kinds[i], effects[i], bases[i]).unwrap())
                .unwrap()
        );
    }
    assert!(score_output_batch2029(&h, s, &kinds, &effects, &[]).is_err());
    assert!(score_output_batch2029(&h, [f32::NAN; 4], &[], &[], &[]).is_err());
    assert!(
        score_output_batch2029(&h, s, &[], &[], &[])
            .unwrap()
            .is_empty()
    );
}
#[test]
fn finite_dot_but_overflowing_old_projection_uses_stable_fallback() {
    let mut h = model([1.; 2]);
    h.inner.gains = [f32::MAX / 8., 1.];
    let e = PositionEffect2018 {
        translation: [1., 0.],
        angular: [0.; 2],
    };
    let t = h
        .forward_detached(
            [1., 0., 1., 1e-38],
            PositionEffectKind2029::Translation,
            e,
            0.,
        )
        .unwrap();
    let g = h.backward_detached(&t, 1.).unwrap();
    assert!(g.inner.state.iter().all(|v| v.is_finite()));
    let raw = (4f64 / 17f64.sqrt()) * (f32::MAX as f64 / 8.) * (1e-38f32 as f64);
    let expected = (1. - raw.tanh().powi(2)) * (1e-38f32 as f64) * 4. * (f32::MAX as f64 / 8.)
        / 17f64.powf(1.5);
    assert!((g.inner.state[0] as f64 - expected).abs() < 1e-6);
    assert!(g.inner.state[3] > 1e37);
}
#[test]
fn overflowing_chain_with_zero_local_derivatives_is_not_a_weight_limit() {
    let mut h = model([f32::MAX, 1.]);
    h.inner.gains = [0.; 2];
    let e = PositionEffect2018 {
        translation: [1., 0.],
        angular: [0.; 2],
    };
    let t = h
        .forward_detached(
            [0., 0., 1., 1.],
            PositionEffectKind2029::Translation,
            e,
            -0.,
        )
        .unwrap();
    let g = h.backward_detached(&t, f32::MAX).unwrap();
    assert_eq!(g.inner.state, [0.; 4]);
    assert_eq!(g.inner.gains, [0.; 2]);
    assert_eq!(g.inner.translation, [0.; 2]);
    assert_eq!(g.amplitudes, [0.; 2]);
    assert_eq!(g.base, f32::MAX);
    assert_eq!(g.inner.before_score, 0.);
    let t = h
        .forward_detached(
            [0.1, 0., 1., 1.],
            PositionEffectKind2029::Translation,
            e,
            0.,
        )
        .unwrap();
    assert!(h.backward_detached(&t, f32::MAX).is_err());
}
#[test]
fn wide_chain_preserves_tiny_live_local_gradient() {
    let mut h = model([f32::MAX, 1.]);
    h.inner.gains = [0.; 2];
    let x = 2.5e-21f32;
    let confidence = 1e-30f32;
    let e = PositionEffect2018 {
        translation: [1., 0.],
        angular: [0.; 2],
    };
    let t = h
        .forward_detached(
            [x, 0., 1., confidence],
            PositionEffectKind2029::Translation,
            e,
            0.,
        )
        .unwrap();
    let g = h.backward_detached(&t, f32::MAX).unwrap();
    let expected = (f64::from(f32::MAX).powi(2) * f64::from(confidence) * 4. * f64::from(x)) as f32;
    assert!(expected > 1e26);
    assert_eq!(g.inner.gains[0].to_bits(), expected.to_bits());
    assert_eq!(g.inner.state, [0.; 4]);
    assert_eq!(g.amplitudes, [0.; 2]);
}

#[test]
fn finite_outer_chain_preserves_zero_and_tiny_projector_adjoints() {
    for (gain, occupancy) in [
        (0.0, 1.0),
        (0.25, 1.0),
        (0.1, 0.5),
        (1e-40, 1.0),
        (f32::MIN_POSITIVE, 1.0),
    ] {
        let h = PositionOutputHead2029::fresh(
            PositionHead2018::fresh([gain, 0.0], AdamWConfig::default()).unwrap(),
            [f32::MAX, 1.0],
            AdamWConfig::default(),
        )
        .unwrap();
        let tape = h
            .forward_detached(
                [0.0, 0.0, occupancy, 1.0],
                PositionEffectKind2029::Translation,
                PositionEffect2018 {
                    translation: [1.0, 0.0],
                    angular: [0.0; 2],
                },
                0.0,
            )
            .unwrap();
        let a = h.backward_detached(&tape, 1.0).unwrap();
        let expected = (f64::from(f32::MAX) * f64::from(occupancy) * 4.0 * f64::from(gain)) as f32;
        assert_eq!(a.inner.state[0].to_bits(), expected.to_bits());
        assert_eq!(a.inner.state[1..], [0.0; 3]);
        assert_eq!(a.inner.gains, [0.0; 2]);
        assert_eq!(a.amplitudes, [0.0; 2]);
        assert_eq!(a.base, 1.0);
    }
}
#[test]
fn genuinely_unrepresentable_position_adjoint_is_rejected() {
    let h = PositionOutputHead2029::fresh(
        PositionHead2018::fresh([1.0, 0.0], AdamWConfig::default()).unwrap(),
        [f32::MAX, 1.0],
        AdamWConfig::default(),
    )
    .unwrap();
    let t = h
        .forward_detached(
            [0.0, 0.0, 1.0, 1.0],
            PositionEffectKind2029::Translation,
            PositionEffect2018 {
                translation: [1.0, 0.0],
                angular: [0.0; 2],
            },
            0.0,
        )
        .unwrap();
    assert!(h.backward_detached(&t, 1.0).is_err());
}

#[test]
fn finite_outer_chain_zero_effect_retains_zero_and_tiny_live_effect_adjoints() {
    for gain in [2.0, -2.0, f32::MAX] {
        let h = PositionOutputHead2029::fresh(
            PositionHead2018::fresh([gain, 0.0], AdamWConfig::default()).unwrap(),
            [f32::MAX, 1.0],
            AdamWConfig::default(),
        )
        .unwrap();
        let t = h
            .forward_detached(
                [0.0, 0.0, 1.0, 1.0],
                PositionEffectKind2029::Translation,
                PositionEffect2018 {
                    translation: [0.0; 2],
                    angular: [0.0; 2],
                },
                0.0,
            )
            .unwrap();
        let a = h.backward_detached(&t, 1.0).unwrap();
        assert_eq!(a.inner.state, [0.0; 4]);
        assert_eq!(a.inner.translation, [0.0; 2]);
        assert_eq!(a.inner.angular, [0.0; 2]);
        assert_eq!(a.inner.gains, [0.0; 2]);
    }
    let h = PositionOutputHead2029::fresh(
        PositionHead2018::fresh([2.0, 0.0], AdamWConfig::default()).unwrap(),
        [f32::MAX, 1.0],
        AdamWConfig::default(),
    )
    .unwrap();
    let t = h
        .forward_detached(
            [2.5e-41, 0.0, 1.0, 1.0],
            PositionEffectKind2029::Translation,
            PositionEffect2018 {
                translation: [0.0; 2],
                angular: [0.0; 2],
            },
            0.0,
        )
        .unwrap();
    let a = h.backward_detached(&t, 1.0).unwrap();
    let u = 4.0f32 * 2.5e-41f32;
    assert_eq!(
        a.inner.translation[0],
        (f64::from(f32::MAX) * 2.0 * f64::from(u)) as f32
    );
    assert!(a.inner.translation[0] > 0.0);
    assert_eq!(a.inner.state, [0.0; 4]);
}

#[test]
fn huge_finite_chain_zero_inner_gain_keeps_live_gain_adjoint() {
    let h = PositionOutputHead2029::fresh(
        PositionHead2018::fresh([0.0; 2], AdamWConfig::default()).unwrap(),
        [f32::MAX, 1.0],
        AdamWConfig::default(),
    )
    .unwrap();
    let t = h
        .forward_detached(
            [0.2, 0.1, 1.0, 1.0],
            PositionEffectKind2029::Translation,
            PositionEffect2018 {
                translation: [1.0, 0.0],
                angular: [0.0; 2],
            },
            0.0,
        )
        .unwrap();
    let a = h.backward_detached(&t, 1.0).unwrap();
    let u = 0.8f32 / (1.0f32 + 0.8 * 0.8 + 0.4 * 0.4).sqrt();
    assert_eq!(a.inner.gains[0], f32::MAX * u);
    assert!(a.inner.gains[0] > 0.0);
    assert_eq!(a.inner.state, [0.0; 4]);
    assert_eq!(a.inner.translation, [0.0; 2]);
}

#[test]
fn ordinary_position_and_effect_derivatives_preserve_original_f32_bits() {
    for state in [
        [0.2, -0.1, 0.9, 0.8],
        [-0.3, 0.4, 0.7, 0.6],
        [0.0, 0.1, 1.0, 1.0],
    ] {
        for kind in [
            PositionEffectKind2029::Translation,
            PositionEffectKind2029::Angular,
        ] {
            for upstream in [0.7, -1.3] {
                let h = PositionHead2018::fresh([1.0, -0.7], AdamWConfig::default()).unwrap();
                let s = score2018(&h, state, effect(kind), 0.0).unwrap();
                let actual = backward2018(&s, upstream).unwrap();
                let v = [4.0 * state[0], 4.0 * state[1]];
                let d = (1.0 + (v[0] * v[0] + v[1] * v[1])).sqrt();
                let u = [v[0] / d, v[1] / d];
                let oc = state[2] * state[3];
                let b = upstream * (1.0 - s.component * s.component);
                let e = [
                    h.gains[0] * s.effect.translation[0] + h.gains[1] * s.effect.angular[0],
                    h.gains[0] * s.effect.translation[1] + h.gains[1] * s.effect.angular[1],
                ];
                let ve = v[0] * e[0] + v[1] * e[1];
                let d3 = d * d * d;
                for (i, &ui) in u.iter().enumerate() {
                    let projected = e[i] / d - v[i] * ve / d3;
                    assert_eq!(
                        actual.state[i].to_bits(),
                        (b * oc * 4.0 * projected).to_bits()
                    );
                    assert_eq!(
                        actual.translation[i].to_bits(),
                        (b * oc * h.gains[0] * ui).to_bits()
                    );
                    assert_eq!(
                        actual.angular[i].to_bits(),
                        (b * oc * h.gains[1] * ui).to_bits()
                    );
                }
            }
        }
    }
}

use super::*;
fn idle() -> PositionInput2013 {
    PositionInput2013 {
        boundary: false,
        capture: false,
        range01: 0.,
        bearing: [0., 0.],
        source_yaw_sc: [0., 1.],
        own_delta_world: [0., 0.],
        observation_confidence: 1.,
        confidence_decay: 1.,
    }
}
fn capture() -> PositionInput2013 {
    PositionInput2013 {
        boundary: true,
        capture: true,
        range01: 0.31,
        bearing: [0.6, 0.8],
        source_yaw_sc: [0.6, 0.8],
        own_delta_world: [0.03, -0.02],
        observation_confidence: 0.8,
        ..idle()
    }
}
fn model() -> PositionModel2013 {
    PositionModel2013::fresh(AdamWConfig::default()).unwrap()
}
fn loss(m: &PositionModel2013, s: PositionState2013, inputs: &[PositionInput2013]) -> f32 {
    let t = forward2013(m, s, inputs).unwrap();
    let s = t.states.last().unwrap();
    s.x * 0.7 - s.z * 0.4 + s.occupancy * 0.2 + s.confidence * 0.3
}
#[test]
fn parameter_input_and_state_adjoint_checks() {
    let m = model();
    let start = PositionState2013 {
        x: 0.05,
        z: -0.04,
        occupancy: 0.8,
        confidence: 0.7,
    };
    let inputs = [
        capture(),
        PositionInput2013 {
            boundary: true,
            own_delta_world: [0.02, -0.03],
            confidence_decay: 0.97,
            ..idle()
        },
        idle(),
    ];
    let t = forward2013(&m, start, &inputs).unwrap();
    let g = backward2013(&m, &t, [0.7, -0.4, 0.2, 0.3]).unwrap();
    for (j, expected) in g.parameters.iter().enumerate() {
        let mut hi = m.clone();
        let mut lo = m.clone();
        hi.weights[j] += 0.001;
        lo.weights[j] -= 0.001;
        let fd = (loss(&hi, start, &inputs) - loss(&lo, start, &inputs)) / 0.002;
        assert!((fd - expected).abs() < 0.0001, "weight{j} {fd} {expected}");
    }
    // Initial state through no-capture transitions, including occupancy -> displacement coupling.
    let inputs2 = [inputs[1].clone(), idle(), inputs[1].clone()];
    let t = forward2013(&m, start, &inputs2).unwrap();
    let g2 = backward2013(&m, &t, [0.7, -0.4, 0.2, 0.3]).unwrap();
    for (j, expected) in g2.initial_state.iter().enumerate() {
        let mut hi = start;
        let mut lo = start;
        let change = |s: &mut PositionState2013, d| match j {
            0 => s.x += d,
            1 => s.z += d,
            2 => s.occupancy += d,
            _ => s.confidence += d,
        };
        change(&mut hi, 0.001);
        change(&mut lo, -0.001);
        let fd = (loss(&m, hi, &inputs2) - loss(&m, lo, &inputs2)) / 0.002;
        assert!((fd - expected).abs() < 0.0001, "state{j}");
    }
    for row in 0..2 {
        for j in 0..9 {
            let mut hi = inputs.clone();
            let mut lo = inputs.clone();
            let change = |i: &mut PositionInput2013, d| {
                let x = match j {
                    0 => &mut i.range01,
                    1 => &mut i.bearing[0],
                    2 => &mut i.bearing[1],
                    3 => &mut i.source_yaw_sc[0],
                    4 => &mut i.source_yaw_sc[1],
                    5 => &mut i.own_delta_world[0],
                    6 => &mut i.own_delta_world[1],
                    7 => &mut i.observation_confidence,
                    _ => &mut i.confidence_decay,
                };
                *x += d;
            };
            // Keep finite-difference inputs inside unit intervals, including otherwise inactive fields.
            if (j == 0 && inputs[row].range01 == 0.)
                || (j == 4 && inputs[row].source_yaw_sc[1].abs() == 1.)
                || (j == 7 && inputs[row].observation_confidence == 1.)
                || (j == 8 && inputs[row].confidence_decay == 1.)
            {
                continue;
            }
            change(&mut hi[row], 0.001);
            change(&mut lo[row], -0.001);
            let fd = (loss(&m, start, &hi) - loss(&m, start, &lo)) / 0.002;
            let expected = if j < 7 {
                g.source_inputs[row][j]
            } else {
                g.confidence_inputs[row][j - 7]
            };
            assert!(
                (fd - expected).abs() < 0.0001,
                "input{row}:{j} {fd} {expected}"
            );
        }
    }
}
#[test]
fn zero_pulse_extreme_inactive_fields_are_exact_identity() {
    let mut m = model();
    m.weights = [f32::MAX; 6];
    let s = PositionState2013 {
        x: -0.,
        z: 1.,
        occupancy: 1.,
        confidence: -0.,
    };
    let i = PositionInput2013 {
        own_delta_world: [f32::MAX, -f32::MAX],
        bearing: [1., 1.],
        source_yaw_sc: [1., 1.],
        ..idle()
    };
    let t = forward2013(&m, s, &[i.clone(), i]).unwrap();
    let end = t.states.last().unwrap();
    assert_eq!(
        [end.x, end.z, end.occupancy, end.confidence].map(f32::to_bits),
        [s.x, s.z, s.occupancy, s.confidence].map(f32::to_bits)
    );
    let adj = [-0., 1., -0., 1.];
    let g = backward2013(&m, &t, adj).unwrap();
    assert_eq!(g.initial_state.map(f32::to_bits), adj.map(f32::to_bits));
    assert_eq!(g.parameters, [0.; 6]);
    assert_eq!(g.source_inputs, vec![[0.; 7]; 2]);
    assert_eq!(g.confidence_inputs, vec![[0.; 2]; 2]);
    assert_eq!(t.work.active_updates, 0);
    assert_eq!(t.work.logical_work().unwrap().scalar_arithmetic, 0);
}
#[test]
fn empty_sequence_and_model_tape_binding() {
    let m = model();
    let s = PositionState2013::default();
    let t = forward2013(&m, s, &[]).unwrap();
    assert_eq!(t.states, vec![s]);
    assert_eq!(t.work, PositionWork2013::default());
    assert_eq!(
        backward2013(&m, &t, [1.; 4]).unwrap().initial_state,
        [1.; 4]
    );
    let mut other = m.clone();
    other.weights[0] += 0.1;
    assert!(backward2013(&other, &t, [1.; 4]).is_err());
}
#[test]
fn malformed_inputs_and_failed_updates_are_transactional() {
    let mut m = model();
    let before = m.clone();
    assert!(m.apply_gradients(&[f32::NAN; 6]).is_err());
    assert_eq!(m, before);
    m.update_ages[3] = u32::MAX;
    let before = m.clone();
    assert!(m.apply_gradients(&[0.1; 6]).is_err());
    assert_eq!(m, before);
    m = model();
    m.optimizer_step = u32::MAX;
    let before = m.clone();
    assert!(m.apply_gradients(&[0.1; 6]).is_err());
    assert_eq!(m, before);
    let mut i = capture();
    i.own_delta_world[0] = f32::INFINITY;
    assert!(forward2013(&model(), PositionState2013::default(), &[i]).is_err());
    let mut i = capture();
    i.bearing[0] = 1.1;
    assert!(forward2013(&model(), PositionState2013::default(), &[i]).is_err());
    assert!(forward2013(
        &model(),
        PositionState2013 {
            x: 2.,
            ..Default::default()
        },
        &[]
    )
    .is_err());
}
#[test]
fn heterogeneous_adam_ages_and_cold_roundtrip() {
    let mut m = model();
    m.update_ages = [0, 1, 2, 3, 4, 5];
    let other = model();
    m.apply_gradients(&[0.2; 6]).unwrap();
    assert_eq!(m.update_ages, [1, 2, 3, 4, 5, 6]);
    assert_eq!(other.update_ages, [0; 6]);
    let bytes = serde_json::to_vec(&m).unwrap();
    let restored: PositionModel2013 = serde_json::from_slice(&bytes).unwrap();
    restored.validate().unwrap();
    assert_eq!(m, restored);
    assert_eq!(bytes, serde_json::to_vec(&restored).unwrap());
}
#[test]
fn confidence_fades_without_contracting_location() {
    let m = model();
    let t = forward2013(&m, Default::default(), &[capture()]).unwrap();
    let s = *t.states.last().unwrap();
    let inputs = vec![
        PositionInput2013 {
            boundary: true,
            confidence_decay: 0.95,
            ..idle()
        };
        96
    ];
    let t = forward2013(&m, s, &inputs).unwrap();
    let end = t.states.last().unwrap();
    assert_eq!([s.x, s.z], [end.x, end.z]);
    assert!((end.confidence - s.confidence * 0.95f32.powi(96)).abs() < 0.00001);
    assert_eq!(end.occupancy, 1.);
}
#[test]
fn logical_work_overflow_and_shape() {
    let w = PositionWork2013 {
        transitions: 6,
        active_updates: 4,
    }
    .logical_work()
    .unwrap();
    assert_eq!(w.dispatches, 6);
    assert_eq!(w.state_reads, 24);
    assert_eq!(w.input_parameter_reads, 68);
    assert_eq!(w.scalar_arithmetic, 172);
    assert_eq!(w.neuron_updates, 16);
    assert!(PositionWork2013 {
        transitions: 1,
        active_updates: 2
    }
    .logical_work()
    .is_err());
    assert!(PositionWork2013 {
        transitions: u64::MAX,
        active_updates: 0
    }
    .logical_work()
    .is_err());
}
#[test]
fn transition_work_overflow_does_not_commit() {
    let m = kernel::Model::default();
    let mut state = kernel::State::default();
    let mut work = kernel::Work {
        transitions: u64::MAX,
        ..Default::default()
    };
    let before = state;
    assert!(kernel::step(&m, &mut state, kernel::Input::default(), &mut work).is_err());
    assert_eq!(state, before);
    assert_eq!(work.transitions, u64::MAX);
}
#[test]
fn ordinary_transition_and_adjoint_match_frozen_reference() {
    for mirror in [-1., 1.] {
        for cadence in [1, 3, 4, 7] {
            let m = model();
            let mut refstate = reference::State::default();
            let refmodel = reference::Model { weights: m.weights };
            let mut refwork = reference::Work::default();
            let mut reftapes = vec![];
            let mut inputs = vec![];
            let mut states = vec![PositionState2013::default()];
            for k in 0..96 {
                let mut i = if k == 0 || k == 47 {
                    capture()
                } else {
                    PositionInput2013 {
                        boundary: true,
                        own_delta_world: [0.007, -0.009 * mirror],
                        confidence_decay: 0.97,
                        ..idle()
                    }
                };
                i.bearing[0] *= mirror;
                for microstep in 0..cadence {
                    let i = if microstep == 0 { i.clone() } else { idle() };
                    let x = i.kernel();
                    let ri = reference::Input {
                        pulse: x.pulse,
                        capture: x.capture,
                        range: x.range,
                        left: x.left,
                        forward: x.forward,
                        sin: x.sin,
                        cos: x.cos,
                        dx: x.dx,
                        dz: x.dz,
                        confidence: x.confidence,
                        fade: x.fade,
                    };
                    reftapes
                        .push(reference::step(&refmodel, &mut refstate, ri, &mut refwork).unwrap());
                    states.push(PositionState2013 {
                        x: refstate.h[0],
                        z: refstate.h[1],
                        occupancy: refstate.occupied,
                        confidence: refstate.confidence,
                    });
                    inputs.push(i);
                }
            }
            let t = forward2013(&m, Default::default(), &inputs).unwrap();
            assert_eq!(t.states.len(), states.len());
            for (a, b) in t.states.iter().zip(states) {
                assert_eq!(
                    [a.x, a.z, a.occupancy, a.confidence].map(f32::to_bits),
                    [b.x, b.z, b.occupancy, b.confidence].map(f32::to_bits)
                );
            }
            assert_eq!(t.work.transitions, refwork.transitions);
            assert_eq!(t.work.active_updates * 6, refwork.bilinear_terms);
            let g = backward2013(&m, &t, [0.7, -0.4, 0.2, 0.3]).unwrap();
            let (rp, _) = reference::backward(&reftapes, [0.7, -0.4]);
            let (rs, rc) =
                reference::full_state_backward(&refmodel, &reftapes, [0.7, -0.4, 0.2, 0.3]);
            assert_eq!(g.parameters, rp);
            assert_eq!(g.initial_state, rs);
            assert_eq!(
                g.source_inputs,
                reference::input_backward(&refmodel, &reftapes, [0.7, -0.4])
            );
            assert_eq!(g.confidence_inputs, rc);
        }
    }
}
#[test]
fn idle_does_not_consume_a_measurement_or_advance_reference() {
    let m = model();
    let s = *forward2013(&m, Default::default(), &[capture()])
        .unwrap()
        .states
        .last()
        .unwrap();
    let measured = PositionInput2013 {
        own_delta_world: [0.1, -0.2],
        ..idle()
    };
    let idle_tape = forward2013(&m, s, &[measured.clone(), measured.clone()]).unwrap();
    assert_eq!(*idle_tape.states.last().unwrap(), s);
    let admitted = PositionInput2013 {
        boundary: true,
        ..measured
    };
    let resumed = forward2013(&m, *idle_tape.states.last().unwrap(), &[admitted.clone()]).unwrap();
    let direct = forward2013(&m, s, &[admitted]).unwrap();
    assert_eq!(resumed.states, direct.states);
    // A stationary interval has zero measured displacement, irrespective of stale action velocity.
    let stationary = forward2013(
        &m,
        s,
        &[PositionInput2013 {
            boundary: true,
            ..idle()
        }],
    )
    .unwrap();
    assert_eq!(*stationary.states.last().unwrap(), s);
}
#[test]
fn inactive_capture_inputs_have_zero_adjoint_without_overflow() {
    let mut m = model();
    m.weights[0] = f32::MAX;
    m.weights[1] = f32::MAX;
    let i = PositionInput2013 {
        boundary: true,
        bearing: [1., 1.],
        source_yaw_sc: [1., 1.],
        ..idle()
    };
    let start = PositionState2013 {
        occupancy: 1.,
        ..Default::default()
    };
    let tape = forward2013(&m, start, &[i.clone()]).unwrap();
    let g = backward2013(&m, &tape, [1., 0., 0., 0.]).unwrap();
    assert_eq!(g.source_inputs[0][..5], [0.; 5]);
    assert_eq!(g.initial_state, [1., 0., 0., 0.]);
    assert_eq!(g.source_inputs[0][5], -1. / 32.);
    let active_capture = PositionInput2013 { capture: true, ..i };
    let tape = forward2013(&m, start, &[active_capture]).unwrap();
    assert!(backward2013(&m, &tape, [1., 0., 0., 0.]).is_err());
}

#[test]
fn saturated_channels_skip_unused_overflowing_range_sums() {
    let mut m = model();
    m.weights[..4].fill(f32::MAX);
    let i = PositionInput2013 {
        boundary: true,
        capture: true,
        range01: 0.1,
        bearing: [1., 1.],
        source_yaw_sc: [1., 1.],
        ..idle()
    };
    let t = forward2013(&m, Default::default(), &[i]).unwrap();
    let end = t.states.last().unwrap();
    assert_eq!([end.x, end.z], [1., 1.]);
    let g = backward2013(&m, &t, [1., 1., 0., 0.]).unwrap();
    assert_eq!(g.parameters, [0.; 6]);
    assert_eq!(g.initial_state, [0.; 4]);
    assert_eq!(g.source_inputs, vec![[0.; 7]]);
}

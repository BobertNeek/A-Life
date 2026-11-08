use super::*;
fn run(control: Control, drives: [f32; 11], steps: usize) -> Kernel {
    let mut k = Kernel::new(control);
    for _ in 0..steps {
        k.step(drives, Modulation::default()).unwrap();
    }
    k
}
fn feature(lane: usize) -> [f32; 54] {
    let mut x = [0.; 54];
    x[lane] = 1.;
    x
}
#[test]
fn exact_recipe_shape_and_motor_only_readouts() {
    assert_eq!(plan::BRANCHES.len(), 70);
    assert_eq!(
        plan::BRANCHES.iter().map(|b| b.inputs.len()).sum::<usize>(),
        184
    );
    assert_eq!(plan::READOUTS.len(), 50);
    assert!(plan::READOUTS.iter().all(|r| (52..92).contains(&r.source)));
    assert!(plan::BRANCHES
        .iter()
        .all(|b| b.target < 92 && b.inputs.iter().all(|x| x.0 < 92)));
    assert_eq!(RECIPE_SHA256.len(), 64);
}
#[test]
fn matched_pressure_has_correct_sign_across_shared_operating_points() {
    for background in [-0.9, 0., 0.9] {
        for gain in [0., 0.1, 0.5, 1., 2.] {
            for threshold in [-0.75, 0., 0.25, 0.9] {
                let m = Modulation {
                    projection_gain: gain,
                    local_threshold: threshold,
                    ..Modulation::default()
                };
                for d in 0..11 {
                    let mut drives = SETPOINTS;
                    drives[d] = (SETPOINTS[d] + 0.2).min(1.);
                    let mut k = Kernel::with_common_activation(Control::Full, background).unwrap();
                    for _ in 0..24 {
                        k.step(drives, m).unwrap();
                        for i in 0..11 {
                            assert!(
                                k.pressure(i).unwrap() >= -2e-7,
                                "drive{i} gain{gain} threshold{threshold}"
                            );
                        }
                    }
                    if gain > 0. {
                        assert!(k.pressure(d).unwrap() > 1e-5);
                    }
                }
            }
        }
    }
}
#[test]
fn setpoints_are_behaviorally_quiet_but_metabolism_is_not_free() {
    let mut k = run(Control::Full, SETPOINTS, 32);
    assert!(k.activations.iter().any(|v| v.abs() > 0.));
    assert!(k.metabolic.iter().any(|v| *v > 0.));
    for i in 0..11 {
        assert_eq!(k.pressure(i).unwrap(), 0.);
    }
    assert_eq!(k.controller(), [0.; 3]);
    for f in 0..8 {
        for lane in [23, 36, 37, 38, 39, 40, 41, 42, 43] {
            assert_eq!(k.candidate_delta(f, &feature(lane)).unwrap(), 0.);
        }
    }
}
#[test]
fn actual_motor_stage_adds_latency_and_lesion_removes_all_new_emission() {
    let mut d = SETPOINTS;
    d[0] = 0.85;
    let mut k = Kernel::new(Control::Full);
    for _ in 0..2 {
        k.step(d, Modulation::default()).unwrap();
        assert_eq!(k.candidate_delta(6, &feature(37)).unwrap(), 0.);
    }
    k.step(d, Modulation::default()).unwrap();
    assert!(k.candidate_delta(6, &feature(37)).unwrap() > 0.);
    let mut cut = run(Control::MotorMediationCut, d, 32);
    assert!(cut.pressure(0).unwrap() > 0.);
    assert!(cut.controller()[1] > 0.);
    for f in 0..8 {
        assert_eq!(cut.candidate_delta(f, &feature(36)).unwrap(), 0.);
    }
    assert_eq!(cut.candidate_delta(6, &feature(37)).unwrap(), 0.);
}
#[test]
fn hunger_prefers_positive_chemistry_without_claiming_nutrition() {
    let mut d = SETPOINTS;
    d[0] = 0.9;
    let mut k = run(Control::Full, d, 64);
    assert!(k.candidate_delta(6, &feature(37)).unwrap() > 0.);
    assert!(k.candidate_delta(6, &feature(38)).unwrap() < 0.);
    assert_eq!(k.candidate_delta(6, &feature(36)).unwrap(), 0.);
}
#[test]
fn fatigue_and_atp_deficit_favor_stationary_recovery_without_suppressing_eating() {
    let mut d = SETPOINTS;
    d[1] = 0.9;
    d[6] = 0.1;
    let mut k = run(Control::Full, d, 64);
    assert!(k.low_atp() > 0.);
    assert!(k.candidate_delta(1, &feature(23)).unwrap() > 0.);
    assert!(k.candidate_delta(3, &feature(36)).unwrap() < 0.);
    assert_eq!(k.candidate_delta(6, &feature(37)).unwrap(), 0.);
    assert_eq!(k.candidate_delta(3, &[0.; 54]).unwrap(), 0.);
    d[6] = 0.9;
    let high = run(Control::Full, d, 64);
    assert_eq!(high.low_atp(), 0.);
}
#[test]
fn unbound_drive_has_no_emission_and_peer_is_not_contact_permission() {
    let mut d = SETPOINTS;
    d[10] = 1.;
    let mut k = run(Control::Full, d, 64);
    assert!(k.pressure(10).unwrap() > 0.);
    for f in 0..8 {
        for lane in [23, 36, 37, 38, 39, 40, 41, 42, 43] {
            assert_eq!(k.candidate_delta(f, &feature(lane)).unwrap(), 0.);
        }
    }
    d = SETPOINTS;
    d[4] = 0.9;
    let mut k = run(Control::Full, d, 64);
    assert!(k.candidate_delta(3, &feature(41)).unwrap() > 0.);
    assert!(k.candidate_delta(7, &feature(42)).unwrap() > 0.);
    assert_eq!(k.candidate_delta(5, &feature(41)).unwrap(), 0.);
}
#[test]
fn washout_and_same_work_sham_do_not_use_hidden_settling() {
    let mut d = SETPOINTS;
    d[1] = 0.9;
    let mut full = run(Control::Full, d, 64);
    let mut sham = run(Control::EqualWorkSham, d, 64);
    let on = full.candidate_delta(1, &feature(23)).unwrap();
    assert!(on > 0.);
    assert_eq!(sham.candidate_delta(1, &feature(23)).unwrap(), 0.);
    assert_eq!(full.work(), sham.work());
    for _ in 0..128 {
        full.step(SETPOINTS, Modulation::default()).unwrap();
    }
    assert!(full.candidate_delta(1, &feature(23)).unwrap().abs() < 1e-6);
    assert_eq!(full.work().microsteps, 192);
}
#[test]
fn mismatched_reference_negative_control_exposes_spurious_pressure() {
    let mut d = SETPOINTS;
    d[6] = 0.6;
    let full = run(Control::Full, d, 64);
    let wrong = run(Control::ReferenceMismatch, d, 64);
    assert_eq!(full.low_atp(), 0.);
    assert!(wrong.low_atp() > 0.);
}
#[test]
fn invalid_input_is_atomic_and_clone_continuation_exact() {
    let mut d = SETPOINTS;
    d[0] = 0.9;
    let mut k = run(Control::Full, d, 9);
    let before = k.clone();
    d[5] = f32::NAN;
    assert_eq!(k.step(d, Modulation::default()), Err("invalid_input"));
    assert_eq!(k, before);
    d[5] = SETPOINTS[5];
    let mut cold = before;
    k.step(d, Modulation::default()).unwrap();
    cold.step(d, Modulation::default()).unwrap();
    assert_eq!(k, cold);
}

#[test]
fn recovery_mediation_cannot_respond_before_fourth_microstep() {
    let mut d = SETPOINTS;
    d[1] = 0.9;
    let mut k = Kernel::new(Control::Full);
    for _ in 0..3 {
        k.step(d, Modulation::default()).unwrap();
        assert_eq!(k.candidate_delta(1, &feature(23)).unwrap(), 0.);
    }
    k.step(d, Modulation::default()).unwrap();
    assert!(k.candidate_delta(1, &feature(23)).unwrap() > 0.);
}

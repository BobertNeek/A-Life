use super::*;
use crate::{TrainingReplayCandidate, TrainingReplayTick};
use alife_core::*;
fn tick() -> TrainingReplayTick {
    TrainingReplayTick {
        encoded_inputs: vec![0.; 2048],
        projection_gain: 0.5,
        local_threshold_shift: 0.,
        microstep_count: 3,
        enabled_routes: vec![],
        effective_weight_offsets: vec![],
        structural_synapses: vec![],
        candidates: vec![TrainingReplayCandidate {
            family: CandidateActionFamily::Inspect,
            decoder_inputs: [0.; 54],
            innate_bias: 0.,
        }],
    }
}
fn retained(t: &TrainingReplayTick) -> RetainedFrame1992 {
    RetainedFrame1992::bind_replay(
        t,
        2,
        2,
        Some(RetainedSource1992 {
            source_sequence: 0,
            source_tick: 0,
            delivered_sequence: 1,
            delivered_tick: 1,
            original_frame_digest: [1; 4],
            original_tuple_digest: [2; 4],
            confidence: 0.75,
        }),
        vec![[0.2, -0.4]],
    )
    .unwrap()
}
#[test]
fn zeros_preserve_negative_zero_but_remain_trainable() {
    let t = tick();
    let f = CurrentSelfFrame2000::bind_replay(&t, vec![true]).unwrap();
    let mut h = CurrentSelfHead2000::fresh_zero(AdamWConfig::default()).unwrap();
    let mut a = vec![0.; 2048];
    a[1791] = 0.7;
    assert_eq!(
        h.score_detached(&f, &t, 0, &a, -0.)
            .unwrap()
            .final_score
            .to_bits(),
        (-0.0f32).to_bits()
    );
    assert_eq!(h.local_gradients(&f, &t, 0, &a, 1.).unwrap(), (0.7, 0.));
    h.apply_gradient(0.7).unwrap();
    assert_ne!(h.coefficient, 0.);
    assert_eq!(h.update_age, 1);
    let r = RetainedHead1992::fresh_zero(32., AdamWConfig::default()).unwrap();
    a[2019] = 0.3;
    let f = retained(&t);
    assert_eq!(
        r.score_detached(&f, &t, 0, &a, -0.)
            .unwrap()
            .final_score
            .to_bits(),
        (-0.0f32).to_bits()
    );
    assert!(r
        .local_gradients(&f, &t, 0, &a, 1.)
        .unwrap()
        .0
        .iter()
        .any(|x| *x != 0.));
}
#[test]
fn source_mask_and_intent_off_are_exact() {
    let t = tick();
    let mut a = vec![0.3; 2048];
    a[1791] = 0.9;
    let mut h = CurrentSelfHead2000::fresh_zero(AdamWConfig::default()).unwrap();
    h.coefficient = 192.;
    let f = CurrentSelfFrame2000::bind_replay(&t, vec![false]).unwrap();
    assert_eq!(h.score_detached(&f, &t, 0, &a, 2.).unwrap().final_score, 2.);
    assert_eq!(h.local_gradients(&f, &t, 0, &a, 1.).unwrap(), (0., 0.));
    let mut r = RetainedHead1992::fresh_zero(32., AdamWConfig::default()).unwrap();
    r.weights.fill(320.);
    let f = RetainedFrame1992::bind_replay(&t, 2, 2, None, vec![[0.2, -0.4]]).unwrap();
    assert_eq!(r.score_detached(&f, &t, 0, &a, 2.).unwrap().final_score, 2.);
    assert_eq!(
        r.local_gradients(&f, &t, 0, &a, 1.).unwrap(),
        (vec![0.; 48], [0.; 8])
    );
}
#[test]
fn gradients_match_finite_difference() {
    let t = tick();
    let f = retained(&t);
    let mut h = RetainedHead1992::fresh_zero(32., AdamWConfig::default()).unwrap();
    for (i, w) in h.weights.iter_mut().enumerate() {
        *w = (i as f32 - 24.) * 0.01;
    }
    let a = vec![0.3; 2048];
    let (g, s) = h.local_gradients(&f, &t, 0, &a, 1.).unwrap();
    for (i, expected) in g.iter().enumerate() {
        let mut p = h.clone();
        let mut m = h.clone();
        p.weights[i] += 0.01;
        m.weights[i] -= 0.01;
        let n = (p.score_detached(&f, &t, 0, &a, 0.).unwrap().final_score
            - m.score_detached(&f, &t, 0, &a, 0.).unwrap().final_score)
            / 0.02;
        assert!((n - expected).abs() < 0.00002);
    }
    for (i, expected) in s.iter().enumerate() {
        let mut p = a.clone();
        let mut m = a.clone();
        p[2019 + i] += 0.01;
        m[2019 + i] -= 0.01;
        let n = (h.score_detached(&f, &t, 0, &p, 0.).unwrap().final_score
            - h.score_detached(&f, &t, 0, &m, 0.).unwrap().final_score)
            / 0.02;
        assert!((n - expected).abs() < 0.00002);
    }
}
#[test]
fn invalid_inputs_fail_without_panics_or_mutation() {
    let t = tick();
    let f = retained(&t);
    let mut h = RetainedHead1992::fresh_zero(32., AdamWConfig::default()).unwrap();
    assert!(h.score_detached(&f, &t, 0, &[], 0.).is_err());
    assert!(h.score_detached(&f, &t, 1, &[0.; 2048], 0.).is_err());
    let before = h.clone();
    assert!(h.apply_gradients(&[f32::NAN; 48]).is_err());
    assert_eq!(before, h);
    h.optimizer_step = u32::MAX;
    let before = h.clone();
    assert!(h.apply_gradients(&[0.; 48]).is_err());
    assert_eq!(before, h);
    let mut stale = t.clone();
    stale.candidates[0].decoder_inputs[3] = 0.5;
    assert!(f.validate(&stale).is_err());
}
#[test]
fn checkpoint_roundtrip_preserves_head_moments_and_ages() {
    let mut h = RetainedHead1992::fresh_zero(32., AdamWConfig::default()).unwrap();
    h.apply_gradients(&[0.2; 48]).unwrap();
    let b = serde_json::to_vec(&h).unwrap();
    let r: RetainedHead1992 = serde_json::from_slice(&b).unwrap();
    r.validate().unwrap();
    assert_eq!(h, r);
    assert_eq!(b, serde_json::to_vec(&r).unwrap());
    let mut s = CurrentSelfHead2000::fresh_zero(AdamWConfig::default()).unwrap();
    s.apply_gradient(-0.3).unwrap();
    let b = serde_json::to_vec(&s).unwrap();
    let r: CurrentSelfHead2000 = serde_json::from_slice(&b).unwrap();
    r.validate().unwrap();
    assert_eq!(s, r);
}
#[test]
fn typed_hold_does_not_alias_observed_look_or_contact() {
    let c = |id, obs, target| {
        ActionCandidate::new(
            0,
            ActionId(id),
            ActionKind::Look,
            CandidateActionFamily::Inspect,
            obs,
            target,
            CandidateFeatureVector([0.; 24]),
            Confidence::new(1.).unwrap(),
            NormalizedScalar::new(0.).unwrap(),
            DurationTicks(1),
            DurationTicks(1),
        )
        .unwrap()
    };
    assert!(is_hold_intention2000(&c(
        606,
        CandidateObservationRef::None,
        ActionTarget::NONE
    )));
    assert!(!is_hold_intention2000(&c(
        601,
        CandidateObservationRef::None,
        ActionTarget::NONE
    )));
    assert!(!is_hold_intention2000(&c(
        606,
        CandidateObservationRef::ObjectSlot(0),
        ActionTarget::NONE
    )));
}
#[test]
fn logical_work_does_not_disappear_for_zero_heads() {
    let w = DecisionContextWorkV1::derive(19, true).unwrap();
    assert_eq!(w.retained_parameter_terms, 912);
    assert_eq!(w.current_parameter_terms, 19);
    let m = DecisionContextWorkV1::derive(19, false).unwrap();
    assert_eq!(m.retained_parameter_terms, 0);
    assert_eq!(m.retained_source_reads, 152);
    assert_eq!(m.current_parameter_terms, 19);
    assert!(DecisionContextWorkV1::derive(u64::MAX, true).is_err());
}
#[test]
fn bound_and_metadata_are_validated() {
    let t = tick();
    let mut f = retained(&t);
    f.source.as_mut().unwrap().confidence = f32::NAN;
    assert!(f.validate(&t).is_err());
    let mut f = retained(&t);
    f.source.as_mut().unwrap().source_sequence = 1;
    assert!(f.validate(&t).is_err());
    let mut h = RetainedHead1992::fresh_zero(32., AdamWConfig::default()).unwrap();
    h.weights.fill(320.);
    assert!(
        h.score_detached(&retained(&t), &t, 0, &[1.; 2048], 0.)
            .unwrap()
            .bounded_score
            .abs()
            <= 32.
    );
    h.second_moment[0] = -1.;
    assert!(h.validate().is_err());
}
#[test]
fn candidate_binding_preserves_all_current_features_and_rejects_mismatch() {
    let mut t = tick();
    let mut features = [0.; 24];
    features[3] = -0.25;
    features[4] = 0.5;
    features[18] = 1.;
    t.candidates[0].decoder_inputs[..24].copy_from_slice(&features);
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
    let before = serde_json::to_vec(&t).unwrap();
    let f = CurrentSelfFrame2000::bind_candidates(&t, &[c]).unwrap();
    assert!(f.hold_intentions[0]);
    assert_eq!(before, serde_json::to_vec(&t).unwrap());
    t.candidates[0].decoder_inputs[3] += 0.01;
    assert!(CurrentSelfFrame2000::bind_candidates(&t, &[c]).is_err());
    assert!(f.validate(&t).is_err());
}
#[test]
fn current_self_parameter_and_source_derivatives_match() {
    let t = tick();
    let f = CurrentSelfFrame2000::bind_replay(&t, vec![true]).unwrap();
    let mut h = CurrentSelfHead2000::fresh_zero(AdamWConfig::default()).unwrap();
    h.coefficient = 0.7;
    let a = vec![0.3; 2048];
    let g = h.local_gradients(&f, &t, 0, &a, 1.).unwrap();
    let mut p = h.clone();
    let mut m = h.clone();
    p.coefficient += 0.001;
    m.coefficient -= 0.001;
    let n = (p.score_detached(&f, &t, 0, &a, 0.).unwrap().final_score
        - m.score_detached(&f, &t, 0, &a, 0.).unwrap().final_score)
        / 0.002;
    assert!((n - g.0).abs() < 0.00002);
    let mut p = a.clone();
    let mut m = a;
    p[1791] += 0.001;
    m[1791] -= 0.001;
    let n = (h.score_detached(&f, &t, 0, &p, 0.).unwrap().final_score
        - h.score_detached(&f, &t, 0, &m, 0.).unwrap().final_score)
        / 0.002;
    assert!((n - g.1).abs() < 0.00002);
}

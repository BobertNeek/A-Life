use super::*;

fn build(
    base: &PerceptionFrameDraft,
    candidates: Vec<ActionCandidate>,
    version: u16,
) -> Result<PerceptionFrameDraft, ScaffoldContractError> {
    let genome = BrainGenome::scaffold(2176, alife_core::BrainCapacityClass::n2048().id());
    if version == 1 {
        return PerceptionFrameDraft::new(
            base.organism_id(),
            base.tick(),
            base.sensor_profile(),
            base.sensory().clone(),
            base.body(),
            *base.homeostasis(),
            candidates,
            base.profile_provenance(),
            base.grounded_object_slots().to_vec(),
        );
    }
    let architecture = if version == 2 {
        genome
            .cognitive_architecture()
            .with_independent_orientation_v2(2)?
    } else {
        genome
            .cognitive_architecture()
            .with_grounded_orientation_v3(2)?
    };
    PerceptionFrameDraft::new_with_grounded_orientation_v3(
        base.organism_id(),
        base.tick(),
        base.sensor_profile(),
        base.sensory().clone(),
        base.body(),
        *base.homeostasis(),
        candidates,
        base.profile_provenance(),
        base.grounded_object_slots().to_vec(),
        &architecture,
    )
}

#[test]
fn orientation_repertoire_v3_admits_only_exact_grounded_patterns() {
    let base = cyan_amber_family_draft();
    let patterns = [
        (
            ActionKind::Move,
            CandidateActionFamily::Approach,
            0.,
            1.,
            19,
        ),
        (ActionKind::Look, CandidateActionFamily::Inspect, 1., 0., 19),
        (
            ActionKind::Look,
            CandidateActionFamily::Inspect,
            -1.,
            0.,
            19,
        ),
        (ActionKind::Look, CandidateActionFamily::Inspect, 1., 0., 18),
        (
            ActionKind::Look,
            CandidateActionFamily::Inspect,
            -1.,
            0.,
            18,
        ),
        (ActionKind::Look, CandidateActionFamily::Inspect, 0., 1., 18),
        (ActionKind::Look, CandidateActionFamily::Inspect, 0., 0., 18),
    ];
    for (kind, family, x, y, marker) in patterns {
        let mut features = [0.; alife_core::CANDIDATE_FEATURE_COUNT];
        features[0] = x;
        features[1] = y;
        features[marker] = 1.;
        let candidate = |features| {
            ActionCandidate::new(
                0,
                ActionId(2176),
                kind,
                family,
                CandidateObservationRef::None,
                ActionTarget::NONE,
                alife_core::CandidateFeatureVector(features),
                Confidence::new(1.).unwrap(),
                NormalizedScalar::new(0.2).unwrap(),
                DurationTicks::new(1),
                DurationTicks::new(2),
            )
            .unwrap()
        };
        assert!(build(&base, vec![candidate(features)], 1).is_err());
        assert!(build(&base, vec![candidate(features)], 2).is_err());
        let admitted = build(&base, vec![candidate(features)], 3).unwrap();
        assert!(admitted.grounded_orientation_repertoire());
        admitted.validate().unwrap();
        features[4] = 0.5;
        assert!(build(&base, vec![candidate(features)], 3).is_err());
    }
}

#[test]
fn orientation_repertoire_proof_survives_rebuilds_and_is_digest_bound() {
    let base = cyan_amber_family_draft();
    let legacy_bytes = serde_json::to_vec(&base).unwrap();
    assert!(!serde_json::to_value(&base)
        .unwrap()
        .as_object()
        .unwrap()
        .contains_key("orientation_repertoire"));
    assert_eq!(build(&base, base.candidates().to_vec(), 1).unwrap(), base);
    let opted = build(&base, base.candidates().to_vec(), 3).unwrap();
    assert_ne!(opted.base_digest(), base.base_digest());
    let rebuilt = opted
        .clone()
        .with_candidates(opted.candidates().to_vec())
        .unwrap()
        .with_semantic_context(None)
        .unwrap()
        .with_remembered_novelty(opted.sensory().channels.novelty_signal.raw())
        .unwrap();
    assert_eq!(rebuilt, opted);
    let restored: PerceptionFrameDraft =
        serde_json::from_slice(&serde_json::to_vec(&opted).unwrap()).unwrap();
    assert_eq!(restored, opted);
    let frame = restored
        .finalize(alife_core::PerceptionContextBlock::empty())
        .unwrap();
    let restored: PerceptionFrame =
        serde_json::from_slice(&serde_json::to_vec(&frame).unwrap()).unwrap();
    assert!(restored.grounded_orientation_repertoire());
    assert_eq!(restored, frame);
    let mut tampered = serde_json::to_value(&opted).unwrap();
    tampered
        .as_object_mut()
        .unwrap()
        .remove("orientation_repertoire");
    assert!(serde_json::from_value::<PerceptionFrameDraft>(tampered).is_err());
    let mut tampered = serde_json::to_value(&base).unwrap();
    tampered["orientation_repertoire"] = "GroundedOrientationV3".into();
    assert!(serde_json::from_value::<PerceptionFrameDraft>(tampered).is_err());
    let restored: PerceptionFrameDraft = serde_json::from_slice(&legacy_bytes).unwrap();
    assert_eq!(legacy_bytes, serde_json::to_vec(&restored).unwrap());
}

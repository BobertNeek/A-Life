//! Frozen copy helpers from 5abafe19; independent of the production helpers.
use super::*;

pub(super) fn legacy_route_focal_candidates(
    draft: PerceptionFrameDraft,
    attention: &AttentionFrame,
) -> Result<PerceptionFrameDraft, ScaffoldContractError> {
    attention.validate_contract()?;
    let Some(alife_core::StableFocusIdentity::TrackedObject(focal_id)) =
        attention.focal_targets.first().copied()
    else {
        return Ok(draft);
    };
    let Some(focal_slot) = draft
        .grounded_object_slots()
        .iter()
        .position(|slot| slot.tracked_object_id == focal_id)
        .and_then(|index| u16::try_from(index).ok())
    else {
        return Ok(draft);
    };

    let mut candidates = draft.candidates().to_vec();
    candidates.sort_by_key(|candidate| {
        (
            !matches!(
                candidate.observation,
                CandidateObservationRef::ObjectSlot(slot) if slot == focal_slot
            ),
            candidate.candidate_index,
        )
    });
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.candidate_index =
            u16::try_from(index).map_err(|_| ScaffoldContractError::InvalidActionCandidate)?;
    }

    PerceptionFrameDraft::new(
        draft.organism_id(),
        draft.tick(),
        draft.sensor_profile(),
        draft.sensory().clone(),
        draft.body(),
        *draft.homeostasis(),
        candidates,
        draft.profile_provenance(),
        draft.grounded_object_slots().to_vec(),
    )
}

pub(super) fn legacy_cognitive_context_with_attention(
    mut context: CognitiveContextFrame,
    attention: AttentionFrame,
) -> Result<CognitiveContextFrame, ScaffoldContractError> {
    context.attention = attention.clone();
    context.peripheral.summaries = attention.peripheral_summaries.clone();
    context.focal.identities = attention.focal_targets.clone();
    context.focal.salience = attention.salience_components.clone();
    context.focal.hysteresis = attention.hysteresis;
    context.budget.peripheral_capacity = attention.budget_receipt.peripheral_capacity;
    context.budget.focal_capacity = attention.budget_receipt.focal_capacity;
    context.budget.work_used = attention.budget_receipt.work_units;
    context.budget.work_limit = attention.budget_receipt.work_units;
    context.validate_contract()?;
    Ok(context)
}

#[test]
fn focused_routing_and_all_fallbacks_preserve_complete_drafts() {
    let mut fixture = OwnerFixture::new(811, true);
    with_candidate_count(&mut fixture, 26);
    let draft = fixture
        .input
        .draft
        .clone()
        .with_remembered_novelty(0.37)
        .unwrap();
    let summaries = grounded_peripheral_summaries(draft.grounded_object_slots()).unwrap();
    let mut cases =
        vec![
            AttentionFrame::empty(draft.organism_id(), fixture.input.sequence_id, draft.tick())
                .unwrap(),
        ];
    for identity in [
        StableFocusIdentity::TrackedObject(draft.grounded_object_slots()[0].tracked_object_id),
        StableFocusIdentity::TrackedObject(draft.grounded_object_slots()[1].tracked_object_id),
        StableFocusIdentity::TrackedObject(TrackedObjectId(9_999_999)),
        StableFocusIdentity::Organism(OrganismId(999)),
    ] {
        let mut summary = summaries[0];
        summary.identity = identity;
        cases.push(
            select_focal_targets(
                draft.organism_id(),
                fixture.input.sequence_id,
                draft.tick(),
                &[summary],
                HysteresisState::default(),
                fixture.input.policy,
            )
            .unwrap(),
        );
    }
    for attention in cases {
        assert_eq!(
            route_focal_candidates(&draft, &attention),
            legacy_route_focal_candidates(draft.clone(), &attention)
        );
    }
    let mut malformed =
        AttentionFrame::empty(draft.organism_id(), fixture.input.sequence_id, draft.tick())
            .unwrap();
    malformed.schema_version = 0;
    assert!(route_focal_candidates(&draft, &malformed).is_err());
    assert_eq!(
        route_focal_candidates(&draft, &malformed),
        legacy_route_focal_candidates(draft.clone(), &malformed)
    );
}

#[test]
fn moved_attention_preserves_complete_context_and_rejections() {
    let fixture = OwnerFixture::new(811, true);
    let draft = &fixture.input.draft;
    let recall = fixture.memory.recall_frame(draft).unwrap();
    let context = cognitive_context_for_recall(
        draft.organism_id(),
        fixture.input.sequence_id,
        &recall,
        &fixture.topology,
    )
    .unwrap();
    let summaries = grounded_peripheral_summaries(draft.grounded_object_slots()).unwrap();
    let attention = select_focal_targets(
        draft.organism_id(),
        fixture.input.sequence_id,
        draft.tick(),
        &summaries,
        fixture.input.hysteresis,
        fixture.input.policy,
    )
    .unwrap();
    let mut wrong_owner = attention.clone();
    wrong_owner.organism_id = OrganismId(999);
    let mut malformed = attention.clone();
    malformed.schema_version = 0;
    for (index, attention) in [
        attention,
        AttentionFrame::empty(draft.organism_id(), fixture.input.sequence_id, draft.tick())
            .unwrap(),
        wrong_owner,
        malformed,
    ]
    .into_iter()
    .enumerate()
    {
        let actual = cognitive_context_with_attention(context.clone(), attention.clone());
        assert_eq!(actual.is_err(), index >= 2);
        assert_eq!(
            actual,
            legacy_cognitive_context_with_attention(context.clone(), attention)
        );
    }
}

#[test]
#[ignore = "CPU-only isolated copy comparison; run release explicitly"]
fn routing_attention_copy_cpu_timing_evidence() {
    for owners in [8, 16, 32, 50] {
        let mut fixtures: Vec<_> = (0..owners)
            .map(|index| OwnerFixture::new(1 + index as u64 * 7, true))
            .collect();
        for fixture in &mut fixtures {
            with_candidate_count(fixture, 26);
        }
        let inputs: Vec<_> = fixtures
            .iter()
            .map(|fixture| {
                let draft = &fixture.input.draft;
                let recall = fixture.memory.recall_frame(draft).unwrap();
                let context = cognitive_context_for_recall(
                    draft.organism_id(),
                    fixture.input.sequence_id,
                    &recall,
                    &fixture.topology,
                )
                .unwrap();
                let summaries =
                    grounded_peripheral_summaries(draft.grounded_object_slots()).unwrap();
                let attention = select_focal_targets(
                    draft.organism_id(),
                    fixture.input.sequence_id,
                    draft.tick(),
                    &summaries,
                    fixture.input.hysteresis,
                    fixture.input.policy,
                )
                .unwrap();
                (draft, context, attention)
            })
            .collect();
        let expected: Vec<_> = inputs
            .iter()
            .map(|(draft, context, attention)| {
                (
                    legacy_route_focal_candidates((*draft).clone(), attention).unwrap(),
                    legacy_cognitive_context_with_attention(context.clone(), attention.clone())
                        .unwrap(),
                )
            })
            .collect();
        let mut samples = [Vec::new(), Vec::new()];
        for sample in 0..9 {
            let order = if sample % 2 == 0 { [0, 1] } else { [1, 0] };
            for implementation in order {
                let mut elapsed = 0;
                for _ in 0..200 {
                    // The caller already owns context and attention; create those
                    // inputs outside the clock for both implementations.
                    let owned: Vec<_> = inputs
                        .iter()
                        .map(|(_, context, attention)| (context.clone(), attention.clone()))
                        .collect();
                    let started = Instant::now();
                    let actual: Vec<_> = inputs
                        .iter()
                        .zip(owned)
                        .map(|((draft, _, attention), (context, owned_attention))| {
                            if implementation == 0 {
                                (
                                    legacy_route_focal_candidates((*draft).clone(), attention)
                                        .unwrap(),
                                    legacy_cognitive_context_with_attention(
                                        context,
                                        owned_attention,
                                    )
                                    .unwrap(),
                                )
                            } else {
                                (
                                    route_focal_candidates(draft, attention).unwrap(),
                                    cognitive_context_with_attention(context, owned_attention)
                                        .unwrap(),
                                )
                            }
                        })
                        .collect();
                    elapsed += elapsed_ns(started);
                    assert_eq!(actual, expected);
                }
                samples[implementation].push(elapsed / 200);
            }
        }
        println!(
            "{}",
            serde_json::json!({"operation":"routing_attention_copy_helpers", "owners":owners, "candidates":26, "object_slots":2, "samples_ns":samples, "complete_typed_output_equal":true, "device_created":false})
        );
    }
}

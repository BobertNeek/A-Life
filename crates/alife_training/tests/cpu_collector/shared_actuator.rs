use super::*;

fn bytes(host: &CpuCanonicalHost, trainer: &CpuFoundationTrainer) -> Vec<u8> {
    serde_json::to_vec(&host.checkpoint(trainer).unwrap()).unwrap()
}

#[test]
fn shared_actuator_quiet_host_is_exact_ordinary_execution() {
    let (mut ordinary, trainer) = fixture();
    let mut candidate = ordinary.snapshot();
    let expected = ordinary.step(&trainer, 1.0).unwrap();
    let actual = candidate
        .step_with_shared_contact_priority(&trainer, 1.0)
        .unwrap();
    assert!(actual.interventions.is_empty());
    assert_eq!(expected.decisions.len(), actual.ordinary.decisions.len());
    assert!(
        bytes(&ordinary, &trainer) == bytes(&candidate, &trainer),
        "quiet state changed"
    );
    assert!(
        serde_json::to_vec(&expected.decisions).unwrap()
            == serde_json::to_vec(&actual.ordinary.decisions).unwrap(),
        "quiet experience changed"
    );
}

#[test]
fn shared_actuator_real_dispatch_consumed_without_false_experience_and_restores() {
    let (game, mut trainer) = new_game_fixture(None);
    // Explicit synthetic optimizer sentinels, not acquired biological learning.
    let mut sentinel = trainer.checkpoint();
    sentinel.optimizer_step = 7;
    sentinel.update_ages[0] = 7;
    sentinel.first_moment[0] = 0.125;
    sentinel.second_moment[0] = 0.25;
    trainer.restore_checkpoint(sentinel).unwrap();
    let founder_before = trainer.checkpoint();
    let (mut host, source) =
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, 0x1818, |world| {
            let (_, entity) = world.organism_entity_ids()[0];
            let position = world.entity(entity).unwrap().position;
            world.editor_spawn_object(WorldEditorSpawnSpec {
                label: "v18-acute-center".into(),
                kind: WorldObjectKind::Hazard,
                organism_id: None,
                position,
                nutrition: 0.0,
                hazard_pain: 0.4,
                radius: 0.02,
                token_id: None,
            })
        })
        .unwrap();
    let before = host.checkpoint(&trainer).unwrap();
    let step = host
        .step_with_shared_contact_priority(&trainer, 1.0)
        .unwrap();
    assert_eq!(step.interventions.len(), 1);
    assert!(step.ordinary.decisions.is_empty());
    let intervention = &step.interventions[0];
    let physical = &intervention.physical;
    let reflex = physical.withdrawal.as_ref().unwrap();
    assert_eq!(
        reflex.direction_basis,
        WithdrawalDirectionBasis::AcuteDamageHeadingFallback
    );
    assert!(physical.allocated_translation_time <= 1.0);
    assert!(
        intervention.cognitive_work.prediction_ops > 0,
        "executed forecast work disappeared"
    );
    assert_eq!(
        intervention.cognitive_work.learning_ops, 1,
        "staged eligibility work disappeared"
    );
    assert_eq!(
        intervention.selected.sequence_id,
        physical.voluntary.bundle.sequence_id
    );
    let after = host.checkpoint(&trainer).unwrap();
    let a = &after.residents[0];
    let b = &before.residents[0];
    assert!(a.neural.fast == b.neural.fast && a.neural.lifetime == b.neural.lifetime);
    assert!(a.neural.eligibility == b.neural.eligibility);
    assert_eq!(a.neural.weight_generation, b.neural.weight_generation);
    assert_eq!(a.next_sequence, b.next_sequence);
    assert!(a.neural.dispatch_generation > b.neural.dispatch_generation);
    assert!(a.memory == b.memory, "false family memory recorded");
    assert!(
        a.predictor == b.predictor,
        "reflex taught a voluntary motor prediction"
    );
    assert!(
        serde_json::to_vec(&a.language_grounding).unwrap()
            == serde_json::to_vec(&b.language_grounding).unwrap()
    );
    let aj = serde_json::to_value(&after).unwrap();
    let bj = serde_json::to_value(&before).unwrap();
    assert!(aj["residents"][0]["replay"] == bj["residents"][0]["replay"]);
    assert!(trainer.checkpoint() == founder_before);
    let saved = serde_json::to_vec(&after).unwrap();
    let mut restored =
        CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&saved).unwrap(), &trainer)
            .unwrap();
    assert!(bytes(&restored, &trainer) == saved);
    // No experience was issued by the consumed dispatch. A new genuine
    // dispatch may issue that sequence, including after ordinary-mode restore.
    host.world_mut().editor_remove_object(source).unwrap();
    restored.world_mut().editor_remove_object(source).unwrap();
    let next = host.step(&trainer, 1.0).unwrap();
    let twin = restored.step(&trainer, 1.0).unwrap();
    assert_eq!(next.decisions.len(), 1);
    assert_eq!(
        next.decisions[0].patch.header().sequence_id.raw(),
        a.next_sequence
    );
    assert!(
        next.decisions[0]
            .patch
            .decision()
            .neural_evidence()
            .unwrap()
            .dispatch_generation
            > intervention.dispatch_generation
    );
    assert!(
        serde_json::to_vec(&next.decisions).unwrap()
            == serde_json::to_vec(&twin.decisions).unwrap()
    );
    assert!(bytes(&host, &trainer) == bytes(&restored, &trainer));
    assert!(trainer.checkpoint() == founder_before);
}

#[test]
fn shared_actuator_repeated_contact_keeps_sequence_unissued_and_releases() {
    let (game, trainer) = new_game_fixture(None);
    let (mut host, source) =
        CpuCanonicalHost::new_prepared_cohort(game, &trainer, 0x1819, |world| {
            let (_, entity) = world.organism_entity_ids()[0];
            let mut position = world.entity(entity).unwrap().position;
            position.x += 0.1;
            world.editor_spawn_object(WorldEditorSpawnSpec {
                label: "v18-repeated-contact".into(),
                kind: WorldObjectKind::Hazard,
                organism_id: None,
                position,
                nutrition: 0.0,
                hazard_pain: 0.02,
                radius: 0.8,
                token_id: None,
            })
        })
        .unwrap();
    let before = host.checkpoint(&trainer).unwrap();
    let mut dispatch = before.residents[0].neural.dispatch_generation;
    for _ in 0..2 {
        let step = host
            .step_with_shared_contact_priority(&trainer, 1.0)
            .unwrap();
        assert!(step.ordinary.decisions.is_empty());
        assert_eq!(step.interventions.len(), 1);
        assert!(step.interventions[0].dispatch_generation > dispatch);
        dispatch = step.interventions[0].dispatch_generation;
        let current = host.checkpoint(&trainer).unwrap();
        assert_eq!(
            current.residents[0].next_sequence,
            before.residents[0].next_sequence
        );
        assert!(current.residents[0].neural.fast == before.residents[0].neural.fast);
    }
    host.world_mut().editor_remove_object(source).unwrap();
    let step = host
        .step_with_shared_contact_priority(&trainer, 1.0)
        .unwrap();
    assert!(
        step.interventions.is_empty(),
        "acute response remained latched"
    );
    assert_eq!(step.ordinary.decisions.len(), 1);
}

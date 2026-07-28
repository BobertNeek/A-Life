use alife_core::{
    ActionCommand, ActionKind, ActionTarget, Confidence, DurationTicks, FoundationId, Intensity,
    LanguageTokenId, OrganismId, SpeechActKind, SpeechMotorPayload, Tick, UtteranceId, Vec3f,
};
use alife_world::{
    AssistanceProvenance, FoundationProvenance, Habitat, HabitatActor, HabitatAuthority,
    HabitatAuthorityKind, HabitatId, HabitatMode, HabitatTransferProvenance,
    HabitatTransferRequest, HeadlessScenarioBuilder, PossessionProvenance, PresentationEvidence,
    QuarantineProvenance, SelectionExposureProvenance,
};

fn organism(raw: u64) -> OrganismId {
    OrganismId::new(raw).unwrap()
}

fn habitat(raw: u64) -> HabitatId {
    HabitatId::new(raw).unwrap()
}

fn token(raw: u16) -> LanguageTokenId {
    LanguageTokenId::new(raw).unwrap()
}

fn authority_for(organisms: &[OrganismId]) -> HabitatAuthority {
    let mut authority = HabitatAuthority::new(vec![
        Habitat::new(habitat(1), "Wild", HabitatMode::Wild).unwrap(),
        Habitat::new(habitat(2), "Reserve", HabitatMode::Reserve).unwrap(),
        Habitat::new(habitat(3), "Managed", HabitatMode::Managed).unwrap(),
        Habitat::new(habitat(4), "School", HabitatMode::School).unwrap(),
    ])
    .unwrap();
    for organism_id in organisms {
        authority
            .register_creature(*organism_id, habitat(1), Tick::ZERO)
            .unwrap();
    }
    authority
}

fn managed_transfer(organism_id: OrganismId, tick: Tick) -> HabitatTransferRequest {
    HabitatTransferRequest {
        organism_id,
        expected_prior_habitat_id: habitat(1),
        new_habitat_id: habitat(3),
        tick,
        provenance: HabitatTransferProvenance {
            actor: Some(HabitatActor::Player),
            authority: Some(HabitatAuthorityKind::ManagedController),
            quarantine: Some(QuarantineProvenance::NotRequired),
            assistance: Some(AssistanceProvenance::Unassisted),
            foundation: Some(FoundationProvenance::Known(FoundationId::N2048_V1)),
            possession: Some(PossessionProvenance::NotPossessed),
            selection_exposure: Some(SelectionExposureProvenance::Unexposed),
        },
    }
}

#[test]
fn projection_reports_only_grounded_speech_and_observed_relationship_evidence() {
    let speaker = organism(1);
    let neighbor = organism(2);
    let mut world = HeadlessScenarioBuilder::new(81_001)
        .agent("speaker", speaker, Vec3f::ZERO)
        .social_agent("neighbor", neighbor, Vec3f::new(2.0, 0.0, 0.0), 0.75)
        .build()
        .unwrap();
    world
        .replace_habitat_authority(authority_for(&[speaker, neighbor]))
        .unwrap();
    world
        .emit_creature_utterance(
            UtteranceId::new(7).unwrap(),
            speaker,
            None,
            SpeechMotorPayload::try_new(
                SpeechActKind::ExpressState,
                vec![token(89), token(105)],
                Confidence::new(0.9).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();

    let projection = world.habitat_presentation_projection().unwrap();
    assert_eq!(projection.tick, Tick::ZERO);
    assert_eq!(projection.creatures.len(), 2);
    let speaker_view = projection
        .creatures
        .iter()
        .find(|creature| creature.organism_id == speaker)
        .unwrap();
    assert!(speaker_view.stable_world_entity_id.is_some());
    assert_eq!(
        speaker_view.latest_grounded_utterance,
        PresentationEvidence::Observed {
            value: vec![token(89), token(105)],
            tick: Tick::ZERO,
        }
    );
    let edge = speaker_view
        .relationships
        .iter()
        .find(|edge| edge.target_organism_id == neighbor)
        .unwrap();
    match edge.affinity {
        PresentationEvidence::Observed { value, tick } => {
            assert_eq!(value.raw(), 0.75);
            assert_eq!(tick, Tick::ZERO);
        }
        PresentationEvidence::Unknown => panic!("world affinity evidence was discarded"),
    }
    assert_eq!(edge.trust, PresentationEvidence::Unknown);
    assert_eq!(edge.fear, PresentationEvidence::Unknown);

    let neighbor_view = projection
        .creatures
        .iter()
        .find(|creature| creature.organism_id == neighbor)
        .unwrap();
    assert_eq!(
        neighbor_view.latest_grounded_utterance,
        PresentationEvidence::Unknown
    );
}

#[test]
fn projection_is_read_only_and_habitat_operations_leave_authoritative_snapshots_unchanged() {
    let creature = organism(1);
    let baseline = HeadlessScenarioBuilder::new(81_002)
        .agent("creature", creature, Vec3f::ZERO)
        .food("berry", Vec3f::new(1.0, 0.0, 0.0), 0.8)
        .build()
        .unwrap();
    let mut operated = baseline.clone();
    let object_before = operated.stable_signature();
    let speech_before = operated.audible_utterances();
    let sensory_before = operated
        .sensory_report(creature, operated.tick())
        .unwrap()
        .core_snapshot;

    let mut authority = authority_for(&[creature]);
    authority
        .transfer(managed_transfer(creature, Tick::ZERO))
        .unwrap();
    operated.replace_habitat_authority(authority).unwrap();
    let projection_before = operated.habitat_presentation_projection().unwrap();
    let projection_after = operated.habitat_presentation_projection().unwrap();

    assert_eq!(projection_before, projection_after);
    assert_eq!(operated.stable_signature(), object_before);
    assert_eq!(operated.audible_utterances(), speech_before);
    assert_eq!(
        operated
            .sensory_report(creature, operated.tick())
            .unwrap()
            .core_snapshot,
        sensory_before
    );

    let command = ActionCommand::structured(
        creature,
        ActionKind::Idle.canonical_id(),
        ActionKind::Idle,
        ActionTarget::new(None, None),
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        Confidence::new(0.9).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap();
    let mut unchanged = baseline;
    let baseline_action = unchanged.apply_command(&command).unwrap();
    let operated_action = operated.apply_command(&command).unwrap();
    assert_eq!(operated_action, baseline_action);

    let mut saved =
        alife_world::PortableSaveFile::from_json_str(include_str!("fixtures/p34/tiny_save.json"))
            .unwrap();
    let neural_before = saved.creatures[0].gpu_brain.clone().unwrap();
    let mind_before = saved.creatures[0].mind.clone();
    let semantic_config_before = saved.config.semantic.clone();
    let mut saved_authority = authority_for(&[creature]);
    saved_authority
        .transfer(managed_transfer(creature, saved.world.tick))
        .unwrap();
    saved.world.habitats = saved_authority;

    assert_eq!(saved.creatures[0].gpu_brain.as_ref(), Some(&neural_before));
    assert_eq!(saved.creatures[0].mind, mind_before);
    assert_eq!(saved.config.semantic, semantic_config_before);
}

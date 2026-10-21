use alife_core::{
    ActionCommand, ActionKind, BrainClassSpec, BrainGenome, BrainScaleTier, ExperiencePatchHeader,
    ExperienceSequenceId, LobeKind, LobeLayout, OrganismId, ScaffoldContractError,
    SemanticPriorRequest, TeacherPerceptionChannel, Tick, WorldEntityId,
};
use alife_core::{Confidence, DurationTicks};

#[test]
fn founder_homologues_cannot_be_disabled() {
    assert_eq!(
        LobeLayout::with_disabled_lobe(
            BrainScaleTier::Nano512.neuron_count().unwrap(),
            LobeKind::SocialCommunication,
        ),
        Err(ScaffoldContractError::PhenotypeCompile)
    );
}

#[test]
fn genome_references_brain_class_without_runtime_weights() {
    let class = BrainClassSpec::for_tier(BrainScaleTier::Small1024);
    let genome = BrainGenome::scaffold(7, class.id);

    assert_eq!(genome.species_seed, 7);
    assert_eq!(genome.brain_class_id, class.id);
    assert!(genome.genetic_prior_seed != 0);
}

#[test]
fn experience_patch_and_action_command_use_versioned_structured_contracts() {
    let patch = ExperiencePatchHeader::new(
        OrganismId(11),
        ExperienceSequenceId(22),
        Tick(33),
        alife_core::SensorProfileProvenance::new(
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            alife_core::SensoryAbiVersion::CURRENT,
            Tick(33),
        )
        .unwrap(),
    )
    .unwrap();
    let action = ActionCommand::new(
        OrganismId(11),
        ActionKind::Interact,
        Some(WorldEntityId(44)),
        Confidence::new(0.75).unwrap(),
        DurationTicks::new(120),
    )
    .unwrap();

    assert_eq!(patch.abi_version, ExperiencePatchHeader::ABI_VERSION);
    assert_eq!(patch.organism_id.0, 11);
    assert_eq!(action.abi_version, ActionCommand::ABI_VERSION);
    assert_eq!(action.target_entity, Some(WorldEntityId(44)));
    assert!(action.confidence.raw() > 0.0);
}

#[test]
fn semantic_prior_is_private_and_teacher_uses_perceptual_channels() {
    let request = SemanticPriorRequest::new(OrganismId(11), ExperienceSequenceId(22)).unwrap();
    let channels = TeacherPerceptionChannel::ALL;

    assert!(request.private_to_organism);
    assert!(channels.contains(&TeacherPerceptionChannel::Hearing));
    assert!(channels.contains(&TeacherPerceptionChannel::Vision));
    assert!(channels.contains(&TeacherPerceptionChannel::Writing));
    assert!(channels.contains(&TeacherPerceptionChannel::Gesture));
    assert!(channels.contains(&TeacherPerceptionChannel::Object));
}

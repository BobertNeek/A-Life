#![cfg(feature = "gpu-tests")]

use alife_core::{
    BrainCapacityClass, CreatureGenome, Era1Ability, Era1Control, Era1EvidencePartition,
    FoundationGeneticIdentity, OrganismId, PolicyBackend,
};
use alife_training::{Era1LearningDisposition, Era1TrialRunRequest, Era1TrialRunner};
use alife_world::{Era1TrialManifest, Era1WorldFamily, ERA1_TRIAL_END_TICK};

const SOURCE_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const SOURCE_TREE: &str = "89abcdef0123456789abcdef0123456789abcdef";
const SUBJECT: OrganismId = OrganismId(301);
const FAMILIAR: OrganismId = OrganismId(302);
const NOVEL: OrganismId = OrganismId(303);

fn founder() -> CreatureGenome {
    let foundation = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    CreatureGenome::early_mammal_founder(0xE11_001, foundation).unwrap()
}

fn manifest(family: Era1WorldFamily) -> Era1TrialManifest {
    Era1TrialManifest::new(55_001, family, SUBJECT, FAMILIAR, NOVEL, 1, false, 41).unwrap()
}

fn request<'a>(
    genome: &'a CreatureGenome,
    manifest: &'a Era1TrialManifest,
    ability: Era1Ability,
    control: Era1Control,
    partition: Era1EvidencePartition,
) -> Era1TrialRunRequest<'a> {
    Era1TrialRunRequest::new(
        SUBJECT,
        0,
        genome,
        manifest,
        ability,
        control,
        partition,
        SOURCE_COMMIT,
        SOURCE_TREE,
    )
    .unwrap()
}

#[test]
fn causal_gpu_loop_binds_world_memory_pending_and_sealed_outcomes() {
    let genome = founder();
    let manifest = manifest(Era1WorldFamily::ForagingHazardMaze);
    let mut runner = Era1TrialRunner::new_required().unwrap();
    let evidence = runner
        .run(request(
            &genome,
            &manifest,
            Era1Ability::FlexibleForaging,
            Era1Control::Intact,
            Era1EvidencePartition::Acquisition,
        ))
        .unwrap();

    evidence.validate_contract().unwrap();
    assert_eq!(
        evidence.receipt.policy_backend,
        PolicyBackend::NeuralClosedLoopGpu
    );
    assert_eq!(evidence.gpu_dispatches, ERA1_TRIAL_END_TICK);
    assert_eq!(evidence.gpu_dispatches, evidence.sealed_outcomes);
    assert_eq!(evidence.memory_context_dispatches, evidence.gpu_dispatches);
    assert_eq!(evidence.learning_commits, evidence.gpu_dispatches);
    assert_eq!(evidence.memory_updates, evidence.gpu_dispatches);
    assert_eq!(evidence.eligibility_discards, 0);
    assert_eq!(evidence.sleep_commits, 0);
    assert!(!evidence.adapter_name.trim().is_empty());
    assert_eq!(evidence.backend_api, "vulkan");

    for step in &evidence.steps {
        assert_eq!(step.organism_id, SUBJECT);
        assert_eq!(step.phenotype_hash, evidence.receipt.phenotype_hash);
        assert_eq!(step.frame_digest, step.pending_frame_digest);
        assert_eq!(step.frame_digest, step.memory_context_final_digest);
        assert_ne!(step.memory_bank_digest, [0; 4]);
        assert_ne!(step.pending_receipt_digest, [0; 4]);
        assert_eq!(step.learning, Era1LearningDisposition::Applied);
        assert!(step.memory_observed);
    }

    let mut wrong_world = evidence.clone();
    wrong_world.receipt.world_digest[0] ^= 1;
    assert!(wrong_world.validate_contract().is_err());
    let mut wrong_pending = evidence.clone();
    wrong_pending.steps[0].pending_frame_digest.0[0] ^= 1;
    assert!(wrong_pending.validate_contract().is_err());
    let mut wrong_phenotype = evidence.clone();
    wrong_phenotype.steps[0].phenotype_hash.0[0] ^= 1;
    assert!(wrong_phenotype.validate_contract().is_err());
    let mut foreign_memory = evidence;
    foreign_memory.steps[0].memory_organism_id = OrganismId(999);
    assert!(foreign_memory.validate_contract().is_err());
}

#[test]
fn causal_controls_change_only_the_named_mechanism() {
    let genome = founder();
    let foraging = manifest(Era1WorldFamily::ForagingHazardMaze);
    let peer = manifest(Era1WorldFamily::PeerDemonstration);
    let retention = manifest(Era1WorldFamily::DelayedLocation);
    let mut runner = Era1TrialRunner::new_required().unwrap();

    let plasticity_disabled = runner
        .run(request(
            &genome,
            &foraging,
            Era1Ability::FlexibleForaging,
            Era1Control::PlasticityDisabled,
            Era1EvidencePartition::Acquisition,
        ))
        .unwrap();
    plasticity_disabled.validate_contract().unwrap();
    assert_eq!(plasticity_disabled.learning_commits, 0);
    assert_eq!(
        plasticity_disabled.eligibility_discards,
        plasticity_disabled.gpu_dispatches
    );
    assert_eq!(
        plasticity_disabled.memory_updates,
        plasticity_disabled.gpu_dispatches
    );
    assert!(plasticity_disabled
        .steps
        .iter()
        .all(|step| step.learning == Era1LearningDisposition::Discarded));

    let memory_disabled = runner
        .run(request(
            &genome,
            &foraging,
            Era1Ability::FlexibleForaging,
            Era1Control::MemoryDisabled,
            Era1EvidencePartition::Acquisition,
        ))
        .unwrap();
    memory_disabled.validate_contract().unwrap();
    assert_eq!(memory_disabled.memory_updates, 0);
    assert_eq!(
        memory_disabled.memory_context_dispatches,
        memory_disabled.gpu_dispatches
    );
    assert!(memory_disabled
        .steps
        .windows(2)
        .all(|pair| pair[0].memory_bank_digest == pair[1].memory_bank_digest));

    let social_disabled = runner
        .run(request(
            &genome,
            &peer,
            Era1Ability::Imitation,
            Era1Control::SocialDisabled,
            Era1EvidencePartition::SocialTransfer,
        ))
        .unwrap();
    social_disabled.validate_contract().unwrap();
    assert!(!social_disabled.social_context_present);
    assert!(social_disabled.steps.iter().all(|step| !step.peer_visible));

    let intact_sleep = runner
        .run(request(
            &genome,
            &retention,
            Era1Ability::PostSleepRetention,
            Era1Control::Intact,
            Era1EvidencePartition::PostSleepProbe,
        ))
        .unwrap();
    intact_sleep.validate_contract().unwrap();
    assert_eq!(intact_sleep.sleep_commits, 1);

    let sleep_disabled = runner
        .run(request(
            &genome,
            &retention,
            Era1Ability::PostSleepRetention,
            Era1Control::SleepDisabled,
            Era1EvidencePartition::PostSleepProbe,
        ))
        .unwrap();
    sleep_disabled.validate_contract().unwrap();
    assert_eq!(sleep_disabled.sleep_commits, 0);
}

#[test]
fn causal_request_rejects_mismatched_subject_generation_and_world_family() {
    let founder = founder();
    let manifest = manifest(Era1WorldFamily::ForagingHazardMaze);
    assert!(Era1TrialRunRequest::new(
        OrganismId(999),
        0,
        &founder,
        &manifest,
        Era1Ability::FlexibleForaging,
        Era1Control::Intact,
        Era1EvidencePartition::Acquisition,
        SOURCE_COMMIT,
        SOURCE_TREE,
    )
    .is_err());
    assert!(Era1TrialRunRequest::new(
        SUBJECT,
        1,
        &founder,
        &manifest,
        Era1Ability::FlexibleForaging,
        Era1Control::Intact,
        Era1EvidencePartition::Acquisition,
        SOURCE_COMMIT,
        SOURCE_TREE,
    )
    .is_err());
    assert!(Era1TrialRunRequest::new(
        SUBJECT,
        0,
        &founder,
        &manifest,
        Era1Ability::GroundedLanguage,
        Era1Control::Intact,
        Era1EvidencePartition::Acquisition,
        SOURCE_COMMIT,
        SOURCE_TREE,
    )
    .is_err());
}

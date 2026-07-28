use alife_core::{FoundationId, OrganismId, PolicyBackend, Tick};
use alife_world::{
    AssistanceProvenance, FoundationProvenance, Habitat, HabitatActor, HabitatAuthority,
    HabitatAuthorityError, HabitatAuthorityKind, HabitatAuthoritySnapshot, HabitatBreedingKind,
    HabitatBreedingRequest, HabitatId, HabitatMembership, HabitatMode, HabitatOperation,
    HabitatOperationRequest, HabitatTagRecord, HabitatTransferProvenance, HabitatTransferRecord,
    HabitatTransferRequest, PossessionProvenance, QuarantineProvenance,
    SelectionExposureProvenance,
};

fn organism(raw: u64) -> OrganismId {
    OrganismId::new(raw).unwrap()
}

fn habitat(raw: u64) -> HabitatId {
    HabitatId::new(raw).unwrap()
}

fn habitats() -> Vec<Habitat> {
    vec![
        Habitat::new(habitat(1), "Wild North", HabitatMode::Wild).unwrap(),
        Habitat::new(habitat(2), "Oak Reserve", HabitatMode::Reserve).unwrap(),
        Habitat::new(habitat(3), "Managed Meadow", HabitatMode::Managed).unwrap(),
        Habitat::new(habitat(4), "Nursery School", HabitatMode::School).unwrap(),
    ]
}

fn complete_provenance() -> HabitatTransferProvenance {
    HabitatTransferProvenance {
        actor: Some(HabitatActor::Player),
        authority: Some(HabitatAuthorityKind::ReserveKeeper),
        quarantine: Some(QuarantineProvenance::RequiredUntil(Tick::new(25))),
        assistance: Some(AssistanceProvenance::CaptureTransport),
        foundation: Some(FoundationProvenance::Known(FoundationId::N2048_V1)),
        possession: Some(PossessionProvenance::NotPossessed),
        selection_exposure: Some(SelectionExposureProvenance::Exposed {
            evaluation_id: "reserve-entry-v1".to_string(),
            exposure_tick: Tick::new(10),
        }),
    }
}

fn tag_for_reserve(authority: &mut HabitatAuthority, organism_id: OrganismId) {
    authority
        .tag_creature(habitat(2), organism_id, Tick::new(4), HabitatActor::Player)
        .unwrap();
}

#[test]
fn one_world_contains_all_modes_and_exactly_one_membership_per_creature() {
    let mut authority = HabitatAuthority::new(habitats()).unwrap();
    authority
        .register_creature(organism(11), habitat(1), Tick::new(3))
        .unwrap();
    authority
        .register_creature(organism(12), habitat(3), Tick::new(4))
        .unwrap();

    assert_eq!(authority.habitats().len(), 4);
    assert_eq!(
        authority
            .habitat(habitat(4))
            .expect("School habitat exists")
            .mode,
        HabitatMode::School
    );
    assert_eq!(
        authority.membership(organism(11)).unwrap().habitat_id,
        habitat(1)
    );
    assert_eq!(
        authority.membership(organism(12)).unwrap().habitat_id,
        habitat(3)
    );
    assert_eq!(authority.validate(&[organism(11), organism(12)]), Ok(()));

    assert_eq!(
        authority.register_creature(organism(11), habitat(2), Tick::new(5)),
        Err(HabitatAuthorityError::DuplicateMembership(organism(11)))
    );
}

#[test]
fn transfers_are_append_only_deterministic_and_preserve_complete_provenance() {
    let mut authority = HabitatAuthority::new(habitats()).unwrap();
    authority
        .register_creature(organism(11), habitat(1), Tick::new(3))
        .unwrap();
    tag_for_reserve(&mut authority, organism(11));
    let provenance = complete_provenance();

    let first = authority
        .transfer(HabitatTransferRequest {
            organism_id: organism(11),
            expected_prior_habitat_id: habitat(1),
            new_habitat_id: habitat(2),
            tick: Tick::new(10),
            provenance: provenance.clone(),
        })
        .unwrap();

    assert_eq!(first.sequence, 1);
    assert_eq!(first.prior_habitat_id, habitat(1));
    assert_eq!(first.new_habitat_id, habitat(2));
    assert_eq!(first.tick, Tick::new(10));
    assert_eq!(first.provenance, provenance);
    assert_eq!(
        authority.membership(organism(11)).unwrap(),
        &HabitatMembership {
            organism_id: organism(11),
            habitat_id: habitat(2),
            entered_tick: Tick::new(10),
            origin_habitat_id: habitat(1),
            origin_tick: Tick::new(3),
            last_transfer_sequence: Some(1),
            quarantine_until: Some(Tick::new(25)),
        }
    );

    let second = authority
        .transfer(HabitatTransferRequest {
            organism_id: organism(11),
            expected_prior_habitat_id: habitat(2),
            new_habitat_id: habitat(3),
            tick: Tick::new(12),
            provenance: HabitatTransferProvenance {
                actor: Some(HabitatActor::Player),
                authority: Some(HabitatAuthorityKind::ManagedController),
                quarantine: Some(QuarantineProvenance::NotRequired),
                assistance: Some(AssistanceProvenance::Unassisted),
                foundation: Some(FoundationProvenance::Known(FoundationId::N2048_V1)),
                possession: Some(PossessionProvenance::NotPossessed),
                selection_exposure: Some(SelectionExposureProvenance::Unexposed),
            },
        })
        .unwrap();

    assert_eq!(second.sequence, 2);
    assert_eq!(authority.transfers(), &[first, second]);
    assert_eq!(authority.validate(&[organism(11)]), Ok(()));
}

#[test]
fn transfer_rejects_unknown_ids_stale_prior_noop_and_missing_provenance() {
    let mut authority = HabitatAuthority::new(habitats()).unwrap();
    authority
        .register_creature(organism(11), habitat(1), Tick::new(3))
        .unwrap();

    let request = |organism_id, prior, new, provenance| HabitatTransferRequest {
        organism_id,
        expected_prior_habitat_id: prior,
        new_habitat_id: new,
        tick: Tick::new(10),
        provenance,
    };

    assert_eq!(
        authority.transfer(request(
            organism(99),
            habitat(1),
            habitat(2),
            complete_provenance()
        )),
        Err(HabitatAuthorityError::UnknownCreature(organism(99)))
    );
    assert_eq!(
        authority.transfer(request(
            organism(11),
            habitat(1),
            habitat(99),
            complete_provenance()
        )),
        Err(HabitatAuthorityError::UnknownHabitat(habitat(99)))
    );
    assert_eq!(
        authority.transfer(request(
            organism(11),
            habitat(2),
            habitat(3),
            complete_provenance()
        )),
        Err(HabitatAuthorityError::StalePriorHabitat {
            organism_id: organism(11),
            expected: habitat(2),
            actual: habitat(1),
        })
    );
    assert_eq!(
        authority.transfer(request(
            organism(11),
            habitat(1),
            habitat(1),
            complete_provenance()
        )),
        Err(HabitatAuthorityError::MalformedTransfer(
            "prior and new habitat must differ"
        ))
    );

    let mut missing = complete_provenance();
    missing.actor = None;
    assert_eq!(
        authority.transfer(request(organism(11), habitat(1), habitat(2), missing)),
        Err(HabitatAuthorityError::MissingProvenance("actor"))
    );
}

#[test]
fn every_transfer_provenance_field_is_required() {
    let mut authority = HabitatAuthority::new(habitats()).unwrap();
    authority
        .register_creature(organism(11), habitat(1), Tick::new(3))
        .unwrap();

    let mut missing_fields = Vec::new();
    let mut value = complete_provenance();
    value.actor = None;
    missing_fields.push(("actor", value));
    let mut value = complete_provenance();
    value.authority = None;
    missing_fields.push(("authority", value));
    let mut value = complete_provenance();
    value.quarantine = None;
    missing_fields.push(("quarantine", value));
    let mut value = complete_provenance();
    value.assistance = None;
    missing_fields.push(("assistance", value));
    let mut value = complete_provenance();
    value.foundation = None;
    missing_fields.push(("foundation", value));
    let mut value = complete_provenance();
    value.possession = None;
    missing_fields.push(("possession", value));
    let mut value = complete_provenance();
    value.selection_exposure = None;
    missing_fields.push(("selection_exposure", value));

    for (field, provenance) in missing_fields {
        assert_eq!(
            authority.transfer(HabitatTransferRequest {
                organism_id: organism(11),
                expected_prior_habitat_id: habitat(1),
                new_habitat_id: habitat(2),
                tick: Tick::new(10),
                provenance,
            }),
            Err(HabitatAuthorityError::MissingProvenance(field))
        );
    }
}

#[test]
fn restore_validation_rejects_duplicate_membership_and_broken_transfer_chains() {
    let base_membership = HabitatMembership {
        organism_id: organism(11),
        habitat_id: habitat(2),
        entered_tick: Tick::new(10),
        origin_habitat_id: habitat(1),
        origin_tick: Tick::new(3),
        last_transfer_sequence: Some(1),
        quarantine_until: Some(Tick::new(25)),
    };
    let transfer = HabitatTransferRecord {
        sequence: 1,
        organism_id: organism(11),
        prior_habitat_id: habitat(1),
        new_habitat_id: habitat(2),
        tick: Tick::new(10),
        provenance: complete_provenance(),
    };

    let duplicate = HabitatAuthoritySnapshot {
        next_transfer_sequence: 2,
        next_tag_sequence: 2,
        habitats: habitats(),
        memberships: vec![base_membership.clone(), base_membership.clone()],
        tags: vec![HabitatTagRecord {
            sequence: 1,
            reserve_id: habitat(2),
            organism_id: organism(11),
            tick: Tick::new(4),
            actor: HabitatActor::Player,
        }],
        transfers: vec![transfer.clone()],
    };
    assert_eq!(
        HabitatAuthority::restore(duplicate, &[organism(11)]),
        Err(HabitatAuthorityError::DuplicateMembership(organism(11)))
    );

    let mut stale = transfer.clone();
    stale.prior_habitat_id = habitat(3);
    let broken_chain = HabitatAuthoritySnapshot {
        next_transfer_sequence: 2,
        next_tag_sequence: 2,
        habitats: habitats(),
        memberships: vec![base_membership],
        tags: vec![HabitatTagRecord {
            sequence: 1,
            reserve_id: habitat(2),
            organism_id: organism(11),
            tick: Tick::new(4),
            actor: HabitatActor::Player,
        }],
        transfers: vec![stale],
    };
    assert_eq!(
        HabitatAuthority::restore(broken_chain, &[organism(11)]),
        Err(HabitatAuthorityError::StaleTransfer {
            sequence: 1,
            organism_id: organism(11),
        })
    );

    let unknown_membership = HabitatAuthoritySnapshot {
        next_transfer_sequence: 1,
        next_tag_sequence: 1,
        habitats: habitats(),
        memberships: vec![HabitatMembership {
            organism_id: organism(99),
            habitat_id: habitat(1),
            entered_tick: Tick::ZERO,
            origin_habitat_id: habitat(1),
            origin_tick: Tick::ZERO,
            last_transfer_sequence: None,
            quarantine_until: None,
        }],
        tags: Vec::new(),
        transfers: Vec::new(),
    };
    assert_eq!(
        HabitatAuthority::restore(unknown_membership, &[organism(11)]),
        Err(HabitatAuthorityError::UnknownCreature(organism(99)))
    );
}

#[test]
fn restore_rejects_unknown_organism_actors_in_transfer_provenance() {
    let mut provenance = complete_provenance();
    provenance.actor = Some(HabitatActor::Organism(organism(99)));
    provenance.authority = Some(HabitatAuthorityKind::CreatureChoice);
    provenance.assistance = Some(AssistanceProvenance::Unassisted);
    provenance.quarantine = Some(QuarantineProvenance::NotRequired);
    provenance.selection_exposure = Some(SelectionExposureProvenance::Unexposed);
    let snapshot = HabitatAuthoritySnapshot {
        next_transfer_sequence: 2,
        next_tag_sequence: 1,
        habitats: habitats(),
        memberships: vec![HabitatMembership {
            organism_id: organism(11),
            habitat_id: habitat(2),
            entered_tick: Tick::new(5),
            origin_habitat_id: habitat(1),
            origin_tick: Tick::new(1),
            last_transfer_sequence: Some(1),
            quarantine_until: None,
        }],
        tags: Vec::new(),
        transfers: vec![HabitatTransferRecord {
            sequence: 1,
            organism_id: organism(11),
            prior_habitat_id: habitat(1),
            new_habitat_id: habitat(2),
            tick: Tick::new(5),
            provenance,
        }],
    };

    assert_eq!(
        HabitatAuthority::restore(snapshot, &[organism(11)]),
        Err(HabitatAuthorityError::UnknownCreature(organism(99)))
    );
}

#[test]
fn every_mode_uses_the_same_gpu_authoritative_cognition_identity() {
    let authority = HabitatAuthority::new(habitats()).unwrap();

    for habitat_id in [habitat(1), habitat(2), habitat(3), habitat(4)] {
        assert_eq!(
            authority.cognition_policy(habitat_id),
            Ok(PolicyBackend::NeuralClosedLoopGpu)
        );
    }
}

#[test]
fn wild_breeding_is_creature_chosen_and_player_breeding_is_rejected() {
    let mut authority = HabitatAuthority::new(habitats()).unwrap();
    for id in [organism(11), organism(12)] {
        authority
            .register_creature(id, habitat(1), Tick::new(1))
            .unwrap();
    }

    assert!(authority
        .authorize_breeding(HabitatBreedingRequest {
            habitat_id: habitat(1),
            first_parent: organism(11),
            second_parent: organism(12),
            kind: HabitatBreedingKind::CreatureChosen,
            actor: HabitatActor::Organism(organism(11)),
            tick: Tick::new(8),
        })
        .is_ok());
    assert_eq!(
        authority.authorize_breeding(HabitatBreedingRequest {
            habitat_id: habitat(1),
            first_parent: organism(11),
            second_parent: organism(12),
            kind: HabitatBreedingKind::Explicit,
            actor: HabitatActor::Player,
            tick: Tick::new(8),
        }),
        Err(HabitatAuthorityError::IllegalBreeding {
            mode: HabitatMode::Wild,
            kind: HabitatBreedingKind::Explicit,
        })
    );
}

#[test]
fn reserve_requires_tags_for_capture_test_breed_and_reintroduction() {
    let mut authority = HabitatAuthority::new(habitats()).unwrap();
    for id in [organism(11), organism(12)] {
        authority
            .register_creature(id, habitat(1), Tick::new(1))
            .unwrap();
    }

    let capture = HabitatOperationRequest {
        habitat_id: habitat(2),
        organism_id: organism(11),
        operation: HabitatOperation::Capture,
        actor: HabitatActor::Player,
        tick: Tick::new(4),
    };
    assert_eq!(
        authority.authorize_operation(capture.clone()),
        Err(HabitatAuthorityError::CreatureNotTagged {
            organism_id: organism(11),
            reserve_id: habitat(2),
        })
    );

    tag_for_reserve(&mut authority, organism(11));
    tag_for_reserve(&mut authority, organism(12));
    assert!(authority.authorize_operation(capture).is_ok());
    for id in [organism(11), organism(12)] {
        authority
            .transfer(HabitatTransferRequest {
                organism_id: id,
                expected_prior_habitat_id: habitat(1),
                new_habitat_id: habitat(2),
                tick: Tick::new(10),
                provenance: complete_provenance(),
            })
            .unwrap();
    }
    assert_eq!(
        authority.authorize_operation(HabitatOperationRequest {
            habitat_id: habitat(2),
            organism_id: organism(11),
            operation: HabitatOperation::Test,
            actor: HabitatActor::Player,
            tick: Tick::new(12),
        }),
        Err(HabitatAuthorityError::QuarantinedUntil {
            organism_id: organism(11),
            until: Tick::new(25),
        })
    );
    assert_eq!(
        authority.authorize_breeding(HabitatBreedingRequest {
            habitat_id: habitat(2),
            first_parent: organism(11),
            second_parent: organism(12),
            kind: HabitatBreedingKind::Explicit,
            actor: HabitatActor::Player,
            tick: Tick::new(12),
        }),
        Err(HabitatAuthorityError::QuarantinedUntil {
            organism_id: organism(11),
            until: Tick::new(25),
        })
    );
    for operation in [HabitatOperation::Test, HabitatOperation::Reintroduce] {
        assert!(authority
            .authorize_operation(HabitatOperationRequest {
                habitat_id: habitat(2),
                organism_id: organism(11),
                operation,
                actor: HabitatActor::Player,
                tick: Tick::new(26),
            })
            .is_ok());
    }
    assert!(authority
        .authorize_breeding(HabitatBreedingRequest {
            habitat_id: habitat(2),
            first_parent: organism(11),
            second_parent: organism(12),
            kind: HabitatBreedingKind::Explicit,
            actor: HabitatActor::Player,
            tick: Tick::new(26),
        })
        .is_ok());
}

#[test]
fn managed_and_school_allow_only_their_explicit_controls() {
    let mut authority = HabitatAuthority::new(habitats()).unwrap();
    authority
        .register_creature(organism(11), habitat(3), Tick::new(1))
        .unwrap();
    authority
        .register_creature(organism(12), habitat(3), Tick::new(1))
        .unwrap();
    authority
        .register_creature(organism(13), habitat(4), Tick::new(1))
        .unwrap();

    for operation in [
        HabitatOperation::Test,
        HabitatOperation::MembershipControl,
        HabitatOperation::StructuredEducation,
    ] {
        assert!(authority
            .authorize_operation(HabitatOperationRequest {
                habitat_id: habitat(3),
                organism_id: organism(11),
                operation,
                actor: HabitatActor::Player,
                tick: Tick::new(5),
            })
            .is_ok());
    }
    assert!(authority
        .authorize_breeding(HabitatBreedingRequest {
            habitat_id: habitat(3),
            first_parent: organism(11),
            second_parent: organism(12),
            kind: HabitatBreedingKind::Explicit,
            actor: HabitatActor::Player,
            tick: Tick::new(5),
        })
        .is_ok());
    assert!(authority
        .authorize_operation(HabitatOperationRequest {
            habitat_id: habitat(4),
            organism_id: organism(13),
            operation: HabitatOperation::StructuredEducation,
            actor: HabitatActor::Teacher,
            tick: Tick::new(5),
        })
        .is_ok());
    assert_eq!(
        authority.authorize_operation(HabitatOperationRequest {
            habitat_id: habitat(4),
            organism_id: organism(13),
            operation: HabitatOperation::Capture,
            actor: HabitatActor::Player,
            tick: Tick::new(5),
        }),
        Err(HabitatAuthorityError::IllegalModeOperation {
            mode: HabitatMode::School,
            operation: HabitatOperation::Capture,
        })
    );
}

#[test]
fn transfer_authority_must_match_an_involved_habitat_mode() {
    let mut authority = HabitatAuthority::new(habitats()).unwrap();
    authority
        .register_creature(organism(11), habitat(1), Tick::new(1))
        .unwrap();

    let mut wrong = complete_provenance();
    wrong.authority = Some(HabitatAuthorityKind::ManagedController);
    assert_eq!(
        authority.transfer(HabitatTransferRequest {
            organism_id: organism(11),
            expected_prior_habitat_id: habitat(1),
            new_habitat_id: habitat(2),
            tick: Tick::new(10),
            provenance: wrong,
        }),
        Err(HabitatAuthorityError::IllegalTransferAuthority {
            prior_mode: HabitatMode::Wild,
            new_mode: HabitatMode::Reserve,
            authority: HabitatAuthorityKind::ManagedController,
        })
    );

    tag_for_reserve(&mut authority, organism(11));
    let mut wrong_actor = complete_provenance();
    wrong_actor.actor = Some(HabitatActor::Teacher);
    assert_eq!(
        authority.transfer(HabitatTransferRequest {
            organism_id: organism(11),
            expected_prior_habitat_id: habitat(1),
            new_habitat_id: habitat(2),
            tick: Tick::new(10),
            provenance: wrong_actor,
        }),
        Err(HabitatAuthorityError::InvalidActor {
            actor: HabitatActor::Teacher,
            context: "reserve transfer",
        })
    );
}

#[test]
fn operation_permissions_reject_actor_roles_that_do_not_own_the_mode() {
    let mut authority = HabitatAuthority::new(habitats()).unwrap();
    authority
        .register_creature(organism(11), habitat(1), Tick::new(1))
        .unwrap();
    assert_eq!(
        authority.tag_creature(
            habitat(2),
            organism(11),
            Tick::new(2),
            HabitatActor::Organism(organism(11)),
        ),
        Err(HabitatAuthorityError::InvalidActor {
            actor: HabitatActor::Organism(organism(11)),
            context: "reserve tag",
        })
    );
    tag_for_reserve(&mut authority, organism(11));
    assert_eq!(
        authority.authorize_operation(HabitatOperationRequest {
            habitat_id: habitat(2),
            organism_id: organism(11),
            operation: HabitatOperation::Capture,
            actor: HabitatActor::Teacher,
            tick: Tick::new(5),
        }),
        Err(HabitatAuthorityError::InvalidActor {
            actor: HabitatActor::Teacher,
            context: "reserve operation",
        })
    );
}

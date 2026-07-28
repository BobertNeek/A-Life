use alife_core::{FoundationId, OrganismId, Tick};
use alife_world::{
    AssistanceProvenance, FoundationProvenance, Habitat, HabitatActor, HabitatAuthority,
    HabitatAuthorityError, HabitatAuthorityKind, HabitatAuthoritySnapshot, HabitatId,
    HabitatMembership, HabitatMode, HabitatTransferProvenance, HabitatTransferRecord,
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
fn restore_validation_rejects_duplicate_membership_and_broken_transfer_chains() {
    let base_membership = HabitatMembership {
        organism_id: organism(11),
        habitat_id: habitat(2),
        entered_tick: Tick::new(10),
        origin_habitat_id: habitat(1),
        origin_tick: Tick::new(3),
        last_transfer_sequence: Some(1),
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
        habitats: habitats(),
        memberships: vec![base_membership.clone(), base_membership.clone()],
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
        habitats: habitats(),
        memberships: vec![base_membership],
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
        habitats: habitats(),
        memberships: vec![HabitatMembership {
            organism_id: organism(99),
            habitat_id: habitat(1),
            entered_tick: Tick::ZERO,
            origin_habitat_id: habitat(1),
            origin_tick: Tick::ZERO,
            last_transfer_sequence: None,
        }],
        transfers: Vec::new(),
    };
    assert_eq!(
        HabitatAuthority::restore(unknown_membership, &[organism(11)]),
        Err(HabitatAuthorityError::UnknownCreature(organism(99)))
    );
}

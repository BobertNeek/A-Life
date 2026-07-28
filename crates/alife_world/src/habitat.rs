//! Engine-neutral habitat membership and transfer authority.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{FoundationId, OrganismId, Tick};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HabitatId(u64);

impl HabitatId {
    pub const DEFAULT_WILD: Self = Self(1);

    pub const fn new(raw: u64) -> Option<Self> {
        if raw == 0 {
            None
        } else {
            Some(Self(raw))
        }
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HabitatMode {
    Wild,
    Reserve,
    Managed,
    School,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Habitat {
    pub id: HabitatId,
    pub label: String,
    pub mode: HabitatMode,
}

impl Habitat {
    pub fn new(
        id: HabitatId,
        label: impl Into<String>,
        mode: HabitatMode,
    ) -> Result<Self, HabitatAuthorityError> {
        let habitat = Self {
            id,
            label: label.into(),
            mode,
        };
        habitat.validate()?;
        Ok(habitat)
    }

    fn validate(&self) -> Result<(), HabitatAuthorityError> {
        if !self.id.is_valid() {
            return Err(HabitatAuthorityError::InvalidHabitatId(self.id.raw()));
        }
        if self.label.trim().is_empty() {
            return Err(HabitatAuthorityError::MalformedHabitat(
                "habitat label is required",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HabitatActor {
    Organism(OrganismId),
    Player,
    Teacher,
    WorldAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HabitatAuthorityKind {
    CreatureChoice,
    ReserveKeeper,
    ManagedController,
    SchoolAdministrator,
    WorldSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuarantineProvenance {
    NotRequired,
    RequiredUntil(Tick),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistanceProvenance {
    Unassisted,
    CaptureTransport,
    StructuredEducation,
    PlayerPossession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoundationProvenance {
    Unknown,
    Known(FoundationId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PossessionProvenance {
    NotPossessed,
    Assisted { started_tick: Tick },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionExposureProvenance {
    Unknown,
    Unexposed,
    Exposed {
        evaluation_id: String,
        exposure_tick: Tick,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HabitatTransferProvenance {
    pub actor: Option<HabitatActor>,
    pub authority: Option<HabitatAuthorityKind>,
    pub quarantine: Option<QuarantineProvenance>,
    pub assistance: Option<AssistanceProvenance>,
    pub foundation: Option<FoundationProvenance>,
    pub possession: Option<PossessionProvenance>,
    pub selection_exposure: Option<SelectionExposureProvenance>,
}

impl HabitatTransferProvenance {
    fn validate(&self, transfer_tick: Tick) -> Result<(), HabitatAuthorityError> {
        let actor = self
            .actor
            .ok_or(HabitatAuthorityError::MissingProvenance("actor"))?;
        self.authority
            .ok_or(HabitatAuthorityError::MissingProvenance("authority"))?;
        let quarantine = self
            .quarantine
            .ok_or(HabitatAuthorityError::MissingProvenance("quarantine"))?;
        let assistance = self
            .assistance
            .ok_or(HabitatAuthorityError::MissingProvenance("assistance"))?;
        let foundation = self
            .foundation
            .ok_or(HabitatAuthorityError::MissingProvenance("foundation"))?;
        let possession = self
            .possession
            .ok_or(HabitatAuthorityError::MissingProvenance("possession"))?;
        let selection_exposure =
            self.selection_exposure
                .as_ref()
                .ok_or(HabitatAuthorityError::MissingProvenance(
                    "selection_exposure",
                ))?;

        if let HabitatActor::Organism(organism_id) = actor {
            if !organism_id.is_valid() {
                return Err(HabitatAuthorityError::MissingProvenance("actor"));
            }
        }
        if let QuarantineProvenance::RequiredUntil(until) = quarantine {
            if until.raw() <= transfer_tick.raw() {
                return Err(HabitatAuthorityError::MalformedProvenance(
                    "quarantine release must follow the transfer tick",
                ));
            }
        }
        if let FoundationProvenance::Known(foundation_id) = foundation {
            if foundation_id.raw() == 0 {
                return Err(HabitatAuthorityError::MalformedProvenance(
                    "foundation id must be nonzero",
                ));
            }
        }
        if let PossessionProvenance::Assisted { started_tick } = possession {
            if started_tick.raw() > transfer_tick.raw() {
                return Err(HabitatAuthorityError::MalformedProvenance(
                    "possession cannot start after the transfer",
                ));
            }
        }
        match selection_exposure {
            SelectionExposureProvenance::Exposed {
                evaluation_id,
                exposure_tick,
            } => {
                if evaluation_id.trim().is_empty() || exposure_tick.raw() > transfer_tick.raw() {
                    return Err(HabitatAuthorityError::MalformedProvenance(
                        "selection exposure requires an id at or before transfer",
                    ));
                }
            }
            SelectionExposureProvenance::Unknown | SelectionExposureProvenance::Unexposed => {}
        }
        if matches!(assistance, AssistanceProvenance::PlayerPossession)
            != matches!(possession, PossessionProvenance::Assisted { .. })
        {
            return Err(HabitatAuthorityError::MalformedProvenance(
                "possession assistance and possession state must agree",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HabitatMembership {
    pub organism_id: OrganismId,
    pub habitat_id: HabitatId,
    pub entered_tick: Tick,
    pub origin_habitat_id: HabitatId,
    pub origin_tick: Tick,
    pub last_transfer_sequence: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HabitatTransferRecord {
    pub sequence: u64,
    pub organism_id: OrganismId,
    pub prior_habitat_id: HabitatId,
    pub new_habitat_id: HabitatId,
    pub tick: Tick,
    pub provenance: HabitatTransferProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HabitatTransferRequest {
    pub organism_id: OrganismId,
    pub expected_prior_habitat_id: HabitatId,
    pub new_habitat_id: HabitatId,
    pub tick: Tick,
    pub provenance: HabitatTransferProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HabitatAuthoritySnapshot {
    pub next_transfer_sequence: u64,
    pub habitats: Vec<Habitat>,
    pub memberships: Vec<HabitatMembership>,
    pub transfers: Vec<HabitatTransferRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HabitatAuthority {
    next_transfer_sequence: u64,
    habitats: Vec<Habitat>,
    memberships: Vec<HabitatMembership>,
    transfers: Vec<HabitatTransferRecord>,
}

impl HabitatAuthority {
    pub fn new(habitats: Vec<Habitat>) -> Result<Self, HabitatAuthorityError> {
        Self::restore(
            HabitatAuthoritySnapshot {
                next_transfer_sequence: 1,
                habitats,
                memberships: Vec::new(),
                transfers: Vec::new(),
            },
            &[],
        )
    }

    pub fn restore(
        mut snapshot: HabitatAuthoritySnapshot,
        known_creatures: &[OrganismId],
    ) -> Result<Self, HabitatAuthorityError> {
        snapshot.habitats.sort_by_key(|habitat| habitat.id.raw());
        snapshot
            .memberships
            .sort_by_key(|membership| membership.organism_id.raw());
        let authority = Self {
            next_transfer_sequence: snapshot.next_transfer_sequence,
            habitats: snapshot.habitats,
            memberships: snapshot.memberships,
            transfers: snapshot.transfers,
        };
        authority.validate(known_creatures)?;
        Ok(authority)
    }

    pub fn habitats(&self) -> &[Habitat] {
        &self.habitats
    }

    pub fn habitat(&self, habitat_id: HabitatId) -> Option<&Habitat> {
        self.habitats
            .binary_search_by_key(&habitat_id.raw(), |habitat| habitat.id.raw())
            .ok()
            .map(|index| &self.habitats[index])
    }

    pub fn membership(&self, organism_id: OrganismId) -> Option<&HabitatMembership> {
        self.memberships
            .binary_search_by_key(&organism_id.raw(), |membership| {
                membership.organism_id.raw()
            })
            .ok()
            .map(|index| &self.memberships[index])
    }

    pub fn transfers(&self) -> &[HabitatTransferRecord] {
        &self.transfers
    }

    pub fn register_creature(
        &mut self,
        organism_id: OrganismId,
        habitat_id: HabitatId,
        tick: Tick,
    ) -> Result<(), HabitatAuthorityError> {
        if !organism_id.is_valid() {
            return Err(HabitatAuthorityError::UnknownCreature(organism_id));
        }
        if self.habitat(habitat_id).is_none() {
            return Err(HabitatAuthorityError::UnknownHabitat(habitat_id));
        }
        match self
            .memberships
            .binary_search_by_key(&organism_id.raw(), |membership| {
                membership.organism_id.raw()
            }) {
            Ok(_) => Err(HabitatAuthorityError::DuplicateMembership(organism_id)),
            Err(index) => {
                self.memberships.insert(
                    index,
                    HabitatMembership {
                        organism_id,
                        habitat_id,
                        entered_tick: tick,
                        origin_habitat_id: habitat_id,
                        origin_tick: tick,
                        last_transfer_sequence: None,
                    },
                );
                Ok(())
            }
        }
    }

    pub fn transfer(
        &mut self,
        request: HabitatTransferRequest,
    ) -> Result<HabitatTransferRecord, HabitatAuthorityError> {
        if !request.organism_id.is_valid() {
            return Err(HabitatAuthorityError::UnknownCreature(request.organism_id));
        }
        if self.habitat(request.new_habitat_id).is_none() {
            return Err(HabitatAuthorityError::UnknownHabitat(
                request.new_habitat_id,
            ));
        }
        if request.expected_prior_habitat_id == request.new_habitat_id {
            return Err(HabitatAuthorityError::MalformedTransfer(
                "prior and new habitat must differ",
            ));
        }
        request.provenance.validate(request.tick)?;

        let membership_index = self
            .memberships
            .binary_search_by_key(&request.organism_id.raw(), |membership| {
                membership.organism_id.raw()
            })
            .map_err(|_| HabitatAuthorityError::UnknownCreature(request.organism_id))?;
        let membership = &self.memberships[membership_index];
        if membership.habitat_id != request.expected_prior_habitat_id {
            return Err(HabitatAuthorityError::StalePriorHabitat {
                organism_id: request.organism_id,
                expected: request.expected_prior_habitat_id,
                actual: membership.habitat_id,
            });
        }
        if request.tick.raw() < membership.entered_tick.raw() {
            return Err(HabitatAuthorityError::StaleTransfer {
                sequence: self.next_transfer_sequence,
                organism_id: request.organism_id,
            });
        }
        let sequence = self.next_transfer_sequence;
        let next_sequence =
            sequence
                .checked_add(1)
                .ok_or(HabitatAuthorityError::MalformedTransfer(
                    "transfer sequence exhausted",
                ))?;
        let record = HabitatTransferRecord {
            sequence,
            organism_id: request.organism_id,
            prior_habitat_id: request.expected_prior_habitat_id,
            new_habitat_id: request.new_habitat_id,
            tick: request.tick,
            provenance: request.provenance,
        };
        self.transfers.push(record.clone());
        self.next_transfer_sequence = next_sequence;
        let membership = &mut self.memberships[membership_index];
        membership.habitat_id = record.new_habitat_id;
        membership.entered_tick = record.tick;
        membership.last_transfer_sequence = Some(record.sequence);
        Ok(record)
    }

    pub fn validate(&self, known_creatures: &[OrganismId]) -> Result<(), HabitatAuthorityError> {
        if self.habitats.is_empty() {
            return Err(HabitatAuthorityError::MalformedHabitat(
                "at least one habitat is required",
            ));
        }
        let mut habitat_ids = BTreeSet::new();
        for habitat in &self.habitats {
            habitat.validate()?;
            if !habitat_ids.insert(habitat.id.raw()) {
                return Err(HabitatAuthorityError::DuplicateHabitat(habitat.id));
            }
        }

        let known = known_creatures
            .iter()
            .map(|organism_id| organism_id.raw())
            .collect::<BTreeSet<_>>();
        let mut membership_ids = BTreeSet::new();
        for membership in &self.memberships {
            if !membership_ids.insert(membership.organism_id.raw()) {
                return Err(HabitatAuthorityError::DuplicateMembership(
                    membership.organism_id,
                ));
            }
            if !membership.organism_id.is_valid() || !known.contains(&membership.organism_id.raw())
            {
                return Err(HabitatAuthorityError::UnknownCreature(
                    membership.organism_id,
                ));
            }
            for habitat_id in [membership.habitat_id, membership.origin_habitat_id] {
                if !habitat_ids.contains(&habitat_id.raw()) {
                    return Err(HabitatAuthorityError::UnknownHabitat(habitat_id));
                }
            }
        }
        for organism_id in known_creatures {
            if !organism_id.is_valid() {
                return Err(HabitatAuthorityError::UnknownCreature(*organism_id));
            }
            if !membership_ids.contains(&organism_id.raw()) {
                return Err(HabitatAuthorityError::MissingMembership(*organism_id));
            }
        }

        if self.next_transfer_sequence == 0 {
            return Err(HabitatAuthorityError::MalformedTransfer(
                "next transfer sequence must be nonzero",
            ));
        }
        let mut chains: BTreeMap<u64, (HabitatId, Tick, u64)> = BTreeMap::new();
        for (index, transfer) in self.transfers.iter().enumerate() {
            let expected_sequence = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or(HabitatAuthorityError::MalformedTransfer(
                    "transfer sequence exhausted",
                ))?;
            if transfer.sequence != expected_sequence
                || transfer.prior_habitat_id == transfer.new_habitat_id
            {
                return Err(HabitatAuthorityError::MalformedTransfer(
                    "transfer sequences must be contiguous and change habitat",
                ));
            }
            if !known.contains(&transfer.organism_id.raw()) {
                return Err(HabitatAuthorityError::UnknownCreature(transfer.organism_id));
            }
            for habitat_id in [transfer.prior_habitat_id, transfer.new_habitat_id] {
                if !habitat_ids.contains(&habitat_id.raw()) {
                    return Err(HabitatAuthorityError::UnknownHabitat(habitat_id));
                }
            }
            transfer.provenance.validate(transfer.tick)?;

            let membership = self
                .membership(transfer.organism_id)
                .ok_or(HabitatAuthorityError::UnknownCreature(transfer.organism_id))?;
            let (expected_prior, earliest_tick) = chains
                .get(&transfer.organism_id.raw())
                .map(|(habitat_id, tick, _)| (*habitat_id, *tick))
                .unwrap_or((membership.origin_habitat_id, membership.origin_tick));
            if transfer.prior_habitat_id != expected_prior
                || transfer.tick.raw() < earliest_tick.raw()
            {
                return Err(HabitatAuthorityError::StaleTransfer {
                    sequence: transfer.sequence,
                    organism_id: transfer.organism_id,
                });
            }
            chains.insert(
                transfer.organism_id.raw(),
                (transfer.new_habitat_id, transfer.tick, transfer.sequence),
            );
        }
        let expected_next = u64::try_from(self.transfers.len())
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(HabitatAuthorityError::MalformedTransfer(
                "transfer sequence exhausted",
            ))?;
        if self.next_transfer_sequence != expected_next {
            return Err(HabitatAuthorityError::MalformedTransfer(
                "next transfer sequence does not follow the ledger",
            ));
        }
        for membership in &self.memberships {
            match chains.get(&membership.organism_id.raw()) {
                Some((habitat_id, tick, sequence)) => {
                    if membership.habitat_id != *habitat_id
                        || membership.entered_tick != *tick
                        || membership.last_transfer_sequence != Some(*sequence)
                    {
                        return Err(HabitatAuthorityError::StaleTransfer {
                            sequence: *sequence,
                            organism_id: membership.organism_id,
                        });
                    }
                }
                None => {
                    if membership.habitat_id != membership.origin_habitat_id
                        || membership.entered_tick != membership.origin_tick
                        || membership.last_transfer_sequence.is_some()
                    {
                        return Err(HabitatAuthorityError::MalformedTransfer(
                            "untransferred membership must match its origin",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HabitatAuthorityError {
    #[error("invalid habitat id {0}")]
    InvalidHabitatId(u64),
    #[error("malformed habitat: {0}")]
    MalformedHabitat(&'static str),
    #[error("duplicate habitat {0:?}")]
    DuplicateHabitat(HabitatId),
    #[error("duplicate membership for {0:?}")]
    DuplicateMembership(OrganismId),
    #[error("missing membership for {0:?}")]
    MissingMembership(OrganismId),
    #[error("unknown habitat {0:?}")]
    UnknownHabitat(HabitatId),
    #[error("unknown creature {0:?}")]
    UnknownCreature(OrganismId),
    #[error("stale prior habitat for {organism_id:?}: expected {expected:?}, actual {actual:?}")]
    StalePriorHabitat {
        organism_id: OrganismId,
        expected: HabitatId,
        actual: HabitatId,
    },
    #[error("stale transfer {sequence} for {organism_id:?}")]
    StaleTransfer {
        sequence: u64,
        organism_id: OrganismId,
    },
    #[error("malformed transfer: {0}")]
    MalformedTransfer(&'static str),
    #[error("missing transfer provenance: {0}")]
    MissingProvenance(&'static str),
    #[error("malformed transfer provenance: {0}")]
    MalformedProvenance(&'static str),
}

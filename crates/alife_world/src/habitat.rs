//! Engine-neutral habitat membership and transfer authority.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{FoundationId, OrganismId, PolicyBackend, Tick};
use serde::{Deserialize, Serialize};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HabitatMode {
    Wild,
    Reserve,
    Managed,
    School,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HabitatOperation {
    Tag,
    Capture,
    Test,
    Reintroduce,
    MembershipControl,
    StructuredEducation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HabitatBreedingKind {
    CreatureChosen,
    Explicit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HabitatActor {
    Organism(OrganismId),
    Player,
    Teacher,
    WorldAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HabitatAuthorityKind {
    CreatureChoice,
    ReserveKeeper,
    ManagedController,
    SchoolAdministrator,
    WorldSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuarantineProvenance {
    NotRequired,
    RequiredUntil(Tick),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistanceProvenance {
    Unassisted,
    CaptureTransport,
    StructuredEducation,
    PlayerPossession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundationProvenance {
    Unknown,
    Known(FoundationId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PossessionProvenance {
    NotPossessed,
    Assisted { started_tick: Tick },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionExposureProvenance {
    Unknown,
    Unexposed,
    Exposed {
        evaluation_id: String,
        exposure_tick: Tick,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    fn quarantine_until(&self) -> Option<Tick> {
        match self.quarantine {
            Some(QuarantineProvenance::RequiredUntil(until)) => Some(until),
            Some(QuarantineProvenance::NotRequired) | None => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatMembership {
    pub organism_id: OrganismId,
    pub habitat_id: HabitatId,
    pub entered_tick: Tick,
    pub origin_habitat_id: HabitatId,
    pub origin_tick: Tick,
    pub last_transfer_sequence: Option<u64>,
    pub quarantine_until: Option<Tick>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatTransferRecord {
    pub sequence: u64,
    pub organism_id: OrganismId,
    pub prior_habitat_id: HabitatId,
    pub new_habitat_id: HabitatId,
    pub tick: Tick,
    pub provenance: HabitatTransferProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatTransferRequest {
    pub organism_id: OrganismId,
    pub expected_prior_habitat_id: HabitatId,
    pub new_habitat_id: HabitatId,
    pub tick: Tick,
    pub provenance: HabitatTransferProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatTagRecord {
    pub sequence: u64,
    pub reserve_id: HabitatId,
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub actor: HabitatActor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatOperationRequest {
    pub habitat_id: HabitatId,
    pub organism_id: OrganismId,
    pub operation: HabitatOperation,
    pub actor: HabitatActor,
    pub tick: Tick,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatPermissionReceipt {
    pub habitat_id: HabitatId,
    pub organism_id: OrganismId,
    pub mode: HabitatMode,
    pub operation: HabitatOperation,
    pub actor: HabitatActor,
    pub tick: Tick,
    pub cognition_policy: PolicyBackend,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatBreedingRequest {
    pub habitat_id: HabitatId,
    pub first_parent: OrganismId,
    pub second_parent: OrganismId,
    pub kind: HabitatBreedingKind,
    pub actor: HabitatActor,
    pub tick: Tick,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatBreedingReceipt {
    pub habitat_id: HabitatId,
    pub first_parent: OrganismId,
    pub second_parent: OrganismId,
    pub mode: HabitatMode,
    pub kind: HabitatBreedingKind,
    pub actor: HabitatActor,
    pub tick: Tick,
    pub cognition_policy: PolicyBackend,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatAuthoritySnapshot {
    pub next_transfer_sequence: u64,
    pub next_tag_sequence: u64,
    pub habitats: Vec<Habitat>,
    pub memberships: Vec<HabitatMembership>,
    pub tags: Vec<HabitatTagRecord>,
    pub transfers: Vec<HabitatTransferRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatAuthority {
    next_transfer_sequence: u64,
    next_tag_sequence: u64,
    habitats: Vec<Habitat>,
    memberships: Vec<HabitatMembership>,
    tags: Vec<HabitatTagRecord>,
    transfers: Vec<HabitatTransferRecord>,
}

impl Default for HabitatAuthority {
    fn default() -> Self {
        Self {
            next_transfer_sequence: 1,
            next_tag_sequence: 1,
            habitats: vec![Habitat {
                id: HabitatId::DEFAULT_WILD,
                label: "Wild".to_string(),
                mode: HabitatMode::Wild,
            }],
            memberships: Vec::new(),
            tags: Vec::new(),
            transfers: Vec::new(),
        }
    }
}

impl HabitatAuthority {
    pub fn new(habitats: Vec<Habitat>) -> Result<Self, HabitatAuthorityError> {
        Self::restore(
            HabitatAuthoritySnapshot {
                next_transfer_sequence: 1,
                next_tag_sequence: 1,
                habitats,
                memberships: Vec::new(),
                tags: Vec::new(),
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
            next_tag_sequence: snapshot.next_tag_sequence,
            habitats: snapshot.habitats,
            memberships: snapshot.memberships,
            tags: snapshot.tags,
            transfers: snapshot.transfers,
        };
        authority.validate(known_creatures)?;
        Ok(authority)
    }

    pub fn habitats(&self) -> &[Habitat] {
        &self.habitats
    }

    pub fn memberships(&self) -> &[HabitatMembership] {
        &self.memberships
    }

    pub(crate) fn is_unassigned_default(&self) -> bool {
        self.habitats.len() == 1
            && self.habitats[0].id == HabitatId::DEFAULT_WILD
            && self.habitats[0].mode == HabitatMode::Wild
            && self.memberships.is_empty()
            && self.tags.is_empty()
            && self.transfers.is_empty()
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

    pub fn tags(&self) -> &[HabitatTagRecord] {
        &self.tags
    }

    pub fn cognition_policy(
        &self,
        habitat_id: HabitatId,
    ) -> Result<PolicyBackend, HabitatAuthorityError> {
        self.habitat(habitat_id)
            .ok_or(HabitatAuthorityError::UnknownHabitat(habitat_id))?;
        Ok(PolicyBackend::NeuralClosedLoopGpu)
    }

    pub fn tag_creature(
        &mut self,
        reserve_id: HabitatId,
        organism_id: OrganismId,
        tick: Tick,
        actor: HabitatActor,
    ) -> Result<HabitatTagRecord, HabitatAuthorityError> {
        let reserve = self
            .habitat(reserve_id)
            .ok_or(HabitatAuthorityError::UnknownHabitat(reserve_id))?;
        if reserve.mode != HabitatMode::Reserve {
            return Err(HabitatAuthorityError::IllegalModeOperation {
                mode: reserve.mode,
                operation: HabitatOperation::Tag,
            });
        }
        let membership = self
            .membership(organism_id)
            .ok_or(HabitatAuthorityError::UnknownCreature(organism_id))?;
        if tick.raw() < membership.origin_tick.raw() {
            return Err(HabitatAuthorityError::StaleTag {
                sequence: self.next_tag_sequence,
                organism_id,
            });
        }
        self.validate_actor_known(actor)?;
        if !matches!(actor, HabitatActor::Player | HabitatActor::WorldAuthority) {
            return Err(HabitatAuthorityError::InvalidActor {
                actor,
                context: "reserve tag",
            });
        }
        if self.is_tagged(reserve_id, organism_id) {
            return Err(HabitatAuthorityError::DuplicateTag {
                organism_id,
                reserve_id,
            });
        }
        let sequence = self.next_tag_sequence;
        self.next_tag_sequence =
            sequence
                .checked_add(1)
                .ok_or(HabitatAuthorityError::MalformedTag(
                    "tag sequence exhausted",
                ))?;
        let record = HabitatTagRecord {
            sequence,
            reserve_id,
            organism_id,
            tick,
            actor,
        };
        self.tags.push(record.clone());
        Ok(record)
    }

    pub fn authorize_operation(
        &self,
        request: HabitatOperationRequest,
    ) -> Result<HabitatPermissionReceipt, HabitatAuthorityError> {
        let habitat = self
            .habitat(request.habitat_id)
            .ok_or(HabitatAuthorityError::UnknownHabitat(request.habitat_id))?;
        let membership = self
            .membership(request.organism_id)
            .ok_or(HabitatAuthorityError::UnknownCreature(request.organism_id))?;
        self.validate_actor_known(request.actor)?;
        if request.tick.raw() < membership.entered_tick.raw() {
            return Err(HabitatAuthorityError::MalformedOperation(
                "operation tick precedes current membership",
            ));
        }
        if matches!(
            request.operation,
            HabitatOperation::Test | HabitatOperation::StructuredEducation
        ) {
            if let Some(until) = membership.quarantine_until {
                if request.tick.raw() < until.raw() {
                    return Err(HabitatAuthorityError::QuarantinedUntil {
                        organism_id: request.organism_id,
                        until,
                    });
                }
            }
        }

        let actor_allowed = match habitat.mode {
            HabitatMode::Wild => false,
            HabitatMode::Reserve => {
                matches!(
                    request.actor,
                    HabitatActor::Player | HabitatActor::WorldAuthority
                )
            }
            HabitatMode::Managed => match request.operation {
                HabitatOperation::StructuredEducation => matches!(
                    request.actor,
                    HabitatActor::Player | HabitatActor::Teacher | HabitatActor::WorldAuthority
                ),
                _ => matches!(
                    request.actor,
                    HabitatActor::Player | HabitatActor::WorldAuthority
                ),
            },
            HabitatMode::School => matches!(
                request.actor,
                HabitatActor::Player | HabitatActor::Teacher | HabitatActor::WorldAuthority
            ),
        };
        if !actor_allowed {
            let context = match habitat.mode {
                HabitatMode::Wild => "wild operation",
                HabitatMode::Reserve => "reserve operation",
                HabitatMode::Managed => "managed operation",
                HabitatMode::School => "school operation",
            };
            return Err(HabitatAuthorityError::InvalidActor {
                actor: request.actor,
                context,
            });
        }

        let allowed = match habitat.mode {
            HabitatMode::Wild => false,
            HabitatMode::Reserve => matches!(
                request.operation,
                HabitatOperation::Capture | HabitatOperation::Test | HabitatOperation::Reintroduce
            ),
            HabitatMode::Managed => matches!(
                request.operation,
                HabitatOperation::Test
                    | HabitatOperation::MembershipControl
                    | HabitatOperation::StructuredEducation
            ),
            HabitatMode::School => matches!(
                request.operation,
                HabitatOperation::MembershipControl | HabitatOperation::StructuredEducation
            ),
        };
        if !allowed {
            return Err(HabitatAuthorityError::IllegalModeOperation {
                mode: habitat.mode,
                operation: request.operation,
            });
        }
        if habitat.mode == HabitatMode::Reserve {
            if !self.is_tagged(request.habitat_id, request.organism_id) {
                return Err(HabitatAuthorityError::CreatureNotTagged {
                    organism_id: request.organism_id,
                    reserve_id: request.habitat_id,
                });
            }
            if request.operation != HabitatOperation::Capture
                && membership.habitat_id != request.habitat_id
            {
                return Err(HabitatAuthorityError::IllegalModeOperation {
                    mode: habitat.mode,
                    operation: request.operation,
                });
            }
        } else if request.operation != HabitatOperation::MembershipControl
            && membership.habitat_id != request.habitat_id
        {
            return Err(HabitatAuthorityError::IllegalModeOperation {
                mode: habitat.mode,
                operation: request.operation,
            });
        }

        Ok(HabitatPermissionReceipt {
            habitat_id: request.habitat_id,
            organism_id: request.organism_id,
            mode: habitat.mode,
            operation: request.operation,
            actor: request.actor,
            tick: request.tick,
            cognition_policy: PolicyBackend::NeuralClosedLoopGpu,
        })
    }

    pub fn authorize_breeding(
        &self,
        request: HabitatBreedingRequest,
    ) -> Result<HabitatBreedingReceipt, HabitatAuthorityError> {
        if request.first_parent == request.second_parent {
            return Err(HabitatAuthorityError::MalformedOperation(
                "breeding requires two distinct parents",
            ));
        }
        let habitat = self
            .habitat(request.habitat_id)
            .ok_or(HabitatAuthorityError::UnknownHabitat(request.habitat_id))?;
        let first = self
            .membership(request.first_parent)
            .ok_or(HabitatAuthorityError::UnknownCreature(request.first_parent))?;
        let second = self.membership(request.second_parent).ok_or(
            HabitatAuthorityError::UnknownCreature(request.second_parent),
        )?;
        self.validate_actor_known(request.actor)?;
        if first.habitat_id != request.habitat_id
            || second.habitat_id != request.habitat_id
            || request.tick.raw() < first.entered_tick.raw()
            || request.tick.raw() < second.entered_tick.raw()
        {
            return Err(HabitatAuthorityError::IllegalBreeding {
                mode: habitat.mode,
                kind: request.kind,
            });
        }
        for membership in [first, second] {
            if let Some(until) = membership.quarantine_until {
                if request.tick.raw() < until.raw() {
                    return Err(HabitatAuthorityError::QuarantinedUntil {
                        organism_id: membership.organism_id,
                        until,
                    });
                }
            }
        }

        let allowed = match (habitat.mode, request.kind) {
            (HabitatMode::Wild, HabitatBreedingKind::CreatureChosen)
            | (HabitatMode::Reserve, HabitatBreedingKind::CreatureChosen) => {
                matches!(
                    request.actor,
                    HabitatActor::Organism(id)
                        if id == request.first_parent || id == request.second_parent
                )
            }
            (HabitatMode::Reserve, HabitatBreedingKind::Explicit) => {
                matches!(
                    request.actor,
                    HabitatActor::Player | HabitatActor::WorldAuthority
                ) && self.is_tagged(request.habitat_id, request.first_parent)
                    && self.is_tagged(request.habitat_id, request.second_parent)
            }
            (HabitatMode::Managed, HabitatBreedingKind::Explicit) => {
                matches!(
                    request.actor,
                    HabitatActor::Player | HabitatActor::WorldAuthority
                )
            }
            _ => false,
        };
        if !allowed {
            if matches!(
                (habitat.mode, request.kind),
                (HabitatMode::Wild, HabitatBreedingKind::CreatureChosen)
                    | (HabitatMode::Reserve, HabitatBreedingKind::CreatureChosen)
                    | (HabitatMode::Reserve, HabitatBreedingKind::Explicit)
                    | (HabitatMode::Managed, HabitatBreedingKind::Explicit)
            ) {
                if habitat.mode == HabitatMode::Reserve
                    && request.kind == HabitatBreedingKind::Explicit
                {
                    for organism_id in [request.first_parent, request.second_parent] {
                        if !self.is_tagged(request.habitat_id, organism_id) {
                            return Err(HabitatAuthorityError::CreatureNotTagged {
                                organism_id,
                                reserve_id: request.habitat_id,
                            });
                        }
                    }
                }
                return Err(HabitatAuthorityError::InvalidActor {
                    actor: request.actor,
                    context: "breeding permission",
                });
            }
            return Err(HabitatAuthorityError::IllegalBreeding {
                mode: habitat.mode,
                kind: request.kind,
            });
        }

        Ok(HabitatBreedingReceipt {
            habitat_id: request.habitat_id,
            first_parent: request.first_parent,
            second_parent: request.second_parent,
            mode: habitat.mode,
            kind: request.kind,
            actor: request.actor,
            tick: request.tick,
            cognition_policy: PolicyBackend::NeuralClosedLoopGpu,
        })
    }

    fn is_tagged(&self, reserve_id: HabitatId, organism_id: OrganismId) -> bool {
        self.tags
            .iter()
            .any(|tag| tag.reserve_id == reserve_id && tag.organism_id == organism_id)
    }

    fn validate_actor_known(&self, actor: HabitatActor) -> Result<(), HabitatAuthorityError> {
        if let HabitatActor::Organism(organism_id) = actor {
            if self.membership(organism_id).is_none() {
                return Err(HabitatAuthorityError::UnknownCreature(organism_id));
            }
        }
        Ok(())
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
                        quarantine_until: None,
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
        let current_habitat_id = self.memberships[membership_index].habitat_id;
        let entered_tick = self.memberships[membership_index].entered_tick;
        if current_habitat_id != request.expected_prior_habitat_id {
            return Err(HabitatAuthorityError::StalePriorHabitat {
                organism_id: request.organism_id,
                expected: request.expected_prior_habitat_id,
                actual: current_habitat_id,
            });
        }
        if request.tick.raw() < entered_tick.raw() {
            return Err(HabitatAuthorityError::StaleTransfer {
                sequence: self.next_transfer_sequence,
                organism_id: request.organism_id,
            });
        }
        self.validate_transfer_authority(
            request.organism_id,
            request.expected_prior_habitat_id,
            request.new_habitat_id,
            &request.provenance,
        )?;
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
        membership.quarantine_until = record.provenance.quarantine_until();
        Ok(record)
    }

    fn validate_transfer_authority(
        &self,
        organism_id: OrganismId,
        prior_habitat_id: HabitatId,
        new_habitat_id: HabitatId,
        provenance: &HabitatTransferProvenance,
    ) -> Result<(), HabitatAuthorityError> {
        let prior_mode = self
            .habitat(prior_habitat_id)
            .ok_or(HabitatAuthorityError::UnknownHabitat(prior_habitat_id))?
            .mode;
        let new_mode = self
            .habitat(new_habitat_id)
            .ok_or(HabitatAuthorityError::UnknownHabitat(new_habitat_id))?
            .mode;
        let actor = provenance
            .actor
            .ok_or(HabitatAuthorityError::MissingProvenance("actor"))?;
        let authority = provenance
            .authority
            .ok_or(HabitatAuthorityError::MissingProvenance("authority"))?;
        self.validate_actor_known(actor)?;

        let invalid_authority = || HabitatAuthorityError::IllegalTransferAuthority {
            prior_mode,
            new_mode,
            authority,
        };
        match authority {
            HabitatAuthorityKind::CreatureChoice => {
                if prior_mode != HabitatMode::Wild || new_mode != HabitatMode::Wild {
                    return Err(invalid_authority());
                }
                if actor != HabitatActor::Organism(organism_id) {
                    return Err(HabitatAuthorityError::InvalidActor {
                        actor,
                        context: "wild transfer",
                    });
                }
            }
            HabitatAuthorityKind::ReserveKeeper => {
                if prior_mode != HabitatMode::Reserve && new_mode != HabitatMode::Reserve {
                    return Err(invalid_authority());
                }
                if !matches!(actor, HabitatActor::Player | HabitatActor::WorldAuthority) {
                    return Err(HabitatAuthorityError::InvalidActor {
                        actor,
                        context: "reserve transfer",
                    });
                }
                let reserve_id = if new_mode == HabitatMode::Reserve {
                    new_habitat_id
                } else {
                    prior_habitat_id
                };
                if !self.is_tagged(reserve_id, organism_id) {
                    return Err(HabitatAuthorityError::CreatureNotTagged {
                        organism_id,
                        reserve_id,
                    });
                }
            }
            HabitatAuthorityKind::ManagedController => {
                if prior_mode != HabitatMode::Managed && new_mode != HabitatMode::Managed {
                    return Err(invalid_authority());
                }
                if !matches!(actor, HabitatActor::Player | HabitatActor::WorldAuthority) {
                    return Err(HabitatAuthorityError::InvalidActor {
                        actor,
                        context: "managed transfer",
                    });
                }
            }
            HabitatAuthorityKind::SchoolAdministrator => {
                if prior_mode != HabitatMode::School && new_mode != HabitatMode::School {
                    return Err(invalid_authority());
                }
                if !matches!(
                    actor,
                    HabitatActor::Player | HabitatActor::Teacher | HabitatActor::WorldAuthority
                ) {
                    return Err(HabitatAuthorityError::InvalidActor {
                        actor,
                        context: "school transfer",
                    });
                }
            }
            HabitatAuthorityKind::WorldSystem => {
                if actor != HabitatActor::WorldAuthority {
                    return Err(HabitatAuthorityError::InvalidActor {
                        actor,
                        context: "world transfer",
                    });
                }
            }
        }
        Ok(())
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

        if self.next_tag_sequence == 0 {
            return Err(HabitatAuthorityError::MalformedTag(
                "next tag sequence must be nonzero",
            ));
        }
        let mut tagged_creatures = BTreeSet::new();
        for (index, tag) in self.tags.iter().enumerate() {
            let expected_sequence = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or(HabitatAuthorityError::MalformedTag(
                    "tag sequence exhausted",
                ))?;
            if tag.sequence != expected_sequence {
                return Err(HabitatAuthorityError::MalformedTag(
                    "tag sequences must be contiguous",
                ));
            }
            let reserve = self
                .habitat(tag.reserve_id)
                .ok_or(HabitatAuthorityError::UnknownHabitat(tag.reserve_id))?;
            if reserve.mode != HabitatMode::Reserve {
                return Err(HabitatAuthorityError::IllegalModeOperation {
                    mode: reserve.mode,
                    operation: HabitatOperation::Tag,
                });
            }
            let membership = self
                .membership(tag.organism_id)
                .ok_or(HabitatAuthorityError::UnknownCreature(tag.organism_id))?;
            self.validate_actor_known(tag.actor)?;
            if !matches!(
                tag.actor,
                HabitatActor::Player | HabitatActor::WorldAuthority
            ) {
                return Err(HabitatAuthorityError::InvalidActor {
                    actor: tag.actor,
                    context: "reserve tag",
                });
            }
            if tag.tick.raw() < membership.origin_tick.raw() {
                return Err(HabitatAuthorityError::StaleTag {
                    sequence: tag.sequence,
                    organism_id: tag.organism_id,
                });
            }
            if !tagged_creatures.insert((tag.reserve_id.raw(), tag.organism_id.raw())) {
                return Err(HabitatAuthorityError::DuplicateTag {
                    organism_id: tag.organism_id,
                    reserve_id: tag.reserve_id,
                });
            }
        }
        let expected_next_tag = u64::try_from(self.tags.len())
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(HabitatAuthorityError::MalformedTag(
                "tag sequence exhausted",
            ))?;
        if self.next_tag_sequence != expected_next_tag {
            return Err(HabitatAuthorityError::MalformedTag(
                "next tag sequence does not follow the ledger",
            ));
        }

        if self.next_transfer_sequence == 0 {
            return Err(HabitatAuthorityError::MalformedTransfer(
                "next transfer sequence must be nonzero",
            ));
        }
        let mut chains: BTreeMap<u64, (HabitatId, Tick, u64, Option<Tick>)> = BTreeMap::new();
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
            if let Some(HabitatActor::Organism(actor_id)) = transfer.provenance.actor {
                if !known.contains(&actor_id.raw()) {
                    return Err(HabitatAuthorityError::UnknownCreature(actor_id));
                }
            }
            self.validate_transfer_authority(
                transfer.organism_id,
                transfer.prior_habitat_id,
                transfer.new_habitat_id,
                &transfer.provenance,
            )?;

            let membership = self
                .membership(transfer.organism_id)
                .ok_or(HabitatAuthorityError::UnknownCreature(transfer.organism_id))?;
            let (expected_prior, earliest_tick) = chains
                .get(&transfer.organism_id.raw())
                .map(|(habitat_id, tick, _, _)| (*habitat_id, *tick))
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
                (
                    transfer.new_habitat_id,
                    transfer.tick,
                    transfer.sequence,
                    transfer.provenance.quarantine_until(),
                ),
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
                Some((habitat_id, tick, sequence, quarantine_until)) => {
                    if membership.habitat_id != *habitat_id
                        || membership.entered_tick != *tick
                        || membership.last_transfer_sequence != Some(*sequence)
                        || membership.quarantine_until != *quarantine_until
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
                        || membership.quarantine_until.is_some()
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

    pub fn validate_at_tick(
        &self,
        known_creatures: &[OrganismId],
        world_tick: Tick,
    ) -> Result<(), HabitatAuthorityError> {
        self.validate(known_creatures)?;
        for membership in &self.memberships {
            if membership.origin_tick.raw() > world_tick.raw()
                || membership.entered_tick.raw() > world_tick.raw()
            {
                return Err(HabitatAuthorityError::MalformedTransfer(
                    "membership tick cannot exceed the world tick",
                ));
            }
        }
        for tag in &self.tags {
            if tag.tick.raw() > world_tick.raw() {
                return Err(HabitatAuthorityError::StaleTag {
                    sequence: tag.sequence,
                    organism_id: tag.organism_id,
                });
            }
        }
        for transfer in &self.transfers {
            if transfer.tick.raw() > world_tick.raw() {
                return Err(HabitatAuthorityError::StaleTransfer {
                    sequence: transfer.sequence,
                    organism_id: transfer.organism_id,
                });
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
    #[error("duplicate reserve tag for {organism_id:?} in {reserve_id:?}")]
    DuplicateTag {
        organism_id: OrganismId,
        reserve_id: HabitatId,
    },
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
    #[error("stale reserve tag {sequence} for {organism_id:?}")]
    StaleTag {
        sequence: u64,
        organism_id: OrganismId,
    },
    #[error("malformed transfer: {0}")]
    MalformedTransfer(&'static str),
    #[error("missing transfer provenance: {0}")]
    MissingProvenance(&'static str),
    #[error("malformed transfer provenance: {0}")]
    MalformedProvenance(&'static str),
    #[error("malformed reserve tag: {0}")]
    MalformedTag(&'static str),
    #[error("malformed habitat operation: {0}")]
    MalformedOperation(&'static str),
    #[error("{operation:?} is illegal in {mode:?} mode")]
    IllegalModeOperation {
        mode: HabitatMode,
        operation: HabitatOperation,
    },
    #[error("{kind:?} breeding is illegal in {mode:?} mode")]
    IllegalBreeding {
        mode: HabitatMode,
        kind: HabitatBreedingKind,
    },
    #[error("{organism_id:?} is not tagged by reserve {reserve_id:?}")]
    CreatureNotTagged {
        organism_id: OrganismId,
        reserve_id: HabitatId,
    },
    #[error("{organism_id:?} is quarantined until {until:?}")]
    QuarantinedUntil {
        organism_id: OrganismId,
        until: Tick,
    },
    #[error("invalid actor {actor:?} for {context}")]
    InvalidActor {
        actor: HabitatActor,
        context: &'static str,
    },
    #[error("{authority:?} cannot transfer between {prior_mode:?} and {new_mode:?} habitats")]
    IllegalTransferAuthority {
        prior_mode: HabitatMode,
        new_mode: HabitatMode,
        authority: HabitatAuthorityKind,
    },
}

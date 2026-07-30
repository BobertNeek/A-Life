//! Production habitat-lab command producers shared by app input and evidence gates.

use alife_core::OrganismId;
use alife_world::{
    HabitatActor, HabitatAuthorityError, HabitatBreedingKind, HabitatBreedingReceipt,
    HabitatBreedingRequest, HabitatId, HeadlessWorld,
};

pub fn produce_habitat_lab_explicit_breed_receipt(
    world: &HeadlessWorld,
    first_parent: OrganismId,
    habitat_id: HabitatId,
    second_parent: OrganismId,
) -> Result<HabitatBreedingReceipt, HabitatAuthorityError> {
    world
        .habitat_authority()
        .authorize_breeding(HabitatBreedingRequest {
            habitat_id,
            first_parent,
            second_parent,
            kind: HabitatBreedingKind::Explicit,
            actor: HabitatActor::Player,
            tick: world.tick(),
        })
}

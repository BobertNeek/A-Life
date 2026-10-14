//! Physical examples within shared categories. These recipes are construction
//! data, never semantic labels or desirability scores supplied to the brain.
use crate::{GroundedPhysicalProperties, HeadlessWorld, WorldEditorSpawnSpec, WorldObjectKind};
use alife_core::{ScaffoldContractError, Vec3f, WorldEntityId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoodVariety {
    Root,
    Fruit,
    Seed,
}

impl FoodVariety {
    pub const fn from_seed(seed: u64) -> Self {
        match seed % 3 {
            0 => Self::Root,
            1 => Self::Fruit,
            _ => Self::Seed,
        }
    }
    pub const fn nutrition(self) -> f32 {
        match self {
            Self::Root => 0.65,
            Self::Fruit => 0.45,
            Self::Seed => 0.85,
        }
    }
    pub fn physical(self, mut physical: GroundedPhysicalProperties) -> GroundedPhysicalProperties {
        let variation = (physical.color[0] - 0.5) * 0.08;
        let (color, shape, chemical) = match self {
            Self::Root => ([0.95, 0.38, 0.08], [0.45, 0.9, 0.45], [0.8, 0.25, 0.35]),
            Self::Fruit => ([0.85, 0.12, 0.08], [0.85, 0.8, 0.85], [0.85, 0.65, 0.8]),
            Self::Seed => ([0.65, 0.5, 0.22], [0.55, 0.35, 0.45], [0.75, 0.1, 0.15]),
        };
        physical.color = color.map(|v: f32| (v + variation).clamp(0.0, 1.0));
        physical.shape = shape;
        physical.material = [0.35, 0.15, 0.65];
        physical.chemical = chemical;
        physical
    }
}

impl HeadlessWorld {
    /// Saved objects keep the resulting values; loading never reruns recipes.
    pub fn set_food_variety(
        &mut self,
        id: WorldEntityId,
        variety: FoodVariety,
    ) -> Result<(), ScaffoldContractError> {
        let object = self.entity(id).ok_or(ScaffoldContractError::InvalidId)?;
        if object.kind != WorldObjectKind::Food || object.consumed || object.carried_by.is_some() {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        let physical = variety.physical(object.grounded_physical);
        self.set_grounded_physical_properties(id, physical)?;
        self.set_food_recipe_nutrition(id, variety.nutrition())
    }

    pub fn spawn_toy(
        &mut self,
        label: &str,
        position: Vec3f,
        movable: bool,
    ) -> Result<WorldEntityId, ScaffoldContractError> {
        self.editor_spawn_object(WorldEditorSpawnSpec {
            label: label.into(),
            kind: if movable {
                WorldObjectKind::Ball
            } else {
                WorldObjectKind::ActivityToy
            },
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: if movable { 0.3 } else { 0.5 },
            token_id: None,
        })
    }
}

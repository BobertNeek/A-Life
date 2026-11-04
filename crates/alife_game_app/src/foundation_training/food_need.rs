//! Prospective scene preparation, never a biological or reward override.
use super::{FoundationTeacherLesson, Result};
use alife_core::OrganismId;
use alife_world::HeadlessWorld;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationFoodNeedCondition {
    #[default]
    NewbornV1,
    OrdinaryHealthyAgingV1,
    /// Explicit obstacle preparation; keeps the original newborn lesson available.
    OrdinaryHealthyObstacleV1,
}

impl std::str::FromStr for FoundationFoodNeedCondition {
    type Err = &'static str;
    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "newborn_v1" => Ok(Self::NewbornV1),
            "ordinary_healthy_aging_v1" => Ok(Self::OrdinaryHealthyAgingV1),
            "ordinary_healthy_obstacle_v1" => Ok(Self::OrdinaryHealthyObstacleV1),
            _ => Err("food need condition must be newborn_v1, ordinary_healthy_aging_v1 or ordinary_healthy_obstacle_v1"),
        }
    }
}

impl FoundationFoodNeedCondition {
    #[cfg(feature = "cpu-training")]
    pub(crate) fn is_newborn(&self) -> bool {
        *self == Self::NewbornV1
    }
    pub(crate) fn validate_lesson(self, lesson: Option<FoundationTeacherLesson>) -> Result<()> {
        match self {
            Self::NewbornV1 => {}
            Self::OrdinaryHealthyAgingV1
                if matches!(
                    lesson,
                    Some(FoundationTeacherLesson::Feeding | FoundationTeacherLesson::RelocatedFood)
                ) => {}
            Self::OrdinaryHealthyObstacleV1
                if lesson == Some(FoundationTeacherLesson::ObstacleNavigation) => {}
            Self::OrdinaryHealthyAgingV1 => {
                return Err("ordinary healthy food aging requires Feeding or RelocatedFood".into())
            }
            Self::OrdinaryHealthyObstacleV1 => {
                return Err("ordinary healthy obstacle aging requires ObstacleNavigation".into())
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationFoodNeedInitialization {
    pub condition: FoundationFoodNeedCondition,
    pub organism: OrganismId,
    pub before: FoodNeedState,
    pub prepared: FoodNeedState,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoodNeedState {
    pub tick: u64,
    pub energy: f32,
    pub hunger: f32,
    pub health: f32,
    pub injury: f32,
    pub pain: f32,
    pub sleeping: bool,
}

fn state(world: &HeadlessWorld, organism: OrganismId) -> Result<FoodNeedState> {
    let bio = world
        .organism_registry()
        .get(organism)
        .ok_or("food need organism missing")?
        .biochemistry();
    Ok(FoodNeedState {
        tick: world.tick().raw(),
        energy: bio.body.energy,
        hunger: bio.homeostasis.drives.hunger,
        health: bio.body.health,
        injury: bio.body.injury,
        pain: bio.homeostasis.drives.pain,
        sleeping: bio.body.sleeping,
    })
}

/// Shared with held/reachable setup: same ordinary ticks and admission bounds.
pub(super) fn advance_healthy_need(world: &mut HeadlessWorld, organism: OrganismId) -> Result<()> {
    for _ in 0..2400 {
        let current = state(world, organism)?;
        if current.hunger >= 0.12 && current.energy <= 0.9 {
            break;
        }
        world.try_advance_tick()?;
    }
    let current = state(world, organism)?;
    validate_ready(&current)
}

fn validate_ready(current: &FoodNeedState) -> Result<()> {
    if [
        current.energy,
        current.hunger,
        current.health,
        current.injury,
        current.pain,
    ]
    .iter()
    .any(|value| !value.is_finite())
        || current.sleeping
        || current.health < 0.95
        || current.injury > 0.01
        || current.pain > 0.01
        || current.energy <= 0.2
        || current.energy > 0.9
        || current.hunger < 0.12
    {
        return Err("food lesson requires a healthy awake organism with ordinary measurable hunger and reserve headroom".into());
    }
    Ok(())
}

pub(super) fn initialize(
    world: &mut HeadlessWorld,
    condition: FoundationFoodNeedCondition,
) -> Result<FoundationFoodNeedInitialization> {
    let organism = world
        .organism_entity_ids()
        .first()
        .ok_or("food need organism missing")?
        .0;
    let before = state(world, organism)?;
    if condition != FoundationFoodNeedCondition::NewbornV1 {
        advance_healthy_need(world, organism)?;
    }
    Ok(FoundationFoodNeedInitialization {
        condition,
        organism,
        before,
        prepared: state(world, organism)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation_training::{
        configure_foundation_scenario, configure_foundation_scenario_with_food_need,
        held_food_lesson_tests::unconfigured, relocated_food::RelocatedFoodCollector,
    };

    #[test]
    fn newborn_default_preserves_world_and_aging_is_only_ordinary_ticks() {
        for lesson in [
            FoundationTeacherLesson::Feeding,
            FoundationTeacherLesson::RelocatedFood,
        ] {
            for seed in [42, 100, 519] {
                let original = unconfigured(seed);
                let mut baseline = original.clone();
                let old =
                    configure_foundation_scenario(&mut baseline, seed, Some(lesson), None, true)
                        .unwrap();
                let mut newborn = original.clone();
                let default = configure_foundation_scenario_with_food_need(
                    &mut newborn,
                    seed,
                    Some(lesson),
                    None,
                    true,
                    FoundationFoodNeedCondition::default(),
                )
                .unwrap();
                assert_eq!(
                    baseline.canonical_signature_digest().unwrap(),
                    newborn.canonical_signature_digest().unwrap()
                );
                let receipt = default.food_need_initialization.unwrap();
                assert_eq!(receipt.before, receipt.prepared);
                assert_eq!(receipt.condition, FoundationFoodNeedCondition::NewbornV1);
                assert_eq!(receipt.before.hunger, 0.0);
                assert!((receipt.before.energy - 0.7925).abs() < 0.00001);
                let body = baseline.organism_entity_ids()[0].1;
                let initial_body = baseline.entity(body).unwrap().clone();
                let initial_food = baseline.entity(old.food).unwrap().clone();

                let mut aged = original.clone();
                let setup = configure_foundation_scenario_with_food_need(
                    &mut aged,
                    seed,
                    Some(lesson),
                    None,
                    true,
                    FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
                )
                .unwrap();
                let receipt = setup.food_need_initialization.as_ref().unwrap();
                assert!(receipt.prepared.tick > receipt.before.tick);
                assert!(receipt.prepared.tick - receipt.before.tick <= 2400);
                assert_eq!(setup.start_tick, receipt.prepared.tick);
                assert!(receipt.prepared.hunger >= 0.12);
                assert!((0.2..=0.9).contains(&receipt.prepared.energy));
                assert!(receipt.prepared.health >= 0.95);
                assert!(receipt.prepared.injury <= 0.01 && receipt.prepared.pain <= 0.01);
                assert!(!receipt.prepared.sleeping);
                assert_eq!(aged.entity(body).unwrap().position, initial_body.position);
                assert_eq!(aged.entity(body).unwrap().body_yaw, initial_body.body_yaw);
                let food = aged.entity(setup.food).unwrap();
                assert_eq!(food.position, initial_food.position);
                assert_eq!(food.carried_by, None);
                assert!(!food.consumed);
                assert_eq!(food.nutrition, initial_food.nutrition);
                assert_eq!(setup.teacher_cue_tokens, old.teacher_cue_tokens);
                assert_eq!(setup.silent_request, old.silent_request);
                assert_eq!(
                    setup.training_language_feedback,
                    old.training_language_feedback
                );
                assert!(
                    setup.held_food_setup.is_none()
                        && setup.reachable_food_setup.is_none()
                        && setup.grab_food_setup.is_none()
                );
                while baseline.tick().raw() < receipt.prepared.tick {
                    baseline.try_advance_tick().unwrap();
                }
                // Entire canonical world, including biology and retained genetic
                // state, must equal ordinary advancement of the old scene.
                assert_eq!(
                    baseline.canonical_signature_digest().unwrap(),
                    aged.canonical_signature_digest().unwrap()
                );
                let encoded = serde_json::to_vec(receipt).unwrap();
                assert_eq!(
                    *receipt,
                    serde_json::from_slice::<FoundationFoodNeedInitialization>(&encoded).unwrap()
                );
            }
        }
    }

    #[test]
    fn aged_relocation_still_waits_two_collected_world_ticks() {
        let mut world = unconfigured(42);
        let setup = configure_foundation_scenario_with_food_need(
            &mut world,
            42,
            Some(FoundationTeacherLesson::RelocatedFood),
            None,
            true,
            FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
        )
        .unwrap();
        let mut collector = RelocatedFoodCollector::new(&world, &setup, 42).unwrap();
        let initial = world.entity(setup.food).unwrap().position;
        for _ in 0..2 {
            collector.before_step(&mut world).unwrap();
            assert_eq!(world.entity(setup.food).unwrap().position, initial);
            world.try_advance_tick().unwrap();
        }
        collector.before_step(&mut world).unwrap();
        assert_ne!(world.entity(setup.food).unwrap().position, initial);
        assert_eq!(world.tick().raw(), setup.start_tick + 2);
    }

    #[test]
    fn healthy_need_admission_rejects_unsafe_or_unmeasurable_states() {
        let valid = FoodNeedState {
            tick: 1,
            energy: 0.7,
            hunger: 0.12,
            health: 1.0,
            injury: 0.0,
            pain: 0.0,
            sleeping: false,
        };
        validate_ready(&valid).unwrap();
        for invalid in [
            FoodNeedState {
                energy: 0.2,
                ..valid.clone()
            },
            FoodNeedState {
                energy: 0.90001,
                ..valid.clone()
            },
            FoodNeedState {
                hunger: 0.11999,
                ..valid.clone()
            },
            FoodNeedState {
                health: 0.94999,
                ..valid.clone()
            },
            FoodNeedState {
                injury: 0.01001,
                ..valid.clone()
            },
            FoodNeedState {
                pain: 0.01001,
                ..valid.clone()
            },
            FoodNeedState {
                sleeping: true,
                ..valid.clone()
            },
            FoodNeedState {
                hunger: f32::NAN,
                ..valid.clone()
            },
        ] {
            assert!(validate_ready(&invalid).is_err());
        }
    }

    #[test]
    fn aged_obstacle_is_ordinary_biology_and_gate_waits_for_prepared_decisions() {
        use crate::foundation_training::close_foundation_navigation_gate_in_world;
        for seed in [42, 100, 519] {
            let original = unconfigured(seed);
            let mut baseline = original.clone();
            let old = configure_foundation_scenario(
                &mut baseline,
                seed,
                Some(FoundationTeacherLesson::ObstacleNavigation),
                None,
                true,
            )
            .unwrap();
            let gate = old.closing_gate.unwrap();
            let open_gate = baseline.entity(gate.0).unwrap().position;
            let original_food = baseline.entity(old.food).unwrap().clone();
            let mut aged = original.clone();
            let setup = configure_foundation_scenario_with_food_need(
                &mut aged,
                seed,
                Some(FoundationTeacherLesson::ObstacleNavigation),
                None,
                true,
                FoundationFoodNeedCondition::OrdinaryHealthyObstacleV1,
            )
            .unwrap();
            let initialization = setup.food_need_initialization.as_ref().unwrap();
            assert!(initialization.prepared.tick > initialization.before.tick);
            assert!(initialization.prepared.hunger >= 0.12);
            assert!(initialization.prepared.health >= 0.95);
            assert_eq!(setup.start_tick, initialization.prepared.tick);
            assert_eq!(setup.closing_gate, old.closing_gate);
            assert_eq!(setup.teacher_cue_tokens, old.teacher_cue_tokens);
            assert_eq!(setup.silent_request, old.silent_request);
            assert_eq!(
                setup.training_language_feedback,
                old.training_language_feedback
            );
            assert_eq!(
                aged.entity(setup.food).unwrap().position,
                original_food.position
            );
            assert_eq!(
                aged.entity(setup.food).unwrap().nutrition,
                original_food.nutrition
            );
            while baseline.tick().raw() < initialization.prepared.tick {
                baseline.try_advance_tick().unwrap();
            }
            assert_eq!(
                baseline.canonical_signature_digest().unwrap(),
                aged.canonical_signature_digest().unwrap()
            );
            // A literal legacy absolute-clock call would close the aged gate
            // immediately. The real helper now leaves two prepared decisions.
            assert!(aged.tick().raw() >= 2);
            let mut legacy_aged = aged.clone();
            crate::foundation_training::move_scenario_object(&mut legacy_aged, gate.0, gate.1)
                .unwrap();
            assert_eq!(legacy_aged.entity(gate.0).unwrap().position, gate.1);
            assert_ne!(aged.entity(gate.0).unwrap().position, gate.1);
            for elapsed in 0..=3 {
                close_foundation_navigation_gate_in_world(&mut aged, &setup).unwrap();
                assert_eq!(aged.tick().raw(), setup.start_tick + elapsed);
                assert_eq!(
                    aged.entity(gate.0).unwrap().position,
                    if elapsed < 2 { open_gate } else { gate.1 }
                );
                aged.try_advance_tick().unwrap();
            }
            // The original newborn absolute-clock behavior remains identical.
            let mut newborn = original.clone();
            let control = configure_foundation_scenario_with_food_need(
                &mut newborn,
                seed,
                Some(FoundationTeacherLesson::ObstacleNavigation),
                None,
                true,
                FoundationFoodNeedCondition::NewbornV1,
            )
            .unwrap();
            assert_eq!(control.start_tick, 0);
            let mut legacy = newborn.clone();
            for _ in 0..=3 {
                if legacy.tick().raw() >= 2 {
                    let (id, position) = control.closing_gate.unwrap();
                    if legacy.entity(id).unwrap().position != position {
                        crate::foundation_training::move_scenario_object(&mut legacy, id, position)
                            .unwrap();
                    }
                }
                close_foundation_navigation_gate_in_world(&mut newborn, &control).unwrap();
                assert_eq!(
                    newborn.canonical_signature_digest().unwrap(),
                    legacy.canonical_signature_digest().unwrap()
                );
                newborn.try_advance_tick().unwrap();
                legacy.try_advance_tick().unwrap();
            }
        }
    }

    #[test]
    fn aged_obstacle_requires_its_explicit_version_and_correct_lesson() {
        let condition: FoundationFoodNeedCondition =
            "ordinary_healthy_obstacle_v1".parse().unwrap();
        assert_eq!(
            condition,
            FoundationFoodNeedCondition::OrdinaryHealthyObstacleV1
        );
        assert_eq!(
            serde_json::from_value::<FoundationFoodNeedCondition>(serde_json::json!(
                "ordinary_healthy_obstacle_v1"
            ))
            .unwrap(),
            condition
        );
        condition
            .validate_lesson(Some(FoundationTeacherLesson::ObstacleNavigation))
            .unwrap();
        FoundationFoodNeedCondition::OrdinaryHealthyAgingV1
            .validate_lesson(Some(FoundationTeacherLesson::ObstacleNavigation))
            .unwrap_err();
        for lesson in [
            None,
            Some(FoundationTeacherLesson::Feeding),
            Some(FoundationTeacherLesson::RelocatedFood),
            Some(FoundationTeacherLesson::EatHeldFood),
        ] {
            let mut world = unconfigured(42);
            let before = world.canonical_signature_digest().unwrap();
            assert!(configure_foundation_scenario_with_food_need(
                &mut world, 42, lesson, None, true, condition
            )
            .is_err());
            assert_eq!(world.canonical_signature_digest().unwrap(), before);
        }
    }

    #[test]
    fn unsupported_need_condition_or_lesson_fails_before_setup() {
        assert!("injected_hunger"
            .parse::<FoundationFoodNeedCondition>()
            .is_err());
        for lesson in [
            None,
            Some(FoundationTeacherLesson::EatHeldFood),
            Some(FoundationTeacherLesson::EatReachableFood),
            Some(FoundationTeacherLesson::VisionSearch),
        ] {
            let mut world = unconfigured(42);
            let before = world.canonical_signature_digest().unwrap();
            assert!(configure_foundation_scenario_with_food_need(
                &mut world,
                42,
                lesson,
                None,
                true,
                FoundationFoodNeedCondition::OrdinaryHealthyAgingV1
            )
            .is_err());
            assert_eq!(world.canonical_signature_digest().unwrap(), before);
        }
    }
}

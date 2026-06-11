use std::collections::HashSet;

use serde::{de::DeserializeOwned, Serialize};

use alife_core::{
    validate_optional_target, Aabb, ActionId, BrainClassId, Confidence, CoreFromAdapter,
    CoreIntoAdapter, CreatureId, DurationTicks, ExperienceSequenceId, FixedPointScale,
    GaussianClusterId, Intensity, MemoryId, NormalizedScalar, OrganismId, Pose, Quatf, Seconds,
    SignedValence, Tick, Vec2f, Vec3f, Velocity, WorldEntityId, WorldEntityIdMapper,
};

fn assert_serde<T: Serialize + DeserializeOwned>() {}

#[test]
fn stable_ids_are_hashable_validated_and_serde_ready() {
    assert_serde::<CreatureId>();
    assert_serde::<WorldEntityId>();
    assert_serde::<GaussianClusterId>();
    assert_serde::<MemoryId>();

    let creature = CreatureId::new(7).unwrap();
    let organism: OrganismId = creature.into();
    let creature_again: CreatureId = organism.into();

    assert_eq!(creature, creature_again);
    assert!(BrainClassId::new(1).unwrap().is_valid());
    assert!(ActionId::new(9).unwrap().is_valid());
    assert!(ExperienceSequenceId::new(2).unwrap().is_valid());
    assert!(CreatureId::new(0).is_none());

    let mut set = HashSet::new();
    set.insert(creature);
    assert!(set.contains(&CreatureId(7)));
}

#[test]
fn math_primitives_reject_nan_and_invalid_bounds() {
    assert_eq!(Vec2f::new(1.0, 2.0).validate().unwrap().x, 1.0);
    assert_eq!(
        Vec3f::from_array([1.0, 2.0, 3.0]).to_array(),
        [1.0, 2.0, 3.0]
    );
    assert!(Vec3f::new(f32::NAN, 0.0, 0.0).validate().is_err());
    assert!(Quatf::IDENTITY.validate().is_ok());
    assert!(Quatf::new(0.0, 0.0, 0.0, 0.0).validate().is_err());

    let bounds = Aabb::new(Vec3f::ZERO, Vec3f::new(1.0, 1.0, 1.0)).unwrap();
    assert_eq!(bounds.max.x, 1.0);
    assert!(Aabb::new(Vec3f::new(2.0, 0.0, 0.0), Vec3f::new(1.0, 1.0, 1.0)).is_err());

    assert!(Pose::IDENTITY.validate().is_ok());
    assert!(Velocity::ZERO.validate().is_ok());
}

#[test]
fn bounded_units_reject_out_of_range_values() {
    assert_eq!(Tick::validate_monotonic(Tick(4), Tick(4)).unwrap(), Tick(4));
    assert!(Tick::validate_monotonic(Tick(4), Tick(3)).is_err());
    assert_eq!(DurationTicks::new(12).raw(), 12);
    assert_eq!(Seconds::new(0.5).unwrap().raw(), 0.5);
    assert!(Seconds::new(-0.1).is_err());

    assert_eq!(NormalizedScalar::new(1.0).unwrap().raw(), 1.0);
    assert!(NormalizedScalar::new(1.1).is_err());
    assert_eq!(SignedValence::new(-1.0).unwrap().raw(), -1.0);
    assert!(SignedValence::new(-1.1).is_err());
    assert_eq!(Confidence::new(0.75).unwrap().raw(), 0.75);
    assert!(Confidence::new(f32::INFINITY).is_err());
    assert_eq!(Intensity::new(0.25).unwrap().raw(), 0.25);
    assert_eq!(FixedPointScale::Q8_8.fractional_bits, 8);
    assert!(FixedPointScale::new(31).is_err());
}

#[test]
fn optional_targets_and_adapter_traits_stay_outside_engine_types() {
    assert_eq!(
        validate_optional_target(Some(WorldEntityId(55))).unwrap(),
        Some(WorldEntityId(55))
    );
    assert!(validate_optional_target(Some(WorldEntityId::INVALID)).is_err());
    assert_eq!(validate_optional_target(None).unwrap(), None);

    struct AdapterEntity(u64);
    struct Mapper;

    impl WorldEntityIdMapper<AdapterEntity> for Mapper {
        fn to_world_entity_id(&self, entity: AdapterEntity) -> WorldEntityId {
            WorldEntityId(entity.0)
        }
    }

    struct AdapterVec3([f32; 3]);

    impl CoreFromAdapter<Vec3f> for AdapterVec3 {
        fn core_from_adapter(value: Vec3f) -> Self {
            Self(value.to_array())
        }
    }

    impl CoreIntoAdapter<Vec3f> for AdapterVec3 {
        fn core_into_adapter(self) -> Vec3f {
            Vec3f::from_array(self.0)
        }
    }

    assert_eq!(
        Mapper.to_world_entity_id(AdapterEntity(99)),
        WorldEntityId(99)
    );
    let adapter_vec = AdapterVec3::core_from_adapter(Vec3f::new(1.0, 2.0, 3.0));
    assert_eq!(adapter_vec.core_into_adapter().to_array(), [1.0, 2.0, 3.0]);
}

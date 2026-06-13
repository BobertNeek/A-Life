use alife_bevy_adapter::{
    ActionSink, AffordanceTags, AlifeBevyAdapterPlugin, CreatureBody, SensoryEmitter,
};
use alife_core::{AffordanceBits, OrganismId, WorldEntityId};
use bevy::prelude::{App, MinimalPlugins, Transform, Vec3};

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AlifeBevyAdapterPlugin);

    app.world_mut().spawn((
        CreatureBody::new(OrganismId(1), WorldEntityId(1)).expect("valid creature body"),
        Transform::from_translation(Vec3::ZERO),
        ActionSink::default(),
        SensoryEmitter::default(),
    ));
    app.world_mut().spawn((
        AffordanceTags::food(0.5),
        SensoryEmitter::default(),
        Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
    ));
    app.world_mut().spawn((
        AffordanceTags::new(AffordanceBits::GLYPH_OR_WRITING),
        SensoryEmitter {
            audible_token: Some(42),
            ..SensoryEmitter::default()
        },
        Transform::from_translation(Vec3::new(2.0, 0.0, 0.0)),
    ));

    app.update();
}

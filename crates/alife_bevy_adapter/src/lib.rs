//! v0 scaffold: Bevy adapter boundary, not cognitive runtime logic.

use bevy::prelude::{App, Plugin};

#[derive(Debug, Default)]
pub struct AlifeBevyAdapterPlugin;

impl Plugin for AlifeBevyAdapterPlugin {
    fn build(&self, _app: &mut App) {}
}

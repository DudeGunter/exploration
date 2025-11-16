use bevy::prelude::*;

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        info!("NetworkingPlugin initialized");
    }
}

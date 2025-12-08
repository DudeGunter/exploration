use bevy::prelude::*;
use noiz::prelude::*;

mod collider;

pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(collider::on_collider_ready);
    }
}

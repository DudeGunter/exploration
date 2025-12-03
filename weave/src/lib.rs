// This crate is meant to represent the composition of the world
// The *weave* so to speak, currently there shuold be voxel and marching
use bevy::prelude::*;

mod marching_cubes;
mod voxel;

/// Adds all weave implementations
/// This includes voxel and marching and their respective terrains
pub struct WeavePlugin;

impl Plugin for WeavePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((marching_cubes::MarchingCubesPlugin, voxel::VoxelPlugin));
    }
}

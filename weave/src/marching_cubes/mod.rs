use bevy::prelude::*;
use noiz::prelude::*;

mod tables;

pub struct MarchingCubesPlugin;

impl Plugin for MarchingCubesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MarchingCubesNoise>();
    }
}

pub type MarchingCubesNoise = crate::noise::TerrainNoise<common_noise::Perlin>;

impl Default for MarchingCubesNoise {
    fn default() -> Self {
        Self(Noise::from(common_noise::Perlin::default()))
    }
}

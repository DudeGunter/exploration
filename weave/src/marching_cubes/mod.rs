use crate::terrain::*;
use bevy::prelude::*;

mod mesh;
mod tables;

pub struct MarchingCubesPlugin;

impl Plugin for MarchingCubesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TerrainNoisePlugin(NoiseParams::default()));
        app.add_systems(Startup, request_area);
        app.add_observer(mesh::recieve_mesh);
    }
}

#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct NoiseParams {
    pub scale: f32,
    pub frequency: f32,
    pub amplitude: f32,
    pub octaves: u32,
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            scale: 1.0,
            frequency: 1.0,
            amplitude: 1.0,
            octaves: 1,
        }
    }
}

impl TerrainNoiseParams for NoiseParams {
    fn frequency(&self) -> f32 {
        self.frequency
    }
    fn amplitude(&self) -> f32 {
        self.amplitude
    }
    fn scale(&self) -> f32 {
        self.scale
    }
    fn octaves(&self) -> u32 {
        self.octaves
    }
}

pub fn request_area(mut commands: Commands) {
    commands.trigger(RequestNoise::<NoiseParams>::new(IVec2::ZERO))
}

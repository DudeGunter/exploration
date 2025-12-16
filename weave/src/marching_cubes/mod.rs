use crate::terrain::*;
use bevy::prelude::*;

mod mesh;
mod tables;

pub struct MarchingCubesPlugin;

impl Plugin for MarchingCubesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TerrainNoisePlugin);
        app.add_systems(Update, request_area);
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
            scale: 0.1,
            frequency: 1.0,
            amplitude: 1.0,
            octaves: 1,
        }
    }
}

pub fn request_area(mut commands: Commands, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::KeyG) {
        for x in -3..3 {
            for y in -3..3 {
                for z in -3..3 {
                    commands.trigger(RequestNoise {
                        position: IVec3::new(x, y, z),
                    });
                }
            }
        }
    }
}

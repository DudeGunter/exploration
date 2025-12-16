pub use bevy::{asset::embedded_asset, prelude::*};
pub use bevy_app_compute::prelude::*;
use bytemuck::{Pod, Zeroable};

pub const FIELD_SIZE: u32 = 17;
pub const WORKGROUP_SIZE: u32 = 4;

pub struct FieldComputePlugin;

impl Plugin for FieldComputePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "noise_field.wgsl");
        app.add_plugins((
            AppComputePlugin,
            AppComputeWorkerPlugin::<FieldComputeWorker>::default(),
        ));
    }
}

#[derive(TypePath)]
pub struct NoiseFieldShader;

impl ComputeShader for NoiseFieldShader {
    fn shader() -> ShaderRef {
        "embedded://weave/terrain/noise_field.wgsl".into()
    }
}

#[derive(Resource)]
pub struct FieldComputeWorker;

impl ComputeWorker for FieldComputeWorker {
    fn build(world: &mut World) -> AppComputeWorker<Self> {
        let worker = AppComputeWorkerBuilder::new(world)
            .add_empty_staging("params", std::mem::size_of::<NoiseParams>() as u64)
            .add_empty_staging(
                "noise_field",
                4 * (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE) as u64,
            ) // 4 is the size of a float
            .add_pass::<NoiseFieldShader>(
                [WORKGROUP_SIZE, WORKGROUP_SIZE, WORKGROUP_SIZE],
                &["params", "noise_field"],
            )
            .one_shot()
            .build();

        worker
    }
}

#[repr(C)]
#[derive(ShaderType, Clone, Copy, Pod, Zeroable)]
pub struct NoiseParams {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub chunk_z: i32,
    pub scale: f32,
    pub frequency: f32,
    pub amplitude: f32,
    pub octaves: u32,
    pub _padding: u32,
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            chunk_x: 0,
            chunk_y: 0,
            chunk_z: 0,
            scale: 1.0,
            frequency: 0.1,
            amplitude: 1.0,
            octaves: 3,
            _padding: 0,
        }
    }
}

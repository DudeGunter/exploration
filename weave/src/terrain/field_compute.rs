use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::{
        render_resource::*,
        renderer::RenderDevice,
        {Render, RenderApp, RenderSystems},
    },
};
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;

use crate::area::RequestChunk;

const CHUNK_SIZE: u32 = 16;
const FIELD_SIZE: u32 = CHUNK_SIZE + 1;
const WORKGROUP_SIZE: u32 = 4;

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

#[derive(Clone)]
pub struct NoiseField {
    pub chunk_coord: IVec3,
    pub values: Vec<f32>,
}

impl NoiseField {
    pub fn new(chunk_coord: IVec3) -> Self {
        Self {
            chunk_coord,
            values: vec![0.0; (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE) as usize],
        }
    }

    fn index(x: u32, y: u32, z: u32) -> usize {
        (x + y * FIELD_SIZE + z * FIELD_SIZE * FIELD_SIZE) as usize
    }

    pub fn get(&self, x: u32, y: u32, z: u32) -> f32 {
        if x >= FIELD_SIZE || y >= FIELD_SIZE || z >= FIELD_SIZE {
            return 0.0;
        }
        self.values[Self::index(x, y, z)]
    }

    pub fn set(&mut self, x: u32, y: u32, z: u32, value: f32) {
        if x < FIELD_SIZE && y < FIELD_SIZE && z < FIELD_SIZE {
            self.values[Self::index(x, y, z)] = value;
        }
    }
}

#[derive(Event)]
pub struct NoiseFieldReady {
    pub field: NoiseField,
    pub params: NoiseParams,
}

#[derive(Resource, Default)]
pub struct NoiseFieldQueue {
    pub pending: Vec<(IVec3, NoiseParams)>,
    pub in_flight: std::collections::HashMap<IVec3, NoiseField>,
}

pub fn dispatch_noise_field_compute(mut queue: ResMut<NoiseFieldQueue>, device: Res<RenderDevice>) {
    let pending: Vec<_> = queue.pending.drain(..).collect();

    for (chunk_coord, params) in pending {
        let field = NoiseField::new(chunk_coord);

        let _params_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("noise_params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let _storage_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("noise_field_storage"),
            size: (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        queue.in_flight.insert(chunk_coord, field);

        // TODO: Enqueue actual compute dispatch using render graph node
    }
}

pub fn readback_noise_field(mut queue: ResMut<NoiseFieldQueue>, mut commands: Commands) {
    for (coord, field) in queue.in_flight.drain() {
        let params = NoiseParams {
            chunk_x: coord.x,
            chunk_y: coord.y,
            chunk_z: coord.z,
            ..Default::default()
        };

        commands.trigger(NoiseFieldReady { field, params });
    }
}

// ============================================================================
// RENDER WORLD PIPELINE
// ============================================================================

#[derive(Resource)]
pub struct NoiseFieldComputePipeline {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline_id: CachedComputePipelineId,
}

fn init_noise_field_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let bind_group_layout = render_device.create_bind_group_layout(
        "noise_field_bind_group_layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        std::num::NonZeroU64::new(std::mem::size_of::<NoiseParams>() as u64)
                            .unwrap(),
                    ),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        std::num::NonZeroU64::new(
                            (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE * 4) as u64,
                        )
                        .unwrap(),
                    ),
                },
                count: None,
            },
        ],
    );

    let pipeline_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("noise_field_compute_pipeline".into()),
        layout: vec![bind_group_layout.clone()],
        push_constant_ranges: vec![],
        shader: asset_server.load("embedded://weave/terrain/noise_field.wgsl"),
        shader_defs: vec![],
        entry_point: Some(Cow::from("main")),
        ..default()
    });

    commands.insert_resource(NoiseFieldComputePipeline {
        bind_group_layout,
        pipeline_id,
    });
}

pub struct NoiseFieldComputePlugin;

impl Plugin for NoiseFieldComputePlugin {
    fn build(&self, app: &mut App) {
        // Register embedded shader
        embedded_asset!(app, "noise_field.wgsl");

        app.init_resource::<NoiseFieldQueue>().add_systems(
            Update,
            (dispatch_noise_field_compute, readback_noise_field).chain(),
        );

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app.add_systems(
            Render,
            init_noise_field_pipeline.in_set(RenderSystems::Prepare),
        );
    }
}

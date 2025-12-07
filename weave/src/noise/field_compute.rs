use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bytemuck::{Pod, Zeroable};

use crate::area::RequestChunk;

const CHUNK_SIZE: u32 = 16;
const FIELD_SIZE: u32 = CHUNK_SIZE + 1; // +1 for marching cubes interpolation

#[derive(Resource)]
pub struct NoiseFieldCompute {
    pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
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

/// Stores generated noise field for a chunk
#[derive(Clone)]
pub struct NoiseField {
    pub chunk_coord: IVec3,
    pub values: Vec<f32>, // FIELD_SIZE^3 values
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

pub fn setup_noise_field_compute(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
) {
    let bind_group_layout = device.create_bind_group_layout(
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
        shader: asset_server.load("shaders/noise_field.wgsl"),
        shader_defs: vec![],
        ..default()
    });

    commands.insert_resource(NoiseFieldCompute {
        pipeline: pipeline_id,
        bind_group_layout,
    });
}

#[derive(Resource, Default)]
pub struct NoiseFieldQueue {
    pub pending: Vec<(IVec3, NoiseParams)>,
    pub in_flight: std::collections::HashMap<IVec3, NoiseField>,
}

pub fn queue_noise_field_request(trigger: On<RequestChunk>, mut queue: ResMut<NoiseFieldQueue>) {
    // This would be populated by your area manager or generation system
    // For now, it's a placeholder for where requests come in
}

pub fn dispatch_noise_field_compute(
    mut queue: ResMut<NoiseFieldQueue>,
    compute: Res<NoiseFieldCompute>,
    pipeline_cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    queue_res: Res<RenderQueue>,
    mut commands: Commands,
) {
    let pending: Vec<_> = queue.pending.drain(..).collect();

    for (chunk_coord, params) in pending {
        let field = NoiseField::new(chunk_coord);

        // Create buffers
        let params_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("noise_params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let storage_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("noise_field_storage"),
            size: (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(
            "noise_field_bind_group",
            &compute.bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: storage_buffer.as_entire_binding(),
                },
            ],
        );

        // Queue for readback
        queue.in_flight.insert(chunk_coord, field);

        // TODO: Enqueue actual compute dispatch in render world
        // This requires a render graph node or render command
        // For now, placeholder
    }
}

pub fn readback_noise_field(mut queue: ResMut<NoiseFieldQueue>, mut commands: Commands) {
    // After compute shader finishes, read buffer back to CPU
    // Trigger NoiseFieldReady event with the data

    for (coord, mut field) in queue.in_flight.drain() {
        // In real implementation:
        // 1. Check if compute is done
        // 2. Map GPU buffer to CPU
        // 3. Copy data into field.values
        // 4. Trigger event

        let params = NoiseParams {
            chunk_x: coord.x,
            chunk_y: coord.y,
            chunk_z: coord.z,
            ..Default::default()
        };

        commands.trigger(NoiseFieldReady { field, params });
    }
}

pub struct NoiseFieldComputePlugin;

impl Plugin for NoiseFieldComputePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NoiseFieldQueue>();

        // Setup happens in render world
        // app.add_systems(Startup, setup_noise_field_compute);
        app.add_observer(queue_noise_field_request);
        app.add_systems(
            Update,
            (dispatch_noise_field_compute, readback_noise_field).chain(),
        );
    }
}

use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
    },
};
use bytemuck::{Pod, Zeroable};
use std::{
    borrow::Cow,
    sync::{Arc, atomic::AtomicBool},
};

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

#[derive(Resource, Default, Clone, ExtractResource)]
pub struct NoiseFieldQueue {
    pub pending: Vec<(IVec3, NoiseParams)>,
    pub pending_readback: Vec<NoiseComputeDispatch>,
    pub ready: Vec<NoiseField>,
}

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
    info!("Initializing noise field compute pipeline");
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

    let shader = asset_server.load("embedded://weave/terrain/noise_field.wgsl");

    let pipeline_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("noise_field_compute_pipeline".into()),
        layout: vec![bind_group_layout.clone()],
        push_constant_ranges: vec![],
        shader,
        shader_defs: vec![],
        entry_point: Some(Cow::from("main")),
        ..default()
    });

    commands.insert_resource(NoiseFieldComputePipeline {
        bind_group_layout,
        pipeline_id,
    });
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct NoiseFieldComputeLabel;

struct NoiseFieldComputeNode {
    state: NoiseComputeState,
}

#[derive(Clone)]
pub struct NoiseComputeDispatch {
    chunk_coord: IVec3,
    staging_buffer: Buffer,
    ready: Arc<AtomicBool>,
}

enum NoiseComputeState {
    Loading,
    Ready,
}

impl Default for NoiseFieldComputeNode {
    fn default() -> Self {
        Self {
            state: NoiseComputeState::Loading,
        }
    }
}

impl render_graph::Node for NoiseFieldComputeNode {
    fn update(&mut self, world: &mut World) {
        let pipeline_id = {
            let pipeline = world.resource::<NoiseFieldComputePipeline>();
            let pipeline_cache = world.resource::<PipelineCache>();

            match self.state {
                NoiseComputeState::Loading => {
                    if let CachedPipelineState::Ok(_) =
                        pipeline_cache.get_compute_pipeline_state(pipeline.pipeline_id)
                    {
                        self.state = NoiseComputeState::Ready;
                    }
                }
                NoiseComputeState::Ready => {}
            }

            pipeline.pipeline_id
        };

        if !matches!(self.state, NoiseComputeState::Ready) {
            return;
        }

        let pending: Vec<_> = {
            let mut queue = world.resource_mut::<NoiseFieldQueue>();
            queue.pending.drain(..).collect()
        };

        for (chunk_coord, params) in pending {
            info!("Processing chunk {:?}", chunk_coord);
            let dispatch = {
                let render_device = world.resource::<RenderDevice>();
                let pipeline_cache = world.resource::<PipelineCache>();
                let pipeline_layout = world
                    .resource::<NoiseFieldComputePipeline>()
                    .bind_group_layout
                    .clone();
                let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline_id) else {
                    return;
                };

                let params_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("noise_params"),
                    contents: bytemuck::cast_slice(&[params]),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

                let storage_buffer = render_device.create_buffer(&BufferDescriptor {
                    label: Some("noise_field_storage"),
                    size: (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE * 4) as u64,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });

                let staging_buffer = render_device.create_buffer(&BufferDescriptor {
                    label: Some("noise_field_staging"),
                    size: (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE * 4) as u64,
                    usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });

                let bind_group = render_device.create_bind_group(
                    Some("noise_field_bind_group"),
                    &pipeline_layout,
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

                let mut encoder =
                    render_device.create_command_encoder(&CommandEncoderDescriptor::default());
                {
                    let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.dispatch_workgroups(
                        (FIELD_SIZE + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
                        (FIELD_SIZE + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
                        (FIELD_SIZE + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
                    );
                }

                encoder.copy_buffer_to_buffer(
                    &storage_buffer,
                    0,
                    &staging_buffer,
                    0,
                    (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE * 4) as u64,
                );

                let render_queue = world.resource::<RenderQueue>();
                render_queue.submit(std::iter::once(encoder.finish()));

                let ready_flag = Arc::new(AtomicBool::new(false));
                let flag_clone = ready_flag.clone();
                staging_buffer
                    .slice(..)
                    .map_async(MapMode::Read, move |result| {
                        if result.is_ok() {
                            flag_clone.store(true, std::sync::atomic::Ordering::Release);
                        }
                    });

                NoiseComputeDispatch {
                    chunk_coord,
                    staging_buffer,
                    ready: ready_flag,
                }
            };

            let mut queue = world.resource_mut::<NoiseFieldQueue>();
            queue.pending_readback.push(dispatch);
        }

        // Stage 2: Check readback
        let mut to_move = Vec::new();
        {
            let queue = world.resource::<NoiseFieldQueue>();
            for (i, dispatch) in queue.pending_readback.iter().enumerate() {
                if dispatch.ready.load(std::sync::atomic::Ordering::Acquire) {
                    let data = dispatch.staging_buffer.slice(..).get_mapped_range();
                    let mut field = NoiseField::new(dispatch.chunk_coord);
                    field
                        .values
                        .copy_from_slice(bytemuck::cast_slice(&data[..]));
                    drop(data);
                    dispatch.staging_buffer.unmap();

                    to_move.push((i, field));
                }
            }
        }

        let mut queue = world.resource_mut::<NoiseFieldQueue>();
        for field_ready in to_move {
            info!("Processing chunk {:?}", field_ready.0);
            queue.ready.push(field_ready.1);
        }

        // Remove in reverse to maintain indices
        for i in queue
            .pending_readback
            .iter()
            .enumerate()
            .filter(|(_, d)| d.ready.load(std::sync::atomic::Ordering::Acquire))
            .map(|(i, _)| i)
            .collect::<Vec<_>>()
            .iter()
            .rev()
        {
            queue.pending_readback.remove(*i);
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        _render_context: &mut RenderContext,
        _world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        Ok(())
    }
}

pub struct NoiseFieldComputePlugin;

impl Plugin for NoiseFieldComputePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "noise_field.wgsl");

        app.add_plugins(ExtractResourcePlugin::<NoiseFieldQueue>::default())
            .init_resource::<NoiseFieldQueue>();

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app.add_systems(RenderStartup, init_noise_field_pipeline);

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(NoiseFieldComputeLabel, NoiseFieldComputeNode::default());
    }
}

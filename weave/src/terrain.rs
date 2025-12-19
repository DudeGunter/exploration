pub use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        gpu_readback::{Readback, ReadbackComplete},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
    },
};
use bytemuck::{Pod, Zeroable};

// TODO list:
// Make it not generic/explore other options
// simplify and remove excess fat for mantainance
// better 2d suport
// more general:
// shader pass systems
// more compute shaders
// make my own bevy_compute_workers with better support

#[derive(Resource)]
pub struct TerrainNoiseParams {
    pub scale: f32,
    pub frequency: f32,
    pub amplitude: f32,
    pub octaves: u32,
}

impl Default for TerrainNoiseParams {
    fn default() -> Self {
        Self {
            scale: 0.05,
            frequency: 1.0,
            amplitude: 1.0,
            octaves: 1,
        }
    }
}

#[derive(EntityEvent)]
pub struct RequestNoise {
    pub entity: Entity,
    pub position: IVec3,
}

#[derive(Event)]
pub struct RequestComplete {
    pub position: IVec3,
    pub data: Vec<f32>,
}

pub fn clear_queue(mut queue: ResMut<NoiseFieldQueue>) {
    queue.queue.clear();
}

pub fn handle_requests(
    trigger: On<RequestNoise>,
    mut commands: Commands,
    params: Res<TerrainNoiseParams>,
    mut queue: ResMut<NoiseFieldQueue>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let buffer: Vec<f32> = vec![0.0; (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE) as usize];
    let mut buffer = ShaderStorageBuffer::from(buffer);
    buffer.buffer_description.usage =
        BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
    let buffer = buffers.add(buffer);
    let coord = trigger.event().position;

    let noise_params = NoiseParams {
        chunk_x: coord.x,
        chunk_y: coord.y,
        chunk_z: coord.z,
        scale: params.scale,
        frequency: params.frequency,
        amplitude: params.amplitude,
        octaves: params.octaves,
        _padding: 0,
    };
    commands
        .spawn((Readback::buffer(buffer.clone()), Params(noise_params)))
        .observe(
            |trigger: On<ReadbackComplete>, mut commands: Commands, query: Query<&Params>| {
                let data: Vec<f32> = trigger.to_shader_type();
                // Unnecessary if you just wait a bit
                if data.iter().sum::<f32>() == 0.0 {
                    warn!("Likely didn't generate properly, all the noise data is zero!");
                    warn!("This is likely caused by the shader not being compiled yet or smth");
                }
                let params = query.get(trigger.entity).unwrap().0;
                commands.trigger(RequestComplete {
                    position: IVec3::new(params.chunk_x, params.chunk_y, params.chunk_z),
                    data,
                });

                commands.entity(trigger.entity).despawn();
            },
        );

    queue.queue.push((noise_params, buffer));
}

pub const FIELD_SIZE: u32 = 32;
pub const WORK_GROUP_SIZE: u32 = 4;

pub fn plugin(app: &mut App) {
    embedded_asset!(app, "noise_field.wgsl");

    app.add_plugins(ExtractResourcePlugin::<NoiseFieldQueue>::default());
    app.insert_resource(NoiseFieldQueue { queue: Vec::new() });

    let render_app = app.get_sub_app_mut(RenderApp).unwrap();
    render_app.add_systems(
        RenderStartup,
        (init_pipeline, add_compute_render_graph_node),
    );
}

#[derive(Component)]
pub struct Params(pub NoiseParams);

#[derive(ExtractResource, Resource, Clone)]
pub struct NoiseFieldQueue {
    pub queue: Vec<(NoiseParams, Handle<ShaderStorageBuffer>)>,
}

#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct TerrainSettings {
    pub scale: f32,
    pub frequency: f32,
    pub amplitude: f32,
    pub octaves: u32,
}

impl Default for TerrainSettings {
    fn default() -> Self {
        Self {
            scale: 0.1,
            frequency: 1.0,
            amplitude: 1.0,
            octaves: 1,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType, PartialEq)]
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
            scale: 0.1,
            frequency: 1.0,
            amplitude: 1.0,
            octaves: 1,
            _padding: 0,
        }
    }
}

#[derive(Resource)]
pub struct NoisePipeline {
    layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

fn init_pipeline(
    mut commands: Commands,
    device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    cache: Res<PipelineCache>,
) {
    // These could be simplified with helper functions from bevy... don't care to fiddle with what works
    let layout = device.create_bind_group_layout(
        "noise_layout",
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

    let pipeline = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        layout: vec![layout.clone()],
        shader: asset_server.load("embedded://weave/noise_field.wgsl"),
        ..default()
    });

    commands.insert_resource(NoisePipeline { layout, pipeline });
}

#[derive(Default)] // I don't know why its {} and default, jus following the ex.
pub struct NoiseComputeNode {}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ComputeNodeLabel;

fn add_compute_render_graph_node(mut render_graph: ResMut<RenderGraph>) {
    render_graph.add_node(ComputeNodeLabel, NoiseComputeNode::default());
}

// None of the labels are needed I imagine
impl render_graph::Node for NoiseComputeNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let queue = world.resource::<NoiseFieldQueue>();
        let pipeline = world.resource::<NoisePipeline>();
        let cache = world.resource::<PipelineCache>();
        let device = world.resource::<RenderDevice>();
        let buffers = world.resource::<RenderAssets<GpuShaderStorageBuffer>>();
        if let Some(init_pipeline) = cache.get_compute_pipeline(pipeline.pipeline) {
            for (param, buffer) in &queue.queue {
                let mut pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("Noise Field Compute Readback"),
                            ..default()
                        });
                let param_buf = device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("Params"),
                    contents: bytemuck::cast_slice(&[*param]),
                    usage: BufferUsages::UNIFORM,
                });
                let bind_group = device.create_bind_group(
                    "noise_bind_group",
                    &pipeline.layout,
                    &BindGroupEntries::sequential((
                        param_buf.as_entire_binding(),
                        buffers.get(buffer).unwrap().buffer.as_entire_binding(),
                    )),
                );
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_pipeline(init_pipeline);

                // I'm pretty sure this is all bullshite. I had AI help me initially, but the ex doesn't require this
                // Still dont full understand the purpose of work groups, especially in this context
                pass.dispatch_workgroups(
                    (FIELD_SIZE + WORK_GROUP_SIZE - 1) / WORK_GROUP_SIZE,
                    (FIELD_SIZE + WORK_GROUP_SIZE - 1) / WORK_GROUP_SIZE,
                    (FIELD_SIZE + WORK_GROUP_SIZE - 1) / WORK_GROUP_SIZE,
                );
            }
        }
        Ok(())
    }
}

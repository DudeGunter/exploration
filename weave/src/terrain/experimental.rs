// Custom render graph node for noise field generation,
// utilizes parallelism on the gpu for extremely fast generation... hopefully.
// This is experimental and hopefully will replace the one-at-a-time generation.
// Note: This isn't probably needed but I want it working.
#![allow(unused)] // for now...
use bevy::{
    asset::embedded_asset,
    ecs::component::QueuedComponents,
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

pub const FIELD_SIZE: u32 = 32;
pub const WORK_GROUP_SIZE: u32 = 4;

pub fn plugin(app: &mut App) {
    embedded_asset!(app, "noise_field.wgsl");

    app.add_plugins(ExtractResourcePlugin::<NoiseFieldQueue>::default());
    app.insert_resource(NoiseFieldQueue { queue: Vec::new() });
    app.add_systems(Startup, testing);

    let mut render_app = app.get_sub_app_mut(RenderApp).unwrap();
    render_app.add_systems(
        RenderStartup,
        (init_pipeline, add_compute_render_graph_node),
    );
}

#[derive(Component)]
pub struct Params(NoiseParams);

pub fn testing(
    mut commands: Commands,
    mut queue: ResMut<NoiseFieldQueue>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let buffer: Vec<f32> = vec![99.0; (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE) as usize];
    let mut buffer = ShaderStorageBuffer::from(buffer);
    buffer.buffer_description.usage =
        BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
    let buffer = buffers.add(buffer);
    commands
        .spawn((
            Readback::buffer(buffer.clone()),
            Params(NoiseParams::default()),
        ))
        .observe(
            |trigger: On<ReadbackComplete>,
             mut commands: Commands,
             mut queue: ResMut<NoiseFieldQueue>,
             query: Query<&Params>| {
                let data: Vec<f32> = trigger.to_shader_type();
                info!("Data readback complete: {:?}", data);
                queue
                    .queue
                    .retain(|(params, _)| *params != query.single().unwrap().0);

                commands.entity(trigger.entity).despawn();
            },
        );

    queue.queue.push((NoiseParams::default(), buffer));
}

#[derive(ExtractResource, Resource, Clone)]
pub struct NoiseFieldQueue {
    pub queue: Vec<(NoiseParams, Handle<ShaderStorageBuffer>)>,
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
    let layout = device.create_bind_group_layout(
        "noise_layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
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

    info!("Layout: {:?}", layout);

    let pipeline = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("noise_pipeline".into()),
        layout: vec![layout.clone()],
        push_constant_ranges: vec![],
        shader: asset_server.load("embedded://weave/terrain/noise_field.wgsl"),
        shader_defs: vec![],
        entry_point: Some(std::borrow::Cow::from("main")),
        ..default()
    });

    commands.insert_resource(NoisePipeline { layout, pipeline });
}

#[derive(Default)] // I don't know why its {} or default, jus following the ex.
pub struct NoiseComputeNode {}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ComputeNodeLabel;

fn add_compute_render_graph_node(mut render_graph: ResMut<RenderGraph>) {
    // Add the compute node as a top-level node to the render graph. This means it will only execute
    // once per frame. Normally, adding a node would use the `RenderGraphApp::add_render_graph_node`
    // method, but it does not allow adding as a top-level node.
    render_graph.add_node(ComputeNodeLabel, NoiseComputeNode::default());
}

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
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
                });
                let storage_buf = device.create_buffer(&BufferDescriptor {
                    label: Some("noise_storage"),
                    size: (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE * 4) as u64,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });
                let bind_group = device.create_bind_group(
                    "noise_bind_group",
                    &pipeline.layout,
                    &BindGroupEntries::sequential((
                        param_buf.as_entire_binding(),
                        storage_buf.as_entire_binding(),
                        //buffers.get(buffer).unwrap().buffer.as_entire_binding(),
                    )),
                );
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_pipeline(init_pipeline);
                info!("Dispatching!!!");
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

// Custom render graph node for noise field generation,
// utilizes parallelism on the gpu for extremely fast generation... hopefully.
// This is experimental and hopefully will replace the one-at-a-time generation.
// Note: This isn't probably needed but I want it working.
//
// IT WORKS BIG NOTE:
// For the longest time I couldn't figure out why I was just getting zeros back
// Turns out that you must wait a bit (6-7 frames) for the shader to be ready.
// (this wasn't noted in the docs)
//#![allow(unused)] // for now... No longer!
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

#[derive(Default)] // I don't know why its {} and default, jus following the ex.
pub struct NoiseComputeNode {}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ComputeNodeLabel;

fn add_compute_render_graph_node(mut render_graph: ResMut<RenderGraph>) {
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

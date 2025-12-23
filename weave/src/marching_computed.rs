#![allow(unused)]
use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        gpu_readback::{Readback, ReadbackComplete},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{binding_types::*, *},
        renderer::{RenderContext, RenderDevice},
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
    },
};
use bytemuck::{Pod, Zeroable};

pub const FIELD_SIZE: u32 = crate::chunks::CHUNK_SIZE + 1;
pub const WORK_GROUP_SIZE: u32 = 4;
pub const MAX_VERTS: usize = 65535;

pub fn plugin(app: &mut App) {
    embedded_asset!(app, "mesh_generation.wgsl");
    embedded_asset!(app, "density_fields.wgsl");

    app.add_plugins(ExtractResourcePlugin::<MainWorldQueue>::default());

    let render_app = app.sub_app_mut(RenderApp);
    render_app.add_systems(RenderStartup, init_pipelines);
}

//
// INIT pipelines
//

#[derive(Resource)]
pub struct ComputePipelines {
    pub density_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
    pub density_pipeline: CachedComputePipelineId,
    pub mesh_pipeline: CachedComputePipelineId,
}

pub fn init_pipelines(
    mut commands: Commands,
    device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    cache: Res<PipelineCache>,
) {
    use std::num::NonZero;
    let density_layout = device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<NoiseParams>(false),
                storage_buffer_sized(
                    false,
                    NonZero::<u64>::new((FIELD_SIZE * FIELD_SIZE * FIELD_SIZE * 4) as u64),
                ),
            ),
        ),
    );
    let density_pipeline = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        layout: vec![density_layout.clone()],
        shader: asset_server.load("embedded:://weave/density_fields.wgsl"),
        ..default()
    });

    let mesh_layout = device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<NoiseParams>(false),
                storage_buffer_sized(
                    false,
                    NonZero::<u64>::new((FIELD_SIZE * FIELD_SIZE * FIELD_SIZE * 4) as u64),
                ),
                storage_buffer::<MeshOutput>(false),
            ),
        ),
    );

    let mesh_pipeline = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        layout: vec![mesh_layout.clone()],
        shader: asset_server.load("embedded:://weave/mesh_generation.wgsl"),
        ..default()
    });

    commands.insert_resource(ComputePipelines {
        density_layout,
        mesh_layout,
        density_pipeline,
        mesh_pipeline,
    });
}

//
// Shader types
//

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
pub struct NoiseParams {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub chunk_z: i32,
    pub scale: f32,
    pub frequency: f32,
    pub amplitude: f32,
    pub octaves: u32,
}

// Mirror of WGSL MeshOutput struct
#[repr(C)]
#[derive(ShaderType, Clone, Copy, Pod, Zeroable)]
pub struct MeshOutput {
    pub vertices: [MarchVertex; MAX_VERTS],
    pub vertex_count: u32,
    pub indices: [u32; MAX_VERTS],
    pub index_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
pub struct MarchVertex {
    pub position: Vec3,
    _pad: f32,
}

//
// Data Management
//

#[derive(Resource, ExtractResource, Clone, Default)]
pub struct MainWorldQueue {
    pub queue: Vec<Request>,
}

#[derive(Clone)]
pub enum Request {
    Density(NoiseParams),
    Mesh(Density),
    Full(NoiseParams),
}

pub type Density = Vec<f32>;

impl MainWorldQueue {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    pub fn queue(&mut self, request: Request) {
        self.queue.push(request);
    }
}

impl Queue for MainWorldQueue {
    fn clear(&mut self) {
        self.queue.clear();
    }
}

pub trait Queue: Resource {
    fn clear(&mut self);
}

pub fn clear_queue<T: Queue>(mut queue: ResMut<T>) {
    queue.clear();
}

//
// Render Graph Node
//

pub struct ComputeNode;

pub struct ComputeLabel;

impl render_graph::Node for ComputeNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let queue = world.resource::<MainWorldQueue>();
        let device = world.resource::<RenderDevice>();
        let cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<ComputePipelines>();
        let buffers = world.resource::<RenderAssets<GpuShaderStorageBuffer>>();

        Ok(())
    }
}

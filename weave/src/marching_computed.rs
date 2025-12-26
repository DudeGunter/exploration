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
pub const TOTAL_WORK_GROUP_SIZE: u32 = (FIELD_SIZE + WORK_GROUP_SIZE - 1) / WORK_GROUP_SIZE;
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

/// Every Request in the queue is processed and cleared
/// before the next update (render world time)
/// Duplicate requests aren't handled by me (as of Dec 25. 2025)
/// or and of my sub-processes
#[derive(Resource, ExtractResource, Clone, Default, Deref, DerefMut)]
pub struct MainWorldQueue(Vec<Request>);

// Could be an event
// Currently doesn't allow you to return density values
// Something to think about if thats needed
// don't want the extra layer of complexity for now
#[derive(Clone)]
pub enum Request {
    Mesh((NoiseParams, Density)),
    Full(NoiseParams),
}

pub type Density = Vec<f32>;

impl MainWorldQueue {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// What ever you queue will be cleared the next frame and not accesible
    pub fn queue(&mut self, request: Request) {
        self.push(request);
    }
}

/// Mesnt to exist strictly in the render world
#[derive(Resource, Deref, DerefMut)]
pub struct ComputeQueue(Vec<Buffer>);

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
        let main_queue = world.resource::<MainWorldQueue>();
        let device = world.resource::<RenderDevice>();
        let cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<ComputePipelines>();
        let buffers = world.resource::<RenderAssets<GpuShaderStorageBuffer>>();

        // the following set of lines could look nicer
        let density_compute = cache.get_compute_pipeline(pipelines.density_pipeline);
        let mesh_compute = cache.get_compute_pipeline(pipelines.mesh_pipeline);
        if density_compute.is_none() || mesh_compute.is_none() {
            warn!("Pipelins aren't ready!");
            return Ok(());
        }

        // The main_queue is cleared in main each frame
        for request in main_queue.iter() {
            match request {
                // User set density field mesh
                // This should (hopefully) be basically instant
                Request::Mesh((noise_params, data)) => {
                    // It seems this likely isn't needed
                    let mut pass = render_context.command_encoder().begin_compute_pass(
                        &ComputePassDescriptor {
                            label: Some("User set density field mesh"),
                            ..default()
                        },
                    );

                    let params_buf = device.create_buffer_with_data(&BufferInitDescriptor {
                        label: Some("User set density field params buffer"),
                        contents: bytemuck::cast_slice(&[*noise_params]),
                        usage: BufferUsages::UNIFORM,
                    });
                    // TODO!: can't figure out the contents sitch.
                    //let density_buf = device.create_buffer_with_data(&BufferInitDescriptor {
                    //    label: Some("User set density data buffer"),
                    //    contents: data,
                    //    usage: BufferUsages::STORAGE,
                    //});
                    todo!("This is unimplemented!");
                }
                // Generated density field + mesh generation
                Request::Full(noise_params) => {
                    let mut pass = render_context.command_encoder().begin_compute_pass(
                        &ComputePassDescriptor {
                            label: Some("Generate Density Values"),
                            ..default()
                        },
                    );

                    let params_buf = device.create_buffer_with_data(&BufferInitDescriptor {
                        label: Some("User set density field params buffer"),
                        contents: bytemuck::cast_slice(&[*noise_params]),
                        usage: BufferUsages::UNIFORM,
                    });

                    let density_buf = device.create_buffer(&BufferDescriptor {
                        label: Some("Generated Density values"),
                        size: (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE * 4) as u64,
                        usage: BufferUsages::STORAGE,
                        mapped_at_creation: false,
                    });
                    todo!("This goes no where and the data is never used!");
                    // This should be attached to a buffer/handle which can be carried -
                    // when done - to the next shader... research on bevy or other things on
                    // how to do this
                    // Note: I'm likely wrong about this impl, the docs are rough
                    let bind_group = device.create_bind_group(
                        "Generated Desnity values",
                        &pipelines.density_layout,
                        &BindGroupEntries::sequential((
                            params_buf.as_entire_binding(),
                            density_buf.as_entire_binding(),
                        )),
                    );

                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.set_pipeline(density_compute.unwrap());
                    pass.dispatch_workgroups(
                        TOTAL_WORK_GROUP_SIZE,
                        TOTAL_WORK_GROUP_SIZE,
                        TOTAL_WORK_GROUP_SIZE,
                    );

                    drop(pass); // claude said ts is needed. I trust you claude

                    todo!("create the second shader pass with the same previous buffer");
                }
            }
        }

        Ok(())
    }
}

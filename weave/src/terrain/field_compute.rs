pub use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        gpu_readback::{Readback, ReadbackComplete},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        storage::ShaderStorageBuffer,
    },
};
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use std::collections::HashMap;

pub const FIELD_SIZE: u32 = 17;
pub const WORKGROUP_SIZE: u32 = 4;

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

#[derive(Resource, Default, Clone, ExtractResource)]
pub struct NoiseRequests(pub HashMap<Entity, (IVec3, NoiseParams)>);

#[derive(Resource)]
struct NoiseComputePipeline {
    layout: BindGroupLayout,
    pipeline_id: CachedComputePipelineId,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct NoiseComputeLabel;

struct NoiseComputeNode(bool);

impl Default for NoiseComputeNode {
    fn default() -> Self {
        Self(false)
    }
}

impl render_graph::Node for NoiseComputeNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<NoiseComputePipeline>();
        let cache = world.resource::<PipelineCache>();
        self.0 = matches!(
            cache.get_compute_pipeline_state(pipeline.pipeline_id),
            CachedPipelineState::Ok(_)
        );
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        if !self.0 {
            return Ok(());
        }

        let requests = world.resource::<NoiseRequests>();
        let pipeline = world.resource::<NoiseComputePipeline>();
        let cache = world.resource::<PipelineCache>();
        let device = world.resource::<RenderDevice>();

        let Some(pipeline) = cache.get_compute_pipeline(pipeline.pipeline_id) else {
            return Ok(());
        };

        for (_entity, (_chunk_coord, params)) in &requests.0 {
            let params_buf = device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("noise_params"),
                contents: bytemuck::cast_slice(&[*params]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

            let storage_buf = device.create_buffer(&BufferDescriptor {
                label: Some("noise_storage"),
                size: (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE * 4) as u64,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            let bind_group = device.create_bind_group(
                Some("noise_bind_group"),
                &world.resource::<NoiseComputePipeline>().layout,
                &[
                    BindGroupEntry {
                        binding: 0,
                        resource: params_buf.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: storage_buf.as_entire_binding(),
                    },
                ],
            );

            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(
                (FIELD_SIZE + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
                (FIELD_SIZE + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
                (FIELD_SIZE + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE,
            );
        }

        Ok(())
    }
}

pub struct NoiseFieldComputePlugin;

impl Plugin for NoiseFieldComputePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "noise_field.wgsl");

        app.add_plugins(ExtractResourcePlugin::<NoiseRequests>::default())
            .init_resource::<NoiseRequests>()
            .add_observer(on_complete);

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app.add_systems(RenderStartup, init_pipeline);

        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(NoiseComputeLabel, NoiseComputeNode::default());
    }
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

    let pipeline_id = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("noise_pipeline".into()),
        layout: vec![layout.clone()],
        push_constant_ranges: vec![],
        shader: asset_server.load("embedded://weave/terrain/noise_field.wgsl"),
        shader_defs: vec![],
        entry_point: Some(Cow::from("main")),
        ..default()
    });

    commands.insert_resource(NoiseComputePipeline {
        layout,
        pipeline_id,
    });
}

fn on_complete(trigger: On<ReadbackComplete>, mut requests: ResMut<NoiseRequests>) {
    if let Some((_coord, _params)) = requests.0.remove(&trigger.entity) {
        let data: Vec<f32> = trigger.to_shader_type();
        info!("Noise ready: {} values", data.len());
    }
}

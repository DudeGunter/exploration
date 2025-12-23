use bevy::{
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
            octaves: 4,
        }
    }
}

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

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            chunk_x: 0,
            chunk_y: 0,
            chunk_z: 0,
            scale: 0.1,
            frequency: 1.0,
            amplitude: 1.0,
            octaves: 4,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
pub struct MarchVertex {
    pub position: Vec3,
    _pad: f32,
}

// ============= EVENTS =============

#[derive(EntityEvent)]
pub struct RequestNoise {
    pub entity: Entity,
    pub position: IVec3,
}

#[derive(EntityEvent)]
pub struct RequestComplete {
    pub entity: Entity,
    pub position: IVec3,
    pub handle: Handle<Mesh>,
}

// ============= RESOURCES & COMPONENTS =============
#[derive(Clone)]
pub struct ComputeJob {
    pub mesh_output: Handle<ShaderStorageBuffer>,
    pub params: NoiseParams,
    pub terrain_entity: Entity,
    pub position: IVec3,
}

#[derive(ExtractResource, Resource, Clone)]
pub struct MarchingCubesQueue {
    pub queue: Vec<ComputeJob>,
}

#[derive(Resource)]
pub struct MarchingCubesPipeline {
    pub layout: BindGroupLayout,
    pub pipeline: CachedComputePipelineId,
}

#[derive(Component)]
pub struct MeshOutputComponent {
    pub terrain_entity: Entity,
    pub position: IVec3,
}

// ============= CONSTANTS =============

pub const FIELD_SIZE: u32 = crate::chunks::CHUNK_SIZE + 1;
pub const WORK_GROUP_SIZE: u32 = 4;
pub const MAX_VERTS: usize = 65535;

// Mirror of WGSL MeshOutput struct
#[repr(C)]
#[derive(ShaderType, Clone, Copy, Pod, Zeroable)]
pub struct MeshOutput {
    pub vertices: [MarchVertex; MAX_VERTS],
    pub vertex_count: u32,
    pub indices: [u32; MAX_VERTS],
    pub index_count: u32,
    pub _pad: [u32; 3], // 12 bytes
}

// ============= SYSTEMS =============

pub fn clear_queue(mut queue: ResMut<MarchingCubesQueue>) {
    queue.queue.clear();
}

pub fn handle_requests(
    trigger: On<RequestNoise>,
    mut commands: Commands,
    params: Res<TerrainNoiseParams>,
    mut queue: ResMut<MarchingCubesQueue>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let coord = trigger.event().position;
    let terrain_entity = trigger.entity;

    let noise_params = NoiseParams {
        chunk_x: coord.x,
        chunk_y: coord.y,
        chunk_z: coord.z,
        scale: params.scale,
        frequency: params.frequency,
        amplitude: params.amplitude,
        octaves: params.octaves,
    };

    // Create mesh output buffer - single struct
    let buffer_size = std::mem::size_of::<MeshOutput>() / 4;
    let mesh_data = vec![0u32; buffer_size];
    let mut mesh_buffer = ShaderStorageBuffer::from(mesh_data);
    mesh_buffer.buffer_description.usage =
        BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST;
    let mesh_output_handle = buffers.add(mesh_buffer);

    let job = ComputeJob {
        mesh_output: mesh_output_handle.clone(),
        params: noise_params,
        terrain_entity,
        position: coord,
    };

    queue.queue.push(job);

    // Schedule readback
    commands
        .spawn((
            Readback::buffer(mesh_output_handle.clone()),
            MeshOutputComponent {
                terrain_entity,
                position: coord,
            },
        ))
        .observe(on_mesh_readback_complete);
}

// ============= READBACK HANDLER =============

pub fn on_mesh_readback_complete(
    trigger: On<ReadbackComplete>,
    mut commands: Commands,
    query: Query<(Entity, &MeshOutputComponent)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if let Ok((entity, mesh_comp)) = query.get(trigger.entity) {
        let mesh_data: &MeshOutput = bytemuck::from_bytes(&trigger.data);

        let vert_count = (mesh_data.vertex_count as usize).min(MAX_VERTS - 1);
        let idx_count = (mesh_data.index_count as usize).min(MAX_VERTS - 1);

        // Only allocate for actual vertices/indices
        let mut vertices = Vec::with_capacity(vert_count);
        let mut indices = Vec::with_capacity(idx_count);

        for i in 0..vert_count {
            let v = mesh_data.vertices[i];
            vertices.push([v.position.x, v.position.y, v.position.z]);
        }

        for i in 0..idx_count {
            indices.push(mesh_data.indices[i]);
        }

        // Create mesh directly
        let mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_indices(bevy::mesh::Indices::U32(indices))
        .with_computed_area_weighted_normals();

        let mesh_handle = meshes.add(mesh);

        // Spawn mesh entity as child of terrain chunk
        commands.trigger(RequestComplete {
            entity: mesh_comp.terrain_entity,
            position: mesh_comp.position,
            handle: mesh_handle,
        });

        commands.entity(entity).despawn();
    }
}

// ============= RENDER GRAPH NODE =============

#[derive(Default)]
pub struct MarchingCubesNode;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct MarchingCubesLabel;

pub fn add_marching_cubes_node(mut render_graph: ResMut<RenderGraph>) {
    render_graph.add_node(MarchingCubesLabel, MarchingCubesNode::default());
}

impl render_graph::Node for MarchingCubesNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let queue = world.resource::<MarchingCubesQueue>();
        let device = world.resource::<RenderDevice>();
        let cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<MarchingCubesPipeline>();
        let buffers = world.resource::<RenderAssets<GpuShaderStorageBuffer>>();

        if let Some(compute) = cache.get_compute_pipeline(pipeline.pipeline) {
            for job in &queue.queue {
                let mut pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("Marching Cubes + Noise"),
                            ..default()
                        });

                let param_buf = device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("MC Params"),
                    contents: bytemuck::cast_slice(&[job.params]),
                    usage: BufferUsages::UNIFORM,
                });

                let bind_group = device.create_bind_group(
                    "march_bind_group",
                    &pipeline.layout,
                    &BindGroupEntries::sequential((
                        param_buf.as_entire_binding(),
                        buffers
                            .get(&job.mesh_output)
                            .unwrap()
                            .buffer
                            .as_entire_binding(),
                    )),
                );

                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_pipeline(compute);
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

// ============= PIPELINE SETUP =============

fn init_pipeline(
    mut commands: Commands,
    device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    cache: Res<PipelineCache>,
) {
    use bevy::render::render_resource::binding_types::*;

    let layout = device.create_bind_group_layout(
        "march_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<NoiseParams>(false),
                storage_buffer::<MeshOutput>(false),
            ),
        ),
    );

    let pipeline = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        layout: vec![layout.clone()],
        shader: asset_server.load("embedded://weave/marching_cubes.wgsl"),
        ..default()
    });

    commands.insert_resource(MarchingCubesPipeline { layout, pipeline });
}

// ============= PLUGIN =============

pub fn plugin(app: &mut App) {
    embedded_asset!(app, "marching_cubes.wgsl");

    app.add_plugins(ExtractResourcePlugin::<MarchingCubesQueue>::default());
    app.insert_resource(MarchingCubesQueue { queue: Vec::new() });

    let render_app = app.get_sub_app_mut(RenderApp).unwrap();
    render_app.add_systems(RenderStartup, (init_pipeline, add_marching_cubes_node));
}

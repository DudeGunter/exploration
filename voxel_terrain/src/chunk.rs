use crate::terrain::*;
use avian3d::{
    math::{Scalar, Vector},
    prelude::*,
};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use noiz::prelude::*;

pub const CHUNK_SIZE: i32 = 32;
pub const VOXEL_SIZE: Vector = Vector::splat(1.0);

#[derive(Component, Reflect, Debug, Clone, Copy, Deref, DerefMut, Hash, Eq, PartialEq)]
pub struct Chunk(pub IVec2);

impl Chunk {
    pub fn new(x: i32, y: i32) -> Self {
        Chunk(IVec2::new(x, y))
    }
}

//#[derive(Component)]
//pub struct ChunkVoxelData {
//    voxels: HashMap<IVec3, VoxelType>,
//}
//
//pub struct VoxelType(u8);

pub fn spawn_generator_task(
    chunk: Chunk,
    noise: TerrainNoise,
    pool: &AsyncComputeTaskPool,
) -> Task<(Mesh, Collider)> {
    pool.spawn(async move {
        let mut points = vec![];
        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                // Offset by chunk location
                let x = (i as Scalar) + (chunk.x * CHUNK_SIZE) as f32;
                let z = (j as Scalar) + (chunk.y * CHUNK_SIZE) as f32;
                let y = noise.sample_for::<f32>(Vec2::new(x, z) * 0.05) * 10.0;
                let point = Vector::new(i as Scalar, y, j as Scalar); // Local coords
                points.push(VOXEL_SIZE * point);
            }
        }

        let collider = Collider::voxels_from_points(VOXEL_SIZE, &points);

        // Compute the mesh for rendering.
        let (vertices, indices) = collider.shape().as_voxels().unwrap().to_trimesh();
        let vertices: Vec<[f32; 3]> = vertices
            .iter()
            .map(|v| [v.x as f32, v.y as f32, v.z as f32])
            .collect();
        let indices: Vec<u32> = indices.into_iter().flatten().collect();
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(indices.clone()))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_duplicated_vertices()
        .with_computed_flat_normals();

        (mesh, collider)
    })
}

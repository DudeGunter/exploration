// Should route how the chunks need to be managed
// A rewrite is in order!!!
use crate::{
    chunk::{CHUNK_SIZE, Chunk, VOXEL_SIZE},
    terrain::{Terrain, TerrainMaterial, TerrainNoise},
};
use avian3d::{
    math::{AsF32, Scalar, Vector},
    prelude::*,
};
use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future},
};
use std::f32::consts::PI;

/// Attach this to the entity where the terrain should be loaded.
#[derive(Component, Reflect, Debug)]
#[require(AreaManaged::Circle(10.0))]
pub struct Observer;

/// In what shape and distance the terrain should be loaded.
#[derive(Component, Reflect, Debug)]
pub enum AreaManaged {
    Circle(f32),
    Rectangle(f32, f32),
}

#[derive(Resource, Reflect, Default)]
pub struct ChunkManager {
    desired_chunks: HashSet<Chunk>,
    chunk_entities: HashMap<Chunk, Entity>,
}

impl ChunkManager {
    pub fn request_chunk(&mut self, pos: Chunk) {
        self.desired_chunks.insert(pos);
    }

    pub fn _unload_chunk(&mut self, pos: Chunk) {
        self.desired_chunks.remove(&pos);
    }

    pub fn should_exist(&self, pos: &Chunk) -> bool {
        self.desired_chunks.contains(pos)
    }

    pub fn get_entity(&self, pos: &Chunk) -> Option<Entity> {
        self.chunk_entities.get(pos).copied()
    }

    pub fn register_chunk(&mut self, pos: Chunk, entity: Entity) {
        self.chunk_entities.insert(pos, entity);
    }

    pub fn _unregister_chunk(&mut self, pos: &Chunk) {
        self.chunk_entities.remove(pos);
    }

    pub fn iter_desired_chunks(&self) -> Vec<Chunk> {
        self.desired_chunks.iter().copied().collect()
    }
}

#[derive(Resource, Reflect)]
pub struct ChunkSpawnLimiter {
    max_spawns_per_frame: usize,
    max_concurrent_tasks: usize,
    target_fps: f32,
    smooth_frame_time: f32,
}

impl Default for ChunkSpawnLimiter {
    fn default() -> Self {
        Self {
            max_spawns_per_frame: 4,
            max_concurrent_tasks: 12,
            target_fps: 60.0,
            smooth_frame_time: 1.0 / 60.0,
        }
    }
}

// State markers - mutually exclusive
#[derive(Component)]
pub struct Loading(Task<(Mesh, Collider)>);
#[derive(Component)]
pub struct Active;
#[derive(Component)]
pub struct Dormant;

pub fn spawn_with_limits(
    mut manager: ResMut<ChunkManager>,
    limiter: Res<ChunkSpawnLimiter>,
    generator: Res<TerrainNoise>,
    loading_chunks: Query<(), With<Loading>>,
    terrain: Single<Entity, With<Terrain>>,
    observer: Single<&GlobalTransform, With<Observer>>,
    mut commands: Commands,
) {
    let current_tasks = loading_chunks.iter().count();
    if current_tasks >= limiter.max_concurrent_tasks {
        return;
    }
    let observer = *observer;
    let observer_pos = observer.translation().xz().as_ivec2() / IVec2::splat(CHUNK_SIZE);

    // Get chunks sorted by priority
    let mut to_spawn: Vec<_> = manager
        .desired_chunks
        .iter()
        .filter(|pos| manager.get_entity(pos).is_none())
        .map(|pos| (*pos, pos.distance_squared(observer_pos)))
        .collect();

    to_spawn.sort_by_key(|(_, dist)| *dist);

    let spawn_count = limiter
        .max_spawns_per_frame
        .min(limiter.max_concurrent_tasks - current_tasks);

    let pool = AsyncComputeTaskPool::get();
    let noise = *generator.into_inner();
    for (chunk, _) in to_spawn.iter().take(spawn_count) {
        let task = crate::chunk::spawn_generator_task(*chunk, noise, pool);
        let entity = commands.spawn((*chunk, Loading(task))).id();
        commands.entity(*terrain).add_child(entity);
        manager.register_chunk(*chunk, entity);
    }
}

pub fn _add_desired_chunks(
    mut manager: ResMut<ChunkManager>,
    query: Query<(&AreaManaged, &GlobalTransform), (With<Observer>, Changed<GlobalTransform>)>,
) {
    //manager.desired_chunks.clear();

    for (area, transform) in query {
        let transform_offset = transform.translation().xz().as_ivec2() / IVec2::splat(CHUNK_SIZE);

        match area {
            AreaManaged::Circle(r) => {
                let r = *r as i32;
                for i in 0..(r * 2 + 1) {
                    for j in 0..(r * 2 + 1) {
                        let x = i - r;
                        let y = j - r;

                        if (x * x + y * y) <= (r * r) {
                            let chunk_pos = IVec2::new(x, y) + transform_offset;
                            manager.request_chunk(Chunk(chunk_pos));
                        }
                    }
                }
            }
            AreaManaged::Rectangle(_, _) => {
                // Basically whats above just without hte if statement
                todo!("Rectangle Not implemented yet!!!");
            }
        }
    }
}

pub fn _spawn_missing_chunks(
    mut commands: Commands,
    terrian: Single<Entity, With<Terrain>>,
    mut manager: ResMut<ChunkManager>,
    generator: Res<TerrainNoise>,
) {
    let pool = AsyncComputeTaskPool::get();
    let noise = *generator.into_inner();
    // Collect desired_chunks to release the immutable borrow on manager
    for chunk in manager.iter_desired_chunks() {
        // Only spawn a chunk if it does not already have an entity registered
        if manager.get_entity(&chunk).is_none() {
            let task = crate::chunk::spawn_generator_task(chunk, noise, pool);
            let entity = commands.spawn((chunk, Loading(task))).id();
            commands.entity(*terrian).add_child(entity);
            manager.register_chunk(chunk, entity);
        }
    }
}

pub fn handle_spawning_chunk(
    query: Query<(Entity, &Chunk, &mut Loading)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<TerrainMaterial>,
) {
    let chunk_world_size = CHUNK_SIZE as Scalar * VOXEL_SIZE.x;

    for (entity, chunk, mut task) in query {
        if let Some((mesh, collider)) = block_on(future::poll_once(&mut task.0)) {
            let location = chunk.0; // Get location from Chunk component
            commands
                .entity(entity)
                .insert((
                    RigidBody::Static,
                    collider,
                    Friction::new(0.2),
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(
                        Vector::new(
                            location.x as f32 * chunk_world_size
                                - CHUNK_SIZE as Scalar / 2.0 * VOXEL_SIZE.x,
                            -5.0 * VOXEL_SIZE.y,
                            location.y as f32 * chunk_world_size
                                - CHUNK_SIZE as Scalar / 2.0 * VOXEL_SIZE.z,
                        )
                        .f32(),
                    ),
                ))
                .remove::<Loading>()
                .insert(Active);
        }
    }
}

pub fn make_chunks_dormant(
    manager: Res<ChunkManager>,
    active_chunks: Query<(Entity, &Chunk), With<Active>>,
    mut commands: Commands,
) {
    for (entity, chunk) in active_chunks {
        if !manager.should_exist(chunk) {
            commands
                .entity(entity)
                .remove::<Active>()
                .insert((Dormant, Visibility::Hidden));
        }
    }
}

pub fn make_dormant_chunks_active(mut commands: Commands, manager: Res<ChunkManager>) {
    for chunk in manager.desired_chunks.iter() {
        if let Some(entity) = manager.get_entity(chunk) {
            commands
                .entity(entity)
                .remove::<Dormant>()
                .insert((Active, Visibility::Inherited));
        }
    }
}

fn adjust_limiter(time: Res<Time>, mut limiter: ResMut<ChunkSpawnLimiter>) {
    let delta = time.delta_secs();
    limiter.smooth_frame_time = limiter.smooth_frame_time * 0.95 + delta * 0.05;

    let target_time = 1.0 / limiter.target_fps;

    if limiter.smooth_frame_time > target_time * 1.3 {
        limiter.max_spawns_per_frame = (limiter.max_spawns_per_frame - 1).max(1);
        limiter.max_concurrent_tasks = (limiter.max_concurrent_tasks - 1).max(4);
    } else if limiter.smooth_frame_time < target_time * 0.7 {
        limiter.max_spawns_per_frame = (limiter.max_spawns_per_frame + 1).min(16);
        limiter.max_concurrent_tasks = (limiter.max_concurrent_tasks + 2).min(32);
    }
}

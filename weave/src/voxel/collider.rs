use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct ChunkCollider {
    pub coord: IVec2,
}

#[derive(Event)]
pub struct ColliderReady {
    pub coord: IVec2,
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u32>,
}

pub fn on_collider_ready(trigger: On<ColliderReady>, mut commands: Commands) {
    let event = trigger.event();

    if event.vertices.is_empty() || event.indices.is_empty() {
        return;
    }

    let triangles: Vec<[u32; 3]> = event
        .indices
        .chunks_exact(3)
        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
        .collect();

    let collider = Collider::trimesh(event.vertices.clone(), triangles);
    let chunk_pos = chunk_to_world(event.coord);

    commands.spawn((
        collider,
        RigidBody::Static,
        Transform::from_translation(chunk_pos),
        GlobalTransform::default(),
        ChunkCollider { coord: event.coord },
    ));
}

fn chunk_to_world(coord: IVec2) -> Vec3 {
    const CHUNK_SIZE: f32 = 16.0;
    Vec3::new(
        coord.x as f32 * CHUNK_SIZE,
        0.0,
        coord.y as f32 * CHUNK_SIZE,
    )
}

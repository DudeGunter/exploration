// To replace the shitty area.rs
// Features:
// - Create different chunk operations easily
// - Manage the rate of specific modifications dynamically and defined
use crate::{mesh::construct_mesh, terrain::*};
use bevy::prelude::*;
use console::message;

pub const CHUNK_SIZE: u32 = 32;

#[derive(Component)]
pub struct RenderDistance(pub u32);

#[derive(Component)]
pub struct Chunk {
    pub position: IVec3,
}

pub enum Lod {
    Low,
    Medium,
    High,
}

#[derive(Event)]
pub struct CreateEmpty(pub IVec3);

pub fn create_empty_chunk(trigger: On<CreateEmpty>, mut commands: Commands) {
    let transform =
        Transform::from_translation(trigger.0.as_vec3() * Vec3::splat(CHUNK_SIZE as f32));
    commands.spawn((
        Visibility::Visible,
        transform,
        Chunk {
            position: trigger.0,
        },
    ));
}

#[derive(Event, Copy, Clone)]
pub struct CreateTerrain(pub IVec3);

#[derive(Component)]
pub struct TerrainMesh;

pub fn create_terrain_chunk(
    trigger: On<CreateTerrain>,
    mut commands: Commands,
    chunks: Query<(Entity, &Chunk)>,
    terrain_meshes: Query<&ChildOf, With<TerrainMesh>>,
) {
    if let Some((entity, chunk)) = chunks.iter().find(|(_, chunk)| chunk.position == trigger.0) {
        if !terrain_meshes.iter().any(|mesh| mesh.0 == entity) {
            commands.trigger(message!("Creating Terrain Mesh"));
            commands.trigger(crate::terrain::RequestNoise {
                entity,
                position: chunk.position,
            });
            commands.entity(entity).observe(
                |trigger: On<RequestComplete>,
                 mut commands: Commands,
                 mut meshes: ResMut<Assets<Mesh>>,
                 mut materials: ResMut<Assets<StandardMaterial>>| {
                    let mesh = construct_mesh(&trigger.event().data);
                    let mesh_handle = meshes.add(mesh);
                    //let collider = Collider::trimesh_from_mesh(&mesh).unwrap();

                    commands.entity(trigger.entity).insert(children![(
                        Name::new("Terrain Mesh"),
                        TerrainMesh,
                        Mesh3d(mesh_handle),
                        //collider,
                        //RigidBody::Static,
                        MeshMaterial3d(
                            materials.add(StandardMaterial::from_color(Color::srgb(0.6, 1.0, 0.4))),
                        ),
                    )]);
                },
            );
        } else {
            commands.trigger(message!("Terrain mesh already exists"));
        }
    } else {
        commands.trigger(message!("Chunk doesn't exist! Creating chunk."));
        commands.trigger(CreateEmpty(trigger.0));
        commands.trigger(CreateTerrain(trigger.0));
    }
}

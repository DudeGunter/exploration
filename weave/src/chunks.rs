// To replace the shitty area.rs
// Features:
// - Create different chunk operations easily
// - Manage the rate of specific modifications dynamically and defined
use crate::{marching_cubes::*, *};
//use bevy::prelude::*;
use console::message;

pub const CHUNK_SIZE: u32 = 64;

// Container for chunks/world
#[derive(Component, Reflect)]
#[require(Name::new("Weave"), Transform, Visibility::Visible)]
pub struct Weave;

// Chunks have children for the purpose of attaching different placeable objects
// in the same chunks (the actual terrain is a child of the chunk)
#[derive(Component)]
pub struct Chunk {
    pub position: IVec3,
}

#[derive(Copy, Clone)]
pub enum Lod {
    Low,
    Medium,
    High,
}

#[derive(Event)]
pub struct CreateEmpty(pub IVec3);

pub fn create_empty_chunk(
    trigger: On<CreateEmpty>,
    mut commands: Commands,
    weave: Single<Entity, With<Weave>>,
) {
    let transform =
        Transform::from_translation(trigger.0.as_vec3() * Vec3::splat(CHUNK_SIZE as f32));
    let new_chunk = commands
        .spawn((
            Name::new(format!("Chunk {}", trigger.0)),
            Visibility::Visible,
            transform,
            Chunk {
                position: trigger.0,
            },
        ))
        .id();
    commands.entity(*weave).add_child(new_chunk);
}

#[derive(Event, Copy, Clone)]
pub struct CreateTerrain {
    pub position: IVec3,
    pub lod: Lod,
}

impl CreateTerrain {
    pub fn new(position: IVec3) -> Self {
        Self {
            position,
            lod: Lod::High,
        }
    }
}

#[derive(Component)]
pub struct TerrainMesh;

pub fn create_terrain_chunk(
    trigger: On<CreateTerrain>,
    mut commands: Commands,
    chunks: Query<(Entity, &Chunk)>,
    terrain_meshes: Query<&ChildOf, With<TerrainMesh>>,
) {
    if let Some((entity, chunk)) = chunks
        .iter()
        .find(|(_, chunk)| chunk.position == trigger.position)
    {
        if !terrain_meshes.iter().any(|mesh| mesh.0 == entity) {
            commands.trigger(message!("Creating Terrain Mesh"));
            commands.trigger(RequestNoise {
                entity,
                position: chunk.position,
            });
            let _lod = trigger.event().lod;
            commands.entity(entity).observe(
                move |trigger: On<RequestComplete>,
                      mut commands: Commands,
                      material: Res<TerrainMeshMaterial>| {
                    let terrain = commands
                        .spawn((
                            Name::new("Terrain Mesh"),
                            TerrainMesh,
                            Mesh3d(trigger.handle.clone()),
                            MeshMaterial3d(material.0.clone()),
                        ))
                        .id();
                    commands.entity(entity).add_child(terrain);
                },
            );
        } else {
            commands.trigger(message!("Terrain mesh already exists"));
        }
    } else {
        commands.trigger(message!("Chunk doesn't exist! Creating chunk."));
        commands.trigger(CreateEmpty(trigger.position));
        commands.trigger(*trigger.event());
    }
}

// To replace the shitty area.rs
// Features:
// - Create different chunk operations easily
// - Manage the rate of specific modifications dynamically and defined
use bevy::prelude::*;

#[derive(Component)]
pub struct RenderDistance(pub u32);

#[derive(Component)]
pub struct Chunk {
    pub position: IVec2,
}

pub enum Lod {
    Low,
    Medium,
    High,
}

pub enum Status {
    Active,
    Inactive,
    PendingOperation(Box<dyn Operation>),
}

pub trait Operation: Send + Sync + 'static {}

#[derive(Event)]
pub struct CreateEmpty(pub IVec2);

impl Operation for CreateEmpty {}

pub fn create_empty_chunk(trigger: On<CreateEmpty>, mut commands: Commands) {
    commands.spawn(Chunk {
        position: trigger.0,
    });
}

#[derive(Event)]
pub struct CreateTerrain(pub IVec2);

impl Operation for CreateTerrain {}

pub fn create_terrain_chunk(
    trigger: On<CreateTerrain>,
    mut commands: Commands,
    chunks: Query<(Entity, &Chunk)>,
) {
    //    for (entity, chunk) in chunks {
    //        if chunk.position == trigger.0 {
    //            commands.entity(entity).insert()
    //        }
    //    }
}

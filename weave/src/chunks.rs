// To replace the shitty area.rs
// Features:
// - Create different chunk operations easily
// - Manage the rate of specific modifications dynamically and defined
use bevy::prelude::*;
use console::message;

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

#[derive(Component)]
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

#[derive(Event, Copy, Clone)]
pub struct CreateTerrain(pub IVec2);

impl Operation for CreateTerrain {}

pub fn create_terrain_chunk(
    trigger: On<CreateTerrain>,
    mut commands: Commands,
    chunks: Query<(Entity, &Chunk)>,
) {
    commands.trigger(message("Attempting to create chunk"));
    if let Some((entity, chunk)) = chunks.iter().find(|(_, chunk)| chunk.position == trigger.0) {
        commands.trigger(crate::terrain::RequestNoise {
            entity,
            position: chunk.position.xxy(),
        });
        commands
            .entity(entity)
            .insert(Status::PendingOperation(Box::new(*trigger.event())))
            .observe(crate::mesh::recieve_mesh)
            .observe(
                |trigger: On<crate::terrain::RequestComplete>, mut commands: Commands| {
                    commands.entity(trigger.entity).insert(Status::Active);
                },
            );
    } else {
        commands.trigger(CreateEmpty(trigger.0));
        commands.trigger(CreateTerrain(trigger.0));
    }
}

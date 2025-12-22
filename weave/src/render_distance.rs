use crate::chunks::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct RenderDistance(pub i32);

pub fn manage_render_distance_terrain_spawning(
    mut commands: Commands,
    query: Query<(&RenderDistance, &GlobalTransform)>,
) {
    for (distance, transform) in query {
        let chunk_position = (transform.translation() / Vec3::splat(CHUNK_SIZE as f32))
            .round()
            .as_ivec3();
        for x in -distance.0 as i32..=distance.0 as i32 {
            for y in -distance.0 as i32..=distance.0 as i32 {
                for z in -distance.0 as i32..=distance.0 as i32 {
                    let chunk_position = chunk_position + IVec3::new(x, y, z);
                    commands.trigger(CreateTerrain::new(chunk_position));
                }
            }
        }
    }
}

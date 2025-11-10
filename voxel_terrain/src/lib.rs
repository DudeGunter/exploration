//
// Wants:
// Voxel specific types
// Lod at a distance,
// Color!,
// Png Textures,
// 3d option?/more noise
//

use bevy::prelude::*;
use manager::*;

mod chunk;
mod manager;
mod terrain;

pub struct VoxelTerrainPlugin;

impl Plugin for VoxelTerrainPlugin {
    fn build(&self, app: &mut App) {
        // These don't have to be fixed, just makes it run a lil less
        app.insert_resource(ChunkManager::default());
        app.insert_resource(ChunkSpawnLimiter::default());
        app.add_systems(
            FixedUpdate,
            (
                spawn_with_limits,
                make_chunks_dormant,
                make_dormant_chunks_active,
                handle_spawning_chunk,
            )
                .chain()
                .run_if(|terrain: Query<&terrain::Terrain>| !terrain.is_empty()),
        );
        app.add_observer(terrain::setup);
    }
}

pub mod prelude {
    pub use crate::VoxelTerrainPlugin;
    pub use crate::chunk::Chunk;
    pub use crate::manager::{AreaManaged, Observer};
    pub use crate::terrain::{Terrain, TerrainMaterial};
}

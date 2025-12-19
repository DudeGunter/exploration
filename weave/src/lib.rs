use bevy::prelude::*;

pub mod chunks;
pub mod mesh;
pub mod tables;
pub mod terrain;

pub struct WeavePlugin;

impl Plugin for WeavePlugin {
    fn build(&self, app: &mut App) {
        // Field compute, there's seperate plugin because the the render node edits
        app.add_plugins(terrain::plugin);
        app.insert_resource(terrain::TerrainNoiseParams::default());
        app.add_systems(PreUpdate, terrain::clear_queue);
        app.add_observer(terrain::handle_requests);

        // Chunk
        app.add_observer(chunks::create_empty_chunk);
    }
}

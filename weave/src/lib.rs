use bevy::prelude::*;
use console::*;

pub mod chunks;
pub mod mesh;
pub mod tables;
pub mod terrain;

pub struct WeavePlugin;

impl Plugin for WeavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, add_commands);
        // Field compute, there's seperate plugin because the the render node edits
        app.add_plugins(terrain::plugin);
        app.insert_resource(terrain::TerrainNoiseParams::default());
        app.add_systems(PreUpdate, terrain::clear_queue);
        app.add_observer(terrain::handle_requests);

        // Chunk
        app.add_observer(chunks::create_empty_chunk);
        app.add_observer(chunks::create_terrain_chunk);
    }
}

pub fn add_commands(mut console_config: ResMut<ConsoleConfig>) {
    console_config.insert_command("create_chunk", create_chunk_command);
}

pub fn create_chunk_command(In(arguments): In<String>, mut commands: Commands) {
    let parts: Vec<&str> = arguments.split_whitespace().collect();
    //commands.trigger(message(format!("{:?}", parts)));
    if let Ok((x, y)) = parts[0]
        .parse::<i32>()
        .and_then(|x| parts[1].parse::<i32>().map(|y| (x, y)))
    {
        commands.trigger(chunks::CreateTerrain(IVec2::new(x, y)));
    } else {
        commands.trigger(message!("Invalid arguments"));
    }
}

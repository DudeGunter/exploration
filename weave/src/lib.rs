use bevy::prelude::*;
use console::*;

pub mod chunks;
pub mod mesh;
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
        app.add_systems(Startup, |mut commands: Commands| {
            commands.spawn(crate::chunks::Weave);
        });
        app.add_observer(chunks::create_empty_chunk);
        app.add_observer(chunks::create_terrain_chunk);

        // Mesh
        app.add_systems(Startup, mesh::setup_terrain_mesh_material);
    }
}

pub fn add_commands(mut console_config: ResMut<ConsoleConfig>) {
    console_config.insert_command("create_chunk", create_chunk_command);
}

pub fn create_chunk_command(In(arguments): In<String>, mut commands: Commands) {
    let coords: Vec<i32> = parse_number_arguments(&arguments);

    match coords.len() {
        3 => {
            let pos = IVec3::new(coords[0], coords[1], coords[2]);
            commands.trigger(chunks::CreateTerrain(pos));
        }
        6 => {
            let min = ivec3(coords[0], coords[1], coords[2]);
            let max = ivec3(coords[3], coords[4], coords[5]);
            for x in min.x..=max.x {
                for y in min.y..=max.y {
                    for z in min.z..=max.z {
                        commands.trigger(chunks::CreateTerrain(ivec3(x, y, z)));
                    }
                }
            }
        }
        _ => commands.trigger(message!("Usage: `x y z` or `x y z to x y z`")),
    }
}

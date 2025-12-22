use bevy::prelude::*;
use console::*;

pub mod chunks;
pub mod mesh;
pub mod render_distance;
pub mod terrain;

pub struct WeavePlugin;

impl Plugin for WeavePlugin {
    fn build(&self, app: &mut App) {
        // Configuration
        app.init_state::<WeaveStates>();
        app.configure_sets(OnEnter(WeaveStates::Init), WeaveSets::Init);
        app.configure_sets(
            PreUpdate,
            WeaveSets::PreUpdate.run_if(in_state(WeaveStates::Update)),
        );
        app.configure_sets(
            Update,
            WeaveSets::Update.run_if(in_state(WeaveStates::Update)),
        );

        // Systems
        app.add_systems(Startup, add_commands); // so I can use a command to change the state
        app.add_systems(
            Startup,
            (
                mesh::setup_terrain_mesh_material,
                spawn_weave,
                mesh::setup_channel,
            )
                .in_set(WeaveSets::Init),
        );
        app.add_systems(PreUpdate, terrain::clear_queue.in_set(WeaveSets::PreUpdate));
        app.add_systems(
            Update,
            (
                render_distance::manage_render_distance_terrain_spawning,
                mesh::handle_received_mesh,
            )
                .in_set(WeaveSets::Update),
        );

        // Field compute, there's seperate plugin because the the render node edits
        app.add_plugins(terrain::plugin);
        app.insert_resource(terrain::TerrainNoiseParams::default());

        // Observers
        app.add_observer(terrain::handle_requests);
        app.add_observer(chunks::create_empty_chunk);
        app.add_observer(chunks::create_terrain_chunk);
        app.add_observer(chunks::compute_terrain_mesh);
    }
}

// Managing sets and states
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaveStates {
    #[default]
    Init,
    Update,
}

#[derive(SystemSet, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaveSets {
    #[default]
    Init,
    PreUpdate,
    Update,
}

// Container for weave logic
pub fn spawn_weave(mut commands: Commands) {
    commands.spawn(crate::chunks::Weave);
}

// Console implementations
pub fn add_commands(mut console_config: ResMut<ConsoleConfig>) {
    console_config.insert_command("create_chunk", create_chunk_command);
    console_config.insert_command(
        "start_weave",
        |In(_): In<String>, mut commands: Commands, mut next: ResMut<NextState<WeaveStates>>| {
            next.set(WeaveStates::Update);
            commands.trigger(success_message!("Successfully changed state to Update"));
        },
    );
}

pub fn create_chunk_command(In(arguments): In<String>, mut commands: Commands) {
    let coords: Vec<i32> = parse_number_arguments(&arguments);

    match coords.len() {
        3 => {
            let pos = IVec3::new(coords[0], coords[1], coords[2]);
            commands.trigger(chunks::CreateTerrain::new(pos));
        }
        6 => {
            let min = ivec3(coords[0], coords[1], coords[2]);
            let max = ivec3(coords[3], coords[4], coords[5]);
            for x in min.x..=max.x {
                for y in min.y..=max.y {
                    for z in min.z..=max.z {
                        commands.trigger(chunks::CreateTerrain::new(ivec3(x, y, z)));
                    }
                }
            }
        }
        _ => commands.trigger(message!("Usage: `x y z` or `x y z to x y z`")),
    }
}

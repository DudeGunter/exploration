use avian3d::prelude::*;
use bevy::{
    camera::Exposure, core_pipeline::tonemapping::Tonemapping, pbr::Atmosphere, prelude::*,
};
use bevy_flycam::prelude::*;
use bevy_hui::prelude::*;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use networking::prelude::*;
use voxel_terrain::prelude::*;
// Everything and anything in bevy diddy blud

fn main() -> AppExit {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        PhysicsPlugins::default(),
        PhysicsPickingPlugin,
        NoCameraPlayerPlugin,
        EguiPlugin::default(),
        HuiPlugin,
        WorldInspectorPlugin::new(),
        VoxelTerrainPlugin,
        NetworkingPlugin,
    ));
    app.add_systems(Startup, (setup, hui_setup));
    app.run()
}

fn setup(mut commands: Commands) {
    commands.trigger(Host::default());
    commands.spawn(Terrain);
    commands.spawn((
        FlyCam,
        Observer,
        AreaManaged::Circle(25.0),
        Camera3d::default(),
        Atmosphere::EARTH,
        AmbientLight {
            brightness: 4000.0,
            color: Color::WHITE,
            ..default()
        },
        Exposure::SUNLIGHT,
        Tonemapping::AcesFitted,
        Transform::from_xyz(13.0, 1.0, 26.0).looking_at(Vec3::new(0.0, -8.0, 0.0), Vec3::Y),
    ));
    // Directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 14e4,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(MovementSettings {
        speed: 100.0,
        ..default()
    });
}

#[derive(Component)]
#[require(Name::new("Basic Button"))]
pub struct BasicButton;

fn hui_setup(
    mut cmds: Commands,
    server: Res<AssetServer>,
    mut html_comps: HtmlComponents,
    mut html_funcs: HtmlFunctions,
) {
    // simple register
    html_comps.register("basic_button", server.load("basic_button.html"));

    // advanced register, with spawn functions
    html_comps.register_with_spawn_fn(
        "basic_button",
        server.load("basic_button.html"),
        |mut entity_commands| {
            entity_commands.insert(BasicButton);
        },
    );

    // create a system binding that will change the game state.
    // any (one-shot) system with `In<Entity>` is valid!
    // the entity represents the node, the function is called on
    html_funcs.register("start_game", |In(entity): In<Entity>| {
        info!("Go into game now");
    });

    cmds.spawn(HtmlNode(server.load("menu.html")));
}

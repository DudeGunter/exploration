#![allow(unused)]
use avian3d::prelude::*;
use bevy::{
    camera::Exposure, core_pipeline::tonemapping::Tonemapping, pbr::Atmosphere, prelude::*,
};
use bevy_flycam::prelude::*;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use networking::prelude::*;
use portal::PortalPlugin;
//use voxel_terrain::prelude::*;
use weave::WeavePlugin;
// Everything and anything in bevy diddy blud

fn main() -> AppExit {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        PhysicsPlugins::default(),
        PhysicsPickingPlugin,
        NoCameraPlayerPlugin,
        EguiPlugin::default(),
        WorldInspectorPlugin::new(),
        //VoxelTerrainPlugin,
        //NetworkingPlugin,
        //WeavePlugin,
        //console::ConsolePlugin,
        //PortalPlugin,
    ));
    app.add_systems(Startup, setup);
    app.run()
}

fn setup(mut commands: Commands) {
    commands.trigger(Host::default());
    commands.spawn((
        FlyCam,
        weave::render_distance::RenderDistance(3),
        //Observer,
        //AreaManaged::Circle(25.0),
        //weave::area::Observer,
        Camera3d::default(),
        Atmosphere::EARTH,
        AmbientLight {
            brightness: 4000.0,
            color: Color::WHITE,
            ..default()
        },
        Exposure::SUNLIGHT,
        Tonemapping::AcesFitted,
        Transform::default().looking_at(Vec3::new(0.0, 0.0, 1.0), Vec3::Y),
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
    commands.insert_resource(MovementSettings { ..default() });
}

fn spawn_example_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("burnout_3_downtown.glb"))),
        Transform::from_scale(Vec3::splat(10.0)).with_translation(Vec3::new(312.0, -15.0, -87.0)),
    ));
}

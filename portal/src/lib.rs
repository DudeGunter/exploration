// Portals
/* Blocking:
 * +Add example world in explore src for reference testing
 * +Character controller potentially??
 * Then:
 * Get a basic mirror or window via camera textures
 * One to one portals
 * Create API for simpler use
 *
 * Longer down the line
 * Stencil approach redesign using shaders/gpu programming (something I need to learn)
 *
 */
use bevy::{prelude::*, render::render_resource::TextureFormat};

pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_portal);
    }
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component)]
pub struct Portal;

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Input {
    entity: Entity,
    image: Handle<Image>,
}

/// Source of truth for the camera output -> input
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Output(Entity);

pub fn create_portal(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let output_image = images.add(Image::new_target_texture(
        512,
        512,
        TextureFormat::bevy_default(),
    ));

    let material = materials.add(StandardMaterial {
        base_color_texture: Some(output_image.clone()),
        unlit: false,
        ..default()
    });

    let camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                target: output_image.clone().into(),
                ..default()
            },
        ))
        .id();

    let portal = commands
        .spawn((
            Name::new("Output portal"),
            Portal,
            MeshMaterial3d(material),
            Mesh3d(meshes.add(Plane3d::new(Vec3::new(0.0, 1.0, 0.0), Vec2::new(1.0, 1.0)))),
            Transform::from_xyz(0.0, 10.0, 0.0),
            Input {
                entity: camera,
                image: output_image,
            },
        ))
        .id();

    commands.entity(camera).insert(Output(portal));
}

#[allow(unused)]
pub fn update_portal(
    mut commands: Commands,
    mut portals: Query<(Entity), (With<Portal>, With<Input>)>,
) {
}

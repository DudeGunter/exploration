use bevy::prelude::*;
use noiz::prelude::*;

// Head, this starts everything
#[derive(Component, Reflect, Debug)]
pub struct Terrain;

// this ideally would adapt to what its set to on line 20
#[derive(Resource, Reflect, Deref, DerefMut, Copy, Clone)]
pub struct TerrainNoise(
    pub Noise<MixCellGradients<OrthoGrid<()>, Smoothstep, QuickGradients, false>>,
);

#[derive(Resource, Reflect, Deref, DerefMut)]
pub struct TerrainMaterial(pub Handle<StandardMaterial>);

pub fn setup(
    trigger: On<Add, Terrain>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TerrainNoise(Noise::from(common_noise::Perlin::default())));
    commands.insert_resource(TerrainMaterial(materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5),
        ..default()
    })));
    commands
        .entity(trigger.entity)
        .insert((Transform::default(), Visibility::Visible));
}

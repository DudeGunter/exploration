use bevy::prelude::*;
use noiz::prelude::*;

// Head, this starts everything
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default)]
#[require(Name::new("Terrain"))]
pub struct Terrain;

// this ideally would adapt to what its set to on line 20
#[derive(Resource, Reflect, Deref, DerefMut, Copy, Clone)]
pub struct TerrainNoise(
    pub  Noise<
        PerCellPointDistances<Voronoi<false, OrthoGrid<()>>, EuclideanLength, WorleyLeastDistance>,
    >,
);

#[derive(Resource, Reflect, Deref, DerefMut)]
pub struct TerrainMaterial(pub Handle<StandardMaterial>);

pub fn setup(
    trigger: On<Add, Terrain>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut noise = Noise::from(common_noise::Worley::default());
    noise.set_frequency(0.05);
    noise.set_period(1.0);
    commands.insert_resource(TerrainNoise(noise));
    commands.insert_resource(TerrainMaterial(materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5),
        ..default()
    })));
    commands
        .entity(trigger.entity)
        .insert((Transform::default(), Visibility::Visible));
}

use crate::noise::TerrainNoise;
use bevy::prelude::*;
use noiz::prelude::*;

mod collider;

pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelNoise>();
        app.add_observer(collider::on_collider_ready);
    }
}

// Example impl
pub type VoxelNoise = TerrainNoise<
    PerCellPointDistances<Voronoi<false, OrthoGrid<()>>, EuclideanLength, WorleyLeastDistance>,
>;

impl Default for VoxelNoise {
    fn default() -> Self {
        Self(Noise::from(common_noise::Worley::default()))
    }
}

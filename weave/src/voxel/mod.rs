use crate::noise::TerrainNoise;
use bevy::prelude::*;
use noiz::prelude::*;

pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, voxel_noise_init);
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

pub fn voxel_noise_init(mut cmds: Commands) {
    cmds.init_resource::<VoxelNoise>();
}

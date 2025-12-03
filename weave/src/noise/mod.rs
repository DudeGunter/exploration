use bevy::prelude::*;
use noiz::prelude::*;

// This I allows it so
#[derive(Resource, Deref, DerefMut, Clone, Copy)]
pub struct TerrainNoise<N>(pub Noise<N>);

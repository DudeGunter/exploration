use bevy::prelude::*;
use noiz::prelude::*;

pub mod field_compute;

// No longer gonna be used, going for compute shader approach
#[derive(Resource, Deref, DerefMut, Clone, Copy)]
pub struct TerrainNoise<N>(pub Noise<N>);

// blah blah blah more functionality can be added Later.

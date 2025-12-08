use crate::area::RequestChunk;
use bevy::prelude::*;
use field_compute::*;

pub mod field_compute;

/// Handles the compute shader noise
pub struct TerrainNoisePlugin<T: TerrainNoiseParams + Clone>(pub T);

impl<T: TerrainNoiseParams + Clone> Plugin for TerrainNoisePlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0.clone());
        app.add_observer(queue_noise_field_request::<T>);
    }
}

pub trait TerrainNoiseParams: Resource {
    fn scale(&self) -> f32;
    fn frequency(&self) -> f32;
    fn amplitude(&self) -> f32;
    fn octaves(&self) -> u32;
}

#[derive(Event)]
pub struct RequestGenerate<T: TerrainNoiseParams> {
    position: IVec3,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: TerrainNoiseParams + Clone> RequestGenerate<T> {
    pub fn new(position: IVec3) -> Self {
        Self {
            position,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn new_2d(position: IVec3) -> Self {
        Self {
            position,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn new_3d(position: IVec3) -> Self {
        Self {
            position,
            _phantom: std::marker::PhantomData,
        }
    }
}

pub fn queue_noise_field_request<C: crate::terrain::TerrainNoiseParams>(
    trigger: On<RequestGenerate<C>>,
    mut queue: ResMut<NoiseFieldQueue>,
    params: Res<C>,
) {
    let coord = trigger.position;

    let noise_params = NoiseParams {
        chunk_x: coord.x,
        chunk_y: coord.y,
        chunk_z: coord.z,
        scale: params.scale(),
        frequency: params.frequency(),
        amplitude: params.amplitude(),
        octaves: params.octaves(),
        _padding: 0,
    };

    queue
        .pending
        .push((IVec3::new(coord.x, coord.y, coord.z), noise_params));
}

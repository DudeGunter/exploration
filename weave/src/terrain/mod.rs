use bevy::prelude::*;
use field_compute::*;

pub mod field_compute;

/// Handles the compute shader noise
pub struct TerrainNoisePlugin<T: TerrainNoiseParams + Clone>(pub T);

impl<T: TerrainNoiseParams + Clone> Plugin for TerrainNoisePlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0.clone());
        app.add_observer(queue_chunk::<T>);
        app.add_observer(on_complete::<T>);
    }
}

pub trait TerrainNoiseParams: Resource {
    fn scale(&self) -> f32;
    fn frequency(&self) -> f32;
    fn amplitude(&self) -> f32;
    fn octaves(&self) -> u32;
}

#[derive(Event)]
pub struct RequestNoise<T: TerrainNoiseParams> {
    position: IVec3,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: TerrainNoiseParams + Clone> RequestNoise<T> {
    pub fn new(position: IVec2) -> Self {
        Self {
            position: position.xyx().with_z(0),
            _phantom: std::marker::PhantomData,
        }
    }

    #[allow(unused)]
    pub fn new_3d(position: IVec3) -> Self {
        Self {
            position,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[derive(Event)]
pub struct RequestComplete<T: TerrainNoiseParams> {
    pub position: IVec3,
    pub data: Vec<f32>,
    _phantom: std::marker::PhantomData<T>,
}

fn queue_chunk<C: TerrainNoiseParams>(
    trigger: On<RequestNoise<C>>,
    mut commands: Commands,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut requests: ResMut<NoiseRequests>,
    params: Res<C>,
) {
    let coord = trigger.event().position;
    let chunk_coord = IVec3::new(coord.x, coord.y, 0);

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

    let mut buffer =
        ShaderStorageBuffer::from(vec![0f32; (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE) as usize]);
    buffer.buffer_description.usage |= BufferUsages::COPY_SRC;
    let buffer_handle = buffers.add(buffer);

    let entity = commands.spawn((Readback::buffer(buffer_handle),)).id();

    requests.0.insert(entity, (chunk_coord, noise_params));
}

// Terrain Noise Params could collide here!!!
fn on_complete<C: TerrainNoiseParams>(
    trigger: On<ReadbackComplete>,
    mut commands: Commands,
    mut requests: ResMut<NoiseRequests>,
) {
    if let Some((position, _params)) = requests.0.remove(&trigger.entity) {
        let data: Vec<f32> = trigger.to_shader_type();
        commands.entity(trigger.entity).despawn();
        commands.trigger(RequestComplete::<C> {
            position,
            data,
            _phantom: std::marker::PhantomData,
        });
    }
}

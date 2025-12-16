use bevy::prelude::*;
use field_compute::*;

pub mod field_compute;

/// Handles the compute shader noise
pub struct TerrainNoisePlugin<T: TerrainNoiseParams + Clone>(pub T);

impl<T: TerrainNoiseParams + Clone> Plugin for TerrainNoisePlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0.clone());
        app.add_observer(queue_chunk::<T>);
        app.add_systems(Update, on_complete::<T>);
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
    params: Res<C>,
    mut compute_worker: ResMut<AppComputeWorker<FieldComputeWorker>>,
) {
    let coord = trigger.event().position;

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
    let stuff: Vec<f32> = vec![0f32; (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE) as usize];

    compute_worker.write("params", &noise_params);
    compute_worker.write_slice("noise_field", &stuff);
    compute_worker.execute();
}

// Terrain Noise Params could collide here!!!
fn on_complete<C: TerrainNoiseParams>(
    mut commands: Commands,
    compute_worker: Res<AppComputeWorker<FieldComputeWorker>>,
) {
    if compute_worker.ready() {
        info!("Its ready");
        let params = compute_worker.read::<NoiseParams>("params");
        let noise_field: Vec<f32> = compute_worker.read_vec("noise_field");

        commands.trigger(RequestComplete::<C> {
            position: IVec3::new(params.chunk_x, params.chunk_y, params.chunk_z),
            data: noise_field,
            _phantom: std::marker::PhantomData,
        });
        info!("Success");
    }
}

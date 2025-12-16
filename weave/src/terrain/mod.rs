use bevy::prelude::*;
use field_compute::*;

pub mod field_compute;

// It should be noted...
// Previously, we didn't use bevy_app_compute or smth,
// it was a more low level integration with the bevy render graph
// I could never get that working properly because of readback issues,
// so im using bevy_app_compute now.
// the issue is you can't run in parallel or it would be more ugly requireing multiple worker resources of
// an unknown amount, making it a queue for now.
//
// It should also be noted...
// I would have avoided generics as a whole if you could query off data + components and not just components
// You could just do components than filter for the appropriate data but I also felt as though these noise funcs
// could be applied to potentially more than just terrain.
// Also... it seems that component + data querys are coming, althouth the syntax is a bit strange

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

#[allow(unused)]
impl<T: TerrainNoiseParams + Clone> RequestNoise<T> {
    pub fn new(position: IVec2) -> Self {
        Self {
            position: position.xxy().with_y(0),
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

#[derive(Event)]
pub struct RequestComplete<T: TerrainNoiseParams> {
    pub position: IVec3,
    pub data: Vec<f32>,
    _phantom: std::marker::PhantomData<T>,
}

pub fn testing<C: TerrainNoiseParams>(
    trigger: On<RequestNoise<C>>,
    mut commands: Commands,
    mut queue: ResMut<NoiseFieldQueue>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    info!("Generating noise field");
    let buffer: Vec<f32> = vec![0.0; (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE) as usize];
    let mut buffer = ShaderStorageBuffer::from(buffer);
    buffer.buffer_description.usage =
        BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
    let buffer = buffers.add(buffer);
    commands
        .spawn((
            Readback::buffer(buffer.clone()),
            Params(NoiseParams::default()),
        ))
        .observe(
            |trigger: On<ReadbackComplete>,
             mut commands: Commands,
             mut queue: ResMut<NoiseFieldQueue>,
             query: Query<&Params>| {
                let data: Vec<f32> = trigger.to_shader_type();
                // Unnecessary if you just wait a bit
                if data.iter().sum::<f32>() == 0.0 {
                    return;
                }
                queue
                    .queue
                    .retain(|(params, _)| *params != query.single().unwrap().0);

                commands.entity(trigger.entity).despawn();
            },
        );

    queue.queue.push((NoiseParams::default(), buffer));
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
        let params = compute_worker.read::<NoiseParams>("params");
        let noise_field: Vec<f32> = compute_worker.read_vec("noise_field");

        commands.trigger(RequestComplete::<C> {
            position: IVec3::new(params.chunk_x, params.chunk_y, params.chunk_z),
            data: noise_field,
            _phantom: std::marker::PhantomData,
        });
    }
}

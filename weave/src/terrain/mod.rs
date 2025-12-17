use bevy::prelude::*;
use field_compute::*;

pub mod field_compute;

// It should be noted... (fixed now) (put in custom compute system)
// Previously, we didn't use bevy_app_compute or smth,
// it was a more low level integration with the bevy render graph
// I could never get that working properly because of readback issues,
// so im using bevy_app_compute now.
// the issue is you can't run in parallel or it would be more ugly requireing multiple worker resources of
// an unknown amount, making it a queue for now.
//
// It should also be noted... (I want this next! have to think about relationship with area, world, and terrain)
// I would have avoided generics as a whole if you could query off data + components and not just components
// You could just do components than filter for the appropriate data but I also felt as though these noise funcs
// could be applied to potentially more than just terrain.
// Also... it seems that component + data querys are coming, althouth the syntax is a bit strange
//
// TODO list:
// Make it not generic/explore other options
// simplify and remove excess fat for mantainance
// better 2d suport
// more general:
// shader pass systems
// more compute shaders
// make my own bevy_compute_workers with better support

/// Handles the compute shader noise
pub struct TerrainNoisePlugin;

impl Plugin for TerrainNoisePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TerrainNoiseParams::default());
        app.add_systems(PreUpdate, clear_queue);
        app.add_observer(handle_requests);
    }
}

#[derive(Resource)]
pub struct TerrainNoiseParams {
    pub scale: f32,
    pub frequency: f32,
    pub amplitude: f32,
    pub octaves: u32,
}

impl Default for TerrainNoiseParams {
    fn default() -> Self {
        Self {
            scale: 0.05,
            frequency: 1.0,
            amplitude: 1.0,
            octaves: 1,
        }
    }
}

#[derive(Event)]
pub struct RequestNoise {
    pub position: IVec3,
}

#[derive(Event)]
pub struct RequestComplete {
    pub position: IVec3,
    pub data: Vec<f32>,
}

pub fn clear_queue(mut queue: ResMut<NoiseFieldQueue>) {
    queue.queue.clear();
}

// This could be broken up
pub fn handle_requests(
    trigger: On<RequestNoise>,
    mut commands: Commands,
    params: Res<TerrainNoiseParams>,
    mut queue: ResMut<NoiseFieldQueue>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let buffer: Vec<f32> = vec![0.0; (FIELD_SIZE * FIELD_SIZE * FIELD_SIZE) as usize];
    let mut buffer = ShaderStorageBuffer::from(buffer);
    buffer.buffer_description.usage =
        BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
    let buffer = buffers.add(buffer);
    let coord = trigger.event().position;

    let noise_params = NoiseParams {
        chunk_x: coord.x,
        chunk_y: coord.y,
        chunk_z: coord.z,
        scale: params.scale,
        frequency: params.frequency,
        amplitude: params.amplitude,
        octaves: params.octaves,
        _padding: 0,
    };
    commands
        .spawn((Readback::buffer(buffer.clone()), Params(noise_params)))
        .observe(
            |trigger: On<ReadbackComplete>, mut commands: Commands, query: Query<&Params>| {
                let data: Vec<f32> = trigger.to_shader_type();
                // Unnecessary if you just wait a bit
                if data.iter().sum::<f32>() == 0.0 {
                    warn!("Likely didn't generate properly, all the noise data is zero!");
                    warn!("This is likely caused by the shader not being compiled yet or smth");
                }
                let params = query.get(trigger.entity).unwrap().0;
                commands.trigger(RequestComplete {
                    position: IVec3::new(params.chunk_x, params.chunk_y, params.chunk_z),
                    data,
                });

                commands.entity(trigger.entity).despawn();
            },
        );

    queue.queue.push((noise_params, buffer));
}

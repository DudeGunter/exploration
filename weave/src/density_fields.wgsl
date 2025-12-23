// ============= DENSITY FIELD GENERATION =============
// Shader 1: Generate density field for a chunk

struct NoiseParams {
    chunk_x: i32,
    chunk_y: i32,
    chunk_z: i32,
    scale: f32,
    frequency: f32,
    amplitude: f32,
    octaves: u32,
}

@group(0) @binding(0)
var<uniform> params: NoiseParams;

@group(0) @binding(1)
var<storage, read_write> density_field: array<f32>;

const FIELD_SIZE: u32 = 65u;
const FIELD_STRIDE: u32 = FIELD_SIZE * FIELD_SIZE;

// ============= NOISE FUNCTIONS =============

fn hash33(p: vec3<f32>) -> vec3<f32> {
    var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx) * 2.0 - 1.0;
}

fn perlin_noise(p: vec3<f32>) -> f32 {
    let pi = floor(p);
    let pf = fract(p);

    let w = pf * pf * pf * (pf * (pf * 6.0 - 15.0) + 10.0);

    let g000 = dot(hash33(pi + vec3<f32>(0.0, 0.0, 0.0)), pf - vec3<f32>(0.0, 0.0, 0.0));
    let g100 = dot(hash33(pi + vec3<f32>(1.0, 0.0, 0.0)), pf - vec3<f32>(1.0, 0.0, 0.0));
    let g010 = dot(hash33(pi + vec3<f32>(0.0, 1.0, 0.0)), pf - vec3<f32>(0.0, 1.0, 0.0));
    let g110 = dot(hash33(pi + vec3<f32>(1.0, 1.0, 0.0)), pf - vec3<f32>(1.0, 1.0, 0.0));
    let g001 = dot(hash33(pi + vec3<f32>(0.0, 0.0, 1.0)), pf - vec3<f32>(0.0, 0.0, 1.0));
    let g101 = dot(hash33(pi + vec3<f32>(1.0, 0.0, 1.0)), pf - vec3<f32>(1.0, 0.0, 1.0));
    let g011 = dot(hash33(pi + vec3<f32>(0.0, 1.0, 1.0)), pf - vec3<f32>(0.0, 1.0, 1.0));
    let g111 = dot(hash33(pi + vec3<f32>(1.0, 1.0, 1.0)), pf - vec3<f32>(1.0, 1.0, 1.0));

    let c00 = mix(g000, g100, w.x);
    let c10 = mix(g010, g110, w.x);
    let c01 = mix(g001, g101, w.x);
    let c11 = mix(g011, g111, w.x);

    let c0 = mix(c00, c10, w.y);
    let c1 = mix(c01, c11, w.y);

    return mix(c0, c1, w.z);
}

fn fbm_3d(p: vec3<f32>, octaves: u32) -> f32 {
    var value = 0.0;
    var amplitude = 1.0;
    var frequency = 1.0;
    var max_value = 0.0;

    for (var i = 0u; i < octaves; i = i + 1u) {
        value = value + amplitude * perlin_noise(p * frequency);
        max_value = max_value + amplitude;
        amplitude = amplitude * 0.5;
        frequency = frequency * 2.0;
    }

    return value / max_value;
}

@compute @workgroup_size(4, 4, 4)
fn density_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id >= vec3<u32>(FIELD_SIZE)) {
        return;
    }

    let size_f = f32(FIELD_SIZE - 1u);
    let chunk_offset = vec3<f32>(
        f32(params.chunk_x),
        f32(params.chunk_y),
        f32(params.chunk_z)
    ) * size_f;

    let local_pos = vec3<f32>(global_id);
    let world_pos = (chunk_offset + local_pos) * params.scale;

    let density = fbm_3d(world_pos, params.octaves);

    // Linear indexing into 3D array
    let idx = global_id.x + global_id.y * FIELD_SIZE + global_id.z * FIELD_STRIDE;
    density_field[idx] = density;
}

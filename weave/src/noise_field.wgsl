// Noise field generation compute shader
// Generates Perlin-like noise for terrain density values

struct NoiseParams {
    chunk_x: i32,
    chunk_y: i32,
    chunk_z: i32,
    scale: f32,
    frequency: f32,
    amplitude: f32,
    octaves: u32,
    _padding: u32,
}

@group(0) @binding(0)
var<uniform> params: NoiseParams;

@group(0) @binding(1)
var<storage, read_write> noise_field: array<f32>;

const FIELD_SIZE: u32 = 65u; // CHUNK_SIZE + 1

// Simple hash function for pseudo-random values
fn hash(v: vec3<f32>) -> f32 {
    let h = dot(v, vec3<f32>(127.1, 311.7, 74.7));
    return fract(sin(h) * 43758.5453123);
}

// Smoothstep interpolation (Hermite curve)
fn smoothstep(t: f32) -> f32 {
    return t * t * (3.0 - 2.0 * t);
}

// Basic Perlin noise implementation
fn perlin_noise(p: vec3<f32>) -> f32 {
    let pi = floor(p);
    let pf = fract(p);

    // Interpolation weights
    let w = vec3<f32>(
        smoothstep(pf.x),
        smoothstep(pf.y),
        smoothstep(pf.z)
    );

    // Sample 8 corners
    let c000 = hash(pi + vec3<f32>(0.0, 0.0, 0.0));
    let c100 = hash(pi + vec3<f32>(1.0, 0.0, 0.0));
    let c010 = hash(pi + vec3<f32>(0.0, 1.0, 0.0));
    let c110 = hash(pi + vec3<f32>(1.0, 1.0, 0.0));
    let c001 = hash(pi + vec3<f32>(0.0, 0.0, 1.0));
    let c101 = hash(pi + vec3<f32>(1.0, 0.0, 1.0));
    let c011 = hash(pi + vec3<f32>(0.0, 1.0, 1.0));
    let c111 = hash(pi + vec3<f32>(1.0, 1.0, 1.0));

    // Trilinear interpolation
    let c00 = mix(c000, c100, w.x);
    let c10 = mix(c010, c110, w.x);
    let c01 = mix(c001, c101, w.x);
    let c11 = mix(c011, c111, w.x);

    let c0 = mix(c00, c10, w.y);
    let c1 = mix(c01, c11, w.y);

    return mix(c0, c1, w.z);
}

// Fractal Brownian Motion (FBM) - sum of multiple noise octaves
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

fn perlin_noise_2d(p: vec2<f32>) -> f32 {
    let pi = floor(p);
    let pf = fract(p);

    let w = vec2<f32>(
        smoothstep(pf.x),
        smoothstep(pf.y)
    );

    // Only 4 corners (not 8)
    let c00 = hash(vec3<f32>(pi.x, pi.y, 0.0));
    let c10 = hash(vec3<f32>(pi.x + 1.0, pi.y, 0.0));
    let c01 = hash(vec3<f32>(pi.x, pi.y + 1.0, 0.0));
    let c11 = hash(vec3<f32>(pi.x + 1.0, pi.y + 1.0, 0.0));

    let c0 = mix(c00, c10, w.x);
    let c1 = mix(c01, c11, w.x);

    return mix(c0, c1, w.y);
}

fn fbm_2d(p: vec2<f32>, octaves: u32) -> f32 {
    var value = 0.0;
    var amplitude = 1.0;
    var frequency = 1.0;
    var max_value = 0.0;

    for (var i = 0u; i < octaves; i = i + 1u) {
        value = value + amplitude * perlin_noise_2d(p * frequency);
        max_value = max_value + amplitude;
        amplitude = amplitude * 0.5;
        frequency = frequency * 2.0;
    }

    return value / max_value;
}

fn surface_noise(p: vec3<f32>) -> f32 {
    // Only X and Z, using 2D noise
    return fbm_2d(vec2<f32>(p.x, p.z), 4u);
}

// CAVE NOISE - full 3D variation
fn cave_noise(p: vec3<f32>) -> f32 {
    return fbm_3d(p, params.octaves);  // All dimensions matter equally
}

// BLEND based on HEIGHT
fn final_noise(p: vec3<f32>) -> f32 {
    let surface_height = surface_noise(p);
    let caves = cave_noise(p);
    let depth = surface_height - p.y;
    let use_surface = f32(p.y > depth);

    return mix(caves, depth + 1.3, use_surface);
}

@compute @workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= FIELD_SIZE || global_id.y >= FIELD_SIZE || global_id.z >= FIELD_SIZE) {
        return;
    }

    // Convert chunk coordinate and local position to world space
    let size = f32(FIELD_SIZE - 1);
    let world_pos = vec3<f32>(
        f32(params.chunk_x) * size + f32(global_id.x),
        f32(params.chunk_y) * size + f32(global_id.y),
        f32(params.chunk_z) * size + f32(global_id.z)
    ) * params.scale;

    // Generate noise value using FBM (returns [0, 1])
    //let noise_value = fbm(world_pos * params.frequency, params.octaves);
    let noise_value = final_noise(world_pos * params.frequency);

    // Shift from [0, 1] to [-1, 1], then scale by amplitude
    let density = (noise_value * 2.0 - 1.0) * params.amplitude;

    // Store in flat buffer (x + y*SIZE + z*SIZE*SIZE)
    let index = global_id.x + global_id.y * FIELD_SIZE + global_id.z * FIELD_SIZE * FIELD_SIZE;
    noise_field[index] = density;
}

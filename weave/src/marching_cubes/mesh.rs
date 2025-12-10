use super::tables::*;
use crate::terrain::{RequestComplete, field_compute::*};
use bevy::{mesh::Indices, platform::collections::HashMap};

pub fn recieve_mesh(
    trigger: On<RequestComplete<super::NoiseParams>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Received terrain noise data");
    let _position = trigger.event().position;
    let mesh_handle = meshes.add(construct_mesh(trigger.event().data.to_owned()));

    commands.spawn((
        Name::new("Terrain Mesh"),
        Mesh3d(mesh_handle),
        MeshMaterial3d(materials.add(StandardMaterial::from_color(Color::srgb(1.0, 1.0, 1.0)))),
    ));
}

const FIELD_SIZE: u32 = 17;
const ISOLEVEL: f32 = 0.0;

pub fn construct_mesh(data: Vec<f32>) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut edge_vertices: HashMap<(u32, u32, u32, u8), u32> = HashMap::new();

    let get_density = |x: u32, y: u32, z: u32| -> f32 {
        if x >= FIELD_SIZE || y >= FIELD_SIZE || z >= FIELD_SIZE {
            0.0
        } else {
            data[(x + y * FIELD_SIZE + z * FIELD_SIZE * FIELD_SIZE) as usize]
        }
    };

    let interpolate = |p1: Vec3, v1: f32, p2: Vec3, v2: f32| -> Vec3 {
        if (ISOLEVEL - v1).abs() < 0.0001 {
            return p1;
        }
        if (ISOLEVEL - v2).abs() < 0.0001 {
            return p2;
        }
        if (v1 - v2).abs() < 0.0001 {
            return p1;
        }
        let t = (ISOLEVEL - v1) / (v2 - v1);
        p1 + (p2 - p1) * t
    };

    for x in 0..FIELD_SIZE - 1 {
        for y in 0..FIELD_SIZE - 1 {
            for z in 0..FIELD_SIZE - 1 {
                let corners = [
                    get_density(x, y, z),
                    get_density(x + 1, y, z),
                    get_density(x + 1, y + 1, z),
                    get_density(x, y + 1, z),
                    get_density(x, y, z + 1),
                    get_density(x + 1, y, z + 1),
                    get_density(x + 1, y + 1, z + 1),
                    get_density(x, y + 1, z + 1),
                ];

                let mut cube_index = 0;
                for i in 0..8 {
                    if corners[i] < ISOLEVEL {
                        cube_index |= 1 << i;
                    }
                }

                if cube_index == 0 || cube_index == 255 {
                    continue;
                }

                let edge_flag = EDGE_TABLE[cube_index as usize];
                let mut edge_list = [Vec3::ZERO; 12];

                let corners_pos = [
                    Vec3::new(x as f32, y as f32, z as f32),
                    Vec3::new((x + 1) as f32, y as f32, z as f32),
                    Vec3::new((x + 1) as f32, (y + 1) as f32, z as f32),
                    Vec3::new(x as f32, (y + 1) as f32, z as f32),
                    Vec3::new(x as f32, y as f32, (z + 1) as f32),
                    Vec3::new((x + 1) as f32, y as f32, (z + 1) as f32),
                    Vec3::new((x + 1) as f32, (y + 1) as f32, (z + 1) as f32),
                    Vec3::new(x as f32, (y + 1) as f32, (z + 1) as f32),
                ];

                for i in 0..12 {
                    if edge_flag & (1 << i) != 0 {
                        let edge = CORNER_POINT_INDICES[i];
                        let p1 = corners_pos[edge[0] as usize];
                        let p2 = corners_pos[edge[1] as usize];
                        let v1 = corners[edge[0] as usize];
                        let v2 = corners[edge[1] as usize];

                        edge_list[i] = interpolate(p1, v1, p2, v2);
                    }
                }

                for i in (0..16).step_by(3) {
                    if TRI_TABLE[cube_index as usize][i] == -1 {
                        break;
                    }

                    for j in 0..3 {
                        let edge_idx = TRI_TABLE[cube_index as usize][i + j] as usize;
                        let vertex = edge_list[edge_idx];

                        let key = (x, y, z, edge_idx as u8);
                        let idx = if let Some(&i) = edge_vertices.get(&key) {
                            i
                        } else {
                            let idx = vertices.len() as u32;
                            vertices.push([vertex.x, vertex.y, vertex.z]);
                            edge_vertices.insert(key, idx);
                            idx
                        };

                        indices.push(idx);
                    }
                }
            }
        }
    }

    Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_indices(Indices::U32(indices))
}

#![allow(unused)]
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct Observer;

#[derive(Component)]
#[require(Observer)]
pub struct AreaManaged {
    render_distance: i32,
    area_size: u32,
    lod_gradient: fn(i32, i32) -> LodLevel,
}

impl Default for AreaManaged {
    fn default() -> Self {
        Self {
            render_distance: 25,
            area_size: 32,
            lod_gradient: |distance_from_center, rd| {
                let distance = distance_from_center.abs();
                let high_upper_bound = rd / 3;
                let medium_upper_bound = rd - (rd / 5);
                let mut lod = LodLevel::Low;
                if distance < high_upper_bound {
                    lod = LodLevel::High;
                }
                if distance < medium_upper_bound {
                    lod = LodLevel::Medium;
                }
                lod
            },
        }
    }
}

#[derive(Component, Reflect)]
#[require(LodLevel::High, AreaLodMeshes)]
pub struct Area(pub IVec2);

#[derive(Component, Default, Reflect, Copy, Clone, PartialEq, Eq)]
pub enum LodLevel {
    Low,
    Medium,
    #[default]
    High,
}

// Should be Handle<Mesh>, material for debug
#[derive(Component, Default, Reflect)]
pub struct AreaLodMeshes {
    low: Option<Handle<StandardMaterial>>,
    medium: Option<Handle<StandardMaterial>>,
    high: Option<Handle<StandardMaterial>>,
}

impl AreaLodMeshes {
    pub fn new(lod: LodLevel, material: Handle<StandardMaterial>) -> Self {
        match lod {
            LodLevel::Low => Self {
                low: Some(material),
                ..default()
            },
            LodLevel::Medium => Self {
                medium: Some(material),
                ..default()
            },
            LodLevel::High => Self {
                high: Some(material),
                ..default()
            },
        }
    }

    pub fn insert_mesh(&mut self, lod: LodLevel, material: Handle<StandardMaterial>) {
        match lod {
            LodLevel::Low => self.low = Some(material),
            LodLevel::Medium => self.medium = Some(material),
            LodLevel::High => self.high = Some(material),
        }
    }

    pub fn get_mesh(&self, lod: LodLevel) -> Option<Handle<StandardMaterial>> {
        match lod {
            LodLevel::Low => self.low.clone(),
            LodLevel::Medium => self.medium.clone(),
            LodLevel::High => self.high.clone(),
        }
    }
}

/// Request a chunk of a noise parameter
#[derive(Event)]
pub struct RequestArea {
    pub position: IVec2,
    pub lod_level: LodLevel,
}

pub fn area_manager(
    mut commands: Commands,
    area_managers: Query<(&AreaManaged, &Transform), With<Observer>>,
    mut areas: Query<(Entity, &Area, &LodLevel, &mut AreaLodMeshes)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (area_managed, transform) in area_managers.iter() {
        let size = area_managed.area_size as f32;
        let global = transform.translation.xz();
        let current_area_location = IVec2::new(
            (global.x / size).floor() as i32,
            (global.y / size).floor() as i32,
        );
        for x in current_area_location.x - area_managed.render_distance
            ..=current_area_location.x + area_managed.render_distance
        {
            for y in current_area_location.y - area_managed.render_distance
                ..=current_area_location.y + area_managed.render_distance
            {
                let position = IVec2::new(x, y);
                let distance_from_center = current_area_location.distance_squared(position);
                let lod =
                    (area_managed.lod_gradient)(distance_from_center, area_managed.render_distance);
                // Everything past here is debug specifc code or something....
                let color = match lod {
                    LodLevel::Low => Color::srgba(1.0, 0.0, 0.0, 0.5),
                    LodLevel::Medium => Color::srgba(0.5, 0.5, 0.0, 0.5),
                    LodLevel::High => Color::srgba(0.0, 1.0, 0.0, 0.5),
                };

                'a: for (entity, area, lod_level, mut meshes) in areas.iter_mut() {
                    if position == area.0 {
                        if *lod_level != lod {
                            if let Some(mesh) = meshes.get_mesh(lod) {
                                commands.entity(entity).insert(MeshMaterial3d(mesh.clone()));
                            } else {
                                let handle = materials.add(StandardMaterial::from_color(color));
                                commands
                                    .entity(entity)
                                    .insert(MeshMaterial3d(handle.clone()));
                                meshes.insert_mesh(lod, handle.clone());
                            }
                            commands.entity(entity).insert(lod);
                        }

                        break 'a; // prob doesn't need the 'a def here, just incase
                    }
                }
                // spawn it
                let handle = materials.add(StandardMaterial::from_color(color));
                let global = Vec3::new((position.x as f32) * size, 0.0, (position.y as f32) * size);
                commands.spawn((
                    Area(position),
                    Transform::from_translation(global),
                    lod,
                    AreaLodMeshes::new(lod, handle.clone()),
                    Mesh3d(meshes.add(Sphere::new(1.0))),
                    MeshMaterial3d(handle),
                ));
            }
        }
    }
}

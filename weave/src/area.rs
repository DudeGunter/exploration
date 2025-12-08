use bevy::prelude::*;

#[derive(Component)]
pub struct Observer;

#[derive(Component)]
pub struct AreaManaged {
    render_distance: i32,
    lod_gradient: fn(i32, i32) -> LodLevel,
}

impl Default for AreaManaged {
    fn default() -> Self {
        Self {
            render_distance: 25,
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

pub enum LodLevel {
    Low,
    Medium,
    High,
}

/// Request a chunk of a noise parameter
#[derive(Event)]
pub struct RequestChunk<T: crate::terrain::TerrainNoiseParams> {
    pub position: IVec2,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: crate::terrain::TerrainNoiseParams> RequestChunk<T> {
    pub fn new(position: IVec2) -> Self {
        Self {
            position,
            _phantom: std::marker::PhantomData,
        }
    }
}

// possibly generic???
pub fn area_manager(area: Query<&AreaManaged, With<Observer>>) {}

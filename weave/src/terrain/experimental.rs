pub use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        gpu_readback::{Readback, ReadbackComplete},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        storage::ShaderStorageBuffer,
    },
};

pub const FIELD_SIZE: u32 = 17;

pub struct GpuFieldComputePlugin;

impl Plugin for GpuFieldComputePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "noise_field.wgsl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            error!("RenderApp not found");
            return;
        };
        render_app.add_systems(
            RenderStartup,
            (setup_noise_field, setup_noise_field_readback),
        );
    }
}

pub fn init_compute_pipeline() {}

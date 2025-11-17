use bevy::prelude::*;
use lightyear::prelude::{client::ClientPlugins, server::ServerPlugins, *};
use std::time::Duration;

pub mod client;
pub mod host;
pub mod shared;

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        let tick_duration = Duration::from_secs_f64(1.0 / shared::FIXED_TIMESTEP_HZ);
        #[cfg(feature = "steam")]
        app.add_steam_resource(shared::STEAM_APP_ID);
        app.add_plugins((
            ClientPlugins { tick_duration },
            ServerPlugins { tick_duration },
        ));
    }
}

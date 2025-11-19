//! This module introduces a settings struct that can be used to configure the server and client.
#![allow(unused_imports)]
#![allow(unused_variables)]
use core::net::{Ipv4Addr, SocketAddr};

use crate::shared::SharedSettings;

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use lightyear::{
    netcode::{NetcodeClient, client_plugin::NetcodeConfig},
    prelude::{client::*, *},
};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Event)]
pub struct ConnectClient {
    pub client_id: u64,
    /// The client port to listen on
    pub client_port: u16,
    /// The socket address of the server
    pub server_addr: SocketAddr,
    /// Which transport to use
    pub transport: ClientTransports,
    pub shared: SharedSettings,
}

impl Default for ConnectClient {
    fn default() -> Self {
        use crate::shared::*;
        let client_id = rand::random::<u64>();
        Self {
            client_id: client_id,
            client_port: CLIENT_PORT,
            server_addr: SERVER_ADDR,
            transport: ClientTransports::WebTransport,
            shared: SHARED_SETTINGS,
        }
    }
}

pub(crate) fn handle_connecting_client(trigger: On<ConnectClient>, mut cmds: Commands) {
    let settings = trigger.event();
    let client_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), settings.client_port);
    let client = cmds
        .spawn((
            Client::default(),
            Link::new(None),
            LocalAddr(client_addr),
            PeerAddr(settings.server_addr),
            ReplicationReceiver::default(),
            PredictionManager::default(),
            Name::from(format!("Client {:?}", settings.client_id)),
        ))
        .id();
    let mut add_netcode = || {
        let auth = Authentication::Manual {
            server_addr: settings.server_addr,
            client_id: settings.client_id,
            private_key: settings.shared.private_key,
            protocol_id: settings.shared.protocol_id,
        };
        let netcode_config = NetcodeConfig {
            // Make sure that the server times out clients when their connection is closed
            client_timeout_secs: 3,
            token_expire_secs: -1,
            ..default()
        };
        cmds.entity(client)
            .insert(NetcodeClient::new(auth, netcode_config).unwrap());
    };
    match settings.transport {
        #[cfg(not(target_family = "wasm"))]
        ClientTransports::Udp => {
            add_netcode();
            cmds.entity(client).insert(UdpIo::default());
        }
        ClientTransports::WebTransport => {
            add_netcode();
            let certificate_digest = {
                #[cfg(target_family = "wasm")]
                {
                    include_str!("../../../certificates/digest.txt").to_string()
                }
                #[cfg(not(target_family = "wasm"))]
                {
                    "".to_string()
                }
            };
            cmds.entity(client)
                .insert(WebTransportClientIo { certificate_digest });
        }
        ClientTransports::WebSocket => {
            add_netcode();
            let config = {
                #[cfg(target_family = "wasm")]
                {
                    ClientConfig::default()
                }
                #[cfg(not(target_family = "wasm"))]
                {
                    ClientConfig::builder().with_no_cert_validation()
                }
            };
            cmds.entity(client).insert(WebSocketClientIo { config });
        }
        #[cfg(feature = "steam")]
        ClientTransports::Steam => {
            entity_mut.insert(SteamClientIo {
                target: ConnectTarget::Addr(settings.server_addr),
                config: Default::default(),
            });
        }
    };
    cmds.trigger(Connect { entity: client });
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ClientTransports {
    #[cfg(not(target_family = "wasm"))]
    Udp,
    WebTransport,
    WebSocket,
    #[cfg(feature = "steam")]
    Steam,
}

#![allow(unused_imports)] // stop the feature gate warnings
#![allow(unused_variables)] // ^
#![allow(unreachable_code)] // stop the TODO warning

use core::net::{Ipv4Addr, SocketAddr};

use bevy::asset::ron;
use bevy::prelude::*;
use core::time::Duration;

use crate::shared::SharedSettings;
#[cfg(not(target_family = "wasm"))]
use async_compat::Compat;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
#[cfg(not(target_family = "wasm"))]
use bevy::tasks::IoTaskPool;
use lightyear::netcode::{NetcodeServer, PRIVATE_KEY_BYTES};
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Event)]
pub struct Host {
    pub transport: ServerTransports,
    pub shared: SharedSettings,
}

impl Default for Host {
    fn default() -> Self {
        use crate::shared::*;
        Self {
            transport: ServerTransports::WebTransport {
                local_port: SERVER_PORT,
                certificate: WebTransportCertificateSettings::default(),
            },
            shared: SHARED_SETTINGS,
        }
    }
}

/// This spawns both the host server and the host client
pub(crate) fn handle_spawning_host(trigger: On<Host>, mut cmds: Commands) {
    let server = cmds.spawn(Name::new("Host Server")).id();
    let settings = trigger.event();

    let mut add_netcode = || {
        let private_key = if let Some(key) = parse_private_key_from_env() {
            info!("Using private key from LIGHTYEAR_PRIVATE_KEY env var");
            key
        } else {
            settings.shared.private_key
        };
        cmds.entity(server)
            .insert(NetcodeServer::new(NetcodeConfig {
                protocol_id: settings.shared.protocol_id,
                private_key,
                ..default()
            }));
    };

    match settings.transport.clone() {
        #[cfg(feature = "udp")]
        ServerTransports::Udp { local_port } => {
            add_netcode();
            let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), local_port);
            cmds.entity(server)
                .insert((LocalAddr(server_addr), ServerUdpIo::default()));
        }
        ServerTransports::WebTransport {
            local_port,
            certificate,
        } => {
            add_netcode();
            let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), local_port);
            cmds.entity(server).insert((
                LocalAddr(server_addr),
                WebTransportServerIo {
                    certificate: (&certificate).into(),
                },
            ));
        }
        ServerTransports::WebSocket { local_port } => {
            add_netcode();
            let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), local_port);
            let sans = vec![
                "localhost".to_string(),
                "127.0.0.1".to_string(),
                "::1".to_string(),
            ];
            let config = ServerConfig::builder()
                .with_bind_address(server_addr)
                .with_identity(lightyear::websocket::server::Identity::self_signed(sans).unwrap());
            cmds.entity(server)
                .insert((LocalAddr(server_addr), WebSocketServerIo { config }));
        }
        #[cfg(feature = "steam")]
        ServerTransports::Steam { local_port } => {
            let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), local_port);
            entity_mut.insert(SteamServerIo {
                target: ListenTarget::Addr(server_addr),
                config: SessionConfig::default(),
            });
        }
    }

    cmds.spawn((
        crate::client::LocalClient,
        Client::default(),
        LinkOf { server },
        Name::new("Host Client"),
    ));
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ServerTransports {
    #[cfg(feature = "udp")]
    Udp {
        local_port: u16,
    },
    WebTransport {
        local_port: u16,
        certificate: WebTransportCertificateSettings,
    },
    WebSocket {
        local_port: u16,
    },
    #[cfg(feature = "steam")]
    Steam {
        local_port: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WebTransportCertificateSettings {
    /// Generate a self-signed certificate, with given SANs list to add to the certifictate
    /// eg: ["example.com", "*.gameserver.example.org", "10.1.2.3", "::1"]
    AutoSelfSigned(Vec<String>),
    /// Load certificate pem files from disk
    FromFile {
        /// Path to cert .pem file
        cert: String,
        /// Path to private key .pem file
        key: String,
    },
}

impl Default for WebTransportCertificateSettings {
    fn default() -> Self {
        let sans = vec![
            "localhost".to_string(),
            "127.0.0.1".to_string(),
            "::1".to_string(),
        ];
        WebTransportCertificateSettings::AutoSelfSigned(sans)
    }
}

impl From<&WebTransportCertificateSettings> for Identity {
    fn from(wt: &WebTransportCertificateSettings) -> Identity {
        match wt {
            WebTransportCertificateSettings::AutoSelfSigned(sans) => {
                // In addition to and Subject Alternate Names (SAN) added via the config,
                // we add the public ip and domain for edgegap, if detected, and also
                // any extra values specified via the SELF_SIGNED_SANS environment variable.
                let mut sans = sans.clone();
                // Are we running on edgegap?
                if let Ok(public_ip) = std::env::var("ARBITRIUM_PUBLIC_IP") {
                    println!("🔐 SAN += ARBITRIUM_PUBLIC_IP: {public_ip}");
                    sans.push(public_ip);
                    sans.push("*.pr.edgegap.net".to_string());
                }
                // generic env to add domains and ips to SAN list:
                // SELF_SIGNED_SANS="example.org,example.com,127.1.1.1"
                if let Ok(san) = std::env::var("SELF_SIGNED_SANS") {
                    println!("🔐 SAN += SELF_SIGNED_SANS: {san}");
                    sans.extend(san.split(',').map(|s| s.to_string()));
                }
                //println!("🔐 Generating self-signed certificate with SANs: {sans:?}");
                let identity = Identity::self_signed(sans).unwrap();
                let digest = identity.certificate_chain().as_slice()[0].hash();
                //println!("🔐 Certificate digest: {digest}");
                identity
            }
            WebTransportCertificateSettings::FromFile {
                cert: cert_pem_path,
                key: private_key_pem_path,
            } => {
                todo!(
                    "This don't work in any of my tests... stolen from lightyear common cli example"
                );

                println!(
                    "Reading certificate PEM files:\n * cert: {cert_pem_path}\n * key: {private_key_pem_path}",
                );
                // this is async because we need to load the certificate from io
                // we need async_compat because wtransport expects a tokio reactor
                let identity = IoTaskPool::get()
                    .scope(|s| {
                        s.spawn(Compat::new(async {
                            Identity::load_pemfiles(cert_pem_path, private_key_pem_path)
                                .await
                                .unwrap()
                        }));
                    })
                    .pop()
                    .unwrap();
                let digest = identity.certificate_chain().as_slice()[0].hash();
                println!("🔐 Certificate digest: {digest}");
                identity
            }
        }
    }
}

/// Reads and parses the LIGHTYEAR_PRIVATE_KEY environment variable into a private key.
pub fn parse_private_key_from_env() -> Option<[u8; PRIVATE_KEY_BYTES]> {
    let Ok(key_str) = std::env::var("LIGHTYEAR_PRIVATE_KEY") else {
        return None;
    };
    let private_key: Vec<u8> = key_str
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect::<String>()
        .split(',')
        .map(|s| {
            s.parse::<u8>()
                .expect("Failed to parse number in private key")
        })
        .collect();

    if private_key.len() != PRIVATE_KEY_BYTES {
        panic!("Private key must contain exactly {PRIVATE_KEY_BYTES} numbers",);
    }

    let mut bytes = [0u8; PRIVATE_KEY_BYTES];
    bytes.copy_from_slice(&private_key);
    Some(bytes)
}

//! mDNS/DNS-SD discovery backend (opt-in).
//!
//! Service type `_apeireth._udp.local`. The advertisement is deliberately
//! minimal — service/version, listen port, NodeId hint, key fingerprint,
//! display name — and never contains capability lists, model lists, tool
//! lists, credentials, session state, or anything user-identifying beyond the
//! configured display name (which defaults to a generic "Apeireth node", never
//! an OS username or hostname).
//!
//! As with every discovery record: what this broadcasts are *hints*. Trust is
//! produced only by the pairing ceremony.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::NodeId;
use apeireth_mesh_protocol::KeyFingerprint;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::discovery::{DiscoveredPeer, DiscoveryBackend, DiscoveryMethod, DiscoverySink};

/// The DNS-SD service type this mesh advertises under.
pub const MESH_SERVICE_TYPE: &str = "_apeireth._udp.local.";

/// What a node broadcasts about itself. Hints only.
#[derive(Debug, Clone)]
pub struct AdvertisedPresence {
    /// The node's identity (pseudonymous technical identifier).
    pub node_id: NodeId,
    /// The mesh listen endpoint (`host:port`).
    pub endpoint: String,
    /// The identity key fingerprint — a hint the ceremony later proves.
    pub fingerprint: KeyFingerprint,
    /// Display name. Generic by default; never an OS username or hostname.
    pub display_name: String,
}

impl AdvertisedPresence {
    /// Presence with a privacy-safe default display name.
    pub fn generic(node_id: NodeId, endpoint: String, fingerprint: KeyFingerprint) -> Self {
        Self {
            node_id,
            endpoint,
            fingerprint,
            display_name: "Apeireth node".to_string(),
        }
    }
}

/// mDNS-backed discovery: advertises presence and reports observations to a
/// sink. Owned by [`crate::MeshDiscoveryService`] (or used standalone in
/// tests); `stop`/drop releases the daemon and its sockets.
pub struct MdnsDiscovery {
    presence: Option<AdvertisedPresence>,
    sink: Mutex<Option<DiscoverySink>>,
    daemon: Mutex<Option<ServiceDaemon>>,
    service_name: String,
}

impl MdnsDiscovery {
    /// A discovering-only backend (no advertisement).
    pub fn browser(service_name: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            presence: None,
            sink: Mutex::new(None),
            daemon: Mutex::new(None),
            service_name: service_name.into(),
        })
    }

    /// An advertising + discovering backend. Advertisement is opt-in: only
    /// call this when the user asked for LAN visibility.
    pub fn advertiser(service_name: impl Into<String>, presence: AdvertisedPresence) -> Arc<Self> {
        Arc::new(Self {
            presence: Some(presence),
            sink: Mutex::new(None),
            daemon: Mutex::new(None),
            service_name: service_name.into(),
        })
    }

    /// Install the observation sink.
    pub fn set_sink(&self, sink: DiscoverySink) {
        *self.sink.lock().unwrap_or_else(|p| p.into_inner()) = Some(sink);
    }

    fn publish(&self, peer: DiscoveredPeer) {
        if let Some(sink) = self.sink.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            sink(peer);
        }
    }
}

#[async_trait::async_trait]
impl DiscoveryBackend for MdnsDiscovery {
    async fn start(&self) -> Result<(), String> {
        let daemon = ServiceDaemon::new().map_err(|error| error.to_string())?;
        if let Some(presence) = &self.presence {
            let props = [
                ("ver", "1"),
                ("node", presence.node_id.as_str()),
                ("fp", presence.fingerprint.as_str()),
                ("name", presence.display_name.as_str()),
            ];
            let (host, port) = split_endpoint(&presence.endpoint)?;
            // mdns-sd wants a hostname ending in `.local.`; derive one from
            // the service instance name rather than leaking system hostnames.
            let hostname = format!("{}.local.", self.service_name);
            let ip: std::net::IpAddr = host
                .parse()
                .map_err(|_| format!("advertised endpoint needs a literal IP: {host:?}"))?;
            let info = ServiceInfo::new(
                MESH_SERVICE_TYPE,
                &self.service_name,
                &hostname,
                ip,
                port,
                &props[..],
            )
            .map_err(|error| error.to_string())?
            .enable_addr_auto();
            daemon.register(info).map_err(|error| error.to_string())?;
        }

        let mut receiver = daemon
            .browse(MESH_SERVICE_TYPE)
            .map_err(|error| error.to_string())?;
        let sink = self.sink.lock().unwrap_or_else(|p| p.into_inner()).clone();
        let service_name = self.service_name.clone();
        tokio::spawn(async move {
            while let Ok(event) = receiver.recv_async().await {
                if let ServiceEvent::ServiceResolved(info) = event {
                    if info.get_hostname().starts_with(&service_name) {
                        continue; // never list ourselves
                    }
                    let props = info.get_properties();
                    let endpoint = format!(
                        "{}:{}",
                        info.get_addresses_v4()
                            .iter()
                            .next()
                            .map(|addr| addr.to_string())
                            .unwrap_or_default(),
                        info.get_port()
                    );
                    let peer = DiscoveredPeer {
                        node_hint: props
                            .get_property_val_str("node")
                            .and_then(|node| NodeId::new(node).ok()),
                        endpoint,
                        advertised_fingerprint: props
                            .get_property_val_str("fp")
                            .and_then(KeyFingerprint::of_hex),
                        display_name: props.get_property_val_str("name").map(str::to_string),
                        discovered_via: DiscoveryMethod::Mdns,
                        last_seen_ms: 0,
                    };
                    if let Some(sink) = &sink {
                        sink(peer);
                    }
                }
            }
        });

        *self.daemon.lock().unwrap_or_else(|p| p.into_inner()) = Some(daemon);
        Ok(())
    }

    async fn stop(&self) -> Result<(), String> {
        if let Some(daemon) = self.daemon.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = daemon.shutdown();
        }
        Ok(())
    }
}

fn split_endpoint(endpoint: &str) -> Result<(&str, u16), String> {
    let addr: SocketAddr = endpoint
        .parse()
        .map_err(|_| format!("invalid endpoint {endpoint:?}"))?;
    // ServiceInfo wants host + port separately; keep the literal host text.
    let host = match endpoint.rsplit_once(':') {
        Some((host, _)) => host.trim_start_matches('[').trim_end_matches(']'),
        None => return Err(format!("invalid endpoint {endpoint:?}")),
    };
    Ok((host, addr.port()))
}

//! Minimal deployable entry point for the rendezvous + relay server.
//!
//! Self-host first: the listen address and audience come from the environment,
//! never a hard-coded SaaS endpoint. This is a deployment component — it runs
//! the connectivity infrastructure and nothing else.

use std::net::SocketAddr;

use apeireth_mesh_rendezvous::{RendezvousRelayServer, ServerConfig};

#[tokio::main]
async fn main() {
    let listen: SocketAddr = std::env::var("APEIRETH_RENDEZVOUS_LISTEN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| "127.0.0.1:7840".parse().expect("default listen"));
    let audience = std::env::var("APEIRETH_RENDEZVOUS_AUDIENCE")
        .unwrap_or_else(|_| "relay://localhost".to_string());

    let config = ServerConfig::new(listen, audience);
    match RendezvousRelayServer::bind(config).await {
        Ok(server) => {
            eprintln!(
                "apeireth-mesh-rendezvous listening on {}",
                server.local_addr()
            );
            let server = std::sync::Arc::new(server);
            if let Err(error) = server.run().await {
                eprintln!("rendezvous server stopped: {error}");
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("failed to bind rendezvous server: {error}");
            std::process::exit(1);
        }
    }
}

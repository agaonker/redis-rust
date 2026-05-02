use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{info, warn};

use super::connection::handle_connection;

pub async fn run(addr: SocketAddr) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {}", listener.local_addr()?);

    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                info!("New connection from {}", peer_addr);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket).await {
                        warn!("Connection {} closed with error: {}", peer_addr, e);
                    }
                });
            }
            Err(e) => {
                warn!("Accept error: {}", e);
            }
        }
    }
}

use std::sync::atomic::{AtomicU64, Ordering};

use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, warn};

use crate::command::{dispatch, Command};
use crate::protocol::{parse, serialize, ParseOutcome, RespValue};
use crate::store::SharedStore;

static NEXT_CLIENT_ID: AtomicU64 = AtomicU64::new(1);

/// Per-connection state. Owns the socket and the read buffer; carries the
/// shared store handle and a monotonic `client_id` used for structured logging.
///
/// Future fields (added in later PRs): `txn_state`, `watched_keys`, `client_name`.
pub struct Connection {
    socket: TcpStream,
    buf: BytesMut,
    store: SharedStore,
    client_id: u64,
}

impl Connection {
    pub fn new(socket: TcpStream, store: SharedStore, client_id: u64) -> Self {
        Self {
            socket,
            buf: BytesMut::with_capacity(4096),
            store,
            client_id,
        }
    }

    /// Drive the connection: read RESP frames, dispatch, write responses,
    /// until the client disconnects or a protocol error closes the stream.
    #[tracing::instrument(name = "conn", skip_all, fields(client_id = self.client_id))]
    pub async fn handle(&mut self) -> std::io::Result<()> {
        loop {
            let n = self.socket.read_buf(&mut self.buf).await?;
            if n == 0 {
                debug!("client disconnected");
                return Ok(());
            }

            loop {
                match parse(&self.buf) {
                    ParseOutcome::Complete(value, consumed) => {
                        debug!(?value, "frame parsed");
                        self.buf.advance(consumed);

                        let response = match Command::try_from(value) {
                            Ok(cmd) => {
                                debug!(cmd = %cmd.name, "dispatching");
                                dispatch(cmd, &self.store).await
                            }
                            Err(e) => RespValue::Error(format!("ERR {}", e)),
                        };
                        self.socket.write_all(&serialize(&response)).await?;
                    }
                    ParseOutcome::Incomplete => break,
                    ParseOutcome::Err(e) => {
                        warn!(error = %e, "parse error");
                        let response = serialize(&RespValue::Error(format!("ERR {}", e)));
                        self.socket.write_all(&response).await?;
                        return Ok(());
                    }
                }
            }
        }
    }
}

/// Backwards-compatible entry point. Mints a fresh `client_id` and delegates
/// to `Connection::handle`. Kept so existing call sites (listener, tests) do
/// not have to manage id assignment.
pub async fn handle_connection(socket: TcpStream, store: SharedStore) -> std::io::Result<()> {
    let client_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);
    Connection::new(socket, store, client_id).handle().await
}

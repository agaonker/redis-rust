use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, warn};

use crate::command::{dispatch, Command};
use crate::protocol::{parse, serialize, ParseOutcome, RespValue};
use crate::store::SharedStore;

pub async fn handle_connection(mut socket: TcpStream, store: SharedStore) -> std::io::Result<()> {
    let mut buf = BytesMut::with_capacity(4096);

    loop {
        // Read more bytes from the socket into the buffer
        let n = socket.read_buf(&mut buf).await?;
        if n == 0 {
            debug!("Client disconnected");
            return Ok(());
        }

        // Parse as many complete frames as the buffer holds
        loop {
            match parse(&buf) {
                ParseOutcome::Complete(value, consumed) => {
                    debug!("Parsed: {:?}", value);
                    buf.advance(consumed);

                    let response = match Command::try_from(value) {
                        Ok(cmd) => dispatch(cmd, &store),
                        Err(e) => RespValue::Error(format!("ERR {}", e)),
                    };
                    socket.write_all(&serialize(&response)).await?;
                }
                ParseOutcome::Incomplete => {
                    // Need more data — go back to reading the socket
                    break;
                }
                ParseOutcome::Err(e) => {
                    warn!("Parse error: {}", e);
                    let response = serialize(&RespValue::Error(format!("ERR {}", e)));
                    socket.write_all(&response).await?;
                    return Ok(());
                }
            }
        }
    }
}


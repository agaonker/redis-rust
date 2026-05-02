/// Phase 1 integration tests.
/// Spawns the real server on a random port, sends raw RESP over TCP, asserts responses.
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

// ── test harness ──────────────────────────────────────────────────────────────

/// Start the server on a random port, return the bound port.
async fn start_server() -> u16 {
    use redis_clone::store::shared_store;
    use tokio::net::TcpListener;

    // Bind port 0 — OS assigns a free port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        let store = shared_store();
        run_with_listener(listener, store).await.unwrap();
    });

    // Give the task a moment to be scheduled
    tokio::time::sleep(Duration::from_millis(50)).await;
    port
}

/// Same as `run` but accepts an already-bound listener (so we can grab the port).
async fn run_with_listener(
    listener: tokio::net::TcpListener,
    store: redis_clone::store::SharedStore,
) -> std::io::Result<()> {
    loop {
        let (socket, _) = listener.accept().await?;
        let store = store.clone();
        tokio::spawn(async move {
            let _ = redis_clone::server::connection::handle_connection(socket, store).await;
        });
    }
}

/// Send raw bytes to the server, read back one response line (up to \r\n).
async fn send(stream: &mut TcpStream, msg: &[u8]) -> String {
    stream.write_all(msg).await.unwrap();

    let mut buf = vec![0u8; 512];
    let n = timeout(Duration::from_secs(2), stream.read(&mut buf))
        .await
        .expect("read timed out")
        .expect("read error");

    String::from_utf8_lossy(&buf[..n]).into_owned()
}

// ── helpers to build RESP frames ─────────────────────────────────────────────

fn resp_array(args: &[&str]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", args.len()).into_bytes();
    for arg in args {
        out.extend_from_slice(format!("${}\r\n{}\r\n", arg.len(), arg).as_bytes());
    }
    out
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn ping_no_args() {
    let port = start_server().await;
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
    let resp = send(&mut conn, &resp_array(&["PING"])).await;
    assert_eq!(resp, "+PONG\r\n");
}

#[tokio::test]
async fn ping_with_message() {
    let port = start_server().await;
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
    let resp = send(&mut conn, &resp_array(&["PING", "hello"])).await;
    assert_eq!(resp, "$5\r\nhello\r\n");
}

#[tokio::test]
async fn ping_wrong_arity() {
    let port = start_server().await;
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
    let resp = send(&mut conn, &resp_array(&["PING", "a", "b"])).await;
    assert!(resp.starts_with('-'));
}

#[tokio::test]
async fn echo_one_arg() {
    let port = start_server().await;
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
    let resp = send(&mut conn, &resp_array(&["ECHO", "world"])).await;
    assert_eq!(resp, "$5\r\nworld\r\n");
}

#[tokio::test]
async fn echo_wrong_arity() {
    let port = start_server().await;
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
    let resp = send(&mut conn, &resp_array(&["ECHO"])).await;
    assert!(resp.starts_with('-'));
}

#[tokio::test]
async fn set_and_get() {
    let port = start_server().await;
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();

    let resp = send(&mut conn, &resp_array(&["SET", "mykey", "myval"])).await;
    assert_eq!(resp, "+OK\r\n");

    let resp = send(&mut conn, &resp_array(&["GET", "mykey"])).await;
    assert_eq!(resp, "$5\r\nmyval\r\n");
}

#[tokio::test]
async fn get_missing_key() {
    let port = start_server().await;
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
    let resp = send(&mut conn, &resp_array(&["GET", "nosuchkey"])).await;
    assert_eq!(resp, "$-1\r\n"); // null bulk string
}

#[tokio::test]
async fn del_existing_and_missing() {
    let port = start_server().await;
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();

    send(&mut conn, &resp_array(&["SET", "k", "v"])).await;

    let resp = send(&mut conn, &resp_array(&["DEL", "k"])).await;
    assert_eq!(resp, ":1\r\n");

    let resp = send(&mut conn, &resp_array(&["DEL", "k"])).await;
    assert_eq!(resp, ":0\r\n");
}

#[tokio::test]
async fn exists_counts_duplicates() {
    let port = start_server().await;
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();

    send(&mut conn, &resp_array(&["SET", "k", "v"])).await;

    let resp = send(&mut conn, &resp_array(&["EXISTS", "k", "k", "k"])).await;
    assert_eq!(resp, ":3\r\n");

    let resp = send(&mut conn, &resp_array(&["EXISTS", "missing"])).await;
    assert_eq!(resp, ":0\r\n");
}

#[tokio::test]
async fn unknown_command_returns_error() {
    let port = start_server().await;
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();
    let resp = send(&mut conn, &resp_array(&["FOOBAR"])).await;
    assert!(resp.starts_with('-'));
}

#[tokio::test]
async fn multiple_commands_on_same_connection() {
    let port = start_server().await;
    let mut conn = TcpStream::connect(format!("127.0.0.1:{}", port)).await.unwrap();

    // Send multiple commands on the same persistent connection
    send(&mut conn, &resp_array(&["SET", "a", "1"])).await;
    send(&mut conn, &resp_array(&["SET", "b", "2"])).await;

    let resp = send(&mut conn, &resp_array(&["GET", "a"])).await;
    assert_eq!(resp, "$1\r\n1\r\n");

    let resp = send(&mut conn, &resp_array(&["GET", "b"])).await;
    assert_eq!(resp, "$1\r\n2\r\n");
}

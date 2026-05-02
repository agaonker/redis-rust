# redis-rust

A Redis-compatible server built from scratch in Rust — async, single-node, speaks RESP.

> **Status:** Phase 1 in progress — foundation layer complete (RESP parser, TCP server, PING/ECHO working)

## Goal

Build a server that real Redis clients (`redis-cli`, `redis-py`) can connect to and get identical responses to real Redis — one baby step at a time.

```
$ redis-cli -p 6379 ping
PONG

$ redis-cli -p 6379 ping "hello world"
"hello world"

$ redis-cli -p 6379 echo "testing"
"testing"
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                        Client                           │
│              (redis-cli / redis-py / any)               │
└───────────────────────┬─────────────────────────────────┘
                        │  TCP  (RESP wire format)
┌───────────────────────▼─────────────────────────────────┐
│                   Network Layer                         │
│         Tokio TcpListener · per-connection task         │
│              src/server/listener.rs                     │
│              src/server/connection.rs                   │
└───────────────────────┬─────────────────────────────────┘
                        │  raw bytes
┌───────────────────────▼─────────────────────────────────┐
│                  Protocol Layer                         │
│        RESP parser  ·  RESP serializer                  │
│   src/protocol/parser.rs · src/protocol/serializer.rs   │
└───────────────────────┬─────────────────────────────────┘
                        │  RespValue
┌───────────────────────▼─────────────────────────────────┐
│               Command Dispatch Layer                    │
│      Command::try_from(RespValue) · dispatch()          │
│   src/command/types.rs · src/command/dispatch.rs        │
└───────────────────────┬─────────────────────────────────┘
                        │  Command { name, args }
┌───────────────────────▼─────────────────────────────────┐
│                Command Handlers                         │
│              PING · ECHO · (more coming)                │
│           src/command/handlers/                         │
└───────────────────────┬─────────────────────────────────┘
                        │  (coming soon)
┌───────────────────────▼─────────────────────────────────┐
│                  Data Store Layer                       │
│     Arc<Mutex<Store>>  ·  Strings, Lists, Hashes,       │
│              Sets, Sorted Sets                          │
│                  src/store/mod.rs                       │
└─────────────────────────────────────────────────────────┘
```

---

## How connections work

Each client gets its own Tokio task — a lightweight async worker (~KB overhead vs ~MB for OS threads). The task loops, parking itself on `.await` between commands, keeping the TCP connection alive until the client disconnects.

```
TcpListener (port 6379)
      │
      ├── Client 1 connects ──► tokio::spawn ──► Task 1 (loops, awaits)
      ├── Client 2 connects ──► tokio::spawn ──► Task 2 (loops, awaits)
      └── Client 3 connects ──► tokio::spawn ──► Task 3 (loops, awaits)

Each task:
  loop {
      read bytes → BytesMut buffer
      parse RESP frame
      dispatch command
      write response
  }  ← exits when client sends FIN (n == 0)
```

---

## RESP Protocol

Redis clients and servers communicate via **RESP** (Redis Serialization Protocol). Every command is an array of bulk strings:

```
redis-cli> set foo bar
                │
                ▼  on the wire
*3\r\n          ← array of 3 elements
$3\r\nSET\r\n   ← bulk string "SET"
$3\r\nfoo\r\n   ← bulk string "foo"
$3\r\nbar\r\n   ← bulk string "bar"
```

Our parser handles all 5 RESP types:

| Type | Prefix | Example |
|---|---|---|
| Simple String | `+` | `+OK\r\n` |
| Error | `-` | `-ERR bad\r\n` |
| Integer | `:` | `:42\r\n` |
| Bulk String | `$` | `$3\r\nfoo\r\n` |
| Array | `*` | `*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n` |

The parser returns a `ParseOutcome` enum — not a `Result` — to cleanly separate three states:
- `Complete(value, bytes_consumed)` — full frame parsed
- `Incomplete` — need more data from the socket
- `Err` — malformed input, close connection

---

## Project Structure

```
redis-clone/
├── Cargo.toml
└── src/
    ├── main.rs                     # entry point, tokio runtime
    ├── error.rs                    # RedisError enum, Result<T> alias
    ├── protocol/
    │   ├── mod.rs
    │   ├── types.rs                # RespValue enum
    │   ├── parser.rs               # RESP parser → ParseOutcome
    │   └── serializer.rs           # RespValue → wire bytes
    ├── command/
    │   ├── mod.rs
    │   ├── types.rs                # Command { name, args }
    │   ├── dispatch.rs             # routes commands to handlers
    │   └── handlers/
    │       ├── mod.rs
    │       ├── ping.rs             # PING
    │       └── echo.rs             # ECHO
    ├── store/
    │   └── mod.rs                  # (coming) Arc<Mutex<Store>>
    └── server/
        ├── mod.rs
        ├── listener.rs             # TcpListener, accept loop
        └── connection.rs           # per-client read/parse/dispatch/write loop
```

---

## Implementation Plan

| Phase | What | Status |
|---|---|---|
| **0** | Cargo project + module skeleton | ✅ done |
| **1** | RESP parser/serializer, TCP server, PING/ECHO/SET/GET/DEL/EXISTS | 🔄 in progress |
| **2** | Lists, Hashes, Sets, Sorted Sets, String extensions | ⬜ |
| **3** | Expiry (EXPIRE/TTL), admin (KEYS/SCAN/INFO) | ⬜ |
| **4** | Transactions (MULTI/EXEC), RDB persistence | ⬜ |

Detailed step-by-step plan: [`redis-rust-design.md`](./redis-rust-design.md)

---

## Running

```bash
# start the server (port 6379)
cargo run

# connect with redis-cli
redis-cli -p 6379

# or with Python
python3 -c "
import redis
r = redis.Redis(port=6379)
print(r.ping())       # True
print(r.echo('hi'))   # b'hi'
"
```

---

## Tech Stack

| Crate | Purpose |
|---|---|
| `tokio` | Async runtime (epoll/kqueue under the hood) |
| `bytes` | `BytesMut` for efficient socket buffer management |
| `thiserror` | Typed error enum with `Display` derives |
| `tracing` | Structured logging |

---

## Design Principles

- **Baby steps** — one reviewable increment at a time, tests before moving on
- **No mutex across `.await`** — lock, do work, drop, then await
- **Binary-safe from day one** — all keys and values are `Vec<u8>`
- **Typed errors** — no `unwrap()` in production paths
- **Hermetic tests** — integration tests bind to port 0 (random), run in parallel

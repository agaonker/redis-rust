# Redis-Rust — Architecture

This document describes what's actually been built (through PR #3, "Phase 2 step 0").
The original intent lives in [`redis-rust-design.md`](./redis-rust-design.md);
deferred work is tracked in [`TODOS.md`](./TODOS.md).

## 1. Layered architecture

```mermaid
flowchart TB
    subgraph Clients
        cli[redis-cli]
        py[redis-py]
        nc[nc / raw TCP]
    end

    subgraph Network["Network layer (server/listener.rs)"]
        listener["TcpListener<br/>accept loop<br/>spawn task per connection"]
    end

    subgraph Connections["Per-connection layer (server/connection.rs)"]
        direction LR
        conn1["Connection #1<br/>{ socket, buf, store, client_id=1 }"]
        conn2["Connection #2<br/>{ socket, buf, store, client_id=2 }"]
        connN["Connection #N<br/>{ socket, buf, store, client_id=N }"]
    end

    subgraph Protocol["Protocol layer (protocol/)"]
        direction LR
        parser["parse<br/>ParseOutcome::{Complete, Incomplete, Err}"]
        serializer["serialize<br/>RespValue -&gt; Vec&lt;u8&gt;"]
        types["RespValue<br/>{Simple, Error, Int, Bulk, Array}"]
    end

    subgraph CommandLayer["Command layer (command/)"]
        direction LR
        ctype["Command::try_from(RespValue)"]
        disp["async dispatch(cmd, &amp;store)<br/>lock once per command"]
        h["handlers/*.rs<br/>fn(args, &amp;mut Store) -&gt; RespValue"]
    end

    subgraph StoreLayer["Store layer (store/)"]
        shared["SharedStore =<br/>Arc&lt;tokio::sync::Mutex&lt;Store&gt;&gt;"]
        store["Store { inner: HashMap&lt;Vec&lt;u8&gt;, StoreValue&gt; }"]
        sv["StoreValue<br/>{ Str | List | Hash | Set }"]
    end

    subgraph Planned["Planned (Phases 2.6 - 4)"]
        direction LR
        zset["ZSet (Phase 2.6)"]
        expiry["Expiry map + sweeper task (Phase 3)"]
        txn["TransactionState + WATCH (Phase 4)"]
        rdb["RDB v9 writer/reader + .bak + autosave (Phase 4)"]
        stubs["Compat stubs:<br/>COMMAND, CLIENT, HELLO"]
    end

    cli --> listener
    py --> listener
    nc --> listener

    listener -. spawn .-> conn1
    listener -. spawn .-> conn2
    listener -. spawn .-> connN

    conn1 -- bytes --> parser
    conn2 -- bytes --> parser
    connN -- bytes --> parser

    parser --> types
    types --> ctype
    ctype --> disp
    disp -- "lock().await" --> shared
    shared --- store
    store --- sv
    disp --> h
    h -- "&amp;mut Store" --> store
    h --> serializer
    serializer -- bytes --> conn1
    serializer -- bytes --> conn2
    serializer -- bytes --> connN

    sv -.future.-> zset
    store -.future.-> expiry
    conn1 -.future.-> txn
    store -.future.-> rdb
    disp -.future.-> stubs

    classDef planned fill:#fef9e7,stroke:#8a6d3b,stroke-dasharray: 4 4,color:#5a4a1c
    class Planned,zset,expiry,txn,rdb,stubs planned
```

**Reading guide**
- Solid arrows = wired today.
- Dashed/yellow nodes = planned for upcoming PRs (per the CEO + eng review on this branch).
- The Mutex is the only shared mutable state. Everything else above the store is per-connection
  or stateless. That's deliberate — it's what makes the model easy to reason about and easy to
  evolve into transactions later.

## 2. Per-command sequence (e.g. `SET key value`)

```mermaid
sequenceDiagram
    autonumber
    actor C as Client (redis-cli)
    participant L as TcpListener
    participant CO as Connection (per conn)
    participant P as parser
    participant CT as Command::try_from
    participant D as dispatch (async)
    participant M as tokio::sync::Mutex
    participant H as handle_set
    participant S as Store

    C->>L: TCP connect
    L->>CO: accept, spawn task, mint client_id
    Note over CO: Connection { socket, buf, store_handle, client_id }

    C->>CO: bytes "*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n"
    CO->>P: parse(&buf)
    P-->>CO: Complete(RespValue::Array(...), consumed=27)
    CO->>CT: Command::try_from(value)
    CT-->>CO: Command { name: "SET", args: [b"k", b"v"] }
    CO->>D: dispatch(cmd, &store).await
    D->>M: lock().await
    M-->>D: MutexGuard<Store>
    D->>H: handle_set(&args, &mut store)
    H->>S: store.set(b"k", StoreValue::Str(b"v"))
    H-->>D: RespValue::SimpleString("OK")
    D-->>CO: RespValue::SimpleString("OK")
    Note over M: guard dropped, lock released
    CO->>CO: serialize(&response)
    CO->>C: bytes "+OK\r\n"

    Note over CO: loop back to read next frame
```

**Things to notice**
- `PING` and `ECHO` short-circuit before the lock — keepalive traffic never contends.
- The lock is held for the duration of *one* command's handler call, then immediately dropped.
  Sequential commands on the same connection re-acquire each iteration.
- All handler work runs synchronously under the guard. No `.await` inside a handler today.
  When transactions land, `EXEC` will hold the guard across N handler calls (single critical section).

## 3. State + ownership (what lives where)

```mermaid
flowchart TB
    subgraph Process["Process-wide (one per server)"]
        NEXT["static NEXT_CLIENT_ID: AtomicU64"]
        STORE_ARC["Arc&lt;tokio::sync::Mutex&lt;Store&gt;&gt;<br/>(constructed in main, cloned per conn)"]
    end

    subgraph PerConn["Per-connection (one per TCP client)"]
        direction TB
        socket["socket: TcpStream"]
        buf["buf: BytesMut (4 KiB cap)"]
        cid["client_id: u64"]
        sharehandle["store: SharedStore (clone of the Arc)"]
        future_state["future:<br/>txn_state, watched_keys, client_name"]
    end

    subgraph PerCmd["Per-command (acquired then released)"]
        guard["MutexGuard&lt;Store&gt; (held for one handler call)"]
    end

    NEXT -- fetch_add(1) --> cid
    STORE_ARC -- "Arc::clone()" --> sharehandle
    sharehandle -- "lock().await" --> guard
    guard --> Process

    classDef future fill:#fef9e7,stroke:#8a6d3b,stroke-dasharray: 4 4,color:#5a4a1c
    class future_state future
```

**Three lifetimes, three concerns**
- **Process-wide:** one store, one monotonic id counter. Survives forever.
- **Per-connection:** the `Connection` struct owns the socket and read buffer, holds a clone of the
  store `Arc`, and carries the connection's identity (`client_id`, soon `client_name`).
  Drops when the client disconnects.
- **Per-command:** a `MutexGuard<Store>` lives only inside `dispatch` for one command. Never escapes
  the function.

## 4. File map

| Layer | Files |
|---|---|
| Network | `src/server/listener.rs` |
| Per-conn | `src/server/connection.rs` (`Connection` struct + `handle_connection` wrapper) |
| Protocol | `src/protocol/{types,parser,serializer}.rs` |
| Command | `src/command/{types,dispatch}.rs` + `src/command/handlers/{ping,echo,set,get,del,exists,list,hash,set_cmd}.rs` |
| Store | `src/store/mod.rs` |
| Errors | `src/error.rs` |
| Entry | `src/main.rs`, `src/lib.rs` |
| Tests | `tests/phase1.rs` (integration), inline `#[cfg(test)]` modules in each handler file |

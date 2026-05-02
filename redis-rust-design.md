# Redis Clone in Rust — Design & Implementation Plan

## Project Goal

Build a single-node Redis-compatible server in Rust that speaks RESP and supports the most-used commands across strings, lists, hashes, sets, sorted sets, transactions, persistence, and key expiry. Scaling (replication, clustering, pub/sub) is explicitly out of scope for phase one.

Success criterion: existing Redis clients (e.g., `redis-py`) connect to the server and the supported commands return identical results to real Redis.

## Architecture (Layered)

- **Network layer** — TCP listener (Tokio), per-connection task, framed RESP read/write
- **Protocol layer** — RESP parser and serializer (simple strings, errors, integers, bulk strings, arrays)
- **Command dispatch layer** — Routes parsed commands to handlers; validates arity and types
- **Data store layer** — In-memory structures behind a concurrency primitive (start with `Mutex<HashMap>`, evolve to sharded locks if needed)
  - Strings, Lists, Hashes, Sets, Sorted Sets
- **Expiry manager** — TTL tracking; lazy expiry on access plus a periodic sweeper task
- **Persistence layer** — RDB-style snapshot first (point-in-time dump), AOF (append-only log) as a stretch goal
- **Transaction manager** — `MULTI`/`EXEC`/`DISCARD` queueing per connection, executed atomically

## Implementation Phases

**Phase 1 — Foundation (week 1)**
- Tokio TCP server accepting connections
- RESP parser + serializer with unit tests
- Command dispatcher skeleton; implement `PING`, `ECHO`, `SET`, `GET`, `DEL`, `EXISTS`
- Validate end-to-end with `redis-py`

**Phase 2 — Data structures (week 2)**
- Strings: `APPEND`, `INCR`, `DECR`, `STRLEN`
- Lists: `LPUSH`, `RPUSH`, `LPOP`, `RPOP`, `LRANGE`, `LLEN`
- Hashes: `HSET`, `HGET`, `HGETALL`, `HDEL`, `HEXISTS`
- Sets: `SADD`, `SREM`, `SMEMBERS`, `SISMEMBER`, `SCARD`
- Sorted sets: `ZADD`, `ZRANGE`, `ZREM`, `ZSCORE`, `ZCARD`

**Phase 3 — Expiry + admin (week 3)**
- `EXPIRE`, `TTL`, `PERSIST`, lazy + periodic expiration
- Admin: `INFO`, `DBSIZE`, `FLUSHDB`, `KEYS`, `SCAN`, `TYPE`

**Phase 4 — Transactions + persistence (week 4)**
- `MULTI`, `EXEC`, `DISCARD`
- RDB snapshot on `SAVE` / `BGSAVE` and on shutdown; load on startup
- (Stretch) AOF append-only logging

## Command List (~40)

**Connection**
- `PING`, `ECHO`, `SELECT`, `QUIT`

**Strings**
- `SET`, `GET`, `DEL`, `EXISTS`, `APPEND`, `STRLEN`, `INCR`, `DECR`, `INCRBY`

**Lists**
- `LPUSH`, `RPUSH`, `LPOP`, `RPOP`, `LRANGE`, `LLEN`

**Hashes**
- `HSET`, `HGET`, `HGETALL`, `HDEL`, `HEXISTS`, `HKEYS`, `HVALS`

**Sets**
- `SADD`, `SREM`, `SMEMBERS`, `SISMEMBER`, `SCARD`

**Sorted sets**
- `ZADD`, `ZRANGE`, `ZREM`, `ZSCORE`, `ZCARD`

**Keys / expiry**
- `EXPIRE`, `TTL`, `PERSIST`, `TYPE`, `KEYS`, `SCAN`

**Transactions**
- `MULTI`, `EXEC`, `DISCARD`

**Admin**
- `INFO`, `DBSIZE`, `FLUSHDB`, `SAVE`, `BGSAVE`, `CONFIG GET`

## Key Design Decisions

- **Async runtime**: Tokio — mature, ergonomic, well-documented
- **Concurrency**: start with a single `Mutex` around the store for simplicity; profile, then shard if contention shows up
- **Error model**: domain-specific `Error` enum; convert to RESP errors at the protocol boundary
- **Testing**: unit tests for the RESP codec and each command handler; integration tests that drive the server with `redis-py`

## Out of Scope (Phase 1)

Replication, clustering, pub/sub, Lua scripting, streams, geo commands, ACLs, TLS. Revisit any of these in phase two if they're useful.

## Working with Claude Code

- Build one layer at a time; review before moving on
- Each phase ends with a working, testable increment
- Use `redis-py` from a separate terminal to validate behavior matches real Redis as you go

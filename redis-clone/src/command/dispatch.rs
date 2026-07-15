use crate::protocol::RespValue;
use crate::store::SharedStore;
use super::types::Command;
use super::handlers::ping::handle_ping;
use super::handlers::echo::handle_echo;
use super::handlers::set::handle_set;
use super::handlers::get::handle_get;
use super::handlers::del::handle_del;
use super::handlers::exists::handle_exists;
use super::handlers::list::{
    handle_lpush, handle_rpush, handle_lpop, handle_rpop, handle_llen,
    handle_lrange, handle_lindex,
};
use super::handlers::hash::{
    handle_hset, handle_hget, handle_hdel, handle_hgetall,
    handle_hexists, handle_hlen, handle_hkeys, handle_hvals,
};
use super::handlers::set_cmd::{
    handle_sadd, handle_srem, handle_smembers, handle_sismember, handle_scard,
};

/// Route a parsed command to its handler.
///
/// Acquires the store lock once per command and passes the locked guard
/// (as `&mut Store`) to handlers that need it. Sync handlers run under the
/// lock; PING/ECHO bypass the store entirely. This keeps every handler as
/// a pure `fn` and concentrates all locking in this one spot.
pub async fn dispatch(cmd: Command, store: &SharedStore) -> RespValue {
    // Commands that don't touch the store
    match cmd.name.as_str() {
        "PING" => return handle_ping(&cmd.args),
        "ECHO" => return handle_echo(&cmd.args),
        _ => {}
    }

    let mut store = store.lock().await;
    match cmd.name.as_str() {
        "SET"    => handle_set(&cmd.args, &mut store),
        "GET"    => handle_get(&cmd.args, &mut store),
        "DEL"    => handle_del(&cmd.args, &mut store),
        "EXISTS" => handle_exists(&cmd.args, &mut store),
        "LPUSH"  => handle_lpush(&cmd.args, &mut store),
        "RPUSH"  => handle_rpush(&cmd.args, &mut store),
        "LPOP"   => handle_lpop(&cmd.args, &mut store),
        "RPOP"   => handle_rpop(&cmd.args, &mut store),
        "LLEN"   => handle_llen(&cmd.args, &mut store),
        "LRANGE"    => handle_lrange(&cmd.args, &mut store),
        "LINDEX"    => handle_lindex(&cmd.args, &mut store),
        "HSET"      => handle_hset(&cmd.args, &mut store),
        "HGET"      => handle_hget(&cmd.args, &mut store),
        "HDEL"      => handle_hdel(&cmd.args, &mut store),
        "HGETALL"   => handle_hgetall(&cmd.args, &mut store),
        "HEXISTS"   => handle_hexists(&cmd.args, &mut store),
        "HLEN"      => handle_hlen(&cmd.args, &mut store),
        "HKEYS"     => handle_hkeys(&cmd.args, &mut store),
        "HVALS"     => handle_hvals(&cmd.args, &mut store),
        "SADD"      => handle_sadd(&cmd.args, &mut store),
        "SREM"      => handle_srem(&cmd.args, &mut store),
        "SMEMBERS"  => handle_smembers(&cmd.args, &mut store),
        "SISMEMBER" => handle_sismember(&cmd.args, &mut store),
        "SCARD"     => handle_scard(&cmd.args, &mut store),
        _ => RespValue::Error(format!("ERR unknown command '{}'", cmd.name)),
    }
}

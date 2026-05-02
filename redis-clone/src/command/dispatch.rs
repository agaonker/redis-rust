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

pub fn dispatch(cmd: Command, store: &SharedStore) -> RespValue {
    match cmd.name.as_str() {
        "PING"   => handle_ping(&cmd.args),
        "ECHO"   => handle_echo(&cmd.args),
        "SET"    => handle_set(&cmd.args, store),
        "GET"    => handle_get(&cmd.args, store),
        "DEL"    => handle_del(&cmd.args, store),
        "EXISTS" => handle_exists(&cmd.args, store),
        "LPUSH"  => handle_lpush(&cmd.args, store),
        "RPUSH"  => handle_rpush(&cmd.args, store),
        "LPOP"   => handle_lpop(&cmd.args, store),
        "RPOP"   => handle_rpop(&cmd.args, store),
        "LLEN"   => handle_llen(&cmd.args, store),
        "LRANGE"  => handle_lrange(&cmd.args, store),
        "LINDEX"  => handle_lindex(&cmd.args, store),
        "HSET"    => handle_hset(&cmd.args, store),
        "HGET"    => handle_hget(&cmd.args, store),
        "HDEL"    => handle_hdel(&cmd.args, store),
        "HGETALL" => handle_hgetall(&cmd.args, store),
        "HEXISTS" => handle_hexists(&cmd.args, store),
        "HLEN"    => handle_hlen(&cmd.args, store),
        "HKEYS"     => handle_hkeys(&cmd.args, store),
        "HVALS"     => handle_hvals(&cmd.args, store),
        "SADD"      => handle_sadd(&cmd.args, store),
        "SREM"      => handle_srem(&cmd.args, store),
        "SMEMBERS"  => handle_smembers(&cmd.args, store),
        "SISMEMBER" => handle_sismember(&cmd.args, store),
        "SCARD"     => handle_scard(&cmd.args, store),
        _ => RespValue::Error(format!("ERR unknown command '{}'", cmd.name)),
    }
}

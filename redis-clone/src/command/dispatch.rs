use crate::protocol::RespValue;
use crate::store::SharedStore;
use super::types::Command;
use super::handlers::ping::handle_ping;
use super::handlers::echo::handle_echo;
use super::handlers::set::handle_set;
use super::handlers::get::handle_get;
use super::handlers::del::handle_del;
use super::handlers::exists::handle_exists;

pub fn dispatch(cmd: Command, store: &SharedStore) -> RespValue {
    match cmd.name.as_str() {
        "PING"   => handle_ping(&cmd.args),
        "ECHO"   => handle_echo(&cmd.args),
        "SET"    => handle_set(&cmd.args, store),
        "GET"    => handle_get(&cmd.args, store),
        "DEL"    => handle_del(&cmd.args, store),
        "EXISTS" => handle_exists(&cmd.args, store),
        _ => RespValue::Error(format!("ERR unknown command '{}'", cmd.name)),
    }
}

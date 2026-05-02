use crate::protocol::RespValue;
use super::types::Command;
use super::handlers::ping::handle_ping;
use super::handlers::echo::handle_echo;

pub fn dispatch(cmd: Command) -> RespValue {
    match cmd.name.as_str() {
        "PING" => handle_ping(&cmd.args),
        "ECHO" => handle_echo(&cmd.args),
        _ => RespValue::Error(format!("ERR unknown command '{}'", cmd.name)),
    }
}

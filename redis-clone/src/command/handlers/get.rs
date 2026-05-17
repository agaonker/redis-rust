use crate::protocol::RespValue;
use crate::store::{wrong_type_error, Store, StoreValue};

pub fn handle_get(args: &[Vec<u8>], store: &mut Store) -> RespValue {
    if args.len() != 1 {
        return RespValue::Error("ERR wrong number of arguments for 'get' command".into());
    }
    match store.get(&args[0]) {
        None => RespValue::BulkString(None),
        Some(StoreValue::Str(v)) => RespValue::BulkString(Some(v.clone())),
        Some(_) => wrong_type_error(),
    }
}

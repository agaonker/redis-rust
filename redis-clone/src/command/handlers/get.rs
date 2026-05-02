use crate::protocol::RespValue;
use crate::store::{SharedStore, StoreValue};

pub fn handle_get(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 1 {
        return RespValue::Error("ERR wrong number of arguments for 'get' command".into());
    }
    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::BulkString(None),
        Some(StoreValue::Str(v)) => RespValue::BulkString(Some(v.clone())),
    }
}

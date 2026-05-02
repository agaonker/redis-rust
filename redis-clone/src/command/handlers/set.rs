use crate::protocol::RespValue;
use crate::store::{SharedStore, StoreValue};

pub fn handle_set(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 2 {
        return RespValue::Error("ERR wrong number of arguments for 'set' command".into());
    }
    let key = args[0].clone();
    let value = args[1].clone();
    store.lock().unwrap().set(key, StoreValue::Str(value));
    RespValue::SimpleString("OK".into())
}

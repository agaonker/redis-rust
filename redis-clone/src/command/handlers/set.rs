use crate::protocol::RespValue;
use crate::store::{Store, StoreValue};

pub fn handle_set(args: &[Vec<u8>], store: &mut Store) -> RespValue {
    if args.len() != 2 {
        return RespValue::Error("ERR wrong number of arguments for 'set' command".into());
    }
    let key = args[0].clone();
    let value = args[1].clone();
    store.set(key, StoreValue::Str(value));
    RespValue::SimpleString("OK".into())
}

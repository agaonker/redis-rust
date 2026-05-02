use crate::protocol::RespValue;
use crate::store::SharedStore;

pub fn handle_exists(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.is_empty() {
        return RespValue::Error("ERR wrong number of arguments for 'exists' command".into());
    }
    let count = store.lock().unwrap().exists(args);
    RespValue::Integer(count as i64)
}

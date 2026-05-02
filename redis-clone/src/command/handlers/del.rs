use crate::protocol::RespValue;
use crate::store::SharedStore;

pub fn handle_del(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.is_empty() {
        return RespValue::Error("ERR wrong number of arguments for 'del' command".into());
    }
    let count = store.lock().unwrap().del(args);
    RespValue::Integer(count as i64)
}

use crate::protocol::RespValue;
use crate::store::Store;

pub fn handle_exists(args: &[Vec<u8>], store: &mut Store) -> RespValue {
    if args.is_empty() {
        return RespValue::Error("ERR wrong number of arguments for 'exists' command".into());
    }
    let count = store.exists(args);
    RespValue::Integer(count as i64)
}

use crate::protocol::RespValue;
use crate::store::Store;

pub fn handle_del(args: &[Vec<u8>], store: &mut Store) -> RespValue {
    if args.is_empty() {
        return RespValue::Error("ERR wrong number of arguments for 'del' command".into());
    }
    let count = store.del(args);
    RespValue::Integer(count as i64)
}

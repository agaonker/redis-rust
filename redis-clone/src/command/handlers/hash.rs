use crate::protocol::RespValue;
use crate::store::{wrong_type_error, SharedStore, StoreValue};

pub fn handle_hset(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    // HSET key field value [field value ...]
    if args.len() < 3 || args.len() % 2 == 0 {
        return RespValue::Error("ERR wrong number of arguments for 'hset' command".into());
    }
    let key = args[0].clone();
    let mut s = store.lock().unwrap();
    match s.get(&key) {
        Some(StoreValue::Str(_)) | Some(StoreValue::List(_)) | Some(StoreValue::Set(_)) => {
            return wrong_type_error()
        }
        _ => {}
    }
    let hash = s.get_or_insert_hash(key);
    let mut added = 0i64;
    for pair in args[1..].chunks(2) {
        if !hash.contains_key(&pair[0]) {
            added += 1;
        }
        hash.insert(pair[0].clone(), pair[1].clone());
    }
    RespValue::Integer(added)
}

pub fn handle_hget(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 2 {
        return RespValue::Error("ERR wrong number of arguments for 'hget' command".into());
    }
    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::BulkString(None),
        Some(StoreValue::Hash(h)) => {
            RespValue::BulkString(h.get(&args[1]).cloned())
        }
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_hdel(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() < 2 {
        return RespValue::Error("ERR wrong number of arguments for 'hdel' command".into());
    }
    let mut s = store.lock().unwrap();
    match s.get_mut(&args[0]) {
        None => RespValue::Integer(0),
        Some(StoreValue::Hash(h)) => {
            let count = args[1..].iter().filter(|f| h.remove(*f).is_some()).count();
            RespValue::Integer(count as i64)
        }
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_hgetall(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 1 {
        return RespValue::Error("ERR wrong number of arguments for 'hgetall' command".into());
    }
    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::Array(Some(vec![])),
        Some(StoreValue::Hash(h)) => {
            let mut out = Vec::with_capacity(h.len() * 2);
            for (field, val) in h {
                out.push(RespValue::BulkString(Some(field.clone())));
                out.push(RespValue::BulkString(Some(val.clone())));
            }
            RespValue::Array(Some(out))
        }
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_hexists(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 2 {
        return RespValue::Error("ERR wrong number of arguments for 'hexists' command".into());
    }
    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::Integer(0),
        Some(StoreValue::Hash(h)) => RespValue::Integer(h.contains_key(&args[1]) as i64),
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_hlen(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 1 {
        return RespValue::Error("ERR wrong number of arguments for 'hlen' command".into());
    }
    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::Integer(0),
        Some(StoreValue::Hash(h)) => RespValue::Integer(h.len() as i64),
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_hkeys(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 1 {
        return RespValue::Error("ERR wrong number of arguments for 'hkeys' command".into());
    }
    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::Array(Some(vec![])),
        Some(StoreValue::Hash(h)) => {
            let keys = h.keys().map(|k| RespValue::BulkString(Some(k.clone()))).collect();
            RespValue::Array(Some(keys))
        }
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_hvals(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 1 {
        return RespValue::Error("ERR wrong number of arguments for 'hvals' command".into());
    }
    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::Array(Some(vec![])),
        Some(StoreValue::Hash(h)) => {
            let vals = h.values().map(|v| RespValue::BulkString(Some(v.clone()))).collect();
            RespValue::Array(Some(vals))
        }
        Some(_) => wrong_type_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::shared_store;

    #[test]
    fn hset_hget_round_trip() {
        let store = shared_store();
        let r = handle_hset(&[b"h".to_vec(), b"f1".to_vec(), b"v1".to_vec(), b"f2".to_vec(), b"v2".to_vec()], &store);
        assert_eq!(r, RespValue::Integer(2)); // 2 new fields

        assert_eq!(handle_hget(&[b"h".to_vec(), b"f1".to_vec()], &store),
            RespValue::BulkString(Some(b"v1".to_vec())));
        assert_eq!(handle_hget(&[b"h".to_vec(), b"missing".to_vec()], &store),
            RespValue::BulkString(None));
    }

    #[test]
    fn hset_update_does_not_increment() {
        let store = shared_store();
        handle_hset(&[b"h".to_vec(), b"f".to_vec(), b"v1".to_vec()], &store);
        let r = handle_hset(&[b"h".to_vec(), b"f".to_vec(), b"v2".to_vec()], &store);
        assert_eq!(r, RespValue::Integer(0)); // existing field updated, not added
    }

    #[test]
    fn hdel_and_hexists() {
        let store = shared_store();
        handle_hset(&[b"h".to_vec(), b"f".to_vec(), b"v".to_vec()], &store);
        assert_eq!(handle_hexists(&[b"h".to_vec(), b"f".to_vec()], &store), RespValue::Integer(1));
        handle_hdel(&[b"h".to_vec(), b"f".to_vec()], &store);
        assert_eq!(handle_hexists(&[b"h".to_vec(), b"f".to_vec()], &store), RespValue::Integer(0));
    }

    #[test]
    fn hlen() {
        let store = shared_store();
        handle_hset(&[b"h".to_vec(), b"a".to_vec(), b"1".to_vec(), b"b".to_vec(), b"2".to_vec()], &store);
        assert_eq!(handle_hlen(&[b"h".to_vec()], &store), RespValue::Integer(2));
    }

    #[test]
    fn wrongtype() {
        let store = shared_store();
        store.lock().unwrap().set(b"k".to_vec(), StoreValue::Str(b"v".to_vec()));
        assert!(matches!(handle_hset(&[b"k".to_vec(), b"f".to_vec(), b"v".to_vec()], &store),
            RespValue::Error(e) if e.starts_with("WRONGTYPE")));
    }
}

use crate::protocol::RespValue;
use crate::store::{wrong_type_error, SharedStore, StoreValue};

pub fn handle_lpush(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() < 2 {
        return RespValue::Error("ERR wrong number of arguments for 'lpush' command".into());
    }
    let key = &args[0];
    let mut s = store.lock().unwrap();

    match s.get(key) {
        Some(StoreValue::Str(_)) => return wrong_type_error(),
        _ => {}
    }

    let list = s.get_or_insert_list(key.clone());
    // Redis pushes multiple elements left-to-right, so last arg ends up at head
    for val in &args[1..] {
        list.push_front(val.clone());
    }
    RespValue::Integer(list.len() as i64)
}

pub fn handle_rpush(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() < 2 {
        return RespValue::Error("ERR wrong number of arguments for 'rpush' command".into());
    }
    let key = &args[0];
    let mut s = store.lock().unwrap();

    match s.get(key) {
        Some(StoreValue::Str(_)) => return wrong_type_error(),
        _ => {}
    }

    let list = s.get_or_insert_list(key.clone());
    for val in &args[1..] {
        list.push_back(val.clone());
    }
    RespValue::Integer(list.len() as i64)
}

pub fn handle_lpop(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 1 {
        return RespValue::Error("ERR wrong number of arguments for 'lpop' command".into());
    }
    let mut s = store.lock().unwrap();
    match s.get_mut(&args[0]) {
        None => RespValue::BulkString(None),
        Some(StoreValue::List(list)) => {
            let val = list.pop_front().map(Some).unwrap_or(None);
            RespValue::BulkString(val)
        }
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_rpop(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 1 {
        return RespValue::Error("ERR wrong number of arguments for 'rpop' command".into());
    }
    let mut s = store.lock().unwrap();
    match s.get_mut(&args[0]) {
        None => RespValue::BulkString(None),
        Some(StoreValue::List(list)) => {
            let val = list.pop_back().map(Some).unwrap_or(None);
            RespValue::BulkString(val)
        }
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_llen(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 1 {
        return RespValue::Error("ERR wrong number of arguments for 'llen' command".into());
    }
    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::Integer(0),
        Some(StoreValue::List(list)) => RespValue::Integer(list.len() as i64),
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_lrange(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 3 {
        return RespValue::Error("ERR wrong number of arguments for 'lrange' command".into());
    }
    let (start_str, stop_str) = (
        std::str::from_utf8(&args[1]).unwrap_or(""),
        std::str::from_utf8(&args[2]).unwrap_or(""),
    );
    let (start, stop) = match (start_str.parse::<i64>(), stop_str.parse::<i64>()) {
        (Ok(s), Ok(e)) => (s, e),
        _ => return RespValue::Error("ERR value is not an integer or out of range".into()),
    };

    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::Array(Some(vec![])),
        Some(StoreValue::List(list)) => {
            let len = list.len() as i64;
            let start = if start < 0 { (len + start).max(0) } else { start.min(len) } as usize;
            let stop  = if stop  < 0 { (len + stop).max(-1) } else { stop.min(len - 1) } as usize;
            if start > stop || list.is_empty() {
                return RespValue::Array(Some(vec![]));
            }
            let elements = list
                .iter()
                .skip(start)
                .take(stop - start + 1)
                .map(|v| RespValue::BulkString(Some(v.clone())))
                .collect();
            RespValue::Array(Some(elements))
        }
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_lindex(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 2 {
        return RespValue::Error("ERR wrong number of arguments for 'lindex' command".into());
    }
    let idx: i64 = match std::str::from_utf8(&args[1]).ok().and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => return RespValue::Error("ERR value is not an integer or out of range".into()),
    };

    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::BulkString(None),
        Some(StoreValue::List(list)) => {
            let len = list.len() as i64;
            let idx = if idx < 0 { len + idx } else { idx };
            if idx < 0 || idx >= len {
                RespValue::BulkString(None)
            } else {
                RespValue::BulkString(Some(list[idx as usize].clone()))
            }
        }
        Some(_) => wrong_type_error(),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::shared_store;

    #[test]
    fn rpush_and_lrange() {
        let store = shared_store();
        handle_rpush(&[b"l".to_vec(), b"a".to_vec(), b"b".to_vec(), b"c".to_vec()], &store);
        let r = handle_lrange(&[b"l".to_vec(), b"0".to_vec(), b"-1".to_vec()], &store);
        assert_eq!(r, RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"a".to_vec())),
            RespValue::BulkString(Some(b"b".to_vec())),
            RespValue::BulkString(Some(b"c".to_vec())),
        ])));
    }

    #[test]
    fn lpush_prepends() {
        let store = shared_store();
        // LPUSH l a b c  →  list is [c, b, a]
        handle_lpush(&[b"l".to_vec(), b"a".to_vec(), b"b".to_vec(), b"c".to_vec()], &store);
        let r = handle_lrange(&[b"l".to_vec(), b"0".to_vec(), b"-1".to_vec()], &store);
        assert_eq!(r, RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"c".to_vec())),
            RespValue::BulkString(Some(b"b".to_vec())),
            RespValue::BulkString(Some(b"a".to_vec())),
        ])));
    }

    #[test]
    fn lpop_rpop() {
        let store = shared_store();
        handle_rpush(&[b"l".to_vec(), b"a".to_vec(), b"b".to_vec(), b"c".to_vec()], &store);
        assert_eq!(handle_lpop(&[b"l".to_vec()], &store), RespValue::BulkString(Some(b"a".to_vec())));
        assert_eq!(handle_rpop(&[b"l".to_vec()], &store), RespValue::BulkString(Some(b"c".to_vec())));
        assert_eq!(handle_llen(&[b"l".to_vec()], &store), RespValue::Integer(1));
    }

    #[test]
    fn lindex_negative() {
        let store = shared_store();
        handle_rpush(&[b"l".to_vec(), b"a".to_vec(), b"b".to_vec(), b"c".to_vec()], &store);
        assert_eq!(handle_lindex(&[b"l".to_vec(), b"-1".to_vec()], &store),
            RespValue::BulkString(Some(b"c".to_vec())));
    }

    #[test]
    fn wrongtype_on_string_key() {
        let store = shared_store();
        store.lock().unwrap().set(b"k".to_vec(), StoreValue::Str(b"v".to_vec()));
        assert!(matches!(handle_lpush(&[b"k".to_vec(), b"x".to_vec()], &store), RespValue::Error(e) if e.starts_with("WRONGTYPE")));
    }
}

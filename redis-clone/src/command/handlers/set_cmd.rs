use crate::protocol::RespValue;
use crate::store::{wrong_type_error, SharedStore, StoreValue};

pub fn handle_sadd(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() < 2 {
        return RespValue::Error("ERR wrong number of arguments for 'sadd' command".into());
    }
    let key = args[0].clone();
    let mut s = store.lock().unwrap();
    match s.get(&key) {
        Some(StoreValue::Str(_)) | Some(StoreValue::List(_)) | Some(StoreValue::Hash(_)) => {
            return wrong_type_error()
        }
        _ => {}
    }
    let set = s.get_or_insert_set(key);
    let added = args[1..].iter().filter(|m| set.insert((*m).clone())).count();
    RespValue::Integer(added as i64)
}

pub fn handle_srem(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() < 2 {
        return RespValue::Error("ERR wrong number of arguments for 'srem' command".into());
    }
    let mut s = store.lock().unwrap();
    match s.get_mut(&args[0]) {
        None => RespValue::Integer(0),
        Some(StoreValue::Set(set)) => {
            let removed = args[1..].iter().filter(|m| set.remove(*m)).count();
            RespValue::Integer(removed as i64)
        }
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_smembers(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 1 {
        return RespValue::Error("ERR wrong number of arguments for 'smembers' command".into());
    }
    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::Array(Some(vec![])),
        Some(StoreValue::Set(set)) => {
            let members = set.iter().map(|m| RespValue::BulkString(Some(m.clone()))).collect();
            RespValue::Array(Some(members))
        }
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_sismember(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 2 {
        return RespValue::Error("ERR wrong number of arguments for 'sismember' command".into());
    }
    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::Integer(0),
        Some(StoreValue::Set(set)) => RespValue::Integer(set.contains(&args[1]) as i64),
        Some(_) => wrong_type_error(),
    }
}

pub fn handle_scard(args: &[Vec<u8>], store: &SharedStore) -> RespValue {
    if args.len() != 1 {
        return RespValue::Error("ERR wrong number of arguments for 'scard' command".into());
    }
    match store.lock().unwrap().get(&args[0]) {
        None => RespValue::Integer(0),
        Some(StoreValue::Set(set)) => RespValue::Integer(set.len() as i64),
        Some(_) => wrong_type_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::shared_store;

    #[test]
    fn sadd_and_scard() {
        let store = shared_store();
        let r = handle_sadd(&[b"s".to_vec(), b"a".to_vec(), b"b".to_vec(), b"c".to_vec()], &store);
        assert_eq!(r, RespValue::Integer(3));
        assert_eq!(handle_scard(&[b"s".to_vec()], &store), RespValue::Integer(3));
    }

    #[test]
    fn sadd_no_duplicates() {
        let store = shared_store();
        handle_sadd(&[b"s".to_vec(), b"a".to_vec()], &store);
        let r = handle_sadd(&[b"s".to_vec(), b"a".to_vec(), b"b".to_vec()], &store);
        assert_eq!(r, RespValue::Integer(1)); // only b is new
    }

    #[test]
    fn srem() {
        let store = shared_store();
        handle_sadd(&[b"s".to_vec(), b"a".to_vec(), b"b".to_vec()], &store);
        assert_eq!(handle_srem(&[b"s".to_vec(), b"a".to_vec()], &store), RespValue::Integer(1));
        assert_eq!(handle_srem(&[b"s".to_vec(), b"a".to_vec()], &store), RespValue::Integer(0));
    }

    #[test]
    fn sismember() {
        let store = shared_store();
        handle_sadd(&[b"s".to_vec(), b"a".to_vec()], &store);
        assert_eq!(handle_sismember(&[b"s".to_vec(), b"a".to_vec()], &store), RespValue::Integer(1));
        assert_eq!(handle_sismember(&[b"s".to_vec(), b"b".to_vec()], &store), RespValue::Integer(0));
    }

    #[test]
    fn wrongtype() {
        let store = shared_store();
        store.lock().unwrap().set(b"k".to_vec(), StoreValue::Str(b"v".to_vec()));
        assert!(matches!(
            handle_sadd(&[b"k".to_vec(), b"x".to_vec()], &store),
            RespValue::Error(e) if e.starts_with("WRONGTYPE")
        ));
    }
}

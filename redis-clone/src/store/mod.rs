use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::protocol::RespValue;

/// Shared, async-locked handle to the store. Tokio mutex (not std) so the
/// lock can be acquired across `.await` points without blocking the runtime
/// and so contending tasks queue fairly instead of CPU-spinning.
pub type SharedStore = Arc<Mutex<Store>>;

pub fn shared_store() -> SharedStore {
    Arc::new(Mutex::new(Store::new()))
}

/// The value stored for a key.
#[derive(Debug, Clone)]
pub enum StoreValue {
    Str(Vec<u8>),
    List(VecDeque<Vec<u8>>),
    Hash(HashMap<Vec<u8>, Vec<u8>>),
    Set(HashSet<Vec<u8>>),
}

impl StoreValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            StoreValue::Str(_)  => "string",
            StoreValue::List(_) => "list",
            StoreValue::Hash(_) => "hash",
            StoreValue::Set(_)  => "set",
        }
    }
}

/// Standard Redis WRONGTYPE error response.
pub fn wrong_type_error() -> RespValue {
    RespValue::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".into(),
    )
}

/// The in-memory store. Held behind `Arc<tokio::sync::Mutex<Store>>`.
/// Lock is acquired once per command in `dispatch`; sync handlers operate
/// on the locked guard via `&mut Store`.
#[derive(Debug, Default)]
pub struct Store {
    inner: HashMap<Vec<u8>, StoreValue>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &[u8]) -> Option<&StoreValue> {
        self.inner.get(key)
    }

    pub fn set(&mut self, key: Vec<u8>, value: StoreValue) {
        self.inner.insert(key, value);
    }

    pub fn del(&mut self, keys: &[Vec<u8>]) -> u64 {
        keys.iter().filter(|k| self.inner.remove(*k).is_some()).count() as u64
    }

    pub fn exists(&self, keys: &[Vec<u8>]) -> u64 {
        keys.iter().filter(|k| self.inner.contains_key(*k)).count() as u64
    }

    pub fn get_mut(&mut self, key: &[u8]) -> Option<&mut StoreValue> {
        self.inner.get_mut(key)
    }

    /// Gets the list at `key`, inserting an empty one if the key is absent.
    pub fn get_or_insert_list(&mut self, key: Vec<u8>) -> &mut VecDeque<Vec<u8>> {
        let entry = self.inner
            .entry(key)
            .or_insert_with(|| StoreValue::List(VecDeque::new()));
        match entry {
            StoreValue::List(l) => l,
            _ => unreachable!("caller checked type before calling get_or_insert_list"),
        }
    }

    /// Gets the hash at `key`, inserting an empty one if the key is absent.
    pub fn get_or_insert_hash(&mut self, key: Vec<u8>) -> &mut HashMap<Vec<u8>, Vec<u8>> {
        let entry = self.inner
            .entry(key)
            .or_insert_with(|| StoreValue::Hash(HashMap::new()));
        match entry {
            StoreValue::Hash(h) => h,
            _ => unreachable!("caller checked type before calling get_or_insert_hash"),
        }
    }

    /// Gets the set at `key`, inserting an empty one if the key is absent.
    pub fn get_or_insert_set(&mut self, key: Vec<u8>) -> &mut HashSet<Vec<u8>> {
        let entry = self.inner
            .entry(key)
            .or_insert_with(|| StoreValue::Set(HashSet::new()));
        match entry {
            StoreValue::Set(s) => s,
            _ => unreachable!("caller checked type before calling get_or_insert_set"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn str_val(s: &str) -> StoreValue {
        StoreValue::Str(s.as_bytes().to_vec())
    }

    #[test]
    fn set_and_get() {
        let mut store = Store::new();
        store.set(b"foo".to_vec(), str_val("bar"));
        assert!(matches!(store.get(b"foo"), Some(StoreValue::Str(_))));
        assert!(store.get(b"missing").is_none());
    }

    #[test]
    fn del_single() {
        let mut store = Store::new();
        store.set(b"k".to_vec(), str_val("v"));
        assert_eq!(store.del(&[b"k".to_vec()]), 1);
        assert_eq!(store.del(&[b"k".to_vec()]), 0); // already gone
    }

    #[test]
    fn del_multiple() {
        let mut store = Store::new();
        store.set(b"a".to_vec(), str_val("1"));
        store.set(b"b".to_vec(), str_val("2"));
        assert_eq!(store.del(&[b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]), 2);
    }

    #[test]
    fn exists_counts_duplicates() {
        let mut store = Store::new();
        store.set(b"k".to_vec(), str_val("v"));
        // same key passed three times counts three times — Redis semantics
        assert_eq!(store.exists(&[b"k".to_vec(), b"k".to_vec(), b"k".to_vec()]), 3);
        assert_eq!(store.exists(&[b"missing".to_vec()]), 0);
    }
}

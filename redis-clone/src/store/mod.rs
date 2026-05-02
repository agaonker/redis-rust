use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type SharedStore = Arc<Mutex<Store>>;

pub fn shared_store() -> SharedStore {
    Arc::new(Mutex::new(Store::new()))
}

/// The value stored for a key. More variants added in Phase 2.
#[derive(Debug, Clone)]
pub enum StoreValue {
    Str(Vec<u8>),
}

/// The in-memory store. Held behind Arc<Mutex<Store>> — never lock across .await.
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

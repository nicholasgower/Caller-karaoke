//! Small cache abstraction shared by native, web, and Android builds.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

pub trait CacheStore: Send {
    fn read(&self, key: &str) -> Option<Vec<u8>>;
    fn write(&mut self, key: &str, value: &[u8]) -> Result<(), String>;
    fn remove(&mut self, key: &str);
    fn clear(&mut self);
}

#[derive(Default)]
pub struct MemoryCache {
    values: HashMap<String, Vec<u8>>,
}

impl CacheStore for MemoryCache {
    fn read(&self, key: &str) -> Option<Vec<u8>> {
        self.values.get(key).cloned()
    }

    fn write(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
        self.values.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn remove(&mut self, key: &str) {
        self.values.remove(key);
    }

    fn clear(&mut self) {
        self.values.clear();
    }
}

pub struct FileCache {
    root: PathBuf,
}

impl FileCache {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        std::fs::create_dir_all(&root)
            .map_err(|error| format!("create cache directory {}: {error}", root.display()))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn path_for(&self, key: &str) -> PathBuf {
        let hash = Sha256::digest(key.as_bytes());
        let mut name = String::with_capacity(hash.len() * 2);
        for byte in hash {
            use std::fmt::Write;
            let _ = write!(name, "{byte:02x}");
        }
        self.root.join(format!("{name}.bin"))
    }
}

impl CacheStore for FileCache {
    fn read(&self, key: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path_for(key)).ok()
    }

    fn write(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
        std::fs::write(self.path_for(key), value)
            .map_err(|error| format!("write cache entry: {error}"))
    }

    fn remove(&mut self, key: &str) {
        let _ = std::fs::remove_file(self.path_for(key));
    }

    fn clear(&mut self) {
        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

pub struct Cache {
    store: Box<dyn CacheStore>,
}

impl Cache {
    pub fn new(store: impl CacheStore + 'static) -> Self {
        Self {
            store: Box::new(store),
        }
    }

    pub fn get(&self, key: &str, ttl_seconds: u64) -> Option<Vec<u8>> {
        let bytes = self.store.read(key)?;
        let (fetched_at, payload) = split_entry(&bytes)?;
        if ttl_seconds > 0 && now_seconds().saturating_sub(fetched_at) > ttl_seconds {
            return None;
        }
        Some(payload.to_vec())
    }

    /// Return a stale value when the network is unavailable.
    pub fn get_stale(&self, key: &str) -> Option<Vec<u8>> {
        let bytes = self.store.read(key)?;
        split_entry(&bytes).map(|(_, payload)| payload.to_vec())
    }

    pub fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
        // A text-safe header keeps the entry usable from `localStorage` as
        // well as from the filesystem.
        let mut bytes = format!("{}\n", now_seconds()).into_bytes();
        bytes.extend_from_slice(value);
        self.store.write(key, &bytes)
    }

    pub fn remove(&mut self, key: &str) {
        self.store.remove(key);
    }

    pub fn clear(&mut self) {
        self.store.clear();
    }
}

fn split_entry(bytes: &[u8]) -> Option<(u64, &[u8])> {
    let newline = bytes.iter().position(|byte| *byte == b'\n')?;
    let timestamp = std::str::from_utf8(&bytes[..newline]).ok()?.parse().ok()?;
    Some((timestamp, &bytes[newline + 1..]))
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_cache_round_trip() {
        let mut cache = Cache::new(MemoryCache::default());
        cache.put("dance/contradb/2409", b"hello").unwrap();
        assert_eq!(
            cache.get("dance/contradb/2409", 60),
            Some(b"hello".to_vec())
        );
        assert_eq!(
            cache.get_stale("dance/contradb/2409"),
            Some(b"hello".to_vec())
        );
        cache.clear();
        assert_eq!(cache.get("dance/contradb/2409", 60), None);
    }

    #[test]
    fn zero_ttl_means_no_expiry() {
        let mut cache = Cache::new(MemoryCache::default());
        cache.put("key", b"value").unwrap();
        assert_eq!(cache.get("key", 0), Some(b"value".to_vec()));
    }

    #[test]
    fn file_cache_stores_hashed_keys() {
        let root = std::env::temp_dir().join(format!(
            "contra-karaoke-cache-test-{}-{}",
            std::process::id(),
            now_seconds()
        ));
        let mut cache = Cache::new(FileCache::new(&root).unwrap());
        cache.put("../unsafe/key", b"data").unwrap();
        assert_eq!(cache.get("../unsafe/key", 0), Some(b"data".to_vec()));
        cache.clear();
        let _ = std::fs::remove_dir_all(root);
    }
}

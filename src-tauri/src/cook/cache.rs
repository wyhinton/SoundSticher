// Caching system for cook results

use crate::artifacts::Artifact;
use crate::cook::Fingerprint;
use crate::graph::OpId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Cache for storing cooking results
#[derive(Debug)]
pub struct CookCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_size: usize,
    current_size: Arc<RwLock<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    op_id: OpId,
    fingerprint: Fingerprint,
    artifact: Artifact,
    created_at: std::time::SystemTime,
    access_count: u64,
    last_accessed: std::time::SystemTime,
}

impl CookCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            current_size: Arc::new(RwLock::new(0)),
        }
    }

    pub fn get(&self, key: &str) -> Option<Artifact> {
        let mut entries = self.entries.write().unwrap();
        if let Some(entry) = entries.get_mut(key) {
            entry.access_count += 1;
            entry.last_accessed = std::time::SystemTime::now();
            Some(entry.artifact.clone())
        } else {
            None
        }
    }

    pub fn put(&self, key: String, op_id: OpId, fingerprint: Fingerprint, artifact: Artifact) {
        let entry = CacheEntry {
            op_id,
            fingerprint,
            artifact,
            created_at: std::time::SystemTime::now(),
            access_count: 1,
            last_accessed: std::time::SystemTime::now(),
        };

        let mut entries = self.entries.write().unwrap();
        entries.insert(key, entry);
    }
}

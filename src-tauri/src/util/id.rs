// SlotMap and ID utilities

pub use slotmap::{DefaultKey, DenseSlotMap, SlotMap};

/// Re-export commonly used ID types
pub type OpId = DefaultKey;
pub type TaskId = DefaultKey;
pub type ArtifactId = DefaultKey;

/// Utility functions for working with IDs
pub mod id_utils {
    use super::*;

    /// Convert an ID to a string representation
    pub fn id_to_string<K: slotmap::Key>(id: K) -> String {
        format!("{:?}", id.data())
    }

    /// Generate a human-readable ID
    pub fn friendly_id<K: slotmap::Key>(id: K, prefix: &str) -> String {
        format!("{}_{}", prefix, id.data().as_ffi())
    }

    /// Generate a globally unique ID string
    pub fn generate_unique_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let random: u32 = rand::random();
        format!("{}_{:08x}", timestamp, random)
    }

    /// Create a new SlotMap with initial capacity
    pub fn new_slotmap_with_capacity<T>(capacity: usize) -> SlotMap<DefaultKey, T> {
        SlotMap::with_capacity(capacity)
    }

    /// Create a new DenseSlotMap with initial capacity
    pub fn new_dense_slotmap_with_capacity<T>(capacity: usize) -> DenseSlotMap<DefaultKey, T> {
        DenseSlotMap::with_capacity(capacity)
    }
}

/// ID generator for creating unique identifiers
#[derive(Debug, Default)]
pub struct IdGenerator {
    counter: std::sync::atomic::AtomicU64,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Generate a new unique ID
    pub fn next_id(&self) -> u64 {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Generate a string ID with prefix
    pub fn next_string_id(&self, prefix: &str) -> String {
        format!("{}_{}", prefix, self.next_id())
    }

    /// Reset the counter (useful for testing)
    pub fn reset(&self) {
        self.counter.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_generator() {
        let generator = IdGenerator::new();

        let id1 = generator.next_id();
        let id2 = generator.next_id();

        assert_ne!(id1, id2);
        assert!(id2 > id1);
    }

    #[test]
    fn test_slotmap_operations() {
        let mut map = SlotMap::new();

        let id = map.insert("test_value");
        assert_eq!(map.get(id), Some(&"test_value"));

        map.remove(id);
        assert_eq!(map.get(id), None);
    }
}

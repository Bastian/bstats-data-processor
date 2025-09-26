use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

#[derive(Clone)]
struct CachedEntry<T: Clone> {
    data: T,
    expires_at: Instant,
}

impl<T: Clone> CachedEntry<T> {
    fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_valid(&self) -> bool {
        Instant::now() < self.expires_at
    }
}

/// A simple in-memory cache with TTL (time-to-live) for each entry.
pub struct Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    entries: Arc<RwLock<HashMap<K, CachedEntry<V>>>>,
    /// Time-to-live duration for each cache entry.
    ttl: Duration,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    pub fn with_ttl_minutes(minutes: u64) -> Self {
        Self::new(Duration::from_secs(minutes * 60))
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        let cache = self.entries.read().await;
        cache
            .get(key)
            .filter(|entry| entry.is_valid())
            .map(|entry| entry.data.clone())
    }

    pub async fn insert(&self, key: K, value: V) {
        let mut cache = self.entries.write().await;
        cache.insert(key, CachedEntry::new(value, self.ttl));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::advance;

    #[tokio::test]
    async fn can_retrieve_inserted_value() {
        let cache: Cache<String, String> = Cache::with_ttl_minutes(5);

        let key = "test_key".to_string();
        let value = "test_value".to_string();

        // Test that get returns None for non-existent key
        assert_eq!(cache.get(&key).await, None);

        // Insert a value
        cache.insert(key.clone(), value.clone()).await;

        // Verify we can retrieve it
        assert_eq!(cache.get(&key).await, Some(value));
    }

    #[tokio::test]
    async fn can_overwrite_existing_key() {
        let cache: Cache<i32, String> = Cache::with_ttl_minutes(5);

        let key = 42;
        let value1 = "first".to_string();
        let value2 = "second".to_string();

        cache.insert(key, value1.clone()).await;
        assert_eq!(cache.get(&key).await, Some(value1));

        cache.insert(key, value2.clone()).await;
        assert_eq!(cache.get(&key).await, Some(value2));
    }

    #[tokio::test]
    async fn can_handle_multiple_keys() {
        let cache: Cache<&str, i32> = Cache::with_ttl_minutes(5);

        cache.insert("one", 1).await;
        cache.insert("two", 2).await;
        cache.insert("three", 3).await;

        assert_eq!(cache.get(&"one").await, Some(1));
        assert_eq!(cache.get(&"two").await, Some(2));
        assert_eq!(cache.get(&"three").await, Some(3));
        assert_eq!(cache.get(&"four").await, None);
    }

    #[tokio::test]
    async fn does_expire_after_ttl() {
        // Use tokio's time control for deterministic testing
        tokio::time::pause();

        let cache: Cache<String, String> = Cache::new(Duration::from_secs(2));
        let key = "expiring_key".to_string();
        let value = "expiring_value".to_string();

        cache.insert(key.clone(), value.clone()).await;

        // Should be valid immediately
        assert_eq!(cache.get(&key).await, Some(value.clone()));

        // Advance time by 1 second - should still be valid
        advance(Duration::from_secs(1)).await;
        assert_eq!(cache.get(&key).await, Some(value.clone()));

        // Advance time past TTL - should be expired
        advance(Duration::from_secs(2)).await;
        assert_eq!(cache.get(&key).await, None);
    }

    #[tokio::test]
    async fn does_reset_ttl_on_update() {
        tokio::time::pause();

        let cache: Cache<&str, i32> = Cache::new(Duration::from_secs(3));
        let key = "refresh_key";

        cache.insert(key, 1).await;

        // Advance time by 2 seconds
        advance(Duration::from_secs(2)).await;
        assert_eq!(cache.get(&key).await, Some(1));

        // Update the value (should reset TTL)
        cache.insert(key, 2).await;

        // Advance time by 2 more seconds (total 4 seconds from first insert)
        advance(Duration::from_secs(2)).await;

        // Should still be valid because TTL was reset
        assert_eq!(cache.get(&key).await, Some(2));

        // Advance past the new TTL
        advance(Duration::from_secs(2)).await;
        assert_eq!(cache.get(&key).await, None);
    }

    #[tokio::test]
    async fn can_handle_concurrent_access() {
        let cache = Arc::new(Cache::<i32, String>::with_ttl_minutes(5));

        let mut handles = vec![];

        // Spawn multiple tasks that read and write concurrently
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = tokio::spawn(async move {
                // Each task writes its own key
                cache_clone.insert(i, format!("value_{}", i)).await;

                // Try to read multiple keys
                for j in 0..10 {
                    let _ = cache_clone.get(&j).await;
                }

                // Verify own key
                cache_clone.get(&i).await
            });
            handles.push(handle);
        }

        // Wait for all tasks and verify they all succeeded
        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.await.unwrap();
            assert_eq!(result, Some(format!("value_{}", i)));
        }
    }

    #[tokio::test]
    async fn can_handle_different_types() {
        // Test with different key-value types to ensure generics work correctly
        let string_cache: Cache<String, String> = Cache::with_ttl_minutes(1);
        string_cache
            .insert("key".to_string(), "value".to_string())
            .await;
        assert_eq!(
            string_cache.get(&"key".to_string()).await,
            Some("value".to_string())
        );

        let int_cache: Cache<i32, Vec<u8>> = Cache::with_ttl_minutes(1);
        int_cache.insert(42, vec![1, 2, 3]).await;
        assert_eq!(int_cache.get(&42).await, Some(vec![1, 2, 3]));

        #[derive(Clone, Hash, Eq, PartialEq, Debug)]
        struct CustomKey {
            id: u64,
            name: String,
        }

        let custom_cache: Cache<CustomKey, bool> = Cache::with_ttl_minutes(1);
        let key = CustomKey {
            id: 1,
            name: "test".to_string(),
        };
        custom_cache.insert(key.clone(), true).await;
        assert_eq!(custom_cache.get(&key).await, Some(true));
    }

    #[tokio::test]
    async fn does_not_return_expired_entries() {
        tokio::time::pause();

        let cache: Cache<&str, i32> = Cache::new(Duration::from_secs(1));

        // Insert multiple entries
        cache.insert("a", 1).await;
        advance(Duration::from_millis(500)).await;
        cache.insert("b", 2).await;

        // Advance time so "a" expires but "b" doesn't
        advance(Duration::from_millis(600)).await;

        assert_eq!(cache.get(&"a").await, None); // expired
        assert_eq!(cache.get(&"b").await, Some(2)); // still valid
    }
}

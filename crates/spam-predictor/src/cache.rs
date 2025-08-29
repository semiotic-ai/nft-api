// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Caching layer for spam prediction operations
//!
//! This module provides high-performance in-memory caching for configurations,
//! model results, and frequently accessed data to minimize API calls and
//! improve prediction latency.

use std::{
    hash::{Hash, Hasher},
    time::{Duration, Instant},
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, trace};

use crate::error::SpamPredictorResult;

/// Cache key for prediction results
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PredictionCacheKey {
    /// Contract metadata hash (simplified representation)
    pub metadata_hash: String,
    /// Model type used for prediction
    pub model_type: String,
    /// Model version used
    pub model_version: String,
    /// Prompt version used
    pub prompt_version: String,
}

impl PredictionCacheKey {
    /// Create a new cache key from prediction parameters
    pub fn new(
        metadata_hash: String,
        model_type: String,
        model_version: String,
        prompt_version: String,
    ) -> Self {
        Self {
            metadata_hash,
            model_type,
            model_version,
            prompt_version,
        }
    }

    /// Create a cache key from contract metadata
    pub fn from_metadata(
        metadata: &api_client::ContractMetadata,
        model_type: &str,
        model_version: &str,
        prompt_version: &str,
    ) -> Self {
        let metadata_hash = Self::hash_metadata(metadata);
        Self::new(
            metadata_hash,
            model_type.to_string(),
            model_version.to_string(),
            prompt_version.to_string(),
        )
    }

    /// Create a simple hash of contract metadata for caching purposes
    fn hash_metadata(metadata: &api_client::ContractMetadata) -> String {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        // Hash key metadata fields
        metadata.address.hash(&mut hasher);
        if let Some(ref name) = metadata.name {
            name.hash(&mut hasher);
        }
        if let Some(ref symbol) = metadata.symbol {
            symbol.hash(&mut hasher);
        }
        if let Some(ref contract_type) = metadata.contract_type {
            format!("{:?}", contract_type).hash(&mut hasher);
        }

        format!("{:x}", hasher.finish())
    }
}

/// Cached prediction result with timestamp
#[derive(Debug, Clone)]
pub struct CachedPrediction {
    /// The prediction result
    pub result: Option<bool>,
    /// When this prediction was cached
    pub cached_at: Instant,
    /// How many times this cache entry has been accessed
    pub access_count: u64,
}

impl CachedPrediction {
    /// Create a new cached prediction
    pub fn new(result: Option<bool>) -> Self {
        Self {
            result,
            cached_at: Instant::now(),
            access_count: 0,
        }
    }

    /// Check if this cached result is still valid
    pub fn is_valid(&self, ttl: Duration) -> bool {
        self.cached_at.elapsed() < ttl
    }

    /// Mark this cache entry as accessed
    pub fn accessed(&mut self) {
        self.access_count += 1;
    }

    /// Get the last access time (approximation based on cached_at + access_count aging)
    pub fn last_access_time(&self) -> Instant {
        // Simple heuristic: more recent accesses push the "last access" time forward
        let access_aging = std::time::Duration::from_millis(self.access_count.min(1000));
        self.cached_at + access_aging
    }
}

/// High-performance cache for spam prediction results and configurations
#[derive(Debug)]
pub struct SpamCache {
    /// Cached prediction results
    predictions: DashMap<PredictionCacheKey, CachedPrediction>,
    /// Cached model registry data
    model_registry: DashMap<String, String>,
    /// Cached prompt data
    prompts: DashMap<String, String>,
    /// Cache TTL for predictions
    prediction_ttl: Duration,
    /// Maximum number of cached predictions
    max_predictions: usize,
    /// Cache statistics
    stats: DashMap<String, u64>,
}

impl Default for SpamCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SpamCache {
    /// Create a new spam cache with default settings
    pub fn new() -> Self {
        Self {
            predictions: DashMap::new(),
            model_registry: DashMap::new(),
            prompts: DashMap::new(),
            prediction_ttl: Duration::from_secs(3600), // 1 hour default
            max_predictions: 10000,                    // 10k predictions max
            stats: DashMap::new(),
        }
    }

    /// Create a new spam cache with custom settings
    pub fn with_settings(prediction_ttl: Duration, max_predictions: usize) -> Self {
        Self {
            predictions: DashMap::new(),
            model_registry: DashMap::new(),
            prompts: DashMap::new(),
            prediction_ttl,
            max_predictions,
            stats: DashMap::new(),
        }
    }

    /// Get a cached prediction result
    pub fn get_prediction(&self, key: &PredictionCacheKey) -> Option<Option<bool>> {
        if let Some(mut cached) = self.predictions.get_mut(key) {
            if cached.is_valid(self.prediction_ttl) {
                cached.accessed();
                self.increment_stat("cache_hits");

                trace!(
                    "Cache hit for prediction key: metadata_hash={}, model={}:{}, prompt={}",
                    key.metadata_hash, key.model_type, key.model_version, key.prompt_version
                );

                return Some(cached.result);
            } else {
                // Remove expired entry
                drop(cached);
                self.predictions.remove(key);
                self.increment_stat("cache_expired");

                debug!("Expired cache entry removed for key: {:?}", key);
            }
        }

        self.increment_stat("cache_misses");
        None
    }

    /// Store a prediction result in the cache
    pub fn store_prediction(&self, key: PredictionCacheKey, result: Option<bool>) {
        // Proactive cache management: start evicting when approaching capacity
        let current_size = self.predictions.len();
        let capacity_threshold = (self.max_predictions as f64 * 0.9) as usize; // 90% threshold

        if current_size >= self.max_predictions {
            // At capacity - must evict
            self.evict_oldest_prediction();
        } else if current_size >= capacity_threshold {
            // Approaching capacity - proactively evict expired entries
            self.cleanup_expired_sync();
        }

        let cached = CachedPrediction::new(result);
        self.predictions.insert(key.clone(), cached);
        self.increment_stat("cache_stores");

        trace!(
            "Stored prediction in cache: metadata_hash={}, model={}:{}, prompt={}, result={:?} (size: {}/{})",
            key.metadata_hash,
            key.model_type,
            key.model_version,
            key.prompt_version,
            result,
            self.predictions.len(),
            self.max_predictions
        );
    }

    /// Evict the least recently used cache entry (enhanced LRU implementation)
    fn evict_oldest_prediction(&self) {
        let mut lru_key: Option<PredictionCacheKey> = None;
        let mut lru_time: Option<Instant> = None;
        let mut lru_access_count = u64::MAX;

        // Find the least recently used entry using combined time + access heuristic
        for item in self.predictions.iter() {
            let entry = item.value();
            let effective_time = entry.last_access_time();

            // Select the entry with the earliest effective time (least recently used)
            // If times are equal, prefer the one with fewer accesses
            let should_evict = match lru_time {
                None => true, // First entry
                Some(current_lru_time) => {
                    effective_time < current_lru_time
                        || (effective_time == current_lru_time
                            && entry.access_count < lru_access_count)
                }
            };

            if should_evict {
                lru_time = Some(effective_time);
                lru_access_count = entry.access_count;
                lru_key = Some(item.key().clone());
            }
        }

        // Remove the LRU entry
        if let Some(key) = lru_key {
            let evicted_entry = self.predictions.remove(&key);
            self.increment_stat("cache_evictions");

            if let Some((_, entry)) = evicted_entry {
                info!(
                    metadata_hash = key.metadata_hash,
                    model_type = key.model_type,
                    model_version = key.model_version,
                    prompt_version = key.prompt_version,
                    access_count = entry.access_count,
                    age_ms = entry.cached_at.elapsed().as_millis(),
                    remaining_predictions = self.predictions.len(),
                    "evicted lru cache entry due to capacity limit"
                );
            }
        }
    }

    /// Get a cached model ID
    pub fn get_model(&self, model_type: &str, version: &str) -> Option<String> {
        let key = format!("{}:{}", model_type, version);
        self.model_registry.get(&key).map(|v| v.clone())
    }

    /// Store a model ID in the cache
    pub fn store_model(&self, model_type: &str, version: &str, model_id: String) {
        let key = format!("{}:{}", model_type, version);
        self.model_registry.insert(key, model_id);
    }

    /// Get a cached prompt
    pub fn get_prompt(&self, version: &str) -> Option<String> {
        self.prompts.get(version).map(|v| v.clone())
    }

    /// Store a prompt in the cache
    pub fn store_prompt(&self, version: &str, prompt: String) {
        self.prompts.insert(version.to_string(), prompt);
    }

    /// Clear all cached predictions
    pub fn clear_predictions(&self) {
        self.predictions.clear();
        debug!("Cleared all cached predictions");
    }

    /// Clear all cached configurations (models and prompts)
    pub fn clear_configurations(&self) {
        self.model_registry.clear();
        self.prompts.clear();
        debug!("Cleared all cached configurations");
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.clear_predictions();
        self.clear_configurations();
        self.stats.clear();
        debug!("Cleared all caches and statistics");
    }

    /// Synchronous cleanup of expired entries (used for proactive management)
    fn cleanup_expired_sync(&self) {
        let mut keys_to_remove = Vec::new();

        // Collect expired keys
        for item in self.predictions.iter() {
            if !item.value().is_valid(self.prediction_ttl) {
                keys_to_remove.push(item.key().clone());
            }
        }

        // Remove expired entries
        let mut removed_count = 0;
        for key in keys_to_remove {
            if self.predictions.remove(&key).is_some() {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            self.increment_stat("cache_expired");
            let stats = self.get_stats();
            info!(
                removed_entries = removed_count,
                remaining_predictions = stats.prediction_count,
                utilization_rate = stats.utilization_rate,
                hit_rate = stats.hit_rate,
                "proactively cleaned up expired cache entries"
            );
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let cache_hits = self.get_stat("cache_hits");
        let cache_misses = self.get_stat("cache_misses");
        let total_requests = cache_hits + cache_misses;
        let hit_rate = if total_requests > 0 {
            cache_hits as f64 / total_requests as f64
        } else {
            0.0
        };

        // Calculate cache utilization
        let current_size = self.predictions.len();
        let utilization_rate = if self.max_predictions > 0 {
            current_size as f64 / self.max_predictions as f64
        } else {
            0.0
        };

        // Calculate average access count for performance insights
        let total_access_count: u64 = self
            .predictions
            .iter()
            .map(|item| item.value().access_count)
            .sum();
        let avg_access_count = if current_size > 0 {
            total_access_count as f64 / current_size as f64
        } else {
            0.0
        };

        CacheStats {
            prediction_count: current_size,
            model_count: self.model_registry.len(),
            prompt_count: self.prompts.len(),
            cache_hits,
            cache_misses,
            cache_stores: self.get_stat("cache_stores"),
            cache_evictions: self.get_stat("cache_evictions"),
            cache_expired: self.get_stat("cache_expired"),
            hit_rate,
            utilization_rate,
            avg_access_count,
            max_capacity: self.max_predictions,
        }
    }

    /// Increment a statistics counter
    fn increment_stat(&self, key: &str) {
        self.stats
            .entry(key.to_string())
            .and_modify(|v| *v += 1)
            .or_insert(1);
    }

    /// Get a statistics value
    fn get_stat(&self, key: &str) -> u64 {
        self.stats.get(key).map(|v| *v).unwrap_or(0)
    }

    /// Clean up expired entries (maintenance operation)
    pub fn cleanup_expired(&self) -> SpamPredictorResult<usize> {
        let mut removed_count = 0;
        let mut keys_to_remove = Vec::new();

        // Collect expired keys
        for item in self.predictions.iter() {
            if !item.value().is_valid(self.prediction_ttl) {
                keys_to_remove.push(item.key().clone());
            }
        }

        // Remove expired entries
        for key in keys_to_remove {
            if self.predictions.remove(&key).is_some() {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            self.stats.insert(
                "cache_expired".to_string(),
                self.get_stat("cache_expired") + removed_count as u64,
            );

            debug!("Cleaned up {} expired cache entries", removed_count);
        }

        Ok(removed_count)
    }
}

/// Cache statistics and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Number of cached predictions
    pub prediction_count: usize,
    /// Number of cached models
    pub model_count: usize,
    /// Number of cached prompts
    pub prompt_count: usize,
    /// Cache hit count
    pub cache_hits: u64,
    /// Cache miss count
    pub cache_misses: u64,
    /// Number of items stored in cache
    pub cache_stores: u64,
    /// Number of cache evictions
    pub cache_evictions: u64,
    /// Number of expired cache entries
    pub cache_expired: u64,
    /// Cache hit rate (0.0 to 1.0)
    pub hit_rate: f64,
    /// Cache utilization rate (0.0 to 1.0)
    pub utilization_rate: f64,
    /// Average access count per cached item
    pub avg_access_count: f64,
    /// Maximum cache capacity
    pub max_capacity: usize,
}

#[cfg(test)]
mod tests {
    use std::thread;

    use alloy_primitives::Address;
    use api_client::ContractMetadata;

    use super::*;

    fn create_test_metadata() -> ContractMetadata {
        ContractMetadata {
            address: Address::ZERO,
            name: Some("Test NFT".to_string()),
            symbol: Some("TEST".to_string()),
            total_supply: None,
            holder_count: None,
            transaction_count: None,
            creation_block: None,
            creation_timestamp: None,
            creator_address: None,
            is_verified: Some(true),
            contract_type: Some(api_client::ContractType::Erc721),
            additional_data: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn cache_key_creation() {
        let metadata = create_test_metadata();
        let key =
            PredictionCacheKey::from_metadata(&metadata, "spam_classification", "latest", "1.0.0");

        assert_eq!(key.model_type, "spam_classification");
        assert_eq!(key.model_version, "latest");
        assert_eq!(key.prompt_version, "1.0.0");
        assert!(!key.metadata_hash.is_empty());
    }

    #[test]
    fn prediction_caching() {
        let cache = SpamCache::new();
        let metadata = create_test_metadata();
        let key = PredictionCacheKey::from_metadata(&metadata, "test_model", "v1", "1.0.0");

        // Initially no cached result
        assert_eq!(cache.get_prediction(&key), None);

        // Store a result
        cache.store_prediction(key.clone(), Some(true));

        // Should retrieve the cached result
        assert_eq!(cache.get_prediction(&key), Some(Some(true)));

        // Store a different result
        cache.store_prediction(key.clone(), Some(false));
        assert_eq!(cache.get_prediction(&key), Some(Some(false)));

        // Store None result
        cache.store_prediction(key.clone(), None);
        assert_eq!(cache.get_prediction(&key), Some(None));
    }

    #[test]
    fn cache_expiration() {
        let cache = SpamCache::with_settings(Duration::from_millis(10), 1000);
        let metadata = create_test_metadata();
        let key = PredictionCacheKey::from_metadata(&metadata, "test_model", "v1", "1.0.0");

        // Store a result
        cache.store_prediction(key.clone(), Some(true));
        assert_eq!(cache.get_prediction(&key), Some(Some(true)));

        // Wait for expiration
        thread::sleep(Duration::from_millis(15));

        // Should not return expired result
        assert_eq!(cache.get_prediction(&key), None);
    }

    #[test]
    fn model_caching() {
        let cache = SpamCache::new();

        // Initially no cached model
        assert_eq!(cache.get_model("spam_classification", "latest"), None);

        // Store a model
        cache.store_model(
            "spam_classification",
            "latest",
            "gpt-4-model-id".to_string(),
        );

        // Should retrieve the cached model
        assert_eq!(
            cache.get_model("spam_classification", "latest"),
            Some("gpt-4-model-id".to_string())
        );
    }

    #[test]
    fn prompt_caching() {
        let cache = SpamCache::new();

        // Initially no cached prompt
        assert_eq!(cache.get_prompt("1.0.0"), None);

        // Store a prompt
        cache.store_prompt("1.0.0", "Test prompt".to_string());

        // Should retrieve the cached prompt
        assert_eq!(cache.get_prompt("1.0.0"), Some("Test prompt".to_string()));
    }

    #[test]
    fn cache_statistics() {
        let cache = SpamCache::new();
        let metadata = create_test_metadata();
        let key = PredictionCacheKey::from_metadata(&metadata, "test_model", "v1", "1.0.0");

        let initial_stats = cache.get_stats();
        assert_eq!(initial_stats.cache_hits, 0);
        assert_eq!(initial_stats.cache_misses, 0);
        assert_eq!(initial_stats.utilization_rate, 0.0);
        assert_eq!(initial_stats.max_capacity, 10000);

        // Miss
        cache.get_prediction(&key);
        let stats = cache.get_stats();
        assert_eq!(stats.cache_misses, 1);

        // Store and hit
        cache.store_prediction(key.clone(), Some(true));
        cache.get_prediction(&key);
        let stats = cache.get_stats();
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_stores, 1);
        assert!(stats.hit_rate > 0.0);
    }

    #[test]
    fn cache_cleanup() {
        let cache = SpamCache::with_settings(Duration::from_millis(10), 1000);
        let metadata = create_test_metadata();
        let key = PredictionCacheKey::from_metadata(&metadata, "test_model", "v1", "1.0.0");

        // Store a result
        cache.store_prediction(key.clone(), Some(true));
        assert_eq!(cache.predictions.len(), 1);

        // Wait for expiration
        thread::sleep(Duration::from_millis(15));

        // Cleanup expired entries
        let removed = cache.cleanup_expired().unwrap();
        assert_eq!(removed, 1);
        assert_eq!(cache.predictions.len(), 0);
    }

    #[test]
    fn enhanced_lru_eviction() {
        // Small cache to trigger eviction quickly
        let cache = SpamCache::with_settings(Duration::from_secs(3600), 2);

        // Create different metadata for different contracts
        let mut metadata1 = create_test_metadata();
        metadata1.name = Some("Contract1".to_string());

        let mut metadata2 = create_test_metadata();
        metadata2.name = Some("Contract2".to_string());

        let mut metadata3 = create_test_metadata();
        metadata3.name = Some("Contract3".to_string());

        // Create keys for different contracts
        let key1 = PredictionCacheKey::from_metadata(&metadata1, "test_model", "v1", "1.0.0");
        let key2 = PredictionCacheKey::from_metadata(&metadata2, "test_model", "v1", "1.0.0");
        let key3 = PredictionCacheKey::from_metadata(&metadata3, "test_model", "v1", "1.0.0");

        // Store first entry and access it multiple times
        cache.store_prediction(key1.clone(), Some(true));
        cache.get_prediction(&key1); // access_count = 1
        cache.get_prediction(&key1); // access_count = 2
        cache.get_prediction(&key1); // access_count = 3

        // Store second entry and access it once
        cache.store_prediction(key2.clone(), Some(false));
        cache.get_prediction(&key2); // access_count = 1

        // Both should be present
        assert_eq!(cache.predictions.len(), 2);

        // Store third entry - should trigger eviction of least recently used (key2)
        cache.store_prediction(key3.clone(), None);

        // key1 should still be there (high access count), key2 should be evicted
        assert_eq!(cache.predictions.len(), 2);
        assert!(cache.get_prediction(&key1).is_some()); // Still cached
        assert!(cache.get_prediction(&key2).is_none()); // Evicted (lower access count)
        assert!(cache.get_prediction(&key3).is_some()); // Newly stored

        // Verify cache stats
        let stats = cache.get_stats();
        assert_eq!(stats.cache_evictions, 1);
        assert!(stats.utilization_rate > 0.8); // Should be high utilization
    }
}

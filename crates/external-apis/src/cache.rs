// SPDX-FileCopyrightText: 2025 Semiotic Labs
//
// SPDX-License-Identifier: Apache-2.0

//! Caching layer for external API operations
//!
//! This module provides high-performance in-memory caching for contract metadata
//! from external APIs (Moralis, Pinax) to minimize API calls and improve response times.

use std::{
    fmt::Display,
    hash::Hash,
    time::{Duration, Instant},
};

use alloy_primitives::Address;
use api_client::ContractMetadata;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use shared_types::ChainId;
use tracing::{debug, info, trace};

/// External API provider enum for type-safe provider tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ApiProvider {
    /// Moralis Web3 API
    Moralis,
    /// Pinax Analytics API
    Pinax,
}

impl Display for ApiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Moralis => write!(f, "moralis"),
            Self::Pinax => write!(f, "pinax"),
        }
    }
}

/// Cache key for contract metadata lookups
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MetadataCacheKey {
    /// Contract address
    pub address: Address,
    /// Blockchain chain identifier
    pub chain_id: ChainId,
}

impl MetadataCacheKey {
    /// Create a new metadata cache key
    pub fn new(address: Address, chain_id: ChainId) -> Self {
        Self { address, chain_id }
    }
}

/// Cached contract metadata with access tracking
#[derive(Debug, Clone)]
pub struct CachedMetadata {
    /// The contract metadata (None if no data found)
    pub metadata: Option<ContractMetadata>,
    /// When this metadata was cached
    pub cached_at: Instant,
    /// How many times this cache entry has been accessed
    pub access_count: u64,
    /// Which API provider supplied this metadata
    pub provider: ApiProvider,
}

impl CachedMetadata {
    /// Create a new cached metadata entry
    pub fn new(metadata: Option<ContractMetadata>, provider: ApiProvider) -> Self {
        Self {
            metadata,
            cached_at: Instant::now(),
            access_count: 0,
            provider,
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

    /// Get the last access time (approximation based on `cached_at` + `access_count` aging)
    pub fn last_access_time(&self) -> Instant {
        let access_aging = Duration::from_millis(self.access_count.min(1000));
        self.cached_at + access_aging
    }
}

/// High-performance cache for external API contract metadata
#[derive(Debug)]
pub struct MetadataCache {
    /// Cached contract metadata results
    metadata: DashMap<MetadataCacheKey, CachedMetadata>,
    /// Cache TTL for metadata entries
    ttl: Duration,
    /// Maximum number of cached entries
    max_entries: usize,
    /// Cache statistics
    stats: DashMap<String, u64>,
}

impl Default for MetadataCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataCache {
    /// Create a new metadata cache with default settings
    pub fn new() -> Self {
        Self {
            metadata: DashMap::new(),
            ttl: Duration::from_secs(21600), // 6 hours default
            max_entries: 50000,              // 50k entries max
            stats: DashMap::new(),
        }
    }

    /// Create a new metadata cache with custom settings
    pub fn with_settings(ttl: Duration, max_entries: usize) -> Self {
        Self {
            metadata: DashMap::new(),
            ttl,
            max_entries,
            stats: DashMap::new(),
        }
    }

    /// Get cached contract metadata
    pub fn get_metadata(&self, key: &MetadataCacheKey) -> Option<Option<ContractMetadata>> {
        if let Some(mut cached) = self.metadata.get_mut(key) {
            if cached.is_valid(self.ttl) {
                cached.accessed();
                self.increment_stat("cache_hits");
                self.increment_stat(&format!("cache_hits_{}", cached.provider));

                trace!(
                    "cache hit for address={} chain_id={} provider={}",
                    key.address,
                    key.chain_id.name(),
                    cached.provider
                );

                return Some(cached.metadata.clone());
            }
            // Remove expired entry
            let expired_provider = cached.provider.clone();
            drop(cached);
            self.metadata.remove(key);
            self.increment_stat("cache_expired");
            self.increment_stat(&format!("cache_expired_{expired_provider}"));

            debug!(
                "expired cache entry removed for address={} chain_id={}",
                key.address,
                key.chain_id.name()
            );
        }

        self.increment_stat("cache_misses");
        None
    }

    /// Store contract metadata in the cache
    pub fn store_metadata(
        &self,
        key: &MetadataCacheKey,
        metadata: Option<&ContractMetadata>,
        provider: &ApiProvider,
    ) {
        // Proactive cache management: start evicting when approaching capacity
        let current_size = self.metadata.len();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let capacity_threshold = (self.max_entries as f64 * 0.9) as usize; // 90% threshold

        if current_size >= self.max_entries {
            // At capacity - must evict
            self.evict_oldest_entry();
        } else if current_size >= capacity_threshold {
            // Approaching capacity - proactively evict expired entries
            self.cleanup_expired_sync();
        }

        let cached = CachedMetadata::new(metadata.cloned(), provider.clone());
        self.metadata.insert(key.clone(), cached);
        self.increment_stat("cache_stores");
        self.increment_stat(&format!("cache_stores_{provider}"));

        let has_metadata = metadata.is_some();
        trace!(
            "stored metadata in cache: address={} chain_id={} provider={} has_data={} (size: {}/{})",
            key.address,
            key.chain_id.name(),
            provider,
            has_metadata,
            self.metadata.len(),
            self.max_entries
        );
    }

    /// Evict the least recently used cache entry
    fn evict_oldest_entry(&self) {
        let mut lru_key: Option<MetadataCacheKey> = None;
        let mut lru_time: Option<Instant> = None;
        let mut lru_access_count = u64::MAX;

        // Find the least recently used entry using combined time + access heuristic
        for item in &self.metadata {
            let entry = item.value();
            let effective_time = entry.last_access_time();

            // Select the entry with the earliest effective time (least recently used)
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
            let evicted_entry = self.metadata.remove(&key);
            self.increment_stat("cache_evictions");

            if let Some((_, entry)) = evicted_entry {
                self.increment_stat(&format!("cache_evictions_{}", entry.provider));
                info!(
                    address = %key.address,
                    chain_id = %key.chain_id,
                    provider = %entry.provider,
                    access_count = entry.access_count,
                    age_ms = entry.cached_at.elapsed().as_millis(),
                    remaining_entries = self.metadata.len(),
                    "evicted lru cache entry due to capacity limit"
                );
            }
        }
    }

    /// Synchronous cleanup of expired entries (used for proactive management)
    fn cleanup_expired_sync(&self) {
        let mut keys_to_remove = Vec::new();

        // Collect expired keys
        for item in &self.metadata {
            if !item.value().is_valid(self.ttl) {
                keys_to_remove.push(item.key().clone());
            }
        }

        // Remove expired entries and track by provider
        let mut removed_count = 0;
        let mut removed_by_provider: std::collections::HashMap<ApiProvider, usize> =
            std::collections::HashMap::new();

        for key in keys_to_remove {
            if let Some((_, entry)) = self.metadata.remove(&key) {
                removed_count += 1;
                *removed_by_provider.entry(entry.provider).or_insert(0) += 1;
            }
        }

        if removed_count > 0 {
            self.increment_stat("cache_expired");

            // Update provider-specific stats
            for (provider, count) in removed_by_provider {
                for _ in 0..count {
                    self.increment_stat(&format!("cache_expired_{provider}"));
                }
            }

            let stats = self.get_stats();
            info!(
                removed_entries = removed_count,
                remaining_entries = stats.entry_count,
                utilization_rate = stats.utilization_rate,
                hit_rate = stats.hit_rate,
                "proactively cleaned up expired cache entries"
            );
        }
    }

    /// Clear all cached metadata
    pub fn clear_metadata(&self) {
        self.metadata.clear();
        debug!("cleared all cached metadata");
    }

    /// Clear all caches and statistics
    pub fn clear_all(&self) {
        self.clear_metadata();
        self.stats.clear();
        debug!("cleared all caches and statistics");
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> MetadataCacheStats {
        let cache_hits = self.get_stat("cache_hits");
        let cache_misses = self.get_stat("cache_misses");
        let total_requests = cache_hits + cache_misses;
        #[allow(clippy::cast_precision_loss)]
        let hit_rate = if total_requests > 0 {
            cache_hits as f64 / total_requests as f64
        } else {
            0.0
        };

        // Calculate cache utilization
        let current_size = self.metadata.len();
        #[allow(clippy::cast_precision_loss)]
        let utilization_rate = if self.max_entries > 0 {
            current_size as f64 / self.max_entries as f64
        } else {
            0.0
        };

        // Calculate average access count
        let total_access_count: u64 = self
            .metadata
            .iter()
            .map(|item| item.value().access_count)
            .sum();
        #[allow(clippy::cast_precision_loss)]
        let avg_access_count = if current_size > 0 {
            total_access_count as f64 / current_size as f64
        } else {
            0.0
        };

        // Provider-specific stats
        let moralis_hits = self.get_stat("cache_hits_moralis");
        let pinax_hits = self.get_stat("cache_hits_pinax");

        MetadataCacheStats {
            entry_count: current_size,
            cache_hits,
            cache_misses,
            cache_stores: self.get_stat("cache_stores"),
            cache_evictions: self.get_stat("cache_evictions"),
            cache_expired: self.get_stat("cache_expired"),
            hit_rate,
            utilization_rate,
            avg_access_count,
            max_capacity: self.max_entries,
            ttl_seconds: self.ttl.as_secs(),
            moralis_hits,
            pinax_hits,
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
        self.stats.get(key).map_or(0, |v| *v)
    }

    /// Clean up expired entries (maintenance operation)
    pub fn cleanup_expired(&self) -> Result<usize, String> {
        let mut removed_count = 0;
        let mut keys_to_remove = Vec::new();

        // Collect expired keys
        for item in &self.metadata {
            if !item.value().is_valid(self.ttl) {
                keys_to_remove.push(item.key().clone());
            }
        }

        // Remove expired entries
        for key in keys_to_remove {
            if self.metadata.remove(&key).is_some() {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            self.stats.insert(
                "cache_expired".to_string(),
                self.get_stat("cache_expired") + removed_count as u64,
            );

            debug!("cleaned up {} expired cache entries", removed_count);
        }

        Ok(removed_count)
    }
}

/// Cache statistics and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataCacheStats {
    /// Number of cached entries
    pub entry_count: usize,
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
    /// TTL in seconds
    pub ttl_seconds: u64,
    /// Cache hits from Moralis provider
    pub moralis_hits: u64,
    /// Cache hits from Pinax provider
    pub pinax_hits: u64,
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
        let key = MetadataCacheKey::new(Address::ZERO, ChainId::Ethereum);
        assert_eq!(key.address, Address::ZERO);
        assert_eq!(key.chain_id, ChainId::Ethereum);
    }

    #[test]
    fn api_provider_display() {
        assert_eq!(ApiProvider::Moralis.to_string(), "moralis");
        assert_eq!(ApiProvider::Pinax.to_string(), "pinax");
    }

    #[test]
    fn metadata_caching() {
        let cache = MetadataCache::new();
        let key = MetadataCacheKey::new(Address::ZERO, ChainId::Ethereum);
        let metadata = create_test_metadata();

        // Initially no cached result
        assert_eq!(cache.get_metadata(&key), None);

        // Store metadata
        cache.store_metadata(&key, Some(&metadata), &ApiProvider::Moralis);

        // Should retrieve the cached result
        let cached = cache.get_metadata(&key);
        assert!(cached.is_some());
        let cached_metadata = cached.unwrap();
        assert!(cached_metadata.is_some());
        assert_eq!(cached_metadata.unwrap().name, metadata.name);

        // Store None result
        cache.store_metadata(&key, None, &ApiProvider::Pinax);
        assert_eq!(cache.get_metadata(&key), Some(None));
    }

    #[test]
    fn cache_expiration() {
        let cache = MetadataCache::with_settings(Duration::from_millis(10), 1000);
        let key = MetadataCacheKey::new(Address::ZERO, ChainId::Ethereum);
        let metadata = create_test_metadata();

        // Store metadata
        cache.store_metadata(&key, Some(&metadata), &ApiProvider::Moralis);
        assert!(cache.get_metadata(&key).is_some());

        // Wait for expiration
        thread::sleep(Duration::from_millis(15));

        // Should not return expired result
        assert_eq!(cache.get_metadata(&key), None);
    }

    #[test]
    fn cache_statistics() {
        let cache = MetadataCache::new();
        let key = MetadataCacheKey::new(Address::ZERO, ChainId::Ethereum);
        let metadata = create_test_metadata();

        let initial_stats = cache.get_stats();
        assert_eq!(initial_stats.cache_hits, 0);
        assert_eq!(initial_stats.cache_misses, 0);
        assert!((initial_stats.utilization_rate - 0.0).abs() < f64::EPSILON);
        assert_eq!(initial_stats.max_capacity, 50000);

        // Miss
        cache.get_metadata(&key);
        let stats = cache.get_stats();
        assert_eq!(stats.cache_misses, 1);

        // Store and hit
        cache.store_metadata(&key, Some(&metadata), &ApiProvider::Moralis);
        cache.get_metadata(&key);
        let stats = cache.get_stats();
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_stores, 1);
        assert_eq!(stats.moralis_hits, 1);
        assert_eq!(stats.pinax_hits, 0);
        assert!(stats.hit_rate > 0.0);
    }

    #[test]
    fn cache_cleanup() {
        let cache = MetadataCache::with_settings(Duration::from_millis(10), 1000);
        let key = MetadataCacheKey::new(Address::ZERO, ChainId::Ethereum);
        let metadata = create_test_metadata();

        // Store metadata
        cache.store_metadata(&key, Some(&metadata), &ApiProvider::Moralis);
        assert_eq!(cache.metadata.len(), 1);

        // Wait for expiration
        thread::sleep(Duration::from_millis(15));

        // Cleanup expired entries
        let removed = cache.cleanup_expired().unwrap();
        assert_eq!(removed, 1);
        assert_eq!(cache.metadata.len(), 0);
    }

    #[test]
    fn lru_eviction() {
        // Small cache to trigger eviction quickly
        let cache = MetadataCache::with_settings(Duration::from_secs(3600), 2);

        let key1 = MetadataCacheKey::new(Address::from([1u8; 20]), ChainId::Ethereum);
        let key2 = MetadataCacheKey::new(Address::from([2u8; 20]), ChainId::Ethereum);
        let key3 = MetadataCacheKey::new(Address::from([3u8; 20]), ChainId::Ethereum);

        let metadata = create_test_metadata();

        // Store first entry and access it multiple times
        cache.store_metadata(&key1, Some(&metadata), &ApiProvider::Moralis);
        cache.get_metadata(&key1); // access_count = 1
        cache.get_metadata(&key1); // access_count = 2

        // Store second entry and access it once
        cache.store_metadata(&key2, Some(&metadata), &ApiProvider::Pinax);
        cache.get_metadata(&key2); // access_count = 1

        // Both should be present
        assert_eq!(cache.metadata.len(), 2);

        // Store third entry - should trigger eviction of least recently used (key2)
        cache.store_metadata(&key3, Some(&metadata), &ApiProvider::Moralis);

        // key1 should still be there (higher access count), key2 should be evicted
        assert_eq!(cache.metadata.len(), 2);
        assert!(cache.get_metadata(&key1).is_some()); // Still cached
        assert!(cache.get_metadata(&key2).is_none()); // Evicted
        assert!(cache.get_metadata(&key3).is_some()); // Newly stored

        // Verify cache stats
        let stats = cache.get_stats();
        assert_eq!(stats.cache_evictions, 1);
        assert!(stats.utilization_rate > 0.8); // Should be high utilization
    }
}

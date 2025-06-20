//! Cache statistics

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of get requests
    pub total_gets: u64,

    /// Number of cache hits
    pub hits: u64,

    /// Number of cache misses
    pub misses: u64,

    /// Total number of put requests
    pub total_puts: u64,

    /// Total number of evictions
    pub evictions: u64,

    /// Current number of entries
    pub entry_count: usize,

    /// Current memory usage in bytes
    pub memory_usage_bytes: Option<usize>,

    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f64,

    /// Average get latency in microseconds
    pub avg_get_latency_us: Option<f64>,

    /// Average put latency in microseconds
    pub avg_put_latency_us: Option<f64>,
}

impl CacheStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self {
            total_gets: 0,
            hits: 0,
            misses: 0,
            total_puts: 0,
            evictions: 0,
            entry_count: 0,
            memory_usage_bytes: None,
            hit_rate: 0.0,
            avg_get_latency_us: None,
            avg_put_latency_us: None,
        }
    }

    /// Calculate hit rate
    pub fn calculate_hit_rate(&mut self) {
        if self.total_gets > 0 {
            self.hit_rate = self.hits as f64 / self.total_gets as f64;
        } else {
            self.hit_rate = 0.0;
        }
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe statistics collector
#[derive(Debug)]
pub struct StatsCollector {
    total_gets: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
    total_puts: AtomicU64,
    evictions: AtomicU64,

    // For latency tracking
    total_get_latency_ns: AtomicU64,
    total_put_latency_ns: AtomicU64,
}

impl StatsCollector {
    /// Create a new stats collector
    pub fn new() -> Self {
        Self {
            total_gets: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            total_puts: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            total_get_latency_ns: AtomicU64::new(0),
            total_put_latency_ns: AtomicU64::new(0),
        }
    }

    /// Record a cache hit
    pub fn record_hit(&self) {
        self.total_gets.fetch_add(1, Ordering::Relaxed);
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss
    pub fn record_miss(&self) {
        self.total_gets.fetch_add(1, Ordering::Relaxed);
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a put operation
    pub fn record_put(&self) {
        self.total_puts.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an eviction
    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record get latency
    pub fn record_get_latency(&self, latency_ns: u64) {
        self.total_get_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);
    }

    /// Record put latency
    pub fn record_put_latency(&self, latency_ns: u64) {
        self.total_put_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);
    }

    /// Get current stats
    pub fn get_stats(&self, entry_count: usize, memory_usage: Option<usize>) -> CacheStats {
        let total_gets = self.total_gets.load(Ordering::Relaxed);
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total_puts = self.total_puts.load(Ordering::Relaxed);
        let evictions = self.evictions.load(Ordering::Relaxed);

        let hit_rate = if total_gets > 0 {
            hits as f64 / total_gets as f64
        } else {
            0.0
        };

        let avg_get_latency_us = if total_gets > 0 {
            Some(self.total_get_latency_ns.load(Ordering::Relaxed) as f64 / total_gets as f64 / 1000.0)
        } else {
            None
        };

        let avg_put_latency_us = if total_puts > 0 {
            Some(self.total_put_latency_ns.load(Ordering::Relaxed) as f64 / total_puts as f64 / 1000.0)
        } else {
            None
        };

        CacheStats {
            total_gets,
            hits,
            misses,
            total_puts,
            evictions,
            entry_count,
            memory_usage_bytes: memory_usage,
            hit_rate,
            avg_get_latency_us,
            avg_put_latency_us,
        }
    }
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared stats collector
pub type SharedStatsCollector = Arc<StatsCollector>;

/// Create a new shared stats collector
pub fn create_stats_collector() -> SharedStatsCollector {
    Arc::new(StatsCollector::new())
}

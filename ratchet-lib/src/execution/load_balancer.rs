use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Worker metrics for load balancing decisions
#[derive(Debug)]
pub struct WorkerMetrics {
    pub tasks_in_flight: AtomicU32,
    pub total_tasks: AtomicU64,
    pub total_failures: AtomicU64,
    pub last_task_duration_ms: AtomicU64,
    pub memory_usage_mb: AtomicU64,
    pub cpu_usage_percent: AtomicU32, // Stored as percentage * 100 for atomic operations
    pub last_activity: Arc<RwLock<Instant>>,
}

impl WorkerMetrics {
    pub fn new() -> Self {
        Self {
            tasks_in_flight: AtomicU32::new(0),
            total_tasks: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            last_task_duration_ms: AtomicU64::new(0),
            memory_usage_mb: AtomicU64::new(0),
            cpu_usage_percent: AtomicU32::new(0),
            last_activity: Arc::new(RwLock::new(Instant::now())),
        }
    }
}

impl Default for WorkerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerMetrics {
    pub async fn record_task_start(&self) {
        self.tasks_in_flight.fetch_add(1, Ordering::Relaxed);
        self.total_tasks.fetch_add(1, Ordering::Relaxed);
        *self.last_activity.write().await = Instant::now();
    }

    pub async fn record_task_completion(&self, duration_ms: u64, success: bool) {
        self.tasks_in_flight.fetch_sub(1, Ordering::Relaxed);
        self.last_task_duration_ms.store(duration_ms, Ordering::Relaxed);
        
        if !success {
            self.total_failures.fetch_add(1, Ordering::Relaxed);
        }
        
        *self.last_activity.write().await = Instant::now();
    }

    pub fn update_system_metrics(&self, memory_mb: u64, cpu_percent: f32) {
        self.memory_usage_mb.store(memory_mb, Ordering::Relaxed);
        self.cpu_usage_percent.store((cpu_percent * 100.0) as u32, Ordering::Relaxed);
    }

    pub fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage_percent.load(Ordering::Relaxed) as f32 / 100.0
    }

    pub async fn get_idle_time(&self) -> Duration {
        self.last_activity.read().await.elapsed()
    }

    pub fn get_failure_rate(&self) -> f32 {
        let total = self.total_tasks.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            let failures = self.total_failures.load(Ordering::Relaxed);
            failures as f32 / total as f32
        }
    }
}

/// Worker health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Worker information for load balancing
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub id: String,
    pub metrics: Arc<WorkerMetrics>,
    pub health: WorkerHealth,
    pub max_concurrent_tasks: u32,
    pub created_at: Instant,
}

impl WorkerInfo {
    pub fn new(id: String, max_concurrent_tasks: u32) -> Self {
        Self {
            id,
            metrics: Arc::new(WorkerMetrics::new()),
            health: WorkerHealth::Unknown,
            max_concurrent_tasks,
            created_at: Instant::now(),
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self.health, WorkerHealth::Healthy | WorkerHealth::Degraded) &&
        self.metrics.tasks_in_flight.load(Ordering::Relaxed) < self.max_concurrent_tasks
    }

    pub async fn calculate_load_score(&self) -> u64 {
        let tasks_in_flight = self.metrics.tasks_in_flight.load(Ordering::Relaxed) as u64;
        let memory_usage = self.metrics.memory_usage_mb.load(Ordering::Relaxed);
        let cpu_usage = self.metrics.get_cpu_usage() as u64;
        let failure_rate = (self.metrics.get_failure_rate() * 1000.0) as u64;
        let idle_time_penalty = self.metrics.get_idle_time().await.as_secs();

        // Calculate composite score (lower is better)
        let utilization_score = (tasks_in_flight * 1000) / (self.max_concurrent_tasks as u64);
        let resource_score = memory_usage / 10 + cpu_usage * 10;
        let reliability_score = failure_rate;
        let freshness_penalty = if idle_time_penalty > 300 { idle_time_penalty } else { 0 };

        utilization_score + resource_score + reliability_score + freshness_penalty
    }
}

/// Load balancing strategy
#[async_trait::async_trait]
pub trait LoadBalancingStrategy: Send + Sync {
    async fn select_worker(&self, workers: &[WorkerInfo]) -> Option<String>;
}

/// Round-robin load balancing
pub struct RoundRobinStrategy {
    counter: AtomicU32,
}

impl RoundRobinStrategy {
    pub fn new() -> Self {
        Self {
            counter: AtomicU32::new(0),
        }
    }
}

#[async_trait::async_trait]
impl LoadBalancingStrategy for RoundRobinStrategy {
    async fn select_worker(&self, workers: &[WorkerInfo]) -> Option<String> {
        let available_workers: Vec<_> = workers.iter()
            .filter(|w| w.is_available())
            .collect();

        if available_workers.is_empty() {
            return None;
        }

        let index = self.counter.fetch_add(1, Ordering::Relaxed) as usize % available_workers.len();
        Some(available_workers[index].id.clone())
    }
}

/// Least loaded strategy (selects worker with lowest load score)
pub struct LeastLoadedStrategy;

#[async_trait::async_trait]
impl LoadBalancingStrategy for LeastLoadedStrategy {
    async fn select_worker(&self, workers: &[WorkerInfo]) -> Option<String> {
        let mut best_worker = None;
        let mut best_score = u64::MAX;

        for worker in workers.iter().filter(|w| w.is_available()) {
            let score = worker.calculate_load_score().await;
            debug!("Worker {} load score: {}", worker.id, score);
            
            if score < best_score {
                best_score = score;
                best_worker = Some(worker.id.clone());
            }
        }

        if let Some(ref worker_id) = best_worker {
            debug!("Selected worker {} with score {}", worker_id, best_score);
        }

        best_worker
    }
}

/// Weighted round-robin strategy
pub struct WeightedRoundRobinStrategy {
    weights: HashMap<String, u32>,
    current_weights: Arc<RwLock<HashMap<String, i32>>>,
}

impl WeightedRoundRobinStrategy {
    pub fn new(weights: HashMap<String, u32>) -> Self {
        Self {
            weights,
            current_weights: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl LoadBalancingStrategy for WeightedRoundRobinStrategy {
    async fn select_worker(&self, workers: &[WorkerInfo]) -> Option<String> {
        let available_workers: Vec<_> = workers.iter()
            .filter(|w| w.is_available())
            .collect();

        if available_workers.is_empty() {
            return None;
        }

        let mut current_weights = self.current_weights.write().await;
        let mut best_worker = None;
        let mut best_weight = i32::MIN;

        for worker in &available_workers {
            let weight = self.weights.get(&worker.id).copied().unwrap_or(1) as i32;
            let current = current_weights.entry(worker.id.clone()).or_insert(0);
            *current += weight;

            if *current > best_weight {
                best_weight = *current;
                best_worker = Some(worker.id.clone());
            }
        }

        // Reduce the weight of the selected worker
        if let Some(ref worker_id) = best_worker {
            let total_weight: i32 = self.weights.values().map(|&w| w as i32).sum();
            if let Some(current) = current_weights.get_mut(worker_id) {
                *current -= total_weight;
            }
        }

        best_worker
    }
}

/// Load balancer manager
pub struct LoadBalancer {
    workers: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    strategy: Box<dyn LoadBalancingStrategy>,
    health_check_interval: Duration,
}

impl LoadBalancer {
    pub fn new(strategy: Box<dyn LoadBalancingStrategy>) -> Self {
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
            strategy,
            health_check_interval: Duration::from_secs(30),
        }
    }

    /// Add a worker to the pool
    pub async fn add_worker(&self, worker: WorkerInfo) {
        info!("Adding worker {} to load balancer", worker.id);
        let mut workers = self.workers.write().await;
        workers.insert(worker.id.clone(), worker);
    }

    /// Remove a worker from the pool
    pub async fn remove_worker(&self, worker_id: &str) {
        info!("Removing worker {} from load balancer", worker_id);
        let mut workers = self.workers.write().await;
        workers.remove(worker_id);
    }

    /// Select the best worker for a new task
    pub async fn select_worker(&self) -> Option<String> {
        let workers = self.workers.read().await;
        let worker_list: Vec<_> = workers.values().cloned().collect();
        drop(workers);

        self.strategy.select_worker(&worker_list).await
    }

    /// Update worker health status
    pub async fn update_worker_health(&self, worker_id: &str, health: WorkerHealth) {
        let mut workers = self.workers.write().await;
        if let Some(worker) = workers.get_mut(worker_id) {
            worker.health = health;
            debug!("Updated worker {} health to {:?}", worker_id, health);
        }
    }

    /// Get worker metrics
    pub async fn get_worker_metrics(&self, worker_id: &str) -> Option<Arc<WorkerMetrics>> {
        let workers = self.workers.read().await;
        workers.get(worker_id).map(|w| w.metrics.clone())
    }

    /// Get load balancer statistics
    pub async fn get_statistics(&self) -> LoadBalancerStats {
        let workers = self.workers.read().await;
        
        let total_workers = workers.len();
        let healthy_workers = workers.values()
            .filter(|w| w.health == WorkerHealth::Healthy)
            .count();
        let degraded_workers = workers.values()
            .filter(|w| w.health == WorkerHealth::Degraded)
            .count();
        let unhealthy_workers = workers.values()
            .filter(|w| w.health == WorkerHealth::Unhealthy)
            .count();

        let total_tasks_in_flight = workers.values()
            .map(|w| w.metrics.tasks_in_flight.load(Ordering::Relaxed) as u64)
            .sum();

        let total_tasks_completed = workers.values()
            .map(|w| w.metrics.total_tasks.load(Ordering::Relaxed))
            .sum();

        let total_failures = workers.values()
            .map(|w| w.metrics.total_failures.load(Ordering::Relaxed))
            .sum();

        LoadBalancerStats {
            total_workers,
            healthy_workers,
            degraded_workers,
            unhealthy_workers,
            total_tasks_in_flight,
            total_tasks_completed,
            total_failures,
        }
    }

    /// Start background health checking
    pub async fn start_health_monitor(self: Arc<Self>) {
        let mut interval = tokio::time::interval(self.health_check_interval);
        
        loop {
            interval.tick().await;
            self.perform_health_checks().await;
        }
    }

    async fn perform_health_checks(&self) {
        let workers = self.workers.read().await;
        let worker_ids: Vec<_> = workers.keys().cloned().collect();
        drop(workers);

        for worker_id in worker_ids {
            let health = self.check_worker_health(&worker_id).await;
            self.update_worker_health(&worker_id, health).await;
        }
    }

    async fn check_worker_health(&self, worker_id: &str) -> WorkerHealth {
        let workers = self.workers.read().await;
        let worker = match workers.get(worker_id) {
            Some(w) => w,
            None => return WorkerHealth::Unknown,
        };

        let failure_rate = worker.metrics.get_failure_rate();
        let idle_time = worker.metrics.get_idle_time().await;
        let cpu_usage = worker.metrics.get_cpu_usage();
        let memory_usage = worker.metrics.memory_usage_mb.load(Ordering::Relaxed);

        // Health determination logic
        if failure_rate > 0.5 || idle_time > Duration::from_secs(600) {
            WorkerHealth::Unhealthy
        } else if failure_rate > 0.1 || cpu_usage > 80.0 || memory_usage > 8192 {
            WorkerHealth::Degraded
        } else {
            WorkerHealth::Healthy
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerStats {
    pub total_workers: usize,
    pub healthy_workers: usize,
    pub degraded_workers: usize,
    pub unhealthy_workers: usize,
    pub total_tasks_in_flight: u64,
    pub total_tasks_completed: u64,
    pub total_failures: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_metrics() {
        let metrics = WorkerMetrics::new();
        
        assert_eq!(metrics.tasks_in_flight.load(Ordering::Relaxed), 0);
        
        metrics.record_task_start().await;
        assert_eq!(metrics.tasks_in_flight.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.total_tasks.load(Ordering::Relaxed), 1);
        
        metrics.record_task_completion(1000, true).await;
        assert_eq!(metrics.tasks_in_flight.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.last_task_duration_ms.load(Ordering::Relaxed), 1000);
        assert_eq!(metrics.total_failures.load(Ordering::Relaxed), 0);
        
        metrics.record_task_completion(500, false).await;
        assert_eq!(metrics.total_failures.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_round_robin_strategy() {
        let strategy = RoundRobinStrategy::new();
        
        let workers = vec![
            WorkerInfo::new("worker1".to_string(), 10),
            WorkerInfo::new("worker2".to_string(), 10),
            WorkerInfo::new("worker3".to_string(), 10),
        ];

        // Set all workers as healthy
        let mut workers = workers;
        for worker in &mut workers {
            worker.health = WorkerHealth::Healthy;
        }

        // Test round-robin selection
        let selected1 = strategy.select_worker(&workers).await.unwrap();
        let selected2 = strategy.select_worker(&workers).await.unwrap();
        let selected3 = strategy.select_worker(&workers).await.unwrap();
        let selected4 = strategy.select_worker(&workers).await.unwrap();

        // Should cycle through workers
        assert_ne!(selected1, selected2);
        assert_ne!(selected2, selected3);
        assert_eq!(selected1, selected4); // Should wrap around
    }

    #[tokio::test]
    async fn test_least_loaded_strategy() {
        let strategy = LeastLoadedStrategy;
        
        let mut workers = vec![
            WorkerInfo::new("worker1".to_string(), 10),
            WorkerInfo::new("worker2".to_string(), 10),
            WorkerInfo::new("worker3".to_string(), 10),
        ];

        // Set all workers as healthy
        for worker in &mut workers {
            worker.health = WorkerHealth::Healthy;
        }

        // Load worker1 with tasks
        workers[0].metrics.tasks_in_flight.store(5, Ordering::Relaxed);
        workers[1].metrics.tasks_in_flight.store(2, Ordering::Relaxed);
        workers[2].metrics.tasks_in_flight.store(8, Ordering::Relaxed);

        let selected = strategy.select_worker(&workers).await.unwrap();
        assert_eq!(selected, "worker2"); // Should select least loaded
    }

    #[tokio::test]
    async fn test_load_balancer() {
        let strategy = Box::new(LeastLoadedStrategy);
        let balancer = LoadBalancer::new(strategy);

        let worker = WorkerInfo::new("test_worker".to_string(), 5);
        balancer.add_worker(worker).await;

        balancer.update_worker_health("test_worker", WorkerHealth::Healthy).await;

        let selected = balancer.select_worker().await;
        assert_eq!(selected, Some("test_worker".to_string()));

        let stats = balancer.get_statistics().await;
        assert_eq!(stats.total_workers, 1);
        assert_eq!(stats.healthy_workers, 1);
    }
}
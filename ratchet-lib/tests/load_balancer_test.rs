/// Load balancer integration tests
use ratchet_lib::execution::load_balancer::{
    LoadBalancer, WorkerInfo, WorkerHealth,
    RoundRobinStrategy, LeastLoadedStrategy, WeightedRoundRobinStrategy,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_round_robin_load_distribution() {
    let strategy = Box::new(RoundRobinStrategy::new());
    let balancer = Arc::new(LoadBalancer::new(strategy));
    
    // Add multiple workers
    for i in 0..4 {
        let mut worker = WorkerInfo::new(format!("worker-{}", i), 10);
        worker.health = WorkerHealth::Healthy;
        balancer.add_worker(worker).await;
    }
    
    // Test distribution
    let mut selection_counts: HashMap<String, u32> = HashMap::new();
    for _ in 0..40 {
        if let Some(worker_id) = balancer.select_worker().await {
            *selection_counts.entry(worker_id).or_insert(0) += 1;
        }
    }
    
    // Should be evenly distributed
    for (worker_id, count) in &selection_counts {
        assert_eq!(*count, 10, "Worker {} should have been selected 10 times", worker_id);
    }
}

#[tokio::test]
async fn test_least_loaded_strategy_with_varying_loads() {
    let strategy = Box::new(LeastLoadedStrategy);
    let balancer = Arc::new(LoadBalancer::new(strategy));
    
    // Create workers with different loads
    for i in 0..3 {
        let mut worker = WorkerInfo::new(format!("worker-{}", i), 10);
        worker.health = WorkerHealth::Healthy;
        
        // Set different loads
        worker.metrics.tasks_in_flight.store(i as u32 * 2, Ordering::Relaxed);
        worker.metrics.memory_usage_mb.store((i + 1) as u64 * 1000, Ordering::Relaxed);
        worker.metrics.update_system_metrics((i + 1) as u64 * 1000, (i as f32 + 1.0) * 10.0);
        
        balancer.add_worker(worker).await;
    }
    
    // Should always select the least loaded worker
    for _ in 0..5 {
        let selected = balancer.select_worker().await.unwrap();
        assert_eq!(selected, "worker-0", "Should select the least loaded worker");
    }
}

#[tokio::test]
async fn test_weighted_round_robin_distribution() {
    let mut weights = HashMap::new();
    weights.insert("worker-heavy".to_string(), 3);
    weights.insert("worker-light".to_string(), 1);
    
    let strategy = Box::new(WeightedRoundRobinStrategy::new(weights));
    let balancer = Arc::new(LoadBalancer::new(strategy));
    
    // Add weighted workers
    let mut heavy_worker = WorkerInfo::new("worker-heavy".to_string(), 10);
    heavy_worker.health = WorkerHealth::Healthy;
    balancer.add_worker(heavy_worker).await;
    
    let mut light_worker = WorkerInfo::new("worker-light".to_string(), 10);
    light_worker.health = WorkerHealth::Healthy;
    balancer.add_worker(light_worker).await;
    
    // Test distribution
    let mut selection_counts: HashMap<String, u32> = HashMap::new();
    for _ in 0..40 {
        if let Some(worker_id) = balancer.select_worker().await {
            *selection_counts.entry(worker_id).or_insert(0) += 1;
        }
    }
    
    // Heavy worker should be selected approximately 3x more often
    let heavy_count = selection_counts.get("worker-heavy").unwrap_or(&0);
    let light_count = selection_counts.get("worker-light").unwrap_or(&0);
    let ratio = *heavy_count as f32 / *light_count as f32;
    assert!(ratio > 2.5 && ratio < 3.5, "Ratio should be approximately 3:1");
}

#[tokio::test]
async fn test_worker_health_monitoring() {
    let strategy = Box::new(RoundRobinStrategy::new());
    let balancer = Arc::new(LoadBalancer::new(strategy));
    
    // Add workers with different health states
    let mut healthy_worker = WorkerInfo::new("healthy".to_string(), 10);
    healthy_worker.health = WorkerHealth::Healthy;
    balancer.add_worker(healthy_worker).await;
    
    let mut degraded_worker = WorkerInfo::new("degraded".to_string(), 10);
    degraded_worker.health = WorkerHealth::Degraded;
    balancer.add_worker(degraded_worker).await;
    
    let mut unhealthy_worker = WorkerInfo::new("unhealthy".to_string(), 10);
    unhealthy_worker.health = WorkerHealth::Unhealthy;
    balancer.add_worker(unhealthy_worker).await;
    
    // Should only select healthy and degraded workers
    let mut selected_workers = std::collections::HashSet::new();
    for _ in 0..20 {
        if let Some(worker_id) = balancer.select_worker().await {
            selected_workers.insert(worker_id);
        }
    }
    
    assert!(selected_workers.contains("healthy"));
    assert!(selected_workers.contains("degraded"));
    assert!(!selected_workers.contains("unhealthy"), "Unhealthy worker should not be selected");
    
    // Update unhealthy worker to healthy
    balancer.update_worker_health("unhealthy", WorkerHealth::Healthy).await;
    
    // Now it should be selected
    for _ in 0..10 {
        if let Some(worker_id) = balancer.select_worker().await {
            if worker_id == "unhealthy" {
                // Test passed
                return;
            }
        }
    }
    panic!("Previously unhealthy worker should now be selected");
}

#[tokio::test]
async fn test_worker_metrics_tracking() {
    let strategy = Box::new(LeastLoadedStrategy);
    let balancer = Arc::new(LoadBalancer::new(strategy));
    
    let mut worker = WorkerInfo::new("test-worker".to_string(), 5);
    worker.health = WorkerHealth::Healthy;
    balancer.add_worker(worker).await;
    
    // Get metrics reference
    let metrics = balancer.get_worker_metrics("test-worker").await.unwrap();
    
    // Simulate task execution
    for i in 0..5 {
        metrics.record_task_start().await;
        sleep(Duration::from_millis(10)).await;
        metrics.record_task_completion(10 + i * 5, i % 2 == 0).await;
    }
    
    // Verify metrics
    assert_eq!(metrics.total_tasks.load(Ordering::Relaxed), 5);
    assert_eq!(metrics.total_failures.load(Ordering::Relaxed), 2);
    assert_eq!(metrics.tasks_in_flight.load(Ordering::Relaxed), 0);
    
    let failure_rate = metrics.get_failure_rate();
    assert!((failure_rate - 0.4).abs() < 0.01, "Failure rate should be approximately 0.4");
}

#[tokio::test]
async fn test_load_balancer_statistics() {
    let strategy = Box::new(RoundRobinStrategy::new());
    let balancer = Arc::new(LoadBalancer::new(strategy));
    
    // Add workers with different states
    for i in 0..4 {
        let mut worker = WorkerInfo::new(format!("worker-{}", i), 10);
        worker.health = match i {
            0 => WorkerHealth::Healthy,
            1 => WorkerHealth::Healthy,
            2 => WorkerHealth::Degraded,
            _ => WorkerHealth::Unhealthy,
        };
        
        // Add some metrics
        worker.metrics.tasks_in_flight.store(i as u32, Ordering::Relaxed);
        worker.metrics.total_tasks.store((i + 1) as u64 * 10, Ordering::Relaxed);
        worker.metrics.total_failures.store(i as u64, Ordering::Relaxed);
        
        balancer.add_worker(worker).await;
    }
    
    let stats = balancer.get_statistics().await;
    
    assert_eq!(stats.total_workers, 4);
    assert_eq!(stats.healthy_workers, 2);
    assert_eq!(stats.degraded_workers, 1);
    assert_eq!(stats.unhealthy_workers, 1);
    assert_eq!(stats.total_tasks_in_flight, 6); // 0 + 1 + 2 + 3
    assert_eq!(stats.total_tasks_completed, 100); // 10 + 20 + 30 + 40
    assert_eq!(stats.total_failures, 6); // 0 + 1 + 2 + 3
}

#[tokio::test]
async fn test_worker_capacity_limits() {
    let strategy = Box::new(RoundRobinStrategy::new());
    let balancer = Arc::new(LoadBalancer::new(strategy));
    
    // Add worker with limited capacity
    let mut worker = WorkerInfo::new("limited-worker".to_string(), 3);
    worker.health = WorkerHealth::Healthy;
    balancer.add_worker(worker).await;
    
    let metrics = balancer.get_worker_metrics("limited-worker").await.unwrap();
    
    // Fill up to capacity
    for _ in 0..3 {
        metrics.record_task_start().await;
    }
    
    // Worker should not be available
    let selected = balancer.select_worker().await;
    assert!(selected.is_none(), "Worker at capacity should not be selected");
    
    // Complete one task
    metrics.record_task_completion(100, true).await;
    
    // Now worker should be available again
    let selected = balancer.select_worker().await;
    assert_eq!(selected, Some("limited-worker".to_string()));
}

#[tokio::test]
async fn test_dynamic_worker_addition_removal() {
    let strategy = Box::new(RoundRobinStrategy::new());
    let balancer = Arc::new(LoadBalancer::new(strategy));
    
    // Start with no workers
    assert!(balancer.select_worker().await.is_none());
    
    // Add workers dynamically
    for i in 0..3 {
        let mut worker = WorkerInfo::new(format!("dynamic-{}", i), 10);
        worker.health = WorkerHealth::Healthy;
        balancer.add_worker(worker).await;
    }
    
    // Verify all workers are being selected
    let mut selected_workers = std::collections::HashSet::new();
    for _ in 0..9 {
        if let Some(worker_id) = balancer.select_worker().await {
            selected_workers.insert(worker_id);
        }
    }
    assert_eq!(selected_workers.len(), 3);
    
    // Remove a worker
    balancer.remove_worker("dynamic-1").await;
    
    // Verify removed worker is no longer selected
    selected_workers.clear();
    for _ in 0..20 {
        if let Some(worker_id) = balancer.select_worker().await {
            selected_workers.insert(worker_id);
        }
    }
    assert!(!selected_workers.contains("dynamic-1"));
    assert_eq!(selected_workers.len(), 2);
}

#[tokio::test]
async fn test_idle_time_tracking() {
    let strategy = Box::new(LeastLoadedStrategy);
    let balancer = Arc::new(LoadBalancer::new(strategy));
    
    let mut worker = WorkerInfo::new("idle-worker".to_string(), 10);
    worker.health = WorkerHealth::Healthy;
    balancer.add_worker(worker).await;
    
    let metrics = balancer.get_worker_metrics("idle-worker").await.unwrap();
    
    // Check initial idle time
    let initial_idle = metrics.get_idle_time().await;
    assert!(initial_idle < Duration::from_millis(100));
    
    // Wait and check again
    sleep(Duration::from_millis(200)).await;
    let later_idle = metrics.get_idle_time().await;
    assert!(later_idle >= Duration::from_millis(200));
    
    // Record activity
    metrics.record_task_start().await;
    let post_activity_idle = metrics.get_idle_time().await;
    assert!(post_activity_idle < Duration::from_millis(50));
}
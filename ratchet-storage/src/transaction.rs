//! Transaction management with retry logic

use async_trait::async_trait;
use std::time::Duration;
use tokio::time::sleep;

use crate::{
    connection::{Connection, Transaction as ConnTransaction},
    StorageResult, StorageError,
};

/// Transaction manager with retry logic and error handling
pub struct TransactionManager {
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
    jitter: bool,
}

/// Transaction wrapper
pub struct Transaction {
    inner: Box<dyn ConnTransaction>,
    committed: bool,
    rolled_back: bool,
}

/// Transaction callback trait
#[async_trait]
pub trait TransactionCallback<T>: Send + Sync {
    async fn execute(&self, tx: &mut Transaction) -> StorageResult<T>;
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            jitter: true,
        }
    }
    
    /// Set maximum number of retries
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
    
    /// Set base retry delay
    pub fn with_base_delay(mut self, delay: Duration) -> Self {
        self.base_delay = delay;
        self
    }
    
    /// Set maximum retry delay
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }
    
    /// Enable or disable jitter in retry delays
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }
    
    /// Execute a transaction with retry logic
    pub async fn execute<T, F>(&self, connection: &dyn Connection, callback: F) -> StorageResult<T>
    where
        F: Fn() -> Box<dyn TransactionCallback<T>> + Send + Sync,
        T: Send + 'static,
    {
        let mut last_error = None;
        
        for attempt in 0..=self.max_retries {
            match self.execute_once(connection, callback()).await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    last_error = Some(error.clone());
                    
                    // Don't retry on final attempt
                    if attempt == self.max_retries {
                        break;
                    }
                    
                    // Check if error is retryable
                    if !self.is_retryable_error(&error) {
                        return Err(error);
                    }
                    
                    // Calculate retry delay
                    let delay = self.calculate_delay(attempt);
                    
                    log::warn!(
                        "Transaction failed (attempt {}/{}), retrying in {:?}: {}",
                        attempt + 1,
                        self.max_retries + 1,
                        delay,
                        error
                    );
                    
                    sleep(delay).await;
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| {
            StorageError::TransactionFailed("Unknown transaction error".to_string())
        }))
    }
    
    /// Execute transaction once
    async fn execute_once<T>(
        &self,
        connection: &dyn Connection,
        callback: Box<dyn TransactionCallback<T>>,
    ) -> StorageResult<T> {
        let tx = connection.begin_transaction().await?;
        let mut transaction = Transaction {
            inner: tx,
            committed: false,
            rolled_back: false,
        };
        
        match callback.execute(&mut transaction).await {
            Ok(result) => {
                if !transaction.committed && !transaction.rolled_back {
                    transaction.commit().await?;
                }
                Ok(result)
            }
            Err(error) => {
                if !transaction.committed && !transaction.rolled_back {
                    let _ = transaction.rollback().await; // Ignore rollback errors
                }
                Err(error)
            }
        }
    }
    
    /// Check if an error is retryable
    fn is_retryable_error(&self, error: &StorageError) -> bool {
        match error {
            StorageError::TransactionFailed(msg) => {
                // Retry on deadlock, serialization failure, or connection issues
                let msg_lower = msg.to_lowercase();
                msg_lower.contains("deadlock") ||
                msg_lower.contains("serialization") ||
                msg_lower.contains("connection") ||
                msg_lower.contains("timeout")
            }
            StorageError::ConnectionFailed(_) => true,
            StorageError::ConcurrencyError(_) => true,
            _ => false,
        }
    }
    
    /// Calculate retry delay with exponential backoff and jitter
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self.base_delay.as_millis() as u64 * 2_u64.pow(attempt);
        let delay = Duration::from_millis(delay_ms).min(self.max_delay);
        
        if self.jitter {
            // Add random jitter ±25%
            let jitter_range = delay.as_millis() / 4;
            let jitter = fastrand::u64(0..=jitter_range as u64 * 2) as i64 - jitter_range as i64;
            let jittered_ms = (delay.as_millis() as i64 + jitter).max(0) as u64;
            Duration::from_millis(jittered_ms)
        } else {
            delay
        }
    }
}

impl Transaction {
    /// Execute a query within the transaction
    pub async fn execute(&mut self, query: &str, params: &[serde_json::Value]) -> StorageResult<u64> {
        self.check_state()?;
        self.inner.execute(query, params).await
    }
    
    /// Fetch JSON rows within the transaction
    pub async fn fetch_json(&mut self, query: &str, params: &[serde_json::Value]) -> StorageResult<Vec<serde_json::Value>> {
        self.check_state()?;
        self.inner.fetch_json(query, params).await
    }
    
    /// Fetch one JSON row within the transaction
    pub async fn fetch_one_json(&mut self, query: &str, params: &[serde_json::Value]) -> StorageResult<serde_json::Value> {
        self.check_state()?;
        self.inner.fetch_one_json(query, params).await
    }
    
    /// Fetch optional JSON row within the transaction
    pub async fn fetch_optional_json(&mut self, query: &str, params: &[serde_json::Value]) -> StorageResult<Option<serde_json::Value>> {
        self.check_state()?;
        self.inner.fetch_optional_json(query, params).await
    }
    
    /// Commit the transaction
    pub async fn commit(&mut self) -> StorageResult<()> {
        self.check_state()?;
        
        // Take ownership to avoid double-commit
        let tx = std::mem::replace(&mut self.inner, Box::new(DummyTransaction));
        tx.commit().await?;
        self.committed = true;
        Ok(())
    }
    
    /// Rollback the transaction
    pub async fn rollback(&mut self) -> StorageResult<()> {
        self.check_state()?;
        
        // Take ownership to avoid double-rollback
        let tx = std::mem::replace(&mut self.inner, Box::new(DummyTransaction));
        tx.rollback().await?;
        self.rolled_back = true;
        Ok(())
    }
    
    /// Check if transaction is in valid state
    fn check_state(&self) -> StorageResult<()> {
        if self.committed {
            return Err(StorageError::TransactionFailed(
                "Transaction already committed".to_string()
            ));
        }
        if self.rolled_back {
            return Err(StorageError::TransactionFailed(
                "Transaction already rolled back".to_string()
            ));
        }
        Ok(())
    }
}

/// Dummy transaction for replaced transactions
struct DummyTransaction;

#[async_trait]
impl ConnTransaction for DummyTransaction {
    async fn execute(&mut self, _query: &str, _params: &[serde_json::Value]) -> StorageResult<u64> {
        Err(StorageError::TransactionFailed("Transaction already finalized".to_string()))
    }
    
    async fn fetch_json(&mut self, _query: &str, _params: &[serde_json::Value]) -> StorageResult<Vec<serde_json::Value>> {
        Err(StorageError::TransactionFailed("Transaction already finalized".to_string()))
    }
    
    async fn fetch_one_json(&mut self, _query: &str, _params: &[serde_json::Value]) -> StorageResult<serde_json::Value> {
        Err(StorageError::TransactionFailed("Transaction already finalized".to_string()))
    }
    
    async fn fetch_optional_json(&mut self, _query: &str, _params: &[serde_json::Value]) -> StorageResult<Option<serde_json::Value>> {
        Err(StorageError::TransactionFailed("Transaction already finalized".to_string()))
    }
    
    async fn commit(self: Box<Self>) -> StorageResult<()> {
        Err(StorageError::TransactionFailed("Transaction already finalized".to_string()))
    }
    
    async fn rollback(self: Box<Self>) -> StorageResult<()> {
        Err(StorageError::TransactionFailed("Transaction already finalized".to_string()))
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{InMemoryConnectionManager, ConnectionManager};
    use std::sync::Arc;
    
    struct TestCallback;
    
    #[async_trait]
    impl TransactionCallback<String> for TestCallback {
        async fn execute(&self, _tx: &mut Transaction) -> StorageResult<String> {
            Ok("test result".to_string())
        }
    }
    
    #[tokio::test]
    async fn test_transaction_manager() {
        let manager = TransactionManager::new()
            .with_max_retries(2)
            .with_base_delay(Duration::from_millis(10));
        
        let conn_manager = Arc::new(InMemoryConnectionManager::new());
        let connection = conn_manager.get_connection().await.unwrap();
        
        let result = manager.execute(
            connection.as_ref(),
            || Box::new(TestCallback)
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test result");
    }
    
    #[test]
    fn test_delay_calculation() {
        let manager = TransactionManager::new()
            .with_base_delay(Duration::from_millis(100))
            .with_jitter(false);
        
        assert_eq!(manager.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(manager.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(manager.calculate_delay(2), Duration::from_millis(400));
    }
}
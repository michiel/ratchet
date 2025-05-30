use sea_orm::{Database, DatabaseConnection, DbErr};
use tempfile::TempDir;
use std::sync::Arc;

use crate::database::{DatabaseConnection as RatchetDatabaseConnection, SafeDatabaseError};

/// Test database for isolated testing
pub struct TestDatabase {
    _temp_dir: TempDir,
    pub connection: DatabaseConnection,
    pub ratchet_connection: RatchetDatabaseConnection,
}

impl TestDatabase {
    /// Create a new test database with an in-memory SQLite database
    pub async fn new() -> Result<Self, TestDatabaseError> {
        Self::new_sqlite().await
    }

    /// Create a new SQLite test database
    pub async fn new_sqlite() -> Result<Self, TestDatabaseError> {
        let temp_dir = TempDir::new()
            .map_err(|e| TestDatabaseError::TempDirCreation(e.to_string()))?;

        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let connection = Database::connect(&db_url)
            .await
            .map_err(|e| TestDatabaseError::Connection(e.to_string()))?;

        // Run migrations
        crate::database::migrations::Migrator::up(&connection, None)
            .await
            .map_err(|e| TestDatabaseError::Migration(e.to_string()))?;

        // Create Ratchet database connection wrapper
        let config = crate::config::DatabaseConfig {
            url: db_url,
            max_connections: 1,
            connection_timeout: std::time::Duration::from_secs(5),
        };

        let ratchet_connection = RatchetDatabaseConnection::new(config)
            .await
            .map_err(|e| TestDatabaseError::RatchetConnection(e.to_string()))?;

        Ok(Self {
            _temp_dir: temp_dir,
            connection,
            ratchet_connection,
        })
    }

    /// Create an in-memory SQLite database (faster for tests)
    pub async fn new_in_memory() -> Result<Self, TestDatabaseError> {
        let db_url = "sqlite::memory:";

        let connection = Database::connect(db_url)
            .await
            .map_err(|e| TestDatabaseError::Connection(e.to_string()))?;

        // Run migrations
        crate::database::migrations::Migrator::up(&connection, None)
            .await
            .map_err(|e| TestDatabaseError::Migration(e.to_string()))?;

        // Create Ratchet database connection wrapper
        let config = crate::config::DatabaseConfig {
            url: db_url.to_string(),
            max_connections: 1,
            connection_timeout: std::time::Duration::from_secs(5),
        };

        let ratchet_connection = RatchetDatabaseConnection::new(config)
            .await
            .map_err(|e| TestDatabaseError::RatchetConnection(e.to_string()))?;

        // For in-memory database, we don't need a temp directory
        let temp_dir = TempDir::new()
            .map_err(|e| TestDatabaseError::TempDirCreation(e.to_string()))?;

        Ok(Self {
            _temp_dir: temp_dir,
            connection,
            ratchet_connection,
        })
    }

    /// Seed the database with test data
    pub async fn seed_tasks(&self, tasks: Vec<crate::database::entities::tasks::Model>) -> Result<(), TestDatabaseError> {
        use crate::database::entities::tasks;
        use sea_orm::ActiveModelTrait;

        for task in tasks {
            let active_model: tasks::ActiveModel = task.into();
            active_model
                .insert(&self.connection)
                .await
                .map_err(|e| TestDatabaseError::Seeding(e.to_string()))?;
        }

        Ok(())
    }

    /// Seed the database with test executions
    pub async fn seed_executions(&self, executions: Vec<crate::database::entities::executions::Model>) -> Result<(), TestDatabaseError> {
        use crate::database::entities::executions;
        use sea_orm::ActiveModelTrait;

        for execution in executions {
            let active_model: executions::ActiveModel = execution.into();
            active_model
                .insert(&self.connection)
                .await
                .map_err(|e| TestDatabaseError::Seeding(e.to_string()))?;
        }

        Ok(())
    }

    /// Seed the database with test jobs
    pub async fn seed_jobs(&self, jobs: Vec<crate::database::entities::jobs::Model>) -> Result<(), TestDatabaseError> {
        use crate::database::entities::jobs;
        use sea_orm::ActiveModelTrait;

        for job in jobs {
            let active_model: jobs::ActiveModel = job.into();
            active_model
                .insert(&self.connection)
                .await
                .map_err(|e| TestDatabaseError::Seeding(e.to_string()))?;
        }

        Ok(())
    }

    /// Seed the database with test schedules
    pub async fn seed_schedules(&self, schedules: Vec<crate::database::entities::schedules::Model>) -> Result<(), TestDatabaseError> {
        use crate::database::entities::schedules;
        use sea_orm::ActiveModelTrait;

        for schedule in schedules {
            let active_model: schedules::ActiveModel = schedule.into();
            active_model
                .insert(&self.connection)
                .await
                .map_err(|e| TestDatabaseError::Seeding(e.to_string()))?;
        }

        Ok(())
    }

    /// Clear all data from the database
    pub async fn clear_all(&self) -> Result<(), TestDatabaseError> {
        use sea_orm::Statement;

        // Disable foreign key constraints temporarily
        self.connection
            .execute(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                "PRAGMA foreign_keys = OFF;".to_string(),
            ))
            .await
            .map_err(|e| TestDatabaseError::Clearing(e.to_string()))?;

        // Clear all tables in reverse dependency order
        let tables = ["executions", "jobs", "schedules", "tasks"];
        for table in &tables {
            self.connection
                .execute(Statement::from_string(
                    sea_orm::DatabaseBackend::Sqlite,
                    format!("DELETE FROM {};", table),
                ))
                .await
                .map_err(|e| TestDatabaseError::Clearing(e.to_string()))?;
        }

        // Re-enable foreign key constraints
        self.connection
            .execute(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                "PRAGMA foreign_keys = ON;".to_string(),
            ))
            .await
            .map_err(|e| TestDatabaseError::Clearing(e.to_string()))?;

        Ok(())
    }

    /// Get a count of records in a table
    pub async fn count_records(&self, table: &str) -> Result<u64, TestDatabaseError> {
        use sea_orm::{Statement, Value};

        let result = self
            .connection
            .query_one(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                format!("SELECT COUNT(*) as count FROM {};", table),
            ))
            .await
            .map_err(|e| TestDatabaseError::Query(e.to_string()))?;

        match result {
            Some(row) => {
                let count: i64 = row.try_get("", "count")
                    .map_err(|e| TestDatabaseError::Query(e.to_string()))?;
                Ok(count as u64)
            }
            None => Ok(0),
        }
    }
}

/// Test database errors
#[derive(Debug, thiserror::Error)]
pub enum TestDatabaseError {
    #[error("Failed to create temporary directory: {0}")]
    TempDirCreation(String),

    #[error("Database connection failed: {0}")]
    Connection(String),

    #[error("Ratchet database connection failed: {0}")]
    RatchetConnection(String),

    #[error("Migration failed: {0}")]
    Migration(String),

    #[error("Seeding failed: {0}")]
    Seeding(String),

    #[error("Clearing failed: {0}")]
    Clearing(String),

    #[error("Query failed: {0}")]
    Query(String),
}

/// Shared test database for reuse across tests
pub struct SharedTestDatabase {
    inner: Arc<TestDatabase>,
}

impl SharedTestDatabase {
    pub async fn new() -> Result<Self, TestDatabaseError> {
        let db = TestDatabase::new_in_memory().await?;
        Ok(Self {
            inner: Arc::new(db),
        })
    }

    pub fn connection(&self) -> &DatabaseConnection {
        &self.inner.connection
    }

    pub fn ratchet_connection(&self) -> &RatchetDatabaseConnection {
        &self.inner.ratchet_connection
    }

    pub async fn clear_all(&self) -> Result<(), TestDatabaseError> {
        self.inner.clear_all().await
    }
}

impl Clone for SharedTestDatabase {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// Convenience macro for creating tests with database
#[macro_export]
macro_rules! test_with_db {
    ($test_name:ident, $test_body:expr) => {
        #[tokio::test]
        async fn $test_name() {
            let db = $crate::testing::TestDatabase::new()
                .await
                .expect("Failed to create test database");
            
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tokio::runtime::Handle::current().block_on(async {
                    $test_body(&db).await
                })
            }));
            
            // Database cleanup happens automatically when db is dropped
            
            if let Err(e) = result {
                std::panic::resume_unwind(e);
            }
        }
    };
}

/// Convenience macro for creating tests with seeded database
#[macro_export]
macro_rules! test_with_seeded_db {
    ($test_name:ident, $seed_fn:expr, $test_body:expr) => {
        #[tokio::test]
        async fn $test_name() {
            let db = $crate::testing::TestDatabase::new()
                .await
                .expect("Failed to create test database");
            
            // Seed the database
            $seed_fn(&db).await.expect("Failed to seed database");
            
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tokio::runtime::Handle::current().block_on(async {
                    $test_body(&db).await
                })
            }));
            
            // Database cleanup happens automatically when db is dropped
            
            if let Err(e) = result {
                std::panic::resume_unwind(e);
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::builders::*;

    #[tokio::test]
    async fn test_database_creation() {
        let db = TestDatabase::new().await.unwrap();
        
        // Verify migrations ran
        let task_count = db.count_records("tasks").await.unwrap();
        assert_eq!(task_count, 0);
    }

    #[tokio::test]
    async fn test_database_seeding() {
        let db = TestDatabase::new().await.unwrap();
        
        let tasks = vec![
            TaskBuilder::new().with_name("task1").build(),
            TaskBuilder::new().with_name("task2").build(),
        ];
        
        db.seed_tasks(tasks).await.unwrap();
        
        let task_count = db.count_records("tasks").await.unwrap();
        assert_eq!(task_count, 2);
    }

    #[tokio::test]
    async fn test_database_clearing() {
        let db = TestDatabase::new().await.unwrap();
        
        // Seed some data
        let tasks = vec![TaskBuilder::new().build()];
        db.seed_tasks(tasks).await.unwrap();
        
        // Verify data exists
        let task_count = db.count_records("tasks").await.unwrap();
        assert_eq!(task_count, 1);
        
        // Clear data
        db.clear_all().await.unwrap();
        
        // Verify data is gone
        let task_count = db.count_records("tasks").await.unwrap();
        assert_eq!(task_count, 0);
    }

    test_with_db!(test_macro_usage, |db: &TestDatabase| async {
        let tasks = vec![TaskBuilder::new().with_name("macro-test").build()];
        db.seed_tasks(tasks).await.unwrap();
        
        let count = db.count_records("tasks").await.unwrap();
        assert_eq!(count, 1);
    });

    test_with_seeded_db!(
        test_seeded_macro_usage,
        |db: &TestDatabase| async {
            let tasks = vec![
                TaskBuilder::new().with_name("seeded1").build(),
                TaskBuilder::new().with_name("seeded2").build(),
            ];
            db.seed_tasks(tasks)
        },
        |db: &TestDatabase| async {
            let count = db.count_records("tasks").await.unwrap();
            assert_eq!(count, 2);
        }
    );
}
use ratchet_config::migration::ConfigMigrator;
use tempfile::TempDir;
use std::fs::File;
use std::io::Write;

#[tokio::main]
async fn main() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.yaml");

    // Create a legacy config file
    let legacy_config = r#"
server:
  bind_address: "127.0.0.1"
  port: 3000
  database:
    url: "sqlite://test.db"
    max_connections: 5
max_execution_duration: 600
validate_schemas: true
max_concurrent_tasks: 8
timeout_grace_period: 60
"#;

    let mut file = File::create(&config_path).unwrap();
    file.write_all(legacy_config.as_bytes()).unwrap();

    // Test migration
    let migrator = ConfigMigrator::new();
    match migrator.migrate_config_file(&config_path).await {
        Ok((config, report)) => {
            println\!("Migration successful\!");
            println\!("Report: {:?}", report);
        }
        Err(e) => {
            println\!("Migration failed: {:?}", e);
        }
    }
}
EOF < /dev/null

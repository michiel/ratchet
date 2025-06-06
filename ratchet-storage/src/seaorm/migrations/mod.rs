use sea_orm_migration::prelude::*;

mod m20241201_000001_create_tasks_table;
mod m20241201_000002_create_executions_table;
mod m20241201_000003_create_schedules_table;
mod m20241201_000004_create_jobs_table;
mod m20241201_000005_create_indexes;
mod m20250106_000001_add_output_destinations;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20241201_000001_create_tasks_table::Migration),
            Box::new(m20241201_000002_create_executions_table::Migration),
            Box::new(m20241201_000003_create_schedules_table::Migration),
            Box::new(m20241201_000004_create_jobs_table::Migration),
            Box::new(m20241201_000005_create_indexes::Migration),
            Box::new(m20250106_000001_add_output_destinations::Migration),
        ]
    }
}

//! SeaORM adapter for Casbin policy storage

use casbin::{error::AdapterError, Adapter, Filter, Model, Result as CasbinResult};
use sea_orm::{
    entity::prelude::*, query::*, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
};
use std::collections::HashMap;

use crate::error::{RbacError, RbacResult};

/// SeaORM adapter for Casbin policies
#[derive(Clone)]
pub struct SeaOrmAdapter {
    db: DatabaseConnection,
}

impl SeaOrmAdapter {
    /// Create a new SeaORM adapter
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Load all policies from database
    async fn load_policies(&self) -> RbacResult<Vec<Vec<String>>> {
        use ratchet_storage::seaorm::entities::{CasbinRules, casbin_rules};

        let rules = CasbinRules::find().all(&self.db).await?;

        let policies = rules
            .into_iter()
            .map(|rule| {
                let mut policy = vec![rule.ptype];
                if let Some(v0) = rule.v0 {
                    policy.push(v0);
                }
                if let Some(v1) = rule.v1 {
                    policy.push(v1);
                }
                if let Some(v2) = rule.v2 {
                    policy.push(v2);
                }
                if let Some(v3) = rule.v3 {
                    policy.push(v3);
                }
                if let Some(v4) = rule.v4 {
                    policy.push(v4);
                }
                if let Some(v5) = rule.v5 {
                    policy.push(v5);
                }
                policy
            })
            .collect();

        Ok(policies)
    }

    /// Save policy to database
    async fn save_policy_internal(&self, policy: &[String]) -> RbacResult<()> {
        use ratchet_storage::seaorm::entities::casbin_rules;

        let rule = casbin_rules::ActiveModel {
            ptype: Set(policy.get(0).cloned().unwrap_or_default()),
            v0: Set(policy.get(1).cloned()),
            v1: Set(policy.get(2).cloned()),
            v2: Set(policy.get(3).cloned()),
            v3: Set(policy.get(4).cloned()),
            v4: Set(policy.get(5).cloned()),
            v5: Set(policy.get(6).cloned()),
            ..Default::default()
        };

        rule.insert(&self.db).await?;
        Ok(())
    }

    /// Remove policy from database
    async fn remove_policy_internal(&self, policy: &[String]) -> RbacResult<bool> {
        use ratchet_storage::seaorm::entities::{CasbinRules, casbin_rules};

        let mut query = CasbinRules::delete_many().filter(
            casbin_rules::Column::Ptype.eq(policy.get(0).cloned().unwrap_or_default()),
        );

        if let Some(v0) = policy.get(1) {
            query = query.filter(casbin_rules::Column::V0.eq(v0.clone()));
        }
        if let Some(v1) = policy.get(2) {
            query = query.filter(casbin_rules::Column::V1.eq(v1.clone()));
        }
        if let Some(v2) = policy.get(3) {
            query = query.filter(casbin_rules::Column::V2.eq(v2.clone()));
        }
        if let Some(v3) = policy.get(4) {
            query = query.filter(casbin_rules::Column::V3.eq(v3.clone()));
        }
        if let Some(v4) = policy.get(5) {
            query = query.filter(casbin_rules::Column::V4.eq(v4.clone()));
        }
        if let Some(v5) = policy.get(6) {
            query = query.filter(casbin_rules::Column::V5.eq(v5.clone()));
        }

        let result = query.exec(&self.db).await?;
        Ok(result.rows_affected > 0)
    }

    /// Remove policies matching filter
    async fn remove_filtered_policy_internal(
        &self,
        field_index: usize,
        field_values: Vec<String>,
    ) -> RbacResult<bool> {
        use ratchet_storage::seaorm::entities::{CasbinRules, casbin_rules};

        if field_values.is_empty() {
            return Ok(false);
        }

        let mut query = CasbinRules::delete_many();

        // Apply filters based on field index
        for (i, value) in field_values.iter().enumerate() {
            match field_index + i {
                0 => query = query.filter(casbin_rules::Column::Ptype.eq(value.clone())),
                1 => query = query.filter(casbin_rules::Column::V0.eq(value.clone())),
                2 => query = query.filter(casbin_rules::Column::V1.eq(value.clone())),
                3 => query = query.filter(casbin_rules::Column::V2.eq(value.clone())),
                4 => query = query.filter(casbin_rules::Column::V3.eq(value.clone())),
                5 => query = query.filter(casbin_rules::Column::V4.eq(value.clone())),
                6 => query = query.filter(casbin_rules::Column::V5.eq(value.clone())),
                _ => break, // Ignore invalid field indices
            }
        }

        let result = query.exec(&self.db).await?;
        Ok(result.rows_affected > 0)
    }
}

#[async_trait::async_trait]
impl Adapter for SeaOrmAdapter {
    async fn load_policy(&mut self, model: &mut dyn Model) -> CasbinResult<()> {
        let policies = self
            .load_policies()
            .await
            .map_err(|e| AdapterError(Box::new(e)))?;

        for policy in policies {
            if policy.is_empty() {
                continue;
            }

            let sec = &policy[0];
            let ptype = sec.chars().next().unwrap_or('p');

            if let Some(ast_map) = model.get_mut_model().get_mut(&ptype.to_string()) {
                if let Some(ast) = ast_map.get_mut(sec) {
                    ast.get_mut_policy().insert(policy[1..].to_vec());
                }
            }
        }

        Ok(())
    }

    async fn save_policy(&mut self, model: &mut dyn Model) -> CasbinResult<()> {
        // Clear existing policies first
        use ratchet_storage::seaorm::entities::CasbinRules;
        CasbinRules::delete_many()
            .exec(&self.db)
            .await
            .map_err(|e| AdapterError(Box::new(e)))?;

        // Save all policies from model
        let mut lines = Vec::new();

        if let Some(ast_map) = model.get_model().get("p") {
            for (ptype, ast) in ast_map {
                for policy in ast.get_policy() {
                    let mut line = vec![ptype.clone()];
                    line.extend_from_slice(policy);
                    lines.push(line);
                }
            }
        }

        if let Some(ast_map) = model.get_model().get("g") {
            for (ptype, ast) in ast_map {
                for policy in ast.get_policy() {
                    let mut line = vec![ptype.clone()];
                    line.extend_from_slice(policy);
                    lines.push(line);
                }
            }
        }

        for line in lines {
            self.save_policy_internal(&line)
                .await
                .map_err(|e| AdapterError(Box::new(e)))?;
        }

        Ok(())
    }

    async fn add_policy(&mut self, _sec: &str, _ptype: &str, rule: Vec<String>) -> CasbinResult<()> {
        self.save_policy_internal(&rule)
            .await
            .map_err(|e| casbin::Error::AdapterError(casbin::error::AdapterError(Box::new(e))))
    }

    async fn remove_policy(
        &mut self,
        _sec: &str,
        _ptype: &str,
        rule: Vec<String>,
    ) -> CasbinResult<()> {
        self.remove_policy_internal(&rule)
            .await
            .map_err(|e| AdapterError(Box::new(e)))?;
        Ok(())
    }

    async fn remove_filtered_policy(
        &mut self,
        _sec: &str,
        _ptype: &str,
        field_index: usize,
        field_values: Vec<String>,
    ) -> CasbinResult<()> {
        self.remove_filtered_policy_internal(field_index, field_values)
            .await
            .map_err(|e| AdapterError(Box::new(e)))?;
        Ok(())
    }

    async fn load_filtered_policy(
        &mut self,
        model: &mut dyn Model,
        _filter: Filter<'_>,
    ) -> CasbinResult<()> {
        // For now, just load all policies - filtered loading can be implemented later
        self.load_policy(model).await
    }

    fn is_filtered(&self) -> bool {
        false
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, MockDatabase, MockExecResult, Transaction};

    #[tokio::test]
    async fn test_adapter_creation() {
        let db = MockDatabase::new(sea_orm::DatabaseBackend::Postgres)
            .into_connection();
        
        let adapter = SeaOrmAdapter::new(db);
        assert!(!adapter.is_filtered());
    }

    #[tokio::test] 
    async fn test_save_policy_internal() {
        let db = MockDatabase::new(sea_orm::DatabaseBackend::Postgres)
            .append_exec_results([MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let adapter = SeaOrmAdapter::new(db);
        let policy = vec![
            "p".to_string(),
            "alice".to_string(),
            "data1".to_string(),
            "read".to_string(),
            "tenant_1".to_string(),
        ];

        let result = adapter.save_policy_internal(&policy).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_policy_internal() {
        let db = MockDatabase::new(sea_orm::DatabaseBackend::Postgres)
            .append_exec_results([MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .into_connection();

        let adapter = SeaOrmAdapter::new(db);
        let policy = vec![
            "p".to_string(),
            "alice".to_string(),
            "data1".to_string(),
            "read".to_string(),
        ];

        let result = adapter.remove_policy_internal(&policy).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
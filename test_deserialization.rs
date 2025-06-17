use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    #[serde(rename = "_start")]
    pub start: Option<u64>,
    #[serde(rename = "_end")]
    pub end: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortQuery {
    #[serde(rename = "_sort")]
    pub sort: Option<String>,
    #[serde(rename = "_order")]
    pub order: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterQuery {
    #[serde(flatten)]
    pub filters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListQuery {
    #[serde(flatten)]
    pub pagination: PaginationQuery,
    #[serde(flatten)]
    pub sort: SortQuery,
    #[serde(flatten)]
    pub filter: FilterQuery,
}

fn main() {
    // Test the exact query string that's failing
    let query_string = "_end=10&_start=0";
    
    println!("Testing query string: {}", query_string);
    
    // Try to deserialize using serde_urlencoded
    match serde_urlencoded::from_str::<ListQuery>(query_string) {
        Ok(query) => {
            println!("✅ Successfully deserialized: {:#?}", query);
        }
        Err(e) => {
            println!("❌ Failed to deserialize: {}", e);
        }
    }
    
    // Test with additional query parameters
    let query_string2 = "_end=10&_start=0&name=test&enabled=true";
    
    println!("\nTesting extended query string: {}", query_string2);
    
    match serde_urlencoded::from_str::<ListQuery>(query_string2) {
        Ok(query) => {
            println!("✅ Successfully deserialized: {:#?}", query);
        }
        Err(e) => {
            println!("❌ Failed to deserialize: {}", e);
        }
    }
}
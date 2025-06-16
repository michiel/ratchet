// Quick test to verify the query parameter fix works
use serde::Deserialize;
use ratchet_web::extractors::query::{ListQuery, PaginationQuery, SortQuery, FilterQuery};

#[derive(Debug, Deserialize)]
struct TestQuery {
    #[serde(flatten)]
    pub list: ListQuery,
}

fn main() {
    // Test the original failing case
    let query_string = "_end=100&_start=0";
    
    // Try to deserialize using serde_urlencoded
    match serde_urlencoded::from_str::<ListQuery>(query_string) {
        Ok(query) => {
            println!("✅ SUCCESS: Query deserialization works!");
            println!("Pagination: start={:?}, end={:?}", query.pagination.start, query.pagination.end);
            println!("Full query: {:?}", query);
            
            // Test conversion to list input
            let list_input = query.to_list_input();
            println!("List input: {:?}", list_input);
            
            // Test validation
            match query.validate() {
                Ok(()) => println!("✅ Validation passed"),
                Err(e) => println!("❌ Validation failed: {:?}", e),
            }
        }
        Err(e) => {
            println!("❌ FAILED: Query deserialization failed: {}", e);
        }
    }
    
    // Test with additional parameters
    let complex_query = "_end=100&_start=0&_sort=name&_order=ASC&filter_name=test";
    match serde_urlencoded::from_str::<ListQuery>(complex_query) {
        Ok(query) => {
            println!("\n✅ SUCCESS: Complex query deserialization works!");
            println!("Complex query: {:?}", query);
        }
        Err(e) => {
            println!("\n❌ FAILED: Complex query deserialization failed: {}", e);
        }
    }
}
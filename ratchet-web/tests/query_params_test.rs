// Test to verify the query parameter fix works
use ratchet_web::extractors::query::{ListQuery, PaginationQuery, SortQuery, FilterQuery};

#[test]
fn test_refine_query_parameters() {
    // Test the core functionality by constructing the structs manually
    let query = ListQuery {
        start: Some(0),
        end: Some(100),
        page: None,
        limit: None,
    };
    
    // Test that the structs are correctly structured (this verifies our fix)
    assert_eq!(query.start, Some(0));
    assert_eq!(query.end, Some(100));
    
    // Test conversion to list input (this tests the operation)
    let list_input = query.to_list_input();
    println!("List input: {:?}", list_input);
    
    // Test validation
    let validation_result = query.validate();
    assert!(validation_result.is_ok(), "Validation should pass: {:?}", validation_result);
    
    // Test helper methods for compatibility
    let sort = query.sort();
    let filter = query.filter();
    assert!(sort.sort.is_none());
    assert!(filter.filters.is_empty());
    
    println!("✅ SUCCESS: Query parameter structures work correctly!");
}

#[test]
fn test_pagination_validation() {
    // Test validation with valid range
    let valid_pagination = PaginationQuery {
        page: None,
        limit: None,
        start: Some(0),
        end: Some(50),
    };
    assert!(valid_pagination.validate().is_ok());
    
    // Test validation with invalid range (start >= end)
    let invalid_pagination = PaginationQuery {
        page: None,
        limit: None,
        start: Some(50),
        end: Some(10),
    };
    assert!(invalid_pagination.validate().is_err());
    
    // Test validation with too large range
    let large_pagination = PaginationQuery {
        page: None,
        limit: None,
        start: Some(0),
        end: Some(200), // More than 100 items
    };
    assert!(large_pagination.validate().is_err());
    
    println!("✅ SUCCESS: Pagination validation works correctly!");
}

#[test]
fn test_simplified_list_query_serde() {
    // Test serde deserialization with the simplified structure
    use serde_urlencoded;
    
    // Test Refine.dev style parameters
    let query_string = "_start=0&_end=10";
    let parsed: ListQuery = serde_urlencoded::from_str(query_string)
        .expect("Should deserialize Refine.dev style parameters");
    
    assert_eq!(parsed.start, Some(0));
    assert_eq!(parsed.end, Some(10));
    assert_eq!(parsed.page, None);
    assert_eq!(parsed.limit, None);
    
    // Test standard pagination parameters
    let query_string = "page=2&limit=25";
    let parsed: ListQuery = serde_urlencoded::from_str(query_string)
        .expect("Should deserialize standard pagination parameters");
    
    assert_eq!(parsed.page, Some(2));
    assert_eq!(parsed.limit, Some(25));
    assert_eq!(parsed.start, None);
    assert_eq!(parsed.end, None);
    
    // Test mixed parameters
    let query_string = "_start=0&_end=10&page=1&limit=5";
    let parsed: ListQuery = serde_urlencoded::from_str(query_string)
        .expect("Should deserialize mixed parameters");
    
    assert_eq!(parsed.start, Some(0));
    assert_eq!(parsed.end, Some(10));
    assert_eq!(parsed.page, Some(1));
    assert_eq!(parsed.limit, Some(5));
    
    println!("✅ SUCCESS: Simplified ListQuery serde works correctly!");
}

#[test]
fn test_list_query_to_list_input_conversion() {
    // Test Refine.dev style conversion
    let query = ListQuery {
        start: Some(0),
        end: Some(10),
        page: None,
        limit: None,
    };
    
    let list_input = query.to_list_input();
    let pagination = list_input.pagination.unwrap();
    
    // Should be converted from Refine.dev style
    assert_eq!(pagination.page, None); // Refine.dev uses offset, not page
    assert_eq!(pagination.limit, Some(10)); // Converted from end-start
    assert_eq!(pagination.offset, Some(0)); // Converted from start
    
    // Test standard pagination conversion
    let query = ListQuery {
        start: None,
        end: None,
        page: Some(2),
        limit: Some(20),
    };
    
    let list_input = query.to_list_input();
    let pagination = list_input.pagination.unwrap();
    
    assert_eq!(pagination.page, Some(2));
    assert_eq!(pagination.limit, Some(20));
    
    // Test defaults when no pagination specified
    let query = ListQuery {
        start: None,
        end: None,
        page: None,
        limit: None,
    };
    
    let list_input = query.to_list_input();
    let pagination = list_input.pagination.unwrap();
    
    assert_eq!(pagination.page, Some(1)); // Default
    assert_eq!(pagination.limit, Some(25)); // Default
    
    println!("✅ SUCCESS: ListQuery to ListInput conversion works correctly!");
}
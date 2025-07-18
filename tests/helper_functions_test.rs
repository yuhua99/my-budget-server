/*!
 * Helper Functions Unit Tests
 *
 * This module contains unit tests for utility functions in the records module,
 * particularly focusing on data extraction and validation functions.
 *
 * Test Categories:
 * - extract_record_from_row function tests (data type handling, edge cases)
 * - Default behavior validation (time ranges, limits)
 * - Database precision and consistency tests
 *
 * All tests use isolated temporary databases for complete test isolation.
 */

mod common;

use common::*;
use my_budget_server::database::get_user_db;
use my_budget_server::records::extract_record_from_row;

// Test data constants - only for widely reused values
const TEST_BASE_TIMESTAMP: i64 = 1700000000; // Nov 14, 2023 22:13:20 UTC

/// Tests the core extract_record_from_row function with standard data types.
/// Verifies that the function correctly extracts all fields from a database row
/// and constructs a proper Record struct.
#[tokio::test]
async fn extract_record_from_row_success() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create a test record
    let timestamp = TEST_BASE_TIMESTAMP;
    let record_id = create_test_record(
        &data_path,
        &user_id,
        "Test Record",
        25.50,
        "test_category",
        timestamp,
    )
    .await;

    // Query the record back to test extraction
    let user_db = get_user_db(&data_path, &user_id)
        .await
        .expect("Failed to get user database for test");
    let conn = user_db.read().await;
    let mut rows = conn
        .query(
            "SELECT id, name, amount, category_id, timestamp FROM records WHERE id = ?",
            [record_id.as_str()],
        )
        .await
        .expect("Failed to execute database query in test");

    if let Some(row) = rows.next().await.expect("Failed to read row from database") {
        // Test the actual extract_record_from_row function
        let record = extract_record_from_row(row).expect("Failed to extract record from row");

        assert_eq!(record.id, record_id);
        assert_eq!(record.name, "Test Record");
        assert_eq!(record.amount, 25.50);
        assert_eq!(record.category_id, "test_category");
        assert_eq!(record.timestamp, timestamp);
    } else {
        panic!(
            "Expected to find test record with ID {}, but query returned no results",
            record_id
        );
    }
}

/// Tests extract_record_from_row with Unicode characters and special symbols.
/// Ensures proper handling of emoji, accented characters, and symbols in names and categories.
#[tokio::test]
async fn extract_record_with_special_characters() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create a record with special characters and unicode
    let special_name = "Test Record with émojis 🎉 and symbols @#$%";
    let special_category = "special-category_123";
    let record_id = create_test_record(
        &data_path,
        &user_id,
        special_name,
        99.99,
        special_category,
        TEST_BASE_TIMESTAMP,
    )
    .await;

    // Verify the record was stored and can be extracted correctly
    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.read().await;
    let mut rows = conn
        .query(
            "SELECT id, name, amount, category_id, timestamp FROM records WHERE id = ?",
            [record_id.as_str()],
        )
        .await
        .expect("Failed to execute database query in test");

    if let Some(row) = rows.next().await.expect("Failed to read row from database") {
        let record =
            extract_record_from_row(row).expect("Failed to extract record with special characters");

        assert_eq!(record.name, special_name);
        assert_eq!(record.category_id, special_category);
        assert_eq!(record.amount, 99.99);
    } else {
        panic!(
            "Expected to find record with special characters (ID: {}), but query returned no results",
            record_id
        );
    }
}

/// Tests extract_record_from_row with extreme numeric values.
/// Validates handling of very large, very small, and negative amounts.
#[tokio::test]
async fn extract_record_with_extreme_values() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Test with extreme float values
    let large_amount = 999999.99;
    let small_amount = 0.01;
    let negative_amount = -50.25;

    let _id1 = create_test_record(
        &data_path,
        &user_id,
        "Large Amount",
        large_amount,
        "test",
        TEST_BASE_TIMESTAMP + 1,
    )
    .await;
    let _id2 = create_test_record(
        &data_path,
        &user_id,
        "Small Amount",
        small_amount,
        "test",
        TEST_BASE_TIMESTAMP + 2,
    )
    .await;
    let _id3 = create_test_record(
        &data_path,
        &user_id,
        "Negative Amount",
        negative_amount,
        "test",
        TEST_BASE_TIMESTAMP + 3,
    )
    .await;

    // Verify all amounts are stored and retrieved correctly
    let (records, _) = get_records_from_db(&data_path, &user_id, None, None, None).await;

    assert_eq!(records.len(), 3);

    // Find records by name since they're ordered by timestamp
    let large_record = records.iter().find(|r| r.name == "Large Amount").unwrap();
    let small_record = records.iter().find(|r| r.name == "Small Amount").unwrap();
    let negative_record = records
        .iter()
        .find(|r| r.name == "Negative Amount")
        .expect("Failed to execute database query in test");

    assert_eq!(large_record.amount, large_amount);
    assert_eq!(small_record.amount, small_amount);
    assert_eq!(negative_record.amount, negative_amount);
}

#[tokio::test]
async fn extract_record_with_long_strings() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Test with long strings (but within reasonable limits)
    let long_name = "A".repeat(200); // 200 character name
    let long_category = "category_".repeat(20); // Long category name

    let record_id = create_test_record(
        &data_path,
        &user_id,
        &long_name,
        42.42,
        &long_category,
        TEST_BASE_TIMESTAMP,
    )
    .await;

    // Verify long strings are handled correctly
    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.read().await;
    let mut rows = conn
        .query(
            "SELECT id, name, amount, category_id, timestamp FROM records WHERE id = ?",
            [record_id.as_str()],
        )
        .await
        .expect("Failed to execute database query in test");

    if let Some(row) = rows.next().await.expect("Failed to read row from database") {
        let record =
            extract_record_from_row(row).expect("Failed to extract record with long strings");

        assert_eq!(record.name, long_name);
        assert_eq!(record.category_id, long_category);
        assert_eq!(record.name.len(), 200);
        assert_eq!(record.amount, 42.42);
    } else {
        panic!(
            "Expected to find record with long strings (ID: {}), but query returned no results",
            record_id
        );
    }
}

/// Tests the default time behavior of get_records function.
/// Verifies that records are retrieved correctly when no time parameters are specified,
/// using the default start_time=0 and end_time=current_time logic.
#[tokio::test]
async fn default_time_behavior() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create a record to test default time behavior
    let current_time = time::OffsetDateTime::now_utc().unix_timestamp();
    create_test_record(
        &data_path,
        &user_id,
        "Current Record",
        50.0,
        "test",
        current_time,
    )
    .await;

    // Test that get_records with no time parameters returns records (uses default start_time=0, end_time=now)
    let (records, total_count) = get_records_from_db(&data_path, &user_id, None, None, None).await;

    assert_eq!(records.len(), 1);
    assert_eq!(total_count, 1);
    assert_eq!(records[0].name, "Current Record");

    // Test that providing start_time=0 explicitly works the same
    let (records2, total_count2) =
        get_records_from_db(&data_path, &user_id, Some(0), None, None).await;

    assert_eq!(records2.len(), records.len());
    assert_eq!(total_count2, total_count);
}

/// Tests the default limit behavior and pagination functionality.
/// Validates that the default limit of 500 works correctly and that explicit limits
/// properly control the number of returned records while maintaining accurate total counts.
#[tokio::test]
async fn limit_default_behavior() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create multiple records to test limit behavior
    let base_time = TEST_BASE_TIMESTAMP;
    for i in 0..10 {
        create_test_record(
            &data_path,
            &user_id,
            &format!("Record {}", i),
            10.0 + i as f64,
            "test",
            base_time + i,
        )
        .await;
    }

    // Test default limit (should be 500, but we only have 10 records)
    let (records, total_count) = get_records_from_db(&data_path, &user_id, None, None, None).await;
    assert_eq!(records.len(), 10); // All records returned
    assert_eq!(total_count, 10);

    // Test explicit limit smaller than available records
    let (limited_records, total_count2) =
        get_records_from_db(&data_path, &user_id, None, None, Some(5)).await;
    assert_eq!(limited_records.len(), 5); // Only 5 returned
    assert_eq!(total_count2, 10); // But total count shows all 10

    // Test limit larger than available records
    let (all_records, total_count3) =
        get_records_from_db(&data_path, &user_id, None, None, Some(20)).await;
    assert_eq!(all_records.len(), 10); // All 10 returned (can't return more than exist)
    assert_eq!(total_count3, 10);
}

#[tokio::test]
async fn database_timestamp_precision() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Test with precise timestamps
    let precise_timestamps = [
        TEST_BASE_TIMESTAMP,     // Base time
        TEST_BASE_TIMESTAMP + 1, // +1 second
        TEST_BASE_TIMESTAMP + 2, // +2 seconds
    ];

    for (i, &timestamp) in precise_timestamps.iter().enumerate() {
        create_test_record(
            &data_path,
            &user_id,
            &format!("Record {}", i),
            10.0,
            "test",
            timestamp,
        )
        .await;
    }

    let (records, _) = get_records_from_db(&data_path, &user_id, None, None, None).await;

    // Verify timestamps are stored as expected (SQLite stores as integers)
    assert_eq!(records.len(), 3);

    // Should be ordered by timestamp DESC
    // Record 2 (1700000002) should come first, then Record 1 (1700000001), then Record 0 (1700000000)
    assert!(records[0].timestamp >= records[1].timestamp);
    assert!(records[1].timestamp >= records[2].timestamp);

    // Verify specific ordering
    assert_eq!(records[0].name, "Record 2"); // Newest (1700000002)
    assert_eq!(records[1].name, "Record 1"); // Middle (1700000001)
    assert_eq!(records[2].name, "Record 0"); // Oldest (1700000000)
}

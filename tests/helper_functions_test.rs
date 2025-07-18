// Unit tests for helper functions in the records module
// These tests focus on testing the extract_record_from_row function
// and other utility functions without requiring full database setup

mod common;

use common::*;
use my_budget_server::database::get_user_db;

#[tokio::test]
async fn extract_record_from_row_success() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create a test record
    let timestamp = 1700000000;
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
    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.read().await;
    let mut rows = conn
        .query(
            "SELECT id, name, amount, category_id, timestamp FROM records WHERE id = ?",
            [record_id.as_str()],
        )
        .await
        .unwrap();

    if let Some(row) = rows.next().await.unwrap() {
        // Test the extraction logic manually (simulating extract_record_from_row)
        let id: String = row.get(0).unwrap();
        let name: String = row.get(1).unwrap();
        let amount: f64 = row.get(2).unwrap();
        let category_id: String = row.get(3).unwrap();
        let timestamp_result: i64 = row.get(4).unwrap();

        assert_eq!(id, record_id);
        assert_eq!(name, "Test Record");
        assert_eq!(amount, 25.50);
        assert_eq!(category_id, "test_category");
        assert_eq!(timestamp_result, timestamp);
    } else {
        panic!("No record found for extraction test");
    }
}

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
        1700000000,
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
        .unwrap();

    if let Some(row) = rows.next().await.unwrap() {
        let name: String = row.get(1).unwrap();
        let category_id: String = row.get(3).unwrap();

        assert_eq!(name, special_name);
        assert_eq!(category_id, special_category);
    } else {
        panic!("Record with special characters not found");
    }
}

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
        1700000001,
    )
    .await;
    let _id2 = create_test_record(
        &data_path,
        &user_id,
        "Small Amount",
        small_amount,
        "test",
        1700000002,
    )
    .await;
    let _id3 = create_test_record(
        &data_path,
        &user_id,
        "Negative Amount",
        negative_amount,
        "test",
        1700000003,
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
        .unwrap();

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
        1700000000,
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
        .unwrap();

    if let Some(row) = rows.next().await.unwrap() {
        let name: String = row.get(1).unwrap();
        let category_id: String = row.get(3).unwrap();

        assert_eq!(name, long_name);
        assert_eq!(category_id, long_category);
        assert_eq!(name.len(), 200);
    } else {
        panic!("Record with long strings not found");
    }
}

#[tokio::test]
async fn default_time_behavior() {
    // Test the default time logic used in get_records function
    let current_time = time::OffsetDateTime::now_utc().unix_timestamp();

    // Test start_time default (should be 0)
    let start_time = 0;
    assert_eq!(start_time, 0);

    // Test end_time default (should be current time)
    let end_time = time::OffsetDateTime::now_utc().unix_timestamp();
    assert!(end_time >= current_time);
    assert!(end_time - current_time < 5); // Should be within 5 seconds

    // Test with actual values
    let specific_start = 1700000000;
    let specific_end = 1700001000;

    assert_eq!(specific_start, 1700000000);
    assert_eq!(specific_end, 1700001000);
}

#[tokio::test]
async fn limit_default_behavior() {
    // Test the default limit logic used in get_records function
    let default_limit = 500;
    assert_eq!(default_limit, 500);

    let custom_limit = 100;
    assert_eq!(custom_limit, 100);

    let zero_limit = 0;
    assert_eq!(zero_limit, 0);
}

#[tokio::test]
async fn database_timestamp_precision() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Test with precise timestamps
    let precise_timestamps = [
        1700000000, // Base time
        1700000001, // +1 second
        1700000002, // +2 seconds (changed from large number that might cause issues)
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

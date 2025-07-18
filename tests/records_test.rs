/*!
 * Records Integration Tests
 *
 * This module contains comprehensive integration tests for the records management functionality.
 * Tests cover the full get_records API including time-range filtering, pagination, and ordering.
 *
 * Test Categories:
 * - Basic CRUD operations (empty database, record retrieval)
 * - Time-range filtering (start_time, end_time, both)
 * - Pagination and limits (default behavior, custom limits)
 * - Ordering and consistency (timestamp ordering, edge cases)
 * - Data integrity (category preservation, amount accuracy)
 *
 * All tests use isolated temporary databases for complete test isolation.
 */

mod common;

use common::*;
use my_budget_server::models::Record;

// Test data constants - only for widely reused values
const TEST_BASE_TIMESTAMP: i64 = 1700000000; // Nov 14, 2023 22:13:20 UTC
const TEST_TIME_INCREMENT: i64 = 100; // 100 seconds between test records

// Test utility functions specific to records tests
fn get_test_timestamps() -> (i64, i64, i64, i64) {
    (
        TEST_BASE_TIMESTAMP,                           // old_time
        TEST_BASE_TIMESTAMP + TEST_TIME_INCREMENT,     // middle_time
        TEST_BASE_TIMESTAMP + TEST_TIME_INCREMENT * 2, // new_time
        TEST_BASE_TIMESTAMP + TEST_TIME_INCREMENT * 3, // future_time
    )
}

async fn create_sample_records(data_path: &str, user_id: &str) {
    let (old_time, middle_time, new_time, _future_time) = get_test_timestamps();

    create_test_record(data_path, user_id, "Old Record", 10.50, "food", old_time).await;
    create_test_record(
        data_path,
        user_id,
        "Middle Record",
        25.75,
        "transport",
        middle_time,
    )
    .await;
    create_test_record(
        data_path,
        user_id,
        "New Record",
        15.25,
        "entertainment",
        new_time,
    )
    .await;
}

// Helper functions for update tests
async fn update_record_in_db(
    data_path: &str,
    user_id: &str,
    record_id: &str,
    name: Option<&str>,
    amount: Option<f64>,
    category_id: Option<&str>,
    timestamp: Option<i64>,
) -> Result<Record, String> {
    use my_budget_server::database::get_user_db;
    use my_budget_server::records::extract_record_from_row;

    // Validate that at least one field is being updated
    if name.is_none() && amount.is_none() && category_id.is_none() && timestamp.is_none() {
        return Err("At least one field must be provided for update".to_string());
    }

    // Input validation for provided fields - reuse production validation functions
    if let Some(name_val) = name {
        if let Err((_, error_msg)) = my_budget_server::records::validate_record_name(name_val) {
            return Err(error_msg);
        }
    }

    if let Some(amount_val) = amount {
        if let Err((_, error_msg)) = my_budget_server::records::validate_record_amount(amount_val) {
            return Err(error_msg);
        }
    }

    if let Some(category_val) = category_id {
        if let Err((_, error_msg)) = my_budget_server::records::validate_category_id(category_val) {
            return Err(error_msg);
        }
    }

    let user_db = get_user_db(data_path, user_id)
        .await
        .map_err(|e| format!("Failed to get user database: {}", e))?;

    let conn = user_db.write().await;

    // First, get the existing record
    let mut existing_rows = conn
        .query(
            "SELECT id, name, amount, category_id, timestamp FROM records WHERE id = ?",
            [record_id],
        )
        .await
        .map_err(|e| format!("Failed to query existing record: {}", e))?;

    let existing_record = if let Some(row) = existing_rows
        .next()
        .await
        .map_err(|e| format!("Failed to read existing record: {}", e))?
    {
        extract_record_from_row(row)
            .map_err(|e| format!("Failed to extract existing record: {}", e.1))?
    } else {
        return Err("Record not found".to_string());
    };

    // Build updated values
    let updated_name = name.unwrap_or(&existing_record.name);
    let updated_amount = amount.unwrap_or(existing_record.amount);
    let updated_category_id = category_id.unwrap_or(&existing_record.category_id);
    let updated_timestamp = timestamp.unwrap_or(existing_record.timestamp);

    // Update the record
    let affected_rows = conn
        .execute(
            "UPDATE records SET name = ?, amount = ?, category_id = ?, timestamp = ? WHERE id = ?",
            (
                updated_name,
                updated_amount,
                updated_category_id,
                updated_timestamp,
                record_id,
            ),
        )
        .await
        .map_err(|e| format!("Failed to update record: {}", e))?;

    if affected_rows == 0 {
        return Err("Record not found or no changes made".to_string());
    }

    Ok(Record {
        id: record_id.to_string(),
        name: updated_name.to_string(),
        amount: updated_amount,
        category_id: updated_category_id.to_string(),
        timestamp: updated_timestamp,
    })
}

async fn get_single_record_from_db(
    data_path: &str,
    user_id: &str,
    record_id: &str,
) -> Option<Record> {
    use my_budget_server::database::get_user_db;
    use my_budget_server::records::extract_record_from_row;

    let user_db = get_user_db(data_path, user_id).await.ok()?;
    let conn = user_db.read().await;

    let mut rows = conn
        .query(
            "SELECT id, name, amount, category_id, timestamp FROM records WHERE id = ?",
            [record_id],
        )
        .await
        .ok()?;

    if let Some(row) = rows.next().await.ok()? {
        extract_record_from_row(row).ok()
    } else {
        None
    }
}

#[tokio::test]
async fn empty_database() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let (records, total_count) = get_records_from_db(&data_path, &user_id, None, None, None).await;

    assert_eq!(records.len(), 0);
    assert_eq!(total_count, 0);
}

#[tokio::test]
async fn get_all_records() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;
    create_sample_records(&data_path, &user_id).await;

    let (records, total_count) = get_records_from_db(&data_path, &user_id, None, None, None).await;

    assert_eq!(records.len(), 3);
    assert_eq!(total_count, 3);

    // Check ordering (newest first)
    assert_eq!(records[0].name, "New Record");
    assert_eq!(records[1].name, "Middle Record");
    assert_eq!(records[2].name, "Old Record");

    // Check amounts
    assert_eq!(records[0].amount, 15.25);
    assert_eq!(records[1].amount, 25.75);
    assert_eq!(records[2].amount, 10.50);
}

#[tokio::test]
async fn time_range_filtering_start_only() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;
    create_sample_records(&data_path, &user_id).await;

    let (_old_time, middle_time, _new_time, _future_time) = get_test_timestamps();

    // Get records from middle_time onward
    let (records, total_count) =
        get_records_from_db(&data_path, &user_id, Some(middle_time), None, None).await;

    assert_eq!(records.len(), 2); // Middle and New records
    assert_eq!(total_count, 2);
    assert_eq!(records[0].name, "New Record");
    assert_eq!(records[1].name, "Middle Record");
}

#[tokio::test]
async fn time_range_filtering_end_only() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;
    create_sample_records(&data_path, &user_id).await;

    let (_old_time, middle_time, _new_time, _future_time) = get_test_timestamps();

    // Get records up to middle_time
    let (records, total_count) =
        get_records_from_db(&data_path, &user_id, None, Some(middle_time), None).await;

    assert_eq!(records.len(), 2); // Old and Middle records
    assert_eq!(total_count, 2);
    assert_eq!(records[0].name, "Middle Record");
    assert_eq!(records[1].name, "Old Record");
}

#[tokio::test]
async fn time_range_filtering_both() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;
    create_sample_records(&data_path, &user_id).await;

    let (old_time, _middle_time, new_time, _future_time) = get_test_timestamps();

    // Get records between old_time + 50 and new_time - 50 (should only include middle)
    let start_time = old_time + 50;
    let end_time = new_time - 50;

    let (records, total_count) =
        get_records_from_db(&data_path, &user_id, Some(start_time), Some(end_time), None).await;

    assert_eq!(records.len(), 1); // Only Middle record
    assert_eq!(total_count, 1);
    assert_eq!(records[0].name, "Middle Record");
}

#[tokio::test]
async fn limit_functionality() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;
    create_sample_records(&data_path, &user_id).await;

    // Test limit of 2
    let (records, total_count) =
        get_records_from_db(&data_path, &user_id, None, None, Some(2)).await;

    assert_eq!(records.len(), 2); // Only 2 records returned
    assert_eq!(total_count, 3); // But total count is still 3

    // Should return the 2 newest records
    assert_eq!(records[0].name, "New Record");
    assert_eq!(records[1].name, "Middle Record");
}

#[tokio::test]
async fn limit_with_time_range() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create more records for this test
    let base_time = 1700000000;
    for i in 0..5 {
        create_test_record(
            &data_path,
            &user_id,
            &format!("Record {}", i),
            10.0 + i as f64,
            "test",
            base_time + i * 10,
        )
        .await;
    }

    // Get 3 records from a time range that includes all 5
    let (records, total_count) = get_records_from_db(
        &data_path,
        &user_id,
        Some(base_time),
        Some(base_time + 100),
        Some(3),
    )
    .await;

    assert_eq!(records.len(), 3); // Only 3 records returned due to limit
    assert_eq!(total_count, 5); // But total count shows all 5 in range

    // Should return newest 3 in descending order
    assert_eq!(records[0].name, "Record 4");
    assert_eq!(records[1].name, "Record 3");
    assert_eq!(records[2].name, "Record 2");
}

#[tokio::test]
async fn default_limit() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;
    create_sample_records(&data_path, &user_id).await;

    // Test with None limit (should use default of 500)
    let (records, total_count) = get_records_from_db(&data_path, &user_id, None, None, None).await;

    assert_eq!(records.len(), 3); // All 3 records (less than default limit)
    assert_eq!(total_count, 3);
}

#[tokio::test]
async fn ordering_consistency() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create records with very close timestamps
    let base_time = 1700000000;
    create_test_record(&data_path, &user_id, "First", 10.0, "test", base_time).await;
    create_test_record(&data_path, &user_id, "Second", 20.0, "test", base_time + 1).await;
    create_test_record(&data_path, &user_id, "Third", 30.0, "test", base_time + 2).await;

    let (records, _) = get_records_from_db(&data_path, &user_id, None, None, None).await;

    // Should be in descending timestamp order (newest first)
    assert_eq!(records[0].name, "Third");
    assert_eq!(records[1].name, "Second");
    assert_eq!(records[2].name, "First");

    // Verify timestamps are in descending order
    assert!(records[0].timestamp > records[1].timestamp);
    assert!(records[1].timestamp > records[2].timestamp);
}

#[tokio::test]
async fn edge_case_no_results_in_range() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;
    create_sample_records(&data_path, &user_id).await;

    let (_old_time, _middle_time, _new_time, future_time) = get_test_timestamps();

    // Query for a time range that has no records
    let (records, total_count) = get_records_from_db(
        &data_path,
        &user_id,
        Some(future_time + 100),
        Some(future_time + 200),
        None,
    )
    .await;

    assert_eq!(records.len(), 0);
    assert_eq!(total_count, 0);
}

#[tokio::test]
async fn single_record_in_range() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let base_time = 1700000000;
    create_test_record(
        &data_path,
        &user_id,
        "Only Record",
        42.0,
        "unique",
        base_time,
    )
    .await;

    // Query for exact timestamp match
    let (records, total_count) =
        get_records_from_db(&data_path, &user_id, Some(base_time), Some(base_time), None).await;

    assert_eq!(records.len(), 1);
    assert_eq!(total_count, 1);
    assert_eq!(records[0].name, "Only Record");
    assert_eq!(records[0].amount, 42.0);
}

#[tokio::test]
async fn category_preservation() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let base_time = 1700000000;
    create_test_record(
        &data_path,
        &user_id,
        "Food Expense",
        12.50,
        "food",
        base_time,
    )
    .await;
    create_test_record(
        &data_path,
        &user_id,
        "Gas Expense",
        45.00,
        "transport",
        base_time + 10,
    )
    .await;

    let (records, _) = get_records_from_db(&data_path, &user_id, None, None, None).await;

    assert_eq!(records.len(), 2);

    // Check that categories are preserved correctly
    assert_eq!(records[0].category_id, "transport");
    assert_eq!(records[1].category_id, "food");

    // Check that names and amounts match categories
    assert_eq!(records[0].name, "Gas Expense");
    assert_eq!(records[0].amount, 45.00);
    assert_eq!(records[1].name, "Food Expense");
    assert_eq!(records[1].amount, 12.50);
}

// Update Record Tests

/// Tests updating only the name field of a record.
/// Verifies that the name changes while other fields remain unchanged.
#[tokio::test]
async fn update_record_single_field_name() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let original_name = "Original Name";
    let original_amount = 25.50;
    let original_category = "test_category";
    let original_timestamp = TEST_BASE_TIMESTAMP;

    let record_id = create_test_record(
        &data_path,
        &user_id,
        original_name,
        original_amount,
        original_category,
        original_timestamp,
    )
    .await;

    // Update only the name
    let updated_name = "Updated Name Only";
    let updated_record = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        Some(updated_name),
        None,
        None,
        None,
    )
    .await
    .expect("Failed to update record name");

    // Verify the update
    assert_eq!(updated_record.name, updated_name);
    assert_eq!(updated_record.amount, original_amount);
    assert_eq!(updated_record.category_id, original_category);
    assert_eq!(updated_record.timestamp, original_timestamp);
    assert_eq!(updated_record.id, record_id);

    // Verify in database
    let db_record = get_single_record_from_db(&data_path, &user_id, &record_id)
        .await
        .expect("Failed to retrieve updated record from database");

    assert_eq!(db_record.name, updated_name);
    assert_eq!(db_record.amount, original_amount);
    assert_eq!(db_record.category_id, original_category);
    assert_eq!(db_record.timestamp, original_timestamp);
}

/// Tests updating only the amount field of a record.
/// Verifies that the amount changes while other fields remain unchanged.
#[tokio::test]
async fn update_record_single_field_amount() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let original_name = "Test Record";
    let original_amount = 15.75;
    let original_category = "food";
    let original_timestamp = TEST_BASE_TIMESTAMP;

    let record_id = create_test_record(
        &data_path,
        &user_id,
        original_name,
        original_amount,
        original_category,
        original_timestamp,
    )
    .await;

    // Update only the amount
    let updated_amount = 99.99;
    let updated_record = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        None,
        Some(updated_amount),
        None,
        None,
    )
    .await
    .expect("Failed to update record amount");

    // Verify the update
    assert_eq!(updated_record.name, original_name);
    assert_eq!(updated_record.amount, updated_amount);
    assert_eq!(updated_record.category_id, original_category);
    assert_eq!(updated_record.timestamp, original_timestamp);
    assert_eq!(updated_record.id, record_id);

    // Verify in database
    let db_record = get_single_record_from_db(&data_path, &user_id, &record_id)
        .await
        .expect("Failed to retrieve updated record from database");

    assert_eq!(db_record.amount, updated_amount);
    assert_eq!(db_record.name, original_name);
}

/// Tests updating only the category field of a record.
/// Verifies that the category changes while other fields remain unchanged.
#[tokio::test]
async fn update_record_single_field_category() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let original_name = "Category Test";
    let original_amount = 33.33;
    let original_category = "entertainment";
    let original_timestamp = TEST_BASE_TIMESTAMP;

    let record_id = create_test_record(
        &data_path,
        &user_id,
        original_name,
        original_amount,
        original_category,
        original_timestamp,
    )
    .await;

    // Update only the category
    let updated_category = "updated_category";
    let updated_record = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        None,
        None,
        Some(updated_category),
        None,
    )
    .await
    .expect("Failed to update record category");

    // Verify the update
    assert_eq!(updated_record.name, original_name);
    assert_eq!(updated_record.amount, original_amount);
    assert_eq!(updated_record.category_id, updated_category);
    assert_eq!(updated_record.timestamp, original_timestamp);
    assert_eq!(updated_record.id, record_id);

    // Verify in database
    let db_record = get_single_record_from_db(&data_path, &user_id, &record_id)
        .await
        .expect("Failed to retrieve updated record from database");

    assert_eq!(db_record.category_id, updated_category);
    assert_eq!(db_record.name, original_name);
    assert_eq!(db_record.amount, original_amount);
}

/// Tests updating only the timestamp field of a record.
/// Verifies that the timestamp changes while other fields remain unchanged.
#[tokio::test]
async fn update_record_single_field_timestamp() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let original_name = "Timestamp Test";
    let original_amount = 77.77;
    let original_category = "transport";
    let original_timestamp = TEST_BASE_TIMESTAMP;

    let record_id = create_test_record(
        &data_path,
        &user_id,
        original_name,
        original_amount,
        original_category,
        original_timestamp,
    )
    .await;

    // Update only the timestamp
    let updated_timestamp = TEST_BASE_TIMESTAMP + 500;
    let updated_record = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        None,
        None,
        None,
        Some(updated_timestamp),
    )
    .await
    .expect("Failed to update record timestamp");

    // Verify the update
    assert_eq!(updated_record.name, original_name);
    assert_eq!(updated_record.amount, original_amount);
    assert_eq!(updated_record.category_id, original_category);
    assert_eq!(updated_record.timestamp, updated_timestamp);
    assert_eq!(updated_record.id, record_id);

    // Verify in database
    let db_record = get_single_record_from_db(&data_path, &user_id, &record_id)
        .await
        .expect("Failed to retrieve updated record from database");

    assert_eq!(db_record.timestamp, updated_timestamp);
    assert_eq!(db_record.name, original_name);
    assert_eq!(db_record.amount, original_amount);
    assert_eq!(db_record.category_id, original_category);
}

/// Tests updating multiple fields of a record simultaneously.
/// Verifies that multiple fields change while unchanged fields remain intact.
#[tokio::test]
async fn update_record_multiple_fields() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let original_name = "Multiple Fields Test";
    let original_amount = 10.00;
    let original_category = "original_category";
    let original_timestamp = TEST_BASE_TIMESTAMP;

    let record_id = create_test_record(
        &data_path,
        &user_id,
        original_name,
        original_amount,
        original_category,
        original_timestamp,
    )
    .await;

    // Update name and amount, leave category and timestamp unchanged
    let updated_name = "Updated Multiple Fields";
    let updated_amount = 55.55;
    let updated_record = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        Some(updated_name),
        Some(updated_amount),
        None,
        None,
    )
    .await
    .expect("Failed to update multiple record fields");

    // Verify the updates
    assert_eq!(updated_record.name, updated_name);
    assert_eq!(updated_record.amount, updated_amount);
    assert_eq!(updated_record.category_id, original_category); // unchanged
    assert_eq!(updated_record.timestamp, original_timestamp); // unchanged
    assert_eq!(updated_record.id, record_id);

    // Verify in database
    let db_record = get_single_record_from_db(&data_path, &user_id, &record_id)
        .await
        .expect("Failed to retrieve updated record from database");

    assert_eq!(db_record.name, updated_name);
    assert_eq!(db_record.amount, updated_amount);
    assert_eq!(db_record.category_id, original_category);
    assert_eq!(db_record.timestamp, original_timestamp);
}

/// Tests updating all fields of a record simultaneously.
/// Verifies that all fields change to their new values.
#[tokio::test]
async fn update_record_all_fields() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let original_name = "All Fields Test";
    let original_amount = 20.00;
    let original_category = "original_all";
    let original_timestamp = TEST_BASE_TIMESTAMP;

    let record_id = create_test_record(
        &data_path,
        &user_id,
        original_name,
        original_amount,
        original_category,
        original_timestamp,
    )
    .await;

    // Update all fields
    let updated_name = "All Fields Updated";
    let updated_amount = 88.88;
    let updated_category = "updated_all";
    let updated_timestamp = TEST_BASE_TIMESTAMP + 1000;

    let updated_record = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        Some(updated_name),
        Some(updated_amount),
        Some(updated_category),
        Some(updated_timestamp),
    )
    .await
    .expect("Failed to update all record fields");

    // Verify all updates
    assert_eq!(updated_record.name, updated_name);
    assert_eq!(updated_record.amount, updated_amount);
    assert_eq!(updated_record.category_id, updated_category);
    assert_eq!(updated_record.timestamp, updated_timestamp);
    assert_eq!(updated_record.id, record_id);

    // Verify in database
    let db_record = get_single_record_from_db(&data_path, &user_id, &record_id)
        .await
        .expect("Failed to retrieve updated record from database");

    assert_eq!(db_record.name, updated_name);
    assert_eq!(db_record.amount, updated_amount);
    assert_eq!(db_record.category_id, updated_category);
    assert_eq!(db_record.timestamp, updated_timestamp);
}

/// Tests updating a record with no fields provided (empty payload).
/// Should fail with appropriate error message.
#[tokio::test]
async fn update_record_empty_payload() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let record_id = create_test_record(
        &data_path,
        &user_id,
        "Empty Payload Test",
        30.00,
        "test_category",
        TEST_BASE_TIMESTAMP,
    )
    .await;

    // Try to update with no fields (should fail)
    let result =
        update_record_in_db(&data_path, &user_id, &record_id, None, None, None, None).await;

    // Should fail because no fields were provided
    assert!(result.is_err());
    let error_message = result.unwrap_err();
    assert!(
        error_message.contains("At least one field must be provided for update")
            || error_message.contains("no changes made")
            || error_message.contains("not found")
    );
}

/// Tests updating a record with empty name.
/// Should fail validation as empty names are not allowed.
#[tokio::test]
async fn update_record_empty_name() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let record_id = create_test_record(
        &data_path,
        &user_id,
        "Empty Name Test",
        40.00,
        "test_category",
        TEST_BASE_TIMESTAMP,
    )
    .await;

    // Test empty name (should fail validation)
    let result = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        Some(""), // Empty name
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let error_message = result.unwrap_err();
    assert!(error_message.contains("Record name cannot be empty"));
}

/// Tests updating a record with zero amount.
/// Should fail validation as zero amounts are not allowed.
#[tokio::test]
async fn update_record_zero_amount() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let record_id = create_test_record(
        &data_path,
        &user_id,
        "Zero Amount Test",
        40.00,
        "test_category",
        TEST_BASE_TIMESTAMP,
    )
    .await;

    // Test zero amount (should fail validation)
    let result = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        None,
        Some(0.0), // Zero amount
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let error_message = result.unwrap_err();
    assert!(error_message.contains("Record amount cannot be zero"));
}

/// Tests updating a record with empty category.
/// Should fail validation as empty categories are not allowed.
#[tokio::test]
async fn update_record_empty_category() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let record_id = create_test_record(
        &data_path,
        &user_id,
        "Empty Category Test",
        40.00,
        "test_category",
        TEST_BASE_TIMESTAMP,
    )
    .await;

    // Test empty category (should fail validation)
    let result = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        None,
        None,
        Some(""), // Empty category
        None,
    )
    .await;

    assert!(result.is_err());
    let error_message = result.unwrap_err();
    assert!(error_message.contains("Category ID cannot be empty"));
}

/// Tests updating a record with whitespace-only name.
/// Should fail validation as whitespace-only names are treated as empty.
#[tokio::test]
async fn update_record_whitespace_only_name() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let record_id = create_test_record(
        &data_path,
        &user_id,
        "Whitespace Test",
        50.00,
        "test_category",
        TEST_BASE_TIMESTAMP,
    )
    .await;

    // Test whitespace-only name (should fail validation)
    let result = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        Some("   "), // Whitespace-only name
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let error_message = result.unwrap_err();
    assert!(error_message.contains("Record name cannot be empty"));

    // Test mixed whitespace (tabs and spaces)
    let result = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        Some(" \t \n "), // Mixed whitespace
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let error_message = result.unwrap_err();
    assert!(error_message.contains("Record name cannot be empty"));
}

/// Tests updating a record with maximum allowed name length (255 characters).
/// Should succeed as 255 characters is the boundary limit.
#[tokio::test]
async fn update_record_max_name_length() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let record_id = create_test_record(
        &data_path,
        &user_id,
        "Max Length Test",
        60.00,
        "test_category",
        TEST_BASE_TIMESTAMP,
    )
    .await;

    // Test 255-character name (should succeed)
    let max_length_name = "a".repeat(255);
    let result = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        Some(&max_length_name),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let updated_record = result.unwrap();
    assert_eq!(updated_record.name, max_length_name);
    assert_eq!(updated_record.name.len(), 255);
}

/// Tests updating a record with name exceeding maximum length (256 characters).
/// Should fail validation as names cannot exceed 255 characters.
#[tokio::test]
async fn update_record_too_long_name() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let record_id = create_test_record(
        &data_path,
        &user_id,
        "Too Long Test",
        70.00,
        "test_category",
        TEST_BASE_TIMESTAMP,
    )
    .await;

    // Test 256-character name (should fail)
    let too_long_name = "a".repeat(256);
    let result = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        Some(&too_long_name),
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let error_message = result.unwrap_err();
    assert!(error_message.contains("Record name must be less than 255 characters"));
}

/// Tests updating a record with negative amount.
/// Should succeed as negative amounts may represent refunds or corrections.
#[tokio::test]
async fn update_record_negative_amount() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record
    let record_id = create_test_record(
        &data_path,
        &user_id,
        "Negative Amount Test",
        80.00,
        "test_category",
        TEST_BASE_TIMESTAMP,
    )
    .await;

    // Test negative amount (should succeed)
    let negative_amount = -25.50;
    let result = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        None,
        Some(negative_amount),
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let updated_record = result.unwrap();
    assert_eq!(updated_record.amount, negative_amount);
}

/// Tests updating a non-existent record.
/// Should fail with "Record not found" error.
#[tokio::test]
async fn update_record_nonexistent_record() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Try to update a record that doesn't exist
    let fake_record_id = "non-existent-record-id";
    let result = update_record_in_db(
        &data_path,
        &user_id,
        fake_record_id,
        Some("This should fail"),
        None,
        None,
        None,
    )
    .await;

    // Should fail because record doesn't exist
    assert!(result.is_err());
    let error_message = result.unwrap_err();
    assert!(error_message.contains("Record not found"));
}

/// Tests that unchanged fields are preserved during partial updates.
/// Verifies data integrity by checking that unmodified fields remain intact.
#[tokio::test]
async fn update_record_preserves_unchanged_fields() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create initial record with specific values
    let original_name = "Preserve Test";
    let original_amount = 123.45;
    let original_category = "preserve_category";
    let original_timestamp = TEST_BASE_TIMESTAMP + 100;

    let record_id = create_test_record(
        &data_path,
        &user_id,
        original_name,
        original_amount,
        original_category,
        original_timestamp,
    )
    .await;

    // Update only the name, leaving everything else unchanged
    let updated_name = "Name Only Updated";
    let updated_record = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        Some(updated_name),
        None,
        None,
        None,
    )
    .await
    .expect("Failed to update record with preserved fields");

    // Verify name changed but everything else preserved
    assert_eq!(updated_record.name, updated_name);
    assert_eq!(updated_record.amount, original_amount);
    assert_eq!(updated_record.category_id, original_category);
    assert_eq!(updated_record.timestamp, original_timestamp);

    // Now update only the amount
    let updated_amount = 999.99;
    let updated_record = update_record_in_db(
        &data_path,
        &user_id,
        &record_id,
        None,
        Some(updated_amount),
        None,
        None,
    )
    .await
    .expect("Failed to update record amount with preserved fields");

    // Verify amount changed but name (from previous update) and other fields preserved
    assert_eq!(updated_record.name, updated_name); // From previous update
    assert_eq!(updated_record.amount, updated_amount); // New update
    assert_eq!(updated_record.category_id, original_category); // Preserved
    assert_eq!(updated_record.timestamp, original_timestamp); // Preserved

    // Verify final state in database
    let db_record = get_single_record_from_db(&data_path, &user_id, &record_id)
        .await
        .expect("Failed to retrieve record after preservation test");

    assert_eq!(db_record.name, updated_name);
    assert_eq!(db_record.amount, updated_amount);
    assert_eq!(db_record.category_id, original_category);
    assert_eq!(db_record.timestamp, original_timestamp);
}

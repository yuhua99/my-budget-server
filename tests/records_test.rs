mod common;

use common::*;

// Test utility functions specific to records tests
fn get_test_timestamps() -> (i64, i64, i64, i64) {
    let base_time = 1700000000; // Fixed base timestamp for consistent testing
    (
        base_time,       // old_time
        base_time + 100, // middle_time
        base_time + 200, // new_time
        base_time + 300, // future_time
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

#[tokio::test]
async fn empty_database() {
    let (data_path, user_id) = setup_test_environment().await;

    let (records, total_count) = get_records_from_db(&data_path, &user_id, None, None, None).await;

    assert_eq!(records.len(), 0);
    assert_eq!(total_count, 0);
}

#[tokio::test]
async fn get_all_records() {
    let (data_path, user_id) = setup_test_environment().await;
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
    let (data_path, user_id) = setup_test_environment().await;
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
    let (data_path, user_id) = setup_test_environment().await;
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
    let (data_path, user_id) = setup_test_environment().await;
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
    let (data_path, user_id) = setup_test_environment().await;
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
    let (data_path, user_id) = setup_test_environment().await;

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
    let (data_path, user_id) = setup_test_environment().await;
    create_sample_records(&data_path, &user_id).await;

    // Test with None limit (should use default of 500)
    let (records, total_count) = get_records_from_db(&data_path, &user_id, None, None, None).await;

    assert_eq!(records.len(), 3); // All 3 records (less than default limit)
    assert_eq!(total_count, 3);
}

#[tokio::test]
async fn ordering_consistency() {
    let (data_path, user_id) = setup_test_environment().await;

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
    let (data_path, user_id) = setup_test_environment().await;
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
    let (data_path, user_id) = setup_test_environment().await;

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
    let (data_path, user_id) = setup_test_environment().await;

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

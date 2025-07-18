use my_budget_server::database::{get_user_db, init_main_db};
use my_budget_server::models::Record;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

pub async fn setup_test_environment() -> (String, String) {
    let temp_dir = tempdir().expect("Failed to create temporary directory");
    let data_path = temp_dir
        .path()
        .to_str()
        .expect("Failed to convert path to string")
        .to_string();
    let user_id = Uuid::new_v4().to_string();

    // Ensure the data directory exists with proper permissions
    fs::create_dir_all(&data_path).expect("Failed to create data directory");

    // Initialize main database (for completeness)
    init_main_db(&data_path)
        .await
        .unwrap_or_else(|e| panic!("Failed to initialize main database at {}: {}", data_path, e));

    // Initialize user database with better error handling
    get_user_db(&data_path, &user_id).await.unwrap_or_else(|e| {
        panic!(
            "Failed to initialize user database for user {} at {}: {}",
            user_id, data_path, e
        )
    });

    // Keep the temp_dir alive by leaking it (for test duration)
    std::mem::forget(temp_dir);

    (data_path, user_id)
}

pub async fn create_test_record(
    data_path: &str,
    user_id: &str,
    name: &str,
    amount: f64,
    category_id: &str,
    timestamp: i64,
) -> String {
    let user_db = get_user_db(data_path, user_id)
        .await
        .unwrap_or_else(|e| panic!("Failed to get user database for {}: {}", user_id, e));
    let record_id = Uuid::new_v4().to_string();

    let conn = user_db.write().await;
    conn.execute(
        "INSERT INTO records (id, name, amount, category_id, timestamp) VALUES (?, ?, ?, ?, ?)",
        (record_id.as_str(), name, amount, category_id, timestamp),
    )
    .await
    .unwrap_or_else(|e| {
        panic!(
            "Failed to insert test record '{}' for user {}: {}",
            name, user_id, e
        )
    });

    record_id
}

pub async fn get_records_from_db(
    data_path: &str,
    user_id: &str,
    start_time: Option<i64>,
    end_time: Option<i64>,
    limit: Option<u32>,
) -> (Vec<Record>, u32) {
    let user_db = get_user_db(data_path, user_id)
        .await
        .unwrap_or_else(|e| panic!("Failed to get user database for {}: {}", user_id, e));
    let conn = user_db.read().await;

    // Use same default logic as get_records function
    let start = start_time.unwrap_or(0);
    let end = end_time.unwrap_or_else(|| time::OffsetDateTime::now_utc().unix_timestamp());
    let lim = limit.unwrap_or(500);

    // Get total count
    let mut count_rows = conn
        .query(
            "SELECT COUNT(*) FROM records WHERE timestamp BETWEEN ? AND ?",
            (start, end),
        )
        .await
        .expect("Failed to execute count query");

    let total_count: u32 =
        if let Some(row) = count_rows.next().await.expect("Failed to read count row") {
            row.get(0).expect("Failed to get count value")
        } else {
            0
        };

    // Get records
    let mut rows = conn
        .query(
            "SELECT id, name, amount, category_id, timestamp FROM records WHERE timestamp BETWEEN ? AND ? ORDER BY timestamp DESC LIMIT ?",
            (start, end, lim),
        )
        .await
        .expect("Failed to execute records query");

    let mut records = Vec::new();
    while let Some(row) = rows.next().await.expect("Failed to read record row") {
        let id: String = row.get(0).expect("Failed to get record id");
        let name: String = row.get(1).expect("Failed to get record name");
        let amount: f64 = row.get(2).expect("Failed to get record amount");
        let category_id: String = row.get(3).expect("Failed to get record category_id");
        let timestamp: i64 = row.get(4).expect("Failed to get record timestamp");

        records.push(Record {
            id,
            name,
            amount,
            category_id,
            timestamp,
        });
    }

    (records, total_count)
}

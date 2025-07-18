use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tempfile::tempdir;
use tokio::runtime::Runtime;
use uuid::Uuid;

use my_budget_server::database::{get_user_db, init_main_db};

// Benchmark constants
const BENCH_BASE_TIMESTAMP: i64 = 1700000000;
const BENCH_RECORD_COUNT: usize = 1000;

async fn setup_benchmark_environment() -> (String, String, tempfile::TempDir) {
    let temp_dir = tempdir().expect("Failed to create temporary directory");
    let data_path = temp_dir.path().to_str().unwrap().to_string();
    let user_id = Uuid::new_v4().to_string();

    init_main_db(&data_path).await.unwrap();
    get_user_db(&data_path, &user_id).await.unwrap();

    (data_path, user_id, temp_dir)
}

async fn create_benchmark_records(data_path: &str, user_id: &str, count: usize) {
    let user_db = get_user_db(data_path, user_id).await.unwrap();
    let conn = user_db.write().await;

    for i in 0..count {
        let record_id = Uuid::new_v4().to_string();
        let timestamp = BENCH_BASE_TIMESTAMP + i as i64;
        let amount = 10.0 + (i % 100) as f64;
        let name = format!("Benchmark Record {}", i);
        let category = format!("category_{}", i % 10);

        conn.execute(
            "INSERT INTO records (id, name, amount, category_id, timestamp) VALUES (?, ?, ?, ?, ?)",
            (
                record_id.as_str(),
                name.as_str(),
                amount,
                category.as_str(),
                timestamp,
            ),
        )
        .await
        .unwrap();
    }
}

async fn benchmark_get_all_records(data_path: &str, user_id: &str) {
    let user_db = get_user_db(data_path, user_id).await.unwrap();
    let conn = user_db.read().await;

    let mut rows = conn
        .query(
            "SELECT id, name, amount, category_id, timestamp FROM records ORDER BY timestamp DESC LIMIT 500",
            (),
        )
        .await
        .unwrap();

    let mut count = 0;
    while let Some(_row) = rows.next().await.unwrap() {
        count += 1;
    }

    black_box(count);
}

async fn benchmark_time_range_query(data_path: &str, user_id: &str) {
    let user_db = get_user_db(data_path, user_id).await.unwrap();
    let conn = user_db.read().await;

    let start_time = BENCH_BASE_TIMESTAMP + 100;
    let end_time = BENCH_BASE_TIMESTAMP + 500;

    let mut rows = conn
        .query(
            "SELECT id, name, amount, category_id, timestamp FROM records WHERE timestamp BETWEEN ? AND ? ORDER BY timestamp DESC LIMIT 500",
            (start_time, end_time),
        )
        .await
        .unwrap();

    let mut count = 0;
    while let Some(_row) = rows.next().await.unwrap() {
        count += 1;
    }

    black_box(count);
}

async fn benchmark_count_query(data_path: &str, user_id: &str) {
    let user_db = get_user_db(data_path, user_id).await.unwrap();
    let conn = user_db.read().await;

    let start_time = BENCH_BASE_TIMESTAMP;
    let end_time = BENCH_BASE_TIMESTAMP + 1000;

    let mut count_rows = conn
        .query(
            "SELECT COUNT(*) FROM records WHERE timestamp BETWEEN ? AND ?",
            (start_time, end_time),
        )
        .await
        .unwrap();

    if let Some(row) = count_rows.next().await.unwrap() {
        let count: u32 = row.get(0).unwrap();
        black_box(count);
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Setup benchmark data once
    let (data_path, user_id, _temp_dir) = rt.block_on(setup_benchmark_environment());
    rt.block_on(create_benchmark_records(
        &data_path,
        &user_id,
        BENCH_RECORD_COUNT,
    ));

    c.bench_function("get_all_records", |b| {
        b.to_async(&rt)
            .iter(|| benchmark_get_all_records(&data_path, &user_id))
    });

    c.bench_function("time_range_query", |b| {
        b.to_async(&rt)
            .iter(|| benchmark_time_range_query(&data_path, &user_id))
    });

    c.bench_function("count_query", |b| {
        b.to_async(&rt)
            .iter(|| benchmark_count_query(&data_path, &user_id))
    });

    // Keep temp_dir alive until the end
    std::mem::forget(_temp_dir);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

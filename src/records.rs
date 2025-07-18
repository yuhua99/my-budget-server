use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use tower_sessions::Session;
use uuid::Uuid;

use crate::auth::get_current_user;
use crate::database::{Db, get_user_db};
use crate::models::{CreateRecordPayload, GetRecordsQuery, GetRecordsResponse, Record};

pub fn extract_record_from_row(row: libsql::Row) -> Result<Record, (StatusCode, String)> {
    let id: String = row.get(0).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get record id: {}", e),
        )
    })?;
    let name: String = row.get(1).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get record name: {}", e),
        )
    })?;
    let amount: f64 = row.get(2).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get record amount: {}", e),
        )
    })?;
    let category_id: String = row.get(3).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get record category_id: {}", e),
        )
    })?;
    let timestamp: i64 = row.get(4).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get record timestamp: {}", e),
        )
    })?;

    Ok(Record {
        id,
        name,
        amount,
        category_id,
        timestamp,
    })
}

pub async fn create_record(
    State(_main_db): State<Db>,
    session: Session,
    Json(payload): Json<CreateRecordPayload>,
) -> Result<(StatusCode, Json<Record>), (StatusCode, String)> {
    // Get current user from session
    let user = get_current_user(&session).await?;

    // Input validation
    if payload.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Record name cannot be empty".to_string(),
        ));
    }
    if payload.name.len() > 255 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Record name must be less than 255 characters".to_string(),
        ));
    }
    if payload.amount == 0.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Record amount cannot be zero".to_string(),
        ));
    }
    if payload.category_id.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Category ID cannot be empty".to_string(),
        ));
    }

    // Get user's database
    let data_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data".to_string());
    let user_db = get_user_db(&data_path, &user.id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get user database: {}", e),
        )
    })?;

    // Create record
    let record_id = Uuid::new_v4().to_string();
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();

    let conn = user_db.write().await;
    conn.execute(
        "INSERT INTO records (id, name, amount, category_id, timestamp) VALUES (?, ?, ?, ?, ?)",
        (
            record_id.as_str(),
            payload.name.trim(),
            payload.amount,
            payload.category_id.trim(),
            timestamp,
        ),
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create record: {}", e),
        )
    })?;

    let record = Record {
        id: record_id,
        name: payload.name.trim().to_string(),
        amount: payload.amount,
        category_id: payload.category_id.trim().to_string(),
        timestamp,
    };

    Ok((StatusCode::CREATED, Json(record)))
}

pub async fn get_records(
    State(_main_db): State<Db>,
    session: Session,
    Query(query): Query<GetRecordsQuery>,
) -> Result<(StatusCode, Json<GetRecordsResponse>), (StatusCode, String)> {
    let user = get_current_user(&session).await?;

    let data_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data".to_string());
    let user_db = get_user_db(&data_path, &user.id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get user database: {}", e),
        )
    })?;

    let limit = query.limit.unwrap_or(500);

    let conn = user_db.read().await;

    // Use default values: start_time defaults to 0, end_time defaults to current timestamp
    let start_time = query.start_time.unwrap_or(0);
    let end_time = query
        .end_time
        .unwrap_or_else(|| time::OffsetDateTime::now_utc().unix_timestamp());

    // Get total count
    let count_query = "SELECT COUNT(*) FROM records WHERE timestamp BETWEEN ? AND ?";
    let mut count_rows = conn
        .query(count_query, (start_time, end_time))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count records: {}", e),
            )
        })?;

    let total_count: u32 = if let Some(row) = count_rows.next().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read count row: {}", e),
        )
    })? {
        row.get(0).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get count value: {}", e),
            )
        })?
    } else {
        0
    };

    // Get records
    let records_query = "SELECT id, name, amount, category_id, timestamp FROM records WHERE timestamp BETWEEN ? AND ? ORDER BY timestamp DESC LIMIT ?";
    let mut rows = conn
        .query(records_query, (start_time, end_time, limit))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to query records: {}", e),
            )
        })?;

    let mut records = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read record row: {}", e),
        )
    })? {
        records.push(extract_record_from_row(row)?);
    }

    Ok((
        StatusCode::OK,
        Json(GetRecordsResponse {
            records,
            total_count,
        }),
    ))
}

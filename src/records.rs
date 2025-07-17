use axum::{Json, extract::State, http::StatusCode};
use tower_sessions::Session;
use uuid::Uuid;

use crate::auth::get_current_user;
use crate::database::{Db, get_user_db};
use crate::models::{CreateRecordPayload, Record};

pub async fn create_record(
    State(main_db): State<Db>,
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

use axum::http::StatusCode;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

use crate::database::get_user_db;

static CACHED_DATABASE_PATH: OnceLock<String> = OnceLock::new();

pub fn get_database_path() -> &'static str {
    CACHED_DATABASE_PATH
        .get_or_init(|| std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data".to_string()))
}

pub async fn get_user_database(
    user_id: &str,
) -> Result<Arc<RwLock<libsql::Connection>>, (StatusCode, String)> {
    let data_path = get_database_path();
    get_user_db(&data_path, user_id).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database access error".to_string(),
        )
    })
}

pub fn db_error() -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Database operation failed".to_string(),
    )
}

pub fn db_error_with_context(context: &str) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Database error: {}", context),
    )
}

pub fn validate_string_length(
    value: &str,
    field_name: &str,
    max_length: usize,
) -> Result<(), (StatusCode, String)> {
    if value.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("{} cannot be empty", field_name),
        ));
    }
    if value.len() > max_length {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("{} must be less than {} characters", field_name, max_length),
        ));
    }
    Ok(())
}

pub async fn validate_category_exists(
    user_db: &Arc<RwLock<libsql::Connection>>,
    category_id: &str,
) -> Result<(), (StatusCode, String)> {
    let conn = user_db.read().await;
    let mut rows = conn
        .query("SELECT id FROM categories WHERE id = ?", [category_id])
        .await
        .map_err(|_| db_error_with_context("failed to check category existence"))?;

    if rows.next().await.map_err(|_| db_error())?.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Category does not exist".to_string(),
        ));
    }
    Ok(())
}

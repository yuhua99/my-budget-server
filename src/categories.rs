use axum::{Json, extract::State, http::StatusCode};
use tower_sessions::Session;
use uuid::Uuid;

use crate::auth::get_current_user;
use crate::database::Db;
use crate::models::{Category, CreateCategoryPayload};
use crate::utils::{db_error, db_error_with_context, get_user_database, validate_string_length};

pub fn validate_category_name(name: &str) -> Result<(), (StatusCode, String)> {
    validate_string_length(name, "Category name", 100)
}

pub fn extract_category_from_row(row: libsql::Row) -> Result<Category, (StatusCode, String)> {
    let id: String = row
        .get(0)
        .map_err(|_| db_error_with_context("invalid category data"))?;
    let name: String = row
        .get(1)
        .map_err(|_| db_error_with_context("invalid category data"))?;

    Ok(Category { id, name })
}

pub async fn create_category(
    State(_main_db): State<Db>,
    session: Session,
    Json(payload): Json<CreateCategoryPayload>,
) -> Result<(StatusCode, Json<Category>), (StatusCode, String)> {
    // Get current user from session
    let user = get_current_user(&session).await?;

    // Input validation and sanitization
    validate_category_name(&payload.name)?;
    let category_name = payload.name.trim().to_string();

    // Get user's database
    let user_db = get_user_database(&user.id).await?;

    // Use a single write connection for the entire transaction
    let conn = user_db.write().await;

    // Check if category name already exists (case-insensitive)
    let mut existing_rows = conn
        .query(
            "SELECT id FROM categories WHERE LOWER(name) = LOWER(?)",
            [category_name.as_str()],
        )
        .await
        .map_err(|_| db_error_with_context("failed to check existing category"))?;

    if existing_rows
        .next()
        .await
        .map_err(|_| db_error())?
        .is_some()
    {
        return Err((
            StatusCode::CONFLICT,
            "Category name already exists (case-insensitive)".to_string(),
        ));
    }

    // Create category
    let category_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO categories (id, name) VALUES (?, ?)",
        (category_id.as_str(), category_name.as_str()),
    )
    .await
    .map_err(|_| db_error_with_context("category creation failed"))?;

    let category = Category {
        id: category_id,
        name: category_name,
    };

    Ok((StatusCode::CREATED, Json(category)))
}

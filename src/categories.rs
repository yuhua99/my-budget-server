use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use tower_sessions::Session;
use uuid::Uuid;

use crate::auth::get_current_user;
use crate::constants::*;
use crate::database::Db;
use crate::models::{Category, CreateCategoryPayload, GetCategoriesQuery, GetCategoriesResponse};
use crate::utils::{
    db_error, db_error_with_context, get_user_database, validate_categories_limit, validate_offset,
    validate_string_length,
};

pub fn validate_category_name(name: &str) -> Result<(), (StatusCode, String)> {
    validate_string_length(name, "Category name", MAX_CATEGORY_NAME_LENGTH)
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

pub async fn get_categories(
    State(_main_db): State<Db>,
    session: Session,
    Query(query): Query<GetCategoriesQuery>,
) -> Result<(StatusCode, Json<GetCategoriesResponse>), (StatusCode, String)> {
    // Get current user from session
    let user = get_current_user(&session).await?;

    // Input validation
    let limit = validate_categories_limit(query.limit)?;
    let offset = validate_offset(query.offset)?;

    // Validate and sanitize search term
    let search_term = query
        .search
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    if let Some(search) = &search_term {
        validate_string_length(search, "Search term", MAX_SEARCH_TERM_LENGTH)?;
    }

    // Get user's database
    let user_db = get_user_database(&user.id).await?;
    let conn = user_db.read().await;

    // Get total count with search filter
    let total_count: u32 = if let Some(search) = &search_term {
        let search_pattern = format!("%{}%", search);
        let mut count_rows = conn
            .query(
                "SELECT COUNT(*) FROM categories WHERE name LIKE ? COLLATE NOCASE",
                [search_pattern.as_str()],
            )
            .await
            .map_err(|_| db_error_with_context("failed to count categories"))?;

        if let Some(row) = count_rows.next().await.map_err(|_| db_error())? {
            row.get(0).map_err(|_| db_error())?
        } else {
            0
        }
    } else {
        let mut count_rows = conn
            .query("SELECT COUNT(*) FROM categories", ())
            .await
            .map_err(|_| db_error_with_context("failed to count categories"))?;

        if let Some(row) = count_rows.next().await.map_err(|_| db_error())? {
            row.get(0).map_err(|_| db_error())?
        } else {
            0
        }
    };

    // Get categories with search filter, pagination, and ordering (utilizing the index)
    let mut rows = if let Some(search) = &search_term {
        let search_pattern = format!("%{}%", search);
        conn.query(
            "SELECT id, name FROM categories WHERE name LIKE ? COLLATE NOCASE ORDER BY name ASC LIMIT ? OFFSET ?",
            (search_pattern.as_str(), limit, offset)
        )
        .await
        .map_err(|_| db_error_with_context("failed to query categories"))?
    } else {
        conn.query(
            "SELECT id, name FROM categories ORDER BY name ASC LIMIT ? OFFSET ?",
            (limit, offset),
        )
        .await
        .map_err(|_| db_error_with_context("failed to query categories"))?
    };

    let mut categories = Vec::new();
    while let Some(row) = rows.next().await.map_err(|_| db_error())? {
        categories.push(extract_category_from_row(row)?);
    }

    Ok((
        StatusCode::OK,
        Json(GetCategoriesResponse {
            categories,
            total_count,
            limit,
            offset,
        }),
    ))
}

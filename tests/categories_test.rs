use axum::http::StatusCode;
use my_budget_server::categories::{
    extract_category_from_row, validate_category_name, validate_category_not_in_use,
};
use my_budget_server::database::get_user_db;
use my_budget_server::models::Category;
use uuid::Uuid;

mod common;
use common::*;

// Category-specific test helper functions
async fn create_test_category(data_path: &str, user_id: &str, name: &str) -> String {
    let user_db = get_user_db(data_path, user_id)
        .await
        .unwrap_or_else(|e| panic!("Failed to get user database for {}: {}", user_id, e));
    let category_id = Uuid::new_v4().to_string();

    let conn = user_db.write().await;
    conn.execute(
        "INSERT INTO categories (id, name) VALUES (?, ?)",
        (category_id.as_str(), name),
    )
    .await
    .unwrap_or_else(|e| {
        panic!(
            "Failed to insert test category '{}' for user {}: {}",
            name, user_id, e
        )
    });

    category_id
}

async fn get_category_from_db(
    data_path: &str,
    user_id: &str,
    category_id: &str,
) -> Option<Category> {
    let user_db = get_user_db(data_path, user_id)
        .await
        .unwrap_or_else(|e| panic!("Failed to get user database for {}: {}", user_id, e));
    let conn = user_db.read().await;

    let mut rows = conn
        .query(
            "SELECT id, name FROM categories WHERE id = ?",
            [category_id],
        )
        .await
        .expect("Failed to execute category query");

    if let Some(row) = rows.next().await.expect("Failed to read category row") {
        let id: String = row.get(0).expect("Failed to get category id");
        let name: String = row.get(1).expect("Failed to get category name");
        Some(Category { id, name })
    } else {
        None
    }
}

async fn get_all_categories_from_db(data_path: &str, user_id: &str) -> Vec<Category> {
    let user_db = get_user_db(data_path, user_id)
        .await
        .unwrap_or_else(|e| panic!("Failed to get user database for {}: {}", user_id, e));
    let conn = user_db.read().await;

    let mut rows = conn
        .query("SELECT id, name FROM categories ORDER BY name ASC", ())
        .await
        .expect("Failed to execute categories query");

    let mut categories = Vec::new();
    while let Some(row) = rows.next().await.expect("Failed to read category row") {
        let id: String = row.get(0).expect("Failed to get category id");
        let name: String = row.get(1).expect("Failed to get category name");
        categories.push(Category { id, name });
    }

    categories
}

async fn count_records_with_category(data_path: &str, user_id: &str, category_id: &str) -> u32 {
    let user_db = get_user_db(data_path, user_id)
        .await
        .unwrap_or_else(|e| panic!("Failed to get user database for {}: {}", user_id, e));
    let conn = user_db.read().await;

    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM records WHERE category_id = ?",
            [category_id],
        )
        .await
        .expect("Failed to execute count query");

    if let Some(row) = rows.next().await.expect("Failed to read count row") {
        row.get(0).expect("Failed to get count value")
    } else {
        0
    }
}

#[tokio::test]
async fn test_validate_category_name_valid() {
    let result = validate_category_name("Valid Category Name");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_category_name_empty() {
    let result = validate_category_name("");
    assert!(result.is_err());
    let (status, message) = result.unwrap_err();
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(message.contains("Category name cannot be empty"));
}

#[tokio::test]
async fn test_validate_category_name_whitespace_only() {
    let result = validate_category_name("   ");
    assert!(result.is_err());
    let (status, message) = result.unwrap_err();
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(message.contains("Category name cannot be empty"));
}

#[tokio::test]
async fn test_validate_category_name_too_long() {
    let long_name = "a".repeat(101); // Assuming max length is 100
    let result = validate_category_name(&long_name);
    assert!(result.is_err());
    let (status, message) = result.unwrap_err();
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(message.contains("must be less than"));
}

#[tokio::test]
async fn test_extract_category_from_row() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create a test category
    let category_id = create_test_category(&data_path, &user_id, "Test Category").await;

    // Query the category and test extraction
    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.read().await;

    let mut rows = conn
        .query(
            "SELECT id, name FROM categories WHERE id = ?",
            [category_id.as_str()],
        )
        .await
        .expect("Failed to query category");

    if let Some(row) = rows.next().await.expect("Failed to read row") {
        let category = extract_category_from_row(row);
        assert!(category.is_ok());
        let cat = category.unwrap();
        assert_eq!(cat.id, category_id);
        assert_eq!(cat.name, "Test Category");
    } else {
        panic!("No category found");
    }
}

#[tokio::test]
async fn test_validate_category_not_in_use_with_records() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let category_id = create_test_category(&data_path, &user_id, "Used Category").await;
    create_test_record(
        &data_path,
        &user_id,
        "Test Record",
        50.0,
        &category_id,
        1234567890,
    )
    .await;

    let user_db = get_user_db(&data_path, &user_id).await.unwrap();

    let result = validate_category_not_in_use(&user_db, &category_id).await;

    assert!(result.is_err());
    let (status, message) = result.unwrap_err();
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(message, "Cannot delete category: it has associated records");
}

#[tokio::test]
async fn test_validate_category_not_in_use_without_records() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let category_id = create_test_category(&data_path, &user_id, "Unused Category").await;

    let user_db = get_user_db(&data_path, &user_id).await.unwrap();

    let result = validate_category_not_in_use(&user_db, &category_id).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_category_update_database_operations() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create a test category
    let category_id = create_test_category(&data_path, &user_id, "Original Name").await;

    // Update the category directly in the database (simulating the update operation)
    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.write().await;

    let new_name = "Updated Name";
    let affected_rows = conn
        .execute(
            "UPDATE categories SET name = ? WHERE id = ?",
            (new_name, category_id.as_str()),
        )
        .await
        .expect("Failed to update category");

    assert_eq!(affected_rows, 1);
    drop(conn);

    // Verify the update worked
    let updated_category = get_category_from_db(&data_path, &user_id, &category_id).await;
    assert!(updated_category.is_some());
    assert_eq!(updated_category.unwrap().name, new_name);
}

#[tokio::test]
async fn test_category_update_nonexistent() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let non_existent_id = Uuid::new_v4().to_string();

    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.write().await;

    let affected_rows = conn
        .execute(
            "UPDATE categories SET name = ? WHERE id = ?",
            ("New Name", non_existent_id.as_str()),
        )
        .await
        .expect("Failed to execute update");

    // Should affect 0 rows since category doesn't exist
    assert_eq!(affected_rows, 0);
}

#[tokio::test]
async fn test_category_duplicate_name_detection() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    // Create two categories
    let _category1_id = create_test_category(&data_path, &user_id, "Category One").await;
    let category2_id = create_test_category(&data_path, &user_id, "Category Two").await;

    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.read().await;

    // Check for case-insensitive duplicate (excluding current category)
    let mut rows = conn
        .query(
            "SELECT id FROM categories WHERE LOWER(name) = LOWER(?) AND id != ?",
            ("CATEGORY ONE", category2_id.as_str()),
        )
        .await
        .expect("Failed to query for duplicates");

    // Should find the existing "Category One"
    assert!(rows.next().await.expect("Failed to read row").is_some());
}

#[tokio::test]
async fn test_category_same_name_update_allowed() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let category_id = create_test_category(&data_path, &user_id, "Category Name").await;

    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.read().await;

    // Check for duplicates excluding the same category (should find none)
    let mut rows = conn
        .query(
            "SELECT id FROM categories WHERE LOWER(name) = LOWER(?) AND id != ?",
            ("Category Name", category_id.as_str()),
        )
        .await
        .expect("Failed to query for duplicates");

    // Should find no duplicates when updating to the same name
    assert!(rows.next().await.expect("Failed to read row").is_none());
}

#[tokio::test]
async fn test_category_delete_database_operations() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let category_id = create_test_category(&data_path, &user_id, "Test Category").await;

    // Verify category exists
    let category = get_category_from_db(&data_path, &user_id, &category_id).await;
    assert!(category.is_some());

    // Delete the category
    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.write().await;

    let affected_rows = conn
        .execute(
            "DELETE FROM categories WHERE id = ?",
            [category_id.as_str()],
        )
        .await
        .expect("Failed to delete category");

    assert_eq!(affected_rows, 1);
    drop(conn);

    // Verify category was deleted
    let deleted_category = get_category_from_db(&data_path, &user_id, &category_id).await;
    assert!(deleted_category.is_none());
}

#[tokio::test]
async fn test_category_delete_nonexistent() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let non_existent_id = Uuid::new_v4().to_string();

    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.write().await;

    let affected_rows = conn
        .execute(
            "DELETE FROM categories WHERE id = ?",
            [non_existent_id.as_str()],
        )
        .await
        .expect("Failed to execute delete");

    // Should affect 0 rows since category doesn't exist
    assert_eq!(affected_rows, 0);
}

#[tokio::test]
async fn test_category_delete_preserves_others() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let category1_id = create_test_category(&data_path, &user_id, "Category 1").await;
    let category2_id = create_test_category(&data_path, &user_id, "Category 2").await;
    let category3_id = create_test_category(&data_path, &user_id, "Category 3").await;

    // Delete category 2
    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.write().await;

    let affected_rows = conn
        .execute(
            "DELETE FROM categories WHERE id = ?",
            [category2_id.as_str()],
        )
        .await
        .expect("Failed to delete category");

    assert_eq!(affected_rows, 1);
    drop(conn);

    // Verify category 2 is gone but others remain
    let all_categories = get_all_categories_from_db(&data_path, &user_id).await;
    assert_eq!(all_categories.len(), 2);

    let remaining_ids: Vec<String> = all_categories.iter().map(|c| c.id.clone()).collect();
    assert!(remaining_ids.contains(&category1_id));
    assert!(!remaining_ids.contains(&category2_id));
    assert!(remaining_ids.contains(&category3_id));
}

#[tokio::test]
async fn test_category_existence_check() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let category_id = create_test_category(&data_path, &user_id, "Test Category").await;

    let user_db = get_user_db(&data_path, &user_id).await.unwrap();
    let conn = user_db.read().await;

    // Check existing category
    let mut rows = conn
        .query(
            "SELECT id FROM categories WHERE id = ?",
            [category_id.as_str()],
        )
        .await
        .expect("Failed to query category");

    assert!(rows.next().await.expect("Failed to read row").is_some());

    // Check non-existent category
    let non_existent_id = Uuid::new_v4().to_string();
    let mut rows = conn
        .query(
            "SELECT id FROM categories WHERE id = ?",
            [non_existent_id.as_str()],
        )
        .await
        .expect("Failed to query category");

    assert!(rows.next().await.expect("Failed to read row").is_none());
}

#[tokio::test]
async fn test_referential_integrity_record_count() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let category_id = create_test_category(&data_path, &user_id, "Category with Records").await;

    // Initially no records
    let count = count_records_with_category(&data_path, &user_id, &category_id).await;
    assert_eq!(count, 0);

    // Add some records
    create_test_record(
        &data_path,
        &user_id,
        "Record 1",
        100.0,
        &category_id,
        1234567890,
    )
    .await;
    create_test_record(
        &data_path,
        &user_id,
        "Record 2",
        200.0,
        &category_id,
        1234567891,
    )
    .await;

    // Should now have 2 records
    let count = count_records_with_category(&data_path, &user_id, &category_id).await;
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_category_after_record_deletion() {
    let (data_path, user_id, _temp_dir) = setup_test_environment().await;

    let category_id = create_test_category(&data_path, &user_id, "Category").await;
    let record_id = create_test_record(
        &data_path,
        &user_id,
        "Test Record",
        75.0,
        &category_id,
        1234567890,
    )
    .await;

    let user_db = get_user_db(&data_path, &user_id).await.unwrap();

    // Initially category is in use
    let result = validate_category_not_in_use(&user_db, &category_id).await;
    assert!(result.is_err());

    // Delete the record
    let conn = user_db.write().await;
    let affected_rows = conn
        .execute("DELETE FROM records WHERE id = ?", [record_id.as_str()])
        .await
        .expect("Failed to delete record");
    assert_eq!(affected_rows, 1);
    drop(conn);

    // Now category should not be in use
    let result = validate_category_not_in_use(&user_db, &category_id).await;
    assert!(result.is_ok());

    // And can be deleted
    let conn = user_db.write().await;
    let affected_rows = conn
        .execute(
            "DELETE FROM categories WHERE id = ?",
            [category_id.as_str()],
        )
        .await
        .expect("Failed to delete category");
    assert_eq!(affected_rows, 1);
}

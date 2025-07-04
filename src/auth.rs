use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::{Json, extract::State, http::StatusCode};
use libsql::Connection;

use crate::database::Db;
use crate::models::{PublicUser, RegisterPayload};

async fn create_user(
    conn: &mut Connection,
    username: &str,
    password: &str,
) -> anyhow::Result<PublicUser> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    conn.execute(
        "INSERT INTO users (name, password_hash) VALUES (?, ?)",
        (username, hash.as_str()),
    )
    .await?;

    let id = conn.last_insert_rowid() as i32;
    Ok(PublicUser {
        id,
        username: username.to_string(),
    })
}

pub async fn register(
    State(db): State<Db>,
    Json(payload): Json<RegisterPayload>,
) -> Result<(StatusCode, Json<PublicUser>), (StatusCode, String)> {
    let mut conn = db.lock().await;
    let user = create_user(&mut conn, &payload.username, &payload.password)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(user)))
}

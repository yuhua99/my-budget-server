use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{Json, extract::State, http::StatusCode};
use tower_sessions::Session;

use crate::database::Db;
use crate::models::{LoginPayload, PublicUser, RegisterPayload, User};

async fn create_user(db: &Db, username: &str, password: &str) -> anyhow::Result<PublicUser> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    let conn = db.write().await;

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
    // TODO: add error handling for invalid arg and duplicate usernames
    let user = create_user(&db, &payload.username, &payload.password)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(user)))
}

async fn get_user_by_username(db: &Db, username: &str) -> anyhow::Result<Option<User>> {
    let conn = db.read().await;
    let mut rows = conn
        .query(
            "SELECT id, name, password_hash FROM users WHERE name = ?",
            [username],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        let id: i32 = row.get(0)?;
        let username: String = row.get(1)?;
        let password_hash: String = row.get(2)?;
        Ok(Some(User {
            id,
            username,
            password_hash,
        }))
    } else {
        Ok(None)
    }
}

fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| anyhow::anyhow!("Failed to parse password hash: {}", e))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub async fn login(
    State(db): State<Db>,
    session: Session,
    Json(payload): Json<LoginPayload>,
) -> Result<(StatusCode, Json<PublicUser>), (StatusCode, String)> {
    let user_data = get_user_by_username(&db, &payload.username)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = match user_data {
        Some(data) => data,
        None => return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string())),
    };

    let is_valid = verify_password(&payload.password, &user.password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !is_valid {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()));
    }

    // Set user session
    session
        .insert("user_id", &user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    session
        .insert("username", &user.username.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(PublicUser {
            id: user.id,
            username: user.username,
        }),
    ))
}

pub async fn get_current_user(session: &Session) -> Result<PublicUser, (StatusCode, String)> {
    let user_id: Option<i32> = session
        .get("user_id")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let username: Option<String> = session
        .get("username")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match (user_id, username) {
        (Some(id), Some(name)) => Ok(PublicUser { id, username: name }),
        _ => Err((StatusCode::UNAUTHORIZED, "Not logged in".to_string())),
    }
}

pub async fn logout(session: Session) -> Result<StatusCode, (StatusCode, String)> {
    session.clear().await;

    Ok(StatusCode::NO_CONTENT)
}

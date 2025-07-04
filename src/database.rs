use anyhow::Result;
use libsql::{Builder, Connection};
use std::{path::Path, sync::Arc};
use tokio::sync::RwLock;

const CREATE_USERS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    name           TEXT    UNIQUE NOT NULL,
    password_hash  TEXT    NOT NULL
);
"#;

pub type Db = Arc<RwLock<Connection>>;

/// Main users registry DB (users.db)
pub async fn init_main_db(data_dir: &str) -> Result<Db> {
    tokio::fs::create_dir_all(data_dir).await?;
    let path = Path::new(data_dir).join("users.db");
    let db = Builder::new_local(path).build().await?;
    let conn = db.connect()?;

    conn.execute(CREATE_USERS_TABLE, ()).await?;
    Ok(Arc::new(RwLock::new(conn)))
}

/// Per-user isolated DB (user_{id}.db)
pub async fn get_user_db(data_dir: &str, user_id: i32) -> Result<Db> {
    let path = Path::new(data_dir).join(format!("user_{}.db", user_id));
    let db = Builder::new_local(path).build().await?;
    let conn = db.connect()?;
    Ok(Arc::new(RwLock::new(conn)))
}

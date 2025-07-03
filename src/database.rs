use anyhow::Result;
use libsql::{Builder, Database};
use std::{path::Path, sync::Arc};

/// Main users registry DB (users.db)
pub async fn init_main_db(data_dir: &str) -> Result<Arc<Database>> {
    tokio::fs::create_dir_all(data_dir).await?;
    let path = Path::new(data_dir).join("users.db");
    let db = Builder::new_local(path).build().await?;
    Ok(Arc::new(db))
}

/// Per-user isolated DB (user_{id}.db)
pub async fn get_user_db(data_dir: &str, user_id: i32) -> Result<Arc<Database>> {
    let path = Path::new(data_dir).join(format!("user_{}.db", user_id));
    let db = Builder::new_local(path).build().await?;
    Ok(Arc::new(db))
}


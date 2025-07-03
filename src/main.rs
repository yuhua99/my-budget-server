use axum::{Router, routing::get};
use std::env;

mod database;
mod models;

#[tokio::main]
async fn main() {
    // load environment variables
    dotenv::dotenv().ok();

    let data_path = env::var("DATABASE_PATH").unwrap_or_else(|_| "data".to_string());
    let main_db = database::init_main_db(&data_path)
        .await
        .expect("Failed to initialize main DB");

    let host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("SERVER_PORT").unwrap_or_else(|_| "3000".to_string());
    let bind_address = format!("{}:{}", host, port);

    let app = Router::new().route("/", get(root)).with_state(main_db);

    let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();
    println!("Server running on http://{}", bind_address);

    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

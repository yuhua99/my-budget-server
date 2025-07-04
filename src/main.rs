use axum::{
    Router,
    response::Html,
    routing::{get, post},
};
use std::env;
use time::Duration;
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer, cookie::Key};

mod auth;
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

    let store = MemoryStore::default();

    // TODO: show detail error message if secret is invalid
    let secret = env::var("SESSION_SECRET")
        .unwrap_or_else(|_| "a_super_secret_and_long_key_that_is_at_least_64_bytes_long".into());
    let session_layer = SessionManagerLayer::new(store)
        .with_secure(false)
        .with_name("axum_sessoin")
        .with_expiry(Expiry::OnInactivity(Duration::days(3)))
        .with_signed(Key::try_from(secret.as_bytes()).unwrap());

    let app = Router::new()
        .route("/", get(root))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .layer(session_layer)
        .with_state(main_db);

    let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();
    println!("Server running on http://{}", bind_address);

    axum::serve(listener, app).await.unwrap();
}

async fn root(session: Session) -> Html<String> {
    let count: usize = session.get("visitor_count").await.unwrap().unwrap_or(0);
    session.insert("visitor_count", count + 1).await.unwrap();

    Html(format!("<h1>Hello!</h1><p>Visit count: {}</p>", count + 1))
}

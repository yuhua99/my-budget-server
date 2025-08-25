use axum::{
    Router,
    response::Html,
    routing::{get, post, put},
};
use time::Duration;
use tower_http::cors::CorsLayer;
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer, cookie::Key};

pub mod auth;
pub mod categories;
pub mod config;
pub mod constants;
pub mod database;
pub mod models;
pub mod records;
pub mod utils;

use config::Config;
use constants::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Load and validate configuration
    let config = Config::from_env().map_err(|e| format!("Configuration error: {}", e))?;

    // Initialize main database
    let main_db = database::init_main_db(&config.data_path)
        .await
        .map_err(|e| format!("Failed to initialize main database: {}", e))?;

    // Create session store
    let store = MemoryStore::default();
    // TODO: Consider adding periodic session cleanup for long-running deployments
    // to prevent memory growth with accumulated expired sessions

    // Create session key with proper error handling
    let session_key = Key::try_from(config.session_secret.as_bytes())
        .map_err(|e| format!("Invalid session secret: {}", e))?;

    // Determine if we should use secure cookies based on environment
    // Only use secure cookies when explicitly in production with HTTPS
    let is_production = std::env::var("PRODUCTION")
        .map(|val| val.to_lowercase() == "true")
        .unwrap_or(false);

    let session_layer = SessionManagerLayer::new(store)
        .with_secure(is_production) // Only secure in production
        .with_name(SESSION_NAME)
        .with_expiry(Expiry::OnInactivity(Duration::days(SESSION_EXPIRY_DAYS)))
        .with_signed(session_key);

    // Configure CORS to allow frontend requests from any origin (development)
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::COOKIE,
        ])
        .allow_credentials(true);

    // Build application router
    let app = Router::new()
        .route("/", get(root))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", post(auth::logout))
        .route(
            "/records",
            post(records::create_record).get(records::get_records),
        )
        .route(
            "/records/{id}",
            put(records::update_record).delete(records::delete_record),
        )
        .route(
            "/categories",
            post(categories::create_category).get(categories::get_categories),
        )
        .route(
            "/categories/{id}",
            put(categories::update_category).delete(categories::delete_category),
        )
        .layer(cors)
        .layer(session_layer)
        .with_state(main_db);

    // Create TCP listener with proper error handling
    let bind_address = config.bind_address();
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .map_err(|e| format!("Failed to bind to {}: {}", bind_address, e))?;

    println!("Server running on http://{}", bind_address);

    // Start server with proper error handling
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    Ok(())
}

async fn root(session: Session) -> Html<String> {
    let count: usize = session
        .get("visitor_count")
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);
    let new_count = count + 1;

    // Ignore session update errors for this simple endpoint
    let _ = session.insert("visitor_count", new_count).await;

    Html(format!(
        "<h1>My Budget Server</h1><p>API Ready - Visit count: {}</p>",
        new_count
    ))
}

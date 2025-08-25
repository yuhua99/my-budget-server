// Server configuration
pub const DEFAULT_HOST: &str = "0.0.0.0";
pub const DEFAULT_PORT: &str = "3000";
pub const DEFAULT_DATA_PATH: &str = "data";

// Session configuration
pub const SESSION_NAME: &str = "axum_session";
pub const SESSION_EXPIRY_DAYS: i64 = 30;
pub const MIN_SESSION_SECRET_LENGTH: usize = 64;

// Database limits and defaults
pub const DEFAULT_CATEGORIES_LIMIT: u32 = 100;
pub const DEFAULT_RECORDS_LIMIT: u32 = 500;
pub const MAX_LIMIT: u32 = 1000;
pub const MAX_OFFSET: u32 = 1_000_000;

// Validation limits
pub const MAX_CATEGORY_NAME_LENGTH: usize = 100;
pub const MAX_RECORD_NAME_LENGTH: usize = 255;
pub const MAX_SEARCH_TERM_LENGTH: usize = 100;
pub const MAX_USERNAME_LENGTH: usize = 50;
pub const MIN_USERNAME_LENGTH: usize = 4;
pub const MIN_PASSWORD_LENGTH: usize = 6;

// Error messages
pub const ERR_DATABASE_ACCESS: &str = "Database access error";
pub const ERR_DATABASE_OPERATION: &str = "Database operation failed";
pub const ERR_INVALID_SESSION: &str = "Invalid session";
pub const ERR_UNAUTHORIZED: &str = "Not logged in";

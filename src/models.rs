use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
}

#[derive(Deserialize)]
pub struct RegisterPayload {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct PublicUser {
    pub id: String,
    pub username: String,
}

#[derive(Deserialize)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Record {
    pub id: String,
    pub name: String,
    pub amount: f64,
    pub category_id: String,
    pub timestamp: i64,
}

#[derive(Deserialize)]
pub struct CreateRecordPayload {
    pub name: String,
    pub amount: f64,
    pub category_id: String,
}

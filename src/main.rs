use std::{collections::HashMap, sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};

use axum::{Json, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const SUPPORTED_LANGUAGES: &[&str] = &[
    "rust", "python", "javascript", "typescript", "go", "java", "c", "cpp",
    "ruby", "php", "swift", "kotlin", "scala", "elixir", "haskell",
    "bash", "sql", "html", "css", "json", "yaml", "toml", "markdown",
    "plaintext",
];
const MAX_CONTENT_LENGTH: usize = 500 * 1024; 

#[derive(Clone)]
struct AppState {
    pastes : Arc<RwLock<HashMap<String,Paste>>>
}
#[derive(Clone,Serialize)]
struct Paste {
    id : String,
    content : String,
    created_at : DateTime<Utc>,
    expires_at : Option<DateTime<Utc>>,
    language : Option<String>
}

#[derive(Deserialize)]
struct CreatePasteRequest {
    content : String,
    language : Option<String>,
    expires_in_seconds : Option<i64>
}
#[derive(Serialize)]
struct CreatePasteResponse {
    id: String,
}
#[derive(Serialize)]
struct GetPasteResponse {
    id: String,
    content: String,
    language: Option<String>,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
}
enum AppError {
    ValidationError(String),
    NotFound(String),
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AppError::InternalError(msg) => {
                eprintln!("internal error:  {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string()
                )
            },
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),

            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
        };

        let body = Json(serde_json::json!({
            "error": message
        }));
        (status, body).into_response()
    }
}

impl<T> From<std::sync::PoisonError<RwLockReadGuard<'_, T>>> for AppError {
    fn from(_: std::sync::PoisonError<RwLockReadGuard<'_, T>>) -> Self {
        AppError::InternalError("lock poisoned".into())
    }
}

impl<T> From<std::sync::PoisonError<RwLockWriteGuard<'_, T>>> for AppError {
    fn from(_: std::sync::PoisonError<RwLockWriteGuard<'_, T>>) -> Self {
        AppError::InternalError("lock poisoned".into())
    }
}

fn main(){

}
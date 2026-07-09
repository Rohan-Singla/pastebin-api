use std::{
    collections::HashMap,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use uuid::Uuid;

const SUPPORTED_LANGUAGES: &[&str] = &[
    "rust", "python", "javascript", "typescript", "go", "java", "c", "cpp", "ruby", "php", "swift",
    "kotlin", "scala", "elixir", "haskell", "bash", "sql", "html", "css", "json", "yaml", "toml",
    "markdown", "plaintext",
];
const MAX_CONTENT_LENGTH: usize = 500 * 1024;

#[derive(Clone)]
struct AppState {
    pastes: Arc<RwLock<HashMap<String, Paste>>>,
}
#[derive(Clone, Serialize)]
struct Paste {
    id: String,
    content: String,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    language: Option<String>,
}

#[derive(Deserialize)]
struct CreatePasteRequest {
    content: String,
    language: Option<String>,
    expires_in_seconds: Option<i64>,
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

#[derive(Debug)]
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
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error".to_string())
            }
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

#[tokio::main]
async fn main() {
    let state = AppState {
        pastes: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/paste", post(create_paste))
        .route("/paste/{id}", get(get_paste))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn create_paste(
    State(state): State<AppState>,
    Json(payload): Json<CreatePasteRequest>,
) -> Result<impl IntoResponse, AppError> {
    validate_create_request(&payload)?;

    let id = &Uuid::new_v4().to_string()[0..8];

    let expires_at = payload.expires_in_seconds.map(|seconds| Utc::now() + Duration::seconds(seconds));

    let paste = Paste {
        id: id.to_string(),
        content: payload.content,
        language: payload.language,
        created_at: Utc::now(),
        expires_at,
    };

    state
        .pastes
        .write()
        .map_err(|e| AppError::from(e))?
        .insert(id.to_string(), paste);

    Ok((
        StatusCode::CREATED,
        Json(CreatePasteResponse { id: id.to_string() }),
    ))
}

async fn get_paste(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let pastes = state.pastes.read().map_err(|e| AppError::from(e))?;
    let paste = pastes
        .get(&id)
        .ok_or_else(|| AppError::NotFound(format!("paste with id '{}' not found", id)))?;

    if let Some(expires_at) = paste.expires_at {
        if Utc::now() > expires_at {
            return Err(AppError::NotFound(format!(
                "paste with id '{}' has expired",
                id
            )));
        }
    }

    let response = GetPasteResponse {
        id: paste.id.clone(),
        content: paste.content.clone(),
        language: paste.language.clone(),
        created_at: paste.created_at,
        expires_at: paste.expires_at,
    };

    Ok((StatusCode::OK, Json(response)))
}

fn validate_create_request(req: &CreatePasteRequest) -> Result<(), AppError> {
    if req.content.trim().is_empty() {
        return Err(AppError::ValidationError(
            "content must not be empty".to_string(),
        ));
    }

    if req.content.len() > MAX_CONTENT_LENGTH {
        return Err(AppError::ValidationError(format!(
            "content exceeds maximum length of {} bytes (got {} bytes)",
            MAX_CONTENT_LENGTH,
            req.content.len()
        )));
    }

    if let Some(ref lang) = req.language {
        if !SUPPORTED_LANGUAGES.contains(&lang.as_str()) {
            return Err(AppError::ValidationError(format!(
                "unsupported language: {}",
                lang
            )));
        }
    }

    if let Some(seconds) = req.expires_in_seconds {
        if seconds <= 0 {
            return Err(AppError::ValidationError(
                "expires_in_seconds must be positive".into(),
            ));
        }
    }
    Ok(())
}
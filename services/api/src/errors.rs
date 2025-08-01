use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;
use axum::http::HeaderMap;

#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
    pub headers: Option<HeaderMap>,
}

impl AppError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            headers: None,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn too_many_requests(message: impl Into<String>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, message)
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.status, self.message)
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "error": {
                "message": self.message,
                "status": self.status.as_u16(),
            }
        }));

        let mut resp = (self.status, body).into_response();

        // Add custom HEADERS
        if let Some(headers) = self.headers {
            let headers_mut = resp.headers_mut();
            for (key, value) in headers.iter() {
                headers_mut.insert(key, value.clone());
            }
        }

        resp
    }
}

impl From<postgres_models::DbError> for AppError {
    fn from(err: postgres_models::DbError) -> Self {
        AppError::internal_server_error(format!("Database error: {}", err))
    }
}

impl From<redis_cache::RedisError> for AppError {
    fn from(err: redis_cache::RedisError) -> Self {
        AppError::internal_server_error(format!("Redis error: {}", err))
    }
}

impl From<diesel::result::Error> for AppError {
    fn from(err: diesel::result::Error) -> Self {
        AppError::internal_server_error(format!("Database error: {}", err))
    }
}

pub type AppResult<T> = Result<T, AppError>;
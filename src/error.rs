use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn internal(error: impl std::fmt::Display) -> Self {
        Self::Internal(anyhow::anyhow!(error.to_string()))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message),
            Self::Forbidden(message) => (StatusCode::FORBIDDEN, message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message),
            Self::Conflict(message) => (StatusCode::CONFLICT, message),
            Self::Internal(error) => {
                tracing::error!(error = %error, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "服务器内部错误，请稍后重试".to_string(),
                )
            }
        };

        (status, Json(json!({ "message": message }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self::internal(error)
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(error: argon2::password_hash::Error) -> Self {
        Self::internal(error)
    }
}

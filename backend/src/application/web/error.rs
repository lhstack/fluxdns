//! HTTP error representation.
//!
//! Maps the business layer's `AppError` onto HTTP responses. This is the only
//! place that knows about status codes.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::infrastructure::common::AppError;

/// API error response body
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    /// Build an error carrying structured field-level details.
    pub fn with_details(
        code: &str,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details: Some(details),
        }
    }

    /// Build a 400 response from a serializable validation report.
    ///
    /// Serialization of a plain report struct cannot fail in practice, but a
    /// failure here must not be silently swallowed, so it degrades to an
    /// explicit message instead of a fabricated success.
    pub fn validation<T: Serialize>(errors: T) -> Self {
        match serde_json::to_value(errors) {
            Ok(details) => Self::with_details("BAD_REQUEST", "Validation failed", details),
            Err(e) => Self {
                code: "INTERNAL_ERROR".to_string(),
                message: format!("Failed to serialize validation errors: {}", e),
                details: None,
            },
        }
    }
}

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        Self {
            code: err.code().to_string(),
            message: err.to_string(),
            details: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "UNAUTHORIZED" => StatusCode::UNAUTHORIZED,
            "FORBIDDEN" => StatusCode::FORBIDDEN,
            "BAD_REQUEST" => StatusCode::BAD_REQUEST,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}

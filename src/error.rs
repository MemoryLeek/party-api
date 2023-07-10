use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tower::BoxError;

#[derive(Serialize)]
pub(crate) struct ApiError {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        Self {
            error: error.to_string(),
        }
    }
}

impl From<BoxError> for ApiError {
    fn from(error: BoxError) -> Self {
        Self {
            error: error.to_string(),
        }
    }
}

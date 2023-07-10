use std::borrow::Cow;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tower::BoxError;
use tower_governor::GovernorError;

#[derive(Serialize)]
pub(crate) struct ApiError {
    #[serde(skip_serializing)]
    code: StatusCode,
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.code, Json(self)).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Database(db_error) if db_error.code() == Some(Cow::Borrowed("2067")) => {
                Self {
                    code: StatusCode::BAD_REQUEST,
                    error: db_error.to_string(),
                }
            }
            _ => Self {
                code: StatusCode::INTERNAL_SERVER_ERROR,
                error: error.to_string(),
            },
        }
    }
}

impl From<BoxError> for ApiError {
    fn from(error: BoxError) -> Self {
        match error.downcast_ref::<GovernorError>() {
            Some(GovernorError::TooManyRequests { .. }) => Self {
                code: StatusCode::TOO_MANY_REQUESTS,
                error: "too many requests".to_owned(),
            },
            Some(_) | None => Self {
                code: StatusCode::INTERNAL_SERVER_ERROR,
                error: error.to_string(),
            },
        }
    }
}

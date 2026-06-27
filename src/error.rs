use std::borrow::Cow;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tower_governor::GovernorError;

use crate::captcha;

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

impl From<captcha::Error> for ApiError {
    fn from(error: captcha::Error) -> Self {
        match error {
            captcha::Error::InvalidToken => Self {
                code: StatusCode::BAD_REQUEST,
                error: "captcha validation failed".to_owned(),
            },
            captcha::Error::ServiceError(_) => Self {
                code: StatusCode::INTERNAL_SERVER_ERROR,
                error: "captcha service unreachable".to_owned(),
            },
        }
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

impl From<GovernorError> for ApiError {
    fn from(error: GovernorError) -> Self {
        match error {
            GovernorError::TooManyRequests { .. } => Self {
                code: StatusCode::TOO_MANY_REQUESTS,
                error: "too many requests".to_owned(),
            },
            _ => Self {
                code: StatusCode::INTERNAL_SERVER_ERROR,
                error: error.to_string(),
            },
        }
    }
}
